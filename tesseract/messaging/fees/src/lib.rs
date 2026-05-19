//! This module allows the relayer to maintain a local database
//! of requests and responses the relayer has delivered successfully

#![allow(unused_imports)]
#![allow(unused)]

/// Log/tracing target for this crate.
pub const LOG_TARGET: &str = "messaging-incentives";
use crate::db::{
	deliveries::{Data, OrderByParam, UniqueWhereParam, WhereParam},
	new_client_with_url,
	read_filters::{IntFilter, StringFilter},
	PrismaClient, PrismaClientBuilder,
};
use anyhow::anyhow;
use codec::{Decode, Encode};
use futures::{stream, StreamExt};
use ismp::{
	consensus::StateMachineHeight,
	host::StateMachine,
	messaging::{hash_request, hash_response, Keccak256, Message, Proof},
	router::{PostRequest, Request},
};
use itertools::Itertools;
use pallet_ismp_relayer::{
	beneficiary_message,
	withdrawal::{Signature, WithdrawalProof},
};
use primitive_types::{H256, U256};
use prisma_client_rust::{query_core::RawQuery, BatchItem, Direction, PrismaValue, Raw};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, mem::discriminant, sync::Arc};
use tesseract_primitives::{
	HyperbridgeClaim, IsmpProvider, StateProofQueryType, TxReceipt, WithdrawFundsResult,
};

mod db;
#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct TransactionPayment {
	pub db: Arc<PrismaClient>,
}

/// Status value written into [`OutboundRotationClaims.status`] for every
/// inserted row. There's no other status — the row is deleted on success.
/// Kept for backward compatibility with the existing schema and for easy
/// inspection with `sqlite3`.
pub mod outbound_rotation_claim_status {
	pub const PENDING: &str = "pending";
}

/// Row view of a persisted outbound-consensus delivery claim. The row is
/// inserted when the outbound delivery task pushes a trigger and deleted
/// once the claim extrinsic lands on Hyperbridge. Anything still present
/// after a relayer restart is a crash-recovery candidate.
#[derive(Debug, Clone, ::serde::Deserialize)]
pub struct OutboundRotationClaimRow {
	pub dest: String,
	pub set_id: i64,
	pub rotation_height: i64,
}

impl TransactionPayment {
	/// Create the local database if it does not exist
	pub async fn initialize(url: &str) -> anyhow::Result<Self> {
		let url = format!("file:{}", url);
		let client = new_client_with_url(&url).await?;
		#[cfg(debug_assertions)]
		client._db_push().await?;
		#[cfg(not(debug_assertions))]
		client._migrate_deploy().await?;
		Ok(Self { db: Arc::new(client) })
	}

	/// Record one or more newly-delivered mandatory rotations as pending
	/// claims in a single round-trip. The outbound task calls this right
	/// after a successful delivery so the claims survive a relayer
	/// restart. Each `upsert` is idempotent on the `(dest, set_id)`
	/// unique key, so duplicate `set_id`s in the input (or a retry of the
	/// same call) are no-ops, letting the caller be sloppy about
	/// deduplicating.
	///
	/// `rows` carries `(set_id, delivery_height)` pairs — `delivery_height`
	/// is the destination block in which the `NewEpoch` log was emitted,
	/// so each pending row pins its claim to the height where the
	/// HandlerV2 `_epochs[set_id]` slot was actually written. Earlier
	/// versions of this function took a single `rotation_height` shared
	/// across every set_id, which forced callers to guess one (typically
	/// `query_finalized_height`) and was the source of the
	/// "Error fetching latest state machine height" / `OutboundDeliveryNotProven`
	/// races at outbound dispatch time.
	pub async fn insert_pending_rotation_claims(
		&self,
		destination: &str,
		rows: &[(u64, u64)],
	) -> anyhow::Result<()> {
		use crate::db::outbound_rotation_claims;
		if rows.is_empty() {
			return Ok(());
		}
		let now = chrono::Utc::now().timestamp() as i32;
		let actions: Vec<_> = rows
			.iter()
			.map(|(set_id, rotation_height)| {
				self.db.outbound_rotation_claims().upsert(
					outbound_rotation_claims::UniqueWhereParam::DestSetIdEquals(
						destination.to_string(),
						*set_id as i64,
					),
					outbound_rotation_claims::create(
						destination.to_string(),
						*set_id as i64,
						*rotation_height as i64,
						outbound_rotation_claim_status::PENDING.to_string(),
						now,
						now,
						vec![],
					),
					vec![],
				)
			})
			.collect();
		self.db._batch(actions).await?;
		Ok(())
	}

	/// Delete the persisted claim row after the extrinsic lands on
	/// Hyperbridge. Best-effort: if the delete fails, the next startup
	/// will replay the row and the pallet's `(dest, set_id)` idempotency
	/// tag will reject the duplicate.
	pub async fn delete_rotation_claim(
		&self,
		destination: &str,
		set_id: u64,
	) -> anyhow::Result<()> {
		use crate::db::{
			outbound_rotation_claims::WhereParam,
			read_filters::{BigIntFilter, StringFilter},
		};
		// `delete_many` is a no-op when the row is absent (e.g. the row
		// was already deleted by a successful prior run, or never inserted).
		self.db
			.outbound_rotation_claims()
			.delete_many(vec![
				WhereParam::Dest(StringFilter::Equals(destination.to_string())),
				WhereParam::SetId(BigIntFilter::Equals(set_id as i64)),
			])
			.exec()
			.await?;
		Ok(())
	}

	/// Load every claim still in `pending`, ordered by creation time. The
	/// claim task calls this at startup and replays each one through its
	/// normal processing path.
	pub async fn list_pending_rotation_claims(
		&self,
	) -> anyhow::Result<Vec<OutboundRotationClaimRow>> {
		use crate::db::outbound_rotation_claims::OrderByParam;
		let rows = self
			.db
			.outbound_rotation_claims()
			.find_many(vec![])
			.order_by(OrderByParam::CreatedAt(prisma_client_rust::Direction::Asc))
			.exec()
			.await?;
		Ok(rows
			.into_iter()
			.map(|data| OutboundRotationClaimRow {
				dest: data.dest,
				set_id: data.set_id,
				rotation_height: data.rotation_height,
			})
			.collect())
	}

	/// Query all deliveries in the db and make them unique by the source & destination pair
	pub async fn distinct_deliveries(&self) -> anyhow::Result<Vec<Data>> {
		let deliveries = self.db.deliveries().find_many(vec![]).exec().await?;
		let data = deliveries
			.into_iter()
			.unique_by(|data| {
				let mut pair = vec![data.source_chain.clone(), data.dest_chain.clone()];
				pair.sort();
				pair.concat()
			})
			.collect();

		Ok(data)
	}

	/// Store entries for delivered post requests
	pub async fn store_messages(&self, receipts: Vec<TxReceipt>) -> anyhow::Result<()> {
		let mut actions = vec![];
		for TxReceipt { query, height } in receipts {
			let action = self.db.deliveries().create(
				hex::encode(query.commitment.as_bytes()),
				query.source_chain.to_string(),
				query.dest_chain.to_string(),
				DeliveryType::PostRequest as i32,
				chrono::Utc::now().timestamp() as i32,
				height as i32,
				Default::default(),
			);

			actions.push(action);
		}
		self.db._batch(actions).await?;
		Ok(())
	}

	/// Delete the requests with the provided hashes from the database
	pub async fn delete_entries(&self, reqs: Vec<Vec<u8>>) -> anyhow::Result<()> {
		let actions = reqs
			.into_iter()
			.map(|hash| {
				self.db.deliveries().delete_many(vec![WhereParam::Hash(StringFilter::Equals(
					hex::encode(hash.as_slice()),
				))])
			})
			.collect::<Vec<_>>();
		self.db._batch(actions).await?;
		Ok(())
	}

	pub async fn highest_delivery_height(
		&self,
		source_chain: StateMachine,
		dest_chain: StateMachine,
	) -> Result<Option<u64>, anyhow::Error> {
		let request_entries = self
			.db
			.deliveries()
			.find_many(vec![
				WhereParam::SourceChain(StringFilter::Equals(source_chain.to_string())),
				WhereParam::DestChain(StringFilter::Equals(dest_chain.to_string())),
				WhereParam::DeliveryType(IntFilter::Equals(DeliveryType::PostRequest as i32)),
			])
			.order_by(OrderByParam::Height(Direction::Asc))
			.exec()
			.await?;

		let dest_height = request_entries.last().map(|data| data.height as u64).unwrap_or_default();

		if dest_height == 0 {
			Ok(None)
		} else {
			Ok(Some(dest_height))
		}
	}

	pub async fn query_state_proofs<H: HyperbridgeClaim + Sync>(
		&self,
		source: Arc<dyn IsmpProvider>,
		dest: Arc<dyn IsmpProvider>,
		source_height: u64,
		dest_height: u64,
		keys: Vec<(H256, Vec<Vec<u8>>, Vec<Vec<u8>>)>,
		hyperbridge: &H,
	) -> Result<Vec<WithdrawalProof>, anyhow::Error> {
		let mut proofs = vec![];
		let source_chain = source.state_machine_id().state_id;
		let cross_chain_type = discriminant(&source.state_machine_id().state_id) !=
			discriminant(&dest.state_machine_id().state_id);
		// One signature is consumed per submitted proof, so each chunk increments
		// the locally-tracked nonce from the snapshot we read at the start.
		let mut nonce = if cross_chain_type {
			hyperbridge.relayer_nonce(dest.address(), source_chain).await?
		} else {
			0
		};
		// Chunk keys by 50 each
		for chunk in keys.chunks(50) {
			// Gather keys to be queried on the source chain
			let mut source_chain_storage_keys = vec![];
			let mut dest_chain_storage_keys = vec![];
			let mut request_response_commitments = vec![];
			for (key, source_key, dest_key) in chunk {
				source_chain_storage_keys.push(source_key.clone());
				dest_chain_storage_keys.push(dest_key.clone());
				request_response_commitments.push(key.clone());
			}

			let source_proof = source
				.query_state_proof(
					source_height,
					StateProofQueryType::Ismp(
						source_chain_storage_keys.into_iter().flatten().collect(),
					),
				)
				.await?;

			let dest_proof = dest
				.query_state_proof(
					dest_height,
					StateProofQueryType::Ismp(
						dest_chain_storage_keys.into_iter().flatten().collect(),
					),
				)
				.await?;

			let beneficiary_details = if cross_chain_type {
				let beneficiary = source.address();
				let prehash = beneficiary_message(nonce, source_chain, &beneficiary);
				let details = Some((beneficiary, dest.sign(&prehash)));
				nonce += 1;
				details
			} else {
				None
			};

			let proof = WithdrawalProof {
				commitments: request_response_commitments,
				source_proof: Proof {
					height: StateMachineHeight {
						id: source.state_machine_id(),
						height: source_height,
					},
					proof: source_proof,
				},
				dest_proof: Proof {
					height: StateMachineHeight { id: dest.state_machine_id(), height: dest_height },
					proof: dest_proof,
				},
				beneficiary_details,
			};

			proofs.push(proof)
		}

		Ok(proofs)
	}

	// todo: Consolidate the state proof query into a single function
	/// Create payment claim proof for all deliveries of requests from source to dest.
	pub async fn create_proof_from_receipts<H: HyperbridgeClaim + Sync>(
		&self,
		source_height: u64,
		dest_height: u64,
		source: Arc<dyn IsmpProvider>,
		dest: Arc<dyn IsmpProvider>,
		receipts: Vec<TxReceipt>,
		hyperbridge: &H,
	) -> anyhow::Result<Vec<WithdrawalProof>> {
		let keys = receipts
			.iter()
			.map(|TxReceipt { query, .. }| {
				let source_key = source.request_commitment_full_key(query.commitment);
				let dest_key = dest.request_receipt_full_key(query.commitment);
				(query.commitment, source_key, dest_key)
			})
			.collect::<Vec<_>>();

		self.query_state_proofs(source, dest, source_height, dest_height, keys, hyperbridge)
			.await
	}

	/// Fetch all pending withdrawals from the db, returns their id so they can be deleted.
	pub async fn pending_withdrawals(
		&self,
		dest: &StateMachine,
	) -> Result<Vec<(WithdrawFundsResult, i32)>, anyhow::Error> {
		let pending = self
			.db
			.pending_withdrawal()
			.find_many(vec![db::pending_withdrawal::WhereParam::Dest(StringFilter::Equals(
				dest.to_string(),
			))])
			.exec()
			.await?
			.into_iter()
			.map(|record| Ok((WithdrawFundsResult::decode(&mut &record.encoded[..])?, record.id)))
			.collect::<Result<Vec<_>, anyhow::Error>>()?;

		Ok(pending)
	}

	/// Delete any pending withdrawals
	pub async fn delete_pending_withdrawals(&self, pending: Vec<i32>) -> Result<(), anyhow::Error> {
		let actions = pending
			.into_iter()
			.map(|item| {
				self.db
					.pending_withdrawal()
					.delete(db::pending_withdrawal::UniqueWhereParam::IdEquals(item))
			})
			.collect::<Vec<_>>();

		self.db._batch(actions).await?;

		Ok(())
	}

	/// Store a pending withdrawal request
	pub async fn store_pending_withdrawals(
		&self,
		pending: Vec<WithdrawFundsResult>,
	) -> Result<Vec<i32>, anyhow::Error> {
		let actions = pending
			.into_iter()
			.map(|item| {
				self.db.pending_withdrawal().create(
					item.post.dest.to_string(),
					item.encode(),
					vec![],
				)
			})
			.collect::<Vec<_>>();

		let ids = self.db._batch(actions).await?.into_iter().map(|record| record.id).collect();

		Ok(ids)
	}

	/// Fetch all unprofitable messages from the db, returns their id so they can be deleted.
	pub async fn unprofitable_messages(
		&self,
		dest: &StateMachine,
	) -> Result<Vec<(Message, i32)>, anyhow::Error> {
		let data = self
			.db
			.unprofitable_messages()
			.find_many(vec![db::unprofitable_messages::WhereParam::Dest(StringFilter::Equals(
				dest.to_string(),
			))])
			.exec()
			.await?;
		// Dedup data
		let mut ids = vec![];
		let mut duplicates = vec![];
		let mut data_set = BTreeSet::new();
		data.into_iter().for_each(|data| {
			let new = data_set.insert(data.encoded);
			if new {
				ids.push(data.id)
			} else {
				duplicates.push(data.id);
			}
		});
		// Delete any duplicates in background
		// A previous delete operation could have failed causing us to have duplicate messages in
		// the db
		let tx = self.clone();
		tokio::spawn(async move {
			let _ = tx.delete_unprofitable_messages(duplicates).await;
		});

		let unprofitable = data_set
			.into_iter()
			.zip(ids)
			.map(|(encoded, id)| Ok((Message::decode(&mut &encoded[..])?, id)))
			.collect::<Result<Vec<_>, anyhow::Error>>()?;

		Ok(unprofitable)
	}

	/// Delete any unprofitable message
	pub async fn delete_unprofitable_messages(
		&self,
		unprofitable: impl IntoIterator<Item = i32>,
	) -> Result<(), anyhow::Error> {
		let actions = unprofitable
			.into_iter()
			.map(|item| {
				self.db
					.unprofitable_messages()
					.delete(db::unprofitable_messages::UniqueWhereParam::IdEquals(item))
			})
			.collect::<Vec<_>>();

		self.db._batch(actions).await?;

		Ok(())
	}

	/// Store unprofitable messages
	pub async fn store_unprofitable_messages(
		&self,
		unprofitable: Vec<Message>,
		dest: StateMachine,
	) -> Result<Vec<i32>, anyhow::Error> {
		let actions = unprofitable
			.into_iter()
			.map(|item| {
				self.db.unprofitable_messages().create(dest.to_string(), item.encode(), vec![])
			})
			.collect::<Vec<_>>();

		let ids = self.db._batch(actions).await?.into_iter().map(|record| record.id).collect();

		Ok(ids)
	}

	/// Create payment claim proof for all deliveries of requests and responses from source to dest
	/// a number of days The default is 30 days
	pub async fn create_claim_proof<H: HyperbridgeClaim + Sync>(
		&self,
		source_height: u64,
		dest_height: u64,
		source: Arc<dyn IsmpProvider>,
		dest: Arc<dyn IsmpProvider>,
		hyperbridge: &H,
	) -> anyhow::Result<Vec<WithdrawalProof>> {
		let source_chain = source.state_machine_id().state_id;
		let dest_chain = dest.state_machine_id().state_id;
		let request_entries = self
			.db
			.deliveries()
			.find_many(vec![
				WhereParam::SourceChain(StringFilter::Equals(source_chain.to_string())),
				WhereParam::DestChain(StringFilter::Equals(dest_chain.to_string())),
				WhereParam::DeliveryType(IntFilter::Equals(DeliveryType::PostRequest as i32)),
			])
			.exec()
			.await?;

		if request_entries.is_empty() {
			return Ok(Default::default());
		}

		let requests = request_entries.iter().filter_map(|data| {
			// Get request commitment keys on source chain
			let hash = H256::from_slice(&hex::decode(data.hash.clone()).ok()?);
			let source_key = source.request_commitment_full_key(hash);
			//Get request receipt keys on dest chain
			let dest_key = dest.request_receipt_full_key(hash);
			Some((hash, source_key, dest_key))
		});

		let mut keys_to_delete = vec![];
		let mut keys_to_prove = vec![];

		for entry in requests {
			let hash = entry.0;
			let fee = source.query_request_fee_metadata(hash).await?;

			if fee.is_zero() {
				keys_to_delete.push(hash);
				continue;
			}

			if hyperbridge.check_claimed(hash).await? {
				keys_to_delete.push(hash);
				continue;
			}
			keys_to_prove.push(entry);
		}

		let tx = self.clone();
		// Delete claimed keys in the background
		tokio::spawn(async move {
			match tx.delete_claimed_entries(keys_to_delete).await {
				Err(_) => {
					tracing::error!(target: LOG_TARGET, "An Error occurred while deleting claimed fees from the db, the claimed keys will be deleted in the next fee accumulation attempt");
				},
				_ => {},
			}
		});

		self.query_state_proofs(
			source,
			dest,
			source_height,
			dest_height,
			keys_to_prove,
			hyperbridge,
		)
		.await
	}

	pub async fn delete_claimed_entries(&self, commitments: Vec<H256>) -> anyhow::Result<()> {
		if !commitments.is_empty() {
			// Remove claimed entries from db
			let entries = commitments.into_iter().map(|req| req.0.to_vec()).collect();

			self.delete_entries(entries).await?;
		}
		Ok(())
	}
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub enum DeliveryType {
	PostRequest = 0,
}

impl TryFrom<i32> for DeliveryType {
	type Error = anyhow::Error;
	fn try_from(value: i32) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(Self::PostRequest),
			_ => Err(anyhow!("Unknown delivery type")),
		}
	}
}
