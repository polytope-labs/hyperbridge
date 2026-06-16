use super::utils::*;
use crate::Keccak256;
use alloy_primitives::{FixedBytes, U256};
use hex_literal::hex;
use ismp::{
	host::StateMachine,
	messaging::hash_request,
	router::{self, Request},
};
use ismp_abi::handler::PostRequestTimeoutMessage;
use polkadot_sdk::*;
use sp_trie::StorageProof;

#[test]
fn test_post_timeout_proof() {
	let mut env = TestEnv::new();

	let module = env.test_module;
	let storage_prefix = hex!("526571756573745265636569707473").to_vec();

	let post = router::PostRequest {
		source: StateMachine::Evm(1),
		dest: StateMachine::Polkadot(2000),
		nonce: 0,
		from: module.as_slice().to_vec(),
		to: module.as_slice().to_vec(),
		timeout_timestamp: 10_000,
		body: storage_prefix.clone(),
	};

	let commitment = hash_request::<Keccak256>(&Request::Post(post.clone()));
	let mut key = storage_prefix.clone();
	key.extend_from_slice(commitment.as_ref());

	let (root, proof) = generate_non_membership_proof(storage_prefix, vec![key.clone()], false);
	let storage_proof = StorageProof::new(proof.clone().into_iter());
	let result = read_proof_check(&root, storage_proof, vec![key.as_slice()]).unwrap();

	// The value should be None since it's a non-membership proof
	assert!(result.get(&key).unwrap().is_none());

	let timeout_height = ismp_abi::handler::StateMachineHeight {
		stateMachineId: U256::from(2000),
		height: U256::from(10),
	};
	let consensus_proof = TestEnv::encode_consensus_proof(
		timeout_height.stateMachineId,
		timeout_height.height,
		U256::from(20_000),
		[0u8; 32],
		root.0,
		U256::ZERO,
	);

	let mut sol_post: ismp_abi::evm_host::EvmHost::PostRequest = post.clone().into();

	let message = PostRequestTimeoutMessage {
		proof: proof.into_iter().map(Into::into).collect(),
		timeouts: vec![to_handler_post_request(sol_post.clone())],
		height: timeout_height,
	};
	sol_post.timeoutTimestamp -= 1;

	// Mint tokens and approve for dispatch
	env.mint_fee_token(env.sender, U256::from(1_000_000_000u128) * U256::from(10u128.pow(18)));
	env.approve_fee_token(env.test_module, U256::MAX);

	// dispatch the post request
	env.dispatch_post_request(sol_post);

	// handle consensus + timeout
	env.handle_consensus(consensus_proof);
	env.warp(5000);
	env.handle_post_request_timeouts(message);
}
