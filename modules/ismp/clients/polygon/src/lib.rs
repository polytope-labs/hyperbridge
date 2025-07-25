#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_variables)]

extern crate alloc;

use alloc::{boxed::Box, collections::BTreeMap, string::ToString, vec, vec::Vec};
use codec::{Decode, Encode};
use ics23::HostFunctionsManager;
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
use scale_info::prelude::time::{SystemTime, UNIX_EPOCH};
use tendermint_primitives::{CodecConsensusProof, CodecTrustedState, Milestone};
use tendermint_verifier::verify_header_update;

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
	pub ics23_state_proof: ICS23Proof,
	// Milestone
	pub milestone: Milestone,
	/// Untrusted header app hash for verification
	pub untrusted_header_app_hash: Vec<u8>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ICS23Proof {
	/// Proof bytes
	pub proof: Vec<u8>,
	/// Key
	pub key: Vec<u8>,
	/// Value
	pub value: Vec<u8>,
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
}

pub const POLYGON_CONSENSUS_CLIENT_ID: ConsensusClientId = *b"PLGN"; // TODO: Change to Polygon Consensus ID

const POLYGON_CHAIN_ID: u32 = 137;
const POLYGON_TESTNET_CHAIN_ID: u32 = 80002;

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

		let milestone_update = polygon_consensus_update.milestone_update.clone();

		if let Some(milestone_update_ref) = &polygon_consensus_update.milestone_update {
			// Check milestone_number == milestone.end_block and both match evm_header.number
			let end_block =
				milestone_update_ref.milestone.end_block.parse::<u64>().map_err(|_| {
					ismp::error::Error::Custom("Invalid milestone.end_block".to_string())
				})?;
			let evm_block = milestone_update_ref.evm_header.number.low_u64();
			if milestone_update_ref.milestone_number != end_block || end_block != evm_block {
				return Err(ismp::error::Error::Custom(
					"Milestone number, end_block, and EVM header number do not match".to_string(),
				));
			}

			if milestone_update_ref.milestone_number <= consensus_state.last_finalized_block {
				return Err(ismp::error::Error::Custom("Expired update".to_string()));
			}

			let commitment_proof = ics23::CommitmentProof::decode(
				&mut &milestone_update_ref.ics23_state_proof.proof[..],
			)
			.map_err(|e| ismp::error::Error::Custom(e.to_string()))?;

			let spec = ics23::tendermint_spec();

			let verification_result = ics23::verify_membership::<HostFunctionsManager>(
				&commitment_proof,
				&spec,
				&milestone_update_ref.untrusted_header_app_hash,
				&milestone_update_ref.ics23_state_proof.key,
				&milestone_update_ref.ics23_state_proof.value,
			);

			if !verification_result {
				return Err(ismp::error::Error::Custom(
					"ICS23 proof verification failed".to_string(),
				));
			}
		}

		let consensus_proof = polygon_consensus_update
			.tendermint_proof
			.to_consensus_proof()
			.map_err(|e| ismp::error::Error::Custom(e.to_string()))?;

		let trusted_state = CodecTrustedState::decode(&mut &consensus_state.tendermint_state[..])
			.map_err(|e| ismp::error::Error::Custom(e.to_string()))?
			.to_trusted_state()
			.map_err(|e| ismp::error::Error::Custom(e.to_string()))?;

		let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

		let result = verify_header_update(trusted_state, consensus_proof, time);

		match result {
			Ok(updated_state) => {
				let mut state_machine_map: BTreeMap<StateMachineId, Vec<StateCommitmentHeight>> =
					BTreeMap::new();

				if let Some(milestone_update) = &milestone_update {
					let evm_header = &milestone_update.evm_header;

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
							state_id: StateMachine::Evm(POLYGON_CHAIN_ID),
							consensus_state_id: _consensus_state_id,
						},
						vec![state_commitment],
					);
				}

				let mut updated_consensus_state = consensus_state.clone();
				updated_consensus_state.last_finalized_block = updated_state.verified_height;
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
			StateMachine::Evm(chain_id)
				if chain_id == POLYGON_CHAIN_ID || chain_id == POLYGON_TESTNET_CHAIN_ID =>
				Ok(Box::new(EvmStateMachine::<H, T>::default())),
			_ => Err(Error::Custom("Unsupported state machine".to_string())),
		}
	}
}
