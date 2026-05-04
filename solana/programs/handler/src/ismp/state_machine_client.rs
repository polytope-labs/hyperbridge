//! `StateMachineClient` impl for substrate-based state machines connected
//! to Hyperbridge.
//!
//! Two distinct verification paths:
//!
//! - `verify_membership` — request/response batches are committed to a
//!   Keccak MMR maintained on the source chain. The MMR root lives in
//!   `StateCommitment.overlay_root`. Mirrors EVM `HandlerV2.handlePostRequests`
//!   (uses `MerkleMountainRange.VerifyProof`).
//! - `verify_state_proof` — non-membership / state queries against the
//!   global Substrate trie at `StateCommitment.state_root`. Used for
//!   timeout / receipt-deletion flows; mirrors EVM's state-trie
//!   non-membership checks.
//!
//! Wire formats:
//! - membership: SCALE-encoded `MmrMembershipProof { leaf_indices, items, leaf_count }`
//! - state:      SCALE-encoded `Vec<Vec<u8>>` of trie nodes (storage key
//!               supplied by caller)

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
    messaging::{hash_request, hash_response, Keccak256, Proof},
    router::RequestResponse,
};
use merkle_mountain_range::{
    leaf_index_to_pos, leaf_index_to_mmr_size, Error as MmrError, Merge, MerkleProof,
};
use parity_scale_codec::{Decode, Encode};
use primitive_types::H256;
use sha3::{Digest, Keccak256 as Sha3Keccak};

use crate::verifier::storage_proof::verify_substrate_storage_proof;

pub struct SubstrateStateMachineClient;

/// SCALE wire for request/response membership proofs. The relayer
/// supplies `leaf_count` and the multi-proof `items`; verifier hashes
/// each request locally and reconstructs the MMR root.
#[derive(Decode, Encode)]
pub struct MmrMembershipProof {
    /// 0-based leaf index for each request, in the same order as the batch.
    pub leaf_indices: Vec<u64>,
    /// Inner-node hashes for the multi-proof.
    pub items: Vec<H256>,
    /// Total leaf count of the source MMR at the proven height.
    pub leaf_count: u64,
}

/// Local `Keccak256` impl so `hash_request` / `hash_response` and
/// `KeccakMerge` can share the same hasher without going through the
/// `IsmpHost` instance.
pub struct Keccak;

impl Keccak256 for Keccak {
    fn keccak256(bytes: &[u8]) -> H256 {
        H256::from_slice(Sha3Keccak::new().chain_update(bytes).finalize().as_slice())
    }
}

/// Inner-node merge for the Hyperbridge MMR (Keccak; matches the
/// `pallet_mmr::Config` `Hashing = Keccak256` on every Hyperbridge runtime).
pub struct KeccakMerge;

impl Merge for KeccakMerge {
    type Item = H256;
    fn merge(left: &H256, right: &H256) -> core::result::Result<H256, MmrError> {
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(left.as_bytes());
        buf[32..].copy_from_slice(right.as_bytes());
        Ok(Keccak::keccak256(&buf))
    }
}

impl StateMachineClient for SubstrateStateMachineClient {
    fn verify_membership(
        &self,
        _host: &dyn IsmpHost,
        item: RequestResponse,
        root: StateCommitment,
        proof: &Proof,
    ) -> core::result::Result<(), IsmpError> {
        let mmr_root = root.overlay_root.ok_or_else(|| {
            IsmpError::MembershipProofVerificationFailed(
                "StateCommitment.overlay_root (MMR root) is not set".to_string(),
            )
        })?;

        let wire = MmrMembershipProof::decode(&mut proof.proof.as_slice())
            .map_err(|e| IsmpError::Custom(format!("decode mmr membership proof: {e:?}")))?;

        let leaf_hashes: Vec<H256> = match item {
            RequestResponse::Request(reqs) => {
                reqs.iter().map(|r| hash_request::<Keccak>(r)).collect()
            },
            RequestResponse::Response(responses) => {
                responses.iter().map(|r| hash_response::<Keccak>(r)).collect()
            },
        };

        if leaf_hashes.len() != wire.leaf_indices.len() {
            return Err(IsmpError::MembershipProofVerificationFailed(format!(
                "batch size {} != proof leaf_indices {}",
                leaf_hashes.len(),
                wire.leaf_indices.len()
            )));
        }
        if wire.leaf_count == 0 {
            return Err(IsmpError::MembershipProofVerificationFailed(
                "mmr leaf_count is zero".to_string(),
            ));
        }
        if let Some(bad) = wire.leaf_indices.iter().find(|i| **i >= wire.leaf_count) {
            return Err(IsmpError::MembershipProofVerificationFailed(format!(
                "leaf_index {bad} out of range for leaf_count {}",
                wire.leaf_count
            )));
        }

        let mmr_size = leaf_index_to_mmr_size(wire.leaf_count - 1);

        let mut leaves: Vec<(u64, H256)> = wire
            .leaf_indices
            .iter()
            .zip(leaf_hashes.iter())
            .map(|(idx, h)| (leaf_index_to_pos(*idx), *h))
            .collect();
        // ckb-merkle-mountain-range's `verify` expects leaves sorted by position.
        leaves.sort_by_key(|(p, _)| *p);

        let mmr_proof = MerkleProof::<H256, KeccakMerge>::new(mmr_size, wire.items);
        let valid = mmr_proof.verify(mmr_root, leaves).map_err(|e| {
            IsmpError::MembershipProofVerificationFailed(format!("mmr verify error: {e:?}"))
        })?;

        if !valid {
            return Err(IsmpError::MembershipProofVerificationFailed(
                "mmr root mismatch".to_string(),
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
    use merkle_mountain_range::util::MemMMR;

    #[test]
    fn mmr_membership_proof_round_trips() {
        let leaves: Vec<H256> = (0..5)
            .map(|i| Keccak::keccak256(&[i as u8; 8]))
            .collect();

        let mut mmr = MemMMR::<H256, KeccakMerge>::default();
        let positions: Vec<u64> =
            leaves.iter().map(|h| mmr.push(*h).unwrap()).collect();
        let root = mmr.get_root().unwrap();

        let target_indices = [0u64, 2, 4];
        let target_positions: Vec<u64> =
            target_indices.iter().map(|i| positions[*i as usize]).collect();
        let merkle_proof = mmr.gen_proof(target_positions).unwrap();

        let wire = MmrMembershipProof {
            leaf_indices: target_indices.to_vec(),
            items: merkle_proof.proof_items().to_vec(),
            leaf_count: leaves.len() as u64,
        };
        let encoded = wire.encode();

        let decoded = MmrMembershipProof::decode(&mut encoded.as_slice()).unwrap();
        let mmr_size = leaf_index_to_mmr_size(decoded.leaf_count - 1);
        let mut verify_leaves: Vec<(u64, H256)> = decoded
            .leaf_indices
            .iter()
            .map(|i| (leaf_index_to_pos(*i), leaves[*i as usize]))
            .collect();
        verify_leaves.sort_by_key(|(p, _)| *p);
        let proof = MerkleProof::<H256, KeccakMerge>::new(mmr_size, decoded.items);
        assert!(proof.verify(root, verify_leaves).unwrap());
    }
}
