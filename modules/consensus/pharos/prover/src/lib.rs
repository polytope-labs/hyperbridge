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

//! Pharos consensus prover for light client.

pub mod error;
pub mod rpc;

pub use error::ProverError;

use pharos_primitives::{
	BlockProof, BlsPublicKey, Config, PharosProofNode, ValidatorInfo, ValidatorSet,
	ValidatorSetProof, VerifierStateUpdate, STAKING_CONTRACT_ADDRESS,
};
use pharos_verifier::state_proof::StakingContractLayout;
use primitive_types::{H160, H256, U256};
use rpc::{
	hex_to_bytes, hex_to_u64, PharosRpcClient, RpcBlockProof, RpcProofNode, RpcValidatorInfo,
};
use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

/// Pharos prover for constructing light client updates.
#[derive(Clone)]
pub struct PharosProver<C: Config> {
	pub rpc: Arc<PharosRpcClient>,
	storage_layout: StakingContractLayout,
	_config: PhantomData<C>,
}

impl<C: Config> PharosProver<C> {
	/// Create a new prover with the given RPC endpoint.
	pub fn new(endpoint: impl Into<String>) -> Result<Self, ProverError> {
		Ok(Self {
			rpc: Arc::new(PharosRpcClient::new(endpoint)?),
			storage_layout: StakingContractLayout::default(),
			_config: PhantomData,
		})
	}

	/// Create a new prover with a custom storage layout.
	pub fn with_storage_layout(
		endpoint: impl Into<String>,
		layout: StakingContractLayout,
	) -> Result<Self, ProverError> {
		Ok(Self {
			rpc: Arc::new(PharosRpcClient::new(endpoint)?),
			storage_layout: layout,
			_config: PhantomData,
		})
	}

	/// Fetch the latest block number from the node.
	pub async fn get_latest_block(&self) -> Result<u64, ProverError> {
		self.rpc.get_block_number().await
	}

	/// Fetch a block update for the given block number.
	///
	/// This will:
	/// 1. Fetch the block header
	/// 2. Fetch the block proof
	/// 3. If at epoch boundary, fetch validator set proof
	pub async fn fetch_block_update(
		&self,
		block_number: u64,
	) -> Result<VerifierStateUpdate, ProverError> {
		let header = self.rpc.get_block_by_number(block_number).await?;

		let rpc_proof = self.rpc.get_block_proof(block_number).await?;
		let block_proof = self.convert_rpc_block_proof(&rpc_proof)?;

		let validator_set_proof = if C::is_epoch_boundary(block_number) {
			Some(self.fetch_validator_set_proof(block_number).await?)
		} else {
			None
		};

		Ok(VerifierStateUpdate { header, block_proof, validator_set_proof })
	}

	/// Fetch only the block proof for a given block number.
	pub async fn fetch_block_proof(&self, block_number: u64) -> Result<BlockProof, ProverError> {
		let rpc_proof = self.rpc.get_block_proof(block_number).await?;
		self.convert_rpc_block_proof(&rpc_proof)
	}

	/// Build a ValidatorSet from RPC validator info.
	pub fn build_validator_set(
		&self,
		validators: &[RpcValidatorInfo],
		epoch: u64,
	) -> Result<ValidatorSet, ProverError> {
		let mut validator_set = ValidatorSet::new(epoch);

		for v in validators {
			let bls_key_bytes = hex_to_bytes(&v.bls_key)?;
			let len = bls_key_bytes.len();
			let bls_public_key: BlsPublicKey =
				bls_key_bytes.try_into().map_err(|_| ProverError::InvalidBlsKeyLength(len))?;

			let pool_id_bytes = hex_to_bytes(&v.validator_id)?;
			let pool_id = if pool_id_bytes.len() == 32 {
				H256::from_slice(&pool_id_bytes)
			} else {
				let mut padded = [0u8; 32];
				let start = 32usize.saturating_sub(pool_id_bytes.len());
				padded[start..].copy_from_slice(&pool_id_bytes);
				H256::from(padded)
			};

			let stake = Self::parse_stake(&v.staking)?;

			let info = ValidatorInfo { bls_public_key, pool_id, stake };
			validator_set.add_validator(info);
		}

		Ok(validator_set)
	}

	/// Parse a hex stake value to U256.
	fn parse_stake(hex: &str) -> Result<U256, ProverError> {
		let hex = hex.trim_start_matches("0x");
		U256::from_str_radix(hex, 16).map_err(|_| ProverError::InvalidNumber)
	}

	/// Fetch validator set proof for an epoch boundary block.
	///
	/// This fetches the storage proof from the staking contract at the
	/// given block, which contains the validator set for the next epoch.
	///
	/// The storage layout follows the Pharos staking contract (V1):
	/// - Slot 6: totalStake
	/// - Slot 1: activePoolIds (bytes32[] array length)
	/// - keccak256(1): array elements (pool IDs)
	/// - For each pool ID: validator data via mapping at slot 0
	pub async fn fetch_validator_set_proof(
		&self,
		block_number: u64,
	) -> Result<ValidatorSetProof, ProverError> {
		let address = H160::from_slice(STAKING_CONTRACT_ADDRESS.as_slice());

		// Fetch base slots (totalStake, activePoolIds length)
		let base_keys = vec![
			self.storage_layout.raw_slot_key(self.storage_layout.total_stake_slot),
			self.storage_layout.raw_slot_key(self.storage_layout.active_pool_set_slot),
		];

		let base_proof = self.rpc.get_proof(address, base_keys.clone(), block_number).await?;

		// Get validator count from activePoolIds length (slot 1)
		let validator_count = if base_proof.storage_proof.len() >= 2 {
			hex_to_u64(&base_proof.storage_proof[1].value)?
		} else {
			return Err(ProverError::MissingStorageProof("activePoolIds length"));
		};

		// Fetch pool IDs from the activePoolIds array
		let mut pool_id_keys = Vec::new();
		for i in 0..validator_count {
			pool_id_keys.push(self.array_element_key(self.storage_layout.active_pool_set_slot, i));
		}

		if pool_id_keys.is_empty() {
			return Err(ProverError::MissingStorageProof("activePoolIds array is empty"));
		}

		let pool_id_proof = self.rpc.get_proof(address, pool_id_keys.clone(), block_number).await?;

		// Extract pool IDs
		let mut pool_ids = Vec::new();
		for sp in &pool_id_proof.storage_proof {
			let bytes = hex_to_bytes(&sp.value)?;
			let mut padded = [0u8; 32];
			if bytes.len() <= 32 {
				padded[32 - bytes.len()..].copy_from_slice(&bytes);
			}
			pool_ids.push(H256::from(padded));
		}

		// Collect storage values and per-key proof paths.
		// Each storage key maps to its own proof path for individual verification.
		let mut storage_proof: BTreeMap<H256, Vec<PharosProofNode>> = BTreeMap::new();
		let mut storage_values: Vec<Vec<u8>> = Vec::new();

		for (i, sp) in base_proof.storage_proof.iter().enumerate() {
			storage_proof.insert(base_keys[i], rpc_to_proof_nodes(&sp.proof)?);
			storage_values.push(hex_to_bytes(&sp.value)?);
		}
		for (i, sp) in pool_id_proof.storage_proof.iter().enumerate() {
			storage_proof.insert(pool_id_keys[i], rpc_to_proof_nodes(&sp.proof)?);
			storage_values.push(hex_to_bytes(&sp.value)?);
		}

		// Fetch validator data concurrently with two-phase fetching per validator.
		// Phase 1: fetch BLS string header + stake to determine BLS data slot count.
		// Phase 2: fetch the dynamically-computed BLS data slots.
		// All validators run concurrently, with sequential phases per validator.
		let validator_futures: Vec<_> = pool_ids
			.iter()
			.map(|pool_id| {
				let (bls_string_slot, stake_slot) =
					self.get_validator_header_and_stake_keys(pool_id);
				let rpc = self.rpc.clone();
				async move {
					// fetch BLS header + stake
					let phase1_keys = vec![bls_string_slot, stake_slot];
					let phase1_proof = rpc.get_proof(address, phase1_keys, block_number).await?;

					if phase1_proof.storage_proof.len() < 2 {
						return Err(ProverError::MissingStorageProof(
							"BLS header or stake in phase 1",
						));
					}

					// Determine BLS data slot count from the string header value
					let header_hex = &phase1_proof.storage_proof[0].value;
					let data_slot_count = bls_data_slots_from_hex(header_hex)?;

					// fetch BLS data slots
					let bls_data_base = H256::from(keccak256(bls_string_slot.as_bytes()));
					let bls_data_base_pos = U256::from_big_endian(bls_data_base.as_bytes());
					let mut data_keys = Vec::new();
					for i in 0..data_slot_count {
						data_keys.push(H256((bls_data_base_pos + U256::from(i)).to_big_endian()));
					}

					let phase2_proof = if !data_keys.is_empty() {
						Some(rpc.get_proof(address, data_keys.clone(), block_number).await?)
					} else {
						None
					};

					// Assemble in order: [header, data_0..N, stake]
					let mut all_keys = Vec::new();
					let mut all_proofs = Vec::new();
					let mut all_values = Vec::new();

					// Header
					all_keys.push(bls_string_slot);
					all_proofs.push(rpc_to_proof_nodes(&phase1_proof.storage_proof[0].proof)?);
					all_values.push(hex_to_bytes(&phase1_proof.storage_proof[0].value)?);

					// Data slots
					if let Some(p2) = phase2_proof {
						for (j, sp) in p2.storage_proof.iter().enumerate() {
							all_keys.push(data_keys[j]);
							all_proofs.push(rpc_to_proof_nodes(&sp.proof)?);
							all_values.push(hex_to_bytes(&sp.value)?);
						}
					}

					// Stake
					all_keys.push(stake_slot);
					all_proofs.push(rpc_to_proof_nodes(&phase1_proof.storage_proof[1].proof)?);
					all_values.push(hex_to_bytes(&phase1_proof.storage_proof[1].value)?);

					Ok::<_, ProverError>((all_keys, all_proofs, all_values))
				}
			})
			.collect();

		let validator_results = futures::future::join_all(validator_futures).await;

		for result in validator_results {
			let (keys, proofs, values) = result?;
			for (j, key) in keys.iter().enumerate() {
				storage_proof.insert(*key, proofs[j].clone());
				storage_values.push(values[j].clone());
			}
		}

		Ok(ValidatorSetProof { storage_proof, storage_values })
	}

	/// Calculate the storage key for a dynamic array element.
	fn array_element_key(&self, base_slot: u64, index: u64) -> H256 {
		let slot_bytes = U256::from(base_slot).to_big_endian();
		let base_key = keccak256(&slot_bytes);
		let base_pos = U256::from_big_endian(&base_key);
		let element_pos = base_pos + U256::from(index);
		H256(element_pos.to_big_endian())
	}

	/// Get the BLS string header slot and stake slot for a validator.
	///
	/// These are fetched first (phase 1) to determine the dynamic BLS data slot count
	/// from the string header value before fetching the data slots (phase 2).
	fn get_validator_header_and_stake_keys(&self, pool_id: &H256) -> (H256, H256) {
		const BLS_PUBLIC_KEY_OFFSET: u64 = 3;
		const TOTAL_STAKE_OFFSET: u64 = 8;

		// Calculate validator base slot: keccak256(pool_id || mapping_slot)
		let mut data = [0u8; 64];
		data[..32].copy_from_slice(pool_id.as_bytes());
		data[32..64].copy_from_slice(
			&U256::from(self.storage_layout.validators_mapping_slot).to_big_endian(),
		);
		let base_slot = H256::from(keccak256(&data));
		let base_pos = U256::from_big_endian(base_slot.as_bytes());

		let bls_string_slot = H256((base_pos + U256::from(BLS_PUBLIC_KEY_OFFSET)).to_big_endian());
		let stake_slot = H256((base_pos + U256::from(TOTAL_STAKE_OFFSET)).to_big_endian());

		(bls_string_slot, stake_slot)
	}

	/// Convert RPC block proof to BlockProof.
	fn convert_rpc_block_proof(
		&self,
		rpc_proof: &RpcBlockProof,
	) -> Result<BlockProof, ProverError> {
		let aggregate_signature = hex_to_bytes(&rpc_proof.bls_aggregated_signature)?;

		let participant_keys: Result<Vec<_>, _> = rpc_proof
			.signed_bls_keys
			.iter()
			.map(|k| {
				let bytes = hex_to_bytes(k)?;
				let len = bytes.len();
				bytes.try_into().map_err(|_| ProverError::InvalidBlsKeyLength(len))
			})
			.collect();

		Ok(BlockProof { aggregate_signature, participant_keys: participant_keys? })
	}
}

/// Convert RPC proof nodes to PharosProofNode format.
pub fn rpc_to_proof_nodes(nodes: &[RpcProofNode]) -> Result<Vec<PharosProofNode>, ProverError> {
	nodes
		.iter()
		.map(|n| {
			Ok(PharosProofNode {
				proof_node: hex_to_bytes(&n.proof_node)?,
				next_begin_offset: n.next_begin_offset,
				next_end_offset: n.next_end_offset,
			})
		})
		.collect()
}

/// Determine the number of BLS data slots from a hex-encoded string header value.
///
/// For Solidity long strings, the header slot contains `length * 2 + 1`.
/// The byte length is `(value - 1) / 2`, and the slot count is `ceil(length / 32)`.
fn bls_data_slots_from_hex(hex_value: &str) -> Result<u64, ProverError> {
	let bytes = hex_to_bytes(hex_value)?;
	let mut padded = [0u8; 32];
	if bytes.len() <= 32 {
		padded[32 - bytes.len()..].copy_from_slice(&bytes);
	}
	let val = U256::from_big_endian(&padded);
	let val_bytes = val.to_big_endian();
	let lowest_byte = val_bytes[31];

	if lowest_byte & 1 == 0 {
		// Short string - data is in the header itself
		Ok(0)
	} else {
		// Long string - header = length * 2 + 1
		let length = (val - U256::from(1)) / U256::from(2);
		let str_len = length.low_u64();
		Ok((str_len + 31) / 32)
	}
}

/// Keccak256 hash using sp_core.
fn keccak256(data: &[u8]) -> [u8; 32] {
	sp_core::keccak_256(data)
}
