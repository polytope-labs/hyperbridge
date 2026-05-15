use super::utils::*;
use alloy_primitives::{FixedBytes, U256};
use ismp::{
	host::StateMachine,
	router::{self, Response, StorageValue},
};
use ismp_abi::handler::{GetResponseLeaf, GetResponseMessage, Proof};
use mmr_primitives::DataOrHash;
use pallet_ismp::offchain::Leaf;
use polkadot_sdk::*;
use primitive_types::H256;

#[test]
fn test_get_response() {
	let mut env = TestEnv::new();

	let destination = env.test_module;
	let key = H256::random().as_bytes().to_vec();

	let get = router::GetRequest {
		dest: StateMachine::Polkadot(2000),
		source: StateMachine::Evm(1),
		nonce: 0,
		from: destination.as_slice().to_vec(),
		keys: vec![key.clone()],
		timeout_timestamp: 100,
		context: Default::default(),
		height: 0,
	};

	let values = vec![StorageValue { key, value: Some(H256::random().as_bytes().to_vec()) }];
	let response = router::GetResponse { get: get.clone(), values };

	let leaf = DataOrHash::Data(Leaf::Response(Response::Get(response.clone())));
	let (overlay_root, proof) = initialize_mmr_tree(leaf, 10).unwrap();

	let consensus_proof = TestEnv::encode_consensus_proof(
		U256::from(2000),
		U256::from(10),
		U256::from(20000),
		overlay_root,
		[0u8; 32],
		U256::ZERO,
	);

	let mut sol_get: ismp_abi::evm_host::EvmHost::GetRequest = get.into();

	let message = GetResponseMessage {
		proof,
		responses: vec![GetResponseLeaf {
			index: U256::from(30),
			response: to_handler_get_response(response),
		}],
	};

	sol_get.timeoutTimestamp -= 1;

	// Mint tokens for per-byte fee
	env.mint_fee_token(
		env.test_module,
		U256::from(32u64) * U256::from(1_000_000_000_000_000_000u128),
	);

	// dispatch the get request first
	env.dispatch_get_request(sol_get);

	// handle consensus + response
	env.handle_consensus(consensus_proof);
	env.warp(10);
	env.handle_get_responses(message);
}
