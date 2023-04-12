use crate::consensus_clients::beacon_consensus_client::types::{
    Account, EvmStateProof, KeccakHasher, REQ_SLOT, RESP_SLOT,
};
use alloc::{format, string::ToString};
use codec::Decode;
use ethabi::{
    ethereum_types::{H160, H256, U256},
    Token,
};
use ismp_rs::{
    consensus_client::{IntermediateState, StateCommitment, StateMachineHeight, StateMachineId},
    error::Error,
    host::ISMPHost,
    messaging::Proof,
    router::RequestResponse,
};
use patricia_merkle_trie::{EIP1186Layout, StorageProof};
use rlp::Rlp;
use sp_std::prelude::*;
use trie_db::{DBValue, Trie, TrieDBBuilder};

pub fn construct_intermediate_state(
    state_id: u64,
    consensus_client_id: u64,
    height: u64,
    timestamp: u64,
    state_root: &[u8],
) -> Result<IntermediateState, Error> {
    let state_machine_id = StateMachineId { state_id, consensus_client: consensus_client_id };

    let state_machine_height = StateMachineHeight { id: state_machine_id, height };

    let state_commitment = StateCommitment {
        timestamp,
        ismp_root: [0u8; 32],
        state_root: to_bytes_32(&state_root[..])?.into(),
    };

    let intermediate_state =
        IntermediateState { height: state_machine_height, commitment: state_commitment };

    Ok(intermediate_state)
}

pub(super) fn decode_evm_state_proof(proof: &Proof) -> Result<EvmStateProof, Error> {
    let proof_vec = proof.proof.clone();
    let evm_state_proof = EvmStateProof::decode(&mut &proof_vec[..]).map_err(|_| {
        Error::ImplementationSpecific(format!("Cannot decode evm state proof {:?}", proof_vec))
    })?;

    Ok(evm_state_proof)
}

pub fn req_res_to_key(host: &dyn ISMPHost, item: RequestResponse) -> Vec<u8> {
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

pub(super) fn to_bytes_32(bytes: &[u8]) -> Result<[u8; 32], Error> {
    if bytes.len() != 32 {
        return Err(Error::ImplementationSpecific(format!(
            "Input vector must have exactly 32 elements {:?}",
            bytes
        )))
    }

    let mut array = [0u8; 32];

    array.copy_from_slice(&bytes);

    Ok(array)
}

pub(super) fn get_contract_storage_root(
    contract_account_proof: Vec<Vec<u8>>,
    contract_address: &[u8; 20],
    root: H256,
) -> Result<H256, Error> {
    use rlp::Decodable;
    let db = StorageProof::new(contract_account_proof).into_memory_db::<KeccakHasher>();
    let trie = TrieDBBuilder::<EIP1186Layout<KeccakHasher>>::new(&db, &root).build();
    let contract_address = H160::from_slice(contract_address);
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

pub(super) fn derive_unhashed_map_key(key: Vec<u8>, slot: u8) -> Vec<u8> {
    ethabi::encode(&[Token::FixedBytes(key), Token::Int(U256::from(slot))])
}

pub(super) fn derive_array_item_key(slot: u8, index: u64) -> Vec<u8> {
    let slot_hash = sp_io::hashing::keccak_256(&ethabi::encode(&[Token::Uint(U256::from(slot))]));
    let slot_index = U256::from_big_endian(&slot_hash[..]) + U256::from(index);
    <[u8; 32]>::from(slot_index).to_vec()
}

pub(super) fn get_value_from_proof(
    key: Vec<u8>,
    root: H256,
    proof: Vec<Vec<u8>>,
) -> Result<Option<DBValue>, Error> {
    let proof_db = StorageProof::new(proof).into_memory_db::<KeccakHasher>();
    let trie = TrieDBBuilder::<EIP1186Layout<KeccakHasher>>::new(&proof_db, &root).build();

    trie.get(&key).map_err(|_| Error::ImplementationSpecific(format!("Error reading proof db")))
}
