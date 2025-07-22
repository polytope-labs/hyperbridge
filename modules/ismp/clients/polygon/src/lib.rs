#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_variables)]

extern crate alloc;

use alloc::{boxed::Box, string::ToString, vec::Vec};
use base64::{engine::general_purpose::STANDARD as base64_engine, Engine};
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateMachineClient,
		VerifiedCommitments,
	},
	error::Error,
	host::{IsmpHost, StateMachine},
};

use prost::Message;
use tendermint_primitives::{ConsensusProof, Milestone, TrustedState, VerificationOptions};
use tendermint_prover::{prove_header_update, Client, HeimdallClient};

/// The consensus update/proof for Polygon
#[derive(Debug, Clone)]
pub struct PolygonConsensusUpdate {
	/// Serialized Tendermint light client update (signed header, validator set, etc.)
	pub tendermint_proof: ConsensusProof,
	/// EVM block header for the milestone's end block
	pub evm_header: geth_primitives::Header,
	/// Milestone number
	pub milestone_number: u64,
	/// ICS23 proof for the milestone inclusion
	pub ics23_state_proof: Vec<u8>,
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

pub async fn polygon_consensus_update() -> Result<PolygonConsensusUpdate, Error> {
	// TODO: Cleanup urls
	let heimdall_rpc_url = "";
	let heimdall_rest_url = "";
	let execution_rpc_url = "";

	let client = HeimdallClient::new(heimdall_rpc_url, heimdall_rest_url, execution_rpc_url)
		.map_err(|e| Error::Custom("PolygonProver: create HeimdallClient failed".to_string()))?;

	let (milestone_number, milestone) = client
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
	let evm_header_hash = evm_header_struct.hash();
	if evm_header_hash.as_bytes() != hash_bytes.as_slice() {
		return Err(Error::Custom("PolygonProver: EVM header hash mismatch".to_string()));
	}

	let latest_height = client
		.latest_height()
		.await
		.map_err(|_| Error::Custom("PolygonProver: fetch latest height failed".to_string()))?;

	let abci_query = client
		.get_ics23_proof(milestone_number, latest_height)
		.await
		.map_err(|_| Error::Custom("PolygonProver: fetch ICS23 proof failed".to_string()))?;

	// ICS23 proof verification

	let proof_bytes = abci_query
		.proof
		.as_ref()
		.ok_or_else(|| Error::Custom("PolygonProver: no proof in abci_query".to_string()))?
		.ops
		.get(0)
		.ok_or_else(|| Error::Custom("PolygonProver: no ops in proof".to_string()))?
		.data
		.as_slice();

	let commitment_proof = ics23::CommitmentProof::decode(proof_bytes).map_err(|_| {
		Error::Custom("PolygonProver: failed to decode CommitmentProof".to_string())
	})?;

	let spec = ics23::tendermint_spec();

	let tendermint_header = client
		.signed_header(end_block)
		.await
		.map_err(|_| Error::Custom("PolygonProver: fetch Signed header failed".to_string()))?;

	let root = tendermint_header.header.app_hash.as_bytes().to_vec().into();

	let verification_result = ics23::verify_membership(
		&commitment_proof,
		&spec,
		&root,
		&abci_query.key,
		&abci_query.value,
	);

	if !verification_result {
		return Err(Error::Custom("PolygonProver: ICS23 proof verification failed".to_string()));
	}

	// Consensus Proof generation

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
		ics23_state_proof: proof_bytes.to_vec(),
		milestone_number,
		evm_header: evm_header_struct,
	};

	Ok(update)
}
