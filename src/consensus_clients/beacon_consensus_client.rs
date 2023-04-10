use crate::primitives::ETHEREUM_CONSENSUS_CLIENT_ID;
use alloc::{format, string::ToString};
use codec::{Decode, Encode};
use core::time::Duration;
use ethabi::{
    ethereum_types::{H160, H256, U256},
    Token,
};
use hash256_std_hasher::Hash256StdHasher;
use hash_db::Hasher;
use hex_literal::hex;
use ismp_rs::{
    consensus_client::{
        ConsensusClient, IntermediateState, StateCommitment, StateMachineHeight, StateMachineId,
    },
    error::Error,
    host::ISMPHost,
    messaging::Proof,
    router::RequestResponse,
};
use patricia_merkle_trie::{EIP1186Layout, StorageProof};
use rlp::Rlp;
use rlp_derive::RlpDecodable;
use sp_std::prelude::*;
use sync_committee_primitives::derived_types::{LightClientState, LightClientUpdate};
use trie_db::{DBValue, Trie, TrieDBBuilder};

pub struct KeccakHasher;

impl Hasher for KeccakHasher {
    type Out = H256;
    type StdHasher = Hash256StdHasher;
    const LENGTH: usize = 32;

    fn hash(x: &[u8]) -> Self::Out {
        sp_io::hashing::keccak_256(x).into()
    }
}

#[derive(Debug, Encode, Decode, Clone)]
pub struct ConsensusState {
    pub frozen_height: Option<u64>,
    pub light_client_state: LightClientState,
}

#[derive(Encode, Decode)]
pub struct Misbehaviour {
    pub update_1: LightClientUpdate,
    pub update_2: LightClientUpdate,
}

#[derive(Encode, Decode)]
pub enum BeaconMessage {
    ConsensusUpdate(LightClientUpdate),
    Misbehaviour(Misbehaviour),
}

/// Slot index for requests map
const REQ_SLOT: u8 = 1;
/// Slot index for responses map
const RESP_SLOT: u8 = 2;

const CONTRACT_ADDRESS: [u8; 20] = hex!("b856af30b938b6f52e5bff365675f358cd52f91b");
#[derive(Encode, Decode, Clone)]
pub struct EvmStateProof {
    pub contract_proof: Vec<Vec<u8>>,
    pub storage_proof: Vec<Vec<u8>>,
}

/// The ethereum account stored in the global state trie.
#[derive(RlpDecodable)]
struct Account {
    _nonce: u64,
    _balance: U256,
    storage_root: H256,
    _code_hash: H256,
}

/// Unbonding period for ethereum after which unstaked validators can withdraw their funds
const UNBONDING_PERIOD_HOURS: u64 = 27;
/// State machine id used for the ethereum execution layer.
const EXECUTION_PAYLOAD_STATE_ID: u64 = 1;

#[derive(Default, Clone)]
pub struct BeaconConsensusClient;

impl ConsensusClient for BeaconConsensusClient {
    fn verify_consensus(
        &self,
        _host: &dyn ISMPHost,
        trusted_consensus_state: Vec<u8>,
        consensus_proof: Vec<u8>,
    ) -> Result<(Vec<u8>, Vec<IntermediateState>), Error> {
        let beacon_message = BeaconMessage::decode(&mut &consensus_proof[..]).map_err(|_| {
            Error::ImplementationSpecific("Cannot decode beacon message".to_string())
        })?;

        match beacon_message {
            BeaconMessage::ConsensusUpdate(light_client_update) => {
                let consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
                    .map_err(|_| {
                        Error::ImplementationSpecific(
                            "Cannot decode trusted consensus state".to_string(),
                        )
                    })?;

                let no_codec_light_client_state =
                    consensus_state.light_client_state.try_into().map_err(|_| {
                        Error::ImplementationSpecific(format!(
                            "Cannot convert light client state to no codec type",
                        ))
                    })?;

                let no_codec_light_client_update =
                    light_client_update.clone().try_into().map_err(|_| {
                        Error::ImplementationSpecific(format!(
                            "Cannot convert light client update to no codec type"
                        ))
                    })?;

                let new_light_client_state =
                    sync_committee_verifier::verify_sync_committee_attestation(
                        no_codec_light_client_state,
                        no_codec_light_client_update,
                    )
                    .map_err(|_| Error::ConsensusProofVerificationFailed {
                        id: ETHEREUM_CONSENSUS_CLIENT_ID,
                    })?;

                let mut intermediate_states = vec![];

                let state_root = light_client_update.execution_payload.state_root;
                let intermediate_state = construct_intermediate_state(
                    EXECUTION_PAYLOAD_STATE_ID,
                    ETHEREUM_CONSENSUS_CLIENT_ID,
                    light_client_update.execution_payload.block_number,
                    light_client_update.execution_payload.timestamp,
                    state_root,
                )?;

                intermediate_states.push(intermediate_state);

                let new_consensus_state = ConsensusState {
                    frozen_height: None,
                    light_client_state: new_light_client_state.try_into().map_err(|_| {
                        Error::ImplementationSpecific(format!(
                            "Cannot convert light client state to codec type"
                        ))
                    })?,
                };

                Ok((new_consensus_state.encode(), intermediate_states))
            }
            _ => unimplemented!(),
        }
    }

    fn unbonding_period(&self) -> Duration {
        Duration::from_secs(UNBONDING_PERIOD_HOURS * 60 * 60)
    }

    fn verify_membership(
        &self,
        host: &dyn ISMPHost,
        item: RequestResponse,
        root: StateCommitment,
        proof: &Proof,
    ) -> Result<(), Error> {
        let evm_state_proof = decode_evm_state_proof(proof)?;
        let key = req_res_to_key(host, item);
        let root = H256::from_slice(&root.state_root[..]);
        let contract_root =
            get_contract_storage_root(evm_state_proof.contract_proof, root.clone())?;
        let _ = get_value_from_proof(key, contract_root, evm_state_proof.storage_proof)?
            .ok_or_else(|| {
                Error::MembershipProofVerificationFailed(format!("There is no DB value"))
            })?;

        Ok(())
    }

    fn verify_state_proof(
        &self,
        _host: &dyn ISMPHost,
        _key: Vec<u8>,
        _root: StateCommitment,
        _proof: &Proof,
    ) -> Result<Vec<u8>, Error> {
        unimplemented!()
    }

    fn verify_non_membership(
        &self,
        host: &dyn ISMPHost,
        item: RequestResponse,
        root: StateCommitment,
        proof: &Proof,
    ) -> Result<(), Error> {
        let evm_state_proof = decode_evm_state_proof(proof)?;

        let key = req_res_to_key(host, item);
        let root = H256::from_slice(&root.state_root[..]);
        let contract_root = get_contract_storage_root(evm_state_proof.contract_proof, root)?;

        let result = get_value_from_proof(key, contract_root, evm_state_proof.storage_proof)?;

        if result.is_some() {
            return Err(Error::NonMembershipProofVerificationFailed(
                "Invalid membership proof".to_string(),
            ))
        }

        Ok(())
    }

    fn is_frozen(&self, consensus_state: &[u8]) -> Result<(), Error> {
        let consensus_state = ConsensusState::decode(&mut &consensus_state[..]).map_err(|_| {
            Error::ImplementationSpecific("Cannot decode trusted consensus state".to_string())
        })?;
        if consensus_state.frozen_height.is_some() {
            Err(Error::FrozenConsensusClient { id: ETHEREUM_CONSENSUS_CLIENT_ID })
        } else {
            Ok(())
        }
    }
}

fn construct_intermediate_state(
    state_id: u64,
    consensus_client_id: u64,
    height: u64,
    timestamp: u64,
    state_root: Vec<u8>,
) -> Result<IntermediateState, Error> {
    let state_machine_id = StateMachineId { state_id, consensus_client: consensus_client_id };

    let state_machine_height = StateMachineHeight { id: state_machine_id, height };

    let state_commitment = StateCommitment {
        timestamp,
        ismp_root: [0u8; 32],
        state_root: to_bytes_32(state_root)?.into(),
    };

    let intermediate_state =
        IntermediateState { height: state_machine_height, commitment: state_commitment };

    Ok(intermediate_state)
}

fn decode_evm_state_proof(proof: &Proof) -> Result<EvmStateProof, Error> {
    let proof_vec = proof.proof.clone();
    let evm_state_proof = EvmStateProof::decode(&mut &proof_vec[..]).map_err(|_| {
        Error::ImplementationSpecific(format!("Cannot decode evm state proof {:?}", proof_vec))
    })?;

    Ok(evm_state_proof)
}

fn req_res_to_key(host: &dyn ISMPHost, item: RequestResponse) -> Vec<u8> {
    match item {
        RequestResponse::Request(request) => {
            let commitment = host.get_request_commitment(&request);
            let unhashed = derive_unhashed_map_key(commitment, REQ_SLOT);
            host.keccak256(&unhashed).to_vec()
        }
        RequestResponse::Response(response) => {
            let commitment = host.get_response_commitment(&response);
            let unhashed = derive_unhashed_map_key(commitment, RESP_SLOT);
            host.keccak256(&unhashed).to_vec()
        }
    }
}

fn to_bytes_32(vec: Vec<u8>) -> Result<[u8; 32], Error> {
    if vec.len() != 32 {
        return Err(Error::ImplementationSpecific(format!(
            "Input vector must have exactly 32 elements {:?}",
            vec
        )))
    }

    let mut array = [0u8; 32];

    array.copy_from_slice(&vec);

    Ok(array)
}

fn get_contract_storage_root(
    contract_account_proof: Vec<Vec<u8>>,
    root: H256,
) -> Result<H256, Error> {
    use rlp::Decodable;
    let db = StorageProof::new(contract_account_proof).into_memory_db::<KeccakHasher>();
    let trie = TrieDBBuilder::<EIP1186Layout<KeccakHasher>>::new(&db, &root).build();
    let contract_address = H160::from_slice(&CONTRACT_ADDRESS[..]);
    let key = ethabi::encode(&[Token::Address(contract_address)]);
    let result = trie
        .get(&key)
        .map_err(|_| Error::ImplementationSpecific("Invalid contract account proof".to_string()))?
        .ok_or_else(|| {
            Error::ImplementationSpecific("Contract account is not present in proof".to_string())
        })?;

    let contract_account = <Account as Decodable>::decode(&Rlp::new(&result)).map_err(|_| {
        Error::ImplementationSpecific(format!(
            "Error decoding contract account from key {:?}",
            &result
        ))
    })?;

    Ok(contract_account.storage_root)
}

fn derive_unhashed_map_key(key: Vec<u8>, slot: u8) -> Vec<u8> {
    ethabi::encode(&[Token::FixedBytes(key), Token::Int(U256::from(slot))])
}

fn get_value_from_proof(
    key: Vec<u8>,
    root: H256,
    proof: Vec<Vec<u8>>,
) -> Result<Option<DBValue>, Error> {
    let proof_db = StorageProof::new(proof).into_memory_db::<KeccakHasher>();
    let trie = TrieDBBuilder::<EIP1186Layout<KeccakHasher>>::new(&proof_db, &root).build();

    trie.get(&key).map_err(|_| Error::ImplementationSpecific(format!("Error reading proof db")))
}
