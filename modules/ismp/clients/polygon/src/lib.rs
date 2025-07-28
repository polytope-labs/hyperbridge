#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_variables)]

extern crate alloc;

use base64::{engine::general_purpose::STANDARD, Engine as _};

use alloc::{boxed::Box, collections::BTreeMap, string::ToString, vec, vec::Vec};
use codec::{Decode, Encode};
use geth_primitives::Header;
use ics23::HostFunctionsManager;
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
		StateMachineId, VerifiedCommitments,
	},
	error::Error,
	host::{IsmpHost, StateMachine},
	messaging::{Keccak256, StateCommitmentHeight},
};

use evm_state_machine::EvmStateMachine;
use pallet_ismp_host_executive::Config as HostExecutiveConfig;
use prost::Message;
use scale_info::prelude::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use tendermint_primitives::{CodecConsensusProof, CodecTrustedState};
use tendermint_verifier::verify_header_update;

pub const POLYGON_CONSENSUS_CLIENT_ID: ConsensusClientId = *b"PLGN";

/// The consensus update/proof for Polygon
#[derive(Debug, Clone, Encode, Decode)]
pub struct PolygonConsensusUpdate {
	/// Serialized Tendermint light client update (signed header, validator set, etc.)
	pub tendermint_proof: CodecConsensusProof,
	/// Milestone update
	pub milestone_update: Option<MilestoneUpdate>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct MilestoneUpdate {
	/// EVM block header for the milestone's end block
	pub evm_header: geth_primitives::CodecHeader,
	/// Milestone number
	pub milestone_number: u64,
	/// ICS23 proof for the milestone inclusion
	pub ics23_state_proof: Vec<u8>,
	// Milestone
	pub milestone: Milestone,
}

/// The trusted consensus state for Polygon
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct ConsensusState {
	///  Codec Trusted Tendermint state
	pub tendermint_state: Vec<u8>,
	/// Last finalized Polygon block number
	pub last_finalized_block: u64,
	/// Last finalized Polygon block hash
	pub last_finalized_hash: Vec<u8>,
	/// Chain ID
	pub chain_id: u32,
}

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
	/// Hash of the milestone
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
	pub fn proto_encode(&self) -> Vec<u8> {
		let proto: ProtoMilestone = self.into();
		proto.encode_to_vec()
	}

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
	base64::engine::general_purpose::STANDARD
		.decode(s)
		.map_err(serde::de::Error::custom)
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoMilestone {
	#[prost(string, tag = "1")]
	pub proposer: String,
	#[prost(uint64, tag = "2")]
	pub start_block: u64,
	#[prost(uint64, tag = "3")]
	pub end_block: u64,
	#[prost(bytes = "vec", tag = "4")]
	pub hash: Vec<u8>,
	#[prost(string, tag = "5")]
	pub bor_chain_id: String,
	#[prost(string, tag = "6")]
	pub milestone_id: String,
	#[prost(uint64, tag = "7")]
	pub timestamp: u64,
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
		_host: &dyn IsmpHost,
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

		let trusted_state = CodecTrustedState::decode(&mut &consensus_state.tendermint_state[..])
			.map_err(|e| ismp::error::Error::Custom(e.to_string()))?
			.to_trusted_state()
			.map_err(|e| ismp::error::Error::Custom(e.to_string()))?;

		let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

		let result = verify_header_update(trusted_state, consensus_proof.clone(), time);

		match result {
			Ok(updated_state) => {
				let mut state_machine_map: BTreeMap<StateMachineId, Vec<StateCommitmentHeight>> =
					BTreeMap::new();
				let mut updated_consensus_state = consensus_state.clone();

				if let Some(milestone_update_ref) = &polygon_consensus_update.milestone_update {
					let evm_header = Header::from(&milestone_update_ref.evm_header.clone());
					let evm_header_hash = evm_header.hash::<KeccakHasher>().as_bytes().to_vec();
					let milestone_hash =
						STANDARD.decode(&milestone_update_ref.milestone.hash).unwrap_or_default();

					if evm_header_hash != milestone_hash {
						return Err(ismp::error::Error::Custom(
							"EVM header hash does not match milestone hash".to_string(),
						));
					}

					if milestone_update_ref.milestone_number !=
						milestone_update_ref.evm_header.number.low_u64()
					{
						return Err(ismp::error::Error::Custom(
							"Milestone number does not match EVM header number".to_string(),
						));
					}

					if milestone_update_ref.evm_header.number.low_u64() <
						consensus_state.last_finalized_block
					{
						return Err(ismp::error::Error::Custom(
							"EVM header number is less than last finalized block".to_string(),
						));
					}

					let commitment_proof = ics23::CommitmentProof::decode(
						&mut &milestone_update_ref.ics23_state_proof[..],
					)
					.map_err(|e| ismp::error::Error::Custom(e.to_string()))?;

					let spec = ics23::tendermint_spec();
					let mut key = vec![0x81];
					key.extend_from_slice(&milestone_update_ref.milestone_number.to_be_bytes());

					let verification_result = ics23::verify_membership::<HostFunctionsManager>(
						&commitment_proof,
						&spec,
						&consensus_proof.signed_header.header.app_hash.as_bytes().to_vec(),
						&key,
						&milestone_update_ref.milestone.proto_encode(),
					);

					if !verification_result {
						return Err(ismp::error::Error::Custom(
							"ICS23 proof verification failed".to_string(),
						));
					}

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

					// Update the consensus state in the milestone block
					updated_consensus_state.last_finalized_block =
						milestone_update_ref.evm_header.number.low_u64();
				}

				updated_consensus_state.tendermint_state =
					CodecTrustedState::from_trusted_state(&updated_state.trusted_state).encode();

				Ok((updated_consensus_state.encode(), state_machine_map))
			},
			Err(e) => Err(ismp::error::Error::Custom(e.to_string())),
		}
	}

	fn verify_fraud_proof(
		&self,
		_host: &dyn IsmpHost,
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

		let trusted_state = CodecTrustedState::decode(&mut &consensus_state.tendermint_state[..])
			.map_err(|e| Error::Custom(e.to_string()))?
			.to_trusted_state()
			.map_err(|e| Error::Custom(e.to_string()))?;

		let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

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
			StateMachine::Evm(_) => Ok(Box::new(EvmStateMachine::<H, T>::default())),
			_ => Err(Error::Custom("Unsupported state machine".to_string())),
		}
	}
}

/// Keccak hasher for EVM headers
pub struct KeccakHasher;
impl Keccak256 for KeccakHasher {
	fn keccak256(bytes: &[u8]) -> primitive_types::H256 {
		sp_core::keccak_256(bytes).into()
	}
}
