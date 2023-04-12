use crate::consensus_clients::beacon_consensus_client::optimism::OptimismPayloadProof;
use codec::{Decode, Encode};
use ethabi::ethereum_types::{H256, U256};
use hash256_std_hasher::Hash256StdHasher;
use hash_db::Hasher;
use rlp_derive::RlpDecodable;
use sp_std::prelude::*;
use sync_committee_primitives::derived_types::{LightClientState, LightClientUpdate};

pub struct KeccakHasher;

impl Hasher for KeccakHasher {
    type Out = H256;
    type StdHasher = Hash256StdHasher;
    const LENGTH: usize = 32;

    fn hash(x: &[u8]) -> Self::Out {
        sp_io::hashing::keccak_256(x).into()
    }
}

#[derive(Debug, Encode, Decode, Clone)]
pub struct ConsensusState {
    pub frozen_height: Option<u64>,
    pub light_client_state: LightClientState,
}

#[derive(Encode, Decode)]
pub struct Misbehaviour {
    pub update_1: LightClientUpdate,
    pub update_2: LightClientUpdate,
}

#[derive(Encode, Decode)]
pub struct BeaconClientUpdate {
    pub consensus_update: LightClientUpdate,
    pub optimism_payload: Option<OptimismPayloadProof>,
}

#[derive(Encode, Decode)]
pub enum BeaconMessage {
    ConsensusUpdate(BeaconClientUpdate),
    Misbehaviour(Misbehaviour),
}

/// Slot index for requests map
pub const REQ_SLOT: u8 = 1;
/// Slot index for responses map
pub const RESP_SLOT: u8 = 2;

#[derive(Encode, Decode, Clone)]
pub struct EvmStateProof {
    pub contract_proof: Vec<Vec<u8>>,
    pub storage_proof: Vec<Vec<u8>>,
}

/// The ethereum account stored in the global state trie.
#[derive(RlpDecodable)]
pub(super) struct Account {
    _nonce: u64,
    _balance: U256,
    pub storage_root: H256,
    _code_hash: H256,
}
