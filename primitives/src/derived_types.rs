use crate::{
	error::Error,
	helpers::{
		to_codec_light_client_state, to_codec_light_client_update, to_no_codec_beacon_header,
		to_no_codec_light_client_state, to_no_codec_light_client_update,
		to_no_codec_sync_committee,
	},
	types,
};
use alloc::vec::Vec;
use codec::{Decode, Encode};
use ethereum_consensus::{bellatrix, primitives::Hash32};

/// Minimum state required by the light client to validate new sync committee attestations
#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq, Default)]
pub struct LightClientState {
	/// The latest recorded finalized header
	pub finalized_header: BeaconBlockHeader,
	/// Latest finalized epoch
	pub latest_finalized_epoch: u64,
	// Sync committees corresponding to the finalized header
	pub current_sync_committee: SyncCommittee,
	pub next_sync_committee: SyncCommittee,
}

impl<const SYNC_COMMITTEE_SIZE: usize> TryFrom<types::LightClientState<SYNC_COMMITTEE_SIZE>>
	for LightClientState
{
	type Error = Error;
	fn try_from(state: types::LightClientState<SYNC_COMMITTEE_SIZE>) -> Result<Self, Self::Error> {
		to_codec_light_client_state(state)
	}
}

impl<const SYNC_COMMITTEE_SIZE: usize> TryFrom<LightClientState>
	for types::LightClientState<SYNC_COMMITTEE_SIZE>
{
	type Error = Error;
	fn try_from(state: LightClientState) -> Result<Self, Self::Error> {
		to_no_codec_light_client_state(state)
	}
}

#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq, Default)]
pub struct BeaconBlockHeader {
	pub slot: u64,
	pub proposer_index: u64,
	pub parent_root: [u8; 32],
	pub state_root: [u8; 32],
	pub body_root: [u8; 32],
}

impl TryFrom<bellatrix::BeaconBlockHeader> for BeaconBlockHeader {
	type Error = Error;

	fn try_from(beacon_block_header: bellatrix::BeaconBlockHeader) -> Result<Self, Self::Error> {
		Ok(BeaconBlockHeader {
			slot: beacon_block_header.slot,
			proposer_index: beacon_block_header.proposer_index as u64,
			parent_root: beacon_block_header
				.parent_root
				.as_bytes()
				.try_into()
				.map_err(|_| Error::InvalidNodeBytes)?,
			state_root: beacon_block_header
				.state_root
				.as_bytes()
				.try_into()
				.map_err(|_| Error::InvalidNodeBytes)?,
			body_root: beacon_block_header
				.body_root
				.as_bytes()
				.try_into()
				.map_err(|_| Error::InvalidNodeBytes)?,
		})
	}
}

#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq, Default)]
pub struct SyncCommittee {
	pub public_keys: Vec<Vec<u8>>,
	pub aggregate_public_key: Vec<u8>,
}

impl<const SYNC_COMMITTEE_SIZE: usize> TryFrom<bellatrix::SyncCommittee<SYNC_COMMITTEE_SIZE>>
	for SyncCommittee
{
	type Error = Error;

	fn try_from(
		sync_committee: bellatrix::SyncCommittee<SYNC_COMMITTEE_SIZE>,
	) -> Result<Self, Self::Error> {
		Ok(SyncCommittee {
			public_keys: sync_committee
				.public_keys
				.iter()
				.map(|public_key| public_key.to_vec())
				.collect(),
			aggregate_public_key: sync_committee.aggregate_public_key.to_vec(),
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
pub struct LightClientUpdate {
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
	pub sync_aggregate: SyncAggregate,
	/// slot at which signature was produced
	pub signature_slot: u64,
	/// ancestors of the finalized block to be verified, may be empty.
	pub ancestor_blocks: Vec<AncestorBlock>,
}

impl<const SYNC_COMMITTEE_SIZE: usize> TryFrom<types::LightClientUpdate<SYNC_COMMITTEE_SIZE>>
	for LightClientUpdate
{
	type Error = Error;
	fn try_from(
		update: types::LightClientUpdate<SYNC_COMMITTEE_SIZE>,
	) -> Result<Self, Self::Error> {
		to_codec_light_client_update(update)
	}
}

impl<const SYNC_COMMITTEE_SIZE: usize> TryFrom<LightClientUpdate>
	for types::LightClientUpdate<SYNC_COMMITTEE_SIZE>
{
	type Error = Error;
	fn try_from(derived_update: LightClientUpdate) -> Result<Self, Self::Error> {
		to_no_codec_light_client_update(derived_update)
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
pub struct SyncCommitteeUpdate {
	// actual sync committee
	pub next_sync_committee: SyncCommittee,
	// sync committee, ssz merkle proof.
	pub next_sync_committee_branch: Vec<Vec<u8>>,
}

impl<const SYNC_COMMITTEE_SIZE: usize> TryFrom<types::SyncCommitteeUpdate<SYNC_COMMITTEE_SIZE>>
	for SyncCommitteeUpdate
{
	type Error = Error;

	fn try_from(
		sync_committee_update: types::SyncCommitteeUpdate<SYNC_COMMITTEE_SIZE>,
	) -> Result<Self, Self::Error> {
		Ok(SyncCommitteeUpdate {
			next_sync_committee: sync_committee_update.next_sync_committee.try_into()?,
			next_sync_committee_branch: sync_committee_update
				.next_sync_committee_branch
				.iter()
				.map(|hash| hash.to_vec())
				.collect(),
		})
	}
}

impl<const SYNC_COMMITTEE_SIZE: usize> TryFrom<SyncCommitteeUpdate>
	for types::SyncCommitteeUpdate<SYNC_COMMITTEE_SIZE>
{
	type Error = Error;

	fn try_from(sync_committee_update: SyncCommitteeUpdate) -> Result<Self, Self::Error> {
		let next_sync_committee =
			to_no_codec_sync_committee(sync_committee_update.next_sync_committee)?;
		Ok(types::SyncCommitteeUpdate {
			next_sync_committee,
			next_sync_committee_branch: sync_committee_update
				.next_sync_committee_branch
				.iter()
				.map(|proof| Hash32::try_from(proof.as_ref()).map_err(|_| Error::InvalidProof))
				.collect::<Result<Vec<_>, Error>>()?,
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
pub struct ExecutionPayloadProof {
	/// The state root in the `ExecutionPayload` which represents the commitment to
	/// the ethereum world state in the yellow paper.
	pub state_root: Vec<u8>,
	/// the block number of the execution header.
	pub block_number: u64,
	/// merkle mutli proof for the state_root & block_number in the [`ExecutionPayload`].
	pub multi_proof: Vec<Vec<u8>>,
	/// merkle proof for the `ExecutionPayload` in the [`BeaconBlockBody`].
	pub execution_payload_branch: Vec<Vec<u8>>,
	/// timestamp
	pub timestamp: u64,
}

impl TryFrom<types::ExecutionPayloadProof> for ExecutionPayloadProof {
	type Error = Error;
	fn try_from(
		execution_payload_proof: types::ExecutionPayloadProof,
	) -> Result<Self, Self::Error> {
		Ok(ExecutionPayloadProof {
			state_root: execution_payload_proof.state_root.to_vec(),
			block_number: execution_payload_proof.block_number,
			multi_proof: execution_payload_proof
				.multi_proof
				.iter()
				.map(|proof| proof.to_vec())
				.collect(),
			execution_payload_branch: execution_payload_proof
				.execution_payload_branch
				.iter()
				.map(|branch| branch.to_vec())
				.collect(),
			timestamp: execution_payload_proof.timestamp,
		})
	}
}

impl TryFrom<ExecutionPayloadProof> for types::ExecutionPayloadProof {
	type Error = Error;
	fn try_from(
		derived_execution_payload_proof: ExecutionPayloadProof,
	) -> Result<Self, Self::Error> {
		let multi_proof = derived_execution_payload_proof
			.multi_proof
			.iter()
			.map(|proof| Hash32::try_from(proof.as_ref()).map_err(|_| Error::InvalidProof))
			.collect::<Result<Vec<_>, _>>()?;

		let execution_payload_branch = derived_execution_payload_proof
			.execution_payload_branch
			.iter()
			.map(|proof| Hash32::try_from(proof.as_ref()).map_err(|_| Error::InvalidProof))
			.collect::<Result<Vec<_>, _>>()?;

		Ok(types::ExecutionPayloadProof {
			state_root: Hash32::try_from(derived_execution_payload_proof.state_root.as_slice())
				.map_err(|_| Error::InvalidRoot)?,
			block_number: derived_execution_payload_proof.block_number,
			multi_proof,
			execution_payload_branch,
			timestamp: derived_execution_payload_proof.timestamp,
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
pub struct FinalityProof {
	/// The latest  finalized epoch
	pub epoch: u64,
	/// Finalized header proof
	pub finality_branch: Vec<Vec<u8>>,
}

impl TryFrom<types::FinalityProof> for FinalityProof {
	type Error = Error;
	fn try_from(finality_proof: types::FinalityProof) -> Result<Self, Self::Error> {
		Ok(FinalityProof {
			epoch: finality_proof.epoch,
			finality_branch: finality_proof
				.finality_branch
				.iter()
				.map(|branch| branch.to_vec())
				.collect(),
		})
	}
}

impl TryFrom<FinalityProof> for types::FinalityProof {
	type Error = Error;
	fn try_from(derived_finality_proof: FinalityProof) -> Result<Self, Self::Error> {
		Ok(types::FinalityProof {
			epoch: derived_finality_proof.epoch,
			finality_branch: derived_finality_proof
				.finality_branch
				.iter()
				.map(|proof| Hash32::try_from(proof.as_ref()).map_err(|_| Error::InvalidProof))
				.collect::<Result<Vec<_>, _>>()?,
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
pub struct SyncAggregate {
	pub sync_committee_bits: Vec<u8>,
	pub sync_committee_signature: Vec<u8>,
}

impl<const SYNC_COMMITTEE_SIZE: usize> TryFrom<bellatrix::SyncAggregate<SYNC_COMMITTEE_SIZE>>
	for SyncAggregate
{
	type Error = Error;
	fn try_from(
		sync_aggregate: bellatrix::SyncAggregate<SYNC_COMMITTEE_SIZE>,
	) -> Result<Self, Self::Error> {
		Ok(SyncAggregate {
			sync_committee_bits: sync_aggregate.sync_committee_bits.clone().to_bitvec().into_vec(),
			sync_committee_signature: sync_aggregate.sync_committee_signature.clone().to_vec(),
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct AncestorBlock {
	/// The actual beacon chain header
	pub header: BeaconBlockHeader,
	/// Associated execution header proofs
	pub execution_payload: ExecutionPayloadProof,
	/// Ancestry proofs of the beacon chain header.
	pub ancestry_proof: AncestryProof,
}

impl TryFrom<types::AncestorBlock> for AncestorBlock {
	type Error = Error;
	fn try_from(ancestor_block: types::AncestorBlock) -> Result<Self, Self::Error> {
		Ok(AncestorBlock {
			header: ancestor_block.header.try_into()?,
			execution_payload: ancestor_block.execution_payload.try_into()?,
			ancestry_proof: ancestor_block.ancestry_proof.try_into()?,
		})
	}
}

impl TryFrom<AncestorBlock> for types::AncestorBlock {
	type Error = Error;
	fn try_from(derived_ancestor_block: AncestorBlock) -> Result<Self, Self::Error> {
		let beacon_block_header = to_no_codec_beacon_header(derived_ancestor_block.header)?;
		Ok(types::AncestorBlock {
			header: beacon_block_header,
			execution_payload: derived_ancestor_block.execution_payload.try_into()?,
			ancestry_proof: derived_ancestor_block.ancestry_proof.try_into()?,
		})
	}
}

/// Holds the neccessary proofs required to verify a header in the `block_roots` field
/// either in [`BeaconState`] or [`HistoricalBatch`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct BlockRootsProof {
	/// Generalized index of the header in the `block_roots` list.
	pub block_header_index: u64,
	/// The proof for the header, needed to reconstruct `hash_tree_root(state.block_roots)`
	pub block_header_branch: Vec<Vec<u8>>,
}

impl TryFrom<types::BlockRootsProof> for BlockRootsProof {
	type Error = Error;
	fn try_from(beacon_block_header: types::BlockRootsProof) -> Result<Self, Self::Error> {
		Ok(BlockRootsProof {
			block_header_index: beacon_block_header.block_header_index,
			block_header_branch: beacon_block_header
				.block_header_branch
				.iter()
				.map(|hash| hash.to_vec())
				.collect(),
		})
	}
}

impl TryFrom<BlockRootsProof> for types::BlockRootsProof {
	type Error = Error;
	fn try_from(derived_beacon_block_header: BlockRootsProof) -> Result<Self, Self::Error> {
		let branch = derived_beacon_block_header
			.block_header_branch
			.iter()
			.map(|proof| Hash32::try_from(proof.as_ref()).map_err(|_| Error::InvalidProof))
			.collect::<Result<Vec<_>, _>>()?;

		Ok(types::BlockRootsProof {
			block_header_index: derived_beacon_block_header.block_header_index,
			block_header_branch: branch,
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum AncestryProof {
	/// This variant defines the proof data for a beacon chain header in the `state.block_roots`
	BlockRoots {
		/// Proof for the header in `state.block_roots`
		block_roots_proof: BlockRootsProof,
		/// The proof for the reconstructed `hash_tree_root(state.block_roots)` in [`BeaconState`]
		block_roots_branch: Vec<Vec<u8>>,
	},
	/// This variant defines the neccessary proofs for a beacon chain header in the
	/// `state.historical_roots`.
	HistoricalRoots {
		/// Proof for the header in `historical_batch.block_roots`
		block_roots_proof: BlockRootsProof,
		/// The proof for the `historical_batch.block_roots`, needed to reconstruct
		/// `hash_tree_root(historical_batch)`
		historical_batch_proof: Vec<Vec<u8>>,
		/// The proof for the `hash_tree_root(historical_batch)` in `state.historical_roots`
		historical_roots_proof: Vec<Vec<u8>>,
		/// The generalized index for the historical_batch in `state.historical_roots`.
		historical_roots_index: u64,
		/// The proof for the reconstructed `hash_tree_root(state.historical_roots)` in
		/// [`BeaconState`]
		historical_roots_branch: Vec<Vec<u8>>,
	},
}

impl TryFrom<types::AncestryProof> for AncestryProof {
	type Error = Error;
	fn try_from(ancestry_proof: types::AncestryProof) -> Result<Self, Self::Error> {
		Ok(match ancestry_proof {
			types::AncestryProof::BlockRoots { block_roots_proof, block_roots_branch } =>
				AncestryProof::BlockRoots {
					block_roots_proof: block_roots_proof.try_into()?,
					block_roots_branch: block_roots_branch
						.iter()
						.map(|hash| hash.to_vec())
						.collect(),
				},
			types::AncestryProof::HistoricalRoots {
				block_roots_proof,
				historical_batch_proof,
				historical_roots_proof,
				historical_roots_index,
				historical_roots_branch,
			} => AncestryProof::HistoricalRoots {
				block_roots_proof: block_roots_proof.try_into()?,
				historical_batch_proof: historical_batch_proof
					.iter()
					.map(|hash| hash.to_vec())
					.collect(),
				historical_roots_proof: historical_roots_proof
					.iter()
					.map(|hash| hash.to_vec())
					.collect(),
				historical_roots_index,
				historical_roots_branch: historical_roots_branch
					.iter()
					.map(|hash| hash.to_vec())
					.collect(),
			},
		})
	}
}

impl TryFrom<AncestryProof> for types::AncestryProof {
	type Error = Error;
	fn try_from(ancestry_proof: AncestryProof) -> Result<Self, Self::Error> {
		Ok(match ancestry_proof {
			AncestryProof::BlockRoots { block_roots_proof, block_roots_branch } =>
				types::AncestryProof::BlockRoots {
					block_roots_proof: block_roots_proof.try_into()?,
					block_roots_branch: block_roots_branch
						.iter()
						.map(|proof| {
							Hash32::try_from(proof.as_ref()).map_err(|_| Error::InvalidProof)
						})
						.collect::<Result<Vec<_>, _>>()?,
				},
			AncestryProof::HistoricalRoots {
				block_roots_proof,
				historical_batch_proof,
				historical_roots_proof,
				historical_roots_index,
				historical_roots_branch,
			} => types::AncestryProof::HistoricalRoots {
				block_roots_proof: block_roots_proof.try_into()?,
				historical_batch_proof: historical_batch_proof
					.iter()
					.map(|proof| Hash32::try_from(proof.as_ref()).map_err(|_| Error::InvalidProof))
					.collect::<Result<Vec<_>, _>>()?,
				historical_roots_proof: historical_roots_proof
					.iter()
					.map(|proof| Hash32::try_from(proof.as_ref()).map_err(|_| Error::InvalidProof))
					.collect::<Result<Vec<_>, _>>()?,
				historical_roots_index,
				historical_roots_branch: historical_roots_branch
					.iter()
					.map(|proof| Hash32::try_from(proof.as_ref()).map_err(|_| Error::InvalidProof))
					.collect::<Result<Vec<_>, _>>()?,
			},
		})
	}
}
