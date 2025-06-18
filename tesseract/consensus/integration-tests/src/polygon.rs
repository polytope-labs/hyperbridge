use codec::Encode;
use futures::StreamExt;
use ismp::{
	host::StateMachine,
	messaging::{CreateConsensusState, Message},
};
use pallet_ismp::primitives::HashAlgorithm;
use tesseract_evm::EvmConfig;
use tesseract_polygon_pos::{PolygonPosConfig, PolygonPosHost};
use tesseract_primitives::{config::Chain, IsmpHost, IsmpProvider};
use tesseract_substrate::{SubstrateClient, SubstrateConfig};

use crate::util::{setup_logging, Hyperbridge};

#[tokio::test]
async fn polygon_consensus_updates() -> anyhow::Result<()> {
	setup_logging();
	dotenv::dotenv().ok();
	let polygon_url = std::env::var("POLYGON_RPC").expect("POLYGON_RPC must be set.");

	let evm_config = EvmConfig {
		rpc_url: polygon_url.clone(),
		state_machine: StateMachine::Polygon,
		consensus_state_id: "POLY".to_string(),
		ismp_host: Default::default(),
		handler: Default::default(),
		signer: "2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622".to_string(),
		etherscan_api_keys: Default::default(),
		tracing_batch_size: None,
	};

	let polygon_config = PolygonPosConfig { evm_config };

	let polygon_host = PolygonPosHost::new(&polygon_config).await?;

	let config_a = SubstrateConfig {
		chain: Chain::Dev,
		hashing: HashAlgorithm::Keccak,
		consensus_state_id: Some("PARA".to_string()),
		chain_rpc_ws: "ws://localhost:9988".to_string(),
		signer: Some(
			"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
		),

		latest_height: None,
	};
	let chain_a = SubstrateClient::<Hyperbridge>::new(config_a).await?;

	let initial_consensus_state = polygon_host.get_consensus_state(Default::default()).await?;

	chain_a
		.create_consensus_state(CreateConsensusState {
			consensus_state: initial_consensus_state.encode(),
			consensus_client_id: *b"POLY",
			consensus_state_id: *b"POLY",
			unbonding_period: 60 * 60 * 60 * 27,
			challenge_period: 0,
			state_machine_commitments: vec![],
		})
		.await?;

	let mut consensus_stream = polygon_host.consensus_notification(chain_a.clone()).await?;

	while let Some(Ok(msg)) = consensus_stream.next().await {
		println!("Submitting consensus message to hyperbridge");

		let _ = chain_a.submit(vec![Message::Consensus(msg.clone())]).await;
	}
	Ok(())
}
