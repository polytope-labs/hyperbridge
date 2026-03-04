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

//! Watches the transaction pool for `place_bid` extrinsics, exposing them
//! over RPC before block inclusion for sub-second bid discovery.

use codec::{Decode, Encode};
use jsonrpsee::{
	core::{async_trait, RpcResult, SubscriptionResult},
	proc_macros::rpc,
	types::{ErrorObject, ErrorObjectOwned},
	PendingSubscriptionSink, SubscriptionMessage,
};
use polkadot_sdk::*;
use sc_client_api::Backend;
use sc_transaction_pool_api::{InPoolTransaction, TransactionPool};
use serde::{Deserialize, Serialize};
use sp_blockchain::HeaderBackend;
use sp_core::{
	offchain::{OffchainStorage, STORAGE_PREFIX},
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
	#[serde(with = "hex_bytes")]
	pub user_op: Vec<u8>,
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
		let entry = BidEntry { filler: filler.clone(), user_op };

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

}

fn runtime_error_into_rpc_error(e: impl std::fmt::Display) -> ErrorObjectOwned {
	ErrorObject::owned(9877, format!("{e}"), None::<String>)
}

/// Construct the storage key prefix for iterating all fillers in the on-chain
/// `Bids` double-map for a given order commitment.
fn bids_storage_prefix(commitment: &H256) -> Vec<u8> {
	let mut prefix = Vec::new();
	prefix.extend_from_slice(&sp_core::hashing::twox_128(b"IntentsCoprocessor"));
	prefix.extend_from_slice(&sp_core::hashing::twox_128(b"Bids"));
	// Blake2_128Concat hasher: blake2_128(key) ++ key
	let commitment_bytes = commitment.as_bytes();
	prefix.extend_from_slice(&sp_core::hashing::blake2_128(commitment_bytes));
	prefix.extend_from_slice(commitment_bytes);
	prefix
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
	backend: Arc<T>,
	offchain_storage: S,
	bid_cache: Arc<BidCache>,
	bid_sender: broadcast::Sender<RpcBidInfo>,
	_marker: std::marker::PhantomData<Block>,
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
		let offchain_storage = backend
			.offchain_storage()
			.ok_or_else(|| "Offchain storage not available in backend".to_string())?;
		Ok(Self { client, backend, offchain_storage, bid_cache, bid_sender, _marker: Default::default() })
	}
}

#[async_trait]
impl<C, Block, S, T> IntentsApiServer for IntentsRpcHandler<C, Block, S, T>
where
	Block: BlockT,
	S: OffchainStorage + Clone + Send + Sync + 'static,
	T: Backend<Block> + Send + Sync + 'static,
	C: HeaderBackend<Block> + Send + Sync + 'static,
{
	fn get_bids_for_order(&self, commitment: H256) -> RpcResult<Vec<RpcBidInfo>> {
		// Get mempool bids from in-memory cache
		let mut bids = self.bid_cache.get_bids(&commitment);

		// Query on-chain bids by iterating on-chain Bids storage and reading
		// offchain storage directly from the backend.
		let best_hash = self.client.info().best_hash;
		let state = self
			.backend
			.state_at(best_hash, sc_client_api::TrieCacheContext::Untrusted)
			.map_err(runtime_error_into_rpc_error)?;

		let prefix = bids_storage_prefix(&commitment);
		let mut current_key = prefix.clone();

		loop {
			let next_key =
				match sc_client_api::StateBackend::next_storage_key(&state, &current_key)
					.map_err(|e| runtime_error_into_rpc_error(format!("{e:?}")))?
				{
					Some(k) => k,
					None => break,
				};

			if !next_key.starts_with(&prefix) {
				break;
			}

			// Key layout after prefix: blake2_128(filler) ++ filler_encoded
			let filler_start = prefix.len() + 16;
			if next_key.len() > filler_start {
				let filler_encoded = &next_key[filler_start..];

				if !bids.iter().any(|b| b.filler == filler_encoded) {
					let mut offchain_key = b"intents::bid::".to_vec();
					offchain_key.extend_from_slice(commitment.as_bytes());
					offchain_key.extend_from_slice(filler_encoded);

					if let Some(data) =
						self.offchain_storage.get(STORAGE_PREFIX, &offchain_key)
					{
						// Bid encoding: filler.encode() ++ user_op.encode()
						if data.len() > filler_encoded.len() {
							if let Ok(user_op) =
								Vec::<u8>::decode(&mut &data[filler_encoded.len()..])
							{
								bids.push(RpcBidInfo {
									commitment,
									filler: filler_encoded.to_vec(),
									user_op,
								});
							}
						}
					}
				}
			}

			current_key = next_key;
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
///
/// The `extract_bid` closure decodes extrinsic bytes using concrete runtime
/// types (`UncheckedExtrinsic`, `RuntimeCall`) and returns
/// `(commitment, filler_encoded, user_op)` for bid calls, or `None`.
pub async fn run_bid_watcher<P, Block, F>(
	pool: Arc<P>,
	bid_cache: Arc<BidCache>,
	bid_sender: broadcast::Sender<RpcBidInfo>,
	extract_bid: F,
) where
	Block: BlockT,
	P: TransactionPool<Block = Block> + 'static,
	F: Fn(&[u8]) -> Option<(H256, Vec<u8>, Vec<u8>)> + Send + 'static,
{
	use futures::StreamExt;

	let mut stream = pool.import_notification_stream();

	while let Some(tx_hash) = stream.next().await {
		let tx = match pool.ready_transaction(&tx_hash) {
			Some(tx) => tx,
			None => continue,
		};

		let extrinsic_bytes = tx.data().encode();

		if let Some((commitment, filler, user_op)) = extract_bid(&extrinsic_bytes) {
			log::info!(
				target: LOG_TARGET,
				"bid in mempool for {commitment:?}",
			);
			bid_cache.insert(commitment, filler.clone(), user_op.clone());

			let _ = bid_sender.send(RpcBidInfo { commitment, filler, user_op });
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

}
