use anyhow::Context;
use ethers::{
	contract::parse_log,
	core::k256::SecretKey,
	prelude::{
		transaction::eip2718::TypedTransaction, LocalWallet, Middleware, MiddlewareBuilder,
		NameOrAddress, Provider, Signer, TransactionRequest, U256,
	},
	providers::{Http, ProviderExt},
	types::H160,
	utils::hex,
};

use crate::{
	internals,
	internals::{request_status_stream, timeout_request_stream},
	providers::interface::Client,
	types::{
		ChainConfig, ClientConfig, EvmConfig, HashAlgorithm, MessageStatusWithMetadata,
		SubstrateConfig, TimeoutStatus,
	},
	HyperClient,
};
use futures::StreamExt;
use hex_literal::hex;
use ismp::{
	consensus::StateMachineId,
	host::{Ethereum, StateMachine},
	router,
};
use ismp_solidity_abi::{
	erc20::ERC20,
	evm_host::{EvmHost, PostRequestEventFilter},
	ping_module::{PingMessage, PingModule},
};
use std::sync::Arc;

const OP_HOST: H160 = H160(hex!("98A6A9c780D7d54aEF9aBd08c55C9ADc17de48A0"));
const OP_HANDLER: H160 = H160(hex!("BE5Ea8A28D5AB4e96532c85F078d83E138EC8d44"));

const SEPOLIA_HOST: H160 = H160(hex!("705ba08d072f26bB92C8f71B7E37a3c7aEA5Ecb1"));
const SEPOLIA_HANDLER: H160 = H160(hex!("12122B8E2E1e24345161A2433fF2d91Fca123FAe"));

const BSC_HOST: H160 = H160(hex!("1488f22baf2027D6e28D79b6F38EB2CfdfE48d62"));
const BSC_HANDLER: H160 = H160(hex!("13463B01CE69F8B5279c5922D38EB1073503e87b"));

const PING_MODULE: H160 = H160(hex!("b51d235CF4461D17FEA88733fed1865873c8D686"));

pub async fn subscribe_to_request_status() -> Result<(), anyhow::Error> {
	tracing::info!("\n\n\n\nStarting request status subscription\n\n\n\n");

	let signing_key = env!("SIGNING_KEY").to_string();
	let bsc_url = env!("BSC_URL").to_string();
	let op_url = env!("OP_URL").to_string();

	let source_chain = EvmConfig {
		rpc_url: bsc_url.clone(),
		state_machine: StateMachine::Bsc,
		host_address: BSC_HOST,
		handler_address: BSC_HANDLER,
		consensus_state_id: *b"BSC0",
	};

	let dest_chain = EvmConfig {
		rpc_url: op_url,
		state_machine: StateMachine::Ethereum(Ethereum::Optimism),
		host_address: OP_HOST,
		handler_address: OP_HANDLER,
		consensus_state_id: *b"ETH0",
	};

	let hyperbrige_config = SubstrateConfig {
		rpc_url: "wss://hyperbridge-paseo-rpc.blockops.network:443".to_string(),
		// rpc_url: "ws://127.0.0.1:9901".to_string(),
		consensus_state_id: *b"PARA",
		hash_algo: HashAlgorithm::Keccak,
	};
	let config = ClientConfig {
		source: ChainConfig::Evm(source_chain.clone()),
		dest: ChainConfig::Evm(dest_chain.clone()),
		hyperbridge: ChainConfig::Substrate(hyperbrige_config),
	};
	let hyperclient = HyperClient::new(config).await?;

	// Send Ping Message
	let signer = hex::decode(signing_key).unwrap();
	let provider = Arc::new(Provider::<Http>::try_connect(&bsc_url).await?);
	let signer = LocalWallet::from(SecretKey::from_slice(signer.as_slice())?)
		.with_chain_id(provider.get_chainid().await?.low_u64());
	let client = Arc::new(provider.with_signer(signer));
	let ping = PingModule::new(PING_MODULE, client.clone());
	let chain = StateMachine::Bsc;
	let host_addr = ping.host().await.context(format!("Error in {chain:?}"))?;
	let host = EvmHost::new(host_addr, client.clone());
	let erc_20 =
		ERC20::new(host.fee_token().await.context(format!("Error in {chain:?}"))?, client.clone());
	let call = erc_20.approve(PING_MODULE, U256::max_value());

	let gas = call.estimate_gas().await.context(format!("Error in {chain:?}"))?;
	call.gas(gas)
		.send()
		.await
		.context(format!("Error in {chain:?}"))?
		.await
		.context(format!("Error in {chain:?}"))?;
	let call = ping.ping(PingMessage {
		dest: dest_chain.state_machine.to_string().as_bytes().to_vec().into(),
		module: PING_MODULE.clone().into(),
		timeout: 10 * 60 * 60,
		fee: U256::from(90_000_000_000_000_000_000u128),
		count: U256::from(1),
	});
	let gas = call.estimate_gas().await.context(format!("Error in {chain:?}"))?;
	let receipt = call
		.gas(gas)
		.send()
		.await
		.context(format!("Error in {chain:?}"))?
		.await
		.context(format!("Error in {chain:?}"))?
		.unwrap();

	let post: router::Post = receipt
		.logs
		.into_iter()
		.find_map(|log| parse_log::<PostRequestEventFilter>(log).ok())
		.expect("Tx should emit post request")
		.try_into()?;
	tracing::info!("Got PostRequest {post}");
	let block = receipt.block_number.unwrap();
	tracing::info!("\n\nTx block: {block}\n\n");

	let mut stream = request_status_stream(&hyperclient, post, block.low_u64()).await;

	while let Some(res) = stream.next().await {
		match res {
			Ok(status) => {
				tracing::info!("Got Status {:#?}", status);
			},
			Err(e) => {
				tracing::info!("Error: {e:#?}");
				Err(e)?
			},
		}
	}
	Ok(())
}

pub async fn test_timeout_request() -> Result<(), anyhow::Error> {
	tracing::info!("\n\n\n\nStarting timeout request test\n\n\n\n");

	let signing_key = env!("SIGNING_KEY").to_string();
	let bsc_url = env!("BSC_URL").to_string();
	let sepolia_url = env!("SEPOLIA_URL").to_string();
	let source_chain = EvmConfig {
		rpc_url: bsc_url.clone(),
		state_machine: StateMachine::Bsc,
		host_address: BSC_HOST,
		handler_address: BSC_HANDLER,
		consensus_state_id: *b"BSC0",
	};

	let dest_chain = EvmConfig {
		rpc_url: sepolia_url,
		state_machine: StateMachine::Ethereum(Ethereum::ExecutionLayer),
		host_address: SEPOLIA_HOST,
		handler_address: SEPOLIA_HANDLER,
		consensus_state_id: *b"ETH0",
	};

	let hyperbrige_config = SubstrateConfig {
		rpc_url: "wss://hyperbridge-paseo-rpc.blockops.network:443".to_string(),
		// rpc_url: "ws://127.0.0.1:9901".to_string(),
		consensus_state_id: *b"PARA",
		hash_algo: HashAlgorithm::Keccak,
	};
	let config = ClientConfig {
		source: ChainConfig::Evm(source_chain.clone()),
		dest: ChainConfig::Evm(dest_chain.clone()),
		hyperbridge: ChainConfig::Substrate(hyperbrige_config),
	};
	let hyperclient = HyperClient::new(config).await?;

	// Send Ping Message
	let pair = hex::decode(signing_key).unwrap();
	let provider = Arc::new(Provider::<Http>::try_connect(&bsc_url).await?);
	let chain_id = provider.get_chainid().await?.low_u64();
	let signer = LocalWallet::from(SecretKey::from_slice(pair.as_slice())?).with_chain_id(chain_id);
	let client = Arc::new(provider.with_signer(signer));
	let ping = PingModule::new(PING_MODULE, client.clone());
	let chain = StateMachine::Bsc;
	let host_addr = ping.host().await.context(format!("Error in {chain:?}"))?;
	let host = EvmHost::new(host_addr, client.clone());
	tracing::info!("{:#?}", host.host_params().await?);

	let erc_20 =
		ERC20::new(host.fee_token().await.context(format!("Error in {chain:?}"))?, client.clone());
	let call = erc_20.approve(PING_MODULE, U256::max_value());

	let gas = call.estimate_gas().await.context(format!("Error in {chain:?}"))?;
	call.gas(gas)
		.send()
		.await
		.context(format!("Error in {chain:?}"))?
		.await
		.context(format!("Error in {chain:?}"))?;

	let mut stream = hyperclient
		.hyperbridge
		.state_machine_update_notification(StateMachineId {
			state_id: StateMachine::Bsc,
			consensus_state_id: *b"BSC0",
		})
		.await?;
	// wait for a bsc update, before sending request
	while let Some(res) = stream.next().await {
		match res {
			Ok(_) => {
				tracing::info!("\n\nGot State Machine update for BSC\n\n");
				break;
			},
			_ => {},
		}
	}

	let call = ping.ping(PingMessage {
		dest: dest_chain.state_machine.to_string().as_bytes().to_vec().into(),
		module: PING_MODULE.clone().into(),
		timeout: 4 * 60,
		fee: U256::from(0u128),
		count: U256::from(1),
	});
	let gas = call.estimate_gas().await.context(format!("Error in {chain:?}"))?;
	let receipt = call
		.gas(gas)
		.send()
		.await
		.context(format!("Error in {chain:?}"))?
		.await
		.context(format!("Error in {chain:?}"))?
		.unwrap();

	let block = receipt.block_number.unwrap();
	tracing::info!("\n\nTx block: {block}\n\n");

	let post: router::Post = receipt
		.logs
		.into_iter()
		.find_map(|log| parse_log::<PostRequestEventFilter>(log).ok())
		.expect("Tx should emit post request")
		.try_into()?;
	tracing::info!("PostRequest {post}");

	let block = receipt.block_number.unwrap();
	tracing::info!("\n\nTx block: {block}\n\n");

	let request_status = request_status_stream(&hyperclient, post.clone(), block.low_u64()).await;

	// Obtaining the request stream and the timeout stream
	let timed_out =
		internals::request_timeout_stream(post.timeout_timestamp, hyperclient.source.clone()).await;

	let mut stream = futures::stream::select(request_status, timed_out);

	while let Some(item) = stream.next().await {
		match item {
			Ok(status) => {
				tracing::info!("\nGot Status {status:#?}\n");
				match status {
					MessageStatusWithMetadata::Timeout => break,
					_ => {},
				};
			},
			Err(err) => {
				tracing::error!("Got error in request_status_stream: {err:#?}")
			},
		}
	}

	let mut stream = timeout_request_stream(&hyperclient, post).await?;

	while let Some(res) = stream.next().await {
		match res {
			Ok(status) => {
				tracing::info!("\nGot Status {:#?}\n", status);
				match status {
					TimeoutStatus::TimeoutMessage { calldata } => {
						let gas_price = client.get_gas_price().await?;
						tracing::info!("Sending timeout to BSC");
						let pending = client
							.send_transaction(
								TypedTransaction::Legacy(TransactionRequest {
									to: Some(NameOrAddress::Address(source_chain.handler_address)),
									gas_price: Some(gas_price * 5), // experiment with higher?
									data: Some(calldata.into()),
									..Default::default()
								}),
								None,
							)
							.await;
						tracing::info!("Send transaction result: {pending:#?}");
						let result = pending?.await;
						tracing::info!("Transaction Receipt: {result:#?}");
					},
					_ => {},
				}
			},
			Err(e) => {
				tracing::info!("{e:?}")
			},
		}
	}
	Ok(())
}
