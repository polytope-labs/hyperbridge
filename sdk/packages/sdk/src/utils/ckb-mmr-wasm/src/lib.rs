use std::collections::HashMap;

use ckb_merkle_mountain_range::{util::MemStore, Merge, Result as CkbResult, MMR};
use hex;
use serde::{Deserialize, Serialize};
use tiny_keccak::{Hasher, Keccak};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsError;

#[derive(Serialize, Deserialize, Debug)]
pub struct MMRResult {
    pub root: String,
    pub proof: Vec<String>,
    pub mmr_size: u64,
    pub leaf_positions: Vec<u64>,
    pub keccak_hash_calldata: String,
}

#[wasm_bindgen]
#[derive(Debug)]
struct KeccakMerge;
impl Merge for KeccakMerge {
    type Item = Vec<u8>;
    fn merge(lhs: &Self::Item, rhs: &Self::Item) -> CkbResult<Self::Item> {
        let mut input = Vec::new();
        input.extend_from_slice(lhs);
        input.extend_from_slice(rhs);

        let mut keccak = Keccak::v256();
        let mut result = [0u8; 32];
        keccak.update(&input);
        keccak.finalize(&mut result);

        Ok(result.to_vec())
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

fn bytes_to_hex_proof(bytes: &[Vec<u8>]) -> Vec<String> {
    bytes.iter().map(|item| bytes_to_hex(item)).collect()
}

fn hash_leaf(data: &[u8]) -> Vec<u8> {
    let mut keccak = Keccak::v256();
    let mut result = [0u8; 32];
    keccak.update(data);
    keccak.finalize(&mut result);
    result.to_vec()
}

#[wasm_bindgen]
pub fn generate_root_with_proof(calldata_bytes: &[u8], tree_size: u64) -> Result<String, JsError> {
    let store = MemStore::<Vec<u8>>::default();
    let mut mmr = MMR::<Vec<u8>, KeccakMerge, &MemStore<Vec<u8>>>::new(0, &store);
    let mut leaf_positions = Vec::new();

    for i in 0..tree_size {
        let mut tx = calldata_bytes.to_vec();

        if i != tree_size - 1 {
            for j in 0..tx.len() {
                tx[j] ^= i as u8;
            }
        }

        let hashed_leaf = hash_leaf(&tx);
        let pos = mmr.push(hashed_leaf)?;

        if i == tree_size - 1 {
            leaf_positions.push(pos);
        }
    }

    let root = mmr.get_root()?;
    let root_hex = bytes_to_hex(&root);

    let proof = mmr.gen_proof(leaf_positions.clone())?;

    let proof_hex = bytes_to_hex_proof(proof.proof_items());
    let mmr_size = mmr.mmr_size();

    Ok(serde_json::to_string(&MMRResult {
        root: root_hex,
        proof: proof_hex,
        mmr_size,
        leaf_positions,
        keccak_hash_calldata: bytes_to_hex(&hash_leaf(calldata_bytes)),
    })?)
}

#[wasm_bindgen]
pub fn verify_proof(
    root_hex: &str,
    proof_hex: Vec<String>,
    mmr_size: u64,
    leaf_position: u64,
    calldata_bytes: &[u8],
) -> Result<bool, JsError> {
    let root = hex::decode(root_hex.trim_start_matches("0x"))
        .map_err(|e| JsError::new(&format!("Failed to decode root: {}", e)))?;

    let proof_items: Vec<Vec<u8>> = proof_hex
        .iter()
        .map(|item| {
            hex::decode(item.trim_start_matches("0x"))
                .map_err(|e| JsError::new(&format!("Failed to decode proof item: {}", e)))
        })
        .collect::<Result<Vec<Vec<u8>>, JsError>>()?;

    let proof = ckb_merkle_mountain_range::MerkleProof::<Vec<u8>, KeccakMerge>::new(
        mmr_size,
        proof_items.clone(),
    );

    let hashed_leaf = hash_leaf(calldata_bytes);

    // For debugging
    #[cfg(test)]
    {
        println!("Verifying leaf at position: {}", leaf_position);
        println!("Leaf hash: {:?}", hashed_leaf);
        println!("Root: {:?}", root);
        println!("Proof items: {:?}", proof_items);
    }

    let result = proof.verify(root.clone(), vec![(leaf_position, hashed_leaf.clone())]);

    // For debugging
    #[cfg(test)]
    {
        println!("Verification result: {:?}", result);
    }

    // Extract the boolean result from the Result
    let is_valid = result.unwrap_or(false);

    Ok(is_valid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proof_verification() {
        let calldata_bytes: Vec<u8> = (0..32).map(|i| i as u8).collect();
        let tree_size = 10u64;

        println!("Original calldata: {:?}", calldata_bytes);
        let root_with_proof = generate_root_with_proof(&calldata_bytes, tree_size).unwrap();
        let result: MMRResult = serde_json::from_str(&root_with_proof).unwrap();

        let last_leaf_position = result.leaf_positions[result.leaf_positions.len() - 1];
        let proof_result = verify_proof(
            &result.root,
            result.proof.clone(),
            result.mmr_size,
            last_leaf_position,
            &calldata_bytes,
        )
        .unwrap();

        assert!(proof_result);

        let mut modified_calldata = calldata_bytes.clone();
        modified_calldata[0] ^= 1; // Flip one bit
        println!("Modified calldata: {:?}", modified_calldata);

        let modified_proof_result = verify_proof(
            &result.root,
            result.proof,
            result.mmr_size,
            last_leaf_position,
            &modified_calldata,
        )
        .unwrap();

        println!("Modified Proof Result: {}", modified_proof_result);
        assert!(!modified_proof_result);
    }
}
