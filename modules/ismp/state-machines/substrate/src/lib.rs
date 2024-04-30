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

//! The state machine implementation in Substrate
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::all)]
#![deny(missing_docs)]

extern crate alloc;

use alloc::{collections::BTreeMap, format, string::ToString, vec, vec::Vec};
use codec::Decode;
use core::{fmt::Debug, marker::PhantomData, time::Duration};
use frame_support::ensure;
use ismp::{
    consensus::{StateCommitment, StateMachineClient},
    error::Error,
    host::IsmpHost,
    messaging::Proof,
    router::{Request, RequestResponse, Response},
    util::{hash_post_response, hash_request, hash_response},
};
use pallet_ismp::{
    child_trie::{RequestCommitments, RequestReceipts, ResponseCommitments, ResponseReceipts},
    host::Host,
    primitives::{HashAlgorithm, SubstrateStateProof, ISMP_ID},
};
use sp_consensus_aura::{Slot, AURA_ENGINE_ID};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, Keccak256},
    Digest, DigestItem,
};
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
{
    fn verify_membership(
        &self,
        _host: &dyn IsmpHost,
        item: RequestResponse,
        state: StateCommitment,
        proof: &Proof,
    ) -> Result<(), Error> {
        let state_proof: SubstrateStateProof = codec::Decode::decode(&mut &*proof.proof)
            .map_err(|e| Error::ImplementationSpecific(format!("failed to decode proof: {e:?}")))?;
        ensure!(
            matches!(state_proof, SubstrateStateProof::OverlayProof { .. }),
            Error::ImplementationSpecific("Expected Overlay Proof".to_string())
        );
        let root = state.overlay_root.ok_or_else(|| {
            Error::ImplementationSpecific(
                "Child trie root is not available for provided state commitment".into(),
            )
        })?;
        let keys = match item {
            RequestResponse::Request(requests) => requests
                .into_iter()
                .map(|request| {
                    let commitment = hash_request::<Host<T>>(&request);
                    RequestCommitments::<T>::storage_key(commitment)
                })
                .collect::<Vec<Vec<u8>>>(),
            RequestResponse::Response(responses) => responses
                .into_iter()
                .map(|response| {
                    let commitment = hash_response::<Host<T>>(&response);
                    ResponseCommitments::<T>::storage_key(commitment)
                })
                .collect::<Vec<Vec<u8>>>(),
        };
        let _ = match state_proof.hasher() {
            HashAlgorithm::Keccak => {
                let db =
                    StorageProof::new(state_proof.storage_proof()).into_memory_db::<Keccak256>();
                let trie = TrieDBBuilder::<LayoutV0<Keccak256>>::new(&db, &root).build();
                keys.into_iter()
                    .map(|key| {
                        let value = trie.get(&key).map_err(|e| {
                            Error::ImplementationSpecific(format!(
                                "Error reading state proof: {e:?}"
                            ))
                        })?.ok_or_else(|| Error::ImplementationSpecific(format!(
                            "Every key in a membership proof should have a value, found a key {:?} with None", key
                        )))?;
                        Ok((key, value))
                    })
                    .collect::<Result<BTreeMap<_, _>, _>>()?
            },
            HashAlgorithm::Blake2 => {
                let db =
                    StorageProof::new(state_proof.storage_proof()).into_memory_db::<BlakeTwo256>();

                let trie = TrieDBBuilder::<LayoutV0<BlakeTwo256>>::new(&db, &root).build();
                keys.into_iter()
                    .map(|key| {
                        let value = trie.get(&key).map_err(|e| {
                            Error::ImplementationSpecific(format!(
                                "Error reading state proof: {e:?}"
                            ))
                        })?.ok_or_else(|| Error::ImplementationSpecific(format!(
                            "Every key in a membership proof should have a value, found a key {:?} with None", key
                        )))?;
                        Ok((key, value))
                    })
                    .collect::<Result<BTreeMap<_, _>, _>>()?
            },
        };

        Ok(())
    }

    fn state_trie_key(&self, items: RequestResponse) -> Vec<Vec<u8>> {
        let mut keys = vec![];
        match items {
            RequestResponse::Request(requests) =>
                for req in requests {
                    match req {
                        Request::Post(post) => {
                            let request = Request::Post(post);
                            let commitment = hash_request::<Host<T>>(&request);
                            keys.push(RequestReceipts::<T>::storage_key(commitment));
                        },
                        Request::Get(_) => continue,
                    }
                },
            RequestResponse::Response(responses) =>
                for res in responses {
                    match res {
                        Response::Post(post_response) => {
                            let commitment = hash_post_response::<Host<T>>(&post_response);
                            keys.push(ResponseReceipts::<T>::storage_key(commitment));
                        },
                        Response::Get(_) => continue,
                    }
                },
        };

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
        let root = match &state_proof {
            SubstrateStateProof::OverlayProof { .. } => root.overlay_root.ok_or_else(|| {
                Error::ImplementationSpecific(
                    "Child trie root is not available for provided state commitment".into(),
                )
            })?,
            SubstrateStateProof::StateProof { .. } => root.state_root,
        };
        let data = match state_proof.hasher() {
            HashAlgorithm::Keccak => {
                let db =
                    StorageProof::new(state_proof.storage_proof()).into_memory_db::<Keccak256>();
                let trie = TrieDBBuilder::<LayoutV0<Keccak256>>::new(&db, &root).build();
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
            },
            HashAlgorithm::Blake2 => {
                let db =
                    StorageProof::new(state_proof.storage_proof()).into_memory_db::<BlakeTwo256>();

                let trie = TrieDBBuilder::<LayoutV0<BlakeTwo256>>::new(&db, &root).build();
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
            },
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

/// Fetches the overlay(ismp) root and timestamp from the header digest
pub fn fetch_overlay_root_and_timestamp(
    digest: &Digest,
    slot_duration: u64,
) -> Result<(u64, H256), Error> {
    let (mut timestamp, mut overlay_root) = (0, H256::default());

    for digest in digest.logs.iter() {
        match digest {
            DigestItem::PreRuntime(consensus_engine_id, value)
                if *consensus_engine_id == AURA_ENGINE_ID =>
            {
                let slot = Slot::decode(&mut &value[..])
                    .map_err(|e| Error::ImplementationSpecific(format!("Cannot slot: {e:?}")))?;
                timestamp = Duration::from_millis(*slot * slot_duration).as_secs();
            },
            DigestItem::Consensus(consensus_engine_id, value)
                if *consensus_engine_id == ISMP_ID =>
            {
                if value.len() != 32 {
                    Err(Error::ImplementationSpecific(
                        "Header contains an invalid ismp root".into(),
                    ))?
                }

                overlay_root = H256::from_slice(&value);
            },
            // don't really care about the rest
            _ => {},
        };
    }

    Ok((timestamp, overlay_root))
}
