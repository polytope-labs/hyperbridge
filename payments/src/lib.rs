//! This module allows the relayer to maintain a local database
//! of requests and responses the relayer has delivered successfully
#![allow(unused_imports)]
#![allow(unused)]
use crate::db::{
	deliveries::{Data, WhereParam},
	new_client_with_url,
	read_filters::{IntFilter, StringFilter},
	PrismaClient, PrismaClientBuilder,
};
use anyhow::anyhow;
use codec::Encode;
use ismp::{
	consensus::StateMachineHeight,
	host::StateMachine,
	messaging::{Message, Proof},
	router::{Post, Request, RequestResponse},
	util::{hash_request, hash_response, Keccak256},
};
use pallet_relayer_fees::withdrawal::{Key, WithdrawalProof};
use primitive_types::H256;
use prisma_client_rust::BatchItem;
use serde::{Deserialize, Serialize};
use sp_core::keccak_256;
use std::sync::Arc;
use tesseract_evm::EvmConfig;
use tesseract_primitives::{IsmpProvider, TxReceipt};
use tesseract_substrate::SubstrateConfig;

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

	pub async fn store_latest_height(
		&self,
		source: StateMachine,
		dest_chain: StateMachine,
		height: u64,
	) -> anyhow::Result<()> {
		self.db
			.latest_heights()
			.delete_many(vec![
				db::latest_heights::WhereParam::SourceChain(StringFilter::Equals(
					source.to_string(),
				)),
				db::latest_heights::WhereParam::DestChain(StringFilter::Equals(
					dest_chain.to_string(),
				)),
			])
			.exec()
			.await?;
		self.db
			.latest_heights()
			.create(source.to_string(), dest_chain.to_string(), height as i32, Default::default())
			.exec()
			.await?;
		Ok(())
	}

	pub async fn retreive_latest_height(
		&self,
		source: StateMachine,
		dest_chain: StateMachine,
	) -> anyhow::Result<Option<u64>> {
		let res = self
			.db
			.latest_heights()
			.find_first(vec![
				db::latest_heights::WhereParam::SourceChain(StringFilter::Equals(
					source.to_string(),
				)),
				db::latest_heights::WhereParam::DestChain(StringFilter::Equals(
					dest_chain.to_string(),
				)),
			])
			.exec()
			.await?
			.map(|data| data.latest_height as u64);

		Ok(res)
	}

	/// Store entries for delivered post requests and responses
	pub async fn store_messages(&self, receipts: Vec<TxReceipt>) -> anyhow::Result<()> {
		let mut actions = vec![];
		for receipt in receipts {
			match receipt {
				TxReceipt::Request(query) => {
					let action = self.db.deliveries().create(
						hex::encode(query.commitment.as_bytes()),
						query.source_chain.to_string(),
						query.dest_chain.to_string(),
						DeliveryType::PostRequest as i32,
						chrono::Utc::now().timestamp() as i32,
						Default::default(),
					);

					actions.push(action);
				},

				TxReceipt::Response { query, request_commitment } => {
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
		self.db
			.deliveries()
			.delete_many(
				reqs.into_iter()
					.map(|hash| {
						WhereParam::Hash(StringFilter::Equals(hex::encode(hash.as_slice())))
					})
					.collect(),
			)
			.exec()
			.await?;
		Ok(())
	}

	/// Create payment claim proof for all deliveries of requests and responses from source to dest
	/// a number of days The default is 30 days
	pub async fn create_claim_proof<A: IsmpProvider, B: IsmpProvider>(
		&self,
		source_height: u64,
		dest_height: u64,
		source: &A,
		dest: &B,
	) -> anyhow::Result<WithdrawalProof> {
		let source_chain = source.state_machine_id().state_id;
		let dest_chain = dest.state_machine_id().state_id;
		let entries = self
			.db
			.deliveries()
			.find_many(vec![
				WhereParam::SourceChain(StringFilter::Equals(source_chain.to_string())),
				WhereParam::DestChain(StringFilter::Equals(dest_chain.to_string())),
				WhereParam::DeliveryType(IntFilter::Equals(DeliveryType::PostRequest as i32)),
			])
			.exec()
			.await?;

		let requests = entries.iter().filter_map(|data| {
			// Get request commitment keys on source chain
			let hash = H256::from_slice(&hex::decode(data.hash.clone()).ok()?);
			let source_key = source.request_commitment_full_key(hash);
			//Get request receipt keys on dest chain
			let dest_key = dest.request_receipt_full_key(hash);
			Some((Key::Request(hash), source_key, dest_key))
		});

		let entries = self
			.db
			.deliveries()
			.find_many(vec![
				WhereParam::SourceChain(StringFilter::Equals(source_chain.to_string())),
				WhereParam::DestChain(StringFilter::Equals(dest_chain.to_string())),
				WhereParam::DeliveryType(IntFilter::Equals(DeliveryType::PostResponse as i32)),
			])
			.exec()
			.await?;

		let responses = entries.iter().filter_map(|data| {
			// Get response commitment keys on source chain
			let concat_hash = hex::decode(data.hash.clone()).ok()?;
			let response_commitment = H256::from_slice(&concat_hash[..32]);
			let source_key = source.response_commitment_full_key(response_commitment);
			//Get response receipt keys on dest chain
			let request_commitment = H256::from_slice(&concat_hash[32..]);
			let dest_key = dest.response_receipt_full_key(request_commitment);
			Some((Key::Response { request_commitment, response_commitment }, source_key, dest_key))
		});
		// Gather keys to be queried on the source chain
		let mut source_chain_storage_keys = vec![];
		let mut dest_chain_storage_keys = vec![];
		let mut request_response_commitments = vec![];

		for (key, source_key, dest_key) in requests.chain(responses) {
			source_chain_storage_keys.push(source_key);
			dest_chain_storage_keys.push(dest_key);
			request_response_commitments.push(key);
		}

		let source_proof = source
			.query_state_proof(
				source_height,
				source_chain_storage_keys.into_iter().flatten().collect(),
			)
			.await?;
		let dest_proof = dest
			.query_state_proof(dest_height, dest_chain_storage_keys.into_iter().flatten().collect())
			.await?;

		Ok(WithdrawalProof {
			commitments: request_response_commitments,
			source_proof: Proof {
				height: StateMachineHeight { id: source.state_machine_id(), height: source_height },
				proof: source_proof,
			},
			dest_proof: Proof {
				height: StateMachineHeight { id: dest.state_machine_id(), height: dest_height },
				proof: dest_proof,
			},
		})
	}

	pub async fn delete_claimed_entries(&self, proof: WithdrawalProof) -> anyhow::Result<()> {
		// Remove claimed entries from db
		let entries = proof
			.commitments
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

pub struct Hasher;

impl Keccak256 for Hasher {
	fn keccak256(bytes: &[u8]) -> H256 {
		keccak_256(bytes).into()
	}
}
