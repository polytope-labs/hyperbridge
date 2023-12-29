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
use polygon_pos_verifier::{primitives::CodecHeader, verify_polygon_header, VerificationResult};
use sp_core::{ConstU32, H160, H256, U256};
use sp_runtime::BoundedVec;

pub const POLYGON_CONSENSUS_ID: ConsensusStateId = *b"POLY";
#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub struct Chain {
    /// Validators for this chain fork
    pub validators: BTreeMap<u64, BTreeSet<H160>>,
    /// Block hashes contained in this fork;
    pub hashes: Vec<H256>,
    /// Cumulative difficulty of this fork
    pub difficulty: U256,
}

impl Chain {
    fn update_fork(&mut self, update: VerificationResult) {
        if let Some(validators) = update.next_validators {
            let span = get_sprint(update.header.number.low_u64() + 1);
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

#[derive(Encode, Decode, Debug)]
pub struct PolygonClientUpdate {
    /// Headers sorted in ascending order
    pub consensus_update: BoundedVec<CodecHeader, ConstU32<256>>,
    /// The leading hash in a fork, use a default value when building on the latest finalized hash
    pub chain_head: H256,
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
        let PolygonClientUpdate { consensus_update, chain_head } =
            PolygonClientUpdate::decode(&mut &proof[..]).map_err(|_| {
                Error::ImplementationSpecific("Cannot decode polygon client update".to_string())
            })?;

        let mut consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
            .map_err(|_| {
                Error::ImplementationSpecific("Cannot decode trusted consensus state".to_string())
            })?;

        if consensus_update.is_empty() {
            Err(Error::ImplementationSpecific("Consensus update is empty".to_string()))?
        }

        if consensus_update[0].parent_hash == consensus_state.finalized_hash {
            let mut chain = Chain {
                validators: Default::default(),
                hashes: vec![],
                difficulty: Default::default(),
            };

            let mut parent_hash = consensus_update[0].parent_hash;
            for header in consensus_update {
                if parent_hash != header.parent_hash {
                    Err(Error::ImplementationSpecific(
                        "Headers are meant to be in sequential order".to_string(),
                    ))?
                }
                log::info!(target: "pallet-ismp", "Parent hash: {:#?} Header {:#?}", parent_hash, (header.number));
                let sprint = get_sprint(header.number.low_u64());
                let validators =
                    chain.validators.get(&sprint).unwrap_or(&consensus_state.finalized_validators);
                let result = verify_polygon_header::<H>(validators, header)
                    .map_err(|e| Error::ImplementationSpecific(e.to_string()))?;
                parent_hash = result.hash;
                chain.update_fork(result.clone());

                Headers::<T>::insert(result.hash, result.header)
            }
            consensus_state.forks.push(chain);
        } else {
            // Find the chain with the given chain head
            let chain = if let Some(chain) = consensus_state
                .forks
                .iter_mut()
                .find(|chain| chain.hashes[chain.hashes.len() - 1] == chain_head)
            {
                chain
            } else {
                Err(Error::ImplementationSpecific("chain not found".to_string()))?
            };

            let mut parent_hash = chain_head;

            for header in consensus_update {
                if parent_hash != header.parent_hash {
                    Err(Error::ImplementationSpecific(
                        "Headers are meant to be in sequential order".to_string(),
                    ))?
                }
                let sprint = get_sprint(header.number.low_u64());
                let validators =
                    chain.validators.get(&sprint).unwrap_or(&consensus_state.finalized_validators);

                let result = verify_polygon_header::<H>(validators, header)
                    .map_err(|e| Error::ImplementationSpecific(e.to_string()))?;
                parent_hash = result.hash;
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
            // Finalize the 10th block in the chain
            // This allows us to have probabilistic finality of atleast 6 mins and avoid reorgs
            let finalized_hash = longest_chain.hashes[10];

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
            let finalized_span = get_sprint(header.number.low_u64());
            if let Some(validators) = longest_chain.validators.get(&finalized_span) {
                consensus_state.finalized_validators = validators.clone();
            }

            longest_chain.hashes = longest_chain.hashes[11..].to_vec();
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
        Ok(())
    }

    fn state_machine(
        &self,
        _id: ismp::host::StateMachine,
    ) -> Result<Box<dyn StateMachineClient>, ismp::error::Error> {
        Ok(Box::new(<EvmStateMachine<H>>::default()))
    }
}

fn get_sprint(number: u64) -> u64 {
    number / 16
}
