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

use crate::types::{ChainConfig, ClientConfig, EvmConfig, HashAlgorithm, SubstrateConfig};
use anyhow::anyhow;
use core::str::FromStr;
use ismp::{
	host::StateMachine,
	router::{PostRequest, PostResponse},
};
use primitive_types::H160;
use serde::{Deserialize, Serialize};
use sp_core::bytes::from_hex;

#[derive(Clone, Eq, PartialEq, Debug, Default, Deserialize, Serialize)]
pub struct JsChainConfig {
	pub rpc_url: String,
	pub state_machine: String,
	pub host_address: String,
	pub consensus_state_id: String,
}

#[derive(Clone, Eq, PartialEq, Debug, Default, Deserialize, Serialize)]
pub struct JsHyperbridgeConfig {
	pub rpc_url: String,
}

#[derive(Clone, Eq, PartialEq, Debug, Default, Deserialize, Serialize)]
pub struct JsClientConfig {
	pub source: JsChainConfig,
	pub dest: JsChainConfig,
	pub hyperbridge: JsHyperbridgeConfig,
	pub indexer: String,
}

impl TryFrom<JsClientConfig> for ClientConfig {
	type Error = anyhow::Error;

	fn try_from(value: JsClientConfig) -> Result<Self, Self::Error> {
		let to_config = |val: &JsChainConfig| {
			if !val.host_address.is_empty() {
				let conf = EvmConfig {
					rpc_url: val.rpc_url.clone(),
					state_machine: StateMachine::from_str(&val.state_machine)
						.map_err(|e| anyhow!("{e:?}"))?,
					host_address: {
						let address = from_hex(&val.host_address)?;
						if address.len() != 20 {
							Err(anyhow!("Invalid host address"))?
						}
						H160::from_slice(&address)
					},
					consensus_state_id: {
						if val.consensus_state_id.len() != 4 {
							Err(anyhow!("Invalid consensus state id"))?
						}
						let mut dest = [0u8; 4];
						dest.copy_from_slice(&val.consensus_state_id.as_bytes());
						dest
					},
				};

				Ok::<_, anyhow::Error>(ChainConfig::Evm(conf))
			} else {
				let conf = SubstrateConfig {
					rpc_url: val.rpc_url.clone(),
					consensus_state_id: {
						if val.consensus_state_id.len() != 4 {
							Err(anyhow!("Invalid consensus state id"))?
						}
						let mut dest = [0u8; 4];
						dest.copy_from_slice(&val.consensus_state_id.as_bytes());
						dest
					},
					hash_algo: HashAlgorithm::Keccak,
				};

				Ok(ChainConfig::Substrate(conf))
			}
		};

		let indexer = if value.indexer.is_empty() {
			None
		} else {
			Some(url::Url::parse(&value.indexer)?.to_string())
		};

		let to_hyperbridge_config = |val: &JsHyperbridgeConfig| {
			let conf = SubstrateConfig {
				rpc_url: val.rpc_url.clone(),
				consensus_state_id: [0u8; 4],
				hash_algo: HashAlgorithm::Keccak,
			};

			Ok::<ChainConfig, Self::Error>(ChainConfig::Substrate(conf))
		};

		let source_config = to_config(&value.source)?;
		let dest_config = to_config(&value.dest)?;
		let hyperbridge = to_hyperbridge_config(&value.hyperbridge)?;

		Ok(ClientConfig { source: source_config, dest: dest_config, hyperbridge, indexer })
	}
}

#[derive(Clone, Eq, PartialEq, Default, Deserialize, Serialize)]
pub struct JsPost {
	/// The source state machine of this request.
	pub source: String,
	/// The destination state machine of this request.
	pub dest: String,
	/// The nonce of this request on the source chain
	pub nonce: u64,
	/// Module Id of the sending module
	pub from: String,
	/// Module ID of the receiving module
	pub to: String,
	/// Timestamp which this request expires in seconds.
	#[serde(rename = "timeoutTimestamp")]
	pub timeout_timestamp: u64,
	/// Encoded Request.
	pub body: String,
	/// Height at which this request was emitted on the source chain
	pub height: u64,
}

impl TryFrom<JsPost> for PostRequest {
	type Error = anyhow::Error;

	fn try_from(value: JsPost) -> Result<Self, Self::Error> {
		let source = if value.source.starts_with("0x") {
			let string = String::from_utf8(from_hex(&value.source)?)?;
			StateMachine::from_str(&string).map_err(|e| anyhow!("{e:?}"))?
		} else {
			StateMachine::from_str(&value.source).map_err(|e| anyhow!("{e:?}"))?
		};

		let dest = if value.dest.starts_with("0x") {
			let string = String::from_utf8(from_hex(&value.dest)?)?;
			StateMachine::from_str(&string).map_err(|e| anyhow!("{e:?}"))?
		} else {
			StateMachine::from_str(&value.dest).map_err(|e| anyhow!("{e:?}"))?
		};

		let post = PostRequest {
			source,
			dest,
			nonce: value.nonce,
			from: from_hex(&value.from)?,
			to: from_hex(&value.to)?,
			timeout_timestamp: value.timeout_timestamp,
			body: from_hex(&value.body)?,
		};
		Ok(post)
	}
}

#[derive(Clone, Eq, PartialEq, Default, Deserialize)]
pub struct JsPostResponse {
	/// The request that triggered this response.
	pub post: JsPost,
	/// The response message.
	pub response: Vec<u8>,
	/// Timestamp at which this response expires in seconds.
	#[serde(rename = "timeoutTimestamp")]
	pub timeout_timestamp: u64,
}

impl TryFrom<JsPostResponse> for PostResponse {
	type Error = anyhow::Error;

	fn try_from(value: JsPostResponse) -> Result<Self, Self::Error> {
		let response = PostResponse {
			post: value.post.try_into()?,
			response: value.response,
			timeout_timestamp: value.timeout_timestamp,
		};

		Ok(response)
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		interfaces::{JsChainConfig, JsClientConfig, JsHyperbridgeConfig, JsPost, JsPostResponse},
		types::{ChainConfig, ClientConfig, EvmConfig, HashAlgorithm, SubstrateConfig},
	};
	use ethers::prelude::H160;
	use hex_literal::hex;
	use ismp::{
		host::{Ethereum, StateMachine},
		router::{PostRequest, PostResponse},
	};
	const OP_HOST: H160 = H160(hex!("1B58A47e61Ca7604b634CBB00b4e275cCd7c9E95"));
	const BSC_HOST: H160 = H160(hex!("022DDE07A21d8c553978b006D93CDe68ac83e677"));

	#[test]
	fn test_chain_config_conversion() {
		let source_chain = EvmConfig {
			rpc_url: "https://127.0.0.1:9990".to_string(),
			state_machine: StateMachine::Bsc,
			host_address: BSC_HOST,
			consensus_state_id: *b"BSC0",
		};

		let dest_chain = EvmConfig {
			rpc_url: "https://127.0.0.1:9990".to_string(),
			state_machine: StateMachine::Ethereum(Ethereum::Optimism),
			host_address: OP_HOST,
			consensus_state_id: *b"ETH0",
		};

		let hyperbrige_config = SubstrateConfig {
			rpc_url: "ws://127.0.0.1:9990".to_string(),
			consensus_state_id: [0u8; 4],
			hash_algo: HashAlgorithm::Keccak,
		};
		let config = ClientConfig {
			source: ChainConfig::Evm(source_chain.clone()),
			dest: ChainConfig::Evm(dest_chain.clone()),
			hyperbridge: ChainConfig::Substrate(hyperbrige_config),
			indexer: Some("http://localhost:3000/".to_string()),
		};

		let js_source = JsChainConfig {
			rpc_url: "https://127.0.0.1:9990".to_string(),
			state_machine: "BSC".to_string(),
			host_address: hex::encode(&BSC_HOST.0),
			consensus_state_id: "BSC0".to_string(),
		};

		let js_dest = JsChainConfig {
			rpc_url: "https://127.0.0.1:9990".to_string(),
			state_machine: "OPTI".to_string(),
			host_address: hex::encode(&OP_HOST.0),
			consensus_state_id: "ETH0".to_string(),
		};

		let js_hyperbridge = JsHyperbridgeConfig { rpc_url: "ws://127.0.0.1:9990".to_string() };

		let js_client_conf = JsClientConfig {
			source: js_source,
			dest: js_dest,
			hyperbridge: js_hyperbridge,
			indexer: "http://localhost:3000/".to_string(),
		};

		assert_eq!(config, js_client_conf.try_into().unwrap());
	}

	#[test]
	fn test_post_conversion() {
		let post_response = PostResponse {
			post: PostRequest {
				source: StateMachine::Bsc,
				dest: StateMachine::Kusama(2000),
				nonce: 100,
				from: vec![30; 20],
				to: vec![15; 20],
				timeout_timestamp: 1_600_000,
				body: vec![40; 256],
			},
			response: vec![80; 256],
			timeout_timestamp: 4_500_000,
		};

		let js_post_response = JsPostResponse {
			post: JsPost {
				source: "BSC".to_string(),
				dest: "KUSAMA-2000".to_string(),
				nonce: 100,
				from: hex::encode(vec![30; 20]),
				to: hex::encode(vec![15; 20]),
				timeout_timestamp: 1_600_000,
				body: hex::encode(vec![40; 256]),
				height: 0,
			},
			response: vec![80; 256],
			timeout_timestamp: 4_500_000,
		};

		assert_eq!(post_response, js_post_response.try_into().unwrap())
	}
}
