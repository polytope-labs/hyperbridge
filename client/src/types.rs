use crate::providers::{
    evm_chain::EvmClient,
    global::Client,
    substrate::{HashAlgorithm, SubstrateClient},
};
use alloc::collections::BTreeMap;
use anyhow::anyhow;
use codec::Encode;
use core::{pin::Pin, str::FromStr};
use ethers::types::H160;
use futures::Stream;
use ismp::{
    consensus::{ConsensusStateId, StateMachineId},
    host::StateMachine,
};
use serde::{Deserialize, Serialize};
use subxt::{
    config::{polkadot::PolkadotExtrinsicParams, substrate::SubstrateHeader, Hasher},
    ext::{codec, codec::Decode},
    tx::TxPayload,
    utils::{AccountId32, MultiAddress, MultiSignature, H256},
    Config, Metadata, OnlineClient,
};
use wasm_bindgen::prelude::wasm_bindgen;

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
        sp_core::keccak_256(s).into()
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

#[derive(Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub source_state_machine: String,
    pub dest_state_machine: String,
    pub hyperbridge_state_machine: String,
    pub source_rpc_url: String,
    pub dest_rpc_url: String,
    pub hyper_bridge_url: String,
    pub destination_ismp_host_address: H160,
    pub source_ismp_host_address: H160,
    pub consensus_state_id_source: ConsensusStateId,
    pub consensus_state_id_dest: ConsensusStateId,
    pub destination_ismp_handler: H160,
    pub source_ismp_handler: H160,
}

#[wasm_bindgen]
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Copy)]
pub enum MessageStatus {
    Pending,
    /// Source state machine has been finalized on hyperbridge
    SourceFinalized,
    /// Message has been delivered to hyperbridge
    HyperbridgeDelivered,
    /// Messaged has been finalized on hyperbridge
    HyperbridgeFinalized,
    /// Delivered to destination
    DestinationDelivered,
    /// Message has timed out
    Timeout,
    /// Message has not timed out
    NotTimedOut,
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

#[derive(Serialize, Deserialize)]
pub struct LeafIndexQuery {
    /// Commitment of the request or response
    pub commitment: H256,
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
        let dest_state_machine = StateMachine::from_str(&self.dest_state_machine).unwrap();

        return match dest_state_machine {
            StateMachine::Bsc | StateMachine::Ethereum(_) | StateMachine::Polygon => {
                let evm_chain = EvmClient::new(
                    self.dest_rpc_url.clone(),
                    self.consensus_state_id_dest,
                    self.destination_ismp_host_address,
                    self.destination_ismp_handler.clone(),
                    self.dest_state_machine.clone(),
                )
                .await?;
                Ok(evm_chain)
            },
            _ => Err(anyhow!("Unknown chain")),
        };
    }

    pub async fn source_chain(&self) -> Result<impl Client, anyhow::Error> {
        let source_state_machine: StateMachine =
            StateMachine::from_str(&self.source_state_machine).unwrap();

        return match source_state_machine {
            StateMachine::Bsc | StateMachine::Ethereum(_) | StateMachine::Polygon => {
                let evm_chain = EvmClient::new(
                    self.source_rpc_url.clone(),
                    self.consensus_state_id_source,
                    self.source_ismp_host_address,
                    self.source_ismp_handler,
                    self.source_state_machine.clone(),
                )
                .await?;
                Ok(evm_chain)
            },
            _ => Err(anyhow!("Unknown chain")),
        };
    }

    pub async fn hyperbridge_client(
        &self,
    ) -> Result<SubstrateClient<HyperBridgeConfig>, anyhow::Error> {
        let api =
            OnlineClient::<HyperBridgeConfig>::from_url(self.hyper_bridge_url.clone()).await?;
        let hyperbridge_state_machine: StateMachine =
            StateMachine::from_str(&self.hyperbridge_state_machine).unwrap();
        Ok(SubstrateClient {
            client: api,
            rpc_url: self.hyper_bridge_url.clone(),
            state_machine: StateMachineId {
                state_id: hyperbridge_state_machine,
                consensus_state_id: *b"PARA",
            },
            hashing: HashAlgorithm::Keccak,
        })
    }
}
