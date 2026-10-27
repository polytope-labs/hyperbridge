use std::{
	collections::{hash_map::Entry, HashMap},
	future::Future,
	sync::{Arc, Mutex},
	time::{Duration, SystemTime, UNIX_EPOCH},
};

use alloy_primitives::keccak256;
use codec::Decode;
use futures::{
	future::{BoxFuture, Shared},
	stream, FutureExt, StreamExt, TryStreamExt,
};
use tokio::{
	sync::{Semaphore, SemaphorePermit},
	time::Instant,
};

use crate::{
	bloom::block_bloom,
	cache::{BlockCache, FinalizedCache},
	config::ChainConfig,
	decode::{decode_events, extrinsic_hashes, DecodeError, MetadataCache},
	mapping::{pallets_for, Mapper, MappingError},
	substrate::{system_events_key, timestamp_now_key, RawBlock, RpcError, SubstrateRpc},
	types::{DecodedBlock, EthLog, Phase, H256},
};

const DECODE_CONCURRENCY: usize = 16;
const MAX_BACKOFF: Duration = Duration::from_secs(5);

#[derive(Debug, thiserror::Error)]
pub enum BlockError {
	#[error(transparent)]
	Rpc(#[from] RpcError),
	#[error(transparent)]
	Decode(#[from] DecodeError),
	#[error(transparent)]
	Mapping(#[from] MappingError),
	#[error("block {0} not found")]
	NotFound(u64),
	#[error("block {number}: {source}")]
	At {
		number: u64,
		#[source]
		source: Box<BlockError>,
	},
	#[error("missing System.Events at block {0}")]
	MissingEvents(u64),
	#[error("missing Timestamp.Now at block {0}")]
	MissingTimestamp(u64),
	#[error("fromBlock {from} is after toBlock {to}")]
	InvalidRange { from: u64, to: u64 },
	#[error("block range {from}..={to} exceeds max {max}")]
	RangeTooLarge { from: u64, to: u64, max: u64 },
	#[error("block range {from}..={to} missed the deadline, decoded up to {decoded_to:?}")]
	RangeDeadline { from: u64, to: u64, decoded_to: Option<u64> },
	#[error("toBlock {to} is beyond finalized head {head}")]
	BeyondHead { to: u64, head: u64 },
	#[error("decode task: {0}")]
	Task(String),
	#[error(transparent)]
	Shared(Arc<BlockError>),
}

impl BlockError {
	fn at(self, number: u64) -> Self {
		match self {
			BlockError::At { .. } => self,
			source => BlockError::At { number, source: Box::new(source) },
		}
	}
}

type InFlight = Shared<BoxFuture<'static, Result<Arc<DecodedBlock>, Arc<BlockError>>>>;

pub struct BlockService {
	rpc: Arc<dyn SubstrateRpc>,
	chain: ChainConfig,
	mapper: Arc<Mapper>,
	metadata: MetadataCache,
	cache: BlockCache,
	finalized: FinalizedCache,
	in_flight: Mutex<HashMap<H256, InFlight>>,
	upstream: Semaphore,
	cooldown_until: Mutex<Option<Instant>>,
	concurrency: usize,
}

impl BlockService {
	pub fn new(rpc: Arc<dyn SubstrateRpc>, chain: ChainConfig) -> Result<Self, BlockError> {
		let mapper = Arc::new(Mapper::new(pallets_for(&chain.pallets)?));
		let cache = BlockCache::new(chain.cache_blocks);
		let upstream = Semaphore::new(chain.upstream_concurrency.max(1));
		Ok(Self {
			rpc,
			chain,
			mapper,
			metadata: MetadataCache::new(),
			cache,
			finalized: FinalizedCache::default(),
			in_flight: Mutex::new(HashMap::new()),
			upstream,
			cooldown_until: Mutex::new(None),
			concurrency: DECODE_CONCURRENCY,
		})
	}

	pub fn chain(&self) -> &ChainConfig {
		&self.chain
	}

	pub fn mapper(&self) -> &Mapper {
		&self.mapper
	}

	pub async fn finalized_number(&self) -> Result<u64, BlockError> {
		let (number, _) = self
			.finalized
			.get_or_refresh(async {
				let hash = self.rpc.finalized_head().await?;
				let header = self.rpc.header(hash).await?.ok_or(RpcError::HashNotFound(hash))?;
				Ok::<_, BlockError>((header.number, hash))
			})
			.await?;
		Ok(number)
	}

	pub async fn block_by_number(
		self: &Arc<Self>,
		number: u64,
	) -> Result<Option<Arc<DecodedBlock>>, BlockError> {
		if number > self.finalized_number().await? {
			return Ok(None);
		}
		if let Some(block) = self.cache.get_by_number(number) {
			return Ok(Some(block));
		}
		let Some(hash) = self
			.upstream(&format!("block {number}"), || self.rpc.block_hash(number))
			.await
			.map_err(|e| BlockError::from(e).at(number))?
		else {
			return Ok(None);
		};
		self.fetch_cached(hash).await.map(Some).map_err(|e| e.at(number))
	}

	pub async fn block_by_hash(
		self: &Arc<Self>,
		hash: H256,
	) -> Result<Option<Arc<DecodedBlock>>, BlockError> {
		if let Some(block) = self.cache.get_by_hash(hash) {
			return Ok(Some(block));
		}
		let Some(number) = self.canonical_number(hash).await? else {
			return Ok(None);
		};
		self.fetch_cached(hash).await.map(Some).map_err(|e| e.at(number))
	}

	pub async fn canonical_number(&self, hash: H256) -> Result<Option<u64>, BlockError> {
		if let Some(block) = self.cache.get_by_hash(hash) {
			return Ok(Some(block.number));
		}
		let Some(header) = self.rpc.header(hash).await? else {
			return Ok(None);
		};
		let number = header.number;
		if number > self.finalized_number().await? {
			return Ok(None);
		}
		let canonical = self
			.upstream(&format!("block {number}"), || self.rpc.block_hash(number))
			.await
			.map_err(|e| BlockError::from(e).at(number))?;
		Ok((canonical == Some(hash)).then_some(number))
	}

	pub async fn range(
		self: &Arc<Self>,
		from: u64,
		to: u64,
	) -> Result<Vec<Arc<DecodedBlock>>, BlockError> {
		if from > to {
			return Err(BlockError::InvalidRange { from, to });
		}
		let max = self.chain.max_block_range;
		if to - from > max {
			return Err(BlockError::RangeTooLarge { from, to, max });
		}
		let head = self.finalized_number().await?;
		if to > head {
			return Err(BlockError::BeyondHead { to, head });
		}

		let mut blocks = Vec::with_capacity((to - from + 1) as usize);
		let mut missing = Vec::new();
		for number in from..=to {
			match self.cache.get_by_number(number) {
				Some(block) => blocks.push(block),
				None => missing.push(number),
			}
		}

		let decode = stream::iter(missing)
			.map(|number| self.fetch_number(number))
			.buffer_unordered(self.concurrency)
			.try_collect::<Vec<Arc<DecodedBlock>>>();
		let deadline = Duration::from_millis(self.chain.range_deadline_ms);
		// Decodes already running keep going as detached tasks and land in the cache, so the
		// hint points the client at the prefix it can get instantly on its retry.
		let fetched = match tokio::time::timeout(deadline, decode).await {
			Ok(fetched) => fetched?,
			Err(_) => {
				let decoded_to = self.cached_prefix_end(from, to);
				tracing::debug!(from, to, decoded_to, "range deadline reached");
				return Err(BlockError::RangeDeadline { from, to, decoded_to });
			},
		};
		blocks.extend(fetched);
		blocks.sort_by_key(|block| block.number);
		Ok(blocks)
	}

	pub async fn storage(&self, key: &[u8], at: u64) -> Result<Option<Vec<u8>>, BlockError> {
		let hash = self.canonical_hash(at).await?;
		self.rpc.storage(key, hash).await.map_err(|e| BlockError::from(e).at(at))
	}

	pub async fn child_storage(
		&self,
		child_key: &[u8],
		key: &[u8],
		at: u64,
	) -> Result<Option<Vec<u8>>, BlockError> {
		let hash = self.canonical_hash(at).await?;
		self.rpc
			.child_storage(child_key, key, hash)
			.await
			.map_err(|e| BlockError::from(e).at(at))
	}

	fn cached_prefix_end(&self, from: u64, to: u64) -> Option<u64> {
		(from..=to)
			.take_while(|number| self.cache.get_by_number(*number).is_some())
			.last()
	}

	async fn canonical_hash(&self, number: u64) -> Result<H256, BlockError> {
		if let Some(block) = self.cache.get_by_number(number) {
			return Ok(block.hash);
		}
		self.upstream(&format!("block {number}"), || self.rpc.block_hash(number))
			.await
			.map_err(|e| BlockError::from(e).at(number))?
			.ok_or(BlockError::NotFound(number))
	}

	async fn fetch_number(self: &Arc<Self>, number: u64) -> Result<Arc<DecodedBlock>, BlockError> {
		let hash = self.canonical_hash(number).await?;
		self.fetch_cached(hash).await.map_err(|e| e.at(number))
	}

	async fn fetch_cached(self: &Arc<Self>, hash: H256) -> Result<Arc<DecodedBlock>, BlockError> {
		let pending = {
			let mut in_flight = self.in_flight.lock().unwrap_or_else(|e| e.into_inner());
			if let Some(block) = self.cache.get_by_hash(hash) {
				return Ok(block);
			}
			match in_flight.entry(hash) {
				Entry::Occupied(entry) => entry.get().clone(),
				Entry::Vacant(entry) => entry.insert(self.spawn_decode(hash)).clone(),
			}
		};
		pending.await.map_err(BlockError::Shared)
	}

	// A decode runs as its own task so a caller that stops waiting for it, such as a range
	// that hit its deadline, still leaves the block in the cache for the retry.
	fn spawn_decode(self: &Arc<Self>, hash: H256) -> InFlight {
		let this = self.clone();
		tokio::spawn(async move {
			let mut guard = DecodeGuard { service: this.clone(), hash, block: None };
			let result = this
				.decode_block(hash)
				.await
				.and_then(|block| block.ok_or(RpcError::HashNotFound(hash).into()))
				.map_err(Arc::new);
			guard.block = result.as_ref().ok().cloned();
			drop(guard);
			result
		})
		.map(|joined| joined.unwrap_or_else(|err| Err(Arc::new(BlockError::Task(err.to_string())))))
		.boxed()
		.shared()
	}

	async fn decode_block(&self, hash: H256) -> Result<Option<Arc<DecodedBlock>>, BlockError> {
		let permit = self.acquire_upstream().await;
		let Some(raw) = self.upstream(&format!("block {hash}"), || self.rpc.block(hash)).await?
		else {
			return Ok(None);
		};
		let number = raw.number;
		let at = format!("block {number}");
		// System.Events at `hash` were written by the runtime live at the parent's state; the
		// block that applies an upgrade already reports the new version at its own hash.
		let spec_at = if number == 0 { hash } else { raw.parent_hash };
		let events_key = system_events_key();
		let timestamp_key = timestamp_now_key();
		let (version, events, timestamp) = tokio::try_join!(
			self.upstream(&at, || self.rpc.runtime_version(spec_at)),
			self.upstream(&at, || self.rpc.storage(&events_key, hash)),
			self.upstream(&at, || self.rpc.storage(&timestamp_key, hash)),
		)?;
		let events = events.ok_or(BlockError::MissingEvents(number))?;
		let timestamp = timestamp.ok_or(BlockError::MissingTimestamp(number))?;
		let timestamp_ms = u64::decode(&mut &timestamp[..])
			.map_err(|e| RpcError::Decode(format!("Timestamp.Now at block {number}: {e}")))?;

		let metadata = match self.metadata.get(version.spec_version) {
			Some(metadata) => metadata,
			None => {
				let raw_metadata = self.upstream(&at, || self.rpc.metadata(spec_at)).await?;
				self.metadata.insert(version.spec_version, &raw_metadata)?
			},
		};
		drop(permit);

		let block =
			self.build_block(raw, version.spec_version, &events, timestamp_ms, &metadata)?;
		Ok(Some(Arc::new(block)))
	}

	// During a rate-limit cooldown every fetch holds two permits, which halves the number of
	// blocks in flight against the upstream.
	async fn acquire_upstream(&self) -> SemaphorePermit<'_> {
		let permits = if self.cooling_down() { 2.min(self.chain.upstream_concurrency) } else { 1 };
		self.upstream
			.acquire_many(permits as u32)
			.await
			.expect("upstream semaphore is never closed")
	}

	fn cooling_down(&self) -> bool {
		self.cooldown_until
			.lock()
			.unwrap_or_else(|e| e.into_inner())
			.is_some_and(|until| Instant::now() < until)
	}

	fn note_rate_limit(&self) {
		let cooldown = Duration::from_millis(self.chain.rate_limit_cooldown_ms);
		*self.cooldown_until.lock().unwrap_or_else(|e| e.into_inner()) =
			Some(Instant::now() + cooldown);
	}

	async fn upstream<T, F, Fut>(&self, at: &str, call: F) -> Result<T, RpcError>
	where
		F: Fn() -> Fut,
		Fut: Future<Output = Result<T, RpcError>>,
	{
		let retries = self.chain.upstream_retries;
		let mut attempt = 0;
		loop {
			let err = match call().await {
				Ok(value) => return Ok(value),
				Err(err) => err,
			};
			if err.is_rate_limited() {
				self.note_rate_limit();
			}
			if !err.is_transient() || attempt >= retries {
				return Err(err);
			}
			attempt += 1;
			let delay = backoff(self.chain.upstream_backoff_ms, attempt);
			tracing::warn!(
				at,
				attempt,
				retries,
				delay_ms = delay.as_millis() as u64,
				%err,
				"retrying upstream call"
			);
			tokio::time::sleep(delay).await;
		}
	}

	fn build_block(
		&self,
		raw: RawBlock,
		spec_version: u32,
		events: &[u8],
		timestamp_ms: u64,
		metadata: &subxt_core::Metadata,
	) -> Result<DecodedBlock, BlockError> {
		let hashes = extrinsic_hashes(self.chain.hasher, &raw.extrinsics);
		let timestamp = timestamp_ms / 1000;
		let init_hash = synthetic_tx_hash(raw.hash, b"init");
		let final_hash = synthetic_tx_hash(raw.hash, b"final");

		let mut logs = Vec::new();
		for event in decode_events(metadata, events)? {
			let Some(mapped) = self.mapper.map_event(&event)? else {
				continue;
			};
			let (transaction_index, transaction_hash) = match event.phase {
				Phase::ApplyExtrinsic(index) => {
					let hash = hashes.get(index as usize).copied().ok_or_else(|| {
						RpcError::Decode(format!(
							"event {} at block {} references extrinsic {index} of {}",
							event.event_index,
							raw.number,
							hashes.len()
						))
					})?;
					(index as u64, hash)
				},
				Phase::Initialization => (hashes.len() as u64, init_hash),
				Phase::Finalization => (hashes.len() as u64 + 1, final_hash),
			};
			logs.push(EthLog {
				address: mapped.address,
				topics: mapped.topics,
				data: mapped.data,
				block_hash: raw.hash,
				block_number: raw.number,
				block_timestamp: timestamp,
				transaction_hash,
				transaction_index,
				log_index: event.event_index as u64,
			});
		}

		let logs_bloom = block_bloom(&logs);
		Ok(DecodedBlock {
			hash: raw.hash,
			parent_hash: raw.parent_hash,
			number: raw.number,
			timestamp,
			timestamp_ms,
			state_root: raw.state_root,
			extrinsics_root: raw.extrinsics_root,
			spec_version,
			extrinsic_hashes: hashes,
			logs,
			logs_bloom,
		})
	}
}

// Caching the block and clearing the in-flight entry happen under one lock so a request
// sees either the cache hit or the pending decode; the guard also clears the entry when
// the decode panics, so the next request retries instead of inheriting a dead task.
struct DecodeGuard {
	service: Arc<BlockService>,
	hash: H256,
	block: Option<Arc<DecodedBlock>>,
}

impl Drop for DecodeGuard {
	fn drop(&mut self) {
		let mut in_flight = self.service.in_flight.lock().unwrap_or_else(|e| e.into_inner());
		if let Some(block) = self.block.take() {
			self.service.cache.insert(block);
		}
		in_flight.remove(&self.hash);
	}
}

fn backoff(base_ms: u64, attempt: u32) -> Duration {
	let exponent = attempt.saturating_sub(1).min(16);
	let delay = Duration::from_millis(base_ms.saturating_mul(1 << exponent)).min(MAX_BACKOFF);
	let spread = delay.as_millis() as u64 / 4;
	let jitter = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|now| u64::from(now.subsec_nanos()) % (spread + 1))
		.unwrap_or(0);
	delay + Duration::from_millis(jitter)
}

fn synthetic_tx_hash(block_hash: H256, tag: &[u8]) -> H256 {
	let mut preimage = block_hash.to_vec();
	preimage.extend_from_slice(tag);
	keccak256(preimage)
}

#[cfg(test)]
mod tests {
	use std::{
		collections::{BTreeMap, HashMap},
		sync::atomic::{AtomicUsize, Ordering},
	};

	use alloy_primitives::{Bloom, Bytes};
	use async_trait::async_trait;
	use codec::Encode;

	use super::*;
	use crate::{
		decode::testdata::NEXUS,
		substrate::{
			fixture::FixtureRuntimeVersion, BlockHeader, FixtureBlock, FixtureRpc, RuntimeVersion,
		},
		types::{pallet_address, Hasher},
	};

	const METADATA: &[u8] = NEXUS.metadata;
	const EVENTS: &[u8] = NEXUS.events;

	const SPEC_VERSION: u32 = 8700;
	const TIMESTAMP_MS: u64 = 1_700_000_123_456;
	const FINALIZED: u64 = 102;

	fn hash_of(number: u64) -> H256 {
		H256::repeat_byte(number as u8)
	}

	fn extrinsics() -> Vec<Bytes> {
		NEXUS.extrinsics().into_iter().map(Bytes::from).collect()
	}

	fn fixture_block(number: u64, events: Option<&[u8]>, timestamp: Option<u64>) -> FixtureBlock {
		let mut storage = BTreeMap::new();
		if let Some(events) = events {
			storage.insert(Bytes::from(system_events_key()), Bytes::from(events.to_vec()));
		}
		if let Some(timestamp) = timestamp {
			storage.insert(Bytes::from(timestamp_now_key()), Bytes::from(timestamp.encode()));
		}
		FixtureBlock {
			hash: hash_of(number),
			parent_hash: hash_of(number - 1),
			number,
			state_root: H256::repeat_byte(0x33),
			extrinsics_root: H256::repeat_byte(0x44),
			extrinsics: extrinsics(),
			storage,
			child_storage: BTreeMap::new(),
			runtime_version: version(SPEC_VERSION),
			parent_runtime_version: None,
		}
	}

	fn version(spec_version: u32) -> FixtureRuntimeVersion {
		FixtureRuntimeVersion { spec_name: "nexus".into(), spec_version, transaction_version: 1 }
	}

	fn empty_events() -> Vec<u8> {
		Vec::<u8>::new().encode()
	}

	fn root(err: &BlockError) -> &BlockError {
		match err {
			BlockError::At { source, .. } => root(source),
			BlockError::Shared(source) => root(source),
			other => other,
		}
	}

	fn number_of(hash: H256) -> u64 {
		hash[0] as u64
	}

	struct Wrapped {
		inner: Arc<FixtureRpc>,
		block_calls: AtomicUsize,
		fork_at: Option<u64>,
		transient_failures: Mutex<HashMap<u64, u32>>,
		transient_hits: AtomicUsize,
		delays: Mutex<HashMap<u64, Duration>>,
	}

	impl Wrapped {
		fn new(inner: Arc<FixtureRpc>) -> Arc<Self> {
			Arc::new(Self::wrap(inner))
		}

		fn wrap(inner: Arc<FixtureRpc>) -> Self {
			Self {
				inner,
				block_calls: AtomicUsize::new(0),
				fork_at: None,
				transient_failures: Mutex::new(HashMap::new()),
				transient_hits: AtomicUsize::new(0),
				delays: Mutex::new(HashMap::new()),
			}
		}

		fn fail_transient_at(&self, number: u64, times: u32) {
			self.transient_failures.lock().unwrap().insert(number, times);
		}

		fn delay_at(&self, number: u64, delay: Duration) {
			self.delays.lock().unwrap().insert(number, delay);
		}

		fn take_transient_failure(&self, number: u64) -> Option<RpcError> {
			let mut failures = self.transient_failures.lock().unwrap();
			let remaining = failures.get_mut(&number)?;
			if *remaining == 0 {
				return None;
			}
			*remaining -= 1;
			self.transient_hits.fetch_add(1, Ordering::SeqCst);
			Some(RpcError::Transport(format!(
				"User error: RPC rate limit exceeded (-32999) at block {number}"
			)))
		}
	}

	#[async_trait]
	impl SubstrateRpc for Wrapped {
		async fn finalized_head(&self) -> Result<H256, RpcError> {
			self.inner.finalized_head().await
		}

		async fn block_hash(&self, number: u64) -> Result<Option<H256>, RpcError> {
			if self.fork_at == Some(number) {
				return Ok(Some(H256::repeat_byte(0xaa)));
			}
			self.inner.block_hash(number).await
		}

		async fn header(&self, hash: H256) -> Result<Option<BlockHeader>, RpcError> {
			self.inner.header(hash).await
		}

		async fn block(&self, hash: H256) -> Result<Option<RawBlock>, RpcError> {
			self.block_calls.fetch_add(1, Ordering::SeqCst);
			let number = number_of(hash);
			if let Some(err) = self.take_transient_failure(number) {
				return Err(err);
			}
			let delay = self.delays.lock().unwrap().get(&number).copied();
			if let Some(delay) = delay {
				tokio::time::sleep(delay).await;
			}
			self.inner.block(hash).await
		}

		async fn storage(&self, key: &[u8], at: H256) -> Result<Option<Vec<u8>>, RpcError> {
			self.inner.storage(key, at).await
		}

		async fn child_storage(
			&self,
			child_key: &[u8],
			key: &[u8],
			at: H256,
		) -> Result<Option<Vec<u8>>, RpcError> {
			self.inner.child_storage(child_key, key, at).await
		}

		async fn runtime_version(&self, at: H256) -> Result<RuntimeVersion, RpcError> {
			self.inner.runtime_version(at).await
		}

		async fn metadata(&self, at: H256) -> Result<Vec<u8>, RpcError> {
			self.inner.metadata(at).await
		}
	}

	fn fixture() -> Arc<FixtureRpc> {
		let blocks = vec![
			fixture_block(100, Some(EVENTS), Some(TIMESTAMP_MS)),
			fixture_block(101, Some(&empty_events()), Some(TIMESTAMP_MS)),
			fixture_block(102, Some(EVENTS), Some(TIMESTAMP_MS)),
		];
		let metadata = HashMap::from([(SPEC_VERSION, METADATA.to_vec())]);
		Arc::new(FixtureRpc::from_blocks(blocks, metadata, FINALIZED))
	}

	fn chain(max_block_range: u64) -> ChainConfig {
		ChainConfig {
			name: "nexus".into(),
			rpc: String::new(),
			eth_chain_id: 1000003367,
			hasher: Hasher::Keccak,
			pallets: vec!["ismp".into(), "consensusIncentives".into()],
			max_block_range,
			cache_blocks: 64,
			upstream_concurrency: 8,
			upstream_retries: 2,
			upstream_backoff_ms: 10,
			rate_limit_cooldown_ms: 1000,
			range_deadline_ms: 60_000,
		}
	}

	fn new_service(rpc: Arc<dyn SubstrateRpc>, max_block_range: u64) -> Arc<BlockService> {
		new_service_with(rpc, chain(max_block_range))
	}

	fn new_service_with(rpc: Arc<dyn SubstrateRpc>, chain: ChainConfig) -> Arc<BlockService> {
		Arc::new(BlockService::new(rpc, chain).unwrap())
	}

	fn state_machine_updated_topic() -> H256 {
		keccak256(b"StateMachineUpdated((string,bytes4),uint64)")
	}

	fn assert_nexus_block(block: &DecodedBlock) {
		let ismp = pallet_address("ismp");
		let update = block
			.logs
			.iter()
			.find(|log| log.topics[0] == state_machine_updated_topic())
			.expect("StateMachineUpdated log");
		assert_eq!(update.address, ismp);
		assert_eq!(update.block_number, block.number);
		assert_eq!(update.block_hash, block.hash);
		assert_eq!(update.block_timestamp, TIMESTAMP_MS / 1000);
		assert_eq!(update.log_index, 8);
		assert_eq!(update.transaction_index, 2);
		assert_eq!(update.transaction_hash, block.extrinsic_hashes[2]);
		assert_eq!(block.timestamp, TIMESTAMP_MS / 1000);
		assert_eq!(block.timestamp_ms, TIMESTAMP_MS);
		assert_eq!(block.spec_version, SPEC_VERSION);
		assert_eq!(block.extrinsic_hashes.len(), 4);
		assert!(block
			.logs_bloom
			.contains_input(alloy_primitives::BloomInput::Raw(ismp.as_slice())));
		assert!(block.logs.iter().all(|log| log.block_number == block.number));
	}

	#[tokio::test]
	async fn range_decodes_consecutive_blocks() {
		let service = new_service(fixture(), 1000);
		let blocks = service.range(100, 102).await.unwrap();
		assert_eq!(blocks.iter().map(|b| b.number).collect::<Vec<_>>(), vec![100, 101, 102]);
		assert_eq!(blocks[1].parent_hash, blocks[0].hash);
		assert_eq!(blocks[2].parent_hash, blocks[1].hash);

		assert_nexus_block(&blocks[0]);
		assert_nexus_block(&blocks[2]);

		assert!(blocks[1].logs.is_empty());
		assert_eq!(blocks[1].logs_bloom, Bloom::default());
		assert_eq!(blocks[1].timestamp, TIMESTAMP_MS / 1000);
	}

	#[tokio::test]
	async fn range_is_all_or_nothing_and_recovers() {
		let rpc = fixture();
		let service = new_service(rpc.clone(), 1000);

		rpc.fail_at(101);
		let err = service.range(100, 102).await.unwrap_err();
		assert!(matches!(err, BlockError::At { number: 101, .. }), "{err}");
		assert!(matches!(
			err,
			BlockError::At { source, .. } if matches!(*source, BlockError::Rpc(RpcError::Transport(_)))
		));

		rpc.recover();
		let blocks = service.range(100, 102).await.unwrap();
		assert_eq!(blocks.len(), 3);
		assert!(blocks.windows(2).all(|pair| pair[0].number + 1 == pair[1].number));
	}

	#[tokio::test]
	async fn range_rejects_oversized_and_unfinalized() {
		let service = new_service(fixture(), 1);
		assert!(matches!(
			service.range(100, 102).await,
			Err(BlockError::RangeTooLarge { from: 100, to: 102, max: 1 })
		));

		let service = new_service(fixture(), 2);
		assert_eq!(service.range(100, 102).await.unwrap().len(), 3);

		let service = new_service(fixture(), 1000);
		assert!(matches!(
			service.range(101, 103).await,
			Err(BlockError::BeyondHead { to: 103, head: 102 })
		));
		assert!(matches!(
			service.range(102, 101).await,
			Err(BlockError::InvalidRange { from: 102, to: 101 })
		));
	}

	#[tokio::test]
	async fn block_by_number_beyond_head_is_none() {
		let service = new_service(fixture(), 1000);
		assert!(service.block_by_number(103).await.unwrap().is_none());
		assert_eq!(service.finalized_number().await.unwrap(), FINALIZED);
	}

	#[tokio::test]
	async fn block_by_number_is_cached() {
		let service = new_service(fixture(), 1000);
		let first = service.block_by_number(100).await.unwrap().unwrap();
		let second = service.block_by_number(100).await.unwrap().unwrap();
		assert!(Arc::ptr_eq(&first, &second));

		let by_hash = service.block_by_hash(first.hash).await.unwrap().unwrap();
		assert!(Arc::ptr_eq(&first, &by_hash));
	}

	#[tokio::test]
	async fn block_by_hash_decodes_and_caches_canonical_blocks() {
		let service = new_service(fixture(), 1000);
		let block = service.block_by_hash(hash_of(102)).await.unwrap().unwrap();
		assert_nexus_block(&block);
		let again = service.block_by_number(102).await.unwrap().unwrap();
		assert!(Arc::ptr_eq(&block, &again));
		assert!(service.block_by_hash(H256::repeat_byte(0xff)).await.unwrap().is_none());
	}

	#[tokio::test]
	async fn storage_reads_at_block_number() {
		let service = new_service(fixture(), 1000);
		assert_eq!(
			service.storage(&timestamp_now_key(), 100).await.unwrap(),
			Some(TIMESTAMP_MS.encode())
		);
		assert_eq!(service.storage(b"missing", 100).await.unwrap(), None);
		assert!(matches!(
			service.storage(&timestamp_now_key(), 103).await,
			Err(BlockError::NotFound(103))
		));
		assert_eq!(service.child_storage(b"child", b"key", 100).await.unwrap(), None);
	}

	#[tokio::test]
	async fn missing_timestamp_and_events_are_reported() {
		let blocks = vec![
			fixture_block(100, Some(EVENTS), None),
			fixture_block(101, None, Some(TIMESTAMP_MS)),
		];
		let metadata = HashMap::from([(SPEC_VERSION, METADATA.to_vec())]);
		let rpc = Arc::new(FixtureRpc::from_blocks(blocks, metadata, 101));
		let service = new_service(rpc, 1000);

		let err = service.block_by_number(100).await.unwrap_err();
		assert!(matches!(err, BlockError::At { number: 100, .. }), "{err}");
		assert!(matches!(root(&err), BlockError::MissingTimestamp(100)), "{err}");
		assert_eq!(err.to_string(), "block 100: missing Timestamp.Now at block 100");

		let err = service.block_by_number(101).await.unwrap_err();
		assert!(matches!(err, BlockError::At { number: 101, .. }), "{err}");
		assert!(matches!(root(&err), BlockError::MissingEvents(101)), "{err}");
	}

	#[tokio::test]
	async fn upgrade_block_decodes_with_the_parent_runtime() {
		let new_spec = SPEC_VERSION + 100;
		let mut upgrade = fixture_block(101, Some(EVENTS), Some(TIMESTAMP_MS));
		upgrade.runtime_version = version(new_spec);
		upgrade.parent_runtime_version = Some(version(SPEC_VERSION));
		let mut after = fixture_block(102, Some(EVENTS), Some(TIMESTAMP_MS));
		after.runtime_version = version(new_spec);
		after.parent_runtime_version = Some(version(new_spec));
		let blocks = vec![fixture_block(100, Some(EVENTS), Some(TIMESTAMP_MS)), upgrade, after];
		let metadata =
			HashMap::from([(SPEC_VERSION, METADATA.to_vec()), (new_spec, b"corrupt".to_vec())]);
		let service = new_service(Arc::new(FixtureRpc::from_blocks(blocks, metadata, 102)), 1000);

		let blocks = service.range(100, 101).await.unwrap();
		assert_nexus_block(&blocks[0]);
		assert_nexus_block(&blocks[1]);
		assert_eq!(blocks[1].spec_version, SPEC_VERSION);

		let err = service.block_by_number(102).await.unwrap_err();
		assert!(matches!(err, BlockError::At { number: 102, .. }), "{err}");
		assert!(matches!(root(&err), BlockError::Decode(_)), "{err}");
	}

	#[tokio::test]
	async fn block_by_hash_ignores_unfinalized_and_forked_hashes() {
		let blocks = vec![
			fixture_block(100, Some(EVENTS), Some(TIMESTAMP_MS)),
			fixture_block(101, Some(EVENTS), Some(TIMESTAMP_MS)),
			fixture_block(102, Some(EVENTS), Some(TIMESTAMP_MS)),
			fixture_block(103, Some(EVENTS), Some(TIMESTAMP_MS)),
		];
		let metadata = HashMap::from([(SPEC_VERSION, METADATA.to_vec())]);
		let rpc = Arc::new(FixtureRpc::from_blocks(blocks, metadata, FINALIZED));
		let service = new_service(rpc.clone(), 1000);
		assert!(service.block_by_hash(hash_of(103)).await.unwrap().is_none());
		assert_eq!(service.canonical_number(hash_of(103)).await.unwrap(), None);
		assert_eq!(service.canonical_number(hash_of(102)).await.unwrap(), Some(102));

		let mut forked = Wrapped::wrap(rpc);
		forked.fork_at = Some(101);
		let service = new_service(Arc::new(forked), 1000);
		assert!(service.block_by_hash(hash_of(101)).await.unwrap().is_none());
		assert_eq!(service.canonical_number(hash_of(101)).await.unwrap(), None);
		assert!(service.block_by_hash(hash_of(100)).await.unwrap().is_some());
	}

	#[tokio::test]
	async fn concurrent_ranges_share_one_decode_per_block() {
		let rpc = Wrapped::new(fixture());
		let service = new_service(rpc.clone(), 1000);
		let (first, second) = tokio::join!(service.range(100, 102), service.range(100, 102));
		let (first, second) = (first.unwrap(), second.unwrap());
		assert_eq!(first.len(), 3);
		assert!(first.iter().zip(&second).all(|(a, b)| Arc::ptr_eq(a, b)));
		assert_eq!(rpc.block_calls.load(Ordering::SeqCst), 3);

		let (first, second) = tokio::join!(service.block_by_number(100), service.range(100, 102));
		assert!(first.unwrap().is_some());
		assert_eq!(second.unwrap().len(), 3);
		assert_eq!(rpc.block_calls.load(Ordering::SeqCst), 3);
	}

	#[tokio::test]
	async fn shared_failures_reach_every_waiter_and_are_retried() {
		let fixture = fixture();
		let rpc = Wrapped::new(fixture.clone());
		let service = new_service(rpc.clone(), 1000);
		let blocks = vec![
			fixture_block(100, Some(EVENTS), Some(TIMESTAMP_MS)),
			fixture_block(101, None, Some(TIMESTAMP_MS)),
			fixture_block(102, Some(EVENTS), Some(TIMESTAMP_MS)),
		];
		let metadata = HashMap::from([(SPEC_VERSION, METADATA.to_vec())]);
		let broken = Wrapped::new(Arc::new(FixtureRpc::from_blocks(blocks, metadata, FINALIZED)));
		let broken_service = new_service(broken.clone(), 1000);

		let (first, second) =
			tokio::join!(broken_service.range(100, 102), broken_service.range(100, 102));
		for err in [first.unwrap_err(), second.unwrap_err()] {
			assert!(matches!(err, BlockError::At { number: 101, .. }), "{err}");
			assert!(matches!(root(&err), BlockError::MissingEvents(101)), "{err}");
		}
		assert_eq!(broken.block_calls.load(Ordering::SeqCst), 3);

		let err = broken_service.block_by_number(101).await.unwrap_err();
		assert!(matches!(root(&err), BlockError::MissingEvents(101)), "{err}");
		assert_eq!(broken.block_calls.load(Ordering::SeqCst), 4);

		assert_eq!(service.range(100, 102).await.unwrap().len(), 3);
		assert_eq!(rpc.block_calls.load(Ordering::SeqCst), 3);
	}

	#[tokio::test(start_paused = true)]
	async fn transient_failures_are_retried() {
		let rpc = Wrapped::new(fixture());
		let service = new_service(rpc.clone(), 1000);
		rpc.fail_transient_at(101, 1);

		let blocks = service.range(100, 102).await.unwrap();
		assert_eq!(blocks.iter().map(|b| b.number).collect::<Vec<_>>(), vec![100, 101, 102]);
		assert_eq!(rpc.transient_hits.load(Ordering::SeqCst), 1);
		assert_eq!(rpc.block_calls.load(Ordering::SeqCst), 4);
		assert!(service.cooling_down());
	}

	#[tokio::test(start_paused = true)]
	async fn non_transient_failures_are_not_retried() {
		let fixture = fixture();
		let rpc = Wrapped::new(fixture.clone());
		let service = new_service(rpc.clone(), 1000);
		fixture.fail_at(101);

		let started = Instant::now();
		let err = service.range(100, 102).await.unwrap_err();
		assert!(matches!(err, BlockError::At { number: 101, .. }), "{err}");
		assert!(
			matches!(root(&err), BlockError::Rpc(RpcError::Transport(msg)) if msg.contains("injected")),
			"{err}"
		);
		assert_eq!(started.elapsed(), Duration::ZERO, "no backoff sleep");
		assert_eq!(rpc.transient_hits.load(Ordering::SeqCst), 0);
		assert!(!service.cooling_down());
	}

	#[tokio::test(start_paused = true)]
	async fn exhausted_retries_fail_the_block() {
		let rpc = Wrapped::new(fixture());
		let service = new_service(rpc.clone(), 1000);
		rpc.fail_transient_at(101, 10);

		let err = service.range(100, 102).await.unwrap_err();
		assert!(matches!(err, BlockError::At { number: 101, .. }), "{err}");
		assert!(
			matches!(root(&err), BlockError::Rpc(RpcError::Transport(msg)) if msg.contains("rate limit")),
			"{err}"
		);
		assert_eq!(rpc.transient_hits.load(Ordering::SeqCst), 3);
		assert_eq!(rpc.block_calls.load(Ordering::SeqCst), 5);

		rpc.fail_transient_at(101, 0);
		assert_eq!(service.range(100, 102).await.unwrap().len(), 3);
	}

	#[tokio::test(start_paused = true)]
	async fn range_deadline_hints_at_the_cached_prefix() {
		let rpc = Wrapped::new(fixture());
		let mut chain = chain(1000);
		chain.range_deadline_ms = 200;
		let service = new_service_with(rpc.clone(), chain);
		rpc.delay_at(102, Duration::from_millis(500));

		let err = service.range(100, 102).await.unwrap_err();
		assert!(
			matches!(err, BlockError::RangeDeadline { from: 100, to: 102, decoded_to: Some(101) }),
			"{err}"
		);
		assert_eq!(rpc.block_calls.load(Ordering::SeqCst), 3);

		let started = Instant::now();
		let blocks = service.range(100, 101).await.unwrap();
		assert_eq!(blocks.len(), 2);
		assert_eq!(started.elapsed(), Duration::ZERO);
		assert_eq!(rpc.block_calls.load(Ordering::SeqCst), 3);

		tokio::time::sleep(Duration::from_secs(1)).await;
		let blocks = service.range(100, 102).await.unwrap();
		assert_eq!(blocks.len(), 3);
		assert_eq!(rpc.block_calls.load(Ordering::SeqCst), 3);
	}

	#[tokio::test(start_paused = true)]
	async fn range_deadline_without_progress_hints_at_the_first_block() {
		let rpc = Wrapped::new(fixture());
		let mut chain = chain(1000);
		chain.range_deadline_ms = 200;
		let service = new_service_with(rpc.clone(), chain);
		rpc.delay_at(100, Duration::from_millis(500));

		let err = service.range(100, 102).await.unwrap_err();
		assert!(
			matches!(err, BlockError::RangeDeadline { from: 100, to: 102, decoded_to: None }),
			"{err}"
		);
	}

	#[test]
	fn backoff_doubles_with_jitter_and_caps() {
		let within = |attempt: u32, base: u64| {
			let delay = backoff(250, attempt).as_millis() as u64;
			assert!(delay >= base && delay <= base + base / 4, "attempt {attempt}: {delay}");
		};
		within(1, 250);
		within(2, 500);
		within(3, 1000);
		within(5, 4000);
		within(6, 5000);
		within(40, 5000);
	}

	#[test]
	fn synthetic_hashes_differ_per_phase() {
		let block = hash_of(1);
		assert_ne!(synthetic_tx_hash(block, b"init"), synthetic_tx_hash(block, b"final"));
		assert_ne!(synthetic_tx_hash(block, b"init"), synthetic_tx_hash(hash_of(2), b"init"));
	}
}
