#![cfg_attr(not(feature = "std"), no_std)]
#[warn(unused_imports)]
#[warn(unused_variables)]
extern crate alloc;

pub mod crypto;
pub mod error;

use crate::error::Error;
use alloc::vec::Vec;
use ark_ec::CurveGroup;
use crypto::subtract_points_from_aggregate;
use ssz_rs::{
	calculate_multi_merkle_root, prelude::is_valid_merkle_branch, GeneralizedIndex, Merkleized,
	Node,
};
use sync_committee_primitives::{
	consensus_types::Checkpoint,
	constants::{Config, Root, DOMAIN_SYNC_COMMITTEE},
	types::{VerifierState, VerifierStateUpdate},
	util::{
		compute_domain, compute_epoch_at_slot, compute_fork_version, compute_signing_root,
		compute_sync_committee_period_at_slot, should_have_sync_committee_update,
	},
};

/// This function simply verifies a sync committee's attestation & it's finalized counterpart.
pub fn verify_sync_committee_attestation<C: Config>(
	trusted_state: VerifierState,
	mut update: VerifierStateUpdate,
) -> Result<VerifierState, Error> {
	if update.finality_proof.finality_branch.len() != C::FINALIZED_ROOT_INDEX_LOG2 as usize &&
		update.sync_committee_update.is_some() &&
		update.sync_committee_update.as_ref().unwrap().next_sync_committee_branch.len() !=
			C::NEXT_SYNC_COMMITTEE_INDEX_LOG2 as usize
	{
		Err(Error::InvalidUpdate("Finality branch is incorrect".into()))?
	}

	// Verify sync committee has super majority participants
	let sync_committee_bits = update.sync_aggregate.sync_committee_bits;
	let sync_aggregate_participants: u64 =
		sync_committee_bits.iter().as_bitslice().count_ones() as u64;

	if sync_aggregate_participants < ((2 * sync_committee_bits.len() as u64) / 3) + 1 {
		Err(Error::SyncCommitteeParticipantsTooLow)?
	}

	// Verify update is valid
	let is_valid_update = update.signature_slot > update.attested_header.slot &&
		update.attested_header.slot > update.finalized_header.slot;
	if !is_valid_update {
		Err(Error::InvalidUpdate(
			"relationship between slots does not meet the requirements".into(),
		))?
	}

	let state_period = trusted_state.state_period;
	let update_signature_period = compute_sync_committee_period_at_slot::<C>(update.signature_slot);
	if !(state_period..=state_period + 1).contains(&update_signature_period) {
		Err(Error::InvalidUpdate("State period does not contain signature period".into()))?
	}

	if update.attested_header.slot <= trusted_state.finalized_header.slot ||
		update.finality_proof.epoch <= trusted_state.latest_finalized_epoch
	{
		Err(Error::InvalidUpdate("Update is expired".into()))?
	}

	// Verify sync committee aggregate signature
	let sync_committee = if update_signature_period == state_period {
		trusted_state.current_sync_committee.clone()
	} else {
		trusted_state.next_sync_committee.clone()
	};

	let sync_committee_pubkeys = sync_committee.public_keys;

	let non_participant_pubkeys = sync_committee_bits
		.iter()
		.zip(sync_committee_pubkeys.iter())
		.filter_map(|(bit, key)| if !(*bit) { Some(key.clone()) } else { None })
		.collect::<Vec<_>>();

	let fork_version = compute_fork_version::<C>(compute_epoch_at_slot::<C>(update.signature_slot));

	let domain = compute_domain(
		DOMAIN_SYNC_COMMITTEE,
		Some(fork_version),
		Some(Root::from_bytes(C::GENESIS_VALIDATORS_ROOT.try_into().expect("Infallible"))),
		C::GENESIS_FORK_VERSION,
	)
	.map_err(|_| Error::InvalidUpdate("Failed to compute domain".into()))?;

	let signing_root = compute_signing_root(&mut update.attested_header, domain)
		.map_err(|_| Error::InvalidRoot("Failed to compute signing root".into()))?;

	let aggregate = subtract_points_from_aggregate(
		&sync_committee.aggregate_public_key,
		&non_participant_pubkeys,
	)?;

	let verify = bls::verify(
		&bls::point_to_pubkey(aggregate.into_affine()),
		&signing_root.as_bytes().to_vec(),
		&update.sync_aggregate.sync_committee_signature,
		&bls::DST_ETHEREUM.as_bytes().to_vec(),
	);

	if !verify {
		Err(Error::SignatureVerification)?
	}

	// Verify that the `finality_branch` confirms `finalized_header`
	// to match the finalized checkpoint root saved in the state of `attested_header`.
	// Note that the genesis finalized checkpoint root is represented as a zero hash.
	let mut finalized_checkpoint = Checkpoint {
		epoch: update.finality_proof.epoch,
		root: update
			.finalized_header
			.hash_tree_root()
			.map_err(|_| Error::MerkleizationError("Error hashing finalized header".into()))?,
	};

	let is_merkle_branch_valid = is_valid_merkle_branch(
		&finalized_checkpoint
			.hash_tree_root()
			.map_err(|_| Error::MerkleizationError("Failed to hash finality checkpoint".into()))?,
		update.finality_proof.finality_branch.iter(),
		C::FINALIZED_ROOT_INDEX_LOG2 as usize,
		C::FINALIZED_ROOT_INDEX as usize,
		&update.attested_header.state_root,
	);

	if !is_merkle_branch_valid {
		Err(Error::InvalidMerkleBranch("Finality branch".into()))?;
	}

	// verify the associated execution header of the finalized beacon header.
	let mut execution_payload = update.execution_payload;
	let execution_payload_root = calculate_multi_merkle_root(
		&[
			Node::from_bytes(execution_payload.state_root.as_ref().try_into().expect("Infallible")),
			execution_payload.block_number.hash_tree_root().map_err(|_| {
				Error::MerkleizationError("Failed to hash execution payload".into())
			})?,
			execution_payload
				.timestamp
				.hash_tree_root()
				.map_err(|_| Error::MerkleizationError("Failed to hash timestamp".into()))?,
		],
		&execution_payload.multi_proof,
		&[
			GeneralizedIndex(C::EXECUTION_PAYLOAD_STATE_ROOT_INDEX as usize),
			GeneralizedIndex(C::EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX as usize),
			GeneralizedIndex(C::EXECUTION_PAYLOAD_TIMESTAMP_INDEX as usize),
		],
	);

	let is_merkle_branch_valid = is_valid_merkle_branch(
		&execution_payload_root,
		execution_payload.execution_payload_branch.iter(),
		C::EXECUTION_PAYLOAD_INDEX_LOG2 as usize,
		C::EXECUTION_PAYLOAD_INDEX as usize,
		&update.finalized_header.state_root,
	);

	if !is_merkle_branch_valid {
		Err(Error::InvalidMerkleBranch("Execution payload branch".into()))?;
	}

	if let Some(mut sync_committee_update) = update.sync_committee_update.clone() {
		let sync_root = sync_committee_update
			.next_sync_committee
			.hash_tree_root()
			.map_err(|_| Error::MerkleizationError("Failed to hash next sync committee".into()))?;

		let is_merkle_branch_valid = is_valid_merkle_branch(
			&sync_root,
			sync_committee_update.next_sync_committee_branch.iter(),
			C::NEXT_SYNC_COMMITTEE_INDEX_LOG2 as usize,
			C::NEXT_SYNC_COMMITTEE_INDEX as usize,
			&update.attested_header.state_root,
		);

		if !is_merkle_branch_valid {
			Err(Error::InvalidMerkleBranch("Next sync committee branch".into()))?;
		}
	}

	let verifier_state = if should_have_sync_committee_update(state_period, update_signature_period)
	{
		if let Some(sync_committee_update) = update.sync_committee_update {
			VerifierState {
				finalized_header: update.finalized_header,
				latest_finalized_epoch: update.finality_proof.epoch,
				current_sync_committee: trusted_state.next_sync_committee,
				next_sync_committee: sync_committee_update.next_sync_committee,
				state_period: state_period + 1,
			}
		} else {
			Err(Error::InvalidUpdate("Expected sync committee update to be present".into()))?
		}
	} else {
		VerifierState {
			finalized_header: update.finalized_header,
			latest_finalized_epoch: update.finality_proof.epoch,
			..trusted_state
		}
	};

	Ok(verifier_state)
}
