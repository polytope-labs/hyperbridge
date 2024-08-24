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

#![cfg(not(target_arch = "wasm32"))]
use std::str::FromStr;

use ismp::{
	host::StateMachine,
	router::{PostResponse, Request},
};
use substrate_state_machine::HashAlgorithm;

use crate::{
	indexing::{query_request_status_from_indexer, query_response_status_from_indexer},
	testing::{get_request_handling, subscribe_to_request_status, test_timeout_request},
	types::{ChainConfig, ClientConfig, EvmConfig, MessageStatusWithMetadata, SubstrateConfig},
	HyperClient,
};

pub fn setup_logging() {
	use tracing_subscriber::{filter::LevelFilter, util::SubscriberInitExt};
	let filter =
		tracing_subscriber::EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into());
	let _ = tracing_subscriber::fmt().with_env_filter(filter).finish().try_init();
}

#[tokio::test]
#[ignore]
async fn hyperclient_integration_tests() -> Result<(), anyhow::Error> {
	setup_logging();
	get_request_handling().await?;

	// test_timeout_request().await?;
	// subscribe_to_request_status().await?;

	Ok(())
}

// "source": "0x42415345",
// "nonce": "6055",
// "from": "0x9cc29770f3d643f4094ee591f3d2e3c98c349761",
// "to": "0x9cc29770f3d643f4094ee591f3d2e3c98c349761",
// "timeoutTimestamp": "1716240884",
// "dest": "0x4f505449",
// "data": "0x68656c6c6f2066726f6d2042415345",

#[tokio::test]
#[ignore]
async fn test_query_status_from_indexer() -> Result<(), anyhow::Error> {
	let post = ismp::router::PostRequest {
		source: StateMachine::from_str(
			&String::from_utf8(hex::decode("42415345".to_string()).unwrap()).unwrap(),
		)
		.unwrap(),
		dest: StateMachine::from_str(
			&String::from_utf8(hex::decode("4f505449".to_string()).unwrap()).unwrap(),
		)
		.unwrap(),
		nonce: 6055,
		from: hex::decode("9cc29770f3d643f4094ee591f3d2e3c98c349761".to_string()).unwrap(),
		to: hex::decode("9cc29770f3d643f4094ee591f3d2e3c98c349761".to_string()).unwrap(),
		timeout_timestamp: 1716240884,
		body: hex::decode("68656c6c6f2066726f6d2042415345".to_string()).unwrap(),
	};

	let request = Request::Post(post);

	let source_chain = EvmConfig {
		rpc_url: "https://bsc-testnet.blockpi.network/v1/rpc/public".to_string(),
		state_machine: StateMachine::Evm(97),
		host_address: Default::default(),
		consensus_state_id: *b"BSC0",
	};

	let dest_chain = EvmConfig {
		rpc_url: "https://optimism-sepolia.blockpi.network/v1/rpc/public".to_string(),
		state_machine: StateMachine::Evm(11155420),
		host_address: Default::default(),
		consensus_state_id: *b"ETH0",
	};

	let hyperbrige_config = SubstrateConfig {
		rpc_url: "wss://hyperbridge-paseo-rpc.blockops.network:443".to_string(),
		consensus_state_id: *b"PARA",
		hash_algo: HashAlgorithm::Keccak,
	};

	let config = ClientConfig {
		source: ChainConfig::Evm(source_chain.clone()),
		dest: ChainConfig::Evm(dest_chain.clone()),
		hyperbridge: ChainConfig::Substrate(hyperbrige_config),
		indexer: Some("http://localhost:3000".to_string()),
	};

	let hyperclient = HyperClient::new(config).await.unwrap();

	let status = query_request_status_from_indexer(request, &hyperclient).await?.unwrap();

	dbg!(&status);
	assert!(matches!(status, MessageStatusWithMetadata::DestinationDelivered { .. }));

	Ok(())
}

// "request": {
// 	"source": "0x425343",
// 	"nonce": "3516",
// 	"from": "0x9cc29770f3d643f4094ee591f3d2e3c98c349761",
// 	"to": "0x9cc29770f3d643f4094ee591f3d2e3c98c349761",
// 	"timeoutTimestamp": "1716240473",
// 	"dest": "0x4f505449",
// 	"data": "0x68656c6c6f2066726f6d20425343"
//   },
//   "id": "0x0039f125db9eb51dd1e25d6dafab8e68e4bc3367145ab943e8350a9e755d3574",
//   "status": "DEST",
//   "chain": "OPTI",
//   "responseTimeoutTimestamp": "3432417653",
//   "responseMessage": "0x48656c6c6f2066726f6d204f505449",
// }

#[tokio::test]
#[ignore]
async fn test_query_response_status_from_indexer() -> Result<(), anyhow::Error> {
	let post = ismp::router::PostRequest {
		source: StateMachine::from_str(
			&String::from_utf8(hex::decode("425343".to_string()).unwrap()).unwrap(),
		)
		.unwrap(),
		dest: StateMachine::from_str(
			&String::from_utf8(hex::decode("4f505449".to_string()).unwrap()).unwrap(),
		)
		.unwrap(),
		nonce: 3516,
		from: hex::decode("9cc29770f3d643f4094ee591f3d2e3c98c349761".to_string()).unwrap(),
		to: hex::decode("9cc29770f3d643f4094ee591f3d2e3c98c349761".to_string()).unwrap(),
		timeout_timestamp: 1716240473,
		body: hex::decode("68656c6c6f2066726f6d20425343".to_string()).unwrap(),
	};

	let response = PostResponse {
		post,
		response: hex::decode("48656c6c6f2066726f6d204f505449".to_string()).unwrap(),
		timeout_timestamp: 3432417653,
	};

	let source_chain = EvmConfig {
		rpc_url: "https://bsc-testnet.blockpi.network/v1/rpc/public".to_string(),
		state_machine: StateMachine::Evm(97),
		host_address: Default::default(),
		consensus_state_id: *b"BSC0",
	};

	let dest_chain = EvmConfig {
		rpc_url: "https://optimism-sepolia.blockpi.network/v1/rpc/public".to_string(),
		state_machine: StateMachine::Evm(11155420),
		host_address: Default::default(),
		consensus_state_id: *b"ETH0",
	};

	let hyperbrige_config = SubstrateConfig {
		rpc_url: "wss://hyperbridge-paseo-rpc.blockops.network:443".to_string(),
		consensus_state_id: *b"PARA",
		hash_algo: HashAlgorithm::Keccak,
	};

	let config = ClientConfig {
		source: ChainConfig::Evm(source_chain.clone()),
		dest: ChainConfig::Evm(dest_chain.clone()),
		hyperbridge: ChainConfig::Substrate(hyperbrige_config),
		indexer: Some("http://localhost:3000".to_string()),
	};

	let hyperclient = HyperClient::new(config).await.unwrap();

	let status =
		query_response_status_from_indexer(ismp::router::Response::Post(response), &hyperclient)
			.await?
			.unwrap();

	dbg!(&status);
	assert!(matches!(status, MessageStatusWithMetadata::DestinationDelivered { .. }));
	Ok(())
}
