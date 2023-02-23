#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod error;

use crate::error::Error;
use alloc::vec::Vec;
use base2::Base2;
use core::{borrow::Borrow, fmt::Display};
use ethereum_consensus::{
	bellatrix::{compute_domain, mainnet::SYNC_COMMITTEE_SIZE, Checkpoint},
	primitives::Root,
	signing::compute_signing_root,
	state_transition::Context,
};
use ssz_rs::{
	calculate_merkle_root, calculate_multi_merkle_root, prelude::is_valid_merkle_branch,
	GeneralizedIndex, Merkleized, Node,
};
use sync_committee_primitives::{
	types::{
		AncestryProof, BLOCK_ROOTS_INDEX, DOMAIN_SYNC_COMMITTEE,
		EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX, EXECUTION_PAYLOAD_INDEX,
		EXECUTION_PAYLOAD_STATE_ROOT_INDEX, FINALIZED_ROOT_INDEX, GENESIS_VALIDATORS_ROOT,
		HISTORICAL_BATCH_BLOCK_ROOTS_INDEX, HISTORICAL_ROOTS_INDEX, NEXT_SYNC_COMMITTEE_INDEX,
	},
	util::{
		compute_epoch_at_slot, compute_fork_version, compute_sync_committee_period_at_slot,
		get_subtree_index, hash_tree_root,
	},
};

pub type LightClientState = sync_committee_primitives::types::LightClientState<SYNC_COMMITTEE_SIZE>;
pub type LightClientUpdate =
	sync_committee_primitives::types::LightClientUpdate<SYNC_COMMITTEE_SIZE>;

/// This function simply verifies a sync committee's attestation & it's finalized counterpart.
pub fn verify_sync_committee_attestation(
	trusted_state: LightClientState,
	update: LightClientUpdate,
) -> Result<LightClientState, Error> {
	if update.finality_proof.finality_branch.len() != FINALIZED_ROOT_INDEX.floor_log2() as usize &&
		update.sync_committee_update.is_some() &&
		update.sync_committee_update.as_ref().unwrap().next_sync_committee_branch.len() !=
			NEXT_SYNC_COMMITTEE_INDEX.floor_log2() as usize
	{
		log::debug!("Invalid update ");
		log::debug!(
			"update finality branch length {} ",
			update.finality_proof.finality_branch.len()
		);
		log::debug!(
			"update next sync committee branch length {} ",
			update.sync_committee_update.as_ref().unwrap().next_sync_committee_branch.len()
		);

		Err(Error::InvalidUpdate)?
	}

	// Verify sync committee has super majority participants
	let sync_committee_bits = update.sync_aggregate.sync_committee_bits;
	let sync_aggregate_participants: u64 =
		sync_committee_bits.iter().as_bitslice().count_ones() as u64;

	if sync_aggregate_participants * 3 >= sync_committee_bits.clone().len() as u64 * 2 {
		log::debug!("SyncCommitteeParticipantsTooLow ");
		log::debug!("sync_aggregate_participants {} ", { sync_aggregate_participants * 3 });
		log::debug!("sync_committee_bits {}", { sync_committee_bits.clone().len() * 2 });
		Err(Error::SyncCommitteeParticipantsTooLow)?
	}

	// Verify update does not skip a sync committee period
	let is_valid_update = update.signature_slot > update.attested_header.slot &&
		update.attested_header.slot >= update.finalized_header.slot;
	if !is_valid_update {
		log::debug!("is_valid_update {} ", is_valid_update);
		log::debug!(
			"update.signature_slot {} update.attested_header.slot {}",
			update.signature_slot,
			update.attested_header.slot
		);
		log::debug!(
			"update.attested_header.slot {} update.finalized_header.slot {}",
			update.attested_header.slot,
			update.finalized_header.slot
		);
		Err(Error::InvalidUpdate)?
	}

	let state_period = compute_sync_committee_period_at_slot(trusted_state.finalized_header.slot);
	let update_signature_period = compute_sync_committee_period_at_slot(update.signature_slot);
	if !(state_period..=state_period + 1).contains(&update_signature_period) {
		log::debug!("invalid update");
		log::debug!("state_period is {}", state_period);
		log::debug!("update_signature_period is {}", update_signature_period);
		Err(Error::InvalidUpdate)?
	}

	// Verify update is relevant
	let update_attested_period = compute_sync_committee_period_at_slot(update.attested_header.slot);
	let update_has_next_sync_committee =
		update.sync_committee_update.is_some() && update_attested_period == state_period;

	if !(update.attested_header.slot > trusted_state.finalized_header.slot ||
		update_has_next_sync_committee)
	{
		Err(Error::InvalidUpdate)?
	}

	// Verify sync committee aggregate signature
	let sync_committee = if update_signature_period == state_period {
		trusted_state.current_sync_committee.clone()
	} else {
		trusted_state.next_sync_committee.clone()
	};

	let sync_committee_pubkeys = sync_committee.public_keys;

	let participant_pubkeys = sync_committee_bits
		.iter()
		.zip(sync_committee_pubkeys.iter())
		.filter_map(|(bit, key)| if *bit { Some(key) } else { None })
		.collect::<Vec<_>>();

	let fork_version = compute_fork_version(compute_epoch_at_slot(update.signature_slot));
	//TODO: we probably need to construct context
	let domain = compute_domain(
		DOMAIN_SYNC_COMMITTEE,
		Some(fork_version),
		Some(Root::from_bytes(GENESIS_VALIDATORS_ROOT.try_into().map_err(|_| Error::InvalidRoot)?)),
		&Context::default(),
	)
	.map_err(|_| Error::InvalidUpdate)?;

	let signing_root = compute_signing_root(&mut update.attested_header.clone(), domain);

	// todo: should be generic
	ethereum_consensus::crypto::fast_aggregate_verify(
		&*participant_pubkeys,
		signing_root.map_err(|_| Error::InvalidRoot)?.as_bytes(),
		&update.sync_aggregate.sync_committee_signature,
	)?;

	// Verify that the `finality_branch` confirms `finalized_header`
	// to match the finalized checkpoint root saved in the state of `attested_header`.
	// Note that the genesis finalized checkpoint root is represented as a zero hash.
	let mut finalized_checkpoint = Checkpoint {
		epoch: update.finality_proof.finalized_epoch,
		root: update
			.finalized_header
			.clone()
			.hash_tree_root()
			.map_err(|_| Error::InvalidRoot)?,
	};

	let branch = update
		.finality_proof
		.finality_branch
		.iter()
		.map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
		.collect::<Vec<_>>();

	let is_merkle_branch_valid = is_valid_merkle_branch(
		&finalized_checkpoint.hash_tree_root().map_err(|_| Error::InvalidRoot)?,
		branch.iter(),
		FINALIZED_ROOT_INDEX.floor_log2() as usize,
		FINALIZED_ROOT_INDEX as usize,
		&update.attested_header.state_root,
	);

	log::debug!("valid merkle branch for  finalized_root {}", is_merkle_branch_valid);
	if !is_merkle_branch_valid {
		log::debug!("invalid merkle branch for finalized root");
		Err(Error::InvalidMerkleBranch)?;
	}

	// verify the associated execution header of the finalized beacon header.
	let mut execution_payload = update.execution_payload;
	let multi_proof_vec = execution_payload.multi_proof;
	let multi_proof_nodes = multi_proof_vec
		.iter()
		.map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
		.collect::<Vec<_>>();
	let execution_payload_root = calculate_multi_merkle_root(
		&[
			Node::from_bytes(
				execution_payload
					.state_root
					.as_ref()
					.try_into()
					.map_err(|_| Error::InvalidRoot)?,
			),
			execution_payload
				.block_number
				.hash_tree_root()
				.map_err(|_| Error::InvalidRoot)?,
		],
		&multi_proof_nodes,
		&[
			GeneralizedIndex(EXECUTION_PAYLOAD_STATE_ROOT_INDEX as usize),
			GeneralizedIndex(EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX as usize),
		],
	);

	let execution_payload_branch = execution_payload
		.execution_payload_branch
		.iter()
		.map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
		.collect::<Vec<_>>();

	let is_merkle_branch_valid = is_valid_merkle_branch(
		&execution_payload_root,
		execution_payload_branch.iter(),
		EXECUTION_PAYLOAD_INDEX.floor_log2() as usize,
		EXECUTION_PAYLOAD_INDEX as usize,
		&update.finalized_header.state_root,
	);

	log::debug!("valid merkle branch for execution_payload_branch");
	if !is_merkle_branch_valid {
		log::debug!("invalid merkle branch for execution_payload_branch");
		Err(Error::InvalidMerkleBranch)?;
	}

	if let Some(mut sync_committee_update) = update.sync_committee_update.clone() {
		if update_attested_period == state_period &&
			sync_committee_update.next_sync_committee !=
				trusted_state.next_sync_committee.clone()
		{
			log::debug!("invalid update for sync committee update");
			Err(Error::InvalidUpdate)?
		}

		let next_sync_committee_branch = sync_committee_update
			.next_sync_committee_branch
			.iter()
			.map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
			.collect::<Vec<_>>();
		let is_merkle_branch_valid = is_valid_merkle_branch(
			&sync_committee_update
				.next_sync_committee
				.hash_tree_root()
				.map_err(|_| Error::MerkleizationError)?,
			next_sync_committee_branch.iter(),
			NEXT_SYNC_COMMITTEE_INDEX.floor_log2() as usize,
			get_subtree_index(NEXT_SYNC_COMMITTEE_INDEX) as usize,
			&update.attested_header.state_root,
		);

		log::debug!("valid merkle branch for  sync committee {}", is_merkle_branch_valid);
		if !is_merkle_branch_valid {
			log::debug!("invalid merkle branch for sync committee");
			Err(Error::InvalidMerkleBranch)?;
		}
	}

	// verify the ancestry proofs
	for mut ancestor in update.ancestor_blocks {
		match ancestor.ancestry_proof {
			AncestryProof::BlockRoots { block_roots_proof, block_roots_branch } => {
				let block_header_branch = block_roots_proof
					.block_header_branch
					.iter()
					.map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
					.collect::<Vec<_>>();

				let block_roots_root = calculate_merkle_root(
					&ancestor.header.hash_tree_root().map_err(|_| Error::MerkleizationError)?,
					&*block_header_branch,
					&GeneralizedIndex(block_roots_proof.block_header_index as usize),
				);

				let block_roots_branch_node = block_roots_branch
					.iter()
					.map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
					.collect::<Vec<_>>();

				let is_merkle_branch_valid = is_valid_merkle_branch(
					&block_roots_root,
					block_roots_branch_node.iter(),
					BLOCK_ROOTS_INDEX.floor_log2() as usize,
					BLOCK_ROOTS_INDEX as usize,
					&update.finalized_header.state_root,
				);
				if !is_merkle_branch_valid {
					Err(Error::InvalidMerkleBranch)?;
				}
			},
			AncestryProof::HistoricalRoots {
				block_roots_proof,
				historical_batch_proof,
				historical_roots_proof,
				historical_roots_index,
				historical_roots_branch,
			} => {
				let block_header_branch = block_roots_proof
					.block_header_branch
					.iter()
					.map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
					.collect::<Vec<_>>();
				let block_roots_root = calculate_merkle_root(
					&ancestor
						.header
						.clone()
						.hash_tree_root()
						.map_err(|_| Error::MerkleizationError)?,
					&block_header_branch,
					&GeneralizedIndex(block_roots_proof.block_header_index as usize),
				);

				let historical_batch_proof_nodes = historical_batch_proof
					.iter()
					.map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
					.collect::<Vec<_>>();
				let historical_batch_root = calculate_merkle_root(
					&block_roots_root,
					&historical_batch_proof_nodes,
					&GeneralizedIndex(HISTORICAL_BATCH_BLOCK_ROOTS_INDEX as usize),
				);

				let historical_roots_proof_nodes = historical_roots_proof
					.iter()
					.map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
					.collect::<Vec<_>>();
				let historical_roots_root = calculate_merkle_root(
					&historical_batch_root,
					&historical_roots_proof_nodes,
					&GeneralizedIndex(historical_roots_index as usize),
				);

				let historical_roots_branch_nodes = historical_roots_branch
					.iter()
					.map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
					.collect::<Vec<_>>();
				let is_merkle_branch_valid = is_valid_merkle_branch(
					&historical_roots_root,
					historical_roots_branch_nodes.iter(),
					HISTORICAL_ROOTS_INDEX.floor_log2() as usize,
					get_subtree_index(HISTORICAL_ROOTS_INDEX) as usize,
					&Node::from_bytes(
						update
							.finalized_header
							.state_root
							.as_ref()
							.try_into()
							.map_err(|_| Error::InvalidRoot)?,
					),
				);

				if !is_merkle_branch_valid {
					Err(Error::InvalidMerkleBranch)?;
				}
			},
		};

		// verify the associated execution paylaod header.
		let execution_payload = ancestor.execution_payload;
		let multi_proof = execution_payload
			.multi_proof
			.iter()
			.map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
			.collect::<Vec<_>>();
		let execution_payload_root = calculate_multi_merkle_root(
			&[
				Node::from_bytes(
					execution_payload
						.state_root
						.as_ref()
						.try_into()
						.map_err(|_| Error::InvalidRoot)?,
				),
				Node::from_bytes(
					hash_tree_root(execution_payload.block_number)
						.map_err(|_| Error::MerkleizationError)?
						.as_ref()
						.try_into()
						.map_err(|_| Error::InvalidRoot)?,
				),
			],
			&multi_proof,
			&[
				GeneralizedIndex(EXECUTION_PAYLOAD_STATE_ROOT_INDEX as usize),
				GeneralizedIndex(EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX as usize),
			],
		);

		let execution_payload_branch = execution_payload
			.execution_payload_branch
			.iter()
			.map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
			.collect::<Vec<_>>();
		let is_merkle_branch_valid = is_valid_merkle_branch(
			&execution_payload_root,
			execution_payload_branch.iter(),
			EXECUTION_PAYLOAD_INDEX.floor_log2() as usize,
			EXECUTION_PAYLOAD_INDEX as usize,
			&Node::from_bytes(
				ancestor.header.state_root.as_ref().try_into().map_err(|_| Error::InvalidRoot)?,
			),
		);

		if !is_merkle_branch_valid {
			Err(Error::InvalidMerkleBranch)?;
		}
	}

	let new_light_client_state = if let Some(sync_committee_update) = update.sync_committee_update {
		LightClientState {
			finalized_header: update.finalized_header,
			current_sync_committee: trusted_state.next_sync_committee,
			next_sync_committee: sync_committee_update.next_sync_committee,
		}
	} else {
		LightClientState { finalized_header: update.finalized_header, ..trusted_state }
	};

	Ok(new_light_client_state)
}
