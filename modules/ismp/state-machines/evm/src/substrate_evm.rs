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

//! Substrate EVM State Machine client implementation

use crate::{prelude::*, req_res_commitment_key, req_res_receipt_keys};
use alloc::{collections::BTreeMap, vec};
use codec::{Decode, Encode};
use ismp:: {
    consensus::{StateCommitment, StateMachineClient},
    error::Error,
    host::IsmpHost,
    messaging::{Proof, StateMachineHeight},
    router::RequestResponse,
};
use pallet_ismp_host_executive::EvmHosts;
use primitive_types::H160;
use polkadot_sdk::*;
use sp_core::{hashing, storage::ChildInfo, H256};
use sp_trie::{LayoutV0, StorageProof, Trie, TrieDBBuilder};
use crate::types::SubstrateEvmProof;

/// Substrate Evm State Machine Client
pub struct SubstrateEvmStateMachine<H: IsmpHost, T: pallet_ismp_host_executive::Config>(
    core::marker::PhantomData<(H, T)>,
);

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config> Default for SubstrateEvmStateMachine<H, T> {
    fn default() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config> StateMachineClient for SubstrateEvmStateMachine<H, T> {
    fn verify_membership(&self, host: &dyn IsmpHost, item: RequestResponse, root: StateCommitment, proof: &Proof) -> Result<(), Error> {
        let contract_address = EvmHosts::<T>::get(&proof.height.id.state_id).ok_or_else(|| Error::Custom("Ismp contract address not found").to_string())?;
        let proof: SubstrateEvmProof = Decode::decode(&mut &proof.proof[..]).map_err(|e| Error::Custom(format!("Failed to decode proof: {:?}", e)))?;

        let state_root = H256::from_slice(&root.state_root[..]);

        // verify contact info in main trie first to get Trie Id
        let contract_info_key = contract_info_key(contract_address);
        let trie_id = fetch_trie_id_from_main_proof::<H>(&proof.main_proof, state_root, &contract_info_key)?;

        let child_root = fetch_child_root_from_main_proof::<H>(&proof.main_proof, state_root, &trie_id)?;

        // verify storage slots in child trie
        let keys = req_res_commitment_key::<H>(item);
        let storage_keys: Vec<Vec<u8>> = keys.into_iter().map(|k| hashing::blake2_256(&k).to_vec()).collect();

        verify_child_trie_membership::<H>(child_root, &proof.child_proof, storage_keys)
    }

    fn receipts_state_trie_key(&self, request: RequestResponse) -> Vec<Vec<u8>> {
        req_res_receipt_keys::<H>(items)
    }

    fn verify_state_proof(&self, host: &dyn IsmpHost, keys: Vec<Vec<u8>>, root: StateCommitment, proof: &Proof) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
        let contract_address = EvmHosts::<T>::get(&proof.height.id.state_id).ok_or_else(|| Error::Custom("Ismp contract address not found").to_string())?;
        let proof: SubstrateEvmProof = Decode::decode(&mut &proof.proof[..]).map_err(|e| Error::Custom(format!("Failed to decode proof: {:?}", e)))?;

        let state_root = H256::from_slice(&root.state_root[..]);

        let contract_info_key = contract_info_key(contract_address);
        let trie_id = fetch_trie_id_from_main_proof::<H>(&proof.main_proof, state_root, &contract_info_key)?;

        let child_root = fetch_child_root_from_main_proof::<H>(&proof.main_proof, state_root, &trie_id)?;


    }
}