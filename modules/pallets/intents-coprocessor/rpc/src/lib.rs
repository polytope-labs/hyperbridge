// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Watches the transaction pool for `place_bid` and `retract_bid` extrinsics,
//! exposing them over RPC before block inclusion for sub-second bid discovery.

use codec::Encode;
use jsonrpsee::{
	core::{async_trait, RpcResult, SubscriptionResult},
	proc_macros::rpc,
	types::{ErrorObject, ErrorObjectOwned},
	PendingSubscriptionSink, SubscriptionMessage,
};
use pallet_intents_runtime_api::IntentsCoprocessorApi;
use polkadot_sdk::*;
use sc_client_api::Backend;
use sc_transaction_pool_api::{InPoolTransaction, TransactionPool};
use serde::{Deserialize, Serialize};
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_core::{
	offchain::{storage::OffchainDb, OffchainDbExt, OffchainStorage},
	H256,
};
use sp_runtime::traits::Block as BlockT;
use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
	time::{Duration, Instant},
};
use tokio::sync::broadcast;

const LOG_TARGET: &str = "intents-rpc";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcBidInfo {
	pub commitment: H256,
	#[serde(with = "hex_bytes")]
	pub filler: Vec<u8>,
	/// Empty for retractions.
	#[serde(with = "hex_bytes")]
	pub user_op: Vec<u8>,
	pub confirmed: bool,
}

mod hex_bytes {
	use serde::{self, Deserialize, Deserializer, Serializer};

	pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&format!("0x{}", hex::encode(bytes)))
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		let s = s.strip_prefix("0x").unwrap_or(&s);
		hex::decode(s).map_err(serde::de::Error::custom)
	}
}

#[derive(Clone, Debug)]
struct BidEntry {
	filler: Vec<u8>,
	user_op: Vec<u8>,
	confirmed: bool,
}

#[derive(Clone, Debug)]
struct OrderBids {
	first_seen: Instant,
	entries: Vec<BidEntry>,
}

/// In-memory bid cache.
pub struct BidCache {
	bids: RwLock<HashMap<H256, OrderBids>>,
	ttl: Duration,
}

impl BidCache {
	pub fn new(ttl: Duration) -> Self {
		Self { bids: RwLock::new(HashMap::new()), ttl }
	}

	pub fn insert(&self, commitment: H256, filler: Vec<u8>, user_op: Vec<u8>) {
		let entry = BidEntry { filler: filler.clone(), user_op, confirmed: false };

		let mut bids = self.bids.write().expect("BidCache lock poisoned");
		let order = bids
			.entry(commitment)
			.or_insert_with(|| OrderBids { first_seen: Instant::now(), entries: Vec::new() });
		if let Some(existing) = order.entries.iter_mut().find(|e| e.filler == filler) {
			*existing = entry;
		} else {
			order.entries.push(entry);
		}
	}

	pub fn remove_bid(&self, commitment: &H256, filler: &[u8]) {
		let mut bids = self.bids.write().expect("BidCache lock poisoned");
		if let Some(order) = bids.get_mut(commitment) {
			order.entries.retain(|e| e.filler != filler);
			if order.entries.is_empty() {
				bids.remove(commitment);
			}
		}
	}

	pub fn get_bids(&self, commitment: &H256) -> Vec<RpcBidInfo> {
		let bids = self.bids.read().expect("BidCache lock poisoned");
		bids.get(commitment)
			.map(|order| {
				order
					.entries
					.iter()
					.map(|e| RpcBidInfo {
						commitment: *commitment,
						filler: e.filler.clone(),
						user_op: e.user_op.clone(),
						confirmed: e.confirmed,
					})
					.collect()
			})
			.unwrap_or_default()
	}

	pub fn remove_expired(&self) {
		let now = Instant::now();
		let mut bids = self.bids.write().expect("BidCache lock poisoned");
		bids.retain(|_commitment, order| now.duration_since(order.first_seen) < self.ttl);
	}

	pub fn confirm_bid(&self, commitment: &H256, filler: &[u8]) {
		let mut bids = self.bids.write().expect("BidCache lock poisoned");
		if let Some(order) = bids.get_mut(commitment) {
			if let Some(entry) = order.entries.iter_mut().find(|e| e.filler == filler) {
				entry.confirmed = true;
			}
		}
	}
}

fn runtime_error_into_rpc_error(e: impl std::fmt::Display) -> ErrorObjectOwned {
	ErrorObject::owned(9877, format!("{e}"), None::<String>)
}

#[rpc(client, server)]
pub trait IntentsApi {
	#[method(name = "intents_getBidsForOrder")]
	fn get_bids_for_order(&self, commitment: H256) -> RpcResult<Vec<RpcBidInfo>>;

	#[subscription(name = "intents_subscribeBids" => "intents_bidNotification", unsubscribe = "intents_unsubscribeBids", item = RpcBidInfo)]
	async fn subscribe_bids(&self, commitment: Option<H256>) -> SubscriptionResult;
}

pub struct IntentsRpcHandler<C, Block, S, T> {
	client: Arc<C>,
	offchain_db: OffchainDb<S>,
	bid_cache: Arc<BidCache>,
	bid_sender: broadcast::Sender<RpcBidInfo>,
	_marker: std::marker::PhantomData<(Block, T)>,
}

impl<C, Block, S, T> IntentsRpcHandler<C, Block, S, T>
where
	Block: BlockT,
	S: OffchainStorage + Clone + Send + Sync + 'static,
	T: Backend<Block, OffchainStorage = S> + Send + Sync + 'static,
{
	pub fn new(
		client: Arc<C>,
		backend: Arc<T>,
		bid_cache: Arc<BidCache>,
		bid_sender: broadcast::Sender<RpcBidInfo>,
	) -> Result<Self, String> {
		let offchain_db = OffchainDb::new(
			backend
				.offchain_storage()
				.ok_or_else(|| "Offchain storage not available in backend".to_string())?,
		);
		Ok(Self { client, offchain_db, bid_cache, bid_sender, _marker: Default::default() })
	}
}

#[async_trait]
impl<C, Block, S, T> IntentsApiServer for IntentsRpcHandler<C, Block, S, T>
where
	Block: BlockT,
	S: OffchainStorage + Clone + Send + Sync + 'static,
	T: Backend<Block> + Send + Sync + 'static,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
	C::Api: IntentsCoprocessorApi<Block>,
{
	fn get_bids_for_order(&self, commitment: H256) -> RpcResult<Vec<RpcBidInfo>> {
		// Get mempool bids from in-memory cache
		let mut bids = self.bid_cache.get_bids(&commitment);

		// Query confirmed bids from offchain storage via the runtime API
		let mut api = self.client.runtime_api();
		api.register_extension(OffchainDbExt::new(self.offchain_db.clone()));
		let at = self.client.info().best_hash;

		if let Ok(onchain_bids) = api.get_bids_for_commitment(at, commitment) {
			for (filler, user_op) in onchain_bids {
				// Deduplicate: skip if a bid from this filler already exists in the cache
				if !bids.iter().any(|b| b.filler == filler) {
					bids.push(RpcBidInfo { commitment, filler, user_op, confirmed: true });
				}
			}
		}

		Ok(bids)
	}

	async fn subscribe_bids(
		&self,
		pending: PendingSubscriptionSink,
		commitment: Option<H256>,
	) -> SubscriptionResult {
		let mut rx = self.bid_sender.subscribe();
		let sink = pending.accept().await.map_err(|e| {
			runtime_error_into_rpc_error(format!("Failed to accept subscription: {e}"))
		})?;

		tokio::spawn(async move {
			while let Ok(bid) = rx.recv().await {
				if let Some(ref filter) = commitment {
					if &bid.commitment != filter {
						continue;
					}
				}
				if let Ok(msg) = SubscriptionMessage::from_json(&bid) {
					if sink.send(msg).await.is_err() {
						break;
					}
				}
			}
		});

		Ok(())
	}
}

/// Watches the tx pool for bid-related extrinsics, updating the cache and
/// notifying subscribers as they arrive.
pub async fn run_bid_watcher<P, C, Block>(
	pool: Arc<P>,
	client: Arc<C>,
	bid_cache: Arc<BidCache>,
	bid_sender: broadcast::Sender<RpcBidInfo>,
) where
	Block: BlockT,
	P: TransactionPool<Block = Block> + 'static,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + 'static,
	C::Api: IntentsCoprocessorApi<Block>,
{
	use futures::StreamExt;

	let mut stream = pool.import_notification_stream();

	while let Some(tx_hash) = stream.next().await {
		let tx = match pool.ready_transaction(&tx_hash) {
			Some(tx) => tx,
			None => continue,
		};

		let extrinsic_bytes = tx.data().encode();
		let best_hash = client.info().best_hash;

		match client.runtime_api().extract_bid(best_hash, extrinsic_bytes) {
			Ok(Some((commitment, filler, user_op))) => {
				let is_retraction = user_op.is_empty();

				if is_retraction {
					log::info!(
						target: LOG_TARGET,
						"retract_bid in mempool for {commitment:?}",
					);
					bid_cache.remove_bid(&commitment, &filler);
				} else {
					log::info!(
						target: LOG_TARGET,
						"place_bid in mempool for {commitment:?}",
					);
					bid_cache.insert(commitment, filler.clone(), user_op.clone());
				}

				let _ =
					bid_sender.send(RpcBidInfo { commitment, filler, user_op, confirmed: false });
			},
			Ok(None) => {},
			Err(e) => {
				log::debug!(target: LOG_TARGET, "extract_bid failed: {e:?}");
			},
		}
	}
}

pub async fn run_bid_cleanup(bid_cache: Arc<BidCache>, interval: Duration) {
	let mut timer = tokio::time::interval(interval);
	loop {
		timer.tick().await;
		bid_cache.remove_expired();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn cache() -> BidCache {
		BidCache::new(Duration::from_secs(300))
	}

	#[test]
	fn insert_and_get() {
		let c = cache();
		let key = H256::random();

		c.insert(key, vec![1, 2, 3], vec![4, 5, 6]);

		let bids = c.get_bids(&key);
		assert_eq!(bids.len(), 1);
		assert_eq!(bids[0].filler, vec![1, 2, 3]);
		assert_eq!(bids[0].user_op, vec![4, 5, 6]);
		assert!(!bids[0].confirmed);
	}

	#[test]
	fn multiple_fillers_same_commitment() {
		let c = cache();
		let key = H256::random();

		c.insert(key, vec![1], vec![10]);
		c.insert(key, vec![2], vec![20]);

		let bids = c.get_bids(&key);
		assert_eq!(bids.len(), 2);
		assert!(bids.iter().any(|b| b.filler == vec![1]));
		assert!(bids.iter().any(|b| b.filler == vec![2]));
	}

	#[test]
	fn duplicate_filler_replaces_previous_bid() {
		let c = cache();
		let key = H256::random();

		c.insert(key, vec![1], vec![10]);
		c.insert(key, vec![1], vec![99]);

		let bids = c.get_bids(&key);
		assert_eq!(bids.len(), 1);
		assert_eq!(bids[0].user_op, vec![99]);
	}

	#[test]
	fn remove_only_bid_clears_commitment() {
		let c = cache();
		let key = H256::random();

		c.insert(key, vec![1], vec![10]);
		c.remove_bid(&key, &[1]);

		assert!(c.get_bids(&key).is_empty());
	}

	#[test]
	fn remove_one_of_many_preserves_others() {
		let c = cache();
		let key = H256::random();

		c.insert(key, vec![1], vec![10]);
		c.insert(key, vec![2], vec![20]);
		c.remove_bid(&key, &[1]);

		let bids = c.get_bids(&key);
		assert_eq!(bids.len(), 1);
		assert_eq!(bids[0].filler, vec![2]);
	}

	#[test]
	fn unknown_commitment_returns_empty() {
		assert!(cache().get_bids(&H256::random()).is_empty());
	}

	#[test]
	fn expired_entries_are_removed() {
		let c = BidCache::new(Duration::from_millis(50));
		let key = H256::random();

		c.insert(key, vec![1], vec![10]);
		std::thread::sleep(Duration::from_millis(100));
		c.remove_expired();

		assert!(c.get_bids(&key).is_empty());
	}

	#[test]
	fn confirm_sets_flag() {
		let c = cache();
		let key = H256::random();

		c.insert(key, vec![1], vec![10]);
		assert!(!c.get_bids(&key)[0].confirmed);

		c.confirm_bid(&key, &[1]);
		assert!(c.get_bids(&key)[0].confirmed);
	}
}
