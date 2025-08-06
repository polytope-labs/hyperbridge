//! Polygon consensus client implementation for ISMP.
//!
//! This module provides a consensus client for Polygon that verifies Tendermint light client
//! updates and milestone proofs to maintain consensus state across the network.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

use alloc::{
	boxed::Box,
	collections::BTreeMap,
	format,
	string::{String, ToString},
	vec,
	vec::Vec,
};
use base64::prelude::{Engine as _, BASE64_STANDARD};
use codec::{Decode, Encode};
use geth_primitives::Header;
use ibc::core::{
	commitment_types::{
		commitment::CommitmentProofBytes,
		merkle::{MerklePath, MerkleProof},
		proto::v1::MerkleRoot,
		specs::ProofSpecs,
	},
	host::types::path::PathBytes,
};
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
		StateMachineId, VerifiedCommitments,
	},
	error::Error,
	host::{IsmpHost, StateMachine},
	messaging::StateCommitmentHeight,
};

use evm_state_machine::EvmStateMachine;
use pallet_ismp_host_executive::Config as HostExecutiveConfig;
use prost::Message;
use serde::{Deserialize, Serialize};
use tendermint_ics23_primitives::ICS23HostFunctions;
use tendermint_primitives::{CodecConsensusProof, CodecTrustedState, TrustedState};
use tendermint_verifier::verify_header_update;

/// Consensus client ID for Polygon
pub const POLYGON_CONSENSUS_CLIENT_ID: ConsensusClientId = *b"PLGN";

/// Polygon mainnet chain ID
pub const POLYGON_MAINNET_CHAIN_ID: u32 = 137;
/// Polygon testnet (Amoy) chain ID
pub const POLYGON_TESTNET_CHAIN_ID: u32 = 80002;

/// The consensus update/proof for Polygon
#[derive(Debug, Clone, Encode, Decode)]
pub struct PolygonConsensusUpdate {
	/// Serialized Tendermint light client update (signed header, validator set, etc.)
	pub tendermint_proof: CodecConsensusProof,
	/// Milestone update
	pub milestone_update: Option<MilestoneUpdate>,
}

/// Milestone update containing EVM header and proof data
#[derive(Debug, Clone, Encode, Decode)]
pub struct MilestoneUpdate {
	/// EVM block header for the milestone's end block
	pub evm_header: geth_primitives::CodecHeader,
	/// Milestone number
	pub milestone_number: u64,
	/// ICS23 proof for the milestone inclusion
	pub ics23_state_proof: Vec<u8>,
	/// Milestone data
	pub milestone: Milestone,
}

/// The trusted consensus state for Polygon
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct ConsensusState {
	/// Codec Trusted Tendermint state
	pub tendermint_state: CodecTrustedState,
	/// Last finalized Polygon block number
	pub last_finalized_block: u64,
	/// Last finalized Polygon block hash
	pub last_finalized_hash: Vec<u8>,
	/// Chain ID
	pub chain_id: u32,
}

/// Milestone data structure containing block range and metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Milestone {
	/// Proposer address (hex string)
	pub proposer: String,
	/// Start block number of the milestone
	#[serde(deserialize_with = "deserialize_u64_from_str")]
	pub start_block: u64,
	/// End block number of the milestone
	#[serde(deserialize_with = "deserialize_u64_from_str")]
	pub end_block: u64,
	/// Hash of the end block
	#[serde(deserialize_with = "deserialize_hash_from_base64")]
	pub hash: Vec<u8>,
	/// Bor chain ID (string)
	pub bor_chain_id: String,
	/// Milestone ID (hex string)
	pub milestone_id: String,
	/// Timestamp of the milestone
	#[serde(deserialize_with = "deserialize_u64_from_str")]
	pub timestamp: u64,
	/// Total difficulty at this milestone
	#[serde(deserialize_with = "deserialize_u64_from_str")]
	pub total_difficulty: u64,
}

impl Milestone {
	/// Encode the milestone to protobuf format
	pub fn proto_encode(&self) -> Vec<u8> {
		let proto: ProtoMilestone = self.into();
		proto.encode_to_vec()
	}

	/// Decode the milestone from protobuf format
	pub fn proto_decode(bytes: &[u8]) -> Result<Self, prost::DecodeError> {
		let proto = ProtoMilestone::decode(bytes)?;
		Ok((&proto).into())
	}
}

fn deserialize_u64_from_str<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
	D: serde::Deserializer<'de>,
{
	use serde::Deserialize;
	let s = String::deserialize(deserializer)?;
	s.parse::<u64>().map_err(serde::de::Error::custom)
}

fn deserialize_hash_from_base64<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	use serde::Deserialize;
	let s = String::deserialize(deserializer)?;
	BASE64_STANDARD.decode(&s).map_err(serde::de::Error::custom)
}

/// Protobuf representation of a milestone
#[derive(Clone, PartialEq, Message)]
pub struct ProtoMilestone {
	/// Proposer address
	#[prost(string, tag = "1")]
	pub proposer: String,
	/// Start block number
	#[prost(uint64, tag = "2")]
	pub start_block: u64,
	/// End block number
	#[prost(uint64, tag = "3")]
	pub end_block: u64,
	/// Hash of the end block
	#[prost(bytes = "vec", tag = "4")]
	pub hash: Vec<u8>,
	/// Bor chain ID
	#[prost(string, tag = "5")]
	pub bor_chain_id: String,
	/// Milestone ID
	#[prost(string, tag = "6")]
	pub milestone_id: String,
	/// Timestamp of the milestone
	#[prost(uint64, tag = "7")]
	pub timestamp: u64,
	/// Total difficulty at this milestone
	#[prost(uint64, tag = "8")]
	pub total_difficulty: u64,
}

impl From<&Milestone> for ProtoMilestone {
	fn from(milestone: &Milestone) -> Self {
		Self {
			proposer: milestone.proposer.clone(),
			start_block: milestone.start_block,
			end_block: milestone.end_block,
			hash: milestone.hash.clone(),
			bor_chain_id: milestone.bor_chain_id.clone(),
			milestone_id: milestone.milestone_id.clone(),
			timestamp: milestone.timestamp,
			total_difficulty: milestone.total_difficulty,
		}
	}
}

impl From<&ProtoMilestone> for Milestone {
	fn from(proto: &ProtoMilestone) -> Self {
		Self {
			proposer: proto.proposer.clone(),
			start_block: proto.start_block,
			end_block: proto.end_block,
			hash: proto.hash.clone(),
			bor_chain_id: proto.bor_chain_id.clone(),
			milestone_id: proto.milestone_id.clone(),
			timestamp: proto.timestamp,
			total_difficulty: proto.total_difficulty,
		}
	}
}

/// Polygon consensus client implementation
pub struct PolygonClient<H: IsmpHost, T: HostExecutiveConfig>(core::marker::PhantomData<(H, T)>);

impl<H: IsmpHost, T: HostExecutiveConfig> Default for PolygonClient<H, T> {
	fn default() -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<H: IsmpHost + Send + Sync + Default + 'static, T: HostExecutiveConfig> ConsensusClient
	for PolygonClient<H, T>
{
	fn verify_consensus(
		&self,
		host: &dyn IsmpHost,
		_consensus_state_id: ConsensusStateId,
		trusted_consensus_state: Vec<u8>,
		proof: Vec<u8>,
	) -> Result<(Vec<u8>, VerifiedCommitments), ismp::error::Error> {
		let polygon_consensus_update: PolygonConsensusUpdate = Decode::decode(&mut &proof[..])
			.map_err(|e| ismp::error::Error::Custom(e.to_string()))?;

		let consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
			.map_err(|e| ismp::error::Error::Custom(e.to_string()))?;

		let consensus_proof = polygon_consensus_update
			.tendermint_proof
			.to_consensus_proof()
			.map_err(|e| ismp::error::Error::Custom(e.to_string()))?;

		let trusted_state: TrustedState = consensus_state.clone().tendermint_state.into();

		let time = host.timestamp().as_secs();

		let updated_state = verify_header_update(trusted_state, consensus_proof.clone(), time)
			.map_err(|e| ismp::error::Error::Custom(e.to_string()))?;

		let mut state_machine_map: BTreeMap<StateMachineId, Vec<StateCommitmentHeight>> =
			BTreeMap::new();
		let mut updated_consensus_state = consensus_state.clone();

		if let Some(milestone_update_ref) = &polygon_consensus_update.milestone_update {
			let evm_header = Header::from(&milestone_update_ref.evm_header.clone());
			let evm_header_hash = evm_header.hash::<H>().as_bytes().to_vec();
			let milestone_hash = &milestone_update_ref.milestone.hash;

			if &evm_header_hash != milestone_hash {
				return Err(ismp::error::Error::Custom(format!(
					"EVM header hash does not match milestone hash: {:?} != {:?}",
					evm_header_hash, milestone_hash
				)));
			}

			if milestone_update_ref.milestone.end_block !=
				milestone_update_ref.evm_header.number.low_u64()
			{
				return Err(ismp::error::Error::Custom(
					"Milestone end block does not match EVM header number".to_string(),
				));
			}

			if milestone_update_ref.evm_header.number.low_u64() <
				consensus_state.last_finalized_block
			{
				return Err(ismp::error::Error::Custom(
					"EVM header number is less than last finalized block".to_string(),
				));
			}

			let commitment_proof =
				CommitmentProofBytes::try_from(milestone_update_ref.ics23_state_proof.clone())
					.map_err(|e| ismp::error::Error::Custom(e.to_string()))?;

			let merkle_proof = MerkleProof::try_from(&commitment_proof)
				.map_err(|e| ismp::error::Error::Custom(e.to_string()))?;

			let mut key = vec![0x81];
			key.extend_from_slice(&milestone_update_ref.milestone_number.to_be_bytes());

			let specs = ProofSpecs::cosmos();
			let root = MerkleRoot {
				hash: consensus_proof.signed_header.header.app_hash.as_bytes().to_vec(),
			};

			let merkle_path = MerklePath::new(vec![
				PathBytes::from_bytes(b"milestone"),
				PathBytes::from_bytes(&key),
			]);

			let start_index = 0;
			let value = milestone_update_ref.milestone.proto_encode();

			merkle_proof
				.verify_membership::<ICS23HostFunctions>(
					&specs,
					root,
					merkle_path,
					value,
					start_index,
				)
				.map_err(|e| ismp::error::Error::Custom(e.to_string()))?;

			let evm_header = &milestone_update_ref.evm_header;
			let state_commitment = StateCommitmentHeight {
				commitment: StateCommitment {
					timestamp: evm_header.timestamp,
					overlay_root: None,
					state_root: evm_header.state_root,
				},
				height: evm_header.number.low_u64(),
			};

			state_machine_map.insert(
				StateMachineId {
					state_id: StateMachine::Evm(consensus_state.chain_id),
					consensus_state_id: _consensus_state_id,
				},
				vec![state_commitment],
			);

			// Update the evm related fields in the consensus state inside the milestone block
			updated_consensus_state.last_finalized_block =
				milestone_update_ref.evm_header.number.low_u64();
			updated_consensus_state.last_finalized_hash = evm_header_hash;
		}

		updated_consensus_state.tendermint_state =
			CodecTrustedState::from(&updated_state.trusted_state);

		Ok((updated_consensus_state.encode(), state_machine_map))
	}

	fn verify_fraud_proof(
		&self,
		host: &dyn IsmpHost,
		trusted_consensus_state: Vec<u8>,
		proof_1: Vec<u8>,
		proof_2: Vec<u8>,
	) -> Result<(), Error> {
		let update_1: PolygonConsensusUpdate =
			Decode::decode(&mut &proof_1[..]).map_err(|e| Error::Custom(e.to_string()))?;
		let update_2: PolygonConsensusUpdate =
			Decode::decode(&mut &proof_2[..]).map_err(|e| Error::Custom(e.to_string()))?;

		let consensus_state: ConsensusState = Decode::decode(&mut &trusted_consensus_state[..])
			.map_err(|e| Error::Custom(e.to_string()))?;

		let height_1 = update_1.tendermint_proof.signed_header.header.height;
		let height_2 = update_2.tendermint_proof.signed_header.header.height;
		if height_1 != height_2 {
			return Err(Error::Custom("Fraud proofs must be for the same block height".to_string()));
		}

		if proof_1 == proof_2 {
			return Err(Error::Custom("Fraud proofs are identical".to_string()));
		}

		let trusted_state: TrustedState = consensus_state.clone().tendermint_state.into();

		let time = host.timestamp().as_secs();

		let consensus_proof_1 = update_1
			.tendermint_proof
			.to_consensus_proof()
			.map_err(|e| Error::Custom(e.to_string()))?;

		let consensus_proof_2 = update_2
			.tendermint_proof
			.to_consensus_proof()
			.map_err(|e| Error::Custom(e.to_string()))?;

		verify_header_update(trusted_state.clone(), consensus_proof_1, time)
			.map_err(|e| Error::Custom(e.to_string()))?;
		verify_header_update(trusted_state, consensus_proof_2, time)
			.map_err(|e| Error::Custom(e.to_string()))?;

		Ok(())
	}

	fn consensus_client_id(&self) -> ConsensusClientId {
		POLYGON_CONSENSUS_CLIENT_ID
	}

	fn state_machine(&self, id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
		match id {
			StateMachine::Evm(chain_id)
				if chain_id == POLYGON_MAINNET_CHAIN_ID || chain_id == POLYGON_TESTNET_CHAIN_ID =>
				Ok(Box::new(EvmStateMachine::<H, T>::default())),
			_ => Err(Error::Custom("Unsupported state machine or chain ID".to_string())),
		}
	}
}
