use polkadot_sdk::*;
use super::utils;
use crate::Keccak256;
use ethers::{
	abi::{AbiEncode, Address, Token, Tokenizable},
	core::types::U256,
};
use forge_testsuite::Runner;
use hex_literal::hex;
use ismp::{
	host::StateMachine,
	messaging::hash_request,
	router::{self, Request},
};
use ismp_solidity_abi::{
	beefy::IntermediateState,
	handler::PostRequestTimeoutMessage,
	shared_types::{PostRequest, StateCommitment, StateMachineHeight},
};
use sp_trie::StorageProof;
use std::{env, path::PathBuf};

#[tokio::test(flavor = "multi_thread")]
async fn test_post_timeout_proof() -> Result<(), anyhow::Error> {
	let base_dir = env::current_dir()?.parent().unwrap().display().to_string();
	let mut runner = Runner::new(PathBuf::from(&base_dir));
	let mut contract = runner.deploy("PostRequestTest").await;

	let module = contract.call::<_, Address>("module", ()).await?;
	let storage_prefix = hex!("526571756573745265636569707473").to_vec();

	// create post request object
	let post = router::PostRequest {
		source: StateMachine::Evm(1),
		dest: StateMachine::Polkadot(2000),
		nonce: 0,
		from: module.as_bytes().to_vec(),
		to: module.as_bytes().to_vec(),
		timeout_timestamp: 10_000,
		body: storage_prefix.clone(),
	};

	let commitment = hash_request::<Keccak256>(&Request::Post(post.clone()));
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

	let mut sol_post: PostRequest = post.clone().into();

	let message = PostRequestTimeoutMessage {
		proof: proof.into_iter().map(Into::into).collect(),
		timeouts: vec![sol_post.clone()],
		height,
	};
	sol_post.timeout_timestamp -= 1;

	// execute the test
	contract
		.call::<_, ()>(
			"PostRequestTimeoutNoChallenge",
			(Token::Bytes(consensus_proof), sol_post.into_token(), message.into_token()),
		)
		.await?;

	Ok(())
}
