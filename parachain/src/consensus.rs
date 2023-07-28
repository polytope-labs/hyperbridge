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

use alloc::{boxed::Box, collections::BTreeMap, format, vec::Vec};
use codec::{Decode, Encode};
use core::fmt::Debug;
use ismp::{
    consensus::{
        ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
        VerifiedCommitments,
    },
    error::Error,
    host::{IsmpHost, StateMachine},
    messaging::StateCommitmentHeight,
};
use ismp_primitives::ISMP_ID;
use parachain_system::{RelaychainDataProvider, RelaychainStateProvider};
use primitive_types::H256;
use sp_consensus_aura::{Slot, AURA_ENGINE_ID};
use sp_runtime::{
    app_crypto::sp_core::storage::StorageKey,
    generic::Header,
    traits::{BlakeTwo256, Header as _},
    DigestItem,
};
use sp_trie::StorageProof;
use substrate_state_machine::{read_proof_check, SubstrateStateMachine};

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

/// ConsensusClientId for [`ParachainConsensusClient`]
pub const PARACHAIN_CONSENSUS_ID: ConsensusClientId = *b"PARA";

/// Slot duration in milliseconds
const SLOT_DURATION: u64 = 12_000;

impl<T, R> ConsensusClient for ParachainConsensusClient<T, R>
where
    R: RelayChainOracle,
    T: pallet_ismp::Config + super::Config,
    T::BlockNumber: Into<u32>,
    T::Hash: From<H256>,
{
    fn verify_consensus(
        &self,
        host: &dyn IsmpHost,
        _consensus_state_id: ConsensusStateId,
        state: Vec<u8>,
        proof: Vec<u8>,
    ) -> Result<(Vec<u8>, VerifiedCommitments), Error> {
        let update: ParachainConsensusProof =
            codec::Decode::decode(&mut &proof[..]).map_err(|e| {
                Error::ImplementationSpecific(format!(
                    "Cannot decode parachain consensus proof: {e:?}"
                ))
            })?;

        // first check our oracle's registry
        let root = R::state_root(update.relay_height)
            // not in our registry? ask parachain_system.
            .or_else(|| {
                let state = RelaychainDataProvider::<T>::current_relay_chain_state();

                if state.number == update.relay_height {
                    Some(state.state_root)
                } else {
                    None
                }
            })
            // well, we couldn't find it
            .ok_or_else(|| {
                Error::ImplementationSpecific(format!(
                    "Cannot find relay chain height: {}",
                    update.relay_height
                ))
            })?;

        let storage_proof = StorageProof::new(update.storage_proof);
        let mut intermediates = BTreeMap::new();

        let keys = update.para_ids.iter().map(|id| parachain_header_storage_key(*id).0);
        let headers =
            read_proof_check::<BlakeTwo256, _>(&root, storage_proof, keys).map_err(|e| {
                Error::ImplementationSpecific(format!("Error verifying parachain header {e:?}",))
            })?;

        for (key, header) in headers {
            let mut state_commitments_vec = Vec::new();

            let id = codec::Decode::decode(&mut &key[(key.len() - 4)..]).map_err(|e| {
                Error::ImplementationSpecific(format!("Error decoding parachain header: {e}"))
            })?;
            let header = header.ok_or_else(|| {
                Error::ImplementationSpecific(format!(
                    "Cannot find parachain header for ParaId({id})",
                ))
            })?;
            // ideally all parachain headers are the same
            let header = Header::<u32, BlakeTwo256>::decode(&mut &*header).map_err(|e| {
                Error::ImplementationSpecific(format!("Error decoding parachain header: {e}"))
            })?;

            let (mut timestamp, mut overlay_root) = (0, H256::default());
            for digest in header.digest().logs.iter() {
                match digest {
                    DigestItem::PreRuntime(consensus_engine_id, value)
                        if *consensus_engine_id == AURA_ENGINE_ID =>
                    {
                        let slot = Slot::decode(&mut &value[..]).map_err(|e| {
                            Error::ImplementationSpecific(format!("Cannot slot: {e:?}"))
                        })?;
                        timestamp = Duration::from_millis(*slot * SLOT_DURATION).as_secs();
                    }
                    DigestItem::Consensus(consensus_engine_id, value)
                        if *consensus_engine_id == ISMP_ID =>
                    {
                        if value.len() != 32 {
                            Err(Error::ImplementationSpecific(
                                "Header contains an invalid ismp root".into(),
                            ))?
                        }

                        overlay_root = H256::from_slice(&value);
                    }
                    // don't really care about the rest
                    _ => {}
                };
            }

            if timestamp == 0 {
                Err(Error::ImplementationSpecific("Timestamp or ismp root not found".into()))?
            }

            let height: u32 = (*header.number()).into();

            let state_id = match host.host_state_machine() {
                StateMachine::Kusama(_) => StateMachine::Kusama(id),
                StateMachine::Polkadot(_) => StateMachine::Polkadot(id),
                _ => Err(Error::ImplementationSpecific(
                    "Host state machine should be a parachain".into(),
                ))?,
            };

            let intermediate = StateCommitmentHeight {
                commitment: StateCommitment {
                    timestamp,
                    overlay_root: Some(overlay_root),
                    state_root: header.state_root,
                },
                height: height.into(),
            };

            state_commitments_vec.push(intermediate);
            intermediates.insert(state_id, state_commitments_vec);
        }

        Ok((state, intermediates))
    }

    fn verify_fraud_proof(
        &self,
        _host: &dyn IsmpHost,
        _trusted_consensus_state: Vec<u8>,
        _proof_1: Vec<u8>,
        _proof_2: Vec<u8>,
    ) -> Result<(), Error> {
        // There are no fraud proofs for the parachain client
        Ok(())
    }

    fn state_machine(&self, _id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
        Ok(Box::new(SubstrateStateMachine::<T>::default()))
    }
}
/// This returns the storage key for a parachain header on the relay chain.
pub fn parachain_header_storage_key(para_id: u32) -> StorageKey {
    let mut storage_key = frame_support::storage::storage_prefix(b"Paras", b"Heads").to_vec();
    let encoded_para_id = para_id.encode();
    storage_key.extend_from_slice(sp_io::hashing::twox_64(&encoded_para_id).as_slice());
    storage_key.extend_from_slice(&encoded_para_id);
    StorageKey(storage_key)
}
