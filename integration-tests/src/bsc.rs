use std::sync::Arc;

use codec::{Decode, Encode};
use ismp::{host::StateMachine, messaging::CreateConsensusState};
use substrate_state_machine::HashAlgorithm;
use tesseract_beefy::{BeefyHost, Network};
use tesseract_bsc::{BscPosConfig, BscPosHost, ConsensusState, HostConfig, KeccakHasher};
use tesseract_evm::EvmConfig;
use tesseract_primitives::IsmpProvider;
use tesseract_substrate::{config::Blake2SubstrateChain, SubstrateClient, SubstrateConfig};

use crate::util::{setup_logging, Hyperbridge};

#[tokio::test]
async fn bsc_consensus_updates() -> anyhow::Result<()> {
	setup_logging();
	dotenv::dotenv().ok();
	let polygon_url = std::env::var("BSC_RPC").expect("BSC_RPC must be set.");

	let evm_config = EvmConfig {
		rpc_urls: vec![polygon_url.clone()],
		state_machine: StateMachine::Bsc,
		consensus_state_id: "BSC0".to_string(),
		ismp_host: Default::default(),
		handler: Default::default(),
		signer: "2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622".to_string(),
		etherscan_api_key: Default::default(),
		..Default::default()
	};

	let bsc_config =
		BscPosConfig { host: HostConfig { consensus_update_frequency: Some(120) }, evm_config };

	let bsc_host = BscPosHost::new(&bsc_config.host, &bsc_config.evm_config).await?;

	let config_a = SubstrateConfig {
		state_machine: StateMachine::Kusama(2000),
		hashing: Some(HashAlgorithm::Keccak),
		consensus_state_id: Some("PARA".to_string()),
		rpc_ws: "ws://localhost:9990".to_string(),
		max_rpc_payload_size: None,
		signer: Some(
			"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
		),
		latest_height: None,
	};

	let host = tesseract_beefy::HostConfig {
		relay_rpc_ws: "ws://104.155.23.240:9944".to_string(),
		consensus_update_frequency: 45,
		zk_beefy: Some(Network::Rococo),
	};
	let hyperbridge = SubstrateClient::<Hyperbridge>::new(config_a.clone()).await?;

	let beefy_host = BeefyHost::<Blake2SubstrateChain, Hyperbridge>::new(
		&host,
		&config_a,
		Arc::new(hyperbridge.clone()),
	)
	.await?;
	let chain_a = Arc::new(beefy_host);

	let initial_consensus_state =
		bsc_host.get_consensus_state::<KeccakHasher>(Default::default()).await?;

	hyperbridge
		.create_consensus_state(CreateConsensusState {
			consensus_state: initial_consensus_state.encode(),
			consensus_client_id: *b"BSCP",
			consensus_state_id: *b"BSC0",
			unbonding_period: 60 * 60 * 60 * 27,
			challenge_period: 0,
			state_machine_commitments: vec![],
		})
		.await?;
	tesseract_consensus::relay(chain_a, Arc::new(bsc_host), Default::default())
		.await
		.unwrap();
	Ok(())
}

#[tokio::test]
async fn fetch_bsc_consensus_state() -> Result<(), anyhow::Error> {
	let config_a = SubstrateConfig {
		state_machine: StateMachine::Kusama(4009),
		hashing: Some(HashAlgorithm::Keccak),
		max_rpc_payload_size: None,
		consensus_state_id: Some("PARA".to_string()),
		rpc_ws: "ws://localhost:9944".to_string(),
		signer: Some(
			"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
		),
		latest_height: None,
	};
	let chain_a = SubstrateClient::<Hyperbridge>::new(config_a).await?;

	let consensus_state = chain_a.query_consensus_state(None, *b"BSC0").await.unwrap();

	let consensus_state = ConsensusState::decode(&mut &consensus_state[..]).unwrap();

	dbg!(&consensus_state);
	Ok(())
}
