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
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_variables)]

extern crate alloc;

pub mod error;

use alloc::format;
use alloy_rlp::Decodable;
pub use error::Error;
use evm_state_machine::{
	derive_array_item_key, derive_map_key, get_contract_account, get_value_from_proof, prelude::*,
};
use geth_primitives::{CodecHeader, Header};
use ismp::{
	consensus::{
		ConsensusStateId, IntermediateState, StateCommitment, StateMachineHeight, StateMachineId,
	},
	host::StateMachine,
	messaging::Keccak256,
};
use primitive_types::{H160, H256, U128, U256};

// Constants

/// Slot for the disputeGames map in DisputeFactory contract
pub const DISPUTE_GAMES_SLOT: u64 = 103;
/// Slot for the gameImpls map in DisputeFactory contract.
///
/// In the pinned DisputeGameFactory (commit `f707883...`) `gameImpls`, `initBonds`, and
/// `_disputeGames` are three sequential mappings; `_disputeGames` is at 103, so `gameImpls`
/// is at 101.
pub const GAME_IMPLS_SLOT: u64 = 101;
/// Slot for the l2Outputs array in the L2Oracle contract
pub const L2_OUTPUTS_SLOT: u64 = 3;

/// Slot of the `claimData[]` dynamic array inside a FaultDisputeGame proxy. For a dynamic array
/// Solidity stores the element count in this slot itself (the elements live at
/// `keccak256(slot)`). A freshly created, unchallenged game holds exactly one entry — the root
/// claim appended in `initialize()` — and every `move()` appends another, so `claimData.length`
/// reads `1` iff the game is unchallenged. Matches the FaultDisputeGame implementation currently
/// deployed across the Superchain (mainnet impl `0x6dDBa0…7499`) where
/// `createdAt`/`resolvedAt`/`status` and assorted flags pack into slot 0 and
/// `l2BlockNumberChallenger` takes slot 1, leaving `claimData` at slot 2.
pub const FAULT_DISPUTE_CLAIM_DATA_SLOT: u64 = 2;

/// Slot of `counteredByIntermediateRootIndexPlusOne` inside Base's AggregateVerifier.
/// The value is `0` for unchallenged games and `intermediateRootIndex + 1` once challenged.
pub const AGGREGATE_VERIFIER_COUNTERED_BY_SLOT: u64 = 5;

/// Known FaultDisputeGame-style implementations whose storage layouts we can verify against.
#[derive(
	codec::Encode,
	codec::Decode,
	scale_info::TypeInfo,
	Debug,
	Clone,
	PartialEq,
	Eq,
	codec::DecodeWithMemTracking,
)]
pub enum DisputeGameImpl {
	/// Succinct's `OPSuccinctDisputeGame` — no challenge mechanism, unchallenged by construction.
	OPSuccinct,
	/// Optimism's `FaultDisputeGame` (and the inheriting `PermissionedDisputeGame`). Unchallenged
	/// when `claimData.length == 1` — i.e. only the root claim has been registered and no `move()`
	/// (attack or defense) has appended a counter-claim.
	FaultDisputeGame,
	/// Base's multiproof `AggregateVerifier`. Unchallenged when
	/// `counteredByIntermediateRootIndexPlusOne == 0`.
	AggregateVerifier,
}

/// Per-game-type verification configuration. Binds a `gameType` to its expected implementation
/// address (enforced via a `gameImpls[gameType]` storage proof against the factory) and to a
/// known storage layout (`kind`) used for the "not challenged" check on the game proxy.
#[derive(
	codec::Encode,
	codec::Decode,
	scale_info::TypeInfo,
	Debug,
	Clone,
	PartialEq,
	Eq,
	codec::DecodeWithMemTracking,
)]
pub struct GameTypeConfig {
	/// The `GameType` registered in the DisputeGameFactory.
	pub game_type: u32,
	/// The expected implementation address the factory must return for `gameImpls[game_type]`.
	pub expected_impl: H160,
	/// The storage layout to use when verifying the proxy's "not challenged" slot.
	pub kind: DisputeGameImpl,
}

#[derive(codec::Encode, codec::Decode, Debug)]
pub struct OptimismPayloadProof {
	/// Actual state root of the optimism execution layer
	pub state_root: H256,
	/// Storage root hash of the optimism withdrawal contracts
	pub withdrawal_storage_root: H256,
	/// Optimism Block hash at which the values aboved were fetched
	pub l2_block_hash: H256,
	/// L2Oracle contract version
	pub version: H256,
	/// Membership Proof for the L2Oracle contract account in the ethereum world trie
	pub l2_oracle_proof: Vec<Vec<u8>>,
	/// Membership proof for output root in l2Outputs array
	pub output_root_proof: Vec<Vec<u8>>,
	/// Membership proof Timestamp and block number in the l2Outputs array
	pub multi_proof: Vec<Vec<u8>>,
	/// Index of the output root that needs to be proved in the l2Outputs array
	pub output_root_index: u64,
	/// Block number
	pub block_number: u64,
	/// Timestamp
	pub timestamp: u64,
}

pub fn verify_optimism_payload<H: Keccak256 + Send + Sync>(
	payload: OptimismPayloadProof,
	root: H256,
	l2_oracle_address: H160,
	consensus_state_id: ConsensusStateId,
) -> Result<IntermediateState, Error> {
	let storage_root =
		get_contract_account::<H>(payload.l2_oracle_proof, &l2_oracle_address.0, root)?
			.storage_root
			.0
			.into();

	let output_root = calculate_output_root::<H>(
		payload.version,
		payload.state_root,
		payload.withdrawal_storage_root,
		payload.l2_block_hash,
	);
	let output_root_key = derive_array_item_key::<H>(L2_OUTPUTS_SLOT, payload.output_root_index, 0);

	let proof_value = match get_value_from_proof::<H>(
		output_root_key,
		storage_root,
		payload.output_root_proof,
	)? {
		Some(value) => value.clone(),
		_ => Err(Error::OutputRootSlotMissing)?,
	};

	let proof_value = <alloy_primitives::U256 as Decodable>::decode(&mut &*proof_value)
		.map_err(|_| Error::DecodeOutputRoot(format!("{:?}", &proof_value)))?
		.to_be_bytes::<32>();

	if proof_value != output_root.0 {
		return Err(Error::OutputRootMismatch);
	}

	// verify timestamp and block number
	let timestamp_block_number_key =
		derive_array_item_key::<H>(L2_OUTPUTS_SLOT, payload.output_root_index, 1);
	let block_and_timestamp = match get_value_from_proof::<H>(
		timestamp_block_number_key,
		storage_root,
		payload.multi_proof,
	)? {
		Some(value) => value.clone(),
		_ => Err(Error::BlockTimestampSlotMissing)?,
	};

	let block_and_timestamp =
		<alloy_primitives::U256 as Decodable>::decode(&mut &*block_and_timestamp)
			.map_err(|_| Error::DecodeBlockTimestamp(format!("{:?}", &block_and_timestamp)))?
			.to_be_bytes::<32>();

	let block_and_timestamp = U256::from_big_endian(&block_and_timestamp);
	// Timestamp is contained in the first two u64 values
	let timestamp = block_and_timestamp.low_u128() as u64;

	// Block number occupies the last two u64 values
	let mut block_number = [0u64; 2];
	block_number.copy_from_slice(&block_and_timestamp.0[2..]);
	let block_number = U128(block_number).as_u128() as u64;

	if payload.timestamp != timestamp || payload.block_number != block_number {
		return Err(Error::BlockTimestampMismatch);
	}

	Ok(IntermediateState {
		height: StateMachineHeight {
			id: StateMachineId {
				// note: This will state machine id should not be used to store the state commitment
				state_id: StateMachine::Evm(Default::default()),
				consensus_state_id,
			},
			height: payload.block_number,
		},
		commitment: StateCommitment {
			timestamp: payload.timestamp,
			overlay_root: None,
			state_root: payload.state_root,
		},
	})
}

#[derive(codec::Encode, codec::Decode, Debug, Clone)]
pub struct OptimismDisputeGameProof {
	/// Op stack header
	pub header: CodecHeader,
	/// Storage root hash of the optimism withdrawal contracts
	pub withdrawal_storage_root: H256,
	/// L2Oracle contract version
	pub version: H256,
	/// Membership Proof for the DisputeFactory contract account in the ethereum world trie
	pub dispute_factory_proof: Vec<Vec<u8>>,
	/// Membership proof for dispute game in disputeGames map
	pub dispute_game_proof: Vec<Vec<u8>>,
	/// Storage proof against the DisputeFactory for `gameImpls[game_type]`. Used to bind the
	/// proxy's storage layout to a known implementation.
	pub game_impl_proof: Vec<Vec<u8>>,
	/// Account proof for the dispute-game proxy in the ethereum world trie.
	pub proxy_account_proof: Vec<Vec<u8>>,
	/// Storage proof against the proxy for the "not challenged" slot. Empty for `OPSuccinct`
	/// games, which have no challenge mechanism.
	pub challenge_proof: Vec<Vec<u8>>,
	/// Dispute game proxy address
	pub proxy: H160,
	/// Extra data that was used in initializing the dispute game
	pub extra_data: Vec<u8>,
	/// Game type
	pub game_type: u32,
	/// L1 Timestamp at game creation
	pub timestamp: u64,
}

// https://github.com/ethereum-optimism/optimism/blob/f707883038d527cbf1e9f8ea513fe33255deadbc/packages/contracts-bedrock/src/dispute/DisputeGameFactory.sol#L127
pub fn get_game_uuid<H: Keccak256>(game_type: u32, root_claim: H256, extra_data: Vec<u8>) -> H256 {
	let tokens = [
		ethabi::Token::Uint(game_type.into()),
		ethabi::Token::FixedBytes(root_claim.0.to_vec()),
		ethabi::Token::Bytes(extra_data),
	];
	let encoded = ethabi::encode(&tokens);
	H::keccak256(&encoded)
}

pub fn calculate_output_root<H: Keccak256>(
	version: H256,
	state_root: H256,
	withdrawal_storage_root: H256,
	l2_block_hash: H256,
) -> H256 {
	let mut buf = Vec::with_capacity(128);
	buf.extend_from_slice(&version[..]);
	buf.extend_from_slice(&state_root[..]);
	buf.extend_from_slice(&withdrawal_storage_root[..]);
	buf.extend_from_slice(&l2_block_hash[..]);

	H::keccak256(&buf)
}

// https://github.com/ethereum-optimism/optimism/blob/f707883038d527cbf1e9f8ea513fe33255deadbc/packages/contracts-bedrock/src/libraries/DisputeTypes.sol#L94
/// Game types
pub const CANNON: u32 = 0;
pub const _PERMISSIONED: u32 = 1;

pub fn verify_optimism_dispute_game_proof<H: Keccak256 + Send + Sync>(
	payload: OptimismDisputeGameProof,
	root: H256,
	dispute_factory_address: H160,
	game_type_configs: Vec<GameTypeConfig>,
	consensus_state_id: ConsensusStateId,
) -> Result<IntermediateState, Error> {
	// Find the per-game-type configuration for this proof's game type.
	let game_config = game_type_configs
		.iter()
		.find(|c| c.game_type == payload.game_type)
		.ok_or(Error::UnsupportedGameType(payload.game_type))?
		.clone();

	let factory_storage_root =
		get_contract_account::<H>(payload.dispute_factory_proof, &dispute_factory_address.0, root)?
			.storage_root
			.0
			.into();
	let l2_block_hash = Header::from(&payload.header).hash::<H>();

	let root_claim = calculate_output_root::<H>(
		payload.version,
		payload.header.state_root,
		payload.withdrawal_storage_root,
		l2_block_hash,
	);

	let game_uuid = get_game_uuid::<H>(payload.game_type, root_claim, payload.extra_data);

	let dispute_game_key = derive_map_key::<H>(game_uuid.0.to_vec(), DISPUTE_GAMES_SLOT);

	// Does the dispute game's unique identifier exist in the _disputeGames map?
	let proof_value = match get_value_from_proof::<H>(
		dispute_game_key.0.to_vec(),
		factory_storage_root,
		payload.dispute_game_proof,
	)? {
		Some(value) => value.clone(),
		_ => Err(Error::DisputeGameIdMissing)?,
	};

	let mut encoded_game_id = <alloy_primitives::Bytes as Decodable>::decode(&mut &*proof_value)
		.map_err(|_| Error::DecodeDisputeGameId(format!("{:?}", &proof_value)))?
		.0
		.to_vec();

	let game_id = get_game_id(payload.game_type, payload.timestamp, payload.proxy);
	let game_id_bytes = game_id.to_big_endian();

	// Pad the encoded game id gotten from proof with zeros so it becomes 32 bytes long
	(0..game_id_bytes.len().saturating_sub(encoded_game_id.len()))
		.for_each(|_| encoded_game_id.insert(0, 0));

	// Derived game id must be equal to encoded game id
	if encoded_game_id != game_id_bytes {
		Err(Error::DisputeGameIdMismatch)?
	}

	// Bind the proxy's storage layout to the expected implementation by proving
	// `gameImpls[game_type]` in the factory matches the configured address. This is what makes
	// the per-kind "not challenged" check below meaningful: a factory upgrade that swaps
	// `gameImpls` to an implementation with a different layout would fail this check.
	let game_type_key = {
		let mut key = vec![0u8; 32];
		key[28..].copy_from_slice(&payload.game_type.to_be_bytes());
		derive_map_key::<H>(key, GAME_IMPLS_SLOT)
	};
	let impl_value = get_value_from_proof::<H>(
		game_type_key.0.to_vec(),
		factory_storage_root,
		payload.game_impl_proof,
	)?
	.ok_or(Error::GameImplsMissing)?;
	let impl_address = decode_address_from_storage_value(&impl_value)?;
	if impl_address != game_config.expected_impl {
		Err(Error::GameImplMismatch {
			game_type: payload.game_type,
			actual: impl_address,
			expected: game_config.expected_impl,
		})?
	}

	// Prove the proxy account, then verify "not challenged" against its storage root.
	verify_not_challenged::<H>(
		&game_config.kind,
		root,
		payload.proxy,
		payload.proxy_account_proof,
		payload.challenge_proof,
	)?;

	Ok(IntermediateState {
		height: StateMachineHeight {
			id: StateMachineId {
				// note: This will state machine id should not be used to store the state commitment
				state_id: StateMachine::Evm(Default::default()),
				consensus_state_id,
			},
			height: payload.header.number.low_u64(),
		},
		commitment: StateCommitment {
			timestamp: payload.header.timestamp,
			overlay_root: None,
			state_root: payload.header.state_root,
		},
	})
}

/// Decodes an address (20 bytes) from an RLP-encoded storage-trie leaf value.
///
/// Storage values are RLP-encoded without leading zeros; pad on the left to 32 bytes and take
/// the last 20.
fn decode_address_from_storage_value(value: &[u8]) -> Result<H160, Error> {
	let raw = <alloy_primitives::Bytes as Decodable>::decode(&mut &*value)
		.map_err(|_| Error::DecodeStorageValue(format!("{:?}", value)))?
		.0
		.to_vec();
	if raw.len() > 32 {
		Err(Error::StorageValueTooLong)?
	}
	let mut padded = vec![0u8; 32 - raw.len()];
	padded.extend_from_slice(&raw);
	let mut addr = [0u8; 20];
	addr.copy_from_slice(&padded[12..]);
	Ok(H160(addr))
}

/// Verifies that the dispute game at `proxy_address` has not been challenged. The check varies
/// by implementation kind. For `OPSuccinct`, no challenge mechanism exists so the proof fields
/// are not consulted.
fn verify_not_challenged<H: Keccak256 + Send + Sync>(
	kind: &DisputeGameImpl,
	root: H256,
	proxy_address: H160,
	proxy_account_proof: Vec<Vec<u8>>,
	challenge_proof: Vec<Vec<u8>>,
) -> Result<(), Error> {
	if matches!(kind, DisputeGameImpl::OPSuccinct) {
		// OPSuccinctDisputeGame has no challenge mechanism by construction, so any game we
		// accepted as registered in the factory is unchallenged.
		return Ok(());
	}

	let proxy_storage_root =
		get_contract_account::<H>(proxy_account_proof, &proxy_address.0, root)?
			.storage_root
			.0
			.into();

	match kind {
		DisputeGameImpl::FaultDisputeGame => {
			// `claimData` is a dynamic `ClaimData[]` at `FAULT_DISPUTE_CLAIM_DATA_SLOT`. Solidity
			// stores a dynamic array's element count in the slot itself (the elements live at
			// `keccak256(slot)`). A freshly created, unchallenged game holds exactly one entry —
			// the root claim appended in `initialize()` — and every `move()` (attack or defense)
			// appends another. So `claimData.length == 1` iff the game has not been challenged.
			// Any other length (including absence, i.e. length 0 for a game that never registered
			// its root claim) is rejected.
			//
			// The MPT trie path for a direct storage slot is `keccak256(slot)`.
			let storage_key = U256::from(FAULT_DISPUTE_CLAIM_DATA_SLOT).to_big_endian();
			let trie_path = H::keccak256(&storage_key);
			let value = get_value_from_proof::<H>(
				trie_path.0.to_vec(),
				proxy_storage_root,
				challenge_proof,
			)?
			.ok_or(Error::ClaimDataSlotMissing)?;
			let raw = <alloy_primitives::Bytes as Decodable>::decode(&mut &*value)
				.map_err(|_| Error::DecodeClaimData(format!("{:?}", value)))?
				.0
				.to_vec();
			if raw.len() > 32 {
				Err(Error::ClaimDataTooLong)?
			}
			// RLP strips leading zeros from the stored length; reconstruct the uint256 and require
			// it to be exactly one.
			if U256::from_big_endian(&raw) != U256::one() {
				Err(Error::FaultDisputeGameChallenged)?
			}
			Ok(())
		},
		DisputeGameImpl::AggregateVerifier => {
			// `counteredByIntermediateRootIndexPlusOne` is a uint256 at a fixed slot.
			// Unchallenged <=> value is zero, which in the storage trie means either absent or
			// encoded as zero. `get_value_from_proof` returns `None` for absent keys.
			//
			// The MPT trie path for a direct storage slot is `keccak256(slot)`.
			let storage_key =
				H256(U256::from(AGGREGATE_VERIFIER_COUNTERED_BY_SLOT).to_big_endian());
			let trie_path = H::keccak256(&storage_key.0);
			let value = get_value_from_proof::<H>(
				trie_path.0.to_vec(),
				proxy_storage_root,
				challenge_proof,
			)?;
			match value {
				None => Ok(()),
				Some(v) => {
					let raw = <alloy_primitives::Bytes as Decodable>::decode(&mut &*v)
						.map_err(|_| Error::DecodeCounteredBy(format!("{:?}", v)))?
						.0
						.to_vec();
					if raw.len() > 32 {
						Err(Error::CounteredByTooLong)?
					}
					// RLP strips leading zeros from the stored uint256. Compare against a slice
					// of zeros of the same length: any non-zero byte means the value is non-zero
					// and the game has been challenged.
					const ZERO_WORD: [u8; 32] = [0u8; 32];
					if raw.as_slice() != &ZERO_WORD[..raw.len()] {
						Err(Error::AggregateVerifierChallenged)?
					}
					Ok(())
				},
			}
		},
		DisputeGameImpl::OPSuccinct => unreachable!("handled above"),
	}
}

// https://github.com/ethereum-optimism/optimism/blob/f707883038d527cbf1e9f8ea513fe33255deadbc/packages/contracts-bedrock/src/dispute/lib/LibGameId.sol#L15
fn get_game_id(game_type: u32, timestamp: u64, game_proxy: H160) -> U256 {
	let mut bytes = U256::zero();
	// Use bitwise shifts and bitwise OR for packing
	bytes |= U256::from(game_type) << 224;
	bytes |= U256::from(timestamp) << 160;

	let mut addr = vec![0u8; 12];
	addr.extend_from_slice(&game_proxy.0);
	let proxy = U256::from_big_endian(&addr);

	bytes |= proxy;
	bytes
}
