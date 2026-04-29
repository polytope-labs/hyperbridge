//! Runs the real Hyperbridge BEEFY SP1 fixture (block #30,701,354)
//! through the handler's `verify_and_extract_update` pipeline.
//!
//! Native test — no Solana runtime, no program-test scaffolding. Covers
//! the path inside `ConsensusClient::verify_consensus`: SCALE-decode
//! envelope, decode trusted state, run SP1 v6 Groth16 verification,
//! extract per-parachain commitments. Fixture provenance is the same
//! as `sp1-beefy-verifier/tests/host_verify.rs`.

use parity_scale_codec::Decode;
use sp1_beefy_verifier::{ConsensusState, Sp1BeefyProof, PROOF_TYPE_SP1};

use handler::verifier::beefy::{
    extract_header_prefix, maybe_rotate_authorities, verify_and_extract_update,
};

const TRUSTED_STATE_SCALE_HEX: &str = "2279d60118532a010000000000000000000000000000000000000000000000000000000000000000751200000000000057020000a7161e52f2f4249039441385a41c6c8e36207a9b6a65d9bfae4272156ec31f49751200000000000057020000a7161e52f2f4249039441385a41c6c8e36207a9b6a65d9bfae4272156ec31f49";
const WIRE_PROOF_HEX: &str = "012a79d6017512000000000000002979d601e1dbc67b9da4b90227fb3dc2e7ffdce4e120d583502399e4bd083c02651ca5eb761200000000000057020000a7161e52f2f4249039441385a41c6c8e36207a9b6a65d9bfae4272156ec31f4963bc2eb07f9c83afe64eb8815b626cd0a7d2a1bbb4630a44a1896af297d0135d04e504739e9bd7f1addf87db9b6a762bd0e1713baa895c3b82b4595080e5ba02fb5b3cf2915702b49122c32b822e6a11384074d8902d5ea5f79c7cb0d7804e49501b8b532298f49e38d3f7140ce1ba61c243152e4e380b37eb628e08d5270d8b2c5e4ebedd84bb14066175726120fbc4d208000000000452505352902a869d4e00b3bb93f1e88e41a2b5f51fc637626b4ce1da15749ef2d79de4797a9ae459070449534d50010118a13886ac93d163a1d22cdef94e018eba5189424a66b7bd03a5ac232beb46bf08b0f9d2b979fff833d7e21a64a5183c61e2630c0b452236baba3c1b4ff41821044953544d20ca3be169000000000561757261010152d45dea4dcf058b0610e12981e0e4c97ad153f26481510c0b78beedf1848b4dd2abd37b8c6b800b72fa12199898eca7651471b49e38d6167a84fb6e2df7c78400000000270d000091054388a21c0000000000000000000000000000000000000000000000000000000000000000002f850ee998974d6cc00e50cd0814b098c05bfade466d28573240d057f2535200000000000000000000000000000000000000000000000000000000000000002ac5e596c552ee76353c176f0870e47a0aa765ceafc4c65b03dbf434e27fa9062f185bdc40f7aae982c1c8c6b766dd491a1e1cd60128efbc58da965e5be96320287f4ce1b04538f0c8287c8eff096c36df67dc17970032546c9b3d4dd5510c5c25e880e13469e1e1aca1b41c367f2ecf04da65f7602fb53ec212b03d0148157b2cd9a79a9779f350d240e6d4c980848302fca8c7447c5fa7ac8d3c6eefcd0c640acff8b27ea316db978652553e3d054765094cf0dab6085a616489cdb973c42b258e22f346ac3ceb3e2e6750c37dad1f98f6ca15d1f70659343caa52dbbcad150b75dd2dcf0ba0a664ea4605b291df54ab1aa5b4c55034b9425ba29cc87eca7b";
const SP1_VKEY_HASH_HEX: &str =
    "0059fd0bff44da77999bb7974cbcf2ac7dc89e5869352f20a2f3cd46c9f53d5c";

fn decode_hex<const N: usize>(s: &str) -> [u8; N] {
    hex::decode(s).unwrap().try_into().unwrap()
}

#[test]
fn verify_and_extract_update_accepts_real_fixture() {
    let trusted_bytes = hex::decode(TRUSTED_STATE_SCALE_HEX).unwrap();
    let wire_bytes = hex::decode(WIRE_PROOF_HEX).unwrap();
    let sp1_vkey: [u8; 32] = decode_hex(SP1_VKEY_HASH_HEX);

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
    let mut input = hex::decode(TRUSTED_STATE_SCALE_HEX).unwrap();
    let mut state = ConsensusState::decode(&mut input.as_slice()).unwrap();

    let wire_bytes = hex::decode(WIRE_PROOF_HEX).unwrap();
    let mut input = &wire_bytes[1..];
    let proof = Sp1BeefyProof::decode(&mut input).unwrap();
    let sp1_vkey: [u8; 32] = decode_hex(SP1_VKEY_HASH_HEX);

    // Bump the trusted height past the proof's height — proof should reject.
    state.latest_beefy_height = proof.block_number + 1;
    let err = verify_and_extract_update(&state, &proof, &sp1_vkey);
    assert!(err.is_err(), "non-monotonic height must fail");
}

#[test]
fn verify_rejects_unknown_authority_set() {
    let trusted_bytes = hex::decode(TRUSTED_STATE_SCALE_HEX).unwrap();
    let wire_bytes = hex::decode(WIRE_PROOF_HEX).unwrap();
    let sp1_vkey: [u8; 32] = decode_hex(SP1_VKEY_HASH_HEX);

    let mut state = ConsensusState::decode(&mut trusted_bytes.as_slice()).unwrap();
    let mut input = &wire_bytes[1..];
    let proof = Sp1BeefyProof::decode(&mut input).unwrap();

    // Shift both authority sets so neither matches the proof's id.
    state.current_authorities.id = proof.validator_set_id.wrapping_add(100);
    state.next_authorities.id = proof.validator_set_id.wrapping_add(200);
    let err = verify_and_extract_update(&state, &proof, &sp1_vkey);
    assert!(err.is_err(), "unknown authority set must fail");
}
