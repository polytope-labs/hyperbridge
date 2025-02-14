use polkadot_sdk::*;
use crate::tests::utils::initialize_mmr_tree;
use ethers::{
	abi::{AbiEncode, Address, Token, Tokenizable},
	core::types::U256,
};
use forge_testsuite::Runner;
use ismp::{
	host::StateMachine,
	router::{self, Response, StorageValue},
};
use ismp_solidity_abi::{
	beefy::{IntermediateState, StateCommitment, StateMachineHeight},
	handler::{GetResponseLeaf, GetResponseMessage},
	shared_types::GetRequest,
};
use mmr_primitives::DataOrHash;
use pallet_ismp::offchain::Leaf;
use primitive_types::H256;
use std::{env, path::PathBuf};

#[tokio::test(flavor = "multi_thread")]
async fn test_get_response() -> Result<(), anyhow::Error> {
	let base_dir = env::current_dir()?.parent().unwrap().display().to_string();
	let mut runner = Runner::new(PathBuf::from(&base_dir));
	let mut contract = runner.deploy("GetRequestTest").await;
	let destination = contract.call::<_, Address>("module", ()).await?;

	let key = H256::random().as_bytes().to_vec();

	// create post request object
	let get = router::GetRequest {
		dest: StateMachine::Polkadot(2000),
		source: StateMachine::Evm(1),
		nonce: 0,
		from: destination.as_bytes().to_vec(),
		keys: vec![key.clone()],
		timeout_timestamp: 100,
		context: Default::default(),
		height: 0,
	};

	let values = vec![StorageValue { key, value: Some(H256::random().as_bytes().to_vec()) }];
	let response = router::GetResponse { get: get.clone(), values };

	let leaf = DataOrHash::Data(Leaf::Response(Response::Get(response.clone())));
	let (overlay_root, proof, k_index) = initialize_mmr_tree(leaf, 10)?;

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

	let mut sol_get: GetRequest = get.into();

	let message = GetResponseMessage {
		proof,
		responses: vec![GetResponseLeaf {
			index: 30.into(),
			k_index: k_index.into(),
			response: response.into(),
		}],
	};

	sol_get.timeout_timestamp -= 1;

	// execute the test
	contract
		.call::<_, ()>(
			"GetResponseNoChallengeNoTimeout",
			(Token::Bytes(consensus_proof), sol_get.into_token(), message.into_token()),
		)
		.await?;

	Ok(())
}
