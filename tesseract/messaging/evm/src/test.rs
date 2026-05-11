use crate::{EvmClient, EvmConfig};
use alloy::{primitives::B256, providers::Provider};
use hex_literal::hex;
use ismp::{
	consensus::{StateCommitment, StateMachineHeight, StateMachineId},
	host::StateMachine,
	messaging::{hash_request, Proof},
	router::{PostRequest, Request, RequestResponse},
};
use ismp_solidity_abi::evm_host::EvmHostInstance;
use ismp_testsuite::mocks::Keccak256Hasher;
use primitive_types::{H160, H256};
use tesseract_primitives::{IsmpProvider, Query};

// Sepolia ISMP host contract
const ISMP_HOST: H160 = H160(hex!("2EdB74C269948b60ec1000040E104cef0eABaae8"));

#[tokio::test]
#[ignore]
async fn test_ismp_state_proof() {
	dotenv::dotenv().ok();
	let geth_url = std::env::var("SEPOLIA_URL").expect("SEPOLIA_URL must be set.");
	let signing_key = std::env::var("SIGNING_KEY").expect("SIGNING_KEY must be set.");
	let config = EvmConfig {
		rpc_urls: vec![geth_url.clone()],
		state_machine: StateMachine::Evm(11155111),
		consensus_state_id: "ETH0".to_string(),
		ismp_host: ISMP_HOST,
		signer: signing_key,
		..Default::default()
	};

	let client = EvmClient::new(config).await.expect("Host creation failed");

	// This request was dispatched on Sepolia at block 10290141.
	let post_request = PostRequest {
		source: StateMachine::Evm(11155111),
		dest: StateMachine::Evm(420420417),
		nonce: 13813,
		from: hex!("fcda26ca021d5535c3059547390e6ccd8de7aca6").to_vec(),
		to: hex!("1c1e5be83df4a54c7a2230c337e4a3e8b7354b1c").to_vec(),
		timeout_timestamp: 1771479552,
		body: hex!(
			"0000000000000000000000000000000000000000000000000000005af3107a40"
			"000f8a193ff464434486c0daf7db2a895884365d2bc84ba47a68fcf89c1b14b5"
			"b800000000000000000000000000000000000000000000000000000000000000"
			"000000000000000000000000006cb7f088fc3d07e145dc0418f12f74268d0d03"
			"090000000000000000000000003bd450e3456c4d7e293cd07757c7d1e001843b"
			"b6"
		)
		.to_vec(),
	};

	let request = Request::Post(post_request);
	let commitment = hash_request::<Keccak256Hasher>(&request);

	let query = Query {
		source_chain: StateMachine::Evm(11155111),
		dest_chain: StateMachine::Evm(420420417),
		nonce: 13813,
		commitment,
	};
	let at = 10290141u64;
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
		.requestCommitments(B256::from_slice(&commitment.0))
		.call()
		.await
		.unwrap();

	dbg!(&request_meta);
	assert!(request_meta.sender != alloy::primitives::Address::ZERO);

	let proof = client
		.query_requests_proof(at, vec![query], StateMachine::Polkadot(1))
		.await
		.unwrap();

	let state_commitment = StateCommitment {
		timestamp: block.header.timestamp,
		overlay_root: None,
		state_root: H256::from(state_root.0),
	};
	let membership_proof = Proof {
		height: StateMachineHeight {
			id: StateMachineId {
				state_id: StateMachine::Evm(11155111),
				consensus_state_id: *b"ETH0",
			},
			height: at,
		},
		proof: proof.clone(),
	};
	evm_state_machine::verify_membership::<Keccak256Hasher>(
		RequestResponse::Request(vec![request]),
		state_commitment,
		&membership_proof,
		ISMP_HOST,
	)
	.expect("verify_membership should succeed");
}

const NEW_HOST: H160 = H160(hex!("Bc0fA79725aCD430D507855e77f30C9d9ED4dC24"));
#[tokio::test]
#[ignore]
async fn fetch_state_commitment() -> anyhow::Result<()> {
	dotenv::dotenv().ok();
	let geth_url = std::env::var("SEPOLIA_URL").expect("SEPOLIA_URL must be set.");
	let signing_key = std::env::var("SIGNING_KEY").expect("SIGNING_KEY must be set.");
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
