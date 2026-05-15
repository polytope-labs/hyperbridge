use super::utils::*;
use crate::Keccak256;
use alloy_primitives::{FixedBytes, U256};
use hex_literal::hex;
use ismp::{
	host::StateMachine,
	messaging::hash_request,
	router::{self, Request},
};
use ismp_abi::handler::GetTimeoutMessage;
use polkadot_sdk::*;
use primitive_types::H256;
use sp_trie::StorageProof;

#[test]
fn test_get_timeout() {
	let mut env = TestEnv::new();

	let destination = env.test_module;
	let storage_prefix = hex!("526571756573745265636569707473").to_vec();

	let key = H256::random().as_bytes().to_vec();

	let get = router::GetRequest {
		dest: StateMachine::Polkadot(2000),
		source: StateMachine::Evm(1),
		nonce: 0,
		from: destination.as_slice().to_vec(),
		keys: vec![key.clone()],
		context: Default::default(),
		timeout_timestamp: 100,
		height: 0,
	};

	let commitment = hash_request::<Keccak256>(&Request::Get(get.clone()));

	let mut trie_key = storage_prefix.clone();
	trie_key.extend_from_slice(commitment.as_ref());

	let (root, proof) =
		generate_non_membership_proof(storage_prefix, vec![trie_key.clone()], false);
	let storage_proof = StorageProof::new(proof.clone().into_iter());
	let result = read_proof_check(&root, storage_proof, vec![trie_key.as_slice()]).unwrap();

	// The value should be None since it's a non-membership proof
	assert!(result.get(&trie_key).unwrap().is_none());

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

	let mut sol_get: ismp_abi::evm_host::EvmHost::GetRequest = get.into();

	let message = GetTimeoutMessage {
		proof: proof.into_iter().map(Into::into).collect(),
		timeouts: vec![to_handler_get_request(sol_get.clone())],
		height: timeout_height,
	};
	sol_get.timeoutTimestamp -= 1;

	// Mint tokens for per-byte fee
	env.mint_fee_token(
		env.test_module,
		U256::from(32u64) * U256::from(1_000_000_000_000_000_000u128),
	);

	// dispatch the get request
	env.dispatch_get_request(sol_get);

	// handle consensus + timeout
	env.handle_consensus(consensus_proof);
	env.warp(1000);
	env.handle_get_request_timeouts(message);
}
