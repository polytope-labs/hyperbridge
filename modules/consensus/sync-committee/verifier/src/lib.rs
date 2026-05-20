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
	GeneralizedIndex, Merkleized, Node, calculate_multi_merkle_root, get_helper_indices,
	prelude::is_valid_merkle_branch,
};
use sync_committee_primitives::{
	consensus_types::Checkpoint,
	constants::{Config, DOMAIN_SYNC_COMMITTEE, Root},
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
	// The finality branch is always required; validate it independently of the optional
	// sync-committee update. The previous combined `&&` chain only triggered when ALL three
	// subconditions held, so a malformed finality branch was accepted whenever the update
	// lacked a sync-committee section or carried a correctly-sized next-committee branch.
	if update.finality_proof.finality_branch.len() != C::FINALIZED_ROOT_INDEX_LOG2 as usize {
		Err(Error::InvalidUpdate("Finality branch is incorrect".into()))?
	}

	if let Some(sync_committee_update) = update.sync_committee_update.as_ref() {
		if sync_committee_update.next_sync_committee_branch.len() !=
			C::NEXT_SYNC_COMMITTEE_INDEX_LOG2 as usize
		{
			Err(Error::InvalidUpdate("Next sync committee branch is incorrect".into()))?
		}
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
	let sync_committee_bits = update.sync_aggregate.sync_committee_bits;

	// Verify sync committee has super majority participants. The bit
	// vector and the pubkey set should both be `SYNC_COMMITTEE_SIZE`,
	// but the threshold is computed against the actual pubkey set size
	// and any bit past it is treated as junk — otherwise an attacker
	// could pad `count_ones()` with positions that have no corresponding
	// validator and trivially clear the supermajority check.
	let committee_size = sync_committee_pubkeys.len();
	if sync_committee_bits
		.iter()
		.enumerate()
		.any(|(i, bit)| i >= committee_size && *bit)
	{
		Err(Error::InvalidUpdate("Sync committee bits set beyond committee size".into()))?
	}

	let sync_aggregate_participants: u64 =
		sync_committee_bits.iter().take(committee_size).filter(|b| **b).count() as u64;

	if sync_aggregate_participants < ((2 * committee_size as u64) / 3) + 1 {
		Err(Error::SyncCommitteeParticipantsTooLow)?
	}

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
	let execution_payload_indices = [
		GeneralizedIndex(C::EXECUTION_PAYLOAD_STATE_ROOT_INDEX as usize),
		GeneralizedIndex(C::EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX as usize),
		GeneralizedIndex(C::EXECUTION_PAYLOAD_TIMESTAMP_INDEX as usize),
	];
	// `calculate_multi_merkle_root` panics on a short `multi_proof` because its final
	// `objects.get(&GeneralizedIndex(1)).unwrap()` cannot reconstruct the root. Reject
	// proofs whose helper-node count does not match what the algorithm requires so an
	// attacker-controlled `multi_proof` cannot panic the runtime via the public unsigned
	// consensus update path.
	if execution_payload.multi_proof.len() != get_helper_indices(&execution_payload_indices).len()
	{
		Err(Error::InvalidMerkleBranch("Execution payload multiproof length".into()))?;
	}
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
		&execution_payload_indices,
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

#[cfg(test)]
mod supermajority_tests {
	use super::*;
	use sync_committee_primitives::{
		consensus_types::{BeaconBlockHeader, SyncAggregate, SyncCommittee},
		constants::{BLS_SIGNATURE_BYTES_LEN, BlsSignature, SYNC_COMMITTEE_SIZE, sepolia::Sepolia},
		types::{ExecutionPayloadProof, FinalityProof, VerifierState, VerifierStateUpdate},
	};

	/// Build a baseline update that passes every check that runs before
	/// the supermajority gate, but whose `sync_committee_bits` are still
	/// caller-controlled. The state lives entirely in period 0 so we
	/// don't have to satisfy the sync-committee-update branches.
	fn baseline() -> (VerifierState, VerifierStateUpdate) {
		// Sepolia has 8192 slots per sync committee period — keep all
		// slots well inside period 0.
		let finalized_slot = 10u64;
		let attested_slot = 20u64;
		let signature_slot = 30u64;

		let trusted_state = VerifierState {
			finalized_header: BeaconBlockHeader { slot: 0, ..Default::default() },
			latest_finalized_epoch: 0,
			current_sync_committee: SyncCommittee::<SYNC_COMMITTEE_SIZE>::default(),
			next_sync_committee: SyncCommittee::<SYNC_COMMITTEE_SIZE>::default(),
			state_period: 0,
		};

		let update = VerifierStateUpdate {
			attested_header: BeaconBlockHeader { slot: attested_slot, ..Default::default() },
			sync_committee_update: None,
			finalized_header: BeaconBlockHeader { slot: finalized_slot, ..Default::default() },
			execution_payload: ExecutionPayloadProof::default(),
			finality_proof: FinalityProof {
				epoch: 1,
				// Branch length must match `Sepolia::FINALIZED_ROOT_INDEX_LOG2` so the
				// finality-branch length check passes; the node contents don't matter for
				// these tests because the supermajority gate fires before any merkle
				// verification.
				finality_branch: vec![Node::default(); Sepolia::FINALIZED_ROOT_INDEX_LOG2 as usize],
			},
			sync_aggregate: SyncAggregate {
				sync_committee_bits: ssz_rs::Bitvector::default(),
				sync_committee_signature: BlsSignature::try_from(vec![
					0u8;
					BLS_SIGNATURE_BYTES_LEN
				])
				.unwrap(),
			},
			signature_slot,
		};

		(trusted_state, update)
	}

	/// 341 of 512 bits set is one short of the `(2*512/3)+1 = 342`
	/// threshold; the supermajority gate must reject.
	#[test]
	fn rejects_under_threshold_participants() {
		let (state, mut update) = baseline();
		for i in 0..341 {
			update.sync_aggregate.sync_committee_bits.set(i, true);
		}

		let err = verify_sync_committee_attestation::<Sepolia>(state, update)
			.expect_err("must reject under-threshold update");
		assert!(matches!(err, Error::SyncCommitteeParticipantsTooLow), "unexpected error: {err:?}");
	}

	/// 342 bits set hits the threshold exactly; the gate must let the
	/// update through. Verification fails downstream (no real BLS
	/// signature, no merkle proofs) — what matters is that the failure
	/// is not the supermajority check.
	#[test]
	fn accepts_exactly_threshold_participants() {
		let (state, mut update) = baseline();
		for i in 0..342 {
			update.sync_aggregate.sync_committee_bits.set(i, true);
		}

		let err = verify_sync_committee_attestation::<Sepolia>(state, update)
			.expect_err("downstream signature/merkle check must fail");
		assert!(
			!matches!(err, Error::SyncCommitteeParticipantsTooLow),
			"supermajority gate should not have rejected: {err:?}"
		);
	}
}
