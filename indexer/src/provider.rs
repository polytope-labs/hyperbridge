use std::{
	future::Future,
	sync::{
		atomic::{AtomicU64, Ordering},
		Arc,
	},
	time::Duration,
};

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

use crate::{errors::is_answer, failover::Failover};

/** Public RPCs cap the addresses in one `eth_getLogs` (publicnode allows 10), so larger address
 * sets are split into concurrent requests, as rindexer's own fetcher does. */
const MAX_ADDRESSES_PER_REQUEST: usize = 10;

/**
 * Re-asks of a tip block that came back empty although its bloom says it can have our logs,
 * 250 ms more apart each time. rindexer's own check re-asks four times, over about 2 s; a busy
 * chain's bloom matches nearly anything, so Base would pay for every re-ask on nearly every
 * block.
 */
const TIP_RETRIES: u32 = 2;
const TIP_RETRY_DELAY: Duration = Duration::from_millis(250);

/**
 * How long one RPC gets to answer a call before the next is tried. rindexer's client retries a
 * rate limit or a 503 for over an hour without returning, so a load balancer with no healthy
 * node behind it would otherwise hold every call. It matches that client's request timeout.
 */
const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(90);

/** Wordings of an RPC that limits the blocks one `eth_getLogs` may span. */
const RANGE_LIMITS: &[&str] = &[
	"this block range should work",
	"try with this block range",
	"block range is too wide",
	"limited to a",
	"block range too large",
];

/**
 * rindexer's provider, except that `eth_getLogs` asks for every topic the chain's handlers
 * subscribe to. rindexer then runs one stream per chain, and handlers see the chain's logs in
 * block order, as SubQuery delivered them.
 *
 * Every call goes through rindexer's own client, which retries rate limits and times out. The
 * chain has one client per RPC URL, and a call fails over between them: see [`Failover`].
 * Two cases rindexer's live loop can't recover from are handled here: an error on a short
 * range, and an empty answer for the tip block from an RPC that hasn't caught up.
 */
#[derive(Debug)]
pub struct AllTopicsProvider {
	/** rindexer's client for each of the chain's RPCs that answered at startup, with its
	 * position among the chain's URLs. */
	clients: Vec<(usize, Arc<JsonRpcCachedProvider>)>,
	failover: Failover,
	attempt_timeout: Duration,
	topics: Vec<B256>,
	reorg_safe_distance: u64,
	/** The highest tip block seen. A window that ends below it is history, with no tip to
	 * check. */
	tip_seen: AtomicU64,
}

impl AllTopicsProvider {
	/** A provider over `clients`, of which there is at least one. */
	pub fn new(
		clients: Vec<(usize, Arc<JsonRpcCachedProvider>)>,
		topics: Vec<B256>,
		reorg_safe_distance: u64,
	) -> Self {
		assert!(!clients.is_empty(), "a chain needs an RPC");
		Self {
			clients,
			failover: Failover::default(),
			attempt_timeout: ATTEMPT_TIMEOUT,
			topics,
			reorg_safe_distance,
			tip_seen: AtomicU64::new(0),
		}
	}

	/**
	 * Runs `call` with the client of the RPC that answered last, then with each other client in
	 * turn while it fails. The first to answer is tried first from then on. With several RPCs,
	 * each gets [`ATTEMPT_TIMEOUT`]. When none answers, the error is the first RPC's answer
	 * there was, such as a revert or a range limit, which says more than no answer.
	 */
	async fn first_answer<'a, T, F, Fut>(&'a self, call: F) -> Result<T, ProviderError>
	where
		F: Fn(&'a JsonRpcCachedProvider) -> Fut,
		Fut: Future<Output = Result<T, ProviderError>>,
	{
		let several = self.clients.len() > 1;
		let mut failure: Option<ProviderError> = None;
		for index in self.failover.order(self.clients.len()) {
			let (position, client) = &self.clients[index];
			let result = match several {
				false => call(client).await,
				true => tokio::time::timeout(self.attempt_timeout, call(client))
					.await
					.unwrap_or_else(|_| {
						Err(ProviderError::CustomError(format!(
							"no answer in {:?}",
							self.attempt_timeout
						)))
					}),
			};
			match result {
				Ok(value) => {
					self.failover.answered(index);
					return Ok(value);
				},
				Err(err) => {
					if several {
						tracing::warn!(rpc = position, "RPC failed, trying the next: {err}");
					}
					let answered = |err: &ProviderError| matches!(err, ProviderError::RequestFailed(err) if is_answer(err));
					if failure.as_ref().is_none_or(|kept| !answered(kept) && answered(&err)) {
						failure = Some(err);
					}
				},
			}
		}
		Err(failure.expect("a chain has an RPC"))
	}

	/**
	 * The logs `addresses` have from `from` to `to`. A short range an RPC says is too long is
	 * fetched in halves: handed back to rindexer, it could stop the stream (see [`is_short`]).
	 */
	async fn range_logs(
		&self,
		addresses: &[Address],
		from: u64,
		to: u64,
		short: bool,
		tip: Option<&AnyRpcBlock>,
	) -> Result<Vec<Log>, ProviderError> {
		let mut ranges = vec![(from, to)];
		let mut logs = vec![];
		while let Some((from, to)) = ranges.pop() {
			match self.request_logs(addresses, from, to, short, tip).await {
				Ok(found) => logs.extend(found),
				Err(err) if short && from < to && limits_the_range(&err) => {
					let middle = from + (to - from) / 2;
					ranges.push((middle + 1, to));
					ranges.push((from, middle));
				},
				Err(err) => return Err(err),
			}
		}
		Ok(logs)
	}

	/**
	 * One request's logs. On a short range, errors are retried here instead of returned, after
	 * 1 s, doubling up to 16 s, except a range limit that halving can still meet.
	 */
	async fn request_logs(
		&self,
		addresses: &[Address],
		from: u64,
		to: u64,
		short: bool,
		tip: Option<&AnyRpcBlock>,
	) -> Result<Vec<Log>, ProviderError> {
		let request = Filter::new()
			.address(addresses.to_vec())
			.event_signature(self.topics.clone())
			.from_block(from)
			.to_block(to);
		let tip = tip.filter(|tip| tip.header.number == to);
		let (mut errors, mut tip_retries) = (0u32, 0u32);
		loop {
			let answer = self
				.first_answer(|client| {
					let request = &request;
					async move { Ok(client.get_inner_provider().get_logs(request).await?) }
				})
				.await;
			match answer {
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
				Err(err) if short && !(from < to && limits_the_range(&err)) => {
					errors += 1;
					let delay = Duration::from_secs(1 << (errors.min(5) - 1));
					tracing::warn!(from, to, "eth_getLogs failed, retrying in {delay:?}: {err}");
					tokio::time::sleep(delay).await;
				},
				Err(err) => return Err(err),
			}
		}
	}
}

/**
 * Whether rindexer can't recover from an error on this range (v0.43.4, `fetch_logs.rs`).
 * After a live error, rindexer caps its head a little past the range's start until a fetch
 * succeeds: at a step from its fallback table, or two blocks for a one-block range. It only
 * fetches below that cap less the reorg-safe distance, so after an error on a range about as
 * short as twice that distance it never fetches again. With no distance, a one-block range's
 * cap passes the head, and an empty answer then skips the blocks up to it. This covers every
 * distance under 76; the configs use at most 24. Live ranges are this short. History ranges
 * are longer, and their errors go back to rindexer, which shrinks the range.
 */
fn is_short(from: u64, to: u64, reorg_safe_distance: u64) -> bool {
	to.saturating_sub(from) < 2 * (reorg_safe_distance + 2)
}

/** Whether an `eth_getLogs` error says the request spans too many blocks. */
fn limits_the_range(err: &ProviderError) -> bool {
	let text = match err {
		ProviderError::RequestFailed(err) => match err.as_error_resp() {
			Some(answer) => {
				let data = answer.data.as_ref().map(|data| data.get()).unwrap_or_default();
				format!("{} {data}", answer.message)
			},
			None => err.to_string(),
		},
		other => other.to_string(),
	};
	let text = text.to_lowercase();
	RANGE_LIMITS.iter().any(|limit| text.contains(limit))
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
		self.clients[0].1.chain()
	}

	fn max_block_range(&self) -> Option<U64> {
		self.clients[0].1.max_block_range()
	}

	fn chain_state_notification(&self) -> Option<Sender<ChainStateNotification>> {
		self.clients[0].1.chain_state_notification()
	}

	async fn get_latest_block(&self) -> Result<Option<Arc<AnyRpcBlock>>, ProviderError> {
		let block = self.first_answer(|client| client.get_latest_block()).await?;
		if let Some(block) = &block {
			self.tip_seen.fetch_max(block.header.number, Ordering::Relaxed);
		}
		Ok(block)
	}

	async fn get_block_number(&self) -> Result<U64, ProviderError> {
		self.first_answer(|client| client.get_block_number()).await
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
		// A window that reaches the tip, which an RPC behind it answers empty, at any length.
		// History windows end below the tip seen, and skip the lookup.
		let reaches_tip = to >= self.tip_seen.load(Ordering::Relaxed);
		let tip = if reaches_tip { self.get_latest_block().await.ok().flatten() } else { None };
		let tip = tip.as_deref().filter(|tip| tip.header.number == to);
		let requests = addresses
			.chunks(MAX_ADDRESSES_PER_REQUEST)
			.map(|chunk| self.range_logs(chunk, from, to, short, tip));
		let mut logs: Vec<Log> = futures::future::try_join_all(requests).await?.concat();
		logs.sort_by_key(|log| (log.block_number, log.log_index));
		Ok(logs)
	}

	async fn get_block_by_number_batch(
		&self,
		block_numbers: &[U64],
		include_txs: bool,
	) -> Result<Vec<AnyRpcBlock>, ProviderError> {
		self.first_answer(|client| client.get_block_by_number_batch(block_numbers, include_txs))
			.await
	}

	async fn get_block_by_number_batch_with_size(
		&self,
		block_numbers: &[U64],
		include_txs: bool,
		rpc_batch_size: Option<usize>,
	) -> Result<Vec<AnyRpcBlock>, ProviderError> {
		self.first_answer(|client| {
			client.get_block_by_number_batch_with_size(block_numbers, include_txs, rpc_batch_size)
		})
		.await
	}

	async fn get_tx_receipts_batch(
		&self,
		hashes: &[TxHash],
	) -> Result<Vec<AnyTransactionReceipt>, ProviderError> {
		self.first_answer(|client| client.get_tx_receipts_batch(hashes)).await
	}

	async fn trace_block(
		&self,
		block_number: U64,
	) -> Result<Vec<LocalizedTransactionTrace>, ProviderError> {
		self.first_answer(|client| client.trace_block(block_number)).await
	}

	async fn debug_trace_block_by_number(
		&self,
		block_number: U64,
	) -> Result<Vec<LocalizedTransactionTrace>, ProviderError> {
		self.first_answer(|client| client.debug_trace_block_by_number(block_number))
			.await
	}

	async fn eth_call(
		&self,
		to: alloy::primitives::Address,
		data: Bytes,
		block_number: u64,
	) -> Result<String, ProviderError> {
		self.first_answer(|client| client.eth_call(to, data.clone(), block_number))
			.await
	}

	async fn eth_call_latest(
		&self,
		to: alloy::primitives::Address,
		data: Bytes,
	) -> Result<String, ProviderError> {
		self.first_answer(|client| client.eth_call_latest(to, data.clone())).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::failover::tests::{MockRpc, Reply};

	/**
	 * Where rindexer v0.43.4 caps its head, past `from`, after an error on `[from, from +
	 * span]` that names no range: its fallback table's step (`FallbackBlockRange`), one lower
	 * when it equals the span, or two blocks for a one-block range (`halved_block_number`).
	 */
	fn rindexer_cap(span: u64) -> u64 {
		const STEPS: [u64; 14] = [5000, 500, 75, 50, 45, 40, 35, 30, 25, 20, 15, 10, 5, 1];
		if span == 0 {
			return 2;
		}
		let index = STEPS.iter().position(|step| span >= *step).expect("1 is a step");
		match STEPS[index] == span {
			true => STEPS.get(index + 1).copied().unwrap_or(1),
			false => STEPS[index],
		}
	}

	/** Whether rindexer then stops fetching for good, or fetches past the last block it may. */
	fn rindexer_cannot_recover(span: u64, reorg_safe_distance: u64) -> bool {
		let cap = rindexer_cap(span);
		cap < reorg_safe_distance || cap > span + reorg_safe_distance
	}

	#[test]
	fn the_cap_model_matches_rindexers_own_tests() {
		// fetch_logs.rs: (600, 603) is retried as (600, 601), and an unknown error on
		// (100, 10100) as (100, 5100).
		assert_eq!(rindexer_cap(3), 1);
		assert_eq!(rindexer_cap(10_000), 5000);
		assert_eq!(rindexer_cap(0), 2);
	}

	#[test]
	fn every_range_rindexer_cannot_recover_from_is_short() {
		for distance in 0..76 {
			for span in 0..6000 {
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

	async fn client(url: &str) -> (usize, Arc<JsonRpcCachedProvider>) {
		let headers = rindexer::HeaderMap::new();
		let client =
			rindexer::provider::create_client(url, 1, None, None, None, headers, None, None);
		(0, client.await.unwrap())
	}

	#[tokio::test]
	async fn calls_move_past_an_rpc_that_went_down() {
		let first = MockRpc::start(Reply::Result("0x1")).await;
		let second = MockRpc::start(Reply::Result("0x1")).await;
		let clients = vec![client(&first.url).await, client(&second.url).await];
		// The first RPC answered at startup, then went down.
		first.server.abort();
		let _ = first.server.await;
		let provider = AllTopicsProvider::new(clients, vec![], 0);
		assert_eq!(provider.get_block_number().await.unwrap(), U64::from(1));
		assert_eq!(provider.failover.order(2).collect::<Vec<_>>(), [1, 0]);
	}

	#[tokio::test]
	async fn calls_move_past_an_rpc_whose_answers_rindexer_keeps_retrying() {
		let first = MockRpc::start(Reply::Result("0x1")).await;
		let second = MockRpc::start(Reply::Result("0x1")).await;
		let clients = vec![client(&first.url).await, client(&second.url).await];
		// A load balancer with no node behind it: rindexer's client retries its 503s for an hour.
		first.set(Reply::Status(503));
		let mut provider = AllTopicsProvider::new(clients, vec![], 0);
		provider.attempt_timeout = Duration::from_millis(500);
		assert_eq!(provider.get_block_number().await.unwrap(), U64::from(1));
		assert_eq!(provider.failover.order(2).collect::<Vec<_>>(), [1, 0]);
	}

	#[tokio::test]
	async fn a_short_range_an_rpc_says_is_too_long_is_fetched_in_halves() {
		let limited = MockRpc::start(Reply::LogsWithin(3)).await;
		let provider = AllTopicsProvider::new(vec![client(&limited.url).await], vec![], 4);
		let address = [Address::repeat_byte(0x11)];
		assert!(is_short(100, 109, 4));
		let logs = tokio::time::timeout(
			Duration::from_secs(10),
			provider.range_logs(&address, 100, 109, true, None),
		)
		.await
		.expect("halving meets the limit");
		assert!(logs.unwrap().is_empty());
		// A history range goes back to rindexer, which shrinks it itself.
		assert!(provider.range_logs(&address, 0, 5_000, false, None).await.is_err());
	}

	#[test]
	fn errors_that_limit_the_range_are_recognised() {
		let custom = |message: &str| limits_the_range(&ProviderError::CustomError(message.into()));
		assert!(custom("this block range should work: [0x100, 0x109]"));
		assert!(custom("eth_getLogs is limited to a 10,000 block range"));
		assert!(custom("block range too large"));
		assert!(!custom("rate limit exceeded"));
		assert!(!custom("error sending request for url: connection refused"));
		// A hint carried in the answer's data.
		let answer = serde_json::from_str(
			r#"{"code":-32000,"message":"query exceeds limits","data":"try with this block range [0x1, 0x2]"}"#,
		)
		.unwrap();
		let err = ProviderError::RequestFailed(alloy::transports::RpcError::ErrorResp(answer));
		assert!(limits_the_range(&err));
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
