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

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use anyhow::Error;
use codec::Encode;
use ismp::{
	consensus::{ConsensusStateId, StateCommitment, StateMachineHeight, StateMachineId},
	events::{Event, StateCommitmentVetoed},
	host::StateMachine,
	messaging::{CreateConsensusState, Message},
};
use pallet_ismp_host_executive::HostParam;
use pharos_primitives::{NonExistenceProof, PharosProofNode};
use pharos_prover::{
	rpc::{PharosRpcClient, RpcAccountProof, hex_to_bytes},
	rpc_to_proof_nodes, rpc_to_sibling_proofs,
};
use pharos_state_machine::AccountProofData;
use primitive_types::{H160, H256, U256};
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::{
	BoxStream, ByzantineHandler, EstimateGasReturnParams, IsmpProvider, Query, Signature,
	StateMachineUpdated, StateProofQueryType, TxResult,
};

use pharos_state_machine::PharosStateProof;

#[derive(Clone)]
pub struct PharosEvmClient {
	pub evm: EvmClient,
	pub rpc: Arc<PharosRpcClient>,
}

impl PharosEvmClient {
	pub async fn new(config: EvmConfig) -> Result<Self, Error> {
		let rpc_url = config
			.rpc_urls
			.first()
			.ok_or_else(|| anyhow::anyhow!("No RPC URL configured"))?;
		let rpc = Arc::new(
			PharosRpcClient::new(rpc_url).map_err(|e| anyhow::anyhow!("RPC init failed: {e:?}"))?,
		);
		let evm = EvmClient::new(config).await?;
		Ok(Self { evm, rpc })
	}

	/// Fetch a Pharos state proof for the given storage keys at the given block.
	/// Handles both existence and non-existence proofs from the RPC response.
	async fn fetch_pharos_proof(
		&self,
		at: u64,
		address: H160,
		slot_hashes: Vec<H256>,
	) -> Result<Vec<u8>, Error> {
		let rpc_proof = self
			.rpc
			.get_proof(address, slot_hashes, at)
			.await
			.map_err(|e| anyhow::anyhow!("eth_getProof failed: {e:?}"))?;

		let mut storage_proof = BTreeMap::new();
		let mut storage_values = BTreeMap::new();
		let mut non_existence_proofs = BTreeMap::new();

		for sp in &rpc_proof.storage_proof {
			let key_bytes =
				hex_to_bytes(&sp.key).map_err(|e| anyhow::anyhow!("hex decode key: {e:?}"))?;
			let mut slot_key = [0u8; 32];
			if key_bytes.len() <= 32 {
				slot_key[32 - key_bytes.len()..].copy_from_slice(&key_bytes);
			}
			let slot_vec = slot_key.to_vec();

			if sp.is_exist {
				let proof_nodes = rpc_to_proof_nodes(&sp.proof)
					.map_err(|e| anyhow::anyhow!("proof node conversion: {e:?}"))?;
				let value_bytes = hex_to_bytes(&sp.value)
					.map_err(|e| anyhow::anyhow!("hex decode value: {e:?}"))?;
				let mut padded = [0u8; 32];
				if value_bytes.len() <= 32 {
					padded[32 - value_bytes.len()..].copy_from_slice(&value_bytes);
				}
				storage_proof.insert(slot_vec.clone(), proof_nodes);
				storage_values.insert(slot_vec, padded.to_vec());
			} else {
				let proof_nodes = rpc_to_proof_nodes(&sp.proof)
					.map_err(|e| anyhow::anyhow!("proof node conversion: {e:?}"))?;
				let sibling_proofs = rpc_to_sibling_proofs(&sp.sibling_leftmost_leaf_proofs)
					.map_err(|e| anyhow::anyhow!("sibling proof conversion: {e:?}"))?;
				non_existence_proofs
					.insert(slot_vec, NonExistenceProof { proof_nodes, sibling_proofs });
			}
		}

		let pharos_proof = PharosStateProof {
			storage_proof,
			storage_values,
			non_existence_proofs,
			account_proofs: BTreeMap::new(),
		};
		Ok(pharos_proof.encode())
	}

	async fn fetch_account_proof(&self, at: u64, address: H160) -> Result<AccountProofData, Error> {
		let rpc_proof = self
			.rpc
			.get_proof(address, vec![], at)
			.await
			.map_err(|e| anyhow::anyhow!("eth_getProof failed: {e:?}"))?;

		let proof_nodes = rpc_to_proof_nodes(&rpc_proof.account_proof)
			.map_err(|e| anyhow::anyhow!("account proof conversion: {e:?}"))?;
		let raw_value =
			hex_to_bytes(&rpc_proof.raw_value).map_err(|e| anyhow::anyhow!("hex decode: {e:?}"))?;

		Ok(AccountProofData { proof_nodes, raw_value })
	}
}

#[async_trait::async_trait]
impl IsmpProvider for PharosEvmClient {
	async fn query_consensus_state(
		&self,
		at: Option<u64>,
		id: ConsensusStateId,
	) -> Result<Vec<u8>, Error> {
		self.evm.query_consensus_state(at, id).await
	}

	async fn query_latest_height(&self, id: StateMachineId) -> Result<u32, Error> {
		self.evm.query_latest_height(id).await
	}

	async fn query_finalized_height(&self) -> Result<u64, Error> {
		self.evm.query_finalized_height().await
	}

	async fn query_state_machine_commitment(
		&self,
		height: StateMachineHeight,
	) -> Result<StateCommitment, Error> {
		self.evm.query_state_machine_commitment(height).await
	}

	async fn query_state_machine_update_time(
		&self,
		height: StateMachineHeight,
	) -> Result<Duration, Error> {
		self.evm.query_state_machine_update_time(height).await
	}

	async fn query_challenge_period(&self, id: StateMachineId) -> Result<Duration, Error> {
		self.evm.query_challenge_period(id).await
	}

	async fn query_timestamp(&self) -> Result<Duration, Error> {
		self.evm.query_timestamp().await
	}

	async fn query_requests_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		_counterparty: StateMachine,
	) -> Result<Vec<u8>, Error> {
		let slot_hashes: Vec<H256> = keys
			.into_iter()
			.map(|q| self.evm.request_commitment_key(q.commitment).1)
			.collect();
		self.fetch_pharos_proof(at, self.evm.config.ismp_host, slot_hashes).await
	}

	async fn query_responses_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		_counterparty: StateMachine,
	) -> Result<Vec<u8>, Error> {
		let slot_hashes: Vec<H256> = keys
			.into_iter()
			.map(|q| self.evm.response_commitment_key(q.commitment).1)
			.collect();
		self.fetch_pharos_proof(at, self.evm.config.ismp_host, slot_hashes).await
	}

	async fn query_state_proof(
		&self,
		at: u64,
		keys: StateProofQueryType,
	) -> Result<Vec<u8>, Error> {
		match keys {
			StateProofQueryType::Ismp(keys) => {
				let slot_hashes: Vec<H256> =
					keys.into_iter().map(|k| H256::from_slice(&k)).collect();
				self.fetch_pharos_proof(at, self.evm.config.ismp_host, slot_hashes).await
			},
			StateProofQueryType::Arbitrary(keys) => {
				// For arbitrary keys, group by contract address and fetch per-contract proofs
				// Then merge into a single PharosStateProof
				let mut storage_proof = BTreeMap::new();
				let mut storage_values = BTreeMap::new();
				let mut non_existence_proofs = BTreeMap::new();
				let mut account_proofs = BTreeMap::new();

				let mut groups: BTreeMap<H160, Vec<H256>> = BTreeMap::new();
				let mut account_queries: Vec<H160> = Vec::new();
				for key in &keys {
					if key.len() == 52 {
						let address = H160::from_slice(&key[..20]);
						let slot = H256::from_slice(&key[20..]);
						groups.entry(address).or_default().push(slot);
					} else if key.len() == 20 {
						account_queries.push(H160::from_slice(key));
					}
				}

				for address in account_queries {
					let data = self.fetch_account_proof(at, address).await?;
					account_proofs.insert(address.0.to_vec(), data);
				}

				for (address, slots) in groups {
					let rpc_proof = self
						.rpc
						.get_proof(address, slots, at)
						.await
						.map_err(|e| anyhow::anyhow!("eth_getProof failed: {e:?}"))?;

					for sp in &rpc_proof.storage_proof {
						let key_bytes = hex_to_bytes(&sp.key)
							.map_err(|e| anyhow::anyhow!("hex decode: {e:?}"))?;
						let mut slot_key = [0u8; 32];
						if key_bytes.len() <= 32 {
							slot_key[32 - key_bytes.len()..].copy_from_slice(&key_bytes);
						}
						let slot_vec = slot_key.to_vec();

						if sp.is_exist {
							let nodes = rpc_to_proof_nodes(&sp.proof)
								.map_err(|e| anyhow::anyhow!("{e:?}"))?;
							let val =
								hex_to_bytes(&sp.value).map_err(|e| anyhow::anyhow!("{e:?}"))?;
							let mut padded = [0u8; 32];
							if val.len() <= 32 {
								padded[32 - val.len()..].copy_from_slice(&val);
							}
							storage_proof.insert(slot_vec.clone(), nodes);
							storage_values.insert(slot_vec, padded.to_vec());
						} else {
							let nodes = rpc_to_proof_nodes(&sp.proof)
								.map_err(|e| anyhow::anyhow!("{e:?}"))?;
							let siblings = rpc_to_sibling_proofs(&sp.sibling_leftmost_leaf_proofs)
								.map_err(|e| anyhow::anyhow!("{e:?}"))?;
							non_existence_proofs.insert(
								slot_vec,
								NonExistenceProof { proof_nodes: nodes, sibling_proofs: siblings },
							);
						}
					}
				}

				let pharos_proof = PharosStateProof {
					storage_proof,
					storage_values,
					non_existence_proofs,
					account_proofs,
				};
				Ok(pharos_proof.encode())
			},
		}
	}

	async fn query_ismp_events(
		&self,
		previous_height: u64,
		event: StateMachineUpdated,
	) -> Result<Vec<Event>, Error> {
		self.evm.query_ismp_events(previous_height, event).await
	}

	fn name(&self) -> String {
		self.evm.name()
	}

	fn state_machine_id(&self) -> StateMachineId {
		self.evm.state_machine_id()
	}

	fn block_max_gas(&self) -> u64 {
		self.evm.block_max_gas()
	}

	fn initial_height(&self) -> u64 {
		self.evm.initial_height()
	}

	async fn estimate_gas(&self, msg: Vec<Message>) -> Result<Vec<EstimateGasReturnParams>, Error> {
		self.evm.estimate_gas(msg).await
	}

	async fn query_request_fee_metadata(&self, hash: H256) -> Result<U256, Error> {
		self.evm.query_request_fee_metadata(hash).await
	}

	async fn query_request_receipt(&self, hash: H256) -> Result<Vec<u8>, Error> {
		self.evm.query_request_receipt(hash).await
	}

	async fn query_response_receipt(&self, hash: H256) -> Result<Vec<u8>, Error> {
		self.evm.query_response_receipt(hash).await
	}

	async fn query_response_fee_metadata(&self, hash: H256) -> Result<U256, Error> {
		self.evm.query_response_fee_metadata(hash).await
	}

	async fn state_machine_update_notification(
		&self,
		counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<StateMachineUpdated>, Error> {
		self.evm.state_machine_update_notification(counterparty_state_id).await
	}

	async fn state_commitment_vetoed_notification(
		&self,
		from: u64,
		height: StateMachineHeight,
	) -> BoxStream<StateCommitmentVetoed> {
		self.evm.state_commitment_vetoed_notification(from, height).await
	}

	async fn submit(
		&self,
		messages: Vec<Message>,
		coprocessor: StateMachine,
	) -> Result<TxResult, Error> {
		self.evm.submit(messages, coprocessor).await
	}

	fn request_commitment_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		self.evm.request_commitment_full_key(commitment)
	}

	fn request_receipt_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		self.evm.request_receipt_full_key(commitment)
	}

	fn response_commitment_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		self.evm.response_commitment_full_key(commitment)
	}

	fn response_receipt_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		self.evm.response_receipt_full_key(commitment)
	}

	fn address(&self) -> Vec<u8> {
		self.evm.address()
	}

	fn sign(&self, msg: &[u8]) -> Signature {
		self.evm.sign(msg)
	}

	async fn set_latest_finalized_height(
		&mut self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), Error> {
		self.evm.set_latest_finalized_height(counterparty).await
	}

	async fn set_initial_consensus_state(
		&self,
		message: CreateConsensusState,
	) -> Result<(), Error> {
		self.evm.set_initial_consensus_state(message).await
	}

	async fn veto_state_commitment(&self, height: StateMachineHeight) -> Result<(), Error> {
		self.evm.veto_state_commitment(height).await
	}

	async fn query_host_params(
		&self,
		state_machine: StateMachine,
	) -> Result<HostParam<u128>, Error> {
		self.evm.query_host_params(state_machine).await
	}

	fn max_concurrent_queries(&self) -> usize {
		self.evm.max_concurrent_queries()
	}

	async fn fee_token_decimals(&self) -> Result<u8, Error> {
		self.evm.fee_token_decimals().await
	}
}

#[async_trait::async_trait]
impl ByzantineHandler for PharosEvmClient {
	async fn check_for_byzantine_attack(
		&self,
		coprocessor: StateMachine,
		counterparty: Arc<dyn IsmpProvider>,
		event: StateMachineUpdated,
	) -> Result<(), Error> {
		self.evm.check_for_byzantine_attack(coprocessor, counterparty, event).await
	}

	async fn state_machine_updates(
		&self,
		counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<Vec<StateMachineUpdated>>, Error> {
		self.evm.state_machine_updates(counterparty_state_id).await
	}
}
