use crate::error::Error;
use alloc::vec::Vec;
use base2::Base2;
use core::borrow::Borrow;
use core::fmt::{Display, Formatter};
use ethereum_consensus::altair::mainnet::SYNC_COMMITTEE_SIZE;
use ethereum_consensus::bellatrix::compute_domain;
use ethereum_consensus::domains::DomainType;
use ethereum_consensus::primitives::Root;
use ethereum_consensus::signing::compute_signing_root;
use ethereum_consensus::state_transition::Context;
use light_client_primitives::types::AncestryProof;
use light_client_primitives::util::{
    compute_epoch_at_slot, compute_fork_version, compute_sync_committee_period_at_slot,
    genesis_validator_root, get_subtree_index, hash_tree_root,
};
use ssz_rs::prelude::is_valid_merkle_branch;
use ssz_rs::Merkleized;
use ssz_rs::{calculate_merkle_root, calculate_multi_merkle_root, GeneralizedIndex, Node};

pub type LightClientState = light_client_primitives::types::LightClientState<SYNC_COMMITTEE_SIZE>;
pub type LightClientUpdate = light_client_primitives::types::LightClientUpdate<SYNC_COMMITTEE_SIZE>;

pub struct EthLightClient {}

impl EthLightClient {
    /// This function simply verifies a sync committee's attestation & it's finalized counterpart.
    pub fn verify_sync_committee_attestation<
        const DOMAIN_SYNC_COMMITTEE: DomainType,
        const FINALIZED_ROOT_INDEX: u64,
        const EXECUTION_PAYLOAD_STATE_ROOT_INDEX: GeneralizedIndex,
        const EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX: GeneralizedIndex,
        const EXECUTION_PAYLOAD_INDEX: u64,
        const NEXT_SYNC_COMMITTEE_INDEX: u64,
        const BLOCK_ROOTS_INDEX: u64,
        const HISTORICAL_BATCH_BLOCK_ROOTS_INDEX: GeneralizedIndex,
        const HISTORICAL_ROOTS_INDEX: u64,
    >(
        state: LightClientState,
        mut update: LightClientUpdate,
    ) -> Result<(), Error> {
        // Verify sync committee has super majority participants
        let sync_committee_bits = update.sync_aggregate.sync_committee_bits;
        let sync_aggregate_participants: u64 = sync_committee_bits.iter().count() as u64;
        if sync_aggregate_participants * 3 >= sync_committee_bits.clone().len() as u64 * 2 {
            Err(Error::SyncCommitteeParticiapntsTooLow)?
        }

        // Verify update does not skip a sync committee period
        let is_valid_update = update.signature_slot > update.attested_header.slot
            && update.attested_header.slot >= update.finalized_header.slot;
        if !is_valid_update {
            Err(Error::InvalidUpdate)?
        }

        let state_period = compute_sync_committee_period_at_slot(state.finalized_header.slot);
        let update_signature_period = compute_sync_committee_period_at_slot(update.signature_slot);
        if !(state_period..=state_period + 1).contains(&update_signature_period) {
            Err(Error::InvalidUpdate)?
        }

        // Verify update is relevant
        let update_attested_period =
            compute_sync_committee_period_at_slot(update.attested_header.slot);
        let update_has_next_sync_committee =
            update.sync_committee_update.is_some() && update_attested_period == state_period;

        if !(update.attested_header.slot > state.finalized_header.slot
            || update_has_next_sync_committee)
        {
            Err(Error::InvalidUpdate)?
        }

        // Verify sync committee aggregate signature
        let sync_committee = if update_signature_period == state_period {
            state.clone().current_sync_committee
        } else {
            state.clone().next_sync_committee
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
            Some(genesis_validator_root()),
            &Context::default(),
        );

        if domain.is_err() {
            Err(Error::InvalidUpdate)?
        }
        let signing_root = compute_signing_root(&mut update.attested_header, domain.unwrap());

        //TODO: not sure if we are to use update to get the signature
        ethereum_consensus::crypto::fast_aggregate_verify(
            &*participant_pubkeys,
            signing_root.unwrap().as_bytes(),
            &update.sync_aggregate.sync_committee_signature,
        )?;

        // Verify that the `finality_branch` confirms `finalized_header`
        // to match the finalized checkpoint root saved in the state of `attested_header`.
        // Note that the genesis finalized checkpoint root is represented as a zero hash.
        let finalized_root = &Node::from_bytes(
            light_client_primitives::util::hash_tree_root(update.finalized_header.clone())
                .unwrap()
                .as_ref()
                .try_into()
                .unwrap(),
        );

        let branch = update
            .finality_branch
            .iter()
            .map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
            .collect::<Vec<_>>();

        let is_merkle_branch_valid = is_valid_merkle_branch(
            finalized_root,
            branch.iter(),
            FINALIZED_ROOT_INDEX.floor_log2() as usize,
            get_subtree_index(FINALIZED_ROOT_INDEX) as usize,
            &Node::from_bytes(
                update
                    .attested_header
                    .state_root
                    .as_ref()
                    .try_into()
                    .unwrap(),
            ),
        );

        if is_merkle_branch_valid {
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
                Node::from_bytes(execution_payload.state_root.as_ref().try_into().unwrap()),
                execution_payload.block_number.hash_tree_root().unwrap(),
            ],
            &multi_proof_nodes,
            &[
                EXECUTION_PAYLOAD_STATE_ROOT_INDEX,
                EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX,
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
            get_subtree_index(EXECUTION_PAYLOAD_INDEX) as usize,
            &Node::from_bytes(
                update
                    .finalized_header
                    .clone()
                    .body_root
                    .as_ref()
                    .try_into()
                    .unwrap(),
            ),
        );

        if !is_merkle_branch_valid {
            Err(Error::InvalidMerkleBranch)?;
        }

        if let Some(sync_committee_update) = update.sync_committee_update {
            if update_attested_period == state_period {
                if sync_committee_update.next_sync_committee != state.clone().next_sync_committee {
                    Err(Error::InvalidUpdate)?
                }
            }

            let next_sync_committee_branch = sync_committee_update
                .next_sync_committee_branch
                .iter()
                .map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
                .collect::<Vec<_>>();
            let is_merkle_branch_valid = is_valid_merkle_branch(
                &Node::from_bytes(
                    light_client_primitives::util::hash_tree_root(
                        sync_committee_update.next_sync_committee,
                    )
                    .unwrap()
                    .as_ref()
                    .try_into()
                    .unwrap(),
                ),
                next_sync_committee_branch.iter(),
                NEXT_SYNC_COMMITTEE_INDEX.floor_log2() as usize,
                get_subtree_index(NEXT_SYNC_COMMITTEE_INDEX) as usize,
                &Node::from_bytes(
                    update
                        .attested_header
                        .state_root
                        .as_ref()
                        .try_into()
                        .unwrap(),
                ),
            );

            if !is_merkle_branch_valid {
                Err(Error::InvalidMerkleBranch)?;
            }
        }

        // verify the ancestry proofs
        for ancestor in update.ancestor_blocks {
            match ancestor.ancestry_proof {
                AncestryProof::BlockRoots {
                    block_roots_proof,
                    block_roots_branch,
                } => {
                    let block_header_branch = block_roots_proof
                        .block_header_branch
                        .iter()
                        .map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
                        .collect::<Vec<_>>();

                    let block_roots_root = calculate_merkle_root(
                        &Node::from_bytes(
                            hash_tree_root(ancestor.header.clone())
                                .unwrap()
                                .as_ref()
                                .try_into()
                                .unwrap(),
                        ),
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
                        get_subtree_index(BLOCK_ROOTS_INDEX) as usize,
                        &Node::from_bytes(
                            update
                                .finalized_header
                                .state_root
                                .as_ref()
                                .try_into()
                                .unwrap(),
                        ),
                    );
                    if !is_merkle_branch_valid {
                        Err(Error::InvalidMerkleBranch)?;
                    }
                }
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
                        &Node::from_bytes(
                            hash_tree_root(ancestor.header.clone())
                                .unwrap()
                                .as_ref()
                                .try_into()
                                .unwrap(),
                        ),
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
                        &HISTORICAL_BATCH_BLOCK_ROOTS_INDEX,
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
                                .unwrap(),
                        ),
                    );

                    if !is_merkle_branch_valid {
                        Err(Error::InvalidMerkleBranch)?;
                    }
                }
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
                    Node::from_bytes(execution_payload.state_root.as_ref().try_into().unwrap()),
                    Node::from_bytes(
                        hash_tree_root(execution_payload.block_number)
                            .unwrap()
                            .as_ref()
                            .try_into()
                            .unwrap(),
                    ),
                ],
                &multi_proof,
                &[
                    EXECUTION_PAYLOAD_STATE_ROOT_INDEX,
                    EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX,
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
                get_subtree_index(EXECUTION_PAYLOAD_INDEX) as usize,
                &Node::from_bytes(ancestor.header.body_root.as_ref().try_into().unwrap()),
            );

            if !is_merkle_branch_valid {
                Err(Error::InvalidMerkleBranch)?;
            }
        }
        Ok(())
    }
}
