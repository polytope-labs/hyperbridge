use crate::{
	derived_types,
	error::Error,
	types,
	types::{LightClientState, LightClientUpdate, SyncCommitteeUpdate},
};
use alloc::vec::Vec;
use ethereum_consensus::{
	bellatrix::{BeaconBlockHeader, SyncAggregate, SyncCommittee},
	crypto::PublicKey,
	primitives::BlsSignature,
};
use ssz_rs::{Bitvector, Deserialize, Node, Vector};

pub fn to_no_codec_beacon_header(
	derived_header: derived_types::BeaconBlockHeader,
) -> Result<BeaconBlockHeader, Error> {
	let finalized_header = BeaconBlockHeader {
		slot: derived_header.slot,
		proposer_index: derived_header.proposer_index as usize,
		parent_root: Node::from_bytes(
			derived_header.parent_root.as_ref().try_into().map_err(|_| Error::InvalidRoot)?,
		),
		state_root: Node::from_bytes(
			derived_header.state_root.as_ref().try_into().map_err(|_| Error::InvalidRoot)?,
		),
		body_root: Node::from_bytes(
			derived_header.body_root.as_ref().try_into().map_err(|_| Error::InvalidRoot)?,
		),
	};

	Ok(finalized_header)
}

pub fn to_no_codec_sync_committee<const SYNC_COMMITTEE_SIZE: usize>(
	derived_sync_committee: derived_types::SyncCommittee,
) -> Result<SyncCommittee<SYNC_COMMITTEE_SIZE>, Error> {
	let public_keys_vector: Vec<PublicKey> = derived_sync_committee
		.public_keys
		.iter()
		.map(|public_key| {
			PublicKey::try_from(public_key.as_slice()).map_err(|_| Error::InvalidPublicKey)
		})
		.collect::<Result<Vec<_>, Error>>()?;
	let sync_committee = SyncCommittee {
		public_keys: Vector::try_from(public_keys_vector).unwrap(),
		aggregate_public_key: PublicKey::try_from(
			derived_sync_committee.aggregate_public_key.as_slice(),
		)
		.map_err(|_| Error::InvalidPublicKey)?,
	};

	Ok(sync_committee)
}

pub fn to_no_codec_sync_aggregate<const SYNC_COMMITTEE_SIZE: usize>(
	derived_sync_aggregate: derived_types::SyncAggregate,
) -> Result<SyncAggregate<SYNC_COMMITTEE_SIZE>, Error> {
	let derived_sync_committee_bits = derived_sync_aggregate.sync_committee_bits;
	let bit_vector = Bitvector::<SYNC_COMMITTEE_SIZE>::deserialize(&derived_sync_committee_bits)
		.map_err(|_| Error::InvalidBitVec)?;

	let sync_aggregate = SyncAggregate {
		sync_committee_bits: bit_vector,
		sync_committee_signature: BlsSignature::try_from(
			derived_sync_aggregate.sync_committee_signature.as_ref(),
		)
		.map_err(|_| Error::InvalidPublicKey)?,
	};

	Ok(sync_aggregate)
}

pub fn to_no_codec_light_client_state<const SYNC_COMMITTEE_SIZE: usize>(
	state: derived_types::LightClientState,
) -> Result<LightClientState<SYNC_COMMITTEE_SIZE>, Error> {
	let finalized_header = to_no_codec_beacon_header(state.finalized_header)?;

	let current_sync_committee = to_no_codec_sync_committee(state.current_sync_committee.clone())?;
	let next_sync_committee = to_no_codec_sync_committee(state.next_sync_committee)?;

	Ok(LightClientState {
		finalized_header,
		latest_finalized_epoch: state.latest_finalized_epoch,
		current_sync_committee,
		next_sync_committee,
	})
}

pub fn to_no_codec_light_client_update<const SYNC_COMMITTEE_SIZE: usize>(
	derived_update: derived_types::LightClientUpdate,
) -> Result<LightClientUpdate<SYNC_COMMITTEE_SIZE>, Error> {
	let sync_committee_update_option: Option<SyncCommitteeUpdate<SYNC_COMMITTEE_SIZE>>;

	match derived_update.sync_committee_update {
		Some(sync_committee_update) =>
			sync_committee_update_option = Some(sync_committee_update.try_into()?),
		None => sync_committee_update_option = None,
	}
	Ok(LightClientUpdate {
		attested_header: to_no_codec_beacon_header(derived_update.attested_header)?,
		sync_committee_update: sync_committee_update_option,
		finalized_header: to_no_codec_beacon_header(derived_update.finalized_header)?,
		execution_payload: derived_update.execution_payload.try_into()?,
		finality_proof: derived_update.finality_proof.try_into()?,
		sync_aggregate: to_no_codec_sync_aggregate(derived_update.sync_aggregate)?,
		signature_slot: derived_update.signature_slot,
		ancestor_blocks: derived_update
			.ancestor_blocks
			.iter()
			.map(|ancestor_block| {
				ancestor_block
					.clone()
					.try_into()
					.map_err(|_| Error::ErrorConvertingAncestorBlock)
			})
			.collect::<Result<Vec<_>, Error>>()?,
	})
}

pub fn to_codec_light_client_state<const SYNC_COMMITTEE_SIZE: usize>(
	state: types::LightClientState<SYNC_COMMITTEE_SIZE>,
) -> Result<derived_types::LightClientState, Error> {
	Ok(derived_types::LightClientState {
		finalized_header: state.finalized_header.try_into()?,
		latest_finalized_epoch: state.latest_finalized_epoch,
		current_sync_committee: state.current_sync_committee.try_into()?,
		next_sync_committee: state.next_sync_committee.try_into()?,
	})
}

pub fn to_codec_light_client_update<const SYNC_COMMITTEE_SIZE: usize>(
	update: types::LightClientUpdate<SYNC_COMMITTEE_SIZE>,
) -> Result<derived_types::LightClientUpdate, Error> {
	let sync_committee_update_option: Option<derived_types::SyncCommitteeUpdate>;

	match update.sync_committee_update {
		Some(sync_committee_update) =>
			sync_committee_update_option = Some(sync_committee_update.try_into()?),

		None => sync_committee_update_option = None,
	}
	Ok(derived_types::LightClientUpdate {
		attested_header: update.attested_header.try_into()?,
		sync_committee_update: sync_committee_update_option,
		finalized_header: update.finalized_header.try_into()?,
		execution_payload: update.execution_payload.try_into()?,
		finality_proof: update.finality_proof.try_into()?,
		sync_aggregate: update.sync_aggregate.try_into()?,
		signature_slot: update.signature_slot,
		ancestor_blocks: update
			.ancestor_blocks
			.iter()
			.map(|ancestor_block| {
				ancestor_block
					.clone()
					.try_into()
					.map_err(|_| Error::ErrorConvertingAncestorBlock)
			})
			.collect::<Result<Vec<_>, Error>>()?,
	})
}
