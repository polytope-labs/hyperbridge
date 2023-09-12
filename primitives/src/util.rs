use crate::{
	consensus_types::ForkData,
	constants::{
		Domain, Root, Slot, Version, ALTAIR_FORK_EPOCH, ALTAIR_FORK_VERSION, BELLATRIX_FORK_EPOCH,
		BELLATRIX_FORK_VERSION, CAPELLA_FORK_EPOCH, CAPELLA_FORK_VERSION,
		EPOCHS_PER_SYNC_COMMITTEE_PERIOD, GENESIS_FORK_VERSION, SLOTS_PER_EPOCH,
	},
	domains::DomainType,
};
use alloc::{vec, vec::Vec};
use anyhow::anyhow;
use ssz_rs::prelude::*;

/// Returns true if the next epoch is the start of a new sync committee period
pub fn should_get_sync_committee_update(slot: Slot) -> bool {
	let next_epoch = compute_epoch_at_slot(slot) + 1;
	next_epoch % EPOCHS_PER_SYNC_COMMITTEE_PERIOD == 0
}

/// Return the sync committee period at the given ``epoch``
pub fn compute_sync_committee_period(epoch: u64) -> u64 {
	epoch / EPOCHS_PER_SYNC_COMMITTEE_PERIOD
}

/// Return the epoch number at ``slot``.
pub fn compute_epoch_at_slot(slot: u64) -> u64 {
	slot / SLOTS_PER_EPOCH
}

#[cfg(not(feature = "testing"))]
/// Return the fork version at the given ``epoch``.
pub fn compute_fork_version(epoch: u64) -> [u8; 4] {
	if epoch >= CAPELLA_FORK_EPOCH {
		CAPELLA_FORK_VERSION
	} else if epoch >= BELLATRIX_FORK_EPOCH {
		BELLATRIX_FORK_VERSION
	} else if epoch >= ALTAIR_FORK_EPOCH {
		ALTAIR_FORK_VERSION
	} else {
		GENESIS_FORK_VERSION
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
pub fn compute_sync_committee_period_at_slot(slot: u64) -> u64 {
	compute_sync_committee_period(compute_epoch_at_slot(slot))
}
