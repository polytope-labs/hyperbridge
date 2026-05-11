use super::utils::*;
use crate::Keccak256;
use alloy_primitives::U256;
use ismp::{
	host::StateMachine,
	messaging::hash_request,
	router::{PostRequest, Request},
};
use ismp_solidity_abi::handler::{PostRequestLeaf, PostRequestMessage};
use mmr_primitives::DataOrHash;
use pallet_ismp::offchain::Leaf;
use polkadot_sdk::*;

#[test]
fn test_post_request_proof() {
	let mut env = TestEnv::new();

	let post = PostRequest {
		source: StateMachine::Polkadot(2000),
		dest: StateMachine::Evm(1),
		nonce: 0,
		from: env.sender.as_slice().to_vec(),
		to: env.test_module.as_slice().to_vec(),
		timeout_timestamp: 100,
		body: vec![],
	};
	let request = DataOrHash::Data(Leaf::Request(Request::Post(post.clone())));
	let (overlay_root, proof) = initialize_mmr_tree(request, 10).unwrap();

	let consensus_proof = TestEnv::encode_consensus_proof(
		U256::from(2000),
		U256::from(10),
		U256::from(20000),
		overlay_root,
		[0u8; 32],
		U256::ZERO,
	);

	let message = PostRequestMessage {
		proof,
		requests: vec![PostRequestLeaf { request: post.clone().into(), index: U256::from(30) }],
	};

	env.handle_consensus(consensus_proof);
	env.warp(10);
	env.handle_post_requests(message);

	// verify request was acknowledged
	let commitment = hash_request::<Keccak256>(&Request::Post(post));
	assert_ne!(env.request_receipt(commitment.0), alloy_primitives::Address::ZERO);
}
