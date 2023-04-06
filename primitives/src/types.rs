use alloc::vec::Vec;
use ethereum_consensus::{
	bellatrix::{BeaconBlockHeader, SyncAggregate, SyncCommittee},
	domains::DomainType,
	primitives::{Hash32, Slot},
};

pub const DOMAIN_SYNC_COMMITTEE: DomainType = DomainType::SyncCommittee;
pub const FINALIZED_ROOT_INDEX: u64 = 52;
pub const EXECUTION_PAYLOAD_STATE_ROOT_INDEX: u64 = 18;
pub const EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX: u64 = 22;
pub const EXECUTION_PAYLOAD_INDEX: u64 = 56;
pub const NEXT_SYNC_COMMITTEE_INDEX: u64 = 55;
pub const BLOCK_ROOTS_INDEX: u64 = 37;
pub const HISTORICAL_ROOTS_INDEX: u64 = 39;
pub const HISTORICAL_BATCH_BLOCK_ROOTS_INDEX: u64 = 2;
pub const EXECUTION_PAYLOAD_TIMESTAMP_INDEX: u64 = 25;
pub const FINALIZED_ROOT_INDEX_LOG2: u64 = 5;
pub const EXECUTION_PAYLOAD_INDEX_LOG2: u64 = 5;
pub const NEXT_SYNC_COMMITTEE_INDEX_LOG2: u64 = 5;
pub const BLOCK_ROOTS_INDEX_LOG2: u64 = 5;
pub const HISTORICAL_ROOTS_INDEX_LOG2: u64 = 5;

#[cfg(not(feature = "testing"))]
pub const GENESIS_VALIDATORS_ROOT: [u8; 32] =
	hex_literal::hex!("4b363db94e286120d76eb905340fdd4e54bfe9f06bf33ff6cf5ad27f511bfe95");
#[cfg(feature = "testing")]
pub const GENESIS_VALIDATORS_ROOT: [u8; 32] =
	hex_literal::hex!("6034f557b4560fc549ac0e2c63269deb07bfac7bf2bbd0b8b7d4d321240bffd9");

/// This holds the relevant data required to prove the state root in the execution payload.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExecutionPayloadProof {
	/// The state root in the `ExecutionPayload` which represents the commitment to
	/// the ethereum world state in the yellow paper.
	pub state_root: Hash32,
	/// the block number of the execution header.
	pub block_number: u64,
	/// merkle mutli proof for the state_root & block_number in the [`ExecutionPayload`].
	pub multi_proof: Vec<Hash32>,
	/// merkle proof for the `ExecutionPayload` in the [`BeaconBlockBody`].
	pub execution_payload_branch: Vec<Hash32>,
	/// timestamp
	pub timestamp: u64,
}

/// Holds the neccessary proofs required to verify a header in the `block_roots` field
/// either in [`BeaconState`] or [`HistoricalBatch`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockRootsProof {
	/// Generalized index of the header in the `block_roots` list.
	pub block_header_index: u64,
	/// The proof for the header, needed to reconstruct `hash_tree_root(state.block_roots)`
	pub block_header_branch: Vec<Hash32>,
}

/// The block header ancestry proof, this is an enum because the header may either exist in
/// `state.block_roots` or `state.historical_roots`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AncestryProof {
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AncestorBlock {
	/// The actual beacon chain header
	pub header: BeaconBlockHeader,
	/// Associated execution header proofs
	pub execution_payload: ExecutionPayloadProof,
	/// Ancestry proofs of the beacon chain header.
	pub ancestry_proof: AncestryProof,
}

/// Holds the latest sync committee as well as an ssz proof for it's existence
/// in a finalized header.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SyncCommitteeUpdate<const SYNC_COMMITTEE_SIZE: usize> {
	// actual sync committee
	pub next_sync_committee: SyncCommittee<SYNC_COMMITTEE_SIZE>,
	// sync committee, ssz merkle proof.
	pub next_sync_committee_branch: Vec<Hash32>,
}

/// Minimum state required by the light client to validate new sync committee attestations
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LightClientState<const SYNC_COMMITTEE_SIZE: usize> {
	/// The latest recorded finalized header
	pub finalized_header: BeaconBlockHeader,
	/// Latest finalized epoch
	pub latest_finalized_epoch: u64,
	// Sync committees corresponding to the finalized header
	pub current_sync_committee: SyncCommittee<SYNC_COMMITTEE_SIZE>,
	pub next_sync_committee: SyncCommittee<SYNC_COMMITTEE_SIZE>,
}

/// Finalized header proof
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FinalityProof {
	/// The latest  finalized epoch
	pub epoch: u64,
	/// Finalized header proof
	pub finality_branch: Vec<Hash32>,
}

/// Data required to advance the state of the light client.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LightClientUpdate<const SYNC_COMMITTEE_SIZE: usize> {
	/// the header that the sync committee signed
	pub attested_header: BeaconBlockHeader,
	/// the sync committee has potentially changed, here's an ssz proof for that.
	pub sync_committee_update: Option<SyncCommitteeUpdate<SYNC_COMMITTEE_SIZE>>,
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
	/// ancestors of the finalized block to be verified, may be empty.
	pub ancestor_blocks: Vec<AncestorBlock>,
}
