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
    arbitrum::ArbitrumPayloadProof,
    optimism::{OptimismDisputeGameProof, OptimismPayloadProof},
    prelude::*,
};
use alloc::collections::BTreeMap;
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use codec::{Decode, Encode};
use ethabi::ethereum_types::{H160, H256};
use hash256_std_hasher::Hash256StdHasher;
use hash_db::Hasher;
use ismp::host::{IsmpHost, StateMachine};
use sync_committee_primitives::types::{VerifierState, VerifierStateUpdate};

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
    pub light_client_state: VerifierState,
    pub ismp_contract_addresses: BTreeMap<StateMachine, H160>,
    pub l2_oracle_address: BTreeMap<StateMachine, H160>,
    pub dispute_factory_address: BTreeMap<StateMachine, H160>,
    pub rollup_core_address: H160,
}

#[derive(Encode, Decode)]
pub struct BeaconClientUpdate {
    pub consensus_update: VerifierStateUpdate,
    pub op_stack_payload: BTreeMap<StateMachine, OptimismPayloadProof>,
    pub dispute_game_payload: BTreeMap<StateMachine, OptimismDisputeGameProof>,
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
#[derive(RlpDecodable, RlpEncodable)]
pub struct Account {
    pub _nonce: u64,
    pub _balance: alloy_primitives::U256,
    pub storage_root: alloy_primitives::B256,
    pub _code_hash: alloy_primitives::B256,
}
