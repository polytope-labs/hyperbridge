use crate::util::{setup_logging, Hyperbridge};

use hex_literal::hex;
use ismp::host::StateMachine;
use std::sync::Arc;
use substrate_state_machine::HashAlgorithm;
use sync_committee_primitives::constants::ETH1_DATA_VOTES_BOUND_GNO;

use tesseract_evm::EvmConfig;
use tesseract_primitives::IsmpHost;
use tesseract_substrate::{SubstrateClient, SubstrateConfig};
use tesseract_sync_committee::{HostConfig as SyncHostConfig, SyncCommitteeHost};

// const ISMP_HANDLER: primitive_types::H160 =
// primitive_types::H160(hex!("574f5260097C90c30427846A560Ae7696A287C56")); const TEST_HOST:
// primitive_types::H160 = primitive_types::H160(hex!("3C51029d8b53f00384272AaFd92BA5c50F94EE6E"));
// const MOCK_MODULE: primitive_types::H160 =
// primitive_types::H160(hex!("3F076aE33723b2F61656166D40a78d409e350625"));

// #[tokio::test]
// async fn beefy_consensus_updates() -> anyhow::Result<()> {
// 	setup_logging();
// 	let chain_a = {
// 		let substrate = SubstrateConfig {
// 			state_machine: StateMachine::Kusama(4009),
// 			hashing: Some(HashAlgorithm::Keccak),
// 			consensus_state_id: Some("PARA".to_string()),
// 			max_rpc_payload_size: None,
// 			rpc_ws: "ws://34.140.78.68:9933".to_string(),
// 			signer: Some(
// 				"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
// 			),
// 			latest_height: None,
// 		};

// 		let host = HostConfig {
// 			relay_rpc_ws: "ws://104.155.23.240:9944".to_string(),
// 			consensus_update_frequency: 45,
// 			zk_beefy: Some(Network::Rococo),
// 		};

// 		let substrate_provider = SubstrateClient::<Hyperbridge>::new(substrate.clone()).await?;

// 		let beefy_host = BeefyHost::<Blake2SubstrateChain, Hyperbridge>::new(
// 			&host,
// 			&substrate,
// 			Arc::new(substrate_provider),
// 		)
// 		.await?;

// 		Arc::new(beefy_host)
// 	};

// 	chain_a
// 		.prover
// 		.inner()
// 		.para
// 		.blocks()
// 		.subscribe_best()
// 		.await
// 		.unwrap()
// 		.skip_while(|result| {
// 			futures::future::ready({
// 				match result {
// 					Ok(block) => block.number() < 1,
// 					Err(_) => false,
// 				}
// 			})
// 		})
// 		.take(1)
// 		.collect::<Vec<_>>()
// 		.await;

// 	println!("Parachains Onboarded");

// 	let chain_b = {
// 		let config = EvmConfig {
// 			rpc_urls: vec!["ws://localhost:8546".to_string()],
// 			state_machine: StateMachine::Ethereum(Ethereum::ExecutionLayer),
// 			consensus_state_id: "BEAC".to_string(),
// 			ismp_host: TEST_HOST,
// 			handler: ISMP_HANDLER,
// 			signer: "2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622".to_string(),
// 			..Default::default()
// 		};

// 		let host = ArbHostConfig {
// 			beacon_rpc_url: vec!["ws://localhost:8546".to_string()],
// 			rollup_core: Default::default(),
// 		};

// 		let arb_client = ArbHost::new(&host, &config).await?;

// 		Arc::new(arb_client)
// 	};

// 	let hash = hex!("32f98c6607a4ba6ce39963f717f960d29ae65306b0fea8340a88c28b2d7f1147");
// 	let initial_state: BeefyConsensusState = chain_a
// 		.prover
// 		.query_initial_consensus_state(Decode::decode(&mut &hash[..])?)
// 		.await?
// 		.into();
// 	let chain_b_provider = chain_b.provider();
// 	let _ = chain_b_provider
// 		.set_initial_consensus_state(CreateConsensusState {
// 			consensus_state: initial_state.encode(),
// 			consensus_client_id: *b"PARA",
// 			consensus_state_id: *b"PARA",
// 			unbonding_period: 0,
// 			challenge_period: 0,
// 			state_machine_commitments: Default::default(),
// 		})
// 		.await;

// 	let task = tokio::spawn({
// 		let chain_a = chain_a.clone();
// 		let chain_b = chain_b.clone();
// 		async move { tesseract_consensus::relay(chain_a, chain_b, Default::default()).await.unwrap() }
// 	});

// 	let _ = chain_a.spawn_prover(vec![chain_b_provider]).await;
// 	let _ = task.await;

// 	Ok(())
// }

// #[tokio::test]
// async fn beefy_consenus_and_messaging_updates() -> anyhow::Result<()> {
// 	setup_logging();
// 	let (chain_a, hyperbridge) = {
// 		let substrate = SubstrateConfig {
// 			state_machine: StateMachine::Kusama(4009),
// 			hashing: Some(HashAlgorithm::Keccak),
// 			consensus_state_id: Some("PARA".to_string()),
// 			max_rpc_payload_size: None,
// 			rpc_ws: "ws://localhost:9988".to_string(),
// 			signer: Some(
// 				"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
// 			),
// 			latest_height: None,
// 		};

// 		let host = HostConfig {
// 			relay_rpc_ws: "ws://104.155.23.240:9944".to_string(),
// 			consensus_update_frequency: 45,
// 			zk_beefy: Some(Network::Rococo),
// 		};

// 		let substrate_provider = SubstrateClient::<Hyperbridge>::new(substrate.clone()).await?;

// 		let beefy_host = BeefyHost::<Blake2SubstrateChain, Hyperbridge>::new(
// 			&host,
// 			&substrate,
// 			Arc::new(substrate_provider.clone()),
// 		)
// 		.await?;

// 		(Arc::new(beefy_host), substrate_provider)
// 	};

// 	chain_a
// 		.prover
// 		.inner()
// 		.para
// 		.blocks()
// 		.subscribe_best()
// 		.await
// 		.unwrap()
// 		.skip_while(|result| {
// 			futures::future::ready({
// 				match result {
// 					Ok(block) => block.number() < 1,
// 					Err(_) => false,
// 				}
// 			})
// 		})
// 		.take(1)
// 		.collect::<Vec<_>>()
// 		.await;

// 	println!("Parachains Onboarded");

// 	let chain_b = {
// 		let config = EvmConfig {
// 			rpc_urls: vec!["ws://localhost:8546".to_string()],
// 			state_machine: StateMachine::Ethereum(Ethereum::ExecutionLayer),
// 			consensus_state_id: "BEAC".to_string(),
// 			ismp_host: TEST_HOST,
// 			handler: ISMP_HANDLER,
// 			signer: "2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622".to_string(),
// 			..Default::default()
// 		};

// 		let host = ArbHostConfig {
// 			beacon_rpc_url: vec!["ws://localhost:8546".to_string()],
// 			rollup_core: Default::default(),
// 		};

// 		let arb_client = ArbHost::new(&host, &config).await?;

// 		Arc::new(arb_client)
// 	};

// 	let chain_b_provider = chain_b.provider();

// 	let initial_state: BeefyConsensusState =
// 		chain_a.prover.inner().get_initial_consensus_state().await?.into();
// 	let _ = chain_b_provider
// 		.set_initial_consensus_state(CreateConsensusState {
// 			consensus_state: initial_state.encode(),
// 			consensus_client_id: *b"PARA",
// 			consensus_state_id: *b"PARA",
// 			unbonding_period: 0,
// 			challenge_period: 0,
// 			state_machine_commitments: Default::default(),
// 		})
// 		.await;

// 	let consensus = tokio::spawn({
// 		let chain_a = chain_a.clone();
// 		let chain_b = chain_b.clone();
// 		async move { tesseract_consensus::relay(chain_a, chain_b, Default::default()).await.unwrap() }
// 	});

// 	let tx_payment = Arc::new(TransactionPayment::initialize("./dev.db").await?);
// 	let _messaging = tokio::spawn({
// 		let hyperbridge = hyperbridge.clone();
// 		async move {
// 			tesseract_messaging::relay(
// 				hyperbridge,
// 				chain_b_provider.clone(),
// 				Default::default(),
// 				StateMachine::Kusama(4009),
// 				tx_payment,
// 				Default::default(),
// 			)
// 			.await
// 			.unwrap()
// 		}
// 	});

// 	let since_the_epoch =
// 		SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");

// 	println!("dispatching message");
// 	hyperbridge
// 		.dispatch_to_evm(EvmParams {
// 			module: MOCK_MODULE,
// 			destination: Ethereum::ExecutionLayer,
// 			timeout: since_the_epoch.as_secs() + (60 * 60),
// 			count: 10,
// 		})
// 		.await?;

// 	let _ = futures::join!(consensus, _messaging);

// 	Ok(())
// }

// const MESSAGE_PARSER: [u8; 20] = hex!("4200000000000000000000000000000000000016");
// const DISPUTE_GAME_FACTORY: [u8; 20] = hex!("05F9613aDB30026FFd634f38e5C4dFd30a197Fa1");

// #[tokio::test]
// async fn sync_committee_consensus_updates() -> anyhow::Result<()> {
// 	setup_logging();
// 	dotenv::dotenv().ok();
// 	let geth_url = std::env::var("GETH_URL").expect("GETH_URL must be set.");
// 	let op_url = std::env::var("OP_URL").expect("OP_URL must be set.");
// 	let beacon_url = std::env::var("BEACON_URL").expect("BEACON_URL must be set.");

// 	let config_a = SubstrateConfig {
// 		state_machine: StateMachine::Kusama(2000),
// 		hashing: Some(HashAlgorithm::Keccak),
// 		consensus_state_id: Some("PARA".to_string()),
// 		max_rpc_payload_size: None,
// 		rpc_ws: "ws://localhost:9990".to_string(),
// 		signer: Some(
// 			"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
// 		),

// 		latest_height: None,
// 	};
// 	let host = tesseract_beefy::HostConfig {
// 		relay_rpc_ws: "ws://104.155.23.240:9944".to_string(),
// 		consensus_update_frequency: 45,
// 		zk_beefy: Some(Network::Rococo),
// 	};
// 	let substrate_provider = SubstrateClient::<Hyperbridge>::new(config_a.clone()).await?;

// 	let beefy_host = BeefyHost::<Blake2SubstrateChain, Hyperbridge>::new(
// 		&host,
// 		&config_a,
// 		Arc::new(substrate_provider),
// 	)
// 	.await?;
// 	let chain_a = Arc::new(beefy_host);

// 	let chain_b = {
// 		let config = EvmConfig {
// 			rpc_urls: vec![geth_url.clone()],
// 			state_machine: StateMachine::Ethereum(Ethereum::ExecutionLayer),
// 			consensus_state_id: "ETH0".to_string(),
// 			ismp_host: TEST_HOST,
// 			handler: ISMP_HANDLER,
// 			signer: "2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622".to_string(),
// 			..Default::default()
// 		};

// 		let sync_commitee_config =
// 			SyncHostConfig { beacon_http_urls: vec![beacon_url], consensus_update_frequency: 60 };

// 		let op_host_config = op_host::HostConfig {
// 			beacon_rpc_url: vec![geth_url],
// 			l2_oracle: None,
// 			message_parser: H160::from(MESSAGE_PARSER),
// 			dispute_game_factory: Some(H160::from(DISPUTE_GAME_FACTORY)),
// 		};
// 		let op_config = OpConfig {
// 			host: op_host_config.clone(),
// 			evm_config: EvmConfig {
// 				rpc_urls: vec![op_url],
// 				consensus_state_id: "ETH0".to_string(),
// 				..Default::default()
// 			},
// 		};

// 		let l2_configs =
// 			vec![(StateMachine::Ethereum(Ethereum::Optimism), L2Config::OpStack(op_config))]
// 				.into_iter()
// 				.collect();

// 		Arc::new(
// 			SyncCommitteeHost::<Sepolia>::new(&sync_commitee_config, &config, l2_configs).await?,
// 		)
// 	};

// 	let ismp_contract_addresses =
// 		BTreeMap::from([(StateMachine::Ethereum(Ethereum::ExecutionLayer), TEST_HOST)]);

// 	let dispute_factory_address = BTreeMap::from([(
// 		StateMachine::Ethereum(Ethereum::Optimism),
// 		H160::from(DISPUTE_GAME_FACTORY),
// 	)]);

// 	let params = GetConsensusStateParams {
// 		ismp_contract_addresses,
// 		l2_oracle_address: Default::default(),
// 		rollup_core_address: Default::default(),
// 		dispute_factory_address,
// 	};

// 	let chain_a_provider = chain_a.provider();
// 	let beacon_consensus_state = chain_b.get_consensus_state(params, None).await?;
// 	let _ = chain_a_provider
// 		.set_initial_consensus_state(CreateConsensusState {
// 			consensus_state: beacon_consensus_state.encode(),
// 			consensus_client_id: *b"BEAC",
// 			consensus_state_id: *b"ETH0",
// 			unbonding_period: 60 * 60 * 60 * 27,
// 			challenge_period: 0,
// 			state_machine_commitments: vec![],
// 		})
// 		.await?;

// 	let handle = tokio::spawn({
// 		let chain_a = chain_a.clone();
// 		let chain_b = chain_b.clone();
// 		async move { tesseract_consensus::relay(chain_a, chain_b, Default::default()).await.unwrap() }
// 	});

// 	handle.await?;

// 	Ok(())
// }

// #[tokio::test]
// async fn evm_messaging_relay() -> anyhow::Result<()> {
// 	setup_logging();

// 	let geth_url = std::env::var("GETH_URL").expect("GETH_URL must be set.");
// 	let op_url = std::env::var("OP_URL").expect("OP_URL must be set.");
// 	let beacon_url = std::env::var("BEACON_URL").expect("BEACON_URL must be set.");

// 	let config_a = SubstrateConfig {
// 		state_machine: StateMachine::Kusama(2000),
// 		hashing: Some(HashAlgorithm::Keccak),
// 		consensus_state_id: Some("PARA".to_string()),
// 		max_rpc_payload_size: None,
// 		rpc_ws: "ws://localhost:9990".to_string(),
// 		signer: Some(
// 			"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
// 		),

// 		latest_height: None,
// 	};
// 	let host = tesseract_beefy::HostConfig {
// 		relay_rpc_ws: "ws://104.155.23.240:9944".to_string(),
// 		consensus_update_frequency: 45,
// 		zk_beefy: Some(Network::Rococo),
// 	};
// 	let hyperbridge = SubstrateClient::<Hyperbridge>::new(config_a.clone()).await?;

// 	let beefy_host = BeefyHost::<Blake2SubstrateChain, Hyperbridge>::new(
// 		&host,
// 		&config_a,
// 		Arc::new(hyperbridge.clone()),
// 	)
// 	.await?;
// 	let chain_a = Arc::new(beefy_host);

// 	let (chain_b, evm_client) = {
// 		let config = EvmConfig {
// 			rpc_urls: vec![geth_url.clone()],
// 			state_machine: StateMachine::Ethereum(Ethereum::ExecutionLayer),
// 			consensus_state_id: "ETH0".to_string(),
// 			ismp_host: TEST_HOST,
// 			handler: ISMP_HANDLER,
// 			signer: "2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622".to_string(),
// 			..Default::default()
// 		};

// 		let sync_commitee_config =
// 			SyncHostConfig { beacon_http_urls: vec![beacon_url], consensus_update_frequency: 60 };

// 		let op_host_config = op_host::HostConfig {
// 			beacon_rpc_url: vec![geth_url],
// 			l2_oracle: None,
// 			message_parser: H160::from(MESSAGE_PARSER),
// 			dispute_game_factory: Some(H160::from(DISPUTE_GAME_FACTORY)),
// 		};
// 		let op_config = OpConfig {
// 			host: op_host_config.clone(),
// 			evm_config: EvmConfig {
// 				rpc_urls: vec![op_url],
// 				consensus_state_id: "ETH0".to_string(),
// 				..Default::default()
// 			},
// 		};

// 		let evm_client = EvmClient::new(config.clone()).await?;
// 		let l2_configs =
// 			vec![(StateMachine::Ethereum(Ethereum::Optimism), L2Config::OpStack(op_config))]
// 				.into_iter()
// 				.collect();

// 		(
// 			Arc::new(
// 				SyncCommitteeHost::<Sepolia>::new(&sync_commitee_config, &config, l2_configs)
// 					.await?,
// 			),
// 			evm_client,
// 		)
// 	};

// 	let ismp_contract_addresses =
// 		BTreeMap::from([(StateMachine::Ethereum(Ethereum::ExecutionLayer), TEST_HOST)]);

// 	let params = GetConsensusStateParams {
// 		ismp_contract_addresses,
// 		l2_oracle_address: Default::default(),
// 		rollup_core_address: Default::default(),
// 		dispute_factory_address: Default::default(),
// 	};

// 	let chain_a_provider = chain_a.provider();
// 	let beacon_consensus_state = chain_b.get_consensus_state(params, None).await?;
// 	let _ = chain_a_provider
// 		.set_initial_consensus_state(CreateConsensusState {
// 			consensus_state: beacon_consensus_state.encode(),
// 			consensus_client_id: *b"BEAC",
// 			consensus_state_id: *b"ETH0",
// 			unbonding_period: 60 * 60 * 60 * 27,
// 			challenge_period: 0,
// 			state_machine_commitments: vec![],
// 		})
// 		.await?;

// 	let tx_payment = Arc::new(TransactionPayment::initialize("./dev.db").await?);

// 	let _handle = tokio::spawn({
// 		let chain_a = chain_a.clone();
// 		let chain_b = chain_b.clone();
// 		async move { tesseract_consensus::relay(chain_a, chain_b, Default::default()).await.unwrap() }
// 	});

// 	let _ = tokio::spawn({
// 		let chain_b = chain_b.clone();
// 		let hyperbridge = hyperbridge.clone();
// 		async move {
// 			tesseract_messaging::relay(
// 				hyperbridge,
// 				chain_b.provider(),
// 				Default::default(),
// 				StateMachine::Kusama(4009),
// 				tx_payment,
// 				Default::default(),
// 			)
// 			.await
// 			.unwrap()
// 		}
// 	});

// 	hyperbridge
// 		.dispatch_to_evm(EvmParams {
// 			module: MOCK_MODULE,
// 			destination: Ethereum::ExecutionLayer,
// 			timeout: 0,
// 			count: 1,
// 		})
// 		.await?;

// 	let mock_contract = PingModule::new(MOCK_MODULE, evm_client.client.clone());
// 	let _events = mock_contract.event::<PostReceivedFilter>();
// 	// let events = events.subscribe().await.unwrap();
// 	// let _ = timeout_future(
// 	// 	events.take(1).into_stream().next(),
// 	// 	60 * 10,
// 	// 	"Did not see Post received Event".to_string(),
// 	// )
// 	// .await;
// 	// println!("🚀🚀 Successfully to dispatched request from parachain to ethereum");
// 	// chain_b.dispatch_to_parachain(MOCK_MODULE, Chain::Dev.para_id()).await?;
// 	//
// 	// let _ = timeout_future(
// 	// 	chain_a.pallet_ismp_demo_events_stream(1, "IsmpDemo", "Request"),
// 	// 	60 * 10,
// 	// 	"Did not see Request received Event".to_string(),
// 	// )
// 	// .await;
// 	// println!("🚀🚀 Successfully to dispatched request from ethereum to parachain");
// 	// let _ = handle.await;
// 	Ok(())
// }

// #[tokio::test]
// async fn l2_state_machine_notification() -> anyhow::Result<()> {
// 	dotenv::dotenv().ok();
// 	let op_url = std::env::var("OP_URL").expect("OP_URL must be set.");
// 	let base_url = std::env::var("BASE_URL").expect("OP_URL must be set.");
// 	let arb_url = std::env::var("ARB_URL").expect("OP_URL must be set.");
// 	let geth_url = std::env::var("GETH_URL").expect("OP_URL must be set.");
// 	let para_id = 4296;

// 	let base = {
// 		let config = EvmConfig {
// 			rpc_urls: vec![base_url],
// 			state_machine: StateMachine::Ethereum(Ethereum::Base),
// 			consensus_state_id: "ETH1".to_string(),
// 			ismp_host: Default::default(),
// 			handler: hex!("E952FC53fcdaAD991916049F4a77F21CEc72A698").into(),
// 			signer: "2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622".to_string(),
// 			..Default::default()
// 		};

// 		EvmClient::new(config).await?
// 	};

// 	let op = {
// 		let config = EvmConfig {
// 			rpc_urls: vec![op_url],
// 			state_machine: StateMachine::Ethereum(Ethereum::Optimism),
// 			consensus_state_id: "ETH1".to_string(),
// 			ismp_host: Default::default(),
// 			handler: hex!("20290590DFc7ED1bd00A35a476047D70357DC081").into(),
// 			signer: "2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622".to_string(),
// 			..Default::default()
// 		};

// 		EvmClient::new(config).await?
// 	};

// 	let eth = {
// 		let config = EvmConfig {
// 			rpc_urls: vec![geth_url],
// 			state_machine: StateMachine::Ethereum(Ethereum::ExecutionLayer),
// 			consensus_state_id: "ETH1".to_string(),
// 			ismp_host: Default::default(),
// 			handler: hex!("2754c36724afBAeB0D91F08E79fdc38BBC9207ad").into(),
// 			signer: "2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622".to_string(),
// 			etherscan_api_key: Default::default(),
// 			..Default::default()
// 		};

// 		EvmClient::new(config).await?
// 	};

// 	let arb = {
// 		let config = EvmConfig {
// 			rpc_urls: vec![arb_url],
// 			state_machine: StateMachine::Ethereum(Ethereum::Arbitrum),
// 			consensus_state_id: "ETH1".to_string(),
// 			ismp_host: Default::default(),
// 			handler: hex!("83ACf4A70bd829Fdd4428819B210b0dA8F4E867d").into(),
// 			signer: "2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622".to_string(),
// 			..Default::default()
// 		};

// 		EvmClient::new(config).await?
// 	};

// 	let state_id =
// 		StateMachineId { state_id: StateMachine::Kusama(para_id), consensus_state_id: *b"PARA" };

// 	let mut stream_base = base.state_machine_update_notification(state_id.clone()).await?;
// 	let mut stream_op = op.state_machine_update_notification(state_id.clone()).await?;
// 	let mut stream_arb = arb.state_machine_update_notification(state_id.clone()).await?;
// 	let mut stream_eth = eth.state_machine_update_notification(state_id).await?;

// 	let handle_base = tokio::spawn(async move {
// 		while let Some(event) = stream_base.next().await {
// 			println!("BASE: {event:?}");
// 		}
// 	});

// 	let handle_op = tokio::spawn(async move {
// 		while let Some(event) = stream_op.next().await {
// 			println!("OP: {event:?}");
// 		}
// 	});

// 	let handle_eth = tokio::spawn(async move {
// 		while let Some(event) = stream_eth.next().await {
// 			println!("ETH: {event:?}");
// 		}
// 	});

// 	let handle_arb = tokio::spawn(async move {
// 		while let Some(event) = stream_arb.next().await {
// 			println!("ARB: {event:?}");
// 		}
// 	});

// 	let _ = join!(handle_op, handle_base, handle_eth, handle_arb);
// 	Ok(())
// }

#[tokio::test]
async fn sync_committee_tests() -> Result<(), anyhow::Error> {
	setup_logging();
	dotenv::dotenv().ok();
	let config_a = SubstrateConfig {
		state_machine: StateMachine::Kusama(2000),
		hashing: Some(HashAlgorithm::Keccak),
		consensus_state_id: Some("PARA".to_string()),
		max_rpc_payload_size: None,
		rpc_ws: "wss://hyperbridge-paseo-rpc.blockops.network".to_string(),
		signer: Some(
			"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
		),

		initial_height: None,
		poll_interval: None,
		max_concurent_queries: None,
	};

	let chain_a = SubstrateClient::<Hyperbridge>::new(config_a).await.unwrap();

	let chain_b = {
		let config = EvmConfig {
			rpc_urls: vec![
				"https://gnosis-chiado.g.alchemy.com/v2/7mvtw2J1R7l8rytkGR9Jf7KiTahCuGy4"
					.to_string(),
			],
			state_machine: StateMachine::Evm(10200),
			consensus_state_id: "GNO0".to_string(),
			ismp_host: hex!("7BdE4Ce065400eE332C20f7df3a35d66674165f6").into(),
			signer: "2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622".to_string(),
			..Default::default()
		};

		let sync_commitee_config = SyncHostConfig {
			beacon_http_urls: vec!["https://rpc-gbc.chiadochain.net".to_string()],
			consensus_update_frequency: 60,
		};

		Arc::new(
			SyncCommitteeHost::<
				sync_committee_primitives::constants::gnosis::Testnet,
				ETH1_DATA_VOTES_BOUND_GNO,
			>::new(&sync_commitee_config, &config, Default::default())
			.await?,
		)
	};

	chain_b.start_consensus(Arc::new(chain_a)).await.unwrap();

	Ok(())
}

// #[tokio::test]
// async fn fetch_eth_consensus_state() -> Result<(), anyhow::Error> {
// 	let config_a = SubstrateConfig {
// 		state_machine: StateMachine::Kusama(4009),
// 		hashing: Some(HashAlgorithm::Keccak),
// 		consensus_state_id: Some("PARA".to_string()),
// 		max_rpc_payload_size: None,
// 		rpc_ws: "ws://localhost:9944".to_string(),
// 		signer: Some(
// 			"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
// 		),
// 		initial_height: None,
// 	};poll_interval: None,
//
// 	let chain_a = SubstrateClient::<Hyperbridge>::new(config_a).await?;

// 	let consensus_state = chain_a.query_consensus_state(None, *b"ETH0").await.unwrap();

// 	let consensus_state = ConsensusState::decode(&mut &consensus_state[..]).unwrap();

// 	dbg!(consensus_state);

// 	Ok(())
// }
