use ethereum_consensus::primitives::Hash32;
use alloc::vec::Vec;
use ethereum_consensus::altair::{BeaconBlockHeader, SyncCommittee};

/// This holds the relevant data required to prove the state root in the execution payload.
struct ExecutionPayloadProof {
    /// The state root in the `ExecutionPayload` which represents the commitment to
    /// the ethereum world state in the yellow paper.
    state_root: Hash32,
    /// the block number of the execution header.
    block_number: u64,
    /// merkle mutli proof for the state_root & block_number in the [`ExecutionPayload`].
    multi_proof: Vec<Hash32>,
    /// merkle proof for the `ExecutionPayload` in the [`BeaconBlockBody`].
    execution_payload_branch: Vec<Hash32>,
}


/// Holds the neccessary proofs required to verify a header in the `block_roots` field
/// either in [`BeaconState`] or [`HistoricalBatch`].
struct BlockRootsProof {
    /// Generalized index of the header in the `block_roots` list.
    block_header_index: u64,
    /// The proof for the header, needed to reconstruct `hash_tree_root(state.block_roots)`
    block_header_branch: Vec<Hash32>,
}

/// The block header ancestry proof, this is an enum because the header may either exist in
/// `state.block_roots` or `state.historical_roots`.
enum AncestryProof {
    /// This variant defines the proof data for a beacon chain header in the `state.block_roots`
    BlockRoots {
        /// Proof for the header in `state.block_roots`
        block_roots_proof: BlockRootsProof,
        /// The proof for the reconstructed `hash_tree_root(state.block_roots)` in [`BeaconState`]
        block_roots_branch: Vec<Hash32>,
    },
    /// This variant defines the neccessary proofs for a beacon chain header in the
    /// `state.historical_roots`.
    HistoricalRoots {
        /// Proof for the header in `historical_batch.block_roots`
        block_roots_proof: BlockRootsProof,
        /// The proof for the `historical_batch.block_roots`, needed to reconstruct
        /// `hash_tree_root(historical_batch)`
        historical_batch_proof: Vec<Hash32>,
        /// The proof for the `hash_tree_root(historical_batch)` in `state.historical_roots`
        historical_roots_proof: Vec<Hash32>,
        /// The generalized index for the historical_batch in `state.historical_roots`.
        historical_roots_index: u64,
        /// The proof for the reconstructed `hash_tree_root(state.historical_roots)` in
        /// [`BeaconState`]
        historical_roots_branch: Vec<Hash32>,
    },
}

/// This defines the neccesary data needed to prove ancestor blocks, relative to the finalized
/// header.
struct AncestorBlock {
    /// The actual beacon chain header
    header: BeaconBlockHeader,
    /// Associated execution header proofs
    execution_payload: ExecutionPayloadProof,
    /// Ancestry proofs of the beacon chain header.
    ancestry_proof: AncestryProof,
}
