// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! The beacon state, in both the shapes this client has to read.
//!
//! Gloas does not extend the pre-Gloas state, it reshapes it: the execution payload header is
//! replaced by a block hash, eight builder and payload-timeliness fields are appended, twelve
//! lists become `ProgressiveList`, and the container merkleizes progressively. Those are two
//! different SSZ types, and no single Rust struct is both.
//!
//! They used to be one struct behind the `glamsterdam` feature, which meant the fork was chosen at
//! build time: a relayer had to be rebuilt at the fork, and a binary built for one side could not
//! read a state from the other. The beacon API already reports which shape it is sending, in the
//! `version` field of the response, so the choice belongs at runtime. [`BeaconState`] is the enum
//! over both, and everything downstream matches on it.
//!
//! Only a handful of fields are ever read; the rest exist so the state hashes to the root the sync
//! committee signed. The accessors below cover the read set, so callers rarely match directly.

use crate::{
	consensus_types::{
		BeaconBlockHeader, Checkpoint, Eth1Data, ExecutionPayloadHeader, Fork, HistoricalSummary,
		SyncCommittee, Validator, Withdrawal,
	},
	constants::*,
	electra::{PendingConsolidation, PendingDeposit, PendingPartialWithdrawal},
	gloas::{
		Builder, BuilderIndex, BuilderPendingPayment, BuilderPendingWithdrawal,
		ExecutionPayloadBid, PTC_SIZE,
	},
	ssz::Root,
};
use alloc::vec::Vec;
use ssz_types::{typenum::Unsigned, BitVector, FixedVector, ProgressiveList, VariableList};
use tree_hash::Hash256;

/// The pre-Gloas beacon state, through Fulu.
///
/// An ordinary SSZ container: the lists are bounded and merkleization is the balanced tree.
#[derive(
	Default,
	Debug,
	Clone,
	PartialEq,
	Eq,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	codec::Encode,
	codec::Decode,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(bound = ""))]
pub struct BeaconStateElectra<
	SLOTS_PER_HISTORICAL_ROOT: Unsigned,
	HISTORICAL_ROOTS_LIMIT: Unsigned,
	ETH1_DATA_VOTES_BOUND: Unsigned,
	VALIDATOR_REGISTRY_LIMIT: Unsigned,
	EPOCHS_PER_HISTORICAL_VECTOR: Unsigned,
	EPOCHS_PER_SLASHINGS_VECTOR: Unsigned,
	SYNC_COMMITTEE_SIZE: Unsigned,
	BYTES_PER_LOGS_BLOOM: Unsigned,
	MAX_EXTRA_DATA_BYTES: Unsigned,
	PENDING_DEPOSITS_LIMIT: Unsigned,
	PENDING_CONSOLIDATIONS_LIMIT: Unsigned,
	PENDING_PARTIAL_WITHDRAWALS_LIMIT: Unsigned,
	PROPOSER_LOOK_AHEAD_LIMIT: Unsigned,
> {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub genesis_time: u64,
	pub genesis_validators_root: Root,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub slot: Slot,
	pub fork: Fork,
	pub latest_block_header: BeaconBlockHeader,
	pub block_roots: FixedVector<Root, SLOTS_PER_HISTORICAL_ROOT>,
	pub state_roots: FixedVector<Root, SLOTS_PER_HISTORICAL_ROOT>,
	pub historical_roots: VariableList<Root, HISTORICAL_ROOTS_LIMIT>,
	pub eth1_data: Eth1Data,
	pub eth1_data_votes: VariableList<Eth1Data, ETH1_DATA_VOTES_BOUND>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub eth1_deposit_index: u64,
	pub validators: VariableList<Validator, VALIDATOR_REGISTRY_LIMIT>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
	pub balances: VariableList<Gwei, VALIDATOR_REGISTRY_LIMIT>,
	pub randao_mixes: FixedVector<Bytes32, EPOCHS_PER_HISTORICAL_VECTOR>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
	pub slashings: FixedVector<Gwei, EPOCHS_PER_SLASHINGS_VECTOR>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_u8_str_or_hex"))]
	pub previous_epoch_participation: VariableList<ParticipationFlags, VALIDATOR_REGISTRY_LIMIT>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_u8_str_or_hex"))]
	pub current_epoch_participation: VariableList<ParticipationFlags, VALIDATOR_REGISTRY_LIMIT>,
	pub justification_bits: BitVector<JUSTIFICATION_BITS_LENGTH>,
	pub previous_justified_checkpoint: Checkpoint,
	pub current_justified_checkpoint: Checkpoint,
	pub finalized_checkpoint: Checkpoint,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
	pub inactivity_scores: VariableList<u64, VALIDATOR_REGISTRY_LIMIT>,
	pub current_sync_committee: SyncCommittee<SYNC_COMMITTEE_SIZE>,
	pub next_sync_committee: SyncCommittee<SYNC_COMMITTEE_SIZE>,
	pub latest_execution_payload_header:
		ExecutionPayloadHeader<BYTES_PER_LOGS_BLOOM, MAX_EXTRA_DATA_BYTES>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub next_withdrawal_index: WithdrawalIndex,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub next_withdrawal_validator_index: ValidatorIndex,
	pub historical_summaries: VariableList<HistoricalSummary, HISTORICAL_ROOTS_LIMIT>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub deposit_requests_start_index: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub deposit_balance_to_consume: Gwei,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub exit_balance_to_consume: Gwei,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub earliest_exit_epoch: Epoch,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub consolidation_balance_to_consume: Gwei,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub earliest_consolidation_epoch: Epoch,
	pub pending_deposits: VariableList<PendingDeposit, PENDING_DEPOSITS_LIMIT>,
	pub pending_partial_withdrawals:
		VariableList<PendingPartialWithdrawal, PENDING_PARTIAL_WITHDRAWALS_LIMIT>,
	pub pending_consolidations: VariableList<PendingConsolidation, PENDING_CONSOLIDATIONS_LIMIT>,
	//  [New in Fulu:EIP7917]
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
	pub proposer_lookahead: FixedVector<ValidatorIndex, PROPOSER_LOOK_AHEAD_LIMIT>,
}

/// The Gloas beacon state.
///
/// A progressive container (EIP-7688) whose growable lists are `ProgressiveList` (EIP-7916). The
/// payload header is gone, replaced by `latest_block_hash` in the slot it used to occupy, which is
/// what keeps the execution leaf's generalized index where the client expects it. Note this shape
/// needs no execution payload bounds at all.
#[derive(
	Default,
	Debug,
	Clone,
	PartialEq,
	Eq,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	codec::Encode,
	codec::Decode,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(bound = ""))]
#[tree_hash(
	struct_behaviour = "progressive_container",
	active_fields(
		1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
		1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1
	)
)]
pub struct BeaconStateGloas<
	SLOTS_PER_HISTORICAL_ROOT: Unsigned,
	HISTORICAL_ROOTS_LIMIT: Unsigned,
	ETH1_DATA_VOTES_BOUND: Unsigned,
	EPOCHS_PER_HISTORICAL_VECTOR: Unsigned,
	EPOCHS_PER_SLASHINGS_VECTOR: Unsigned,
	SYNC_COMMITTEE_SIZE: Unsigned,
	PROPOSER_LOOK_AHEAD_LIMIT: Unsigned,
> {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub genesis_time: u64,
	pub genesis_validators_root: Root,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub slot: Slot,
	pub fork: Fork,
	pub latest_block_header: BeaconBlockHeader,
	pub block_roots: FixedVector<Root, SLOTS_PER_HISTORICAL_ROOT>,
	pub state_roots: FixedVector<Root, SLOTS_PER_HISTORICAL_ROOT>,
	pub historical_roots: VariableList<Root, HISTORICAL_ROOTS_LIMIT>,
	pub eth1_data: Eth1Data,
	pub eth1_data_votes: VariableList<Eth1Data, ETH1_DATA_VOTES_BOUND>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub eth1_deposit_index: u64,
	pub validators: ProgressiveList<Validator>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
	pub balances: ProgressiveList<Gwei>,
	pub randao_mixes: FixedVector<Bytes32, EPOCHS_PER_HISTORICAL_VECTOR>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
	pub slashings: FixedVector<Gwei, EPOCHS_PER_SLASHINGS_VECTOR>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_u8_str_or_hex"))]
	pub previous_epoch_participation: ProgressiveList<ParticipationFlags>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_u8_str_or_hex"))]
	pub current_epoch_participation: ProgressiveList<ParticipationFlags>,
	pub justification_bits: BitVector<JUSTIFICATION_BITS_LENGTH>,
	pub previous_justified_checkpoint: Checkpoint,
	pub current_justified_checkpoint: Checkpoint,
	pub finalized_checkpoint: Checkpoint,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
	pub inactivity_scores: ProgressiveList<u64>,
	pub current_sync_committee: SyncCommittee<SYNC_COMMITTEE_SIZE>,
	pub next_sync_committee: SyncCommittee<SYNC_COMMITTEE_SIZE>,
	// [New in Gloas:EIP7732] takes over the slot the payload header used to occupy, so the
	// generalized index of the execution leaf is unchanged.
	pub latest_block_hash: Hash32,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub next_withdrawal_index: WithdrawalIndex,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub next_withdrawal_validator_index: ValidatorIndex,
	pub historical_summaries: VariableList<HistoricalSummary, HISTORICAL_ROOTS_LIMIT>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub deposit_requests_start_index: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub deposit_balance_to_consume: Gwei,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub exit_balance_to_consume: Gwei,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub earliest_exit_epoch: Epoch,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub consolidation_balance_to_consume: Gwei,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub earliest_consolidation_epoch: Epoch,
	pub pending_deposits: ProgressiveList<PendingDeposit>,
	pub pending_partial_withdrawals: ProgressiveList<PendingPartialWithdrawal>,
	pub pending_consolidations: ProgressiveList<PendingConsolidation>,
	//  [New in Fulu:EIP7917]
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
	pub proposer_lookahead: FixedVector<ValidatorIndex, PROPOSER_LOOK_AHEAD_LIMIT>,
	// [New in Gloas:EIP7732] the builder registry and payload timeliness bookkeeping. Nothing
	// here is proven, but the fields are part of the container, so they have to be present for
	// the state to hash to the root the sync committee signed over.
	pub builders: ProgressiveList<Builder>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub next_withdrawal_builder_index: BuilderIndex,
	pub execution_payload_availability: BitVector<SLOTS_PER_HISTORICAL_ROOT>,
	pub builder_pending_payments:
		FixedVector<BuilderPendingPayment, BUILDER_PENDING_PAYMENTS_LIMIT>,
	pub builder_pending_withdrawals: ProgressiveList<BuilderPendingWithdrawal>,
	pub latest_execution_payload_bid: ExecutionPayloadBid,
	pub payload_expected_withdrawals: ProgressiveList<Withdrawal>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_seq_of_str"))]
	pub ptc_window: FixedVector<FixedVector<ValidatorIndex, PTC_SIZE>, PTC_WINDOW_LIMIT>,
}

/// A beacon state of whichever fork the node served.
///
/// The variant is chosen from the `version` the beacon API reports, not from a build flag, so one
/// binary reads either side of the fork and a relayer needs no rebuild when it happens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeaconState<
	SLOTS_PER_HISTORICAL_ROOT: Unsigned,
	HISTORICAL_ROOTS_LIMIT: Unsigned,
	ETH1_DATA_VOTES_BOUND: Unsigned,
	VALIDATOR_REGISTRY_LIMIT: Unsigned,
	EPOCHS_PER_HISTORICAL_VECTOR: Unsigned,
	EPOCHS_PER_SLASHINGS_VECTOR: Unsigned,
	SYNC_COMMITTEE_SIZE: Unsigned,
	BYTES_PER_LOGS_BLOOM: Unsigned,
	MAX_EXTRA_DATA_BYTES: Unsigned,
	PENDING_DEPOSITS_LIMIT: Unsigned,
	PENDING_CONSOLIDATIONS_LIMIT: Unsigned,
	PENDING_PARTIAL_WITHDRAWALS_LIMIT: Unsigned,
	PROPOSER_LOOK_AHEAD_LIMIT: Unsigned,
> {
	/// Any fork up to and including Fulu.
	Electra(
		BeaconStateElectra<
			SLOTS_PER_HISTORICAL_ROOT,
			HISTORICAL_ROOTS_LIMIT,
			ETH1_DATA_VOTES_BOUND,
			VALIDATOR_REGISTRY_LIMIT,
			EPOCHS_PER_HISTORICAL_VECTOR,
			EPOCHS_PER_SLASHINGS_VECTOR,
			SYNC_COMMITTEE_SIZE,
			BYTES_PER_LOGS_BLOOM,
			MAX_EXTRA_DATA_BYTES,
			PENDING_DEPOSITS_LIMIT,
			PENDING_CONSOLIDATIONS_LIMIT,
			PENDING_PARTIAL_WITHDRAWALS_LIMIT,
			PROPOSER_LOOK_AHEAD_LIMIT,
		>,
	),
	/// Gloas and later.
	Gloas(
		BeaconStateGloas<
			SLOTS_PER_HISTORICAL_ROOT,
			HISTORICAL_ROOTS_LIMIT,
			ETH1_DATA_VOTES_BOUND,
			EPOCHS_PER_HISTORICAL_VECTOR,
			EPOCHS_PER_SLASHINGS_VECTOR,
			SYNC_COMMITTEE_SIZE,
			PROPOSER_LOOK_AHEAD_LIMIT,
		>,
	),
}

/// Read the same field from either shape.
macro_rules! either {
	($self:ident, $field:ident) => {
		match $self {
			BeaconState::Electra(state) => &state.$field,
			BeaconState::Gloas(state) => &state.$field,
		}
	};
}

impl<
		SLOTS_PER_HISTORICAL_ROOT: Unsigned,
		HISTORICAL_ROOTS_LIMIT: Unsigned,
		ETH1_DATA_VOTES_BOUND: Unsigned,
		VALIDATOR_REGISTRY_LIMIT: Unsigned,
		EPOCHS_PER_HISTORICAL_VECTOR: Unsigned,
		EPOCHS_PER_SLASHINGS_VECTOR: Unsigned,
		SYNC_COMMITTEE_SIZE: Unsigned,
		BYTES_PER_LOGS_BLOOM: Unsigned,
		MAX_EXTRA_DATA_BYTES: Unsigned,
		PENDING_DEPOSITS_LIMIT: Unsigned,
		PENDING_CONSOLIDATIONS_LIMIT: Unsigned,
		PENDING_PARTIAL_WITHDRAWALS_LIMIT: Unsigned,
		PROPOSER_LOOK_AHEAD_LIMIT: Unsigned,
	>
	BeaconState<
		SLOTS_PER_HISTORICAL_ROOT,
		HISTORICAL_ROOTS_LIMIT,
		ETH1_DATA_VOTES_BOUND,
		VALIDATOR_REGISTRY_LIMIT,
		EPOCHS_PER_HISTORICAL_VECTOR,
		EPOCHS_PER_SLASHINGS_VECTOR,
		SYNC_COMMITTEE_SIZE,
		BYTES_PER_LOGS_BLOOM,
		MAX_EXTRA_DATA_BYTES,
		PENDING_DEPOSITS_LIMIT,
		PENDING_CONSOLIDATIONS_LIMIT,
		PENDING_PARTIAL_WITHDRAWALS_LIMIT,
		PROPOSER_LOOK_AHEAD_LIMIT,
	>
{
	/// True when this is a Gloas state.
	pub fn is_gloas(&self) -> bool {
		matches!(self, BeaconState::Gloas(_))
	}

	pub fn slot(&self) -> Slot {
		*either!(self, slot)
	}

	pub fn finalized_checkpoint(&self) -> &Checkpoint {
		either!(self, finalized_checkpoint)
	}

	pub fn current_sync_committee(&self) -> &SyncCommittee<SYNC_COMMITTEE_SIZE> {
		either!(self, current_sync_committee)
	}

	pub fn next_sync_committee(&self) -> &SyncCommittee<SYNC_COMMITTEE_SIZE> {
		either!(self, next_sync_committee)
	}

	pub fn latest_block_header(&self) -> &BeaconBlockHeader {
		either!(self, latest_block_header)
	}

	/// The execution block hash the state commits to.
	///
	/// Pre-Gloas it is a field of the payload header; from Gloas the header is gone and the hash
	/// stands alone in the slot it vacated.
	pub fn execution_block_hash(&self) -> &Hash32 {
		match self {
			BeaconState::Electra(state) => &state.latest_execution_payload_header.block_hash,
			BeaconState::Gloas(state) => &state.latest_block_hash,
		}
	}

	/// The state's hash tree root, merkleized the way its own fork requires.
	pub fn tree_hash_root(&self) -> Hash256 {
		use tree_hash::TreeHash;
		match self {
			BeaconState::Electra(state) => state.tree_hash_root(),
			BeaconState::Gloas(state) => state.tree_hash_root(),
		}
	}

	/// Prove a single field, addressed by generalized index.
	///
	/// The balanced and progressive shapes build a branch differently, and this is the only place
	/// that has to know which is which.
	pub fn prove_gindex(&self, gindex: u64) -> Result<Vec<Root>, tree_hash::proof::Error> {
		use tree_hash::proof::{generate_multiproof, ContainerFields, TreeHashFields};
		let branch = match self {
			BeaconState::Electra(state) => generate_multiproof(&state.field_roots(), &[gindex])?,
			BeaconState::Gloas(state) => state.prove_gindex(gindex)?.1,
		};
		Ok(branch.into_iter().map(Into::into).collect())
	}
}

/// The parts of a beacon block this client actually reads.
///
/// Blocks are fetched as json and never merkleized or ssz encoded here, and serde ignores fields
/// a struct does not name. So where the full `BeaconBlockBody` diverges at Gloas, and would need
/// two shapes like the state does, this one lean type deserializes either fork's block.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(bound = ""))]
pub struct BeaconBlockSummary<SYNC_COMMITTEE_SIZE: Unsigned> {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub slot: Slot,
	pub parent_root: Root,
	pub state_root: Root,
	pub body: BeaconBlockBodySummary<SYNC_COMMITTEE_SIZE>,
}

/// The one body field this client reads: the sync aggregate carrying the committee's signature.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(bound = ""))]
pub struct BeaconBlockBodySummary<SYNC_COMMITTEE_SIZE: Unsigned> {
	pub sync_aggregate: crate::consensus_types::SyncAggregate<SYNC_COMMITTEE_SIZE>,
}
