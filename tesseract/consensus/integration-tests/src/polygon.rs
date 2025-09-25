use codec::Encode;
use ismp::{host::StateMachine, messaging::CreateConsensusState};
use std::sync::Arc;
use substrate_state_machine::HashAlgorithm;
use subxt_utils::Hyperbridge;
use tesseract_evm::EvmConfig;
use tesseract_polygon::{HostConfig, PolygonPosConfig, PolygonPosHost};
use tesseract_primitives::IsmpHost;
use tesseract_substrate::{SubstrateClient, SubstrateConfig};

use crate::util::setup_logging;

#[tokio::test]
async fn polygon_consensus_updates() -> anyhow::Result<()> {
	setup_logging();
	dotenv::dotenv().ok();
	let polygon_execution_url =
		std::env::var("POLYGON_EXECUTION_RPC").expect("POLYGON_EXECUTION_RPC must be set.");
	let polygon_heimdall_rpc =
		std::env::var("POLYGON_HEIMDALL").expect("POLYGON_HEIMDALL must be set.");
	let polygon_heimdall_rest =
		std::env::var("POLYGON_HEIMDALL_REST").expect("POLYGON_HEIMDALL_REST must be set.");

	let evm_config = EvmConfig {
		rpc_urls: vec![polygon_execution_url.clone()],
		state_machine: StateMachine::Evm(137),
		consensus_state_id: "POLY".to_string(),
		ismp_host: Default::default(),
		signer: "2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622".to_string(),
		etherscan_api_key: Default::default(),
		tracing_batch_size: None,
		query_batch_size: None,
		poll_interval: None,
		gas_price_buffer: None,
		client_type: Default::default(),
		initial_height: None,
	};

	let host_config = HostConfig {
		consensus_update_frequency: Some(300),
		heimdall_rpc_url: polygon_heimdall_rpc,
		disable: Some(false),
	};

	let polygon_config = PolygonPosConfig { host: host_config, evm_config };

	let polygon_host =
		PolygonPosHost::new(&polygon_config.host, &polygon_config.evm_config).await?;

	let config_a = SubstrateConfig {
		state_machine: StateMachine::Kusama(2000),
		hashing: Some(HashAlgorithm::Keccak),
		consensus_state_id: Some("PARA".to_string()),
		rpc_ws: "ws://localhost:9990".to_string(),
		max_rpc_payload_size: None,
		signer: Some(
			"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
		),
		initial_height: None,
		max_concurent_queries: None,
		poll_interval: None,
		fee_token_decimals: None,
	};
	let chain_a = SubstrateClient::<Hyperbridge>::new(config_a).await?;

	let initial_consensus_state = polygon_host.get_consensus_state().await?;

	chain_a
		.create_consensus_state(CreateConsensusState {
			consensus_state: initial_consensus_state.encode(),
			consensus_client_id: *b"PLGN",
			consensus_state_id: *b"POLY",
			unbonding_period: 82 * 3600,
			challenge_periods: vec![(StateMachine::Evm(137), 2 * 60)].into_iter().collect(),
			state_machine_commitments: vec![],
		})
		.await?;

	println!("created consensus state");

	polygon_host.start_consensus(Arc::new(chain_a)).await?;

	Ok(())
}
