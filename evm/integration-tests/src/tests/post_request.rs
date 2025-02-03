use polkadot_sdk::*;
use crate::tests::utils::initialize_mmr_tree;
use ethers::{
	abi::{AbiEncode, Address, Token, Tokenizable},
	core::types::U256,
};
use forge_testsuite::Runner;
use ismp::{
	host::StateMachine,
	router::{PostRequest, Request},
};
use ismp_solidity_abi::{
	beefy::IntermediateState,
	handler::{PostRequestLeaf, PostRequestMessage},
	shared_types::StateCommitment,
};
use mmr_primitives::DataOrHash;
use pallet_ismp::offchain::Leaf;
use std::{env, path::PathBuf};

#[tokio::test(flavor = "multi_thread")]
async fn test_post_request_proof() -> Result<(), anyhow::Error> {
	let base_dir = env::current_dir()?.parent().unwrap().display().to_string();
	let mut runner = Runner::new(PathBuf::from(&base_dir));
	let mut contract = runner.deploy("PostRequestTest").await;
	let destination = contract.call::<_, Address>("module", ()).await?;

	// create post request object
	let post = PostRequest {
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

	// create intermediate state
	let consensus_proof = IntermediateState {
		state_machine_id: proof.height.state_machine_id,
		height: proof.height.height,
		commitment: StateCommitment {
			timestamp: U256::from(20000),
			overlay_root,
			state_root: [0u8; 32],
		},
	}
	.encode();

	let message = PostRequestMessage {
		proof,
		requests: vec![PostRequestLeaf {
			request: post.into(),
			index: 30.into(),
			k_index: k_index.into(),
		}],
	};

	// execute the test
	contract
		.call::<_, ()>(
			"PostRequestNoChallengeNoTimeout",
			(Token::Bytes(consensus_proof), message.into_token()),
		)
		.await?;

	Ok(())
}
