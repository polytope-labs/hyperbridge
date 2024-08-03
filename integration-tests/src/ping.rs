use ismp_solidity_abi::shared_types;
use pallet_ismp::mmr::LeafIndexQuery;
use std::{
	sync::Arc,
	time::{SystemTime, UNIX_EPOCH},
};
use substrate_state_machine::HashAlgorithm;
use subxt::rpc_params;
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
use sp_core::Pair;
use tesseract_evm::{
	abi::{erc_20::Erc20, PingMessage, PingModule},
	EvmConfig,
};
use tesseract_primitives::{IsmpProvider, StateMachineUpdated};

#[tokio::test]
#[ignore]
async fn test_ping() -> anyhow::Result<()> {
	dotenv::dotenv().ok();
	let _op_url = std::env::var("OP_URL").expect("OP_URL was missing in env variables");
	let _base_url = std::env::var("BASE_URL").expect("BASE_URL was missing in env variables");
	let _arb_url = std::env::var("ARB_URL").expect("ARB_URL was missing in env variables");
	let _geth_url = std::env::var("GETH_URL").expect("GETH_URL was missing in env variables");
	let _bsc_url = std::env::var("BSC_URL").expect("BSC_URL was missing in env variables");
	let respond = option_env!("RESPOND");

	let signing_key =
		std::env::var("SIGNING_KEY").expect("SIGNING_KEY was missing in env variables");

	let ping_addr = H160(hex!("76Af4528383200CD7456E3Db967Bec309FAc583a"));

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
		signer: None,
		latest_height: None,
		max_concurent_queries: None,
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
				let ping = PingModule::new(ping_addr.clone(), client.clone());

				let host_addr = ping.host().await.context(format!("Error in {chain}"))?;
				dbg!((&chain, &host_addr));

				if respond.is_some() {
					let config = EvmConfig {
						rpc_urls: vec![url.clone()],
						ismp_host: host_addr.clone(),
						state_machine: chain.clone(),
						consensus_state_id: "PARA".to_string(),
						signer: signing_key.clone(),
						etherscan_api_key: Default::default(),
						tracing_batch_size: Default::default(),
						query_batch_size: Default::default(),
						poll_interval: Default::default(),
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
							.client
							.rpc()
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
					let call = erc_20.approve(ping_addr, U256::max_value());
					let gas = call.estimate_gas().await.context(format!("Error in {chain}"))?;
					call.gas(gas)
						.send()
						.await
						.context(format!("Failed to send approval for {ping_addr} in {chain}"))?
						.await
						.context(format!("Failed to approve {ping_addr} in {chain}"))?;

					for (chain, _, _) in chains_clone.iter().filter(|(c, _, _)| chain != *c) {
						for _ in 0..10 {
							let call = ping.ping(PingMessage {
								dest: chain.to_string().as_bytes().to_vec().into(),
								module: ping_addr.clone().into(),
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
