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

//! Constants and methods used for evm verification

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_variables)]

extern crate alloc;
use alloc::collections::BTreeMap;
use ethabi::ethereum_types::{H160, H256};
use ismp::{
    consensus::StateCommitment, error::Error, host::IsmpHost, messaging::Proof,
    router::RequestResponse,
};

pub mod prelude {
    pub use alloc::{boxed::Box, string::ToString, vec, vec::Vec};
}

use prelude::*;

pub mod presets;
pub mod types;
pub mod utils;
pub use utils::*;

pub fn verify_membership<H: IsmpHost + Send + Sync>(
    item: RequestResponse,
    root: StateCommitment,
    proof: &Proof,
    contract_address: H160,
) -> Result<(), Error> {
    let mut evm_state_proof = decode_evm_state_proof(proof)?;
    let storage_proof = evm_state_proof
        .storage_proof
        .remove(&contract_address.0.to_vec())
        .ok_or_else(|| {
            Error::ImplementationSpecific("Ismp contract account trie proof is missing".to_string())
        })?;
    let keys = req_res_to_key::<H>(item);
    let root = H256::from_slice(&root.state_root[..]);
    let contract_root = get_contract_storage_root::<H>(
        evm_state_proof.contract_proof,
        &contract_address.0,
        root.clone(),
    )?;
    let values = get_values_from_proof::<H>(keys, contract_root, storage_proof)?;

    if values.into_iter().any(|val| val.is_none()) {
        Err(Error::ImplementationSpecific("Missing values for some keys in the proof".to_string()))?
    }

    Ok(())
}

pub fn verify_state_proof<H: IsmpHost + Send + Sync>(
    keys: Vec<Vec<u8>>,
    root: StateCommitment,
    proof: &Proof,
    ismp_address: H160,
) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
    let evm_state_proof = decode_evm_state_proof(proof)?;
    let mut map = BTreeMap::new();
    let mut contract_to_keys = BTreeMap::new();
    // Group keys by the contract address they belong to
    for key in keys {
        // For keys less than 52 bytes we default to the ismp contract address as the contract
        // key
        let contract_address =
            if key.len() == 52 { H160::from_slice(&key[..20]) } else { ismp_address };
        let entry = contract_to_keys.entry(contract_address.0.to_vec()).or_insert(vec![]);

        let slot_hash = if key.len() == 52 {
            H::keccak256(&key[20..]).0.to_vec()
        } else {
            H::keccak256(&key).0.to_vec()
        };

        entry.push((key, slot_hash));
    }

    for (contract_address, storage_proof) in evm_state_proof.storage_proof {
        let contract_root = get_contract_storage_root::<H>(
            evm_state_proof.contract_proof.clone(),
            &contract_address,
            root.state_root,
        )?;

        if let Some(keys) = contract_to_keys.remove(&contract_address) {
            let slot_hashes = keys.iter().map(|(_, slot_hash)| slot_hash.clone()).collect();
            let values = get_values_from_proof::<H>(slot_hashes, contract_root, storage_proof)?;
            keys.into_iter().zip(values).for_each(|((key, _), value)| {
                map.insert(key, value);
            });
        }
    }

    Ok(map)
}
