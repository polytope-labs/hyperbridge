use crate::error::Error;
use alloc::vec::Vec;
use core::fmt::{Display, Formatter};
use ethereum_consensus::altair::mainnet::SYNC_COMMITTEE_SIZE;
use ethereum_consensus::bellatrix::compute_domain;
use ethereum_consensus::domains::DomainType;
use ethereum_consensus::primitives::Root;
use ethereum_consensus::signing::compute_signing_root;
use ethereum_consensus::state_transition::Context;
use light_client_primitives::util::{
    compute_epoch_at_slot, compute_fork_version, compute_sync_committee_period_at_slot,
    genesis_validator_root,
};
use ssz_rs::Node;

pub type LightClientState = light_client_primitives::types::LightClientState<SYNC_COMMITTEE_SIZE>;
pub type LightClientUpdate = light_client_primitives::types::LightClientUpdate<SYNC_COMMITTEE_SIZE>;

//TODO: we might change this
const DOMAIN_SYNC_COMMITTEE: DomainType = DomainType::SyncCommittee;

pub struct EthLightClient {}

impl EthLightClient {
    /// This function simply verifies a sync committee's attestation & it's finalized counterpart.
    pub fn verify_sync_committee_attestation(
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
            state.current_sync_committee
        } else {
            state.next_sync_committee
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

        Ok(())
    }
}
