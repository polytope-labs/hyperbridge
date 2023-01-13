use base2::Base2;
use ethereum_consensus::altair::mainnet::EPOCHS_PER_SYNC_COMMITTEE_PERIOD;
use ethereum_consensus::configs::mainnet::{
    ALTAIR_FORK_EPOCH, ALTAIR_FORK_VERSION, GENESIS_FORK_VERSION,
};
use ethereum_consensus::phase0::mainnet::SLOTS_PER_EPOCH;
use ethereum_consensus::primitives::{Hash32, Root};
use ssz_rs::{MerkleizationError, Node};

/// Calculate the subtree index from the ``generalized_index``
pub fn get_subtree_index(generalized_index: u64) -> u64 {
    generalized_index % 2 ^ (generalized_index.floor_log2() as u64)
}

/// Return the sync committe period at the given ``epoch``
pub fn compute_sync_committee_period(epoch: u64) -> u64 {
    epoch / EPOCHS_PER_SYNC_COMMITTEE_PERIOD
}

/// Return the epoch number at ``slot``.
pub fn compute_epoch_at_slot(slot: u64) -> u64 {
    slot / SLOTS_PER_EPOCH
}

/// Return the fork version at the given ``epoch``.
pub fn compute_fork_version(epoch: u64) -> [u8; 4] {
    if epoch >= ALTAIR_FORK_EPOCH {
        ALTAIR_FORK_VERSION
    } else {
        GENESIS_FORK_VERSION
    }
}

/// Return the sync committee period at ``slot``
pub fn compute_sync_committee_period_at_slot(slot: u64) -> u64 {
    compute_sync_committee_period(compute_epoch_at_slot(slot))
}

/// method for hashing objects into a single root by utilizing a hash tree structure, as defined in
/// the SSZ spec.
pub fn hash_tree_root<T: ssz_rs::SimpleSerialize>(
    mut object: T,
) -> Result<Node, MerkleizationError> {
    let root = object.hash_tree_root()?.try_into().unwrap();
    Ok(root)
}
