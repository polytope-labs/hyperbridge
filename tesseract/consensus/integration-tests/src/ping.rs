use ismp_solidity_abi::shared_types;
use pallet_ismp::offchain::LeafIndexQuery;
use std::{
	sync::Arc,
	time::{SystemTime, UNIX_EPOCH},
};
use substrate_state_machine::HashAlgorithm;
use subxt::ext::subxt_rpcs::rpc_params;
use subxt_utils::Hyperbridge;
use tesseract_substrate::{SubstrateClient, SubstrateConfig};

use anyhow::Context;
use ethers::{
	core::k256::SecretKey,
	prelude::{LocalWallet, MiddlewareBuilder, Signer},
	providers::{Http, Middleware, Provider, ProviderExt},
};
use futures::TryStreamExt;
use hex_literal::hex;
use ismp::{events::Event, host::StateMachine, router::Request};
use ismp_solidity_abi::evm_host::EvmHost;
use primitive_types::{H160, U256};
use sp_core::{Pair, H256};
use tesseract_evm::{
	abi::{erc_20::Erc20, PingMessage, PingModule},
	EvmConfig,
};
use tesseract_primitives::{IsmpProvider, StateMachineUpdated};

const PING_ADDR: H160 = H160(hex!("FE9f23F0F2fE83b8B9576d3FC94e9a7458DdDD35"));

#[tokio::test]
#[ignore]
async fn dispatch_ping() -> anyhow::Result<()> {
	dotenv::dotenv().ok();
	let _op_url = std::env::var("OP_URL").expect("OP_URL was missing in env variables");
	let _base_url = std::env::var("BASE_URL").expect("BASE_URL was missing in env variables");
	let _arb_url = std::env::var("ARB_URL").expect("ARB_URL was missing in env variables");
	let _geth_url = std::env::var("GETH_URL").expect("GETH_URL was missing in env variables");
	let _bsc_url = std::env::var("BSC_URL").expect("BSC_URL was missing in env variables");
	let respond = option_env!("RESPOND");

	let signing_key =
		std::env::var("SIGNING_KEY").expect("SIGNING_KEY was missing in env variables");

	let chains = vec![
		(StateMachine::Evm(11155111), _geth_url, 6328728),
		(StateMachine::Evm(421614), _arb_url, 64565289),
		(StateMachine::Evm(11155420), _op_url, 14717202),
		(StateMachine::Evm(84532), _base_url, 10218678),
		(StateMachine::Evm(97), _bsc_url, 42173080),
	];

	let stream = futures::stream::iter(chains.clone().into_iter().map(Ok::<_, anyhow::Error>));
	let hyperbridge_config = SubstrateConfig {
		state_machine: StateMachine::Kusama(2000),
		max_rpc_payload_size: None,
		hashing: Some(HashAlgorithm::Keccak),
		consensus_state_id: Some("PARA".to_string()),
		// rpc_ws: "wss://hyperbridge-paseo-rpc.blockops.network:443".to_string(),
		rpc_ws: "ws://127.0.0.1:9001".to_string(),
		signer: format!("{:?}", H256::random()),
		initial_height: None,
		poll_interval: None,
		max_concurent_queries: None,
		fee_token_decimals: None,
	};

	println!("Connecting .. ");
	let hyperbridge = SubstrateClient::<Hyperbridge>::new(hyperbridge_config).await?;
	println!("Connected .. ");

	stream
		.try_for_each_concurrent(None, |(chain, url, _previous_height)| {
			let chains_clone = chains.clone();
			let signing_key = signing_key.clone();
			let hyperbridge = hyperbridge.clone();
			async move {
				let signer = sp_core::ecdsa::Pair::from_seed_slice(
					&hex::decode(signing_key.clone()).unwrap(),
				)?;
				let provider = Arc::new(Provider::<Http>::try_connect(&url).await?);
				let signer = LocalWallet::from(SecretKey::from_slice(signer.seed().as_slice())?)
					.with_chain_id(provider.get_chainid().await?.low_u64());
				let client = Arc::new(provider.with_signer(signer));
				let ping = PingModule::new(PING_ADDR.clone(), client.clone());

				let host_addr = ping.host().await.context(format!("Error in {chain}"))?;
				dbg!((&chain, &host_addr));

				if respond.is_some() {
					let config = EvmConfig {
						rpc_urls: vec![url.clone()],
						ismp_host: host_addr.clone().0.into(),
						state_machine: chain.clone(),
						consensus_state_id: "PARA".to_string(),
						signer: signing_key.clone(),
						etherscan_api_key: Default::default(),
						tracing_batch_size: Default::default(),
						query_batch_size: Default::default(),
						poll_interval: Default::default(),
						initial_height: None,
						gas_price_buffer: Default::default(),
						client_type: None,
					};
					let client = config.into_client().await?;
					let latest_height = StateMachineUpdated {
						latest_height: client.client.get_block_number().await?.as_u64(),
						state_machine_id: client.state_machine_id(),
					};
					let events = client.query_ismp_events(_previous_height, latest_height).await?;
					for event in events {
						let commitment = match event {
							Event::PostRequestHandled(handled) => handled.commitment,
							_ => continue,
						};

						let request = hyperbridge
							.rpc_client
							.request::<Vec<Request>>(
								"ismp_queryRequests",
								rpc_params![vec![LeafIndexQuery { commitment }]],
							)
							.await?
							.remove(0);
						// should be a request
						let Request::Post(post) = request else {
							println!("Found {:?} instead of post request", request);
							continue;
						};

						if matches!(post.source, StateMachine::Kusama(_)) {
							continue;
						}

						let start = SystemTime::now();
						let now = start
							.duration_since(UNIX_EPOCH)
							.expect("Time went backwards")
							.as_secs();
						let response = shared_types::PostResponse {
							request: post.into(),
							response: format!("Hello from {}", chain.to_owned())
								.as_bytes()
								.to_vec()
								.into(),
							timeout_timestamp: now + (60 * 60 * 2),
						};
						let call = ping.dispatch_post_response(response);
						let gas = call
							.estimate_gas()
							.await
							.context(format!("Failed to estimate gas in {chain}"))?;
						let receipt = call
							.gas(gas)
							.send()
							.await?
							.await
							.context(format!("Failed to execute ping message on {chain}"))?;

						assert!(receipt.is_some());
					}
				} else {
					let host = EvmHost::new(host_addr, client.clone());
					let erc_20 = Erc20::new(
						host.fee_token().await.context(format!("Error in {chain}"))?,
						client.clone(),
					);
					let call = erc_20.approve(PING_ADDR, U256::max_value());
					let gas = call.estimate_gas().await.context(format!("Error in {chain}"))?;
					call.gas(gas)
						.send()
						.await
						.context(format!("Failed to send approval for {PING_ADDR} in {chain}"))?
						.await
						.context(format!("Failed to approve {PING_ADDR} in {chain}"))?;

					for (chain, _, _) in chains_clone.iter().filter(|(c, _, _)| chain != *c) {
						for _ in 0..5 {
							let call = ping.ping(PingMessage {
								dest: chain.to_string().as_bytes().to_vec().into(),
								module: PING_ADDR.clone().into(),
								timeout: 10 * 60 * 60,
								fee: U256::from(30_000_000_000_000_000_000u128),
								count: U256::from(100),
							});
							let gas = call
								.estimate_gas()
								.await
								.context(format!("Failed to estimate gas in {chain}"))?;
							let call = call.gas(gas);
							let Ok(tx) = call.send().await else { continue };
							let receipt = tx
								.await
								.context(format!("Failed to execute ping message on {chain}"))?;

							assert!(receipt.is_some());
						}
					}
				}

				Ok(())
			}
		})
		.await?;

	Ok(())
}

// #[tokio::test]
// #[ignore]
// async fn test_ping_get_request() -> anyhow::Result<()> {
// 	dotenv::dotenv().ok();
// 	let _bsc_url = std::env::var("BSC_URL").expect("BSC_URL was missing in env variables");
// 	let _geth_url = std::env::var("GETH_URL").expect("GETH_URL was missing in env variables");

// 	let signing_key =
// 		std::env::var("SIGNING_KEY").expect("SIGNING_KEY was missing in env variables");

// 	let hyperbridge_config = SubstrateConfig {
// 		state_machine: StateMachine::Kusama(2000),
// 		max_rpc_payload_size: None,
// 		hashing: Some(HashAlgorithm::Keccak),
// 		consensus_state_id: Some("PARA".to_string()),
// 		// rpc_ws: "wss://hyperbridge-paseo-rpc.blockops.network:443".to_string(),
// 		rpc_ws: "ws://127.0.0.1:9001".to_string(),
// 		signer: None,
// 		initial_height: None,
// 		poll_interval: None,
// 		max_concurent_queries: None,
// 	};

// 	// sepolia host
// 	let sepolia_host = H160(hex!("F1c7a386325B7D22025D7542b28Ee881Cdf107b3"));

// 	let config = EvmConfig {
// 		rpc_urls: vec![_geth_url.clone()],
// 		ismp_host: sepolia_host.clone(),
// 		state_machine: StateMachine::Evm(11155111),
// 		consensus_state_id: "ETH0".to_string(),
// 		signer: signing_key.clone(),
// 		etherscan_api_key: Default::default(),
// 		tracing_batch_size: Default::default(),
// 		query_batch_size: Default::default(),
// 		poll_interval: Default::default(),
// 		gas_price_buffer: Default::default(),
// 		initial_height: None,
// 		client_type: None,
// 	};
// 	let sepolia_client = config.into_client().await?;

// 	println!("Connecting .. ");
// 	let hyperbridge = SubstrateClient::<Hyperbridge>::new(hyperbridge_config).await?;
// 	println!("Connected .. ");

// 	let signer = sp_core::ecdsa::Pair::from_seed_slice(&hex::decode(signing_key.clone()).unwrap())?;
// 	let provider = Arc::new(Provider::<Http>::try_connect(&_bsc_url).await?);
// 	let signer = LocalWallet::from(SecretKey::from_slice(signer.seed().as_slice())?)
// 		.with_chain_id(provider.get_chainid().await?.low_u64());
// 	let bsc_client = Arc::new(provider.with_signer(signer));
// 	let ping = PingModule::new(PING_ADDR.clone(), bsc_client.clone());

// 	let latest_sepolia_height = hyperbridge
// 		.query_latest_height(StateMachineId {
// 			state_id: StateMachine::Evm(11155111),
// 			consensus_state_id: *b"ETH0",
// 		})
// 		.await?;

// 	// We'll query the state commitment of the latest hyperbridge height from the evm host on
// 	// sepolia

// 	let contract = EvmHost::new(sepolia_host, sepolia_client.client.clone());
// 	let latest_hyperbridge_height = contract
// 		.latest_state_machine_height(4009u32.into())
// 		.block(BlockId::Number(ethers::types::BlockNumber::Number(latest_sepolia_height.into())))
// 		.call()
// 		.await?;

// 	dbg!(latest_hyperbridge_height);
// 	let keys = {
// 		let keys = state_comitment_key(4009u32.into(), latest_hyperbridge_height);
// 		let key_1 = {
// 			let mut bytes = sepolia_host.0.to_vec();
// 			bytes.extend_from_slice(keys.0.as_bytes());
// 			bytes
// 		};

// 		let key_2 = {
// 			let mut bytes = sepolia_host.0.to_vec();
// 			bytes.extend_from_slice(keys.1.as_bytes());
// 			bytes
// 		};

// 		let key_3 = {
// 			let mut bytes = sepolia_host.0.to_vec();
// 			bytes.extend_from_slice(keys.2.as_bytes());
// 			bytes
// 		};

// 		vec![key_1.into(), key_2.into(), key_3.into()]
// 	};

// 	let state = sepolia_client
// 		.query_state_machine_commitment(StateMachineHeight {
// 			id: StateMachineId {
// 				state_id: StateMachine::Kusama(4009),
// 				consensus_state_id: *b"PARA",
// 			},
// 			height: latest_hyperbridge_height.low_u64(),
// 		})
// 		.await?;

// 	dbg!(state);

// 	let get_request = GetRequest {
// 		source: Default::default(),
// 		dest: StateMachine::Evm(11155111).to_string().as_bytes().to_vec().into(),
// 		nonce: Default::default(),
// 		from: Default::default(),
// 		timeout_timestamp: 0,
// 		context: Default::default(),
// 		keys,
// 		height: latest_sepolia_height.into(),
// 	};

// 	let call = ping.dispatch_with_request(get_request);
// 	let gas = call.estimate_gas().await.context(format!("Failed to estimate gas"))?;
// 	let _receipt = call
// 		.gas(gas)
// 		.send()
// 		.await?
// 		.await
// 		.context(format!("Failed to execute ping message"))?;

// 	Ok(())
// }
