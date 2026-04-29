//! `StateMachineClient` impl wrapping `verify_substrate_storage_proof`.
//!
//! Wire format for `Proof.proof`: `SCALE((storage_key, proof_nodes))`.
//! Storage key stays relayer-supplied because child-trie wrapping varies
//! by source chain.

extern crate alloc;
use alloc::{
    collections::BTreeMap,
    format,
    string::ToString,
    vec::Vec,
};

use ismp::{
    consensus::{StateCommitment, StateMachineClient},
    error::Error as IsmpError,
    host::IsmpHost,
    messaging::Proof,
    router::RequestResponse,
};
use parity_scale_codec::Decode;

use crate::verifier::storage_proof::verify_substrate_storage_proof;

pub struct SubstrateStateMachineClient;

#[derive(Decode)]
struct WireProof {
    storage_key: Vec<u8>,
    proof_nodes: Vec<Vec<u8>>,
}

impl StateMachineClient for SubstrateStateMachineClient {
    fn verify_membership(
        &self,
        _host: &dyn IsmpHost,
        _item: RequestResponse,
        root: StateCommitment,
        proof: &Proof,
    ) -> core::result::Result<(), IsmpError> {
        let mut input = proof.proof.as_slice();
        let wire = WireProof::decode(&mut input)
            .map_err(|e| IsmpError::Custom(format!("decode wire proof: {e:?}")))?;

        let mut state_root = [0u8; 32];
        state_root.copy_from_slice(root.state_root.as_bytes());

        let value = verify_substrate_storage_proof(&state_root, &wire.storage_key, wire.proof_nodes)
            .map_err(|e| {
                IsmpError::MembershipProofVerificationFailed(format!(
                    "storage proof verification: {e:?}"
                ))
            })?;

        if value.is_none() {
            return Err(IsmpError::MembershipProofVerificationFailed(
                "storage key provably absent at state_root".to_string(),
            ));
        }

        Ok(())
    }

    fn receipts_state_trie_key(&self, _request: RequestResponse) -> Vec<Vec<u8>> {
        Vec::new()
    }

    fn verify_state_proof(
        &self,
        _host: &dyn IsmpHost,
        keys: Vec<Vec<u8>>,
        root: StateCommitment,
        proof: &Proof,
    ) -> core::result::Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, IsmpError> {
        let mut input = proof.proof.as_slice();
        let proof_nodes: Vec<Vec<u8>> = Vec::<Vec<u8>>::decode(&mut input)
            .map_err(|e| IsmpError::Custom(format!("decode storage proof nodes: {e:?}")))?;
        let mut state_root = [0u8; 32];
        state_root.copy_from_slice(root.state_root.as_bytes());

        let mut out = BTreeMap::new();
        for key in keys {
            let value = verify_substrate_storage_proof(&state_root, &key, proof_nodes.clone())
                .map_err(|e| IsmpError::Custom(format!("verify_state_proof: {e:?}")))?;
            out.insert(key, value);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parity_scale_codec::Encode;

    #[test]
    fn wire_proof_round_trips() {
        let key = b"RequestCommitments<some-h256>".to_vec();
        let nodes: Vec<Vec<u8>> = vec![vec![1, 2, 3], vec![4, 5, 6, 7], vec![]];
        let encoded = (key.clone(), nodes.clone()).encode();

        let mut input = encoded.as_slice();
        let decoded = WireProof::decode(&mut input).unwrap();
        assert_eq!(decoded.storage_key, key);
        assert_eq!(decoded.proof_nodes, nodes);
    }
}
