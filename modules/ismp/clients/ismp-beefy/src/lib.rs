#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use core::marker::PhantomData;

use beefy_verifier_primitives::ConsensusState;
use codec::{Decode, Encode};
use ismp::{
    consensus::{
        ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
    },
    error::Error,
    host::{IsmpHost, StateMachine},
    messaging::{Proof, StateCommitmentHeight},
    router::RequestResponse,
};
use sp_core::{H160, H256};
use substrate_state_machine::{read_proof_check, SubstrateStateMachine};


pub const ISMP_BEEFY_CONSENSUS_ID: ConsensusStateId = *b"IBCP";

pub struct IsmpBeefyClient<C: pallet_ismp::Config, H: IsmpHost>(PhantomData<(C, H)>);

impl<C,H> Default for IsmpBeefyClient<C,H> 
where C: pallet_ismp::Config,
      H: IsmpHost + Send + Sync + Default + 'static {
        fn default() -> Self {
            Self(PhantomData)
        }
}

impl<C,H> Clone for IsmpBeefyClient<C,H> 
where C: pallet_ismp::Config,
      H: IsmpHost + Send + Sync + Default + 'static {
        fn clone(&self) -> Self {
            Self(PhantomData)
        }
}

impl<C, H> ConsensusClient for IsmpBeefyClient<C, H>
where
    C: pallet_ismp::Config,
    H: IsmpHost + Send + Sync + Default + 'static,
{
    fn verify_consensus(
        &self,
        host: &dyn IsmpHost,
        consensus_state_id: ConsensusStateId,
        trusted_consensus_state: Vec<u8>,
        proof: Vec<u8>,
    ) -> Result<(Vec<u8>, ismp::consensus::VerifiedCommitments), Error> {
            // We must use beefy verifier to verify beefy proofs produced by beefy prover
            // Things to consider
            // 1. Per parachain
            todo!()                                                
    }

    fn verify_fraud_proof(
        &self,
        host: &dyn IsmpHost,
        trusted_consensus_state: Vec<u8>,
        proof_1: Vec<u8>,
        proof_2: Vec<u8>,
    ) -> Result<(), Error> {
        todo!()
    }

    fn consensus_client_id(&self) -> ConsensusClientId {
        ISMP_BEEFY_CONSENSUS_ID
    }

    fn state_machine(&self, id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
        match id {
            
            StateMachine::Beefy(_consensus_client_id) => {Ok(Box::new(SubstrateStateMachine::<C>::default()))},
            state_machine => return Err(Error::ImplementationSpecific(alloc::format!(
                "Unsupported state machine: {state_machine:?}"
            ))),
        }
    }
}