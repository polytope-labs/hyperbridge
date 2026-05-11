use alloy::{
	primitives::{Address, U256 as AlloyU256},
	providers::{Provider, ProviderBuilder},
	signers::local::PrivateKeySigner,
};
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
use futures::TryStreamExt;
use hex_literal::hex;
use ismp::{events::Event, host::StateMachine, router::Request};
use ismp_solidity_abi::{
	erc20::ERC20Instance,
	evm_host::EvmHostInstance,
	ping_module::{PingMessage, PingModuleInstance, PostRequest as SolPostRequest, PostResponse},
};
use sp_core::{Pair, H160, H256};
use tesseract_evm::EvmConfig;
use tesseract_primitives::{IsmpProvider, StateMachineUpdated};

const PING_ADDR: Address = Address::new(hex!("FE9f23F0F2fE83b8B9576d3FC94e9a7458DdDD35"));

#[tokio::test]
#[ignore]
async fn dispatch_ping() -> anyhow::Result<()> {
	dotenv::dotenv().ok();
	// let _polygon_url = std::env::var("POLYGON_URL").expect("OP_URL was missing in env
	// variables");
	// let _arb_url = std::env::var("ARBITRUM_URL").expect("ARB_URL was missing in env variables");
	let _sepolia_url = std::env::var("SEPOLIA_URL").expect("GETH_URL was missing in env variables");
	let _bsc_url = std::env::var("BSC_URL").expect("BSC_URL was missing in env variables");
	let respond = option_env!("RESPOND");

	// println!("{_arb_url}\n{_sepolia_url}\n{_bsc_url}");

	let signing_key =
		std::env::var("SIGNING_KEY").expect("SIGNING_KEY was missing in env variables");

	let chains = vec![
		(StateMachine::Evm(11155111), _sepolia_url, 6328728),
		// (StateMachine::Evm(421614), _arb_url, 64565289),
		// (StateMachine::Evm(11155420), _op_url, 14717202),
		// (StateMachine::Evm(84532), _base_url, 10218678),
		(StateMachine::Evm(97), _bsc_url, 42173080),
	];

	let stream = futures::stream::iter(chains.clone().into_iter().map(Ok::<_, anyhow::Error>));
	let hyperbridge_config = SubstrateConfig {
		state_machine: Some(StateMachine::Kusama(2000)),
		max_rpc_payload_size: None,
		hashing: Some(HashAlgorithm::Keccak),
		consensus_state_id: Some("PAS0".to_string()),
		rpc_ws: "wss://gargantua.rpc.polytope.technology".to_string(),
		// rpc_ws: "ws://127.0.0.1:9001".to_string(),
		signer: Some(format!("{:?}", H256::random())),
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
				let signer_pair = sp_core::ecdsa::Pair::from_seed_slice(
					&hex::decode(signing_key.clone()).unwrap(),
				)?;
				let signing_key_bytes = alloy::signers::k256::ecdsa::SigningKey::from_slice(
					signer_pair.seed().as_slice(),
				)?;
				let wallet = PrivateKeySigner::from_signing_key(signing_key_bytes);
				let wallet = alloy::network::EthereumWallet::from(wallet);
				let provider = ProviderBuilder::new().wallet(wallet).connect_http(url.parse()?);
				let client = Arc::new(provider);
				let ping = PingModuleInstance::new(PING_ADDR, client.clone());

				let host_addr =
					Address::from(ping.host().call().await.context(format!("Error in {chain}"))?.0);
				dbg!((&chain, &host_addr));

				if respond.is_some() {
					let config = EvmConfig {
						rpc_urls: vec![url.clone()],
						ismp_host: Some(H160::from_slice(host_addr.as_slice())),
						state_machine: Some(chain.clone()),
						consensus_state_id: Some("PAS0".to_string()),
						signer: Some(signing_key.clone()),
						tracing_batch_size: Default::default(),
						query_batch_size: Default::default(),
						poll_interval: Default::default(),
						initial_height: None,
						gas_price_buffer: Default::default(),
						client_type: None,
						transport: tesseract_evm::transport::RpcTransport::Standard,
					};
					let client = config.into_client().await?;
					let latest_height = StateMachineUpdated {
						latest_height: Provider::get_block_number(&*client.client).await?,
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
						let response = PostResponse {
							request: SolPostRequest {
								source: post.source.to_string().into_bytes().into(),
								dest: post.dest.to_string().into_bytes().into(),
								nonce: post.nonce,
								from: post.from.into(),
								to: post.to.into(),
								timeoutTimestamp: post.timeout_timestamp,
								body: post.body.into(),
							},
							response: format!("Hello from {}", chain.to_owned())
								.as_bytes()
								.to_vec()
								.into(),
							timeoutTimestamp: now + (60 * 60 * 2),
						};
						let call = ping.dispatchPostResponse(response);
						let gas = call
							.estimate_gas()
							.await
							.context(format!("Failed to estimate gas in {chain}"))?;
						let receipt = call
							.gas(gas)
							.send()
							.await?
							.get_receipt()
							.await
							.context(format!("Failed to execute ping message on {chain}"))?;

						assert!(receipt.status());
					}
				} else {
					let host = EvmHostInstance::new(host_addr, client.clone());
					let fee_token = Address::from(
						host.feeToken().call().await.context(format!("Error in {chain}"))?.0,
					);
					let erc_20 = ERC20Instance::new(fee_token, client.clone());
					let call = erc_20.approve(PING_ADDR, AlloyU256::MAX);
					let gas = call.estimate_gas().await.context(format!("Error in {chain}"))?;
					call.gas(gas)
						.send()
						.await
						.context(format!("Failed to send approval for {PING_ADDR} in {chain}"))?
						.get_receipt()
						.await
						.context(format!("Failed to approve {PING_ADDR} in {chain}"))?;

					for (chain, _, _) in chains_clone.iter().filter(|(c, _, _)| chain != *c) {
						for _ in 0..1 {
							let call = ping.ping(PingMessage {
								dest: chain.to_string().as_bytes().to_vec().into(),
								module: PING_ADDR,
								timeout: 10 * 60 * 60,
								fee: AlloyU256::from(30_000_000_000_000_000_000u128),
								count: AlloyU256::from(5),
							});
							let gas = call
								.estimate_gas()
								.await
								.context(format!("Failed to estimate gas in {chain}"))?;
							let call = call.gas(gas);
							let Ok(tx) = call.send().await else { continue };
							let receipt = tx
								.get_receipt()
								.await
								.context(format!("Failed to execute ping message on {chain}"))?;

							assert!(receipt.status());
						}
					}
				}

				Ok(())
			}
		})
		.await?;

	Ok(())
}

/// Issue a single `ping.ping(...)` from the source PingModule on Polkadot
/// Asset Hub Paseo (substrate-EVM, chain id 420420417) targeting the
/// destination PingModule on BSC Chapel (chain id 97).
///
/// Run with the relevant env vars set:
/// ```bash
/// ASSETHUB_URL=https://eth-rpc-testnet.polkadot.io/ \
/// SIGNING_KEY=<hex-encoded ECDSA seed> \
/// cargo test --release -p tesseract-integration-tests \
///   --test integration_tests dispatch_ping_assethub_to_bsc \
///   -- --ignored --nocapture
/// ```
///
/// Mirrors `dispatch_ping`'s structure but pinned to a single source/dest
/// pair and the addresses called out in the task — separate test so the
/// fee-token approval and the ping send don't have to be threaded through
/// the iterator-based fan-out.
#[tokio::test]
#[ignore]
async fn dispatch_ping_assethub_to_bsc() -> anyhow::Result<()> {
	dotenv::dotenv().ok();
	let assethub_url =
		std::env::var("ASSETHUB_URL").expect("ASSETHUB_URL was missing in env variables");
	let signing_key =
		std::env::var("SIGNING_KEY").expect("SIGNING_KEY was missing in env variables");

	// Source PingModule on Polkadot Asset Hub Paseo Revive.
	const SOURCE_PING: Address = Address::new(hex!("11e24eb75b27a4a48ada1c5fb036fa8e718b32b4"));
	// Destination PingModule on BSC Chapel (matches the constant the
	// fan-out test uses).
	const DEST_PING: Address = Address::new(hex!("FE9f23F0F2fE83b8B9576d3FC94e9a7458DdDD35"));
	// Asset Hub Paseo Revive — chain id from the registry.
	const SOURCE_CHAIN: StateMachine = StateMachine::Evm(420420417);
	// BSC Chapel — chain id from the registry.
	const DEST_CHAIN: StateMachine = StateMachine::Evm(97);

	let signer_pair = sp_core::ecdsa::Pair::from_seed_slice(&hex::decode(signing_key.clone())?)?;
	let signing_key_bytes =
		alloy::signers::k256::ecdsa::SigningKey::from_slice(signer_pair.seed().as_slice())?;
	let wallet = PrivateKeySigner::from_signing_key(signing_key_bytes);
	let wallet = alloy::network::EthereumWallet::from(wallet);
	let provider = ProviderBuilder::new().wallet(wallet).connect_http(assethub_url.parse()?);
	let client = Arc::new(provider);
	let ping = PingModuleInstance::new(SOURCE_PING, client.clone());

	// Fetch the source IsmpHost from the ping module so we can resolve its
	// fee token. The ping module stores a pointer to its host on
	// construction, so this stays correct across host migrations.
	let host_addr = Address::from(
		ping.host()
			.call()
			.await
			.context(format!("ping.host() failed on {SOURCE_CHAIN}"))?
			.0,
	);
	let host = EvmHostInstance::new(host_addr, client.clone());
	let fee_token = Address::from(
		host.feeToken()
			.call()
			.await
			.context(format!("host.feeToken() failed on {SOURCE_CHAIN}"))?
			.0,
	);

	// Allow the source ping module to pull fees out of the relayer's
	// account. Idempotent: re-running approves up to MAX again, which is
	// a no-op past the first run.
	let erc20 = ERC20Instance::new(fee_token, client.clone());
	let approve = erc20.approve(SOURCE_PING, AlloyU256::MAX);
	let gas = approve
		.estimate_gas()
		.await
		.context(format!("approve gas estimate failed on {SOURCE_CHAIN}"))?;
	approve
		.gas(gas)
		.send()
		.await
		.context(format!("approve send failed on {SOURCE_CHAIN}"))?
		.get_receipt()
		.await
		.context(format!("approve receipt failed on {SOURCE_CHAIN}"))?;

	// Single PostRequest via `ping.dispatch(request)`. The host fills
	// `source`/`nonce`/`from` itself when it dispatches, so those input
	// fields are informational; only `dest`, `to`, `body`, and the
	// timeout matter on this side. Unlike `ping.ping(...)`, dispatch
	// pays just the per-byte fee with no relayer-incentive on top — if
	// no relayer picks the message up there's no extra reward. Bump the
	// timeout if you want a longer holdout.
	let now = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("system time before epoch")
		.as_secs();
	let request = SolPostRequest {
		source: SOURCE_CHAIN.to_string().as_bytes().to_vec().into(),
		dest: DEST_CHAIN.to_string().as_bytes().to_vec().into(),
		nonce: 0, // overwritten by the host
		from: SOURCE_PING.0.to_vec().into(),
		to: DEST_PING.0.to_vec().into(),
		timeoutTimestamp: now + 10 * 60 * 60,
		body: b"hello from assethub".to_vec().into(),
	};
	let call = ping.dispatch_0(request);
	let gas = call
		.estimate_gas()
		.await
		.context(format!("dispatch gas estimate failed on {SOURCE_CHAIN}"))?;
	let receipt = call
		.gas(gas)
		.send()
		.await
		.context(format!("dispatch send failed on {SOURCE_CHAIN}"))?
		.get_receipt()
		.await
		.context(format!("dispatch receipt failed on {SOURCE_CHAIN}"))?;

	assert!(
		receipt.status(),
		"dispatch tx reverted on {SOURCE_CHAIN}: {:?}",
		receipt.transaction_hash,
	);
	println!(
		"PostRequest dispatched: {SOURCE_CHAIN} {SOURCE_PING} -> {DEST_CHAIN} {DEST_PING}, tx={:?}",
		receipt.transaction_hash,
	);

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
