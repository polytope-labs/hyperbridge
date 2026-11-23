use std::{
	collections::{hash_map::Entry, BTreeMap, HashMap},
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
use serde_json::Value as Json;
use sp_crypto_hashing::twox_128;
use subxt_core::{blocks::Extrinsics, config::SubstrateConfig, Metadata};
use tokio::{
	sync::{OwnedSemaphorePermit, Semaphore},
	time::Instant,
};

use crate::{
	bloom::block_bloom,
	cache::{BlockCache, FinalizedCache},
	config::ChainConfig,
	decode::{decode_events, extrinsic_hashes, DecodeError, MetadataCache},
	explorer, governance,
	json::Encoder,
	mapping::{pallets_for, MappedLog, Mapper, MappingError},
	substrate::{system_events_key, timestamp_now_key, RawBlock, RpcError, SubstrateRpc},
	types::{DecodedBlock, EthLog, Hasher, Phase, H256},
};

const DECODE_CONCURRENCY: usize = 16;
const MAX_BACKOFF: Duration = Duration::from_secs(5);
/** The largest call `call_json` decodes, `pallet_preimage`'s largest preimage. */
pub const MAX_CALL_SIZE: usize = 4 * 1024 * 1024;

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
	#[error("call of {len} bytes exceeds max {max}")]
	CallTooLarge { len: usize, max: usize },
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
	metadata_fetch: Mutex<BTreeMap<u32, Arc<tokio::sync::Mutex<()>>>>,
	upstream: Arc<Semaphore>,
	cooldown_until: Mutex<Option<Instant>>,
	concurrency: usize,
}

impl BlockService {
	pub fn new(rpc: Arc<dyn SubstrateRpc>, chain: ChainConfig) -> Result<Self, BlockError> {
		let mapper = Arc::new(Mapper::new(pallets_for(&chain.pallets)?));
		let cache = BlockCache::new(chain.cache_blocks);
		let upstream = Arc::new(Semaphore::new(chain.upstream_concurrency.max(1)));
		Ok(Self {
			rpc,
			chain,
			mapper,
			metadata: MetadataCache::new(),
			cache,
			finalized: FinalizedCache::default(),
			in_flight: Mutex::new(HashMap::new()),
			metadata_fetch: Mutex::new(BTreeMap::new()),
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
				let at = "finalized head";
				let hash = self.upstream(at, || self.rpc.finalized_head()).await?;
				let header = self
					.upstream(at, || self.rpc.header(hash))
					.await?
					.ok_or(RpcError::HashNotFound(hash))?;
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
		let Some(header) =
			self.upstream(&format!("block {hash}"), || self.rpc.header(hash)).await?
		else {
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
		let hash = self.finalized_hash(at).await?;
		self.upstream(&format!("block {at}"), || self.rpc.storage(key, hash))
			.await
			.map_err(|e| BlockError::from(e).at(at))
	}

	pub async fn child_storage(
		&self,
		child_key: &[u8],
		key: &[u8],
		at: u64,
	) -> Result<Option<Vec<u8>>, BlockError> {
		let hash = self.finalized_hash(at).await?;
		self.upstream(&format!("block {at}"), || self.rpc.child_storage(child_key, key, hash))
			.await
			.map_err(|e| BlockError::from(e).at(at))
	}

	/** The value at `key` as of block `at`, decoded as its storage entry's type, as JSON text. */
	pub async fn storage_json(&self, key: &[u8], at: u64) -> Result<Option<String>, BlockError> {
		let (metadata, value) = tokio::try_join!(self.state_metadata(at), self.storage(key, at))?;
		let Some((value, ty)) = value.zip(storage_type(&metadata, key)) else {
			return Ok(None);
		};
		self.render(metadata, move |encoder| encoder.decode(ty, &value)).await
	}

	/** A pallet's constant in the runtime of block `at`, as JSON text. */
	pub async fn constant_json(
		&self,
		pallet: &str,
		name: &str,
		at: u64,
	) -> Result<Option<String>, BlockError> {
		let metadata = self.state_metadata(at).await?;
		let (pallet, name) = (pallet.to_owned(), name.to_owned());
		self.render(metadata, move |encoder| {
			let constant = encoder.metadata.pallet_by_name(&pallet)?.constant_by_name(&name)?;
			encoder.decode(constant.ty(), constant.value())
		})
		.await
	}

	/** An encoded call decoded with the runtime of block `at`, as JSON text, or `None` when the
	 * bytes aren't one of its calls. */
	pub async fn call_json(&self, call: &[u8], at: u64) -> Result<Option<String>, BlockError> {
		if call.len() > MAX_CALL_SIZE {
			return Err(BlockError::CallTooLarge { len: call.len(), max: MAX_CALL_SIZE });
		}
		let metadata = self.state_metadata(at).await?;
		let call = call.to_vec();
		self.render(metadata, move |encoder| {
			encoder.decode(encoder.metadata.outer_enums().call_enum_ty(), &call)
		})
		.await
	}

	/** The call of extrinsic `index` of block `at` as JSON, or `None` when the block has no such
	 * extrinsic. It shows a call that only lived within the block, such as a motion proposed
	 * and closed in it. The call bytes are checked as `call_json` checks them. */
	pub async fn extrinsic_json(&self, index: u32, at: u64) -> Result<Option<String>, BlockError> {
		let metadata = self.state_metadata(at).await?;
		let hash = self.canonical_hash(at).await?;
		let block = self
			.upstream(&format!("block {at}"), || self.rpc.block(hash))
			.await
			.map_err(|e| BlockError::from(e).at(at))?;
		let Some(extrinsic) =
			block.and_then(|block| block.extrinsics.into_iter().nth(index as usize))
		else {
			return Ok(None);
		};
		self.render(metadata, move |encoder| {
			let extrinsics = Extrinsics::<SubstrateConfig>::decode_from(
				vec![extrinsic],
				encoder.metadata.clone(),
			)
			.ok()?;
			let extrinsic = extrinsics.iter().next()?;
			encoder.decode(encoder.metadata.outer_enums().call_enum_ty(), extrinsic.call_bytes())
		})
		.await
	}

	/** Runs a JSON decode on the blocking pool under an upstream permit, so that
	 * `upstream_concurrency` bounds these decodes as it bounds block decodes. The decode owns
	 * the permit, so a cancelled request still holds it until its decode finishes. */
	async fn render<F>(
		&self,
		metadata: Arc<Metadata>,
		decode: F,
	) -> Result<Option<String>, BlockError>
	where
		F: FnOnce(&Encoder) -> Option<Json> + Send + 'static,
	{
		let permit = self.acquire_upstream().await?;
		tokio::task::spawn_blocking(move || {
			let _permit = permit;
			decode(&Encoder::ss58(&metadata)).map(|json| json.to_string())
		})
		.await
		.map_err(|err| BlockError::Task(err.to_string()))
	}

	/** The metadata of the runtime that wrote block `at`'s state, which is the runtime its
	 * parent reports, as for its events. A block in the cache already carries that version. */
	async fn state_metadata(&self, at: u64) -> Result<Arc<Metadata>, BlockError> {
		if at > self.finalized_number().await? {
			return Err(BlockError::NotFound(at));
		}
		let label = format!("block {at}");
		let (spec_version, spec_at, _permit) = match self.cache.get_by_number(at) {
			Some(block) => {
				let spec_at = if at == 0 { block.hash } else { block.parent_hash };
				(block.spec_version, spec_at, None)
			},
			None => {
				let permit = self.acquire_upstream().await?;
				let spec_at = self.canonical_hash(at.saturating_sub(1)).await?;
				let version = self
					.upstream(&label, || self.rpc.runtime_version(spec_at))
					.await
					.map_err(|e| BlockError::from(e).at(at))?;
				(version.spec_version, spec_at, Some(permit))
			},
		};
		self.metadata(spec_version, spec_at, &label).await
	}

	async fn metadata(
		&self,
		spec_version: u32,
		at: H256,
		label: &str,
	) -> Result<Arc<Metadata>, BlockError> {
		if let Some(metadata) = self.metadata.get(spec_version) {
			return Ok(metadata);
		}
		// Fetches of one runtime take turns, so requests that miss it together fetch it once,
		// while misses of different runtimes still fetch in parallel.
		let fetch = self
			.metadata_fetch
			.lock()
			.unwrap_or_else(|e| e.into_inner())
			.entry(spec_version)
			.or_default()
			.clone();
		let _fetching = fetch.lock().await;
		if let Some(metadata) = self.metadata.get(spec_version) {
			return Ok(metadata);
		}
		let fetched = match self.upstream(label, || self.rpc.metadata(at)).await {
			Ok(raw) => self.metadata.insert(spec_version, &raw).map_err(BlockError::from),
			Err(err) => Err(err.into()),
		};
		let mut fetches = self.metadata_fetch.lock().unwrap_or_else(|e| e.into_inner());
		if fetches.get(&spec_version).is_some_and(|entry| Arc::ptr_eq(entry, &fetch)) {
			fetches.remove(&spec_version);
		}
		fetched
	}

	fn cached_prefix_end(&self, from: u64, to: u64) -> Option<u64> {
		(from..=to)
			.take_while(|number| self.cache.get_by_number(*number).is_some())
			.last()
	}

	// Storage reads take any block number, so a number above the finalized head is not
	// found rather than resolved against the best chain. `range` checks its bound once.
	async fn finalized_hash(&self, number: u64) -> Result<H256, BlockError> {
		if number > self.finalized_number().await? {
			return Err(BlockError::NotFound(number));
		}
		self.canonical_hash(number).await
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
		let permit = self.acquire_upstream().await?;
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
		// Genesis is never executed, so its state holds neither events nor a timestamp.
		let events = match events {
			Some(events) => events,
			None if number == 0 => vec![0],
			None => return Err(BlockError::MissingEvents(number)),
		};
		let timestamp_ms = match timestamp {
			Some(timestamp) => u64::decode(&mut &timestamp[..])
				.map_err(|e| RpcError::Decode(format!("Timestamp.Now at block {number}: {e}")))?,
			None if number == 0 => 0,
			None => return Err(BlockError::MissingTimestamp(number)),
		};

		let metadata = self.metadata(version.spec_version, spec_at, &at).await?;
		// Decoding is CPU work, so it runs on the blocking pool and keeps the permit, which
		// makes `upstream_concurrency` the bound on parallel decodes too.
		let mapper = self.mapper.clone();
		let hasher = self.chain.hasher;
		let spec_version = version.spec_version;
		let block = tokio::task::spawn_blocking(move || {
			build_block(&mapper, hasher, raw, spec_version, &events, timestamp_ms, &metadata)
		})
		.await
		.map_err(|err| BlockError::Task(err.to_string()))??;
		drop(permit);
		Ok(Some(Arc::new(block)))
	}

	// During a rate-limit cooldown every fetch holds two permits, which halves the number of
	// blocks in flight against the upstream.
	async fn acquire_upstream(&self) -> Result<OwnedSemaphorePermit, BlockError> {
		let permits = if self.cooling_down() { 2.min(self.chain.upstream_concurrency) } else { 1 };
		self.upstream
			.clone()
			.acquire_many_owned(permits as u32)
			.await
			.map_err(|err| BlockError::Task(err.to_string()))
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
}

/** The value type of the storage entry `key` belongs to, found by its pallet and entry
 * prefix. */
pub(crate) fn storage_type(metadata: &Metadata, key: &[u8]) -> Option<u32> {
	metadata.pallets().find_map(|pallet| {
		let storage = pallet.storage()?;
		let prefix = twox_128(storage.prefix().as_bytes());
		if key.get(..16)? != prefix {
			return None;
		}
		storage.entries().iter().find_map(|entry| {
			(key.get(16..32)? == twox_128(entry.name().as_bytes()))
				.then(|| entry.entry_type().value_ty())
		})
	})
}

fn build_block(
	mapper: &Mapper,
	hasher: Hasher,
	raw: RawBlock,
	spec_version: u32,
	events: &[u8],
	timestamp_ms: u64,
	metadata: &subxt_core::Metadata,
) -> Result<DecodedBlock, BlockError> {
	let hashes = extrinsic_hashes(hasher, &raw.extrinsics);
	let timestamp = timestamp_ms / 1000;
	let init_hash = synthetic_tx_hash(raw.hash, b"init");
	let final_hash = synthetic_tx_hash(raw.hash, b"final");

	let transaction = |phase: &Phase| match phase {
		Phase::ApplyExtrinsic(index) =>
			hashes.get(*index as usize).map(|hash| (*index as u64, *hash)).ok_or_else(|| {
				RpcError::Decode(format!(
					"block {} references extrinsic {index} of {}",
					raw.number,
					hashes.len()
				))
			}),
		Phase::Initialization => Ok((hashes.len() as u64, init_hash)),
		Phase::Finalization => Ok((hashes.len() as u64 + 1, final_hash)),
	};
	let log = |mapped: MappedLog, phase: &Phase, log_index: u64| {
		let (transaction_index, transaction_hash) = transaction(phase)?;
		Ok::<_, RpcError>(EthLog {
			address: mapped.address,
			topics: mapped.topics,
			data: mapped.data,
			block_hash: raw.hash,
			block_number: raw.number,
			block_timestamp: timestamp,
			transaction_hash,
			transaction_index,
			log_index,
		})
	};

	let events = decode_events(metadata, events)?;
	let mut logs = Vec::new();
	for event in &events {
		if let Some(mapped) = mapper.map_event(event)? {
			logs.push(log(mapped, &event.phase, event.event_index as u64)?);
		}
	}
	// Explorer and governance logs come after every event's index, so they never share one
	// with a pallet log.
	let mut extra = Vec::new();
	if mapper.explorer() {
		extra.extend(explorer::logs(metadata, &raw, spec_version, &events));
	}
	if mapper.governance() {
		extra.extend(governance::logs(metadata, &events));
	}
	for (offset, (mapped, phase)) in extra.into_iter().enumerate() {
		logs.push(log(mapped, &phase, (events.len() + offset) as u64)?);
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
		decode::{decode_metadata, testdata::NEXUS},
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
		hash_calls: AtomicUsize,
		version_calls: AtomicUsize,
		metadata_calls: AtomicUsize,
		fork_at: Option<u64>,
		transient_failures: Mutex<HashMap<u64, u32>>,
		transient_hits: AtomicUsize,
		storage_failures: AtomicUsize,
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
				hash_calls: AtomicUsize::new(0),
				version_calls: AtomicUsize::new(0),
				metadata_calls: AtomicUsize::new(0),
				fork_at: None,
				transient_failures: Mutex::new(HashMap::new()),
				transient_hits: AtomicUsize::new(0),
				storage_failures: AtomicUsize::new(0),
				delays: Mutex::new(HashMap::new()),
			}
		}

		fn fail_transient_at(&self, number: u64, times: u32) {
			self.transient_failures.lock().unwrap().insert(number, times);
		}

		/** Rate-limits the next `times` storage reads, at any block. */
		fn fail_storage(&self, times: usize) {
			self.storage_failures.store(times, Ordering::SeqCst);
		}

		fn take_storage_failure(&self) -> Result<(), RpcError> {
			match self
				.storage_failures
				.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
			{
				Ok(_) =>
					Err(RpcError::Transport("User error: RPC rate limit exceeded (-32999)".into())),
				Err(_) => Ok(()),
			}
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
			self.hash_calls.fetch_add(1, Ordering::SeqCst);
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
			self.take_storage_failure()?;
			self.inner.storage(key, at).await
		}

		async fn child_storage(
			&self,
			child_key: &[u8],
			key: &[u8],
			at: H256,
		) -> Result<Option<Vec<u8>>, RpcError> {
			self.take_storage_failure()?;
			self.inner.child_storage(child_key, key, at).await
		}

		async fn runtime_version(&self, at: H256) -> Result<RuntimeVersion, RpcError> {
			self.version_calls.fetch_add(1, Ordering::SeqCst);
			// Yielding lets concurrent requests overlap, as they would against a real node.
			tokio::task::yield_now().await;
			self.inner.runtime_version(at).await
		}

		async fn metadata(&self, at: H256) -> Result<Vec<u8>, RpcError> {
			self.metadata_calls.fetch_add(1, Ordering::SeqCst);
			tokio::task::yield_now().await;
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

	fn remark(metadata: &Metadata) -> Vec<u8> {
		let system = metadata.pallet_by_name("System").unwrap();
		let remark = system.call_variant_by_name("remark").unwrap();
		(system.index(), remark.index, b"hi".to_vec()).encode()
	}

	#[tokio::test]
	async fn json_reads_of_a_cached_block_make_no_upstream_calls() {
		let rpc = Wrapped::new(fixture());
		let service = new_service(rpc.clone(), 1000);
		service.block_by_number(100).await.unwrap().unwrap();
		let calls = |rpc: &Wrapped| {
			[&rpc.hash_calls, &rpc.version_calls, &rpc.metadata_calls]
				.map(|calls| calls.load(Ordering::SeqCst))
		};
		let before = calls(&rpc);

		let tracks = service.constant_json("Referenda", "Tracks", 100).await.unwrap().unwrap();
		assert!(tracks.starts_with("[[0,"));
		let metadata = decode_metadata(METADATA).unwrap();
		let call = service.call_json(&remark(&metadata), 100).await.unwrap().unwrap();
		assert!(call.contains(r#""method":"remark""#));
		let now = service.storage_json(&timestamp_now_key(), 100).await.unwrap();
		assert_eq!(now.as_deref(), Some(TIMESTAMP_MS.to_string().as_str()));
		assert_eq!(calls(&rpc), before);

		assert!(matches!(
			service.constant_json("Referenda", "Tracks", 103).await,
			Err(BlockError::NotFound(103))
		));
	}

	#[tokio::test]
	async fn json_reads_of_an_uncached_block_fetch_metadata_once() {
		let rpc = Wrapped::new(fixture());
		let service = new_service(rpc.clone(), 1000);
		let reads = (0..8).map(|_| service.constant_json("Referenda", "Tracks", 101));
		let tracks = futures::future::try_join_all(reads).await.unwrap();
		assert!(tracks.iter().all(|tracks| tracks.is_some()));
		assert_eq!(rpc.version_calls.load(Ordering::SeqCst), 8);
		assert_eq!(rpc.metadata_calls.load(Ordering::SeqCst), 1);
	}

	#[tokio::test]
	async fn extrinsics_decode_to_their_calls() {
		let service = new_service(fixture(), 1000);
		let first = service.extrinsic_json(0, 101).await.unwrap().unwrap();
		let first: Json = serde_json::from_str(&first).unwrap();
		assert_eq!(
			(first["section"].as_str(), first["method"].as_str()),
			(Some("timestamp"), Some("set"))
		);
		assert_eq!(service.extrinsic_json(10_000, 101).await.unwrap(), None);
	}

	#[tokio::test]
	async fn call_json_rejects_oversized_calls() {
		let service = new_service(fixture(), 1000);
		let call = vec![0; MAX_CALL_SIZE + 1];
		assert!(matches!(
			service.call_json(&call, 100).await,
			Err(BlockError::CallTooLarge { len, max: MAX_CALL_SIZE }) if len == MAX_CALL_SIZE + 1
		));
	}

	#[tokio::test(start_paused = true)]
	async fn storage_reads_retry_rate_limits() {
		let rpc = Wrapped::new(fixture());
		let service = new_service(rpc.clone(), 1000);

		rpc.fail_storage(2);
		assert_eq!(
			service.storage(&timestamp_now_key(), 100).await.unwrap(),
			Some(TIMESTAMP_MS.encode())
		);
		rpc.fail_storage(2);
		assert_eq!(service.child_storage(b"child", b"key", 100).await.unwrap(), None);
		assert!(service.cooling_down());

		// `upstream_retries` is 2, so a third failure in a row reaches the caller.
		rpc.fail_storage(3);
		assert!(matches!(
			service.storage(&timestamp_now_key(), 100).await,
			Err(BlockError::At { number: 100, .. })
		));
	}

	#[tokio::test]
	async fn storage_ignores_unfinalized_blocks() {
		let blocks = vec![
			fixture_block(102, Some(EVENTS), Some(TIMESTAMP_MS)),
			fixture_block(103, Some(EVENTS), Some(TIMESTAMP_MS + 1)),
		];
		let metadata = HashMap::from([(SPEC_VERSION, METADATA.to_vec())]);
		let rpc = Arc::new(FixtureRpc::from_blocks(blocks, metadata, FINALIZED));
		let service = new_service(rpc, 1000);
		assert!(matches!(
			service.storage(&timestamp_now_key(), 103).await,
			Err(BlockError::NotFound(103))
		));
		assert!(matches!(
			service.child_storage(b"child", b"key", 103).await,
			Err(BlockError::NotFound(103))
		));
		assert_eq!(
			service.storage(&timestamp_now_key(), 102).await.unwrap(),
			Some(TIMESTAMP_MS.encode())
		);
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
	async fn genesis_decodes_without_events_or_timestamp() {
		let mut genesis = fixture_block(1, None, None);
		genesis.number = 0;
		genesis.hash = hash_of(0);
		genesis.parent_hash = H256::ZERO;
		let blocks = vec![genesis, fixture_block(1, None, Some(TIMESTAMP_MS))];
		let metadata = HashMap::from([(SPEC_VERSION, METADATA.to_vec())]);
		let service = new_service(Arc::new(FixtureRpc::from_blocks(blocks, metadata, 1)), 1000);

		let block = service.block_by_number(0).await.unwrap().unwrap();
		assert_eq!(block.number, 0);
		assert!(block.logs.is_empty());
		assert_eq!(block.logs_bloom, Bloom::default());
		assert_eq!(block.timestamp, 0);
		assert_eq!(block.timestamp_ms, 0);

		let err = service.block_by_number(1).await.unwrap_err();
		assert!(matches!(root(&err), BlockError::MissingEvents(1)), "{err}");
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
