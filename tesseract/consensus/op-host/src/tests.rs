use crate::{HostConfig, OpConfig, OpHost};
use ethers::providers::Middleware;
use hex_literal::hex;
use primitive_types::H160;
// use ismp_testsuite::mocks::Host;
// use op_verifier::{verify_optimism_dispute_game_proof, verify_optimism_payload};
use ismp::host::StateMachine;
use tesseract_evm::EvmConfig;

const L2_ORACLE: [u8; 20] = hex!("90E9c4f8a994a250F6aEfd61CAFb4F2e895D458F");
const MESSAGE_PARSER: [u8; 20] = hex!("4200000000000000000000000000000000000016");
const DISPUTE_GAME_FACTORY: [u8; 20] = hex!("05F9613aDB30026FFd634f38e5C4dFd30a197Fa1");

#[tokio::test]
#[ignore]
async fn test_payload_proof_verification() {
	dotenv::dotenv().ok();
	let op_orl = std::env::var("OP_URL").expect("OP_URL must be set.");
	let geth_url = std::env::var("GETH_URL").expect("GETH_URL must be set.");
	let host = HostConfig {
		ethereum_rpc_url: vec![geth_url],
		l2_oracle: Some(H160::from(L2_ORACLE)),
		message_parser: H160::from(MESSAGE_PARSER),
		dispute_game_factory: Some(H160::from(DISPUTE_GAME_FACTORY)),
		proposer_config: None,
		l1_state_machine: StateMachine::Evm(10),
		l1_consensus_state_id: "ETH0".to_string(),
		consensus_update_frequency: None,
	};
	let config = OpConfig {
		host: host.clone(),
		evm_config: EvmConfig {
			rpc_urls: vec![op_orl],
			consensus_state_id: "ETH0".to_string(),
			..Default::default()
		},
	};

	let op_client = OpHost::new(&host, &config.evm_config).await.expect("Host creation failed");

	let event = op_client
		.latest_event(5519662, 5519662)
		.await
		.expect("Failed to fetch latest event")
		.expect("There should be an event");

	let _payload_proof = op_client
		.fetch_op_payload(5519662, event)
		.await
		.expect("Error fetching payload proof");

	let l1_header = op_client
		.beacon_execution_client
		.get_block(5519662)
		.await
		.unwrap()
		.expect("Block should exist");

	let _state_root = l1_header.state_root;

	// let _ = verify_optimism_payload::<Host>(
	// 	payload_proof,
	// 	state_root,
	// 	op_client.l2_oracle.unwrap(),
	// 	Default::default(),
	// )
	// .expect("Payload proof verification should succeed");
}

#[tokio::test]
#[ignore]
async fn test_dispute_game_proof_verification() {
	dotenv::dotenv().ok();
	let op_orl = std::env::var("OP_URL").expect("OP_URL must be set.");
	let geth_url = std::env::var("GETH_URL").expect("GETH_URL must be set.");
	let host = HostConfig {
		ethereum_rpc_url: vec![geth_url],
		l2_oracle: Some(H160::from(L2_ORACLE)),
		message_parser: H160::from(MESSAGE_PARSER),
		dispute_game_factory: Some(H160::from(DISPUTE_GAME_FACTORY)),
		proposer_config: None,
		l1_state_machine: StateMachine::Evm(10),
		l1_consensus_state_id: "ETH0".to_string(),
		consensus_update_frequency: None,
	};
	let config = OpConfig {
		host: host.clone(),
		evm_config: EvmConfig {
			rpc_urls: vec![op_orl],
			consensus_state_id: "ETH0".to_string(),
			..Default::default()
		},
	};

	let op_client = OpHost::new(&host, &config.evm_config).await.expect("Host creation failed");

	let events = op_client
		.latest_dispute_games(5524041, 5524180, vec![0])
		.await
		.expect("Failed to fetch latest event");
	assert!(events.len() >= 1);

	let _payload_proof = op_client
		.fetch_dispute_game_payload(5524180, vec![0], events)
		.await
		.expect("Error fetching payload proof")
		.unwrap();

	let l1_header = op_client
		.beacon_execution_client
		.get_block(5524180)
		.await
		.unwrap()
		.expect("Block should exist");

	let _state_root = l1_header.state_root;

	// let _ = verify_optimism_dispute_game_proof::<Host>(
	// 	payload_proof,
	// 	state_root,
	// 	op_client.dispute_game_factory.unwrap(),
	// 	Default::default(),
	// )
	// .expect("Payload proof verification should succeed");
}
