use crate::{
	consensus_types::ForkData,
	constants::{Config, Domain, Root, Version},
	domains::DomainType,
};
use alloc::{vec, vec::Vec};
use anyhow::anyhow;
use ssz_rs::prelude::*;

/// Returns true if sync committee update is required
pub fn should_have_sync_committee_update(state_period: u64, signature_period: u64) -> bool {
	signature_period == state_period + 1
}

/// Return the sync committee period at the given ``epoch``
pub fn compute_sync_committee_period<C: Config>(epoch: u64) -> u64 {
	epoch / C::EPOCHS_PER_SYNC_COMMITTEE_PERIOD
}

/// Return the epoch number at ``slot``.
pub fn compute_epoch_at_slot<C: Config>(slot: u64) -> u64 {
	slot / C::SLOTS_PER_EPOCH
}

/// Return the fork version at the given ``epoch``.
pub fn compute_fork_version<C: Config>(epoch: u64) -> [u8; 4] {
    if epoch >= C::FULU_FORK_EPOCH {
		C::FULU_FORK_VERSION
	} else if epoch >= C::ELECTRA_FORK_EPOCH {
		C::ELECTRA_FORK_VERSION
	} else if epoch >= C::DENEB_FORK_EPOCH {
		C::DENEB_FORK_VERSION
	} else if epoch >= C::CAPELLA_FORK_EPOCH {
		C::CAPELLA_FORK_VERSION
	} else if epoch >= C::BELLATRIX_FORK_EPOCH {
		C::BELLATRIX_FORK_VERSION
	} else if epoch >= C::ALTAIR_FORK_EPOCH {
		C::ALTAIR_FORK_VERSION
	} else {
		C::GENESIS_FORK_VERSION
	}
}

pub fn compute_domain(
	domain_type: DomainType,
	fork_version: Option<Version>,
	genesis_validators_root: Option<Root>,
	genesis_fork_version: Version,
) -> Result<Domain, anyhow::Error> {
	let fork_version = fork_version.unwrap_or(genesis_fork_version);
	let genesis_validators_root = genesis_validators_root.unwrap_or_default();
	let fork_data_root = compute_fork_data_root(fork_version, genesis_validators_root)?;
	let mut domain = Domain::default();
	domain[..4].copy_from_slice(&domain_type.as_bytes());
	domain[4..].copy_from_slice(&fork_data_root.as_ref()[..28]);
	Ok(domain)
}

#[derive(Default, Debug, SimpleSerialize)]
pub struct SigningData {
	pub object_root: Root,
	pub domain: Domain,
}

pub fn compute_signing_root<T: SimpleSerialize>(
	ssz_object: &mut T,
	domain: Domain,
) -> Result<Root, anyhow::Error> {
	let object_root = ssz_object.hash_tree_root().map_err(|e| anyhow!("{:?}", e))?;

	let mut s = SigningData { object_root, domain };
	s.hash_tree_root().map_err(|e| anyhow!("{:?}", e))
}

pub fn compute_fork_data_root(
	current_version: Version,
	genesis_validators_root: Root,
) -> Result<Root, anyhow::Error> {
	ForkData { current_version, genesis_validators_root }
		.hash_tree_root()
		.map_err(|e| anyhow!("{:?}", e))
}

/// Return the sync committee period at ``slot``
pub fn compute_sync_committee_period_at_slot<C: Config>(slot: u64) -> u64 {
	compute_sync_committee_period::<C>(compute_epoch_at_slot::<C>(slot))
}
