// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::Context;
use ethers::{
	contract::parse_log,
	core::k256::SecretKey,
	prelude::{
		transaction::eip2718::TypedTransaction, LocalWallet, Middleware, MiddlewareBuilder,
		NameOrAddress, Provider, Signer, TransactionRequest,
	},
	providers::{Http, ProviderExt},
	utils::hex,
};

use primitive_types::H160;

use crate::{
	internals,
	internals::{post_request_status_stream, timeout_post_request_stream},
	providers::interface::Client,
	types::{
		ChainConfig, ClientConfig, EvmConfig, HashAlgorithm, MessageStatusStreamState,
		MessageStatusWithMetadata, SubstrateConfig, TimeoutStatus, TimeoutStreamState,
	},
	HyperClient,
};
use futures::StreamExt;
use hex_literal::hex;
use ismp::{
	consensus::StateMachineId,
	host::StateMachine,
	router::{self, Request},
};
use ismp_solidity_abi::{
	beefy::GetRequest,
	erc20::ERC20,
	evm_host::{EvmHost, GetRequestEventFilter, PostRequestEventFilter},
	ping_module::{PingMessage, PingModule},
};
use std::sync::Arc;

const OP_HOST: H160 = H160(hex!("6d51b678836d8060d980605d2999eF211809f3C2"));
const SEPOLIA_HOST: H160 = H160(hex!("2EdB74C269948b60ec1000040E104cef0eABaae8"));
const BSC_HOST: H160 = H160(hex!("8Aa0Dea6D675d785A882967Bf38183f6117C09b7"));
const PING_MODULE: H160 = H160(hex!("FE9f23F0F2fE83b8B9576d3FC94e9a7458DdDD35"));

pub async fn subscribe_to_request_status() -> Result<(), anyhow::Error> {
	tracing::info!("\n\n\n\nStarting request status subscription\n\n\n\n");

	let signing_key = std::env::var("SIGNING_KEY").unwrap_or_else(|_| "".to_string());
	let bsc_url = std::env::var("BSC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string());
	let op_url = std::env::var("OP_URL").unwrap_or_else(|_| "http://localhost:8546".to_string());

	let source_chain = EvmConfig {
		rpc_url: bsc_url.clone(),
		state_machine: StateMachine::Evm(97),
		host_address: BSC_HOST,
		consensus_state_id: *b"BSC0",
	};

	let dest_chain = EvmConfig {
		rpc_url: op_url,
		state_machine: StateMachine::Evm(11155420),
		host_address: OP_HOST,
		consensus_state_id: *b"ETH0",
	};

	let hyperbrige_config = SubstrateConfig {
		state_machine: StateMachine::Kusama(4009),

		rpc_url: "wss://hyperbridge-paseo-rpc.blockops.network:443".to_string(),
		// rpc_url: "ws://127.0.0.1:9001".to_string(),
		consensus_state_id: *b"PARA",
		hash_algo: HashAlgorithm::Keccak,
	};
	let config = ClientConfig {
		source: ChainConfig::Evm(source_chain.clone()),
		dest: ChainConfig::Evm(dest_chain.clone()),
		hyperbridge: ChainConfig::Substrate(hyperbrige_config),
		tracing: false,
	};
	let hyperclient = HyperClient::new(config).await?;

	// Send Ping Message
	let signer = hex::decode(signing_key).unwrap();
	let provider = Arc::new(Provider::<Http>::try_connect(&bsc_url).await?);
	let signer = LocalWallet::from(SecretKey::from_slice(signer.as_slice())?)
		.with_chain_id(provider.get_chainid().await?.low_u64());
	let client = Arc::new(provider.with_signer(signer));
	let ping = PingModule::new(PING_MODULE.0, client.clone());
	let chain = StateMachine::Evm(97);
	let host_addr = ping.host().await.context(format!("Error in {chain:?}"))?;
	let host = EvmHost::new(host_addr, client.clone());
	let erc_20 =
		ERC20::new(host.fee_token().await.context(format!("Error in {chain:?}"))?, client.clone());
	let call = erc_20.approve(PING_MODULE.0.into(), ethers::types::U256::max_value());

	let gas = call.estimate_gas().await.context(format!("Error in {chain:?}"))?;
	call.gas(gas)
		.send()
		.await
		.context(format!("Error in {chain:?}"))?
		.await
		.context(format!("Error in {chain:?}"))?;
	let call = ping.ping(PingMessage {
		dest: dest_chain.state_machine.to_string().as_bytes().to_vec().into(),
		module: PING_MODULE.clone().0.into(),
		timeout: 10 * 60 * 60,
		fee: ethers::types::U256::from(90_000_000_000_000_000_000u128),
		count: ethers::types::U256::from(1),
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

	let post: router::PostRequest = receipt
		.logs
		.into_iter()
		.find_map(|log| parse_log::<PostRequestEventFilter>(log).ok())
		.expect("Tx should emit post request")
		.try_into()?;
	tracing::info!("Got PostRequest {post}");
	let block = receipt.block_number.unwrap();
	tracing::info!("\n\nTx block: {block}\n\n");

	let mut stream = post_request_status_stream(
		&hyperclient,
		post,
		MessageStatusStreamState::Dispatched(block.low_u64()),
	)
	.await?;

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

	let signing_key = std::env::var("SIGNING_KEY").unwrap_or_else(|_| "".to_string());
	let bsc_url = std::env::var("BSC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string());
	let sepolia_url =
		std::env::var("SEPOLIA_URL").unwrap_or_else(|_| "http://localhost:8547".to_string());
	let source_chain = EvmConfig {
		rpc_url: bsc_url.clone(),
		state_machine: StateMachine::Evm(97),
		host_address: BSC_HOST,
		consensus_state_id: *b"BSC0",
	};

	let dest_chain = EvmConfig {
		rpc_url: sepolia_url,
		state_machine: StateMachine::Evm(11155111),
		host_address: SEPOLIA_HOST,
		consensus_state_id: *b"ETH0",
	};

	let hyperbrige_config = SubstrateConfig {
		state_machine: StateMachine::Kusama(4009),

		rpc_url: "wss://hyperbridge-paseo-rpc.blockops.network:443".to_string(),
		// rpc_url: "ws://127.0.0.1:9001".to_string(),
		consensus_state_id: *b"PARA",
		hash_algo: HashAlgorithm::Keccak,
	};
	let config = ClientConfig {
		source: ChainConfig::Evm(source_chain.clone()),
		dest: ChainConfig::Evm(dest_chain.clone()),
		hyperbridge: ChainConfig::Substrate(hyperbrige_config),
		tracing: false,
	};
	let hyperclient = HyperClient::new(config).await?;

	// Send Ping Message
	let pair = hex::decode(signing_key).unwrap();
	let provider = Arc::new(Provider::<Http>::try_connect(&bsc_url).await?);
	let chain_id = provider.get_chainid().await?.low_u64();
	let signer = LocalWallet::from(SecretKey::from_slice(pair.as_slice())?).with_chain_id(chain_id);
	let client = Arc::new(provider.with_signer(signer));
	let ping = PingModule::new(PING_MODULE.0, client.clone());
	let chain = StateMachine::Evm(97);
	let host_addr = ping.host().await.context(format!("Error in {chain:?}"))?;
	let host = EvmHost::new(host_addr, client.clone());
	let host_params = host.host_params().await?;
	tracing::info!("{:#?}", host_params);

	let erc_20 =
		ERC20::new(host.fee_token().await.context(format!("Error in {chain:?}"))?, client.clone());
	let call = erc_20.approve(PING_MODULE.0.into(), ethers::types::U256::max_value());

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
			state_id: StateMachine::Evm(97),
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
		module: PING_MODULE.clone().0.into(),
		timeout: 3 * 60,
		fee: ethers::types::U256::from(0u128),
		count: ethers::types::U256::from(1),
	});
	let gas = call.estimate_gas().await.context(format!("Estimate gas error in {chain:?}"))?;
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

	let post: router::PostRequest = receipt
		.logs
		.into_iter()
		.find_map(|log| parse_log::<PostRequestEventFilter>(log).ok())
		.expect("Tx should emit post request")
		.try_into()?;
	tracing::info!("PostRequest {post}");

	let block = receipt.block_number.unwrap();
	tracing::info!("\n\nTx block: {block}\n\n");

	let request_status = post_request_status_stream(
		&hyperclient,
		post.clone(),
		MessageStatusStreamState::Dispatched(block.low_u64()),
	)
	.await?;

	// Obtaining the request stream and the timeout stream
	let timed_out = internals::message_timeout_stream(
		post.timeout_timestamp,
		hyperclient.source.clone(),
		Request::Post(post.clone()),
	)
	.await;

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

	let mut stream =
		timeout_post_request_stream(&hyperclient, post, TimeoutStreamState::Pending).await?;

	while let Some(res) = stream.next().await {
		match res {
			Ok(status) => {
				tracing::info!("\nGot Status {:#?}\n", status);
				match status {
					TimeoutStatus::HyperbridgeFinalized { calldata, .. } => {
						let gas_price = client.get_gas_price().await?;
						tracing::info!("Sending timeout to BSC");
						let pending = client
							.send_transaction(
								TypedTransaction::Legacy(TransactionRequest {
									to: Some(NameOrAddress::Address(host_params.handler)),
									gas_price: Some(gas_price * 5), // experiment with higher?
									data: Some(calldata.0.into()),
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

pub async fn get_request_handling() -> Result<(), anyhow::Error> {
	tracing::info!("\n\n\n\nStarting get request test\n\n\n\n");

	let signing_key = std::env::var("SIGNING_KEY").unwrap_or_else(|_| "".to_string());
	let bsc_url = std::env::var("BSC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string());
	let sepolia_url =
		std::env::var("SEPOLIA_URL").unwrap_or_else(|_| "http://localhost:8547".to_string());

	let source_chain = EvmConfig {
		rpc_url: bsc_url.clone(),
		state_machine: StateMachine::Evm(97),
		host_address: BSC_HOST,
		consensus_state_id: *b"BSC0",
	};

	let dest_chain = EvmConfig {
		rpc_url: sepolia_url,
		state_machine: StateMachine::Evm(11155111),
		host_address: SEPOLIA_HOST,
		consensus_state_id: *b"ETH0",
	};

	let hyperbrige_config = SubstrateConfig {
		state_machine: StateMachine::Kusama(4009),

		// rpc_url: "wss://hyperbridge-paseo-rpc.blockops.network:443".to_string(),
		rpc_url: "ws://127.0.0.1:9001".to_string(),
		consensus_state_id: *b"PARA",
		hash_algo: HashAlgorithm::Keccak,
	};
	let config = ClientConfig {
		source: ChainConfig::Evm(source_chain.clone()),
		dest: ChainConfig::Evm(dest_chain.clone()),
		hyperbridge: ChainConfig::Substrate(hyperbrige_config),
		tracing: false,
	};
	let hyperclient = HyperClient::new(config).await?;

	let latest_height = hyperclient
		.hyperbridge
		.query_latest_state_machine_height(StateMachineId {
			consensus_state_id: *b"ETH0",
			state_id: dest_chain.state_machine,
		})
		.await?;

	let pair = hex::decode(signing_key).unwrap();
	let provider = Arc::new(Provider::<Http>::try_connect(&bsc_url).await?);
	let chain_id = provider.get_chainid().await?.low_u64();
	let signer = LocalWallet::from(SecretKey::from_slice(pair.as_slice())?).with_chain_id(chain_id);
	let client = Arc::new(provider.with_signer(signer));

	let ping = PingModule::new(PING_MODULE.0, client.clone());
	let chain = StateMachine::Evm(97);
	let host_addr = ping.host().await.context(format!("Error in {chain:?}"))?;
	let host = EvmHost::new(host_addr, client.clone());
	let host_params = host.host_params().await?;

	let ping = PingModule::new(PING_MODULE.0, client.clone());
	let request = GetRequest {
		dest: dest_chain.state_machine.to_string().as_bytes().to_vec().into(),
		height: latest_height,
		// just query the account of the host in the world state
		keys: vec![SEPOLIA_HOST.as_bytes().to_vec().into()],
		timeout_timestamp: 0,
		..Default::default()
	};
	let call = ping.dispatch_with_request(request);

	let gas = call.estimate_gas().await.context(format!("Estimate gas error in {chain:?}"))?;
	let receipt = call
		.gas(gas)
		.send()
		.await
		.context(format!("Error in {chain:?}"))?
		.await
		.context(format!("Error in {chain:?}"))?
		.unwrap();

	dbg!(&receipt);

	let get_request: router::GetRequest = receipt
		.logs
		.into_iter()
		.find_map(|log| parse_log::<GetRequestEventFilter>(log).ok())
		.expect("Tx should emit post request")
		.try_into()?;

	dbg!(&get_request);

	let mut stream = internals::get_request_status_stream(
		&hyperclient,
		get_request.clone(),
		MessageStatusStreamState::Dispatched(receipt.block_number.unwrap().low_u64()),
	)
	.await?;

	while let Some(res) = stream.next().await {
		match res {
			Ok(status) => {
				tracing::info!("\nGot Status {:#?}\n", status);
				match status {
					MessageStatusWithMetadata::HyperbridgeFinalized { calldata, .. } => {
						let gas_price = client.get_gas_price().await?;
						tracing::info!("Sending Get response to BSC");
						let pending = client
							.send_transaction(
								TypedTransaction::Legacy(TransactionRequest {
									to: Some(NameOrAddress::Address(host_params.handler)),
									gas_price: Some(gas_price * 5), // experiment with higher?
									data: Some(calldata.0.into()),
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
