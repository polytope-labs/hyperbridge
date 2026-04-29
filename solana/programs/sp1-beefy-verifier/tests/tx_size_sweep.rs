//! Multi-header tx-size sweep.
//!
//! The SP1 v6 Groth16 proof is fixed at 356 B regardless of how many
//! parachain headers the inner circuit processed. Only `public_inputs`
//! (Solidity-ABI-encoded) grows, by exactly 64 B per additional header.
//!
//! This test synthesizes `public_inputs` for `N = 1..=10` headers — using
//! the real SP1 proof bytes but dummy parachain-hash entries — builds a
//! fully-signed Solana transaction for each `N`, and asserts the wire
//! size against the per-tx cap.
//!
//! No validator required. Verification would fail on-chain for `N > 1`
//! (the SP1 proof commits to one specific public-input vector), but tx
//! wire size is identical to a real multi-header proof.

use alloy_primitives::{FixedBytes, U256};
use alloy_sol_types::{SolValue, sol};
use parity_scale_codec::{Decode, Encode};
use sha3::{Digest, Keccak256};

use sp1_beefy_verifier::{
    fixtures::{sp1_vkey_hash, trusted_state_bytes, wire_proof_bytes},
    ConsensusState, PROOF_TYPE_SP1, Sp1BeefyProof,
};

use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

const TX_CAP: usize = 1232;
const MAX_N: u32 = 10;

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

fn build_public_inputs_padded(
    trusted: &ConsensusState,
    proof: &Sp1BeefyProof,
    n_headers: u32,
) -> Vec<u8> {
    let authority = if proof.validator_set_id == trusted.next_authorities.id {
        &trusted.next_authorities
    } else {
        &trusted.current_authorities
    };

    let real: Vec<ParachainHeaderHash> = proof
        .headers
        .iter()
        .map(|h| ParachainHeaderHash {
            id: U256::from(h.para_id),
            hash: FixedBytes::from(keccak256(&h.header)),
        })
        .collect();

    let mut headers = Vec::with_capacity(n_headers as usize);
    headers.extend(real.iter().cloned());
    for i in (real.len() as u32)..n_headers {
        headers.push(ParachainHeaderHash {
            id: U256::from(2000u32 + i),
            hash: FixedBytes::from([0xabu8; 32]),
        });
    }

    PublicInputs {
        authorities_root: FixedBytes::from(authority.keyset_commitment),
        authorities_len: U256::from(authority.len),
        leaf_hash: FixedBytes::from(keccak256(&proof.mmr_leaf.encode())),
        headers,
    }
    .abi_encode()
}

fn build_tx_size(
    program_id: Pubkey,
    payer: &Keypair,
    trusted: &ConsensusState,
    sp1_proof: &Sp1BeefyProof,
    sp1_vkey_hash: [u8; 32],
    n_headers: u32,
) -> usize {
    let public_inputs = build_public_inputs_padded(trusted, sp1_proof, n_headers);
    let proof_bytes = &sp1_proof.proof;

    let mut data = Vec::with_capacity(32 + 4 + proof_bytes.len() + public_inputs.len());
    data.extend_from_slice(&sp1_vkey_hash);
    data.extend_from_slice(&(proof_bytes.len() as u32).to_be_bytes());
    data.extend_from_slice(proof_bytes);
    data.extend_from_slice(&public_inputs);

    let ix = Instruction {
        program_id,
        accounts: vec![AccountMeta::new_readonly(payer.pubkey(), true)],
        data,
    };
    let cu_budget = ComputeBudgetInstruction::set_compute_unit_limit(1_400_000);

    let tx = Transaction::new_signed_with_payer(
        &[cu_budget, ix],
        Some(&payer.pubkey()),
        &[payer],
        Hash::default(),
    );

    bincode::serialize(&tx).unwrap().len()
}

/// Asserts that the single-header tx fits comfortably and that at least 6
/// headers per tx are achievable. If this regresses, something material
/// changed in the encoding or in Solana's tx framing.
#[test]
fn multi_header_capacity_at_least_six() {
    let wire_bytes = wire_proof_bytes();
    let trusted_bytes = trusted_state_bytes();
    assert_eq!(wire_bytes[0], PROOF_TYPE_SP1);

    let trusted = ConsensusState::decode(&mut &trusted_bytes[..]).unwrap();
    let sp1_proof = Sp1BeefyProof::decode(&mut &wire_bytes[1..]).unwrap();
    let sp1_vkey_hash = sp1_vkey_hash();

    let payer = Keypair::new();
    let program_id = Pubkey::new_unique();

    let mut max_fit = 0u32;
    for n in 1..=MAX_N {
        let tx_size = build_tx_size(program_id, &payer, &trusted, &sp1_proof, sp1_vkey_hash, n);
        if tx_size <= TX_CAP {
            max_fit = n;
        } else {
            break;
        }
    }

    assert!(
        max_fit >= 6,
        "expected at least 6 parachain headers per tx; got {max_fit}"
    );

    // Single-header upper-bound check — guard against silent bloat.
    let single = build_tx_size(program_id, &payer, &trusted, &sp1_proof, sp1_vkey_hash, 1);
    assert!(
        single < 900,
        "single-header tx grew unexpectedly: {single} bytes"
    );
}
