use super::utils::*;
use crate::{Keccak256, Mmr};
use alloy_primitives::{FixedBytes, U256};
use alloy_sol_types::SolValue;
use hex_literal::hex;
use ismp::{
	host::StateMachine,
	messaging::hash_response,
	router::{self, Request, GetResponse},
};
use ismp_abi::{
	evm_host::EvmHost::StateMachineHeight,
	handler::{
		PostRequestLeaf, PostRequestMessage, PostResponseLeaf, PostResponseMessage,
		PostResponseTimeoutMessage, Proof,
	},
};
use mmr_primitives::DataOrHash;
use pallet_ismp::offchain::Leaf;
use polkadot_sdk::*;
use primitive_types::H256;

#[test]
fn test_post_response_proof() {
	let mut env = TestEnv::new();

	let module = env.test_module;

	// create post request object
	let post = router::PostRequest {
		source: StateMachine::Evm(1),
		dest: StateMachine::Polkadot(2000),
		nonce: 0,
		from: module.as_slice().to_vec(),
		to: module.as_slice().to_vec(),
		timeout_timestamp: 30,
		body: vec![2u8; 32],
	};

	let post_response =
		router::PostResponse { post: post.clone(), response: vec![1u8; 64], timeout_timestamp: 0 };
	let response = DataOrHash::Data(Leaf::GetResponse(router::GetResponse::Post(post_response.clone())));

	// create the mmr tree and insert it
	let mut mmr = Mmr::default();
	let leaf_count = 30u64;

	for _ in 0..leaf_count {
		let hash = H256::random();
		mmr.push(DataOrHash::Hash(hash)).unwrap();
	}

	let pos = mmr.push(response).unwrap();

	let proof = mmr.gen_proof(vec![pos]).unwrap();
	let overlay_root = mmr.get_root().unwrap().hash().0;
	let multiproof: Vec<FixedBytes<32>> =
		proof.proof_items().iter().map(|h| FixedBytes(h.hash().0)).collect();

	// create consensus proof
	let height = ismp_abi::handler::StateMachineHeight {
		stateMachineId: U256::from(2000),
		height: U256::from(10),
	};
	let consensus_proof = TestEnv::encode_consensus_proof(
		height.stateMachineId,
		height.height,
		U256::from(20000),
		overlay_root,
		[0u8; 32],
		U256::ZERO,
	);

	let mut sol_post: ismp_abi::evm_host::EvmHost::PostRequest =
		post_response.post.clone().into();

	let message = PostResponseMessage {
		proof: Proof { height, multiproof, leafCount: U256::from(leaf_count + 1) },
		responses: vec![PostResponseLeaf {
			response: post_response.into(),
			index: U256::from(leaf_count),
		}],
	};

	sol_post.timeoutTimestamp -= 1;

	// dispatch the request first so the host knows about it
	env.dispatch_post_request(sol_post);

	// handle consensus + response
	env.handle_consensus(consensus_proof);
	env.warp(10);
	env.handle_post_responses(message);
}

#[test]
fn test_post_response_timeout() {
	let mut env = TestEnv::new();

	let storage_prefix = hex!("526573706f6e73655265636569707473").to_vec();
	let destination = env.test_module;

	// create post request object
	let post = router::PostRequest {
		source: StateMachine::Polkadot(2000),
		dest: StateMachine::Evm(1),
		nonce: 0,
		from: env.sender.as_slice().to_vec(),
		to: destination.as_slice().to_vec(),
		timeout_timestamp: 100,
		body: vec![],
	};
	let request = DataOrHash::Data(Leaf::Request(Request::Post(post.clone())));
	let (overlay_root, proof) = initialize_mmr_tree(request, 10).unwrap();

	// consensus proof 1 - for the request
	let consensus_proof_1 = TestEnv::encode_consensus_proof(
		U256::from(2000),
		U256::from(10),
		U256::from(100),
		overlay_root,
		[0u8; 32],
		U256::ZERO,
	);

	let request_message = PostRequestMessage {
		proof,
		requests: vec![PostRequestLeaf { request: post.clone().into(), index: U256::from(30) }],
	};

	let response = router::PostResponse { post, response: vec![], timeout_timestamp: 200 };
	let commitment = hash_response::<Keccak256>(&Response::Post(response.clone()));
	let mut key = storage_prefix.clone();
	key.extend_from_slice(commitment.as_ref());
	let (root, proof) = generate_non_membership_proof(storage_prefix, vec![key.clone()], false);

	// consensus proof 2 - for the timeout
	let consensus_proof_2 = TestEnv::encode_consensus_proof(
		U256::from(2000),
		U256::from(20),
		U256::from(300), // expired
		[0u8; 32],
		root.0,
		U256::ZERO,
	);

	let timeout_height = ismp_abi::handler::StateMachineHeight {
		stateMachineId: U256::from(2000),
		height: U256::from(20),
	};
	let timeout = PostResponseTimeoutMessage {
		timeouts: vec![response.clone().into()],
		height: timeout_height,
		proof: proof.into_iter().map(|p| p.into()).collect(),
	};

	// Mint tokens and set up for the test
	env.mint_fee_token(env.sender, U256::from(1_000_000_000u128) * U256::from(10u128.pow(18)));
	env.approve_fee_token(env.test_module, U256::MAX);

	// Step 1: handle consensus + request
	env.handle_consensus(consensus_proof_1);
	env.warp(10);
	env.handle_post_requests(request_message);

	// Step 2: dispatch the response
	// Adjust timeout relative to block.timestamp (host will add block.timestamp back)
	let mut adjusted_response = response.clone();
	let block_ts = env.block_timestamp();
	adjusted_response.timeout_timestamp =
		adjusted_response.timeout_timestamp.saturating_sub(block_ts);
	let sol_response: ismp_abi::evm_host::EvmHost::PostResponse = adjusted_response.into();
	env.dispatch_post_response(sol_response);

	// verify we know this response (host stores with original timeout after adding block.timestamp)
	let response_commitment = hash_response::<Keccak256>(&Response::Post(response.clone()));
	assert_ne!(
		env.response_commitment(response_commitment.0).sender,
		alloy_primitives::Address::ZERO
	);

	// Step 3: handle consensus + timeout
	env.handle_consensus(consensus_proof_2);
	env.warp(10);
	env.handle_post_response_timeouts(timeout);

	// verify response no longer exists
	assert_eq!(
		env.response_commitment(response_commitment.0).sender,
		alloy_primitives::Address::ZERO
	);
}

#[test]
fn test_post_response_malicious_timeout() {
	let mut env = TestEnv::new();

	let storage_prefix = hex!("526573706f6e73655265636569707473").to_vec();
	let destination = env.test_module;

	let post = router::PostRequest {
		source: StateMachine::Polkadot(2000),
		dest: StateMachine::Evm(1),
		nonce: 0,
		from: env.sender.as_slice().to_vec(),
		to: destination.as_slice().to_vec(),
		timeout_timestamp: 100,
		body: vec![],
	};
	let request = DataOrHash::Data(Leaf::Request(Request::Post(post.clone())));
	let (overlay_root, proof) = initialize_mmr_tree(request, 10).unwrap();

	let consensus_proof_1 = TestEnv::encode_consensus_proof(
		U256::from(2000),
		U256::from(10),
		U256::from(100),
		overlay_root,
		[0u8; 32],
		U256::ZERO,
	);

	let request_message = PostRequestMessage {
		proof,
		requests: vec![PostRequestLeaf { request: post.clone().into(), index: U256::from(30) }],
	};

	let response = router::PostResponse { post, response: vec![], timeout_timestamp: 200 };
	let commitment = hash_response::<Keccak256>(&Response::Post(response.clone()));
	let mut key = storage_prefix.clone();
	key.extend_from_slice(commitment.as_ref());
	// insert_keys = true -> proof actually contains the response (malicious)
	let (root, proof) = generate_non_membership_proof(storage_prefix, vec![key.clone()], true);

	let consensus_proof_2 = TestEnv::encode_consensus_proof(
		U256::from(2000),
		U256::from(20),
		U256::from(300),
		[0u8; 32],
		root.0,
		U256::ZERO,
	);

	let timeout_height = ismp_abi::handler::StateMachineHeight {
		stateMachineId: U256::from(2000),
		height: U256::from(20),
	};
	let timeout = PostResponseTimeoutMessage {
		timeouts: vec![response.clone().into()],
		height: timeout_height,
		proof: proof.into_iter().map(|p| p.into()).collect(),
	};

	env.mint_fee_token(env.sender, U256::from(1_000_000_000u128) * U256::from(10u128.pow(18)));
	env.approve_fee_token(env.test_module, U256::MAX);

	env.handle_consensus(consensus_proof_1);
	env.warp(10);
	env.handle_post_requests(request_message);

	let sol_response: ismp_abi::evm_host::EvmHost::PostResponse = response.into();
	env.dispatch_post_response(sol_response);

	env.handle_consensus(consensus_proof_2);
	env.warp(10);

	// The malicious timeout should revert because the proof contains the response
	use alloy_sol_types::SolCall;
	let calldata = ismp_abi::handler::handlePostResponseTimeoutsCall {
		host: env.host,
		message: timeout,
	}
	.abi_encode();
	assert!(env.call_reverts(env.handler, calldata));
}
