use crate::{EvmClient, EvmConfig};
use alloy::{primitives::B256, providers::Provider};
use codec::Decode;
use evm_state_machine::{
	get_contract_account, get_value_from_proof, types::EvmStateProof, verify_membership,
};
use hex_literal::hex;
use ismp::{
	consensus::{StateCommitment, StateMachineHeight, StateMachineId},
	host::StateMachine,
	messaging::{hash_request, Proof},
	router::{PostRequest, RequestResponse},
};
use ismp_solidity_abi::evm_host::EvmHostInstance;
use ismp_testsuite::mocks::{Host, Keccak256Hasher};
use primitive_types::{H160, H256};
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
	let geth_url = std::env::var("SEPOLIA_URL").expect("SEPOLIA_URL must be set.");
	let signing_key =
		std::env::var("SIGNING_KEY").expect("SIGNING_KEY must be set.");
	let config = EvmConfig {
		rpc_urls: vec![geth_url.clone()],
		state_machine: StateMachine::Evm(1),
		consensus_state_id: "SYNC".to_string(),
		ismp_host: ISMP_HOST,
		signer: signing_key,
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
	let block = client
		.client
		.get_block_by_number(at.into())
		.await
		.unwrap()
		.expect("Block not found");
	let state_root = block.header.state_root;

	let host_addr = alloy::primitives::Address::from_slice(&ISMP_HOST.0);
	let host_contract = EvmHostInstance::new(host_addr, (*client.client).clone());

	let request_meta = host_contract
		.requestCommitments(B256::from_slice(&query.commitment.0))
		.call()
		.await
		.unwrap();

	dbg!(&request_meta);
	assert!(request_meta.sender != alloy::primitives::Address::ZERO);

	let proof = client
		.query_requests_proof(at, vec![query], StateMachine::Polkadot(1))
		.await
		.unwrap();
	let evm_state_proof = EvmStateProof::decode(&mut &*proof).unwrap();
	let contract_root = get_contract_account::<Keccak256Hasher>(
		evm_state_proof.contract_proof,
		&ISMP_HOST.0,
		state_root.0.into(),
	)
	.unwrap()
	.storage_root;

	let key = sp_core::keccak_256(&client.request_commitment_key(query.commitment).1 .0).to_vec();
	let value = get_value_from_proof::<Keccak256Hasher>(
		key.clone(),
		H256::from_slice(contract_root.as_slice()),
		evm_state_proof.storage_proof.get(&key).unwrap().clone(),
	)
	.unwrap();
	assert!(value.is_some());

	let decoded_address: alloy_primitives::Address =
		alloy_rlp::Decodable::decode(&mut &*value.unwrap()).unwrap();

	assert_eq!(request_meta.sender.0, decoded_address.0);

	verify_membership::<Keccak256Hasher>(
		RequestResponse::Request(vec![req]),
		StateCommitment { timestamp: 0, overlay_root: None, state_root: state_root.0.into() },
		&Proof {
			height: StateMachineHeight {
				id: StateMachineId {
					state_id: StateMachine::Evm(97),
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

const NEW_HOST: H160 = H160(hex!("Bc0fA79725aCD430D507855e77f30C9d9ED4dC24"));
#[tokio::test]
#[ignore]
async fn fetch_state_commitment() -> anyhow::Result<()> {
	dotenv::dotenv().ok();
	let geth_url = std::env::var("SEPOLIA_URL").expect("SEPOLIA_URL must be set.");
	let signing_key =
		std::env::var("SIGNING_KEY").expect("SIGNING_KEY must be set.");
	let config = EvmConfig {
		rpc_urls: vec![geth_url.clone()],
		state_machine: StateMachine::Evm(1),
		consensus_state_id: "ETH0".to_string(),
		ismp_host: NEW_HOST,
		signer: signing_key,
		..Default::default()
	};

	let client = EvmClient::new(config).await.expect("Host creation failed");

	let state_machine_height = StateMachineHeight {
		id: StateMachineId { state_id: StateMachine::Kusama(4009), consensus_state_id: *b"PARA" },
		height: 899092,
	};

	let state_commitment = client.query_state_machine_commitment(state_machine_height).await?;
	dbg!(&state_commitment);
	assert_eq!(
		state_commitment.overlay_root,
		Some(hex!("2268395e6c16e773cd60bc3a7593ec885098599d5d648aca21fa556de2838342").into())
	);
	assert_eq!(
		state_commitment.state_root,
		hex!("f8972a624b169db9b0fa86030921f7ba5ddd5f4af967ee5906a761ea5ded24e0").into()
	);
	assert_eq!(state_commitment.timestamp, 3443504784000);
	Ok(())
}
