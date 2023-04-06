use ethereum_consensus::{
	altair::mainnet::EPOCHS_PER_SYNC_COMMITTEE_PERIOD,
	configs::mainnet::{
		ALTAIR_FORK_EPOCH, ALTAIR_FORK_VERSION, BELLATRIX_FORK_EPOCH, BELLATRIX_FORK_VERSION,
		GENESIS_FORK_VERSION,
	},
	phase0::mainnet::SLOTS_PER_EPOCH,
};

/// Return the sync committe period at the given ``epoch``
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
	if epoch >= BELLATRIX_FORK_EPOCH {
		BELLATRIX_FORK_VERSION
	} else if epoch >= ALTAIR_FORK_EPOCH {
		ALTAIR_FORK_VERSION
	} else {
		GENESIS_FORK_VERSION
	}
}

#[cfg(feature = "testing")]
pub fn compute_fork_version(_epoch: u64) -> [u8; 4] {
	BELLATRIX_FORK_VERSION
}

/// Return the sync committee period at ``slot``
pub fn compute_sync_committee_period_at_slot(slot: u64) -> u64 {
	compute_sync_committee_period(compute_epoch_at_slot(slot))
}
