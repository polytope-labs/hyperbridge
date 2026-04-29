//! Runs the real Hyperbridge BEEFY SP1 fixture (block #30,701,354)
//! through the handler's `verify_and_extract_update` pipeline.
//!
//! Native test — no Solana runtime, no program-test scaffolding. Covers
//! the path inside `ConsensusClient::verify_consensus`: SCALE-decode
//! envelope, decode trusted state, run SP1 v6 Groth16 verification,
//! extract per-parachain commitments. Fixture provenance is the same
//! as `sp1-beefy-verifier/tests/host_verify.rs`.

use parity_scale_codec::Decode;
use sp1_beefy_verifier::{
    fixtures::{sp1_vkey_hash, trusted_state_bytes, wire_proof_bytes},
    ConsensusState, Sp1BeefyProof, PROOF_TYPE_SP1,
};

use handler::verifier::beefy::{
    extract_header_prefix, maybe_rotate_authorities, verify_and_extract_update,
};

#[test]
fn verify_and_extract_update_accepts_real_fixture() {
    let trusted_bytes = trusted_state_bytes();
    let wire_bytes = wire_proof_bytes();
    let sp1_vkey = sp1_vkey_hash();

    assert_eq!(wire_bytes[0], PROOF_TYPE_SP1, "wire envelope tag");
    let mut input = &wire_bytes[1..];
    let proof = Sp1BeefyProof::decode(&mut input).expect("decode proof");
    let mut input = trusted_bytes.as_slice();
    let mut state = ConsensusState::decode(&mut input).expect("decode state");

    let update = verify_and_extract_update(&state, &proof, &sp1_vkey)
        .expect("real fixture should verify");

    assert!(update.new_height > state.latest_beefy_height);
    assert_eq!(update.new_height, proof.block_number);
    assert_eq!(update.authority_set_id, proof.validator_set_id);
    assert_eq!(update.commitments.len(), proof.headers.len());

    // Each emitted commitment lines up with the corresponding header's
    // SCALE-encoded prefix.
    for ((para_id, num, root), header) in update.commitments.iter().zip(&proof.headers) {
        let (h_num, h_root) = extract_header_prefix(&header.header).unwrap();
        assert_eq!(*para_id, header.para_id);
        assert_eq!(*num, h_num);
        assert_eq!(*root, h_root);
    }

    // Rotation only fires when the proof was signed by `next_authorities`.
    // After rotation, `next_authorities` becomes the new set from the mmr
    // leaf — that's the observable bit (current may end up with the same
    // id when both pre-rotation sets shared an id, as happens here).
    let rotates = proof.validator_set_id == state.next_authorities.id;
    let pre_next_id = state.next_authorities.id;
    let leaf_next_id = proof.mmr_leaf.beefy_next_authority_set.id;
    maybe_rotate_authorities(&mut state, &proof);
    if rotates {
        assert_eq!(state.next_authorities.id, leaf_next_id);
    } else {
        assert_eq!(state.next_authorities.id, pre_next_id);
    }
}

#[test]
fn verify_rejects_non_monotonic_height() {
    let trusted = trusted_state_bytes();
    let mut state = ConsensusState::decode(&mut trusted.as_slice()).unwrap();

    let wire_bytes = wire_proof_bytes();
    let mut input = &wire_bytes[1..];
    let proof = Sp1BeefyProof::decode(&mut input).unwrap();
    let sp1_vkey = sp1_vkey_hash();

    state.latest_beefy_height = proof.block_number + 1;
    let err = verify_and_extract_update(&state, &proof, &sp1_vkey);
    assert!(err.is_err(), "non-monotonic height must fail");
}

#[test]
fn verify_rejects_unknown_authority_set() {
    let trusted_bytes = trusted_state_bytes();
    let wire_bytes = wire_proof_bytes();
    let sp1_vkey = sp1_vkey_hash();

    let mut state = ConsensusState::decode(&mut trusted_bytes.as_slice()).unwrap();
    let mut input = &wire_bytes[1..];
    let proof = Sp1BeefyProof::decode(&mut input).unwrap();

    state.current_authorities.id = proof.validator_set_id.wrapping_add(100);
    state.next_authorities.id = proof.validator_set_id.wrapping_add(200);
    let err = verify_and_extract_update(&state, &proof, &sp1_vkey);
    assert!(err.is_err(), "unknown authority set must fail");
}
