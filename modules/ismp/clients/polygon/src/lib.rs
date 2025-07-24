#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_variables)]

extern crate alloc;

use alloc::{boxed::Box, string::ToString, vec::Vec};
use codec::{Decode, Encode};
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateMachineClient,
		VerifiedCommitments,
	},
	error::Error,
	host::{IsmpHost, StateMachine},
};
use tendermint_primitives::{CodecConsensusProof, Milestone};

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
	pub milestone_number: Option<u64>,
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
}

pub const POLYGON_CONSENSUS_CLIENT_ID: ConsensusClientId = *b"PLGN"; // TODO: Change to Polygon Consensus ID

pub struct PolygonClient<H: IsmpHost>(core::marker::PhantomData<H>);

impl<H: IsmpHost> Default for PolygonClient<H> {
	fn default() -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<H: IsmpHost + Send + Sync + Default + 'static> ConsensusClient for PolygonClient<H> {
	fn verify_consensus(
		&self,
		_host: &dyn IsmpHost,
		_consensus_state_id: ConsensusStateId,
		_trusted_consensus_state: Vec<u8>,
		_proof: Vec<u8>,
	) -> Result<(Vec<u8>, VerifiedCommitments), Error> {
		Err(Error::Custom("Not implemented".to_string()))
	}

	fn verify_fraud_proof(
		&self,
		_host: &dyn IsmpHost,
		_trusted_consensus_state: Vec<u8>,
		_proof_1: Vec<u8>,
		_proof_2: Vec<u8>,
	) -> Result<(), Error> {
		Err(Error::Custom("fraud proof verification unimplemented".to_string()))
	}

	fn consensus_client_id(&self) -> ConsensusClientId {
		POLYGON_CONSENSUS_CLIENT_ID
	}

	fn state_machine(&self, _id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
		Err(Error::Custom("State machine not supported".to_string()))
	}
}
