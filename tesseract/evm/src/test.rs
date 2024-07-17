use crate::{EvmClient, EvmConfig};
use codec::Decode;
use ethers::providers::Middleware;
use evm_common::{
	get_contract_storage_root, get_value_from_proof, types::EvmStateProof, verify_membership,
};
use hex_literal::hex;
use ismp::{
	consensus::{StateCommitment, StateMachineHeight, StateMachineId},
	host::{Ethereum, StateMachine},
	messaging::{hash_request, Proof},
	router::{PostRequest, RequestResponse},
};
use ismp_solidity_abi::evm_host::EvmHost;
use ismp_testsuite::mocks::{Host, Keccak256Hasher};
use primitive_types::H160;
use std::str::FromStr;
use tesseract_primitives::{IsmpProvider, Query};

// source :
// 45544845
// dest :
// 42415345
// from :
// D21C7893BD7A96732E65CEB2B9E6DD9CA95846C9
// to :
// 66819E1BBB03760D227745C71FE76C5783A5F810
// timeoutTimestamp :
// 1707167196
// data :
// 68656C6C6F2066726F6D2045544845
// gaslimit :
// 0
// fee :
// 0

const ISMP_HOST: H160 = H160(hex!("7b27ab4C64cdc30d219cEa9aC3Dd442Fd4D00E50"));
#[tokio::test]
#[ignore]
async fn test_ismp_state_proof() {
	dotenv::dotenv().ok();
	let geth_url = std::env::var("GETH_URL").expect("GETH_URL must be set.");
	let config = EvmConfig {
		rpc_urls: vec![geth_url.clone()],
		state_machine: StateMachine::Ethereum(Ethereum::ExecutionLayer),
		consensus_state_id: "SYNC".to_string(),
		ismp_host: ISMP_HOST,
		signer: "2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622".to_string(),
		..Default::default()
	};

	let client = EvmClient::new(config).await.expect("Host creation failed");

	let post = PostRequest {
		source: StateMachine::from_str(
			&String::from_utf8(hex::decode("45544845").unwrap()).unwrap(),
		)
		.unwrap(),
		dest: StateMachine::from_str(&String::from_utf8(hex::decode("42415345").unwrap()).unwrap())
			.unwrap(),
		nonce: 119,
		from: hex::decode("D21C7893BD7A96732E65CEB2B9E6DD9CA95846C9").unwrap(),
		to: hex::decode("66819E1BBB03760D227745C71FE76C5783A5F810").unwrap(),
		timeout_timestamp: 1707167196,
		body: hex::decode("68656C6C6F2066726F6D2045544845").unwrap(),
	};

	let req = ismp::router::Request::Post(post.clone());
	let query = Query {
		source_chain: post.source,
		dest_chain: post.dest,
		nonce: post.nonce,
		commitment: hash_request::<Host>(&req),
	};
	let at = 5224621u64;
	let state_root = client.client.get_block(at).await.unwrap().unwrap().state_root;

	let host_contract = EvmHost::new(ISMP_HOST, client.client.clone());

	let request_meta = host_contract.request_commitments(query.commitment.0).await.unwrap();

	dbg!(&request_meta);
	assert!(request_meta.sender != H160::zero());

	let proof = client.query_requests_proof(at, vec![query]).await.unwrap();
	let evm_state_proof = EvmStateProof::decode(&mut &*proof).unwrap();
	let contract_root = get_contract_storage_root::<Keccak256Hasher>(
		evm_state_proof.contract_proof,
		&ISMP_HOST.0,
		state_root,
	)
	.unwrap();

	let key = sp_core::keccak_256(&client.request_commitment_key(query.commitment).1 .0).to_vec();
	let value = get_value_from_proof::<Keccak256Hasher>(
		key.clone(),
		contract_root,
		evm_state_proof.storage_proof.get(&key).unwrap().clone(),
	)
	.unwrap();
	assert!(value.is_some());

	let decoded_address: alloy_primitives::Address =
		alloy_rlp::Decodable::decode(&mut &*value.unwrap()).unwrap();

	assert_eq!(request_meta.sender.0, decoded_address.0);

	verify_membership::<Keccak256Hasher>(
		RequestResponse::Request(vec![req]),
		StateCommitment { timestamp: 0, overlay_root: None, state_root },
		&Proof {
			height: StateMachineHeight {
				id: StateMachineId {
					state_id: StateMachine::Polygon,
					consensus_state_id: [0, 0, 0, 0],
				},
				height: 0,
			},
			proof,
		},
		ISMP_HOST,
	)
	.unwrap();
}
