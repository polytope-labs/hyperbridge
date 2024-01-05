#![cfg_attr(not(feature = "std"), no_std)]
#[warn(unused_imports)]
#[warn(unused_variables)]
extern crate alloc;

pub mod pallet;
#[cfg(test)]
mod test;

use core::marker::PhantomData;

use alloc::{boxed::Box, collections::BTreeMap, string::ToString, vec, vec::Vec};
use codec::{Decode, Encode};
use geth_primitives::CodecHeader;
use ismp::{
    consensus::{ConsensusClient, ConsensusStateId, StateCommitment, StateMachineClient},
    error::Error,
    host::{IsmpHost, StateMachine},
    messaging::StateCommitmentHeight,
};
use ismp_sync_committee::EvmStateMachine;
use pallet::{Config, Headers};
use polygon_pos_verifier::{
    primitives::{SPAN_LENGTH, SPRINT_LENGTH},
    verify_polygon_header, VerificationResult,
};
use sp_core::{ConstU32, H160, H256, U256};
use sp_runtime::BoundedVec;

pub const POLYGON_CONSENSUS_ID: ConsensusStateId = *b"POLY";
#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub struct Chain {
    /// Validators for different spans in this chain fork
    pub validators: BTreeMap<u64, Vec<H160>>,
    /// Block hashes contained in this fork and signer;
    /// And the span of the block
    pub hashes: Vec<(H160, H256)>,
    /// Cumulative difficulty of this fork
    pub difficulty: U256,
}

impl Chain {
    fn update_fork(&mut self, update: VerificationResult) {
        let span = get_span(update.header.number.low_u64() + 1);
        if let Some(validators) = update.next_validators {
            if !self.validators.contains_key(&span) {
                self.validators.insert(span, validators);
            }
        }
        self.hashes.push((update.signer, update.hash));
        self.difficulty += update.header.difficulty;
    }
}

#[derive(Debug, Encode, Decode, Clone, Default)]
pub struct ConsensusState {
    pub frozen_height: Option<u64>,
    pub finalized_hash: H256,
    pub finalized_validators: Vec<H160>,
    pub forks: Vec<Chain>,
    pub ismp_contract_address: H160,
}

#[derive(Encode, Decode, Debug)]
pub struct PolygonClientUpdate {
    /// Headers sorted in ascending order
    pub consensus_update: BoundedVec<CodecHeader, ConstU32<256>>,
    /// Parent hash of the first header in the list
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

                let span = get_span(header.number.low_u64());
                let validators =
                    chain.validators.get(&span).unwrap_or(&consensus_state.finalized_validators);
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
                .find(|chain| chain.hashes[chain.hashes.len() - 1].1 == chain_head)
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
                let span = get_span(header.number.low_u64());
                let validators =
                    chain.validators.get(&span).unwrap_or(&consensus_state.finalized_validators);

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
            .filter(|chain| {
                log::info!(target: "pallet-ismp", "Chain : {:?} --> {:?}; Difficulty -> {:#?}; length: {:?}", chain.hashes[0].1, chain.hashes[chain.hashes.len() - 1].1, chain.difficulty, chain.hashes.len());
                chain.hashes.len() >= (consensus_state.finalized_validators.len() * SPRINT_LENGTH as usize)
            })
            .collect::<Vec<&Chain>>();

        let longest_chain = {
            if longest_chains.is_empty() {
                None
            } else {
                // Sort by highest cumulative difficulty
                longest_chains.sort_by(|a, b| a.difficulty.cmp(&b.difficulty));
                if longest_chains.len() > 1 &&
                    longest_chains[longest_chains.len() - 1].difficulty ==
                        longest_chains[longest_chains.len() - 2].difficulty
                {
                    None
                } else {
                    longest_chains.pop().cloned()
                }
            }
        };

        // we want to ensure that before we finalize a chain, most blocks have been signed by unique
        // validators
        let longest_chain = if let Some(chain) = longest_chain {
            // The composition of validators in consecutive chunks must be unique
            let mut validator_distribution = vec![];
            for (i, hashes) in chain.hashes.chunks(SPRINT_LENGTH as usize).enumerate() {
                let mut validator_dist = BTreeMap::<H160, u64>::new();
                hashes.iter().for_each(|(signer, _)| {
                    let entry = validator_dist.entry(*signer).or_insert(0);
                    *entry += 1;
                });

                let vals = validator_dist.into_iter().map(|a| a.0).collect::<Vec<_>>();
                validator_distribution.push(vals);
            }

            log::info!(target: "pallet-ismp", "Validator distribution : {:?}", validator_distribution);

            // Ensure that the composition of validators in each chunk is different
            let mut prev = &validator_distribution[0];
            if validator_distribution[1..].iter().all(|next| {
                let check = next != prev;
                prev = next;
                check
            }) {
                Some(chain)
            } else {
                None
            }
        } else {
            None
        };

        let mut state_machine_map: BTreeMap<StateMachine, Vec<StateCommitmentHeight>> =
            BTreeMap::new();
        if let Some(mut longest_chain) = longest_chain {
            // we want 16 mins of probabilistic finality
            let finality_index = longest_chain.hashes.len().saturating_sub(480);
            let finalized_hash = longest_chain.hashes[finality_index].1;

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

            longest_chain.hashes = longest_chain.hashes[(finality_index + 1)..].to_vec();
            longest_chain.validators.remove(&finalized_span);
            // Drop all other chain forks
            consensus_state.forks = vec![longest_chain];
        }

        Ok((consensus_state.encode(), state_machine_map))
    }

    fn verify_fraud_proof(
        &self,
        _host: &dyn IsmpHost,
        trusted_consensus_state: Vec<u8>,
        proof_1: Vec<u8>,
        proof_2: Vec<u8>,
    ) -> Result<(), ismp::error::Error> {
        let header_1 = CodecHeader::decode(&mut &*proof_1)
            .map_err(|_| Error::ImplementationSpecific("Failed to decode header".to_string()))?;
        let header_2 = CodecHeader::decode(&mut &*proof_2)
            .map_err(|_| Error::ImplementationSpecific("Failed to decode header".to_string()))?;

        if header_1.number != header_2.number {
            Err(Error::ImplementationSpecific("Invalid Fraud proof".to_string()))?
        }

        let consensus_state =
            ConsensusState::decode(&mut &trusted_consensus_state[..]).map_err(|_| {
                Error::ImplementationSpecific("Cannot decode trusted consensus state".to_string())
            })?;
        let res_1 =
            verify_polygon_header::<H>(&consensus_state.finalized_validators, header_1.clone())
                .map_err(|_| {
                    Error::ImplementationSpecific("Failed to verify first header".to_string())
                })?;

        let res_2 =
            verify_polygon_header::<H>(&consensus_state.finalized_validators, header_2.clone())
                .map_err(|_| {
                    Error::ImplementationSpecific("Failed to verify second header".to_string())
                })?;

        // Fraud proof Scenario 1: Same block number with different hashes signed by the same
        // validator
        if res_1.hash != res_2.hash && res_1.signer == res_2.signer {
            return Ok(())
        }

        // The difficulty of an in turn block is equal to the total number of validators
        // https://github.com/maticnetwork/bor/blob/930c9463886d7695b1335b7daf275eb88514a8a7/consensus/bor/snapshot.go#L225
        // Fraud Proof Scenario 2:  Two valid blocks with the same in turn or out turn difficulty by
        // different or the same signers
        if header_1.difficulty == header_2.difficulty && res_1.hash != res_2.hash {
            return Ok(())
        }

        Err(Error::ImplementationSpecific("Invalid Fraud Proof".to_string()))
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
