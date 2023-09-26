use crate::{arbitrum::ArbitrumPayloadProof, optimism::OptimismPayloadProof, prelude::*};
use alloc::collections::BTreeMap;
use alloy_rlp_derive::RlpDecodable;
use codec::{Decode, Encode};
use ethabi::ethereum_types::{H160, H256};
use hash256_std_hasher::Hash256StdHasher;
use hash_db::Hasher;
use ismp::host::{IsmpHost, StateMachine};
use sync_committee_primitives::types::{LightClientState, LightClientUpdate};

pub struct KeccakHasher<H: IsmpHost>(core::marker::PhantomData<H>);

impl<H: IsmpHost + Send + Sync> Hasher for KeccakHasher<H> {
    type Out = H256;
    type StdHasher = Hash256StdHasher;
    const LENGTH: usize = 32;

    fn hash(x: &[u8]) -> Self::Out {
        H::keccak256(x)
    }
}

#[derive(Debug, Encode, Decode, Clone)]
pub struct ConsensusState {
    pub frozen_height: Option<u64>,
    pub light_client_state: LightClientState,
    pub ismp_contract_addresses: BTreeMap<StateMachine, H160>,
    pub l2_oracle_address: BTreeMap<StateMachine, H160>,
    pub rollup_core_address: H160,
}

#[derive(Encode, Decode)]
pub struct BeaconClientUpdate {
    pub consensus_update: LightClientUpdate,
    pub op_stack_payload: BTreeMap<StateMachine, OptimismPayloadProof>,
    pub arbitrum_payload: Option<ArbitrumPayloadProof>,
}

#[derive(Encode, Decode, Clone)]
pub struct EvmStateProof {
    /// Contract account proof
    pub contract_proof: Vec<Vec<u8>>,
    /// A map of storage key to the associated storage proof
    pub storage_proof: BTreeMap<Vec<u8>, Vec<Vec<u8>>>,
}

/// The ethereum account stored in the global state trie.
#[derive(RlpDecodable)]
pub struct Account {
    _nonce: u64,
    _balance: alloy_primitives::U256,
    pub storage_root: alloy_primitives::B256,
    _code_hash: alloy_primitives::B256,
}
