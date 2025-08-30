//! This module allows the relayer to maintain a local database
//! of requests and responses the relayer has delivered successfully
#![allow(unused_imports)]
#![allow(unused)]
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
	router::{PostRequest, Request, RequestResponse},
};
use itertools::Itertools;
use pallet_ismp_relayer::withdrawal::{Key, Signature, WithdrawalProof};
use primitive_types::{H256, U256};
use prisma_client_rust::{query_core::RawQuery, BatchItem, Direction, PrismaValue, Raw};
use serde::{Deserialize, Serialize};
use sp_core::keccak_256;
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

	/// Store entries for delivered post requests and responses
	pub async fn store_messages(&self, receipts: Vec<TxReceipt>) -> anyhow::Result<()> {
		let mut actions = vec![];
		for receipt in receipts {
			match receipt {
				TxReceipt::Request { query, height } => {
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
				},

				TxReceipt::Response { query, request_commitment, height } => {
					// When inserting the hash for responses we concatenate the response
					// commitment with the request commitment
					let mut commitment = vec![];
					commitment.extend_from_slice(query.commitment.as_bytes());
					commitment.extend_from_slice(request_commitment.as_bytes());
					let action = self.db.deliveries().create(
						hex::encode(commitment.as_slice()),
						query.source_chain.to_string(),
						query.dest_chain.to_string(),
						DeliveryType::PostResponse as i32,
						chrono::Utc::now().timestamp() as i32,
						height as i32,
						Default::default(),
					);
					actions.push(action);
				},
			}
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

		let response_entries = self
			.db
			.deliveries()
			.find_many(vec![
				WhereParam::SourceChain(StringFilter::Equals(source_chain.to_string())),
				WhereParam::DestChain(StringFilter::Equals(dest_chain.to_string())),
				WhereParam::DeliveryType(IntFilter::Equals(DeliveryType::PostResponse as i32)),
			])
			.order_by(OrderByParam::Height(Direction::Asc))
			.exec()
			.await?;

		let highest_request_delivery_height =
			request_entries.get(request_entries.len() - 1).map(|data| data.height as u64);

		let highest_response_delivery_height =
			response_entries.get(response_entries.len() - 1).map(|data| data.height as u64);

		let dest_height = std::cmp::max(
			highest_request_delivery_height.unwrap_or_default(),
			highest_response_delivery_height.unwrap_or_default(),
		);

		if dest_height == 0 {
			Ok(None)
		} else {
			Ok(Some(dest_height))
		}
	}

	pub async fn query_state_proofs(
		&self,
		source: Arc<dyn IsmpProvider>,
		dest: Arc<dyn IsmpProvider>,
		source_height: u64,
		dest_height: u64,
		keys: Vec<(Key, Vec<Vec<u8>>, Vec<Vec<u8>>)>,
	) -> Result<Vec<WithdrawalProof>, anyhow::Error> {
		let mut proofs = vec![];
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
				beneficiary_details: {
					// If they are not the same chain type
					if discriminant(&source.state_machine_id().state_id) !=
						discriminant(&dest.state_machine_id().state_id)
					{
						Some((source.address(), dest.sign(&keccak_256(&source.address()))))
					} else {
						None
					}
				},
			};

			proofs.push(proof)
		}

		Ok(proofs)
	}

	// todo: Consolidate the state proof query into a single function
	/// Create payment claim proof for all deliveries of requests and responses from source to dest
	/// a number of days The default is 30 days
	pub async fn create_proof_from_receipts(
		&self,
		source_height: u64,
		dest_height: u64,
		source: Arc<dyn IsmpProvider>,
		dest: Arc<dyn IsmpProvider>,
		receipts: Vec<TxReceipt>,
	) -> anyhow::Result<Vec<WithdrawalProof>> {
		let keys = receipts
			.iter()
			.map(|data| {
				match data {
					TxReceipt::Request { query, height } => {
						let source_key = source.request_commitment_full_key(query.commitment);
						//Get request receipt keys on dest chain
						let dest_key = dest.request_receipt_full_key(query.commitment);
						(Key::Request(query.commitment), source_key, dest_key)
					},
					TxReceipt::Response { query, request_commitment, height } => {
						let source_key = source.response_commitment_full_key(query.commitment);
						//Get response receipt keys on dest chain
						let dest_key = dest.response_receipt_full_key(*request_commitment);
						(
							Key::Response {
								request_commitment: *request_commitment,
								response_commitment: query.commitment,
							},
							source_key,
							dest_key,
						)
					},
				}
			})
			.collect::<Vec<_>>();

		self.query_state_proofs(source, dest, source_height, dest_height, keys).await
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
	pub async fn create_claim_proof<H: HyperbridgeClaim>(
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

		let response_entries = self
			.db
			.deliveries()
			.find_many(vec![
				WhereParam::SourceChain(StringFilter::Equals(source_chain.to_string())),
				WhereParam::DestChain(StringFilter::Equals(dest_chain.to_string())),
				WhereParam::DeliveryType(IntFilter::Equals(DeliveryType::PostResponse as i32)),
			])
			.exec()
			.await?;

		if request_entries.is_empty() && response_entries.is_empty() {
			return Ok(Default::default());
		}

		let requests = request_entries.iter().filter_map(|data| {
			// Get request commitment keys on source chain
			let hash = H256::from_slice(&hex::decode(data.hash.clone()).ok()?);
			let source_key = source.request_commitment_full_key(hash);
			//Get request receipt keys on dest chain
			let dest_key = dest.request_receipt_full_key(hash);
			Some((Key::Request(hash), source_key, dest_key))
		});

		let responses = response_entries.iter().filter_map(|data| {
			// Get response commitment keys on source chain
			let concat_hash = hex::decode(data.hash.clone()).ok()?;
			let response_commitment = H256::from_slice(&concat_hash[..32]);
			let source_key = source.response_commitment_full_key(response_commitment);
			//Get response receipt keys on dest chain
			let request_commitment = H256::from_slice(&concat_hash[32..]);
			let dest_key = dest.response_receipt_full_key(request_commitment);
			Some((Key::Response { request_commitment, response_commitment }, source_key, dest_key))
		});

		let mut keys_to_delete = vec![];
		let mut keys_to_prove = vec![];

		for key in requests.chain(responses) {
			let fee = match &key.0 {
				Key::Request(hash) => source.query_request_fee_metadata(*hash).await?,
				Key::Response { response_commitment, .. } =>
					source.query_response_fee_metadata(*response_commitment).await?,
			};

			if fee.is_zero() {
				keys_to_delete.push(key.0);
				continue;
			}

			if hyperbridge.check_claimed(key.0.clone()).await? {
				keys_to_delete.push(key.0);
				continue;
			}
			keys_to_prove.push(key);
		}

		let tx = self.clone();
		// Delete claimed keys in the background
		tokio::spawn(async move {
			match tx.delete_claimed_entries(keys_to_delete).await {
				Err(_) => {
					tracing::error!("An Error occurred while deleting claimed fees from the db, the claimed keys will be deleted in the next fee accumulation attempt");
				},
				_ => {},
			}
		});

		self.query_state_proofs(source, dest, source_height, dest_height, keys_to_prove)
			.await
	}

	pub async fn delete_claimed_entries(&self, commitments: Vec<Key>) -> anyhow::Result<()> {
		if !commitments.is_empty() {
			// Remove claimed entries from db
			let entries = commitments
				.into_iter()
				.map(|key| match key {
					Key::Request(req) => req.0.to_vec(),
					Key::Response { request_commitment, response_commitment } => {
						let mut key = vec![];
						key.extend_from_slice(&response_commitment.0);
						key.extend_from_slice(&request_commitment.0);
						key
					},
				})
				.collect();

			self.delete_entries(entries).await?;
		}
		Ok(())
	}
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub enum DeliveryType {
	PostRequest = 0,
	PostResponse = 1,
}

impl TryFrom<i32> for DeliveryType {
	type Error = anyhow::Error;
	fn try_from(value: i32) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(Self::PostRequest),
			1 => Ok(Self::PostResponse),
			_ => Err(anyhow!("Unknown delivery type")),
		}
	}
}
