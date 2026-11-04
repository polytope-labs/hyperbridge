use std::{sync::Arc, time::Duration};

use alloy::{
	network::{AnyRpcBlock, AnyTransactionReceipt},
	primitives::{Address, Bloom, BloomInput, Bytes, TxHash, B256, U64},
	providers::Provider,
	rpc::types::{trace::parity::LocalizedTransactionTrace, Filter, Log},
};
use alloy_chains::Chain;
use async_trait::async_trait;
use rindexer::{
	event::RindexerEventFilter,
	notifications::ChainStateNotification,
	provider::{ChainProvider, JsonRpcCachedProvider, ProviderError},
};
use tokio::sync::broadcast::Sender;

/** Public RPCs cap the addresses in one `eth_getLogs` (publicnode allows 10), so larger address
 * sets are split into concurrent requests, as rindexer's own fetcher does. */
const MAX_ADDRESSES_PER_REQUEST: usize = 10;

/** Re-asks of a tip block that came back empty although its bloom says it can have our logs,
 * 200 ms more apart each time, as rindexer re-asks when its own bloom checks are on. */
const TIP_RETRIES: u32 = 4;
const TIP_RETRY_DELAY: Duration = Duration::from_millis(200);

/**
 * rindexer's provider, except that `eth_getLogs` asks for every topic the chain's handlers
 * subscribe to. rindexer then runs one stream per chain, and handlers see the chain's logs in
 * block order, as SubQuery delivered them. The requests go through rindexer's own client, which
 * retries rate limits and times out. Two cases rindexer's live loop can't recover from are
 * handled here: an error on a short range, and an empty answer for the tip block from an RPC
 * that hasn't caught up.
 */
#[derive(Debug)]
pub struct AllTopicsProvider {
	inner: Arc<JsonRpcCachedProvider>,
	topics: Vec<B256>,
	reorg_safe_distance: u64,
}

impl AllTopicsProvider {
	pub fn new(
		inner: Arc<JsonRpcCachedProvider>,
		topics: Vec<B256>,
		reorg_safe_distance: u64,
	) -> Self {
		Self { inner, topics, reorg_safe_distance }
	}

	/**
	 * One request's logs. On a short range, errors are retried here instead of returned, after
	 * 1 s, doubling up to 16 s.
	 */
	async fn request_logs(
		&self,
		request: &Filter,
		short: bool,
		tip: Option<&AnyRpcBlock>,
		addresses: &[Address],
	) -> Result<Vec<Log>, ProviderError> {
		let rpc = self.inner.get_inner_provider();
		let (mut errors, mut tip_retries) = (0u32, 0u32);
		loop {
			match rpc.get_logs(request).await {
				Ok(logs) => {
					let suspect = tip.is_some_and(|tip| {
						let (bloom, number) = (&tip.header.logs_bloom, tip.header.number);
						lacks_tip_logs(bloom, number, addresses, &self.topics, &logs)
					});
					if suspect && tip_retries < TIP_RETRIES {
						tip_retries += 1;
						tokio::time::sleep(TIP_RETRY_DELAY * tip_retries).await;
						continue;
					}
					return Ok(logs);
				},
				Err(err) if short => {
					errors += 1;
					let delay = Duration::from_secs(1 << (errors.min(5) - 1));
					let blocks = &request.block_option;
					tracing::warn!(?blocks, "eth_getLogs failed, retrying in {delay:?}: {err}");
					tokio::time::sleep(delay).await;
				},
				Err(err) => return Err(err.into()),
			}
		}
	}
}

/**
 * Whether rindexer can't recover from an error on this range (v0.43.4, `fetch_logs.rs`).
 * After a live error, rindexer caps its head at the middle of the range, and at least two
 * blocks past its start, until a fetch succeeds. It only fetches below that cap less the
 * reorg-safe distance, so after an error on a range shorter than about twice that distance it
 * never fetches again. With no distance, the cap can pass the head, and an empty answer then
 * skips the blocks up to it. Live ranges are this short. History ranges are longer, and their
 * errors go back to rindexer, which shrinks the range.
 */
fn is_short(from: u64, to: u64, reorg_safe_distance: u64) -> bool {
	to.saturating_sub(from) < 2 * (reorg_safe_distance + 2)
}

/**
 * Whether `logs` has none in the tip block although its bloom says `addresses` can have logs
 * with `topics` there. A bloom has no false negatives, so the likely reason is an RPC that
 * hasn't caught up, such as one backend of a load balancer.
 */
fn lacks_tip_logs(
	bloom: &Bloom,
	tip: u64,
	addresses: &[Address],
	topics: &[B256],
	logs: &[Log],
) -> bool {
	let has = |bytes: &[u8]| bloom.contains_input(BloomInput::Raw(bytes));
	let may_have_logs = addresses.iter().any(|address| has(address.as_slice())) &&
		topics.iter().any(|topic| has(topic.as_slice()));
	may_have_logs && !logs.iter().any(|log| log.block_number == Some(tip))
}

#[async_trait]
impl ChainProvider for AllTopicsProvider {
	fn chain(&self) -> Chain {
		self.inner.chain()
	}

	fn max_block_range(&self) -> Option<U64> {
		self.inner.max_block_range()
	}

	fn chain_state_notification(&self) -> Option<Sender<ChainStateNotification>> {
		self.inner.chain_state_notification()
	}

	async fn get_latest_block(&self) -> Result<Option<Arc<AnyRpcBlock>>, ProviderError> {
		self.inner.get_latest_block().await
	}

	async fn get_block_number(&self) -> Result<U64, ProviderError> {
		self.inner.get_block_number().await
	}

	async fn get_logs(&self, filter: &RindexerEventFilter) -> Result<Vec<Log>, ProviderError> {
		let mut addresses: Vec<_> = match filter.contract_addresses().await {
			Some(addresses) if addresses.is_empty() => return Ok(vec![]),
			Some(addresses) => addresses.into_iter().collect(),
			None =>
				return Err(ProviderError::CustomError(
					"refusing an eth_getLogs with no address filter".into(),
				)),
		};
		addresses.sort();
		let (from, to) = (filter.from_block().to::<u64>(), filter.to_block().to::<u64>());
		let short = is_short(from, to, self.reorg_safe_distance);
		// The live window's last block may be the tip, which an RPC behind it answers empty.
		let tip = if short { self.inner.get_latest_block().await.ok().flatten() } else { None };
		let tip = tip.as_deref().filter(|tip| tip.header.number == to);
		let requests = addresses.chunks(MAX_ADDRESSES_PER_REQUEST).map(|chunk| async move {
			let request = Filter::new()
				.address(chunk.to_vec())
				.event_signature(self.topics.clone())
				.from_block(from)
				.to_block(to);
			self.request_logs(&request, short, tip, chunk).await
		});
		let mut logs: Vec<Log> = futures::future::try_join_all(requests).await?.concat();
		logs.sort_by_key(|log| (log.block_number, log.log_index));
		Ok(logs)
	}

	async fn get_block_by_number_batch(
		&self,
		block_numbers: &[U64],
		include_txs: bool,
	) -> Result<Vec<AnyRpcBlock>, ProviderError> {
		self.inner.get_block_by_number_batch(block_numbers, include_txs).await
	}

	async fn get_block_by_number_batch_with_size(
		&self,
		block_numbers: &[U64],
		include_txs: bool,
		rpc_batch_size: Option<usize>,
	) -> Result<Vec<AnyRpcBlock>, ProviderError> {
		self.inner
			.get_block_by_number_batch_with_size(block_numbers, include_txs, rpc_batch_size)
			.await
	}

	async fn get_tx_receipts_batch(
		&self,
		hashes: &[TxHash],
	) -> Result<Vec<AnyTransactionReceipt>, ProviderError> {
		self.inner.get_tx_receipts_batch(hashes).await
	}

	async fn trace_block(
		&self,
		block_number: U64,
	) -> Result<Vec<LocalizedTransactionTrace>, ProviderError> {
		self.inner.trace_block(block_number).await
	}

	async fn debug_trace_block_by_number(
		&self,
		block_number: U64,
	) -> Result<Vec<LocalizedTransactionTrace>, ProviderError> {
		self.inner.debug_trace_block_by_number(block_number).await
	}

	async fn eth_call(
		&self,
		to: alloy::primitives::Address,
		data: Bytes,
		block_number: u64,
	) -> Result<String, ProviderError> {
		self.inner.eth_call(to, data, block_number).await
	}

	async fn eth_call_latest(
		&self,
		to: alloy::primitives::Address,
		data: Bytes,
	) -> Result<String, ProviderError> {
		self.inner.eth_call_latest(to, data).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/**
	 * rindexer's live loop after an error on `[from, from + span]`. It takes `from + cap` as
	 * the head until a fetch succeeds: whether it then stops fetching for good, or fetches
	 * past the last block it may.
	 */
	fn rindexer_cannot_recover(span: u64, reorg_safe_distance: u64) -> bool {
		let cap = (span / 2).max(2);
		cap < reorg_safe_distance || cap > span + reorg_safe_distance
	}

	#[test]
	fn every_range_rindexer_cannot_recover_from_is_short() {
		for distance in 0..=64 {
			for span in 0..512 {
				if rindexer_cannot_recover(span, distance) {
					assert!(
						is_short(1_000, 1_000 + span, distance),
						"span {span}, distance {distance}"
					);
				}
			}
			// History ranges are far longer, and keep rindexer's range shrinking.
			assert!(!is_short(0, 5_000, distance));
		}
	}

	#[test]
	fn an_empty_answer_for_a_bloom_positive_tip_is_suspect() {
		let (host, other) = (Address::repeat_byte(0x11), Address::repeat_byte(0x22));
		let topic = B256::repeat_byte(0x33);
		let mut bloom = Bloom::default();
		bloom.accrue(BloomInput::Raw(host.as_slice()));
		bloom.accrue(BloomInput::Raw(topic.as_slice()));
		let at = |block: u64| Log { block_number: Some(block), ..Default::default() };

		assert!(lacks_tip_logs(&bloom, 10, &[host], &[topic], &[]));
		assert!(lacks_tip_logs(&bloom, 10, &[host], &[topic], &[at(9)]));
		assert!(!lacks_tip_logs(&bloom, 10, &[host], &[topic], &[at(10)]));
		// The bloom rules the block out for this request's addresses or topics.
		assert!(!lacks_tip_logs(&bloom, 10, &[other], &[topic], &[]));
		assert!(!lacks_tip_logs(&bloom, 10, &[host], &[B256::repeat_byte(0x44)], &[]));
	}
}
