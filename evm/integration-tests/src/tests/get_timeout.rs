use polkadot_sdk::*;
use super::utils;
use crate::Keccak256;
use ethers::abi::{AbiEncode, Address, Token, Tokenizable};
use forge_testsuite::Runner;
use hex_literal::hex;
use ismp::{
	host::StateMachine,
	messaging::hash_request,
	router::{self, Request},
};
use ismp_solidity_abi::{
	beefy::{IntermediateState, StateCommitment, StateMachineHeight},
	handler::GetTimeoutMessage,
	shared_types::GetRequest,
};
use primitive_types::H256;
use sp_core::U256;
use sp_trie::StorageProof;
use std::{env, path::PathBuf};

#[tokio::test(flavor = "multi_thread")]
async fn test_get_timeout() -> Result<(), anyhow::Error> {
	let base_dir = env::current_dir()?.parent().unwrap().display().to_string();
	let mut runner = Runner::new(PathBuf::from(&base_dir));
	let mut contract = runner.deploy("GetRequestTest").await;

	let destination = contract.call::<_, Address>("module", ()).await?;
	let storage_prefix = hex!("526571756573745265636569707473").to_vec();

	let key = H256::random().as_bytes().to_vec();

	// create post request object
	let get = router::GetRequest {
		dest: StateMachine::Polkadot(2000),
		source: StateMachine::Evm(1),
		nonce: 0,
		from: destination.as_bytes().to_vec(),
		keys: vec![key.clone()],
		context: Default::default(),
		timeout_timestamp: 100,
		height: 0,
	};

	let commitment = hash_request::<Keccak256>(&Request::Get(get.clone()));

	let mut key = storage_prefix.clone();
	key.extend_from_slice(commitment.as_ref());

	let (root, proof) =
		utils::generate_non_membership_proof(storage_prefix, vec![key.clone()], false);
	let storage_proof = StorageProof::new(proof.clone().into_iter());
	let result = utils::read_proof_check(&root, storage_proof, vec![key.as_slice()]).unwrap();

	// The value should be None since it's a None membership proof
	assert!(result.get(&key).unwrap().is_none());

	// create intermediate state
	let height = StateMachineHeight { state_machine_id: U256::from(2000), height: U256::from(10) };
	let consensus_proof = IntermediateState {
		state_machine_id: height.state_machine_id,
		height: height.height,
		commitment: StateCommitment {
			timestamp: U256::from(20_000),
			overlay_root: [0u8; 32],
			state_root: root.0,
		},
	}
	.encode();

	let mut sol_get: GetRequest = get.into();

	let message = GetTimeoutMessage {
		proof: proof.into_iter().map(Into::into).collect(),
		timeouts: vec![sol_get.clone()],
		height,
	};
	sol_get.timeout_timestamp -= 1;

	// execute the test
	contract
		.call::<_, ()>(
			"GetTimeoutNoChallenge",
			(Token::Bytes(consensus_proof), sol_get.into_token(), message.into_token()),
		)
		.await?;

	Ok(())
}
