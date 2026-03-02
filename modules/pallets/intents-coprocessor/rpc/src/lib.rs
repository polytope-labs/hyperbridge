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
use sc_transaction_pool_api::{InPoolTransaction, TransactionPool};
use serde::{Deserialize, Serialize};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::H256;
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
	observed_at: Instant,
	confirmed: bool,
}

/// In-memory bid cache backed by SQLite for persistence across restarts.
pub struct BidCache {
	bids: RwLock<HashMap<H256, Vec<BidEntry>>>,
	db: std::sync::Mutex<rusqlite::Connection>,
	ttl: Duration,
}

impl BidCache {
	pub fn new(db_path: &str, ttl: Duration) -> Result<Self, rusqlite::Error> {
		let conn = rusqlite::Connection::open(db_path)?;
		conn.execute_batch(
			"CREATE TABLE IF NOT EXISTS bids (
				commitment BLOB NOT NULL,
				filler BLOB NOT NULL,
				user_op BLOB NOT NULL,
				confirmed INTEGER NOT NULL DEFAULT 0,
				PRIMARY KEY (commitment, filler)
			)",
		)?;

		let bids = Self::load_bids_from_db(&conn)?;
		Ok(Self { bids: RwLock::new(bids), db: std::sync::Mutex::new(conn), ttl })
	}

	fn load_bids_from_db(
		conn: &rusqlite::Connection,
	) -> Result<HashMap<H256, Vec<BidEntry>>, rusqlite::Error> {
		let mut bids: HashMap<H256, Vec<BidEntry>> = HashMap::new();
		let mut stmt = conn.prepare("SELECT commitment, filler, user_op, confirmed FROM bids")?;
		let rows = stmt.query_map([], |row| {
			let commitment_bytes: Vec<u8> = row.get(0)?;
			let filler: Vec<u8> = row.get(1)?;
			let user_op: Vec<u8> = row.get(2)?;
			let confirmed: bool = row.get(3)?;
			Ok((commitment_bytes, filler, user_op, confirmed))
		})?;

		for row in rows {
			let (commitment_bytes, filler, user_op, confirmed) = row?;
			if commitment_bytes.len() == 32 {
				let commitment = H256::from_slice(&commitment_bytes);
				bids.entry(commitment).or_default().push(BidEntry {
					filler,
					user_op,
					observed_at: Instant::now(),
					confirmed,
				});
			}
		}

		Ok(bids)
	}

	fn persist_upsert(&self, commitment: &H256, filler: &[u8], user_op: &[u8]) {
		if let Ok(db) = self.db.lock() {
			if let Err(e) = db.execute(
				"INSERT OR REPLACE INTO bids (commitment, filler, user_op, confirmed) \
				 VALUES (?1, ?2, ?3, 0)",
				rusqlite::params![commitment.as_bytes().to_vec(), filler, user_op],
			) {
				log::warn!(target: LOG_TARGET, "Failed to persist bid: {e}");
			}
		}
	}

	fn persist_delete(&self, commitment: &H256, filler: &[u8]) {
		if let Ok(db) = self.db.lock() {
			if let Err(e) = db.execute(
				"DELETE FROM bids WHERE commitment = ?1 AND filler = ?2",
				rusqlite::params![commitment.as_bytes().to_vec(), filler],
			) {
				log::warn!(target: LOG_TARGET, "Failed to delete bid: {e}");
			}
		}
	}

	pub fn insert(&self, commitment: H256, filler: Vec<u8>, user_op: Vec<u8>) {
		let entry = BidEntry {
			filler: filler.clone(),
			user_op: user_op.clone(),
			observed_at: Instant::now(),
			confirmed: false,
		};

		{
			let mut bids = self.bids.write().expect("BidCache lock poisoned");
			let entries = bids.entry(commitment).or_default();
			if let Some(existing) = entries.iter_mut().find(|e| e.filler == filler) {
				*existing = entry;
			} else {
				entries.push(entry);
			}
		}

		self.persist_upsert(&commitment, &filler, &user_op);
	}

	pub fn remove_bid(&self, commitment: &H256, filler: &[u8]) {
		{
			let mut bids = self.bids.write().expect("BidCache lock poisoned");
			if let Some(entries) = bids.get_mut(commitment) {
				entries.retain(|e| e.filler != filler);
				if entries.is_empty() {
					bids.remove(commitment);
				}
			}
		}

		self.persist_delete(commitment, filler);
	}

	pub fn get_bids(&self, commitment: &H256) -> Vec<RpcBidInfo> {
		let bids = self.bids.read().expect("BidCache lock poisoned");
		bids.get(commitment)
			.map(|entries| {
				entries
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
		let mut expired: Vec<H256> = Vec::new();

		{
			let mut bids = self.bids.write().expect("BidCache lock poisoned");
			bids.retain(|commitment, entries| {
				entries.retain(|e| now.duration_since(e.observed_at) < self.ttl);
				if entries.is_empty() {
					expired.push(*commitment);
					false
				} else {
					true
				}
			});
		}

		if expired.is_empty() {
			return;
		}

		if let Ok(db) = self.db.lock() {
			for commitment in &expired {
				if let Err(e) = db.execute(
					"DELETE FROM bids WHERE commitment = ?1",
					rusqlite::params![commitment.as_bytes().to_vec()],
				) {
					log::warn!(target: LOG_TARGET, "Failed to delete expired bid: {e}");
				}
			}
		}
	}

	pub fn confirm_bid(&self, commitment: &H256, filler: &[u8]) {
		{
			let mut bids = self.bids.write().expect("BidCache lock poisoned");
			if let Some(entries) = bids.get_mut(commitment) {
				if let Some(entry) = entries.iter_mut().find(|e| e.filler == filler) {
					entry.confirmed = true;
				}
			}
		}

		if let Ok(db) = self.db.lock() {
			if let Err(e) = db.execute(
				"UPDATE bids SET confirmed = 1 WHERE commitment = ?1 AND filler = ?2",
				rusqlite::params![commitment.as_bytes().to_vec(), filler],
			) {
				log::warn!(target: LOG_TARGET, "Failed to confirm bid: {e}");
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

pub struct IntentsRpcHandler {
	bid_cache: Arc<BidCache>,
	bid_sender: broadcast::Sender<RpcBidInfo>,
}

impl IntentsRpcHandler {
	pub fn new(bid_cache: Arc<BidCache>, bid_sender: broadcast::Sender<RpcBidInfo>) -> Self {
		Self { bid_cache, bid_sender }
	}
}

#[async_trait]
impl IntentsApiServer for IntentsRpcHandler {
	fn get_bids_for_order(&self, commitment: H256) -> RpcResult<Vec<RpcBidInfo>> {
		Ok(self.bid_cache.get_bids(&commitment))
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

				let _ = bid_sender.send(RpcBidInfo {
					commitment,
					filler,
					user_op,
					confirmed: false,
				});
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
