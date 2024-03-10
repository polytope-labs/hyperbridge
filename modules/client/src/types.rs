use crate::providers::{evm::EvmClient, interface::Client, substrate::SubstrateClient};
use alloc::collections::BTreeMap;
use anyhow::anyhow;
use codec::Encode;
use core::pin::Pin;
use ethers::types::H160;
use futures::Stream;
use ismp::{consensus::ConsensusStateId, host::StateMachine};
use serde::{Deserialize, Serialize};
use subxt::{
    config::{polkadot::PolkadotExtrinsicParams, substrate::SubstrateHeader, Hasher},
    ext::{codec, codec::Decode},
    tx::TxPayload,
    utils::{AccountId32, MultiAddress, MultiSignature, H256},
    Config, Metadata,
};

// ========================================
// TYPES
// ========================================

/// Implements [`subxt::Config`] for substrate chains with keccak as their hashing algorithm
#[derive(Clone)]
pub struct HyperBridgeConfig;

/// A type that can hash values using the keccak_256 algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode)]
pub struct KeccakHasher;

impl Hasher for KeccakHasher {
    type Output = H256;
    fn hash(s: &[u8]) -> Self::Output {
        use tiny_keccak::Hasher;

        let mut keccak = tiny_keccak::Keccak::v256();
        let mut output = H256::default();
        keccak.update(s);
        keccak.finalize(&mut output[..]);
        output
    }
}

impl Config for HyperBridgeConfig {
    type Hash = H256;
    type AccountId = AccountId32;
    type Address = MultiAddress<Self::AccountId, u32>;
    type Signature = MultiSignature;
    type Hasher = KeccakHasher;
    type Header = SubstrateHeader<u32, KeccakHasher>;
    type ExtrinsicParams = PolkadotExtrinsicParams<Self>;
}

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
    async fn into_client(&self) -> Result<EvmClient, anyhow::Error> {
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

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Copy)]
#[serde(tag = "kind")]
pub enum MessageStatus {
    Pending,
    /// Source state machine has been finalized on hyperbridge.
    SourceFinalized {
        /// Hyperbridge height when this was finalized
        height: u64,
    },
    /// Message has been delivered to hyperbridge
    HyperbridgeDelivered {
        /// Hyperbridge height when the request was delivered
        height: u64,
    },
    /// Messaged has been finalized on hyperbridge
    HyperbridgeFinalized {
        /// Hyperbridge height that finalized the request
        height: u64,
    },
    /// Delivered to destination
    DestinationDelivered {
        /// Destination height at which request was delivered
        height: u64,
    },
    /// Message has timed out
    Timeout,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PostStreamState {
    /// Message has been finalized on source chain
    Pending,
    /// Source state machine has been updated on hyperbridge
    SourceFinalized,
    /// Message has been delivered to hyperbridge
    HyperbridgeDelivered(u64),
    /// Message has been finalized by hyperbridge
    HyperbridgeFinalized,
    /// Message has been delivered to destination
    DestinationDelivered,
    /// Stream has ended, check the message status
    End,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum TimeoutStatus {
    Pending,
    /// Destination state machine has been finalized on hyperbridge
    DestinationFinalized {
        /// Hyperbridge height when the destination finalized the timeout.
        height: u64,
    },
    /// Message has been timed out on hyperbridge
    HyperbridgeTimedout {
        /// Hyperbridge height when the request was timed-out
        height: u64,
    },
    /// Hyperbridge has been finalized on source state machine
    HyperbridgeFinalized {
        /// Hyperbridge height when that finalizes the timeout
        height: u64,
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

#[derive(Encode, Decode, Clone)]
pub struct EvmStateProof {
    /// Contract account proof
    pub contract_proof: Vec<Vec<u8>>,
    /// A map of storage key to the associated storage proof
    pub storage_proof: BTreeMap<Vec<u8>, Vec<Vec<u8>>>,
}

/// Hashing algorithm for the state proof
#[derive(
    Debug, Encode, Decode, Clone, Copy, serde::Deserialize, serde::Serialize, Eq, PartialEq,
)]
pub enum HashAlgorithm {
    /// For chains that use keccak as their hashing algo
    Keccak,
    /// For chains that use blake2 as their hashing algo
    Blake2,
}

/// Holds the relevant data needed for state proof verification
#[derive(Debug, Encode, Decode, Clone)]
pub struct SubstrateStateProof {
    /// Algorithm to use for state proof verification
    pub hasher: HashAlgorithm,
    /// Storage proof for the parachain headers
    pub storage_proof: Vec<Vec<u8>>,
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

    pub async fn hyperbridge_client(
        &self,
    ) -> Result<SubstrateClient<HyperBridgeConfig>, anyhow::Error> {
        match self.hyperbridge {
            ChainConfig::Substrate(ref config) => config.into_client::<HyperBridgeConfig>().await,
            _ => Err(anyhow!("Hyperbridge config should be a substrate variant")),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::MessageStatus;

    #[test]
    fn test_serialization() -> Result<(), anyhow::Error> {
        assert_eq!(
            r#"{"kind":"DestinationDelivered","height":23}"#,
            json::to_string(&MessageStatus::DestinationDelivered { height: 23 })?
        );
        assert_eq!(r#"{"kind":"Timeout"}"#, json::to_string(&MessageStatus::Timeout)?);

        Ok(())
    }
}
