use polkadot_sdk::*;
use crate::{
	tests::{utils, utils::initialize_mmr_tree},
	Keccak256, Mmr,
};
use ethers::{
	abi::{AbiEncode, Address, Token, Tokenizable},
	core::types::U256,
};
use forge_testsuite::Runner;
use hex_literal::hex;
use ismp::{
	host::StateMachine,
	messaging::hash_response,
	router::{self, Request, Response},
};
use ismp_solidity_abi::{
	beefy::IntermediateState,
	handler::{
		PostRequestLeaf, PostRequestMessage, PostResponseLeaf, PostResponseMessage,
		PostResponseTimeoutMessage, Proof,
	},
	shared_types::{PostRequest, PostResponse, StateCommitment, StateMachineHeight},
};

use mmr_primitives::DataOrHash;
use pallet_ismp::offchain::Leaf;
use primitive_types::H256;
use std::{env, path::PathBuf};

#[tokio::test(flavor = "multi_thread")]
async fn test_post_response_proof() -> Result<(), anyhow::Error> {
	let base_dir = env::current_dir()?.parent().unwrap().display().to_string();
	let mut runner = Runner::new(PathBuf::from(&base_dir));
	let mut contract = runner.deploy("PostResponseTest").await;

	let module = contract.call::<_, Address>("module", ()).await?;

	// create post request object
	let post = router::PostRequest {
		source: StateMachine::Evm(1),
		dest: StateMachine::Polkadot(2000),
		nonce: 0,
		from: module.as_bytes().to_vec(),
		to: module.as_bytes().to_vec(),
		timeout_timestamp: 30,
		body: vec![2u8; 32],
	};

	let post_response =
		router::PostResponse { post: post.clone(), response: vec![1u8; 64], timeout_timestamp: 0 };
	let response = DataOrHash::Data(Leaf::Response(router::Response::Post(post_response.clone())));

	// create the mmr tree and insert it
	let mut mmr = Mmr::default();
	let leaf_count = 30;

	for _ in 0..leaf_count {
		let hash = H256::random();
		mmr.push(DataOrHash::Hash(hash))?;
	}

	let pos = mmr.push(response)?;
	let k_index = mmr_primitives::mmr_position_to_k_index(vec![pos], mmr.mmr_size())[0].1;

	let proof = mmr.gen_proof(vec![pos])?;
	let overlay_root = mmr.get_root()?.hash().0;
	let multiproof = proof.proof_items().iter().map(|h| h.hash().0).collect();

	// create intermediate state
	let height = StateMachineHeight { state_machine_id: U256::from(2000), height: U256::from(10) };
	let consensus_proof = IntermediateState {
		state_machine_id: height.state_machine_id,
		height: height.height,
		commitment: StateCommitment {
			timestamp: U256::from(20000),
			overlay_root,
			state_root: [0u8; 32],
		},
	}
	.encode();

	let mut sol_post: PostRequest = post_response.post.clone().into();

	let message = PostResponseMessage {
		proof: Proof { height, multiproof, leaf_count: (leaf_count + 1).into() },
		responses: vec![PostResponseLeaf {
			response: post_response.into(),
			index: leaf_count.into(),
			k_index: k_index.into(),
		}],
	};

	sol_post.timeout_timestamp -= 1;

	// execute the test
	contract
		.call::<_, ()>(
			"PostResponseNoChallengeNoTimeout",
			(Token::Bytes(consensus_proof), sol_post.into_token(), message.into_token()),
		)
		.await?;

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_post_response_timeout() -> Result<(), anyhow::Error> {
	let base_dir = env::current_dir()?.parent().unwrap().display().to_string();
	let mut runner = Runner::new(PathBuf::from(&base_dir));
	let mut contract = runner.deploy("PostResponseTest").await;
	let storage_prefix = hex!("526573706f6e73655265636569707473").to_vec();
	let destination = contract.call::<_, Address>("module", ()).await?;

	// create post request object
	let post = router::PostRequest {
		source: StateMachine::Polkadot(2000),
		dest: StateMachine::Evm(1),
		nonce: 0,
		from: contract.runner.sender.as_bytes().to_vec(),
		to: destination.as_bytes().to_vec(),
		timeout_timestamp: 100,
		body: vec![],
	};
	let request = DataOrHash::Data(Leaf::Request(Request::Post(post.clone())));
	let (overlay_root, proof, k_index) = initialize_mmr_tree(request, 10)?;

	// create intermediate state 1
	let consensus_proof_1 = IntermediateState {
		state_machine_id: proof.height.state_machine_id,
		height: proof.height.height,
		commitment: StateCommitment {
			timestamp: U256::from(100),
			overlay_root,
			state_root: [0u8; 32],
		},
	}
	.encode();

	// request message
	let request_message = PostRequestMessage {
		proof,
		requests: vec![PostRequestLeaf {
			request: post.clone().into(),
			index: 30.into(),
			k_index: k_index.into(),
		}],
	};

	let response = router::PostResponse { post, response: vec![], timeout_timestamp: 200 };
	let commitment = hash_response::<Keccak256>(&Response::Post(response.clone()));
	let mut key = storage_prefix.clone();
	key.extend_from_slice(commitment.as_ref());
	let (root, proof) =
		utils::generate_non_membership_proof(storage_prefix, vec![key.clone()], false);

	// create intermediate state
	let height = StateMachineHeight { state_machine_id: U256::from(2000), height: U256::from(20) };
	let consensus_proof_2 = IntermediateState {
		state_machine_id: height.state_machine_id,
		height: height.height,
		commitment: StateCommitment {
			timestamp: U256::from(300), // expired
			overlay_root: [0u8; 32],
			state_root: root.0, // todo
		},
	}
	.encode();

	let timeout = PostResponseTimeoutMessage {
		timeouts: vec![response.clone().into()],
		height,
		proof: proof.into_iter().map(Into::into).collect(),
	};

	let sol_response: PostResponse = response.into();

	// execute the test
	contract
		.call::<_, ()>(
			"PostResponseTimeoutNoChallenge",
			(
				Token::Bytes(consensus_proof_1),
				Token::Bytes(consensus_proof_2),
				request_message.into_token(),
				sol_response.into_token(),
				timeout.into_token(),
			),
		)
		.await?;

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_post_response_malicious_timeout() -> Result<(), anyhow::Error> {
	let base_dir = env::current_dir()?.parent().unwrap().display().to_string();
	let mut runner = Runner::new(PathBuf::from(&base_dir));
	let mut contract = runner.deploy("PostResponseTest").await;
	let storage_prefix = hex!("526573706f6e73655265636569707473").to_vec();
	let destination = contract.call::<_, Address>("module", ()).await?;

	// create post request object
	let post = router::PostRequest {
		source: StateMachine::Polkadot(2000),
		dest: StateMachine::Evm(1),
		nonce: 0,
		from: contract.runner.sender.as_bytes().to_vec(),
		to: destination.as_bytes().to_vec(),
		timeout_timestamp: 100,
		body: vec![],
	};
	let request = DataOrHash::Data(Leaf::Request(Request::Post(post.clone())));
	let (overlay_root, proof, k_index) = initialize_mmr_tree(request, 10)?;

	// create intermediate state 1
	let consensus_proof_1 = IntermediateState {
		state_machine_id: proof.height.state_machine_id,
		height: proof.height.height,
		commitment: StateCommitment {
			timestamp: U256::from(100),
			overlay_root,
			state_root: [0u8; 32],
		},
	}
	.encode();

	// request message
	let request_message = PostRequestMessage {
		proof,
		requests: vec![PostRequestLeaf {
			request: post.clone().into(),
			index: 30.into(),
			k_index: k_index.into(),
		}],
	};

	let response = router::PostResponse { post, response: vec![], timeout_timestamp: 200 };
	let commitment = hash_response::<Keccak256>(&Response::Post(response.clone()));
	let mut key = storage_prefix.clone();
	key.extend_from_slice(commitment.as_ref());
	let (root, proof) =
		utils::generate_non_membership_proof(storage_prefix, vec![key.clone()], true);

	// create intermediate state
	let height = StateMachineHeight { state_machine_id: U256::from(2000), height: U256::from(20) };
	let consensus_proof_2 = IntermediateState {
		state_machine_id: height.state_machine_id,
		height: height.height,
		commitment: StateCommitment {
			timestamp: U256::from(300), // expired
			overlay_root: [0u8; 32],
			state_root: root.0, // todo
		},
	}
	.encode();

	let timeout = PostResponseTimeoutMessage {
		timeouts: vec![response.clone().into()],
		height,
		proof: proof.into_iter().map(Into::into).collect(),
	};

	let sol_response: PostResponse = response.into();

	// execute the test
	contract
		.call::<_, ()>(
			"PostResponseMaliciousTimeoutNoChallenge",
			(
				Token::Bytes(consensus_proof_1),
				Token::Bytes(consensus_proof_2),
				request_message.into_token(),
				sol_response.into_token(),
				timeout.into_token(),
			),
		)
		.await?;

	Ok(())
}
