use crate::{
	consensus_types::{BeaconBlockHeader, SyncAggregate, SyncCommittee},
	constants::{Slot, SYNC_COMMITTEE_SIZE},
};
use alloc::vec::Vec;
use primitive_types::H256;
use ssz_rs::Node;

/// This holds the relevant data required to prove the state root in the execution payload.
#[derive(Debug, Clone, PartialEq, Eq, Default, codec::Encode, codec::Decode)]
pub struct ExecutionPayloadProof {
	/// The state root in the `ExecutionPayload` which represents the commitment to
	/// the ethereum world state in the yellow paper.
	pub state_root: H256,
	/// the block number of the execution header.
	pub block_number: u64,
	/// merkle mutli proof for the state_root & block_number in the [`ExecutionPayload`].
	pub multi_proof: Vec<Node>,
	/// merkle proof for the `ExecutionPayload` in the [`BeaconBlockBody`].
	pub execution_payload_branch: Vec<Node>,
	/// timestamp
	pub timestamp: u64,
}

/// Holds the neccessary proofs required to verify a header in the `block_roots` field
/// either in [`BeaconState`] or [`HistoricalBatch`].
#[derive(Debug, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
pub struct BlockRootsProof {
	/// Generalized index of the header in the `block_roots` list.
	pub block_header_index: u64,
	/// The proof for the header, needed to reconstruct `hash_tree_root(state.block_roots)`
	pub block_header_branch: Vec<Node>,
}

/// The block header ancestry proof, this is an enum because the header may either exist in
/// `state.block_roots` or `state.historical_roots`.
#[derive(Debug, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
pub enum AncestryProof {
	/// This variant defines the proof data for a beacon chain header in the `state.block_roots`
	BlockRoots {
		/// Proof for the header in `state.block_roots`
		block_roots_proof: BlockRootsProof,
		/// The proof for the reconstructed `hash_tree_root(state.block_roots)` in [`BeaconState`]
		block_roots_branch: Vec<Node>,
	},
	/// This variant defines the neccessary proofs for a beacon chain header in the
	/// `state.historical_roots`.
	HistoricalRoots {
		/// Proof for the header in `historical_batch.block_roots`
		block_roots_proof: BlockRootsProof,
		/// The proof for the `historical_batch.block_roots`, needed to reconstruct
		/// `hash_tree_root(historical_batch)`
		historical_batch_proof: Vec<Node>,
		/// The proof for the `hash_tree_root(historical_batch)` in `state.historical_roots`
		historical_roots_proof: Vec<Node>,
		/// The generalized index for the historical_batch in `state.historical_roots`.
		historical_roots_index: u64,
		/// The proof for the reconstructed `hash_tree_root(state.historical_roots)` in
		/// [`BeaconState`]
		historical_roots_branch: Vec<Node>,
	},
}

/// This defines the neccesary data needed to prove ancestor blocks, relative to the finalized
/// header.
#[derive(Debug, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
pub struct AncestorBlock {
	/// The actual beacon chain header
	pub header: BeaconBlockHeader,
	/// Associated execution header proofs
	pub execution_payload: ExecutionPayloadProof,
	/// Ancestry proofs of the beacon chain header.
	pub ancestry_proof: AncestryProof,
}

/// Holds the latest sync committee as well as an ssz proof for it's existence
/// in an attested header.
#[derive(Debug, Clone, PartialEq, Eq, Default, codec::Encode, codec::Decode)]
pub struct SyncCommitteeUpdate {
	/// actual sync committee
	pub next_sync_committee: SyncCommittee<SYNC_COMMITTEE_SIZE>,
	/// next sync committee, ssz merkle proof.
	pub next_sync_committee_branch: Vec<Node>,
}

/// Minimum state required by the light client to validate new sync committee attestations
#[derive(Debug, Clone, PartialEq, Eq, Default, codec::Encode, codec::Decode)]
pub struct VerifierState {
	/// The latest recorded finalized header
	pub finalized_header: BeaconBlockHeader,
	/// Latest finalized epoch
	pub latest_finalized_epoch: u64,
	/// Sync committees corresponding to the finalized header
	pub current_sync_committee: SyncCommittee<SYNC_COMMITTEE_SIZE>,
	/// Committee for the next sync period
	pub next_sync_committee: SyncCommittee<SYNC_COMMITTEE_SIZE>,
	/// state_period
	pub state_period: u64,
}

/// Finalized header proof
#[derive(Debug, Clone, PartialEq, Eq, Default, codec::Encode, codec::Decode)]
pub struct FinalityProof {
	/// The latest  finalized epoch
	pub epoch: u64,
	/// Finalized header proof
	pub finality_branch: Vec<Node>,
}

/// Data required to advance the state of the light client.
#[derive(Debug, Clone, PartialEq, Eq, Default, codec::Encode, codec::Decode)]
pub struct VerifierStateUpdate {
	/// the header that the sync committee signed
	pub attested_header: BeaconBlockHeader,
	/// the sync committee has potentially changed, here's an ssz proof for that.
	pub sync_committee_update: Option<SyncCommitteeUpdate>,
	/// the actual header which was finalized by the ethereum attestation protocol.
	pub finalized_header: BeaconBlockHeader,
	/// execution payload of the finalized header
	pub execution_payload: ExecutionPayloadProof,
	/// Finalized header proof
	pub finality_proof: FinalityProof,
	/// signature & participation bits
	pub sync_aggregate: SyncAggregate<SYNC_COMMITTEE_SIZE>,
	/// slot at which signature was produced
	pub signature_slot: Slot,
}
