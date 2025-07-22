#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_variables)]

extern crate alloc;

use alloc::{boxed::Box, format, string::ToString, vec::Vec};
use base64::{engine::general_purpose::STANDARD as base64_engine, Engine};
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateMachineClient,
		VerifiedCommitments,
	},
	error::Error,
	host::{IsmpHost, StateMachine},
};

use tendermint_primitives::{ConsensusProof, Milestone, TrustedState, VerificationOptions};
use tendermint_prover::{prove_header_update, Client, HeimdallClient};

/// The consensus update/proof for Polygon
#[derive(Debug, Clone)]
pub struct PolygonConsensusUpdate {
	/// Serialized Tendermint light client update (signed header, validator set, etc.)
	pub tendermint_proof: ConsensusProof,
	/// The milestone object (decoded from protobuf)
	pub milestone: Milestone,
	/// ICS23 proof for the milestone inclusion
	pub milestone_proof: Vec<u8>,
	/// Heimdall block height at which the proof is made
	pub milestone_height: u64,
	/// EVM block header for the milestone's end block (concrete type)
	pub evm_header: geth_primitives::Header,
}

/// The trusted consensus state for Polygon
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusState {
	/// Trusted Tendermint state (serialized)
	pub tendermint_state: Vec<u8>,
	/// Last known milestone
	pub last_milestone: Milestone,
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

pub async fn polygon_consensus_update() -> Result<(Vec<u8>, VerifiedCommitments), Error> {
	let heimdall_rpc_url = "";
	let heimdall_rest_url = "";
	let execution_rpc_url = "";

	let client = HeimdallClient::new(heimdall_rpc_url, heimdall_rest_url, execution_rpc_url)
		.map_err(|e| Error::Custom("PolygonProver: create HeimdallClient failed".to_string()))?;

	let milestone = client
		.get_latest_milestone()
		.await
		.map_err(|_| Error::Custom("PolygonProver: fetch milestone failed".to_string()))?;

	let hash_bytes = base64_engine
		.decode(&milestone.hash)
		.map_err(|_| Error::Custom("PolygonProver: decode milestone hash failed".to_string()))?;
	let hash_hex = hex::encode(&hash_bytes);

	let end_block = milestone
		.end_block
		.parse::<u64>()
		.map_err(|_| Error::Custom("PolygonProver: parse end_block failed".to_string()))?;

	let evm_header = client
		.fetch_header(end_block)
		.await
		.map_err(|_| Error::Custom("PolygonProver: fetch EVM header failed".to_string()))?;
	let evm_header = evm_header
		.ok_or_else(|| Error::Custom("PolygonProver: EVM header not found".to_string()))?;

	let evm_header_struct: geth_primitives::Header = (&evm_header).into();
	let evm_header_hash = evm_header_struct.hash(); // TODO: Add type param if needed
	if evm_header_hash.as_bytes() != hash_bytes.as_slice() {
		return Err(Error::Custom("PolygonProver: EVM header hash mismatch".to_string()));
	}

	let abci_query = client
		.get_ics23_proof()
		.await
		.map_err(|_| Error::Custom("PolygonProver: fetch ICS23 proof failed".to_string()))?;
	let milestone_proof = abci_query.proof.map(|p| p.ops).unwrap_or_default();
	let milestone_proof_bytes = bincode::serialize(&milestone_proof)
		.map_err(|_| Error::Custom("PolygonProver: serialize ICS23 proof failed".to_string()))?;

	let chain_id = client
		.chain_id()
		.await
		.map_err(|_| Error::Custom("PolygonProver: fetch chain ID failed".to_string()))?;
	let latest_height = client
		.latest_height()
		.await
		.map_err(|_| Error::Custom("PolygonProver: fetch latest height failed".to_string()))?;

	let trusted_height = latest_height.saturating_sub(50);
	let trusted_header = client.signed_header(trusted_height).await.map_err(|_| {
		Error::Custom("PolygonProver: fetch trusted signed header failed".to_string())
	})?;
	let trusted_validators = client
		.validators(trusted_height)
		.await
		.map_err(|_| Error::Custom("PolygonProver: fetch trusted validators failed".to_string()))?;
	let trusted_next_validators = client.next_validators(trusted_height).await.map_err(|_| {
		Error::Custom("PolygonProver: fetch trusted next validators failed".to_string())
	})?;

	let mut trusted_state = TrustedState::new(
		chain_id,
		trusted_height,
		trusted_header.header.time.unix_timestamp() as u64,
		trusted_header.header.hash().as_bytes().try_into().unwrap(),
		trusted_validators,
		trusted_next_validators,
		trusted_header.header.next_validators_hash.as_bytes().try_into().unwrap(),
		7200, // 2 hour trusting period
		VerificationOptions::default(),
	);

	let target_height = milestone.end_block.parse::<u64>().map_err(|_| {
		Error::Custom("PolygonProver: parse end_block for target_height failed".to_string())
	})?;
	let consensus_proof = prove_header_update(&client, &trusted_state, target_height)
		.await
		.map_err(|_| Error::Custom("PolygonProver: prove header update failed".to_string()))?;

	let update = PolygonConsensusUpdate {
		tendermint_proof: consensus_proof,
		milestone: milestone.clone(),
		milestone_proof: milestone_proof_bytes,
		milestone_height: milestone.end_block.parse().unwrap_or(0),
		evm_header: evm_header_struct,
	};

	// TODO: Update the returns
	Ok((
		bincode::serialize(&update)
			.map_err(|_| Error::Custom("PolygonProver: serialize update failed".to_string()))?,
		VerifiedCommitments::default(),
	))
}
