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

use alloy::{
	network::EthereumWallet,
	primitives::{Address, Bytes, U256},
	providers::{Provider, ProviderBuilder},
	rpc::types::{Filter, TransactionRequest},
	signers::local::PrivateKeySigner,
	sol_types::SolEvent,
};
use anyhow::Context;

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
	erc20::{ERC20Instance, ERC20},
	evm_host::{EvmHostInstance, EvmHost, GetRequestEvent, PostRequestEvent},
	ping_module::{PingMessage, PingModuleInstance, PingModule},
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
	let signer: PrivateKeySigner = signing_key.parse()?;
	let wallet = EthereumWallet::from(signer);
	let provider = Arc::new(
		ProviderBuilder::new()
			.wallet(wallet)
			.on_http(bsc_url.parse()?),
	);

	let ping_addr = Address::from_slice(&PING_MODULE.0);
	let ping = PingModuleInstance::new(ping_addr, provider.clone());
	let chain = StateMachine::Evm(97);
	let host_addr_result = ping.host().call().await.context(format!("Error in {chain:?}"))?;
	let host_addr = host_addr_result._0;
	let host = EvmHostInstance::new(host_addr, provider.clone());
	let fee_token = host.feeToken().call().await.context(format!("Error in {chain:?}"))?._0;
	let erc_20 = ERC20Instance::new(fee_token, provider.clone());

	let approve_call = erc_20.approve(ping_addr, U256::MAX);
	let pending = approve_call.send().await.context(format!("Error in {chain:?}"))?;
	pending.watch().await.context(format!("Error in {chain:?}"))?;

	let ping_message = PingMessage {
		dest: Bytes::from(dest_chain.state_machine.to_string().as_bytes().to_vec()),
		module: Bytes::from(PING_MODULE.0.to_vec()),
		timeout: 10 * 60 * 60,
		fee: U256::from(90_000_000_000_000_000_000u128),
		count: U256::from(1),
	};
	let call = ping.ping(ping_message);
	let pending = call.send().await.context(format!("Error in {chain:?}"))?;
	let receipt = pending.get_receipt().await.context(format!("Error in {chain:?}"))?;

	let post: router::PostRequest = receipt
		.inner
		.logs()
		.iter()
		.find_map(|log| {
			PostRequestEvent::decode_log(log.inner.clone(), true).ok()
		})
		.expect("Tx should emit post request")
		.try_into()?;
	tracing::info!("Got PostRequest {post}");
	let block = receipt.block_number.unwrap();
	tracing::info!("\n\nTx block: {block}\n\n");

	let mut stream = post_request_status_stream(
		&hyperclient,
		post,
		MessageStatusStreamState::Dispatched(block),
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
	let signer: PrivateKeySigner = signing_key.parse()?;
	let wallet = EthereumWallet::from(signer);
	let provider = Arc::new(
		ProviderBuilder::new()
			.wallet(wallet)
			.on_http(bsc_url.parse()?),
	);

	let ping_addr = Address::from_slice(&PING_MODULE.0);
	let ping = PingModuleInstance::new(ping_addr, provider.clone());
	let chain = StateMachine::Evm(97);
	let host_addr_result = ping.host().call().await.context(format!("Error in {chain:?}"))?;
	let host_addr = host_addr_result._0;
	let host = EvmHostInstance::new(host_addr, provider.clone());
	let host_params = host.hostParams().call().await?;
	tracing::info!("{:#?}", host_params._0);

	let fee_token = host.feeToken().call().await.context(format!("Error in {chain:?}"))?._0;
	let erc_20 = ERC20Instance::new(fee_token, provider.clone());
	let approve_call = erc_20.approve(ping_addr, U256::MAX);

	let pending = approve_call.send().await.context(format!("Error in {chain:?}"))?;
	pending.watch().await.context(format!("Error in {chain:?}"))?;

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

	let ping_message = PingMessage {
		dest: Bytes::from(dest_chain.state_machine.to_string().as_bytes().to_vec()),
		module: Bytes::from(PING_MODULE.0.to_vec()),
		timeout: 3 * 60,
		fee: U256::from(0u128),
		count: U256::from(1),
	};
	let call = ping.ping(ping_message);
	let pending = call.send().await.context(format!("Error in {chain:?}"))?;
	let receipt = pending.get_receipt().await.context(format!("Error in {chain:?}"))?;

	let block = receipt.block_number.unwrap();
	tracing::info!("\n\nTx block: {block}\n\n");

	let post: router::PostRequest = receipt
		.inner
		.logs()
		.iter()
		.find_map(|log| {
			PostRequestEvent::decode_log(log.inner.clone(), true).ok()
		})
		.expect("Tx should emit post request")
		.try_into()?;
	tracing::info!("PostRequest {post}");

	let request_status = post_request_status_stream(
		&hyperclient,
		post.clone(),
		MessageStatusStreamState::Dispatched(block),
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
						tracing::info!("Sending timeout to BSC");
						let tx = TransactionRequest::default()
							.to(host_params._0.handler)
							.input(calldata.0.into());
						let pending = provider.send_transaction(tx).await;
						tracing::info!("Send transaction result: {pending:#?}");
						let result = pending?.get_receipt().await;
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

	let signer: PrivateKeySigner = signing_key.parse()?;
	let wallet = EthereumWallet::from(signer);
	let provider = Arc::new(
		ProviderBuilder::new()
			.wallet(wallet)
			.on_http(bsc_url.parse()?),
	);

	let ping_addr = Address::from_slice(&PING_MODULE.0);
	let ping = PingModuleInstance::new(ping_addr, provider.clone());
	let chain = StateMachine::Evm(97);
	let host_addr_result = ping.host().call().await.context(format!("Error in {chain:?}"))?;
	let host_addr = host_addr_result._0;
	let host = EvmHostInstance::new(host_addr, provider.clone());
	let host_params = host.hostParams().call().await?;

	let request = GetRequest {
		dest: Bytes::from(dest_chain.state_machine.to_string().as_bytes().to_vec()),
		height: latest_height,
		// just query the account of the host in the world state
		keys: vec![Bytes::from(SEPOLIA_HOST.as_bytes().to_vec())],
		timeoutTimestamp: 0,
		..Default::default()
	};
	let call = ping.dispatchWithRequest(request);

	let pending = call.send().await.context(format!("Error in {chain:?}"))?;
	let receipt = pending.get_receipt().await.context(format!("Error in {chain:?}"))?;

	dbg!(&receipt);

	let get_request: router::GetRequest = receipt
		.inner
		.logs()
		.iter()
		.find_map(|log| {
			GetRequestEvent::decode_log(log.inner.clone(), true).ok()
		})
		.expect("Tx should emit get request")
		.try_into()?;

	dbg!(&get_request);

	let mut stream = internals::get_request_status_stream(
		&hyperclient,
		get_request.clone(),
		MessageStatusStreamState::Dispatched(receipt.block_number.unwrap()),
	)
	.await?;

	while let Some(res) = stream.next().await {
		match res {
			Ok(status) => {
				tracing::info!("\nGot Status {:#?}\n", status);
				match status {
					MessageStatusWithMetadata::HyperbridgeFinalized { calldata, .. } => {
						tracing::info!("Sending Get response to BSC");
						let tx = TransactionRequest::default()
							.to(host_params._0.handler)
							.input(calldata.0.into());
						let pending = provider.send_transaction(tx).await;
						tracing::info!("Send transaction result: {pending:#?}");
						let result = pending?.get_receipt().await;
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
