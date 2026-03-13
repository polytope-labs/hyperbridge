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
use polkadot_sdk::{frame_support::traits::IsSubType, *};
use sc_client_api::{Backend, StorageProvider};
use sc_transaction_pool_api::{InPoolTransaction, TransactionPool};
use serde::{Deserialize, Serialize};
use sp_blockchain::HeaderBackend;
use sp_core::{
	offchain::{OffchainStorage, STORAGE_PREFIX},
	H256,
};
use sp_runtime::traits::Block as BlockT;
use std::{
	collections::{BTreeSet, HashMap},
	sync::{Arc, RwLock},
	time::{Duration, Instant},
};
use tokio::sync::broadcast;

pub use pallet_intents_coprocessor;

const LOG_TARGET: &str = "intents-rpc";

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct RpcBidInfo {
	pub commitment: H256,
	#[serde(with = "hex_bytes")]
	pub filler: Vec<u8>,
	#[serde(with = "hex_bytes")]
	pub user_op: Vec<u8>,
}

/// A single price entry returned by the RPC
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct RpcPriceEntry {
	/// The filler's EVM address (zero address for unverified submissions)
	#[serde(with = "hex_bytes")]
	pub filler: Vec<u8>,
	/// Lower bound of the base token amount range (inclusive), with 18 decimal places
	pub range_start: String,
	/// Upper bound of the base token amount range (inclusive), with 18 decimal places
	pub range_end: String,
	/// The price of the base token in the quote token, with 18 decimal places
	pub price: String,
	/// Timestamp of submission (seconds)
	pub timestamp: u64,
}

/// Response for the `intents_getPairPrices` RPC method
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct RpcPairPrices {
	/// High confidence prices (from verified fillers with proofs)
	pub verified: Vec<RpcPriceEntry>,
	/// Low confidence prices (from unverified submitters)
	pub unverified: Vec<RpcPriceEntry>,
}

impl Ord for RpcBidInfo {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.filler.cmp(&other.filler)
	}
}

impl PartialOrd for RpcBidInfo {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
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

	pub fn insert(
		&self,
		commitment: H256,
		filler: Vec<u8>,
		user_op: Vec<u8>,
	) -> Result<(), String> {
		let entry = BidEntry { filler: filler.clone(), user_op };

		let mut bids = self.bids.write().map_err(|e| format!("BidCache lock poisoned: {e}"))?;
		let order = bids
			.entry(commitment)
			.or_insert_with(|| OrderBids { first_seen: Instant::now(), entries: Vec::new() });
		if let Some(existing) = order.entries.iter_mut().find(|e| e.filler == filler) {
			*existing = entry;
		} else {
			order.entries.push(entry);
		}
		Ok(())
	}

	pub fn get_bids(&self, commitment: &H256) -> Result<BTreeSet<RpcBidInfo>, String> {
		let bids = self.bids.read().map_err(|e| format!("BidCache lock poisoned: {e}"))?;
		Ok(bids
			.get(commitment)
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
			.unwrap_or_default())
	}

	pub fn remove_expired(&self) -> Result<(), String> {
		let now = Instant::now();
		let mut bids = self.bids.write().map_err(|e| format!("BidCache lock poisoned: {e}"))?;
		bids.retain(|_commitment, order| now.duration_since(order.first_seen) < self.ttl);
		Ok(())
	}
}

fn runtime_error_into_rpc_error(e: impl std::fmt::Display) -> ErrorObjectOwned {
	ErrorObject::owned(9877, format!("{e}"), None::<String>)
}

/// Construct the full storage key for a `StorageMap` entry with `Blake2_128Concat` hasher.
fn storage_map_key(pallet: &[u8], storage: &[u8], map_key: &H256) -> Vec<u8> {
	let mut key = Vec::new();
	key.extend_from_slice(&sp_core::hashing::twox_128(pallet));
	key.extend_from_slice(&sp_core::hashing::twox_128(storage));
	let map_key_bytes = map_key.as_bytes();
	key.extend_from_slice(&sp_core::hashing::blake2_128(map_key_bytes));
	key.extend_from_slice(map_key_bytes);
	key
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

	/// Get all prices for a token pair, separated by confidence level
	#[method(name = "intents_getPairPrices")]
	fn get_pair_prices(&self, pair_id: H256) -> RpcResult<RpcPairPrices>;

	#[subscription(name = "intents_subscribeBids" => "intents_bidNotification", unsubscribe = "intents_unsubscribeBids", item = RpcBidInfo)]
	async fn subscribe_bids(&self, commitment: Option<H256>) -> SubscriptionResult;
}

pub struct IntentsRpcHandler<C, Block, S, T> {
	client: Arc<C>,
	offchain_storage: S,
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
		let offchain_storage = backend
			.offchain_storage()
			.ok_or_else(|| "Offchain storage not available in backend".to_string())?;
		Ok(Self { client, offchain_storage, bid_cache, bid_sender, _marker: Default::default() })
	}
}

#[async_trait]
impl<C, Block, S, T> IntentsApiServer for IntentsRpcHandler<C, Block, S, T>
where
	Block: BlockT,
	S: OffchainStorage + Clone + Send + Sync + 'static,
	T: Backend<Block> + Send + Sync + 'static,
	C: HeaderBackend<Block> + StorageProvider<Block, T> + Send + Sync + 'static,
{
	fn get_bids_for_order(&self, commitment: H256) -> RpcResult<Vec<RpcBidInfo>> {
		// Get mempool bids from in-memory cache
		let mut bids =
			self.bid_cache.get_bids(&commitment).map_err(runtime_error_into_rpc_error)?;

		// Query on-chain bids via StorageProvider and read bid data from offchain storage.
		let best_hash = self.client.info().best_hash;
		let prefix = bids_storage_prefix(&commitment);
		let prefix_key = sp_core::storage::StorageKey(prefix.clone());

		let keys = self
			.client
			.storage_keys(best_hash, Some(&prefix_key), None)
			.map_err(runtime_error_into_rpc_error)?;

		const MAX_ON_CHAIN_BIDS: usize = 30;

		for key in keys.take(MAX_ON_CHAIN_BIDS) {
			// Key layout after prefix: blake2_128(filler) ++ filler_encoded
			let filler_start = prefix.len() + 16;
			if key.0.len() > filler_start {
				let filler_encoded = &key.0[filler_start..];

				let offchain_key =
					pallet_intents_coprocessor::offchain_bid_key_raw(&commitment, filler_encoded);

				if let Some(data) = self.offchain_storage.get(STORAGE_PREFIX, &offchain_key) {
					// Bid encoding: filler.encode() ++ user_op.encode()
					if data.len() > filler_encoded.len() {
						if let Ok(user_op) = Vec::<u8>::decode(&mut &data[filler_encoded.len()..]) {
							bids.insert(RpcBidInfo {
								commitment,
								filler: filler_encoded.to_vec(),
								user_op,
							});
						}
					}
				}
			}
		}

		Ok(bids.into_iter().collect())
	}

	fn get_pair_prices(&self, pair_id: H256) -> RpcResult<RpcPairPrices> {
		let best_hash = self.client.info().best_hash;

		let decode_entries = |storage_name: &[u8]| -> Vec<RpcPriceEntry> {
			let key = storage_map_key(b"IntentsCoprocessor", storage_name, &pair_id);
			let storage_key = sp_core::storage::StorageKey(key);

			let data = match self.client.storage(best_hash, &storage_key) {
				Ok(Some(data)) => data.0,
				_ => return Vec::new(),
			};

			// Decode Vec<PriceEntry>
			// PriceEntry SCALE-encodes as (H160, U256, U256, U256, u64)
			type Entry = (
				primitive_types::H160,
				primitive_types::U256,
				primitive_types::U256,
				primitive_types::U256,
				u64,
			);
			match Vec::<Entry>::decode(&mut &data[..]) {
				Ok(entries) => entries
					.into_iter()
					.map(|(filler, range_start, range_end, price, timestamp)| RpcPriceEntry {
						filler: filler.as_bytes().to_vec(),
						range_start: range_start.to_string(),
						range_end: range_end.to_string(),
						price: price.to_string(),
						timestamp,
					})
					.collect(),
				Err(_) => Vec::new(),
			}
		};

		Ok(RpcPairPrices {
			verified: decode_entries(b"VerifiedPrices"),
			unverified: decode_entries(b"UnverifiedPrices"),
		})
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

/// Extract a bid from encoded extrinsic bytes using generic runtime types.
///
/// Decodes the extrinsic and uses `IsSubType` to extract the pallet-level
/// `place_bid` call, returning `(commitment, filler_encoded, user_op)`.
pub fn extract_bid<T, Extra>(encoded: &[u8]) -> Option<(H256, Vec<u8>, Vec<u8>)>
where
	T: pallet_intents_coprocessor::Config,
	T::RuntimeCall: frame_support::traits::IsSubType<pallet_intents_coprocessor::Call<T>> + Decode,
	T::AccountId: Encode + From<[u8; 32]>,
	Extra: Decode,
{
	let xt = sp_runtime::generic::UncheckedExtrinsic::<
		sp_runtime::MultiAddress<T::AccountId, ()>,
		T::RuntimeCall,
		sp_runtime::MultiSignature,
		Extra,
	>::decode(&mut &encoded[..])
	.ok()?;

	let filler = match &xt.preamble {
		sp_runtime::generic::Preamble::Signed(address, _, _) => match address {
			sp_runtime::MultiAddress::Id(id) => id.encode(),
			_ => return None,
		},
		_ => return None,
	};

	match xt.function.is_sub_type()? {
		pallet_intents_coprocessor::Call::place_bid { commitment, user_op } =>
			Some((commitment.clone(), filler, user_op.to_vec())),
		_ => None,
	}
}

/// Watches the tx pool for bid-related extrinsics, updating the cache and
/// notifying subscribers as they arrive. Also periodically cleans up expired
/// cache entries.
///
/// Generic over the runtime config `T` and signed extensions `Extra`.
pub async fn run_bid_watcher<P, Block, T, Extra>(
	pool: Arc<P>,
	bid_cache: Arc<BidCache>,
	bid_sender: broadcast::Sender<RpcBidInfo>,
	cleanup_interval: Duration,
) where
	Block: BlockT,
	P: TransactionPool<Block = Block> + 'static,
	T: pallet_intents_coprocessor::Config,
	T::RuntimeCall: frame_support::traits::IsSubType<pallet_intents_coprocessor::Call<T>> + Decode,
	T::AccountId: Encode + From<[u8; 32]>,
	Extra: Decode + Send + 'static,
{
	use futures::StreamExt;

	let mut stream = pool.import_notification_stream();
	let mut timer = tokio::time::interval(cleanup_interval);

	loop {
		tokio::select! {
			Some(tx_hash) = stream.next() => {
				let tx = match pool.ready_transaction(&tx_hash) {
					Some(tx) => tx,
					None => continue,
				};

				let extrinsic_bytes = tx.data().encode();

				if let Some((commitment, filler, user_op)) = extract_bid::<T, Extra>(&extrinsic_bytes) {
					log::info!(
						target: LOG_TARGET,
						"bid in mempool for {commitment:?}",
					);
					if let Err(e) = bid_cache.insert(commitment, filler.clone(), user_op.clone()) {
						log::warn!(target: LOG_TARGET, "failed to cache bid: {e}");
						continue;
					}

					let _ = bid_sender.send(RpcBidInfo { commitment, filler, user_op });
				}
			}
			_ = timer.tick() => {
				if let Err(e) = bid_cache.remove_expired() {
					log::warn!(target: LOG_TARGET, "failed to clean bid cache: {e}");
				}
			}
		}
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
		c.insert(key, vec![1, 2, 3], vec![4, 5, 6]).unwrap();

		let bids: Vec<_> = c.get_bids(&key).unwrap().into_iter().collect();
		assert_eq!(bids.len(), 1);
		assert_eq!(bids[0].filler, vec![1, 2, 3]);
		assert_eq!(bids[0].user_op, vec![4, 5, 6]);
	}

	#[test]
	fn multiple_fillers_same_commitment() {
		let c = cache();
		let key = H256::random();

		c.insert(key, vec![1], vec![10]).unwrap();
		c.insert(key, vec![2], vec![20]).unwrap();

		let bids = c.get_bids(&key).unwrap();
		assert_eq!(bids.len(), 2);
		assert!(bids.iter().any(|b| b.filler == vec![1]));
		assert!(bids.iter().any(|b| b.filler == vec![2]));
	}

	#[test]
	fn duplicate_filler_replaces_previous_bid() {
		let c = cache();
		let key = H256::random();

		c.insert(key, vec![1], vec![10]).unwrap();
		c.insert(key, vec![1], vec![99]).unwrap();

		let bids: Vec<_> = c.get_bids(&key).unwrap().into_iter().collect();
		assert_eq!(bids.len(), 1);
		assert_eq!(bids[0].user_op, vec![99]);
	}

	#[test]
	fn unknown_commitment_returns_empty() {
		assert!(cache().get_bids(&H256::random()).unwrap().is_empty());
	}

	#[test]
	fn expired_entries_are_removed() {
		let c = BidCache::new(Duration::from_millis(50));
		let key = H256::random();

		c.insert(key, vec![1], vec![10]).unwrap();
		std::thread::sleep(Duration::from_millis(100));
		c.remove_expired().unwrap();

		assert!(c.get_bids(&key).unwrap().is_empty());
	}
}
