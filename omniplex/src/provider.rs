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
 * Re-asks of a range whose tip block came back without our logs although the block's bloom
 * says it can have them, when the tip's logs asked by its hash don't settle it: they are there,
 * or the RPC takes no hash filter, or the request fails. Also the re-reads of a tip block a
 * node answers without. Each wait is 250 ms longer.
 */
const TIP_RETRIES: u32 = 2;
const TIP_RETRY_DELAY: Duration = Duration::from_millis(250);

/**
 * How long one RPC gets to answer a call before the next is tried. rindexer's client retries a
 * rate limit or a 503 for over an hour without returning, so a load balancer with no healthy
 * node behind it would otherwise hold every call. It matches that client's request timeout.
 *
 * Two things a timeout leaves behind, both in rindexer's client, which this crate can't change:
 * alloy's retry layer counts each call it starts until it returns, so every call cut short adds
 * one for good, and past about 30 the node's later rate-limit retries wait longer; and a block
 * batch's spawned chunk requests keep running against that node.
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
		let (mut errors, mut tip_checks) = (0u32, 0u32);
		loop {
			let answer = self
				.first_answer(|client| {
					let request = &request;
					async move { Ok(client.get_inner_provider().get_logs(request).await?) }
				})
				.await;
			match answer {
				Ok(logs) => {
					let Some(tip) = tip.filter(|tip| {
						let (bloom, number) = (&tip.header.logs_bloom, tip.header.number);
						lacks_tip_logs(bloom, number, addresses, &self.topics, &logs)
					}) else {
						return Ok(logs);
					};
					tip_checks += 1;
					let retry = tip_checks <= TIP_RETRIES;
					match self.tip_logs(addresses, tip).await {
						// The bloom matched other logs, so the tip has none of ours.
						Ok(Some(tip_logs)) if tip_logs.is_empty() => return Ok(logs),
						// A node behind the tip answered the range. Another answer can come from
						// a node with the tip, and so with the whole range.
						Ok(Some(_)) if retry =>
							tracing::warn!(to, "eth_getLogs missed the tip's logs; asking again"),
						// The range kept missing the tip's logs, so the hash's answer supplies
						// them, unless the tip has been replaced since it was read.
						Ok(Some(tip_logs)) => {
							let replaced = self
								.block_at(to)
								.await
								.is_some_and(|block| block.header.hash != tip.header.hash);
							if replaced {
								return Ok(logs);
							}
							return Ok(logs.into_iter().chain(tip_logs).collect());
						},
						Err(err) if retry =>
							tracing::warn!(to, "the tip's logs by hash failed; asking again: {err}"),
						Ok(None) if retry => {},
						// Nothing settled the tip, so the answer stands, as rindexer took it.
						Ok(None) | Err(_) => return Ok(logs),
					}
					tokio::time::sleep(TIP_RETRY_DELAY * tip_checks).await;
				},
				Err(err) if short && !(from < to && limits_the_range(&err)) => {
					errors += 1;
					let delay = retry_delay(errors);
					tracing::warn!(from, to, "eth_getLogs failed, retrying in {delay:?}: {err}");
					tokio::time::sleep(delay).await;
				},
				Err(err) => return Err(err),
			}
		}
	}

	/**
	 * The tip block's logs, asked by its hash. A node without the block fails such a request
	 * (EIP-234) instead of answering empty, so an answer is definitive. `None` when the RPC
	 * takes no hash filter, as substrate-eth-rpc, which refuses a range past its head instead.
	 */
	async fn tip_logs(
		&self,
		addresses: &[Address],
		tip: &AnyRpcBlock,
	) -> Result<Option<Vec<Log>>, ProviderError> {
		let request = Filter::new()
			.address(addresses.to_vec())
			.event_signature(self.topics.clone())
			.at_block_hash(tip.header.hash);
		let answer = self
			.first_answer(|client| {
				let request = &request;
				async move { Ok(client.get_inner_provider().get_logs(request).await?) }
			})
			.await;
		match answer {
			Ok(logs) => Ok(Some(logs)),
			Err(err) if refuses_hash_filters(&err) => Ok(None),
			Err(err) => Err(err),
		}
	}

	/**
	 * Block `to`, for the tip check. rindexer has just read it as the head, and its client
	 * keeps the head for 50 ms, so the head is usually it. When the head has moved on, or came
	 * from a node behind it, the block is read by its number.
	 *
	 * Without the block there is no check, and a node behind the tip could answer the range
	 * empty. So a short range waits for the block, as it waits out a failed `eth_getLogs`, and
	 * a longer one fails.
	 */
	async fn tip_block(&self, to: u64, short: bool) -> Result<Arc<AnyRpcBlock>, ProviderError> {
		if let Ok(Some(head)) = self.get_latest_block().await {
			if head.header.number == to {
				return Ok(head);
			}
		}
		let mut misses = 0u32;
		loop {
			if let Some(block) = self.block_at(to).await {
				return Ok(block);
			}
			if !short {
				return Err(ProviderError::CustomError(format!("no RPC has the tip block {to}")));
			}
			misses += 1;
			let delay = retry_delay(misses);
			tracing::warn!(to, "no RPC has the tip block, reading it again in {delay:?}");
			tokio::time::sleep(delay).await;
		}
	}

	/**
	 * Block `number`, read by its number. A node behind it answers without it, which counts as
	 * a failure, so the next RPC is asked. The block is read again up to [`TIP_RETRIES`] times.
	 */
	async fn block_at(&self, number: u64) -> Option<Arc<AnyRpcBlock>> {
		for attempt in 1..=TIP_RETRIES + 1 {
			let answer = self
				.first_answer(|client| async move {
					let block =
						client.get_inner_provider().get_block_by_number(number.into()).await?;
					block.map(Arc::new).ok_or_else(|| {
						ProviderError::CustomError(format!("block {number} not found"))
					})
				})
				.await;
			match answer {
				Ok(block) => return Some(block),
				Err(err) => tracing::warn!(number, "reading the block failed: {err}"),
			}
			if attempt <= TIP_RETRIES {
				tokio::time::sleep(TIP_RETRY_DELAY * attempt).await;
			}
		}
		None
	}
}

/** The wait before a short range's retry after this many failures: 1 s, doubling up to 16 s. */
fn retry_delay(failures: u32) -> Duration {
	Duration::from_secs(1 << (failures.clamp(1, 5) - 1))
}

/** Whether an RPC refused an `eth_getLogs` by block hash as a filter it doesn't take. */
fn refuses_hash_filters(err: &ProviderError) -> bool {
	let ProviderError::RequestFailed(err) = err else { return false };
	err.as_error_resp().is_some_and(|answer| {
		let message = answer.message.to_lowercase();
		answer.code == -32602 &&
			(message.contains("not supported") || message.contains("blockhash"))
	})
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
		let tip = if reaches_tip { Some(self.tip_block(to, short).await?) } else { None };
		let requests = addresses
			.chunks(MAX_ADDRESSES_PER_REQUEST)
			.map(|chunk| self.range_logs(chunk, from, to, short, tip.as_deref()));
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
	use std::sync::atomic::AtomicUsize;

	use alloy::rpc::types::ValueOrArray;
	use rindexer::event::contract_setup::AddressDetails;

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
		// Long enough for the healthy node on a busy machine, well short of rindexer's retries.
		provider.attempt_timeout = Duration::from_secs(2);
		assert_eq!(provider.get_block_number().await.unwrap(), U64::from(1));
		assert_eq!(provider.failover.order(2).collect::<Vec<_>>(), [1, 0]);
	}

	#[tokio::test]
	async fn with_no_rpc_answering_well_the_answer_beats_no_answer() {
		let first = MockRpc::start(Reply::Result("0x1")).await;
		let second = MockRpc::start(Reply::Result("0x1")).await;
		let clients = vec![client(&first.url).await, client(&second.url).await];
		first.server.abort();
		let _ = first.server.await;
		second.set(Reply::Error(-32602, "invalid params"));
		let provider = AllTopicsProvider::new(clients, vec![], 0);
		// The first RPC is tried first and doesn't answer; the second's answer is the error.
		let err = provider.get_block_number().await.unwrap_err();
		let answered =
			matches!(&err, ProviderError::RequestFailed(err) if err.as_error_resp().is_some());
		assert!(answered, "{err}");
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

	/** A tip block whose bloom holds `address` and `topic`. */
	fn tip_block(number: u64, address: Address, topic: B256) -> AnyRpcBlock {
		block_with_hash(number, B256::repeat_byte(0x77), address, topic)
	}

	fn block_with_hash(number: u64, hash: B256, address: Address, topic: B256) -> AnyRpcBlock {
		let mut bloom = Bloom::default();
		bloom.accrue(BloomInput::Raw(address.as_slice()));
		bloom.accrue(BloomInput::Raw(topic.as_slice()));
		let zero = B256::ZERO;
		serde_json::from_value(serde_json::json!({
			"hash": hash, "parentHash": zero, "sha3Uncles": zero,
			"miner": Address::ZERO, "stateRoot": zero, "transactionsRoot": zero,
			"receiptsRoot": zero, "logsBloom": bloom, "difficulty": "0x0",
			"number": format!("{number:#x}"), "gasLimit": "0x0", "gasUsed": "0x0",
			"timestamp": "0x0", "extraData": "0x", "mixHash": zero,
			"nonce": "0x0000000000000000", "transactions": [], "uncles": [],
		}))
		.unwrap()
	}

	/** One log of `address` with `topic` in `tip`, as an RPC sends it. */
	fn tip_log(tip: &AnyRpcBlock, address: Address, topic: B256) -> &'static str {
		let log = serde_json::json!({
			"address": address, "topics": [topic], "data": "0x",
			"blockNumber": format!("{:#x}", tip.header.number), "blockHash": tip.header.hash,
			"transactionHash": B256::repeat_byte(0x88), "transactionIndex": "0x0",
			"logIndex": "0x0", "removed": false,
		});
		Box::leak(log.to_string().into_boxed_str())
	}

	fn leak_json(block: &AnyRpcBlock) -> &'static str {
		Box::leak(serde_json::to_string(block).unwrap().into_boxed_str())
	}

	/**
	 * A tip check against an RPC whose range answers miss the tip's logs `misses` times. When
	 * `replaced`, another block now has the tip's height.
	 */
	async fn tip_check(
		misses: usize,
		hash_error: Option<(i64, &'static str)>,
		replaced: bool,
	) -> (usize, usize) {
		let (address, topic) = (Address::repeat_byte(0x11), B256::repeat_byte(0x22));
		let tip = tip_block(10, address, topic);
		let canonical = match replaced {
			true => leak_json(&block_with_hash(10, B256::repeat_byte(0x99), address, topic)),
			false => leak_json(&tip),
		};
		let misses = Arc::new(AtomicUsize::new(misses));
		let log = tip_log(&tip, address, topic);
		let reply = Reply::TipLogs { log, misses: misses.clone(), hash_error, canonical };
		let rpc = MockRpc::start(reply).await;
		let provider = AllTopicsProvider::new(vec![client(&rpc.url).await], vec![topic], 0);
		let logs = tokio::time::timeout(
			Duration::from_secs(20),
			provider.range_logs(&[address], 9, 10, true, Some(&tip)),
		)
		.await
		.expect("the tip check ends")
		.unwrap();
		(logs.len(), misses.load(Ordering::SeqCst))
	}

	/**
	 * A node behind the tip answers the range without the tip's logs. Asked by hash, the tip
	 * has them, so the range is asked again, and the next answer has them.
	 */
	#[tokio::test]
	async fn a_range_answer_missing_the_tips_logs_is_asked_again() {
		assert_eq!(tip_check(1, None, false).await, (1, 0));
	}

	/**
	 * After two more answers without them, the tip's logs come from the hash's answer. Not if
	 * another block has the tip's height by then: the range's answers had it right.
	 */
	#[tokio::test]
	async fn a_range_that_keeps_missing_the_tips_logs_takes_them_by_hash() {
		assert_eq!(tip_check(10, None, false).await, (1, 7), "the range was asked three times");
		assert_eq!(tip_check(10, None, true).await, (0, 7));
	}

	/**
	 * An RPC that takes no hash filter, or fails the request in a way not known, gets the range
	 * asked twice more, then its answer stands.
	 */
	#[tokio::test]
	async fn an_unsettled_tip_is_asked_twice_more() {
		let refused = (-32602, "blockHash filter not supported");
		assert_eq!(tip_check(5, Some(refused), false).await, (0, 2));
		assert_eq!(tip_check(5, Some((-32000, "unknown block")), false).await, (0, 2));
	}

	/** A node behind the tip answers without the block, so it's read again, up to twice. */
	#[tokio::test]
	async fn the_tip_block_is_read_again_until_a_node_has_it() {
		let (address, topic) = (Address::repeat_byte(0x11), B256::repeat_byte(0x22));
		let block = leak_json(&tip_block(10, address, topic));
		for (nulls, found) in [(2, true), (3, false)] {
			let rpc = MockRpc::start(Reply::Block { block, nulls: Arc::new(nulls.into()) }).await;
			let provider = AllTopicsProvider::new(vec![client(&rpc.url).await], vec![topic], 0);
			let tip = provider.block_at(10).await;
			assert_eq!(tip.map(|tip| tip.header.number), found.then_some(10), "{nulls} nulls");
		}
	}

	/** With no node having the tip block, a long range fails and a short one waits for it. */
	#[tokio::test]
	async fn a_range_is_not_fetched_without_its_tip_block() {
		let (address, topic) = (Address::repeat_byte(0x11), B256::repeat_byte(0x22));
		let block = leak_json(&tip_block(10, address, topic));
		let details =
			AddressDetails { address: ValueOrArray::Value(address), indexed_filters: None };
		let range_from = |from: u64| {
			let (from, to) = (U64::from(from), U64::from(10));
			RindexerEventFilter::new_address_filter(&topic, "Event", &details, from, to).unwrap()
		};
		// Each call reads the head once, then the block three times.
		let rpc = MockRpc::start(Reply::Block { block, nulls: Arc::new(8.into()) }).await;
		let provider = AllTopicsProvider::new(vec![client(&rpc.url).await], vec![topic], 0);
		let err = provider.get_logs(&range_from(1)).await.unwrap_err();
		assert!(err.to_string().ends_with("no RPC has the tip block 10"), "{err}");
		let logs = tokio::time::timeout(Duration::from_secs(20), provider.get_logs(&range_from(9)))
			.await
			.expect("the node catches up");
		assert!(logs.unwrap().is_empty());
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
