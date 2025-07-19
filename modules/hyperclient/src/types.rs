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

use crate::{
	any_client::AnyClient,
	providers::{evm::EvmClient, substrate::SubstrateClient},
};
use anyhow::anyhow;
use core::{fmt, pin::Pin};
use futures::Stream;
use hex_fmt::HexFmt;
use ismp::{consensus::ConsensusStateId, host::StateMachine};
use primitive_types::H160;
use serde::{Deserialize, Serialize};
pub use substrate_state_machine::{HashAlgorithm, SubstrateStateProof};
use subxt::{utils::H256, Config};
use subxt_utils::{BlakeSubstrateChain, Hyperbridge};

// ========================================
// TYPES
// ========================================

pub type BoxStream<I> = Pin<Box<dyn Stream<Item = Result<I, anyhow::Error>>>>;

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct EvmConfig {
	pub rpc_url: String,
	pub state_machine: StateMachine,
	pub host_address: H160,
	pub consensus_state_id: ConsensusStateId,
}

impl EvmConfig {
	pub async fn into_client(&self) -> Result<EvmClient, anyhow::Error> {
		let client = EvmClient::new(
			self.rpc_url.clone(),
			self.consensus_state_id,
			self.host_address,
			self.state_machine,
		)
		.await?;

		Ok(client)
	}
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct SubstrateConfig {
	pub rpc_url: String,
	pub consensus_state_id: ConsensusStateId,
	pub hash_algo: HashAlgorithm,
	pub state_machine: StateMachine,
}

impl SubstrateConfig {
	async fn into_client<C: Config + Clone>(&self) -> Result<SubstrateClient<C>, anyhow::Error> {
		let client = SubstrateClient::<C>::new(
			self.rpc_url.clone(),
			self.hash_algo,
			self.consensus_state_id,
			self.state_machine,
		)
		.await?;
		Ok(client)
	}
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum ChainConfig {
	Evm(EvmConfig),
	Substrate(SubstrateConfig),
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct ClientConfig {
	pub source: ChainConfig,
	pub dest: ChainConfig,
	pub hyperbridge: ChainConfig,
	pub tracing: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq, Default, Copy)]
pub struct EventMetadata {
	/// The hash of the block where the event was emitted
	pub block_hash: H256,
	/// The hash of the extrinsic responsible for the event
	pub transaction_hash: H256,
	/// The block number where the event was emitted
	pub block_number: u64,
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Bytes(pub Vec<u8>);

impl AsRef<[u8]> for Bytes {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}

impl From<Vec<u8>> for Bytes {
	fn from(value: Vec<u8>) -> Bytes {
		Bytes(value)
	}
}

impl From<Bytes> for Vec<u8> {
	fn from(value: Bytes) -> Vec<u8> {
		value.0
	}
}

impl fmt::Debug for Bytes {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_tuple("Bytes").field(&HexFmt(self.0.clone())).finish()
	}
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum TimeoutStreamState {
	Pending,
	/// Destination state machine has been finalized on Hyperbridge
	DestinationFinalized(u64),
	/// The message time out has been verified by Hyperbridge, holds the block where the message
	/// was verified
	HyperbridgeVerified(u64),
	/// Stream has ended
	End,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum MessageStatusStreamState {
	/// Waiting for the message to be finalized on the source chain, holds the tx height for the
	/// source chain.
	Dispatched(u64),
	/// Source state machine has been finalized on hyperbridge, holds the block number at which the
	/// source was finalized on hyperbridge
	SourceFinalized(u64),
	/// The message has been verified by Hyperbridge, holds the block where the message was
	/// verified
	HyperbridgeVerified(u64),
	/// Message has been finalized by hyperbridge, holds the destination block number where
	/// hyperbridge was finalized.
	HyperbridgeFinalized(u64),
	/// Message has been delivered to destination
	DestinationDelivered,
	/// Stream has ended, check the message status
	End,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum MessageStatusWithMetadata {
	Pending,
	/// Source state machine has been finalized on hyperbridge.
	SourceFinalized {
		/// Block height of the source chain that was finalized.
		finalized_height: u64,
		/// Metadata about the event on hyperbridge
		#[serde(flatten)]
		meta: EventMetadata,
	},
	/// The message has been verified by Hyperbridge
	HyperbridgeVerified {
		/// Metadata about the event on hyperbridge
		#[serde(flatten)]
		meta: EventMetadata,
	},
	/// Messaged has been finalized on hyperbridge
	HyperbridgeFinalized {
		/// Block height of hyperbridge chain that was finalized.
		finalized_height: u64,
		/// Metadata about the event on the destination chain
		#[serde(flatten)]
		meta: EventMetadata,
		/// Calldata that encodes the proof for the message to be sent to the destination.
		#[serde(with = "serde_hex_utils::as_hex")]
		calldata: Bytes,
	},
	/// Delivered to destination
	DestinationDelivered {
		/// Metadata about the event on the destination chain
		#[serde(flatten)]
		meta: EventMetadata,
	},
	/// An error was encountered in the stream
	Error {
		/// Error description
		description: String,
	},
	/// Message has timed out
	Timeout,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TimeoutStatus {
	Pending,
	/// Destination state machine has been finalized the timeout on hyperbridge
	DestinationFinalized {
		/// Block height of the destination chain that was finalized.
		finalized_height: u64,
		/// Metadata about the event on hyperbridge
		#[serde(flatten)]
		meta: EventMetadata,
	},
	/// The message time out has been verified by Hyperbridge
	HyperbridgeVerified {
		/// Metadata about the event on hyperbridge
		#[serde(flatten)]
		meta: EventMetadata,
	},
	/// Hyperbridge has been finalized the timeout on source state machine
	HyperbridgeFinalized {
		/// Block height of hyperbridge chain that was finalized.
		finalized_height: u64,
		/// Metadata about the event on the destination chain
		#[serde(flatten)]
		meta: EventMetadata,
		/// Calldata that encodes the proof for the message to be sent to the destination.
		#[serde(with = "serde_hex_utils::as_hex")]
		calldata: Bytes,
	},
	/// An error was encountered in the stream
	Error {
		/// Error description
		description: String,
	},
}

impl ClientConfig {
	pub async fn dest_chain(&self) -> Result<AnyClient, anyhow::Error> {
		match &self.dest {
			ChainConfig::Evm(config) => {
				let client = config.into_client().await?;
				Ok(AnyClient::Evm(client))
			},
			ChainConfig::Substrate(config) => match config.hash_algo {
				HashAlgorithm::Keccak => {
					let client = config.into_client::<Hyperbridge>().await?;
					Ok(AnyClient::KeccakSubstrateChain(client))
				},
				HashAlgorithm::Blake2 => {
					let client = config.into_client::<BlakeSubstrateChain>().await?;
					Ok(AnyClient::BlakeSubstrateChain(client))
				},
			},
		}
	}

	pub async fn source_chain(&self) -> Result<AnyClient, anyhow::Error> {
		match &self.source {
			ChainConfig::Evm(config) => {
				let client = config.into_client().await?;
				Ok(AnyClient::Evm(client))
			},
			ChainConfig::Substrate(config) => match config.hash_algo {
				HashAlgorithm::Keccak => {
					let client = config.into_client::<Hyperbridge>().await?;
					Ok(AnyClient::KeccakSubstrateChain(client))
				},
				HashAlgorithm::Blake2 => {
					let client = config.into_client::<BlakeSubstrateChain>().await?;
					Ok(AnyClient::BlakeSubstrateChain(client))
				},
			},
		}
	}

	pub async fn hyperbridge_client(&self) -> Result<SubstrateClient<Hyperbridge>, anyhow::Error> {
		match self.hyperbridge {
			ChainConfig::Substrate(ref config) => config.into_client::<Hyperbridge>().await,
			_ => Err(anyhow!("Hyperbridge config should be a substrate variant")),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::types::{MessageStatusStreamState, MessageStatusWithMetadata, TimeoutStreamState};

	#[test]
	fn test_serialization() -> Result<(), anyhow::Error> {
		assert_eq!(
			r#"{"kind":"DestinationDelivered","block_hash":"0x0000000000000000000000000000000000000000000000000000000000000000","transaction_hash":"0x0000000000000000000000000000000000000000000000000000000000000000","block_number":0}"#,
			json::to_string(&MessageStatusWithMetadata::DestinationDelivered {
				meta: Default::default()
			})?
		);

		assert_eq!(
			r#"{"DestinationFinalized":24}"#,
			json::to_string(&TimeoutStreamState::DestinationFinalized(24))?
		);
		assert_eq!(r#""Pending""#, json::to_string(&TimeoutStreamState::Pending)?);
		assert_eq!(
			r#"{"HyperbridgeVerified":24}"#,
			json::to_string(&MessageStatusStreamState::HyperbridgeVerified(24))?
		);

		assert_eq!(
			r#"{"Dispatched":23}"#,
			json::to_string(&MessageStatusStreamState::Dispatched(23))?
		);

		Ok(())
	}
}
