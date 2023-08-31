use crate::{
    prelude::*,
    presets::{REQUEST_COMMITMENTS_SLOT, RESPONSE_COMMITMENTS_SLOT},
    types::{Account, EvmStateProof, KeccakHasher},
};
use alloc::{collections::BTreeMap, format, string::ToString};
use codec::Decode;
use ethabi::{
    ethereum_types::{H160, H256, U256},
    Token,
};
use ismp::{
    consensus::{
        ConsensusStateId, IntermediateState, StateCommitment, StateMachineHeight, StateMachineId,
    },
    error::Error,
    host::{IsmpHost, StateMachine},
    messaging::Proof,
    router::RequestResponse,
    util::{hash_request, hash_response},
};
use patricia_merkle_trie::{EIP1186Layout, StorageProof};
use rlp::Rlp;
use trie_db::{DBValue, Trie, TrieDBBuilder};

pub fn construct_intermediate_state(
    state_id: StateMachine,
    consensus_state_id: ConsensusStateId,
    height: u64,
    timestamp: u64,
    state_root: &[u8],
) -> Result<IntermediateState, Error> {
    let state_machine_id = StateMachineId { state_id, consensus_state_id };

    let state_machine_height = StateMachineHeight { id: state_machine_id, height };

    let state_commitment = StateCommitment {
        timestamp,
        overlay_root: None,
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

pub fn req_res_to_key<H: IsmpHost>(item: RequestResponse) -> Vec<Vec<u8>> {
    let mut keys = vec![];
    match item {
        RequestResponse::Request(requests) =>
            for req in requests {
                let commitment = hash_request::<H>(&req);
                let unhashed =
                    derive_unhashed_map_key(commitment.0.to_vec(), REQUEST_COMMITMENTS_SLOT);
                let key = H::keccak256(&unhashed).0.to_vec();
                keys.push(key)
            },
        RequestResponse::Response(responses) =>
            for res in responses {
                let commitment = hash_response::<H>(&res);
                let unhashed =
                    derive_unhashed_map_key(commitment.0.to_vec(), RESPONSE_COMMITMENTS_SLOT);
                let key = H::keccak256(&unhashed).0.to_vec();
                keys.push(key)
            },
    }

    keys
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

pub fn get_contract_storage_root<H: IsmpHost + Send + Sync>(
    contract_account_proof: Vec<Vec<u8>>,
    contract_address: H160,
    root: H256,
) -> Result<H256, Error> {
    use rlp::Decodable;
    let db = StorageProof::new(contract_account_proof).into_memory_db::<KeccakHasher<H>>();
    let trie = TrieDBBuilder::<EIP1186Layout<KeccakHasher<H>>>::new(&db, &root).build();
    let key = H::keccak256(contract_address.as_bytes()).0;
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

pub(super) fn derive_array_item_key<H: IsmpHost>(slot: u8, index: u64, offset: u64) -> Vec<u8> {
    let mut bytes = [0u8; 32];
    U256::from(slot as u64).to_big_endian(&mut bytes);

    let hash_result = H::keccak256(&bytes);

    let array_pos = U256::from_big_endian(&hash_result.0);
    let item_pos = array_pos + U256::from(index * 2) + U256::from(offset);

    let mut pos = [0u8; 32];
    item_pos.to_big_endian(&mut pos);

    H::keccak256(&pos).0.to_vec()
}

pub(super) fn get_values_from_proof<H: IsmpHost + Send + Sync>(
    keys: Vec<Vec<u8>>,
    root: H256,
    mut proof: BTreeMap<Vec<u8>, Vec<Vec<u8>>>,
) -> Result<Vec<Option<DBValue>>, Error> {
    let mut values = vec![];
    for key in keys {
        let proof_db = StorageProof::new(
            proof
                .remove(&key)
                .ok_or_else(|| Error::ImplementationSpecific(format!("Missing proof")))?,
        )
        .into_memory_db::<KeccakHasher<H>>();
        let trie = TrieDBBuilder::<EIP1186Layout<KeccakHasher<H>>>::new(&proof_db, &root).build();
        let val = trie
            .get(&key)
            .map_err(|_| Error::ImplementationSpecific(format!("Error reading proof db")))?;
        values.push(val);
    }

    Ok(values)
}

pub(super) fn get_value_from_proof<H: IsmpHost + Send + Sync>(
    key: Vec<u8>,
    root: H256,
    proof: Vec<Vec<u8>>,
) -> Result<Option<DBValue>, Error> {
    let proof_db = StorageProof::new(proof).into_memory_db::<KeccakHasher<H>>();
    let trie = TrieDBBuilder::<EIP1186Layout<KeccakHasher<H>>>::new(&proof_db, &root).build();
    let val = trie
        .get(&key)
        .map_err(|e| Error::ImplementationSpecific(format!("Error reading proof db {:?}", e)))?;

    Ok(val)
}
