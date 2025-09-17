//! Tendermint consensus client implementation for ISMP.
//!
//! This module provides a consensus client for a Tendermint-based chain that verifies Tendermint light client
//! updates

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

use alloc::{boxed::Box, collections::BTreeMap, string::ToString, vec, vec::Vec};
use codec::{Decode, Encode};
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
		StateMachineId, VerifiedCommitments,
	},
	error::Error,
	host::{IsmpHost, StateMachine},
	messaging::StateCommitmentHeight,
};

use pallet_ismp_host_executive::Config as HostExecutiveConfig;
use tendermint_primitives::{CodecConsensusProof, CodecTrustedState, TrustedState};
use tendermint_verifier::verify_header_update;

/// Consensus client ID for Tendermint
/// TODO: Make this configurable
pub const TENDERMINT_CONSENSUS_CLIENT_ID: ConsensusClientId = *b"TNDR";

/// The consensus update/proof for Tendermint
#[derive(Debug, Clone, Encode, Decode)]
pub struct TendermintConsensusUpdate {
	/// Serialized Tendermint light client update (signed header, validator set, etc.)
	pub tendermint_proof: CodecConsensusProof,
}

/// The trusted consensus state for Tendermint
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct ConsensusState {
	/// Codec Trusted Tendermint state
	pub tendermint_state: CodecTrustedState,
	/// Chain ID
	pub chain_id: [u8; 4],
}

/// Tendermint consensus client implementation
pub struct TendermintClient<H: IsmpHost, T: HostExecutiveConfig>(core::marker::PhantomData<(H, T)>);

impl<H: IsmpHost, T: HostExecutiveConfig> Default for TendermintClient<H, T> {
	fn default() -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<H: IsmpHost + Send + Sync + Default + 'static, T: HostExecutiveConfig> ConsensusClient
	for TendermintClient<H, T>
{
	fn verify_consensus(
		&self,
		host: &dyn IsmpHost,
		_consensus_state_id: ConsensusStateId,
		trusted_consensus_state: Vec<u8>,
		proof: Vec<u8>,
	) -> Result<(Vec<u8>, VerifiedCommitments), ismp::error::Error> {
		let tendermint_consensus_update: TendermintConsensusUpdate =
			Decode::decode(&mut &proof[..])
				.map_err(|e| ismp::error::Error::Custom(e.to_string()))?;

		let consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
			.map_err(|e| ismp::error::Error::Custom(e.to_string()))?;

		let consensus_proof = tendermint_consensus_update
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

		let state_commitment = StateCommitmentHeight {
			commitment: StateCommitment {
				timestamp: updated_state.verified_timestamp,
				overlay_root: None,
				// Is finalized header hash the same as the state root?
				state_root: primitive_types::H256(
					updated_state.trusted_state.finalized_header_hash,
				),
			},
			height: updated_state.trusted_state.height,
		};

		state_machine_map.insert(
			StateMachineId {
				state_id: StateMachine::Tendermint(consensus_state.chain_id),
				consensus_state_id: _consensus_state_id,
			},
			vec![state_commitment],
		);

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
		let update_1: TendermintConsensusUpdate =
			Decode::decode(&mut &proof_1[..]).map_err(|e| Error::Custom(e.to_string()))?;
		let update_2: TendermintConsensusUpdate =
			Decode::decode(&mut &proof_2[..]).map_err(|e| Error::Custom(e.to_string()))?;

		let consensus_state: ConsensusState = Decode::decode(&mut &trusted_consensus_state[..])
			.map_err(|e| Error::Custom(e.to_string()))?;

		let height_1 = update_1.tendermint_proof.signed_header.header.height;
		let height_2 = update_2.tendermint_proof.signed_header.header.height;
		if height_1 != height_2 {
			return Err(Error::Custom(
				"Fraud proofs must be for the same block height".to_string(),
			));
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
		TENDERMINT_CONSENSUS_CLIENT_ID
	}

	// TODO: May need to implement TendermintStateMachine after clarifying
	// How to use TendermintStateMachine here?
	fn state_machine(&self, id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
		// match id {
		// 	StateMachine::Tendermint(chain_id) => {
		// 		Ok(Box::new(TendermintStateMachine::<H, T>::default()))
		// 	},
		// 	_ => Err(Error::Custom("Unsupported state machine or chain ID".to_string())),
		// }
		unimplemented!()
	}
}
