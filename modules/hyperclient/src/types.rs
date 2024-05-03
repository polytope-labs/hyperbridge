use crate::providers::{evm::EvmClient, interface::Client, substrate::SubstrateClient};
use anyhow::anyhow;
use codec::Encode;
use core::pin::Pin;
use ethers::types::H160;
pub use evm_common::types::EvmStateProof;
use futures::Stream;
use ismp::{consensus::ConsensusStateId, host::StateMachine};
use serde::{Deserialize, Serialize};
pub use substrate_state_machine::{HashAlgorithm, SubstrateStateProof};
use subxt::{tx::TxPayload, utils::H256, Config, Metadata};
use subxt_utils::Hyperbridge;

// ========================================
// TYPES
// ========================================

pub type BoxStream<I> = Pin<Box<dyn Stream<Item = Result<I, anyhow::Error>>>>;

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct EvmConfig {
    pub rpc_url: String,
    pub state_machine: StateMachine,
    pub host_address: H160,
    pub handler_address: H160,
    pub consensus_state_id: ConsensusStateId,
}

impl EvmConfig {
    pub async fn into_client(&self) -> Result<EvmClient, anyhow::Error> {
        let client = EvmClient::new(
            self.rpc_url.clone(),
            self.consensus_state_id,
            self.host_address,
            self.handler_address,
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
}

impl SubstrateConfig {
    async fn into_client<C: Config + Clone>(&self) -> Result<SubstrateClient<C>, anyhow::Error> {
        let client = SubstrateClient::<C>::new(
            self.rpc_url.clone(),
            self.hash_algo,
            self.consensus_state_id,
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

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum MessageStatus {
    Pending,
    /// Source state machine has been finalized on hyperbridge.
    SourceFinalized,
    /// Message has been delivered to hyperbridge
    HyperbridgeDelivered,
    /// Messaged has been finalized on hyperbridge
    HyperbridgeFinalized,
    /// Delivered to destination
    DestinationDelivered,
    /// Message has timed out
    Timeout,
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
    /// Message has been delivered to hyperbridge
    HyperbridgeDelivered {
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PostStreamState {
    /// Message has been finalized on source chain
    Pending,
    /// Source state machine has been updated on hyperbridge, holds the block number at which the
    /// source was finalized on hyperbridge
    SourceFinalized(u64),
    /// Message has been finalized by hyperbridge
    HyperbridgeFinalized(u64),
    /// Message has been delivered to hyperbridge, holds the block where the message was delivered
    HyperbridgeDelivered(u64),
    /// Message has been delivered to destination
    DestinationDelivered,
    /// Stream has ended, check the message status
    End,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TimeoutStatus {
    Pending,
    /// Destination state machine has been finalized the timeout on hyperbridge
    DestinationFinalized {
        /// Metadata about the event on hyperbridge
        #[serde(flatten)]
        meta: EventMetadata,
    },
    /// Message has been timed out on hyperbridge
    HyperbridgeTimedout {
        /// Metadata about the event on hyperbridge
        #[serde(flatten)]
        meta: EventMetadata,
    },
    /// Hyperbridge has been finalized the timeout on source state machine
    HyperbridgeFinalized {
        /// Metadata about the event on the destination
        #[serde(flatten)]
        meta: EventMetadata,
    },
    /// An error was encountered in the stream
    Error {
        /// Error description
        description: String,
    },
    /// Encoded call data to be submitted to source chain
    TimeoutMessage {
        /// Calldata that encodes the proof for the timeout message on the source.
        calldata: Vec<u8>,
    },
}

/// Implements [`TxPayload`] for extrinsic encoding
pub struct Extrinsic {
    /// The pallet name, used to query the metadata
    pallet_name: String,
    /// The call name
    call_name: String,
    /// The encoded pallet call. Note that this should be the pallet call. Not runtime call
    encoded: Vec<u8>,
}

// =======================================
// IMPLs                            =
// =======================================
impl Extrinsic {
    /// Creates a new extrinsic ready to be sent with subxt.
    pub fn new(
        pallet_name: impl Into<String>,
        call_name: impl Into<String>,
        encoded_call: Vec<u8>,
    ) -> Self {
        Extrinsic {
            pallet_name: pallet_name.into(),
            call_name: call_name.into(),
            encoded: encoded_call,
        }
    }
}

impl TxPayload for Extrinsic {
    fn encode_call_data_to(
        &self,
        metadata: &Metadata,
        out: &mut Vec<u8>,
    ) -> Result<(), subxt::Error> {
        // encode the pallet index
        let pallet = metadata.pallet_by_name_err(&self.pallet_name).unwrap();
        let call_index = pallet.call_variant_by_name(&self.call_name).unwrap().index;
        let pallet_index = pallet.index();
        pallet_index.encode_to(out);
        call_index.encode_to(out);

        // copy the encoded call to out
        out.extend_from_slice(&self.encoded);

        Ok(())
    }
}

impl ClientConfig {
    pub async fn dest_chain(&self) -> Result<impl Client, anyhow::Error> {
        match &self.dest {
            ChainConfig::Evm(config) => config.into_client().await,
            _ => Err(anyhow!("Support for substrate coming: requires an AnyClient implementation")),
        }
    }

    pub async fn source_chain(&self) -> Result<impl Client, anyhow::Error> {
        match &self.source {
            ChainConfig::Evm(config) => config.into_client().await,
            _ => Err(anyhow!("Support for substrate coming: requires an AnyClient implementation")),
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
    use crate::types::{MessageStatus, MessageStatusWithMetadata};

    #[test]
    fn test_serialization() -> Result<(), anyhow::Error> {
        assert_eq!(
            r#"{"kind":"DestinationDelivered","block_hash":"0x0000000000000000000000000000000000000000000000000000000000000000","transaction_hash":"0x0000000000000000000000000000000000000000000000000000000000000000","block_number":0}"#,
            json::to_string(&MessageStatusWithMetadata::DestinationDelivered {
                meta: Default::default()
            })?
        );
        assert_eq!(r#"{"kind":"Timeout"}"#, json::to_string(&MessageStatus::Timeout)?);

        Ok(())
    }
}
