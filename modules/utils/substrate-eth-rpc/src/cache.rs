use std::{
	future::Future,
	num::NonZeroUsize,
	sync::{Arc, Mutex},
	time::Duration,
};

use lru::LruCache;
use tokio::time::Instant;

use crate::types::{DecodedBlock, H256};

pub struct BlockCache {
	by_hash: Mutex<LruCache<H256, Arc<DecodedBlock>>>,
	by_number: Mutex<LruCache<u64, H256>>,
}

impl BlockCache {
	pub fn new(capacity: usize) -> Self {
		let capacity = NonZeroUsize::new(capacity.max(1)).expect("capacity is at least one");
		Self {
			by_hash: Mutex::new(LruCache::new(capacity)),
			by_number: Mutex::new(LruCache::new(capacity)),
		}
	}

	pub fn get_by_hash(&self, hash: H256) -> Option<Arc<DecodedBlock>> {
		self.by_hash.lock().unwrap_or_else(|e| e.into_inner()).get(&hash).cloned()
	}

	pub fn get_by_number(&self, number: u64) -> Option<Arc<DecodedBlock>> {
		let hash = *self.by_number.lock().unwrap_or_else(|e| e.into_inner()).get(&number)?;
		self.get_by_hash(hash)
	}

	pub fn insert(&self, block: Arc<DecodedBlock>) {
		self.by_number
			.lock()
			.unwrap_or_else(|e| e.into_inner())
			.put(block.number, block.hash);
		self.by_hash.lock().unwrap_or_else(|e| e.into_inner()).put(block.hash, block);
	}
}

const DEFAULT_FINALIZED_TTL: Duration = Duration::from_millis(500);

pub struct FinalizedCache {
	ttl: Duration,
	head: Mutex<Option<(Instant, u64, H256)>>,
}

impl Default for FinalizedCache {
	fn default() -> Self {
		Self::new(DEFAULT_FINALIZED_TTL)
	}
}

impl FinalizedCache {
	pub fn new(ttl: Duration) -> Self {
		Self { ttl, head: Mutex::new(None) }
	}

	pub async fn get_or_refresh<F, E>(&self, fetch: F) -> Result<(u64, H256), E>
	where
		F: Future<Output = Result<(u64, H256), E>>,
	{
		if let Some((fetched_at, number, hash)) =
			*self.head.lock().unwrap_or_else(|e| e.into_inner())
		{
			if fetched_at.elapsed() < self.ttl {
				return Ok((number, hash));
			}
		}
		let (number, hash) = fetch.await?;
		*self.head.lock().unwrap_or_else(|e| e.into_inner()) = Some((Instant::now(), number, hash));
		Ok((number, hash))
	}
}

#[cfg(test)]
mod tests {
	use alloy_primitives::Bloom;

	use super::*;

	fn block(number: u64, byte: u8) -> Arc<DecodedBlock> {
		Arc::new(DecodedBlock {
			hash: H256::repeat_byte(byte),
			parent_hash: H256::ZERO,
			number,
			timestamp: 0,
			timestamp_ms: 0,
			state_root: H256::ZERO,
			extrinsics_root: H256::ZERO,
			spec_version: 1,
			extrinsic_hashes: vec![],
			logs: vec![],
			logs_bloom: Bloom::default(),
		})
	}

	#[test]
	fn serves_by_hash_and_number() {
		let cache = BlockCache::new(2);
		let first = block(1, 0x01);
		cache.insert(first.clone());
		assert!(Arc::ptr_eq(&cache.get_by_hash(first.hash).unwrap(), &first));
		assert!(Arc::ptr_eq(&cache.get_by_number(1).unwrap(), &first));
		assert!(cache.get_by_number(2).is_none());
		assert!(cache.get_by_hash(H256::repeat_byte(0x02)).is_none());
	}

	#[test]
	fn evicts_least_recently_used() {
		let cache = BlockCache::new(2);
		cache.insert(block(1, 0x01));
		cache.insert(block(2, 0x02));
		cache.insert(block(3, 0x03));
		assert!(cache.get_by_number(1).is_none());
		assert!(cache.get_by_number(2).is_some());
		assert!(cache.get_by_number(3).is_some());
	}

	#[test]
	fn zero_capacity_still_holds_one_block() {
		let cache = BlockCache::new(0);
		cache.insert(block(1, 0x01));
		assert!(cache.get_by_number(1).is_some());
	}

	#[tokio::test]
	async fn finalized_serves_cached_head_within_ttl() {
		let cache = FinalizedCache::new(Duration::from_secs(60));
		let fetch =
			|number: u64| async move { Ok::<_, ()>((number, H256::repeat_byte(number as u8))) };
		assert_eq!(cache.get_or_refresh(fetch(10)).await.unwrap().0, 10);
		assert_eq!(cache.get_or_refresh(fetch(11)).await.unwrap().0, 10);
	}

	#[tokio::test]
	async fn finalized_refreshes_after_ttl() {
		let cache = FinalizedCache::new(Duration::ZERO);
		let fetch =
			|number: u64| async move { Ok::<_, ()>((number, H256::repeat_byte(number as u8))) };
		assert_eq!(cache.get_or_refresh(fetch(10)).await.unwrap().0, 10);
		assert_eq!(cache.get_or_refresh(fetch(11)).await.unwrap().0, 11);
	}

	#[tokio::test]
	async fn finalized_does_not_cache_failures() {
		let cache = FinalizedCache::default();
		assert!(cache.get_or_refresh(async { Err::<(u64, H256), ()>(()) }).await.is_err());
		assert_eq!(
			cache.get_or_refresh(async { Ok::<_, ()>((5, H256::ZERO)) }).await.unwrap().0,
			5
		);
	}
}
