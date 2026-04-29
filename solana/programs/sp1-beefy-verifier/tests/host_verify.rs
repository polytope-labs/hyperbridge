//! Host-side smoke test: decode the real Hyperbridge BEEFY SP1 v6 fixture
//! (lifted verbatim from `modules/pallets/beefy-consensus-proofs/src/
//! benchmarking.rs:37,39`) and run [`verify_sp1_v6`] end-to-end.
//!
//! Intentionally runs natively (no validator). The on-chain CU and tx-size
//! measurements live in the `onchain_tx` example.

use alloy_primitives::{FixedBytes, U256};
use alloy_sol_types::{SolValue, sol};
use parity_scale_codec::{Decode, Encode};
use sha3::{Digest, Keccak256};

use sp1_beefy_verifier::{
    fixtures::{sp1_vkey_hash, trusted_state_bytes, wire_proof_bytes},
    ConsensusState, PROOF_TYPE_SP1, Sp1BeefyProof, VK_ROOT_V6_1_0_BYTES, extract_vk_root,
    verify_sp1_v6,
};

sol! {
    struct ParachainHeaderHash {
        uint256 id;
        bytes32 hash;
    }
    struct PublicInputs {
        bytes32 authorities_root;
        uint256 authorities_len;
        bytes32 leaf_hash;
        ParachainHeaderHash[] headers;
    }
}

fn keccak256(bytes: &[u8]) -> [u8; 32] {
    let mut h = Keccak256::new();
    h.update(bytes);
    h.finalize().into()
}

fn build_public_inputs(trusted: &ConsensusState, proof: &Sp1BeefyProof) -> Vec<u8> {
    let authority = if proof.validator_set_id == trusted.next_authorities.id {
        &trusted.next_authorities
    } else if proof.validator_set_id == trusted.current_authorities.id {
        &trusted.current_authorities
    } else {
        panic!("validator_set_id matches neither current nor next authorities");
    };

    let headers: Vec<ParachainHeaderHash> = proof
        .headers
        .iter()
        .map(|h| ParachainHeaderHash {
            id: U256::from(h.para_id),
            hash: FixedBytes::from(keccak256(&h.header)),
        })
        .collect();

    PublicInputs {
        authorities_root: FixedBytes::from(authority.keyset_commitment),
        authorities_len: U256::from(authority.len),
        leaf_hash: FixedBytes::from(keccak256(&proof.mmr_leaf.encode())),
        headers,
    }
    .abi_encode()
}

#[test]
fn verifies_real_fixture_end_to_end() {
    let trusted_bytes = trusted_state_bytes();
    let wire_bytes = wire_proof_bytes();
    assert_eq!(wire_bytes[0], PROOF_TYPE_SP1);

    let trusted = ConsensusState::decode(&mut &trusted_bytes[..]).unwrap();
    let sp1_proof = Sp1BeefyProof::decode(&mut &wire_bytes[1..]).unwrap();

    let public_inputs = build_public_inputs(&trusted, &sp1_proof);

    let extracted = extract_vk_root(&sp1_proof.proof).unwrap();
    assert_eq!(
        extracted, VK_ROOT_V6_1_0_BYTES,
        "fixture vk_root must match the hardcoded constant",
    );

    let sp1_vkey_hash = sp1_vkey_hash();
    let exit_code_success = [0u8; 32];

    let (exit_code, vk_root, _proof_nonce) = verify_sp1_v6(
        &sp1_proof.proof,
        &public_inputs,
        &sp1_vkey_hash,
        &VK_ROOT_V6_1_0_BYTES,
        &exit_code_success,
    )
    .expect("verification should succeed against the real fixture");

    assert_eq!(exit_code, exit_code_success);
    assert_eq!(vk_root, VK_ROOT_V6_1_0_BYTES);
}
