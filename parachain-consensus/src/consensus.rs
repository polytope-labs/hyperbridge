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

//! The parachain consensus client module

use core::{marker::PhantomData, time::Duration};

use alloc::{format, vec, vec::Vec};
use codec::{Decode, Encode};
use hex_literal::hex;
use ismp::{
    consensus_client::{
        ConsensusClient, ConsensusClientId, IntermediateState, StateCommitment, StateMachineHeight,
        StateMachineId,
    },
    error::Error,
    host::{ISMPHost, StateMachine},
    messaging::Proof,
    router::RequestResponse,
};
use ismp_primitives::mmr::{DataOrHash, Leaf, MmrHasher};
use merkle_mountain_range::MerkleProof;
use pallet_ismp::host::Host;
use primitive_types::H256;
use sp_consensus_aura::AURA_ENGINE_ID;
use sp_runtime::{
    traits::{BlakeTwo256, Header, Keccak256},
    DigestItem,
};
use sp_trie::{LayoutV0, StorageProof, Trie, TrieDBBuilder};

use crate::RelayChainOracle;

/// The parachain consensus client implementation for ISMP.
pub struct ParachainConsensusClient<T, R>(PhantomData<(T, R)>);

impl<T, R> Default for ParachainConsensusClient<T, R> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// Information necessary to prove the sibling parachain's finalization to this
/// parachain.
#[derive(Debug, Encode, Decode)]
pub struct ParachainConsensusProof {
    /// List of para ids contained in the proof
    pub para_ids: Vec<u32>,
    /// Height of the relay chain for the given proof
    pub relay_height: u32,
    /// Storage proof for the parachain headers
    pub storage_proof: Vec<Vec<u8>>,
}

/// Hashing algorithm for the state proof
#[derive(Debug, Encode, Decode)]
pub enum HashAlgorithm {
    Keccak,
    Blake2,
}

/// Holds the relevant data needed for state proof verification
#[derive(Debug, Encode, Decode)]
pub struct ParachainStateProof {
    /// Algorithm to use for state proof verification
    pub hasher: HashAlgorithm,
    /// Storage proof for the parachain headers
    pub storage_proof: Vec<Vec<u8>>,
}

/// Holds the relevant data needed for request/response proof verification
#[derive(Debug, Encode, Decode)]
pub struct MembershipProof {
    /// Size of the mmr at the time this proof was generated
    pub mmr_size: u64,
    /// Leaf indices for the proof
    pub leaf_indices: Vec<u64>,
    /// Mmr proof
    pub proof: Vec<H256>,
}

/// Static key for parachain headers in the relay chain storage
const PARACHAIN_HEADS_KEY: [u8; 32] =
    hex!("cd710b30bd2eab0352ddcc26417aa1941b3c252fcb29d88eff4f3de5de4476c3");

/// The `ConsensusEngineId` of ISMP digest in the parachain header.
pub const ISMP_ID: sp_runtime::ConsensusEngineId = *b"ISMP";

/// ConsensusClientId for [`ParachainConsensusClient`]
pub const PARACHAIN_CONSENSUS_ID: ConsensusClientId = *b"PARA";

/// Slot duration in milliseconds
const SLOT_DURATION: u64 = 12_000;

impl<T, R> ConsensusClient for ParachainConsensusClient<T, R>
where
    R: RelayChainOracle,
    T: pallet_ismp::Config,
    T::BlockNumber: Into<u32>,
    T::Hash: From<H256>,
{
    fn verify_consensus(
        &self,
        _host: &dyn ISMPHost,
        state: Vec<u8>,
        proof: Vec<u8>,
    ) -> Result<(Vec<u8>, Vec<IntermediateState>), Error> {
        let update: ParachainConsensusProof =
            codec::Decode::decode(&mut &proof[..]).map_err(|e| {
                Error::ImplementationSpecific(format!(
                    "Cannot decode parachain consensus proof: {e:?}"
                ))
            })?;

        let root = R::state_root(update.relay_height).ok_or_else(|| {
            Error::ImplementationSpecific(format!(
                "Cannot find relay chain height: {}",
                update.relay_height
            ))
        })?;

        let db = StorageProof::new(update.storage_proof).into_memory_db::<BlakeTwo256>();
        let trie = TrieDBBuilder::<LayoutV0<BlakeTwo256>>::new(&db, &root).build();

        let parachain_heads_key = PARACHAIN_HEADS_KEY.to_vec();

        let mut intermediates = vec![];

        for id in update.para_ids {
            let mut full_key = parachain_heads_key.clone();
            full_key.extend(sp_io::hashing::twox_64(&*id.encode()));
            let header = trie
                .get(&full_key)
                .map_err(|e| {
                    Error::ImplementationSpecific(
                        format!("Error verifying parachain header {e:?}",),
                    )
                })?
                .ok_or_else(|| {
                    Error::ImplementationSpecific(format!(
                        "Cannot find parachain header for ParaId({id})",
                    ))
                })?;

            // ideally all parachain headers are the same
            let header = T::Header::decode(&mut &*header).map_err(|e| {
                Error::ImplementationSpecific(format!("Error decoding parachain header: {e:?}",))
            })?;

            let (mut timestamp, mut ismp_root) = (0, H256::default());
            for digest in header.digest().logs.iter() {
                match digest {
                    DigestItem::PreRuntime(consensus_engine_id, value)
                        if *consensus_engine_id == AURA_ENGINE_ID =>
                    {
                        let slot = u64::decode(&mut &value[..]).map_err(|e| {
                            Error::ImplementationSpecific(format!(
                                "Cannot decode beacon message: {e:?}"
                            ))
                        })?;
                        timestamp = Duration::from_millis(slot * SLOT_DURATION).as_secs();
                    }
                    DigestItem::Consensus(consensus_engine_id, value)
                        if *consensus_engine_id == ISMP_ID =>
                    {
                        if value.len() != 32 {
                            Err(Error::ImplementationSpecific(
                                "Header contains an invalid ismp root".into(),
                            ))?
                        }

                        ismp_root = H256::from_slice(&value);
                    }
                    // don't really care about the rest
                    _ => {}
                };
            }

            if timestamp == 0 || ismp_root == H256::default() {
                Err(Error::ImplementationSpecific("Timestamp or ismp root not found".into()))?
            }

            let height: u32 = (*header.number()).into();

            let state_id = match _host.host_state_machine() {
                StateMachine::Kusama(_) => StateMachine::Kusama(id),
                StateMachine::Polkadot(_) => StateMachine::Polkadot(id),
                _ => Err(Error::ImplementationSpecific(
                    "Host state machine should be a parachain".into(),
                ))?,
            };

            let intermediate = IntermediateState {
                height: StateMachineHeight {
                    id: StateMachineId { state_id, consensus_client: PARACHAIN_CONSENSUS_ID },
                    height: height as u64,
                },
                commitment: StateCommitment {
                    timestamp,
                    ismp_root: Some(ismp_root),
                    state_root: H256::from_slice(header.state_root().as_ref()),
                },
            };

            intermediates.push(intermediate);
        }

        Ok((state, intermediates))
    }

    fn unbonding_period(&self) -> Duration {
        // there's no notion of client expiry, since there's shared security.
        Duration::from_secs(u64::MAX)
    }

    fn verify_membership(
        &self,
        _host: &dyn ISMPHost,
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
        let leaves = match item {
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
            .ismp_root
            .ok_or_else(|| Error::ImplementationSpecific("ISMP root should not be None".into()))?;

        let valid = proof
            .verify(DataOrHash::Hash(root.into()), leaves)
            .map_err(|e| Error::ImplementationSpecific(format!("Error verifying mmr: {e:?}")))?;

        if !valid {
            Err(Error::ImplementationSpecific("Invalid membership proof".into()))?
        }

        Ok(())
    }

    fn state_trie_key(&self, _request: RequestResponse) -> Vec<Vec<u8>> {
        todo!()
    }

    fn verify_state_proof(
        &self,
        _host: &dyn ISMPHost,
        keys: Vec<Vec<u8>>,
        root: StateCommitment,
        proof: &Proof,
    ) -> Result<Vec<Option<Vec<u8>>>, Error> {
        let state_proof: ParachainStateProof = codec::Decode::decode(&mut &*proof.proof)
            .map_err(|e| Error::ImplementationSpecific(format!("failed to decode proof: {e:?}")))?;

        let data = match state_proof.hasher {
            HashAlgorithm::Keccak => {
                let db = StorageProof::new(state_proof.storage_proof).into_memory_db::<Keccak256>();
                let trie = TrieDBBuilder::<LayoutV0<Keccak256>>::new(&db, &root.state_root).build();
                keys.into_iter()
                    .map(|key| {
                        trie.get(&key).map_err(|e| {
                            Error::ImplementationSpecific(format!(
                                "Error reading state proof: {e:?}"
                            ))
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
            HashAlgorithm::Blake2 => {
                let db =
                    StorageProof::new(state_proof.storage_proof).into_memory_db::<BlakeTwo256>();

                let trie =
                    TrieDBBuilder::<LayoutV0<BlakeTwo256>>::new(&db, &root.state_root).build();
                keys.into_iter()
                    .map(|key| {
                        trie.get(&key).map_err(|e| {
                            Error::ImplementationSpecific(format!(
                                "Error reading state proof: {e:?}"
                            ))
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
        };

        Ok(data)
    }

    fn is_frozen(&self, _: &[u8]) -> Result<(), Error> {
        // parachain consensus client can never be frozen.
        Ok(())
    }
}
