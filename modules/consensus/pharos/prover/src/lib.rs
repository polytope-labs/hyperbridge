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

use geth_primitives::CodecHeader;
use pharos_primitives::{
	BlsPublicKey, BlockProof, Config, PharosProofNode, ValidatorInfo, ValidatorSet,
	ValidatorSetProof, VerifierStateUpdate, STAKING_CONTRACT_ADDRESS,
};
use pharos_verifier::state_proof::StakingContractLayout;
use primitive_types::{H160, H256, U256};
use rpc::{
	hex_to_bytes, hex_to_h256, hex_to_u64, PharosRpcClient, RpcBlock, RpcBlockProof, RpcProofNode,
	RpcValidatorInfo,
};
use std::{marker::PhantomData, sync::Arc};

/// Pharos prover for constructing light client updates.
#[derive(Clone)]
pub struct PharosProver<C: Config> {
	pub rpc: Arc<PharosRpcClient>,
	storage_layout: StakingContractLayout,
	_config: PhantomData<C>,
}

impl<C: Config> PharosProver<C> {
	/// Create a new prover with the given RPC endpoint.
	pub fn new(endpoint: impl Into<String>) -> Self {
		Self {
			rpc: Arc::new(PharosRpcClient::new(endpoint)),
			storage_layout: StakingContractLayout::default(),
			_config: PhantomData,
		}
	}

	/// Create a new prover with a custom storage layout.
	pub fn with_storage_layout(endpoint: impl Into<String>, layout: StakingContractLayout) -> Self {
		Self {
			rpc: Arc::new(PharosRpcClient::new(endpoint)),
			storage_layout: layout,
			_config: PhantomData,
		}
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
		let block = self.rpc.get_block_by_number(block_number).await?;
		let header = self.convert_block_to_header(&block)?;

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
	/// The storage layout follows the Pharos staking contract:
	/// - Slot 6: totalStake
	/// - Slot 22: activePoolSets (EnumerableSet._values array length)
	/// - keccak256(22): array elements (pool IDs)
	/// - For each pool ID: validator data via mapping at slot 0
	pub async fn fetch_validator_set_proof(
		&self,
		block_number: u64,
	) -> Result<ValidatorSetProof, ProverError> {
		let address = H160::from_slice(STAKING_CONTRACT_ADDRESS.as_slice());

		// Fetch base slots (totalStake, activePoolSets length)
		let base_keys = vec![
			self.storage_layout.raw_slot_key(self.storage_layout.total_stake_slot),
			self.storage_layout.raw_slot_key(self.storage_layout.active_pool_set_slot),
		];

		let base_proof = self.rpc.get_proof(address, base_keys.clone(), block_number).await?;

		// Get validator count from activePoolSets length (slot 22)
		let validator_count = if base_proof.storage_proof.len() >= 2 {
			hex_to_u64(&base_proof.storage_proof[1].value).unwrap_or(0)
		} else {
			0
		};

		// Fetch pool IDs from the activePoolSets array
		let mut pool_id_keys = Vec::new();
		for i in 0..validator_count {
			pool_id_keys.push(self.array_element_key(self.storage_layout.active_pool_set_slot, i));
		}

		let pool_id_proof = if !pool_id_keys.is_empty() {
			self.rpc.get_proof(address, pool_id_keys.clone(), block_number).await?
		} else {
			base_proof.clone()
		};

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

		// For each pool ID, fetch validator data (BLS key slots + stake)
		let mut validator_keys = Vec::new();
		for pool_id in &pool_ids {
			validator_keys.extend(self.get_validator_storage_keys(pool_id));
		}

		// Combine all storage keys for the final proof
		let mut all_keys = base_keys;
		all_keys.extend(pool_id_keys);
		all_keys.extend(validator_keys);

		// Fetch the complete proof with all keys
		let full_proof = self.rpc.get_proof(address, all_keys, block_number).await?;

		// Convert account proof nodes to PharosProofNode format
		let account_proof = rpc_to_proof_nodes(&full_proof.account_proof)?;

		// Collect all storage proofs into a single proof set
		let mut storage_proof: Vec<PharosProofNode> = Vec::new();
		for sp in &full_proof.storage_proof {
			let nodes = rpc_to_proof_nodes(&sp.proof)?;
			for node in nodes {
				if !storage_proof.contains(&node) {
					storage_proof.push(node);
				}
			}
		}

		// Parse storage_hash from RPC response
		let storage_hash = hex_to_h256(&full_proof.storage_hash)?;

		// Parse raw_account_value from RPC response
		let raw_account_value = hex_to_bytes(&full_proof.raw_value)?;

		Ok(ValidatorSetProof { account_proof, storage_proof, storage_hash, raw_account_value })
	}

	/// Calculate the storage key for a dynamic array element.
	fn array_element_key(&self, base_slot: u64, index: u64) -> H256 {
		let slot_bytes = U256::from(base_slot).to_big_endian();
		let base_key = keccak256(&slot_bytes);
		let base_pos = U256::from_big_endian(&base_key);
		let element_pos = base_pos + U256::from(index);
		H256(element_pos.to_big_endian())
	}

	/// Get storage keys for a specific validator's data.
	///
	/// Returns keys for:
	/// - BLS public key string slot (offset 3 from validator base)
	/// - BLS public key data slots (3 slots for long string)
	/// - totalStake (offset 8 from validator base)
	fn get_validator_storage_keys(&self, pool_id: &H256) -> Vec<H256> {
		const BLS_PUBLIC_KEY_OFFSET: u64 = 3;
		const TOTAL_STAKE_OFFSET: u64 = 8;
		const BLS_STRING_DATA_SLOTS: u64 = 3;

		let mut keys = Vec::new();

		// Calculate validator base slot: keccak256(pool_id || mapping_slot)
		let mut data = [0u8; 64];
		data[..32].copy_from_slice(pool_id.as_bytes());
		data[32..64].copy_from_slice(
			&U256::from(self.storage_layout.validators_mapping_slot).to_big_endian(),
		);
		let base_slot = H256::from(keccak256(&data));
		let base_pos = U256::from_big_endian(base_slot.as_bytes());

		// BLS public key string slot (offset 3)
		let bls_string_slot = H256((base_pos + U256::from(BLS_PUBLIC_KEY_OFFSET)).to_big_endian());
		keys.push(bls_string_slot);

		// BLS public key data slots (for long strings at keccak256(string_slot))
		let bls_data_base = H256::from(keccak256(bls_string_slot.as_bytes()));
		let bls_data_base_pos = U256::from_big_endian(bls_data_base.as_bytes());
		for i in 0..BLS_STRING_DATA_SLOTS {
			let slot_pos = bls_data_base_pos + U256::from(i);
			keys.push(H256(slot_pos.to_big_endian()));
		}

		// totalStake slot (offset 8)
		keys.push(H256((base_pos + U256::from(TOTAL_STAKE_OFFSET)).to_big_endian()));

		keys
	}

	/// Convert RPC block to CodecHeader.
	fn convert_block_to_header(&self, block: &RpcBlock) -> Result<CodecHeader, ProverError> {
		use ethabi::ethereum_types::H64;

		Ok(CodecHeader {
			parent_hash: hex_to_h256(&block.parent_hash)?,
			uncle_hash: hex_to_h256(&block.sha3_uncles)?,
			coinbase: {
				let bytes = hex_to_bytes(&block.miner)?;
				if bytes.len() != 20 {
					return Err(ProverError::InvalidAddressLength(bytes.len()));
				}
				H160::from_slice(&bytes)
			},
			state_root: hex_to_h256(&block.state_root)?,
			transactions_root: hex_to_h256(&block.transactions_root)?,
			receipts_root: hex_to_h256(&block.receipts_root)?,
			logs_bloom: {
				let bytes = hex_to_bytes(&block.logs_bloom)?;
				if bytes.len() != 256 {
					return Err(ProverError::InvalidLogsBloomLength(bytes.len()));
				}
				let mut bloom = [0u8; 256];
				bloom.copy_from_slice(&bytes);
				bloom.into()
			},
			difficulty: U256::from_str_radix(block.difficulty.trim_start_matches("0x"), 16)
				.unwrap_or_default(),
			number: U256::from_str_radix(block.number.trim_start_matches("0x"), 16)
				.unwrap_or_default(),
			gas_limit: hex_to_u64(&block.gas_limit)?,
			gas_used: hex_to_u64(&block.gas_used)?,
			timestamp: hex_to_u64(&block.timestamp)?,
			extra_data: hex_to_bytes(&block.extra_data)?,
			mix_hash: H256::default(),
			nonce: {
				let bytes =
					block.nonce.as_ref().map(|n| hex_to_bytes(n)).transpose()?.unwrap_or_default();
				if bytes.len() == 8 {
					let mut arr = [0u8; 8];
					arr.copy_from_slice(&bytes);
					H64::from(arr)
				} else {
					H64::default()
				}
			},
			base_fee_per_gas: block
				.base_fee_per_gas
				.as_ref()
				.and_then(|f| U256::from_str_radix(f.trim_start_matches("0x"), 16).ok()),
			withdrawals_hash: block
				.withdrawals_root
				.as_ref()
				.map(|r| hex_to_h256(r))
				.transpose()?,
			blob_gas_used: block.blob_gas_used.as_ref().map(|g| hex_to_u64(g)).transpose()?,
			excess_blob_gas_used: block
				.excess_blob_gas
				.as_ref()
				.map(|g| hex_to_u64(g))
				.transpose()?,
			parent_beacon_root: block
				.parent_beacon_block_root
				.as_ref()
				.map(|r| hex_to_h256(r))
				.transpose()?,
			requests_hash: None,
		})
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
fn rpc_to_proof_nodes(nodes: &[RpcProofNode]) -> Result<Vec<PharosProofNode>, ProverError> {
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

/// Simple keccak256 implementation using tiny_keccak.
fn keccak256(data: &[u8]) -> [u8; 32] {
	use tiny_keccak::{Hasher, Keccak};
	let mut hasher = Keccak::v256();
	let mut output = [0u8; 32];
	hasher.update(data);
	hasher.finalize(&mut output);
	output
}
