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
    ConsensusState, PROOF_TYPE_SP1, Sp1BeefyProof, VK_ROOT_V6_1_0_BYTES, extract_vk_root,
    verify_sp1_v6,
};

// Fixture provenance: SCALE-encoded BEEFY consensus update artefacts copied
// verbatim from `modules/pallets/beefy-consensus-proofs/src/benchmarking.rs`
// (TRUSTED_STATE @ line 37, WIRE_PROOF @ line 39, FIXTURE_VKEY @ line 41).
// Same fixture as `evm/test/SP1BeefyTest.sol::testVerifySp1Optional` and
// `modules/consensus/beefy/verifier/src/test.rs:285-337`. Block #30,701,354.

const TRUSTED_STATE_SCALE_HEX: &str = "2279d60118532a010000000000000000000000000000000000000000000000000000000000000000751200000000000057020000a7161e52f2f4249039441385a41c6c8e36207a9b6a65d9bfae4272156ec31f49751200000000000057020000a7161e52f2f4249039441385a41c6c8e36207a9b6a65d9bfae4272156ec31f49";
const WIRE_PROOF_HEX: &str = "012a79d6017512000000000000002979d601e1dbc67b9da4b90227fb3dc2e7ffdce4e120d583502399e4bd083c02651ca5eb761200000000000057020000a7161e52f2f4249039441385a41c6c8e36207a9b6a65d9bfae4272156ec31f4963bc2eb07f9c83afe64eb8815b626cd0a7d2a1bbb4630a44a1896af297d0135d04e504739e9bd7f1addf87db9b6a762bd0e1713baa895c3b82b4595080e5ba02fb5b3cf2915702b49122c32b822e6a11384074d8902d5ea5f79c7cb0d7804e49501b8b532298f49e38d3f7140ce1ba61c243152e4e380b37eb628e08d5270d8b2c5e4ebedd84bb14066175726120fbc4d208000000000452505352902a869d4e00b3bb93f1e88e41a2b5f51fc637626b4ce1da15749ef2d79de4797a9ae459070449534d50010118a13886ac93d163a1d22cdef94e018eba5189424a66b7bd03a5ac232beb46bf08b0f9d2b979fff833d7e21a64a5183c61e2630c0b452236baba3c1b4ff41821044953544d20ca3be169000000000561757261010152d45dea4dcf058b0610e12981e0e4c97ad153f26481510c0b78beedf1848b4dd2abd37b8c6b800b72fa12199898eca7651471b49e38d6167a84fb6e2df7c78400000000270d000091054388a21c0000000000000000000000000000000000000000000000000000000000000000002f850ee998974d6cc00e50cd0814b098c05bfade466d28573240d057f2535200000000000000000000000000000000000000000000000000000000000000002ac5e596c552ee76353c176f0870e47a0aa765ceafc4c65b03dbf434e27fa9062f185bdc40f7aae982c1c8c6b766dd491a1e1cd60128efbc58da965e5be96320287f4ce1b04538f0c8287c8eff096c36df67dc17970032546c9b3d4dd5510c5c25e880e13469e1e1aca1b41c367f2ecf04da65f7602fb53ec212b03d0148157b2cd9a79a9779f350d240e6d4c980848302fca8c7447c5fa7ac8d3c6eefcd0c640acff8b27ea316db978652553e3d054765094cf0dab6085a616489cdb973c42b258e22f346ac3ceb3e2e6750c37dad1f98f6ca15d1f70659343caa52dbbcad150b75dd2dcf0ba0a664ea4605b291df54ab1aa5b4c55034b9425ba29cc87eca7b";
const SP1_VKEY_HASH_HEX: &str =
    "0x0059fd0bff44da77999bb7974cbcf2ac7dc89e5869352f20a2f3cd46c9f53d5c";

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

fn decode_hex_to_32(hex_with_prefix: &str) -> [u8; 32] {
    let stripped = hex_with_prefix.strip_prefix("0x").unwrap();
    hex::decode(stripped).unwrap().try_into().unwrap()
}

#[test]
fn verifies_real_fixture_end_to_end() {
    let trusted_bytes = hex::decode(TRUSTED_STATE_SCALE_HEX).unwrap();
    let wire_bytes = hex::decode(WIRE_PROOF_HEX).unwrap();
    assert_eq!(wire_bytes[0], PROOF_TYPE_SP1);

    let trusted = ConsensusState::decode(&mut &trusted_bytes[..]).unwrap();
    let sp1_proof = Sp1BeefyProof::decode(&mut &wire_bytes[1..]).unwrap();

    let public_inputs = build_public_inputs(&trusted, &sp1_proof);

    let extracted = extract_vk_root(&sp1_proof.proof).unwrap();
    assert_eq!(
        extracted, VK_ROOT_V6_1_0_BYTES,
        "fixture vk_root must match the hardcoded constant",
    );

    let sp1_vkey_hash = decode_hex_to_32(SP1_VKEY_HASH_HEX);
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
