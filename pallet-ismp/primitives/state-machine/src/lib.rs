// Copyright (C) 2023 Polytope Labs.
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

//! The state machine implementation in Substrate
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::all)]
#![deny(missing_docs)]

extern crate alloc;

use alloc::{collections::BTreeMap, format, vec, vec::Vec};
use codec::Decode;
use core::{fmt::Debug, marker::PhantomData};
use ismp::{
    consensus::{StateCommitment, StateMachineClient},
    error::Error,
    host::IsmpHost,
    messaging::Proof,
    router::{Request, RequestResponse},
    util::hash_request,
};
use ismp_primitives::{
    mmr::{DataOrHash, Leaf, MmrHasher},
    HashAlgorithm, MembershipProof, SubstrateStateProof,
};
use merkle_mountain_range::MerkleProof;
use pallet_ismp::host::Host;
use primitive_types::H256;
use sp_runtime::traits::{BlakeTwo256, Keccak256};
use sp_trie::{HashDBT, LayoutV0, StorageProof, Trie, TrieDBBuilder, EMPTY_PREFIX};

/// The parachain and grandpa consensus client implementation for ISMP.
pub struct SubstrateStateMachine<T>(PhantomData<T>);

impl<T> Default for SubstrateStateMachine<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T> StateMachineClient for SubstrateStateMachine<T>
where
    T: pallet_ismp::Config,
    T::BlockNumber: Into<u32>,
    T::Hash: From<H256>,
{
    fn verify_membership(
        &self,
        _host: &dyn IsmpHost,
        item: RequestResponse,
        state: StateCommitment,
        proof: &Proof,
    ) -> Result<(), Error> {
        let membership = MembershipProof::decode(&mut &*proof.proof).map_err(|e| {
            Error::ImplementationSpecific(format!("Cannot decode membership proof: {e:?}"))
        })?;
        let nodes = membership.proof.into_iter().map(|h| DataOrHash::Hash(h.into())).collect();
        let proof =
            MerkleProof::<DataOrHash<T>, MmrHasher<T, Host<T>>>::new(membership.mmr_size, nodes);
        let leaves: Vec<(u64, DataOrHash<T>)> = match item {
            RequestResponse::Request(req) => membership
                .leaf_indices
                .into_iter()
                .zip(req.into_iter())
                .map(|(pos, req)| (pos, DataOrHash::Data(Leaf::Request(req))))
                .collect(),
            RequestResponse::Response(res) => membership
                .leaf_indices
                .into_iter()
                .zip(res.into_iter())
                .map(|(pos, res)| (pos, DataOrHash::Data(Leaf::Response(res))))
                .collect(),
        };
        let root = state
            .overlay_root
            .ok_or_else(|| Error::ImplementationSpecific("ISMP root should not be None".into()))?;

        let calc_root = proof
            .calculate_root(leaves.clone())
            .map_err(|e| Error::ImplementationSpecific(format!("Error verifying mmr: {e:?}")))?;
        let valid = calc_root.hash::<Host<T>>() == root.into();

        if !valid {
            Err(Error::ImplementationSpecific("Invalid membership proof".into()))?
        }

        Ok(())
    }

    fn state_trie_key(&self, requests: Vec<Request>) -> Vec<Vec<u8>> {
        let mut keys = vec![];

        for req in requests {
            match req {
                Request::Post(post) => {
                    let request = Request::Post(post);
                    let commitment = hash_request::<Host<T>>(&request).0.to_vec();
                    keys.push(pallet_ismp::RequestReceipts::<T>::hashed_key_for(commitment));
                }
                Request::Get(_) => continue,
            }
        }

        keys
    }

    fn verify_state_proof(
        &self,
        _host: &dyn IsmpHost,
        keys: Vec<Vec<u8>>,
        root: StateCommitment,
        proof: &Proof,
    ) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
        let state_proof: SubstrateStateProof = codec::Decode::decode(&mut &*proof.proof)
            .map_err(|e| Error::ImplementationSpecific(format!("failed to decode proof: {e:?}")))?;

        let data = match state_proof.hasher {
            HashAlgorithm::Keccak => {
                let db = StorageProof::new(state_proof.storage_proof).into_memory_db::<Keccak256>();
                let trie = TrieDBBuilder::<LayoutV0<Keccak256>>::new(&db, &root.state_root).build();
                keys.into_iter()
                    .map(|key| {
                        let value = trie.get(&key).map_err(|e| {
                            Error::ImplementationSpecific(format!(
                                "Error reading state proof: {e:?}"
                            ))
                        })?;
                        Ok((key, value))
                    })
                    .collect::<Result<BTreeMap<_, _>, _>>()?
            }
            HashAlgorithm::Blake2 => {
                let db =
                    StorageProof::new(state_proof.storage_proof).into_memory_db::<BlakeTwo256>();

                let trie =
                    TrieDBBuilder::<LayoutV0<BlakeTwo256>>::new(&db, &root.state_root).build();
                keys.into_iter()
                    .map(|key| {
                        let value = trie.get(&key).map_err(|e| {
                            Error::ImplementationSpecific(format!(
                                "Error reading state proof: {e:?}"
                            ))
                        })?;
                        Ok((key, value))
                    })
                    .collect::<Result<BTreeMap<_, _>, _>>()?
            }
        };

        Ok(data)
    }
}

/// Lifted directly from [`sp_state_machine::read_proof_check`](https://github.com/paritytech/substrate/blob/b27c470eaff379f512d1dec052aff5d551ed3b03/primitives/state-machine/src/lib.rs#L1075-L1094)
pub fn read_proof_check<H, I>(
    root: &H::Out,
    proof: StorageProof,
    keys: I,
) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error>
where
    H: hash_db::Hasher,
    H::Out: Debug,
    I: IntoIterator,
    I::Item: AsRef<[u8]>,
{
    let db = proof.into_memory_db();

    if !db.contains(root, EMPTY_PREFIX) {
        Err(Error::ImplementationSpecific("Invalid Proof".into()))?
    }

    let trie = TrieDBBuilder::<LayoutV0<H>>::new(&db, root).build();
    let mut result = BTreeMap::new();

    for key in keys.into_iter() {
        let value = trie
            .get(key.as_ref())
            .map_err(|e| Error::ImplementationSpecific(format!("Error reading from trie: {e:?}")))?
            .and_then(|val| Decode::decode(&mut &val[..]).ok());
        result.insert(key.as_ref().to_vec(), value);
    }

    Ok(result)
}
