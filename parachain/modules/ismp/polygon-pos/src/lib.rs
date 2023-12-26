#![cfg_attr(not(feature = "std"), no_std)]
#[warn(unused_imports)]
#[warn(unused_variables)]
extern crate alloc;

pub mod pallet;
use core::marker::PhantomData;

use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    string::ToString,
    vec,
    vec::Vec,
};
use codec::{Decode, Encode};
use ismp::{
    consensus::{ConsensusClient, ConsensusStateId, StateCommitment, StateMachineClient},
    error::Error,
    host::{IsmpHost, StateMachine},
    messaging::StateCommitmentHeight,
};
use ismp_sync_committee::EvmStateMachine;
use pallet::{Config, Headers};
use polygon_pos_verifier::{
    primitives::{CodecHeader, SPAN_LENGTH},
    verify_polygon_header, VerificationResult,
};
use sp_core::{ConstU32, H160, H256, U256};
use sp_runtime::BoundedVec;

pub const POLYGON_CONSENSUS_ID: ConsensusStateId = *b"POLY";
#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub struct Chain {
    /// Block hash of this chain head
    pub hash: H256,
    /// Validators for this chain fork
    pub validators: BTreeMap<u64, BTreeSet<H160>>,
    /// Block hashes contained in this fork;
    pub hashes: Vec<H256>,
    /// Cumulative difficulty of this fork
    pub difficulty: U256,
}

impl Chain {
    fn update_fork(&mut self, update: VerificationResult) {
        self.hash = update.hash;
        if let Some(validators) = update.next_validators {
            let span = get_span(update.header.number.low_u64() + 1);
            self.validators.insert(span, validators);
        }
        self.hashes.push(update.hash);
        self.difficulty += update.header.difficulty;
    }
}

#[derive(Debug, Encode, Decode, Clone, Default)]
pub struct ConsensusState {
    pub frozen_height: Option<u64>,
    pub finalized_hash: H256,
    pub finalized_validators: BTreeSet<H160>,
    pub forks: Vec<Chain>,
    pub ismp_contract_address: H160,
}

#[derive(Encode, Decode)]
pub struct PolygonClientUpdate {
    pub consensus_update: BoundedVec<CodecHeader, ConstU32<64>>,
}

pub struct PolygonClient<T: Config, H: IsmpHost>(PhantomData<(T, H)>);

impl<T: Config, H: IsmpHost> Default for PolygonClient<T, H> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Config, H: IsmpHost> Clone for PolygonClient<T, H> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<T: Config, H: IsmpHost + Send + Sync + Default + 'static> ConsensusClient
    for PolygonClient<T, H>
{
    fn verify_consensus(
        &self,
        _host: &dyn IsmpHost,
        _consensus_state_id: ismp::consensus::ConsensusStateId,
        trusted_consensus_state: Vec<u8>,
        proof: Vec<u8>,
    ) -> Result<(Vec<u8>, ismp::consensus::VerifiedCommitments), ismp::error::Error> {
        let PolygonClientUpdate { mut consensus_update } =
            PolygonClientUpdate::decode(&mut &proof[..]).map_err(|_| {
                Error::ImplementationSpecific("Cannot decode polygon client update".to_string())
            })?;

        let mut consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
            .map_err(|_| {
                Error::ImplementationSpecific("Cannot decode trusted consensus state".to_string())
            })?;

        consensus_update.sort_by(|a, b| a.number.cmp(&b.number));

        for header in consensus_update {
            let parent_hash = header.parent_hash;
            // Header must be a descendant of the latest finalized header or one of the known chain
            // forks
            if parent_hash == consensus_state.finalized_hash {
                let result =
                    verify_polygon_header::<H>(&consensus_state.finalized_validators, header)
                        .map_err(|e| Error::ImplementationSpecific(e.to_string()))?;

                let chain = Chain {
                    hash: result.hash,
                    validators: {
                        let span = get_span(result.header.number.low_u64() + 1);
                        let mut vals = BTreeMap::new();
                        if let Some(next_validators) = result.next_validators {
                            vals.insert(span, next_validators);
                        }
                        vals
                    },
                    hashes: vec![result.hash],
                    difficulty: result.header.difficulty,
                };

                consensus_state.forks.push(chain);
                Headers::<T>::insert(result.hash, result.header)
            } else {
                let chain = if let Some(chain) =
                    consensus_state.forks.iter_mut().find(|chain| chain.hash == header.parent_hash)
                {
                    chain
                } else {
                    // If header does not belong to any known chain, we skip it
                    continue
                };

                let span = get_span(header.number.low_u64());
                let validators =
                    chain.validators.get(&span).unwrap_or(&consensus_state.finalized_validators);

                let result = verify_polygon_header::<H>(validators, header)
                    .map_err(|e| Error::ImplementationSpecific(e.to_string()))?;
                chain.update_fork(result.clone());
                Headers::<T>::insert(result.hash, result.header)
            }
        }

        // Try to finalize the longest chain
        let mut longest_chains = consensus_state
            .forks
            .iter()
            .filter(|chain| chain.hashes.len() >= 200)
            .collect::<Vec<&Chain>>();
        // Sort by highest cumulative difficulty
        longest_chains.sort_by(|a, b| a.difficulty.cmp(&b.difficulty));
        let longest_chain = longest_chains.pop().cloned();
        let mut state_machine_map: BTreeMap<StateMachine, Vec<StateCommitmentHeight>> =
            BTreeMap::new();
        if let Some(mut longest_chain) = longest_chain {
            let finalized_hash = longest_chain.hashes[50];

            let header = Headers::<T>::get(finalized_hash).ok_or_else(|| {
                Error::ImplementationSpecific("Expected header to be found in storage".to_string())
            })?;
            let state_commitment = StateCommitmentHeight {
                commitment: StateCommitment {
                    timestamp: header.timestamp,
                    overlay_root: None,
                    state_root: header.state_root,
                },
                height: header.number.low_u64(),
            };

            state_machine_map.insert(StateMachine::Polygon, vec![state_commitment]);
            consensus_state.finalized_hash = finalized_hash;
            let finalized_span = get_span(header.number.low_u64());
            if let Some(validators) = longest_chain.validators.get(&finalized_span) {
                consensus_state.finalized_validators = validators.clone();
            }

            longest_chain.hashes = longest_chain.hashes[51..].to_vec();
            longest_chain.validators.remove(&finalized_span);
            // Drop all other chain forks
            consensus_state.forks = vec![longest_chain];
        }

        Ok((consensus_state.encode(), state_machine_map))
    }

    fn verify_fraud_proof(
        &self,
        _host: &dyn IsmpHost,
        _trusted_consensus_state: Vec<u8>,
        _proof_1: Vec<u8>,
        _proof_2: Vec<u8>,
    ) -> Result<(), ismp::error::Error> {
        unimplemented!()
    }

    fn state_machine(
        &self,
        _id: ismp::host::StateMachine,
    ) -> Result<Box<dyn StateMachineClient>, ismp::error::Error> {
        Ok(Box::new(<EvmStateMachine<H>>::default()))
    }
}

fn get_span(number: u64) -> u64 {
    number / SPAN_LENGTH
}
