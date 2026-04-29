//! Build + submit a real transaction to the deployed `sp1_beefy_verifier`
//! program on a local `solana-test-validator`, then read the consumed CU
//! from the receipt.
//!
//! Lives as an example (not a test) because it requires a running validator
//! and a deployed program. Run:
//!
//! ```sh
//! # Terminal 1
//! solana-test-validator --reset
//!
//! # Terminal 2
//! solana config set --url http://127.0.0.1:8899
//! solana airdrop 10
//! cargo build-sbf --features entrypoint \
//!   --manifest-path programs/sp1-beefy-verifier/Cargo.toml
//! solana program deploy target/deploy/sp1_beefy_verifier.so
//! # copy the printed Program Id
//! PROGRAM_ID=<program-id> cargo run --example onchain-tx
//! ```

use std::env;

use alloy_primitives::{FixedBytes, U256};
use alloy_sol_types::{SolValue, sol};
use parity_scale_codec::{Decode, Encode};
use sha3::{Digest, Keccak256};

use sp1_beefy_verifier::{
    fixtures::{sp1_vkey_hash, trusted_state_bytes, wire_proof_bytes},
    ConsensusState, PROOF_TYPE_SP1, Sp1BeefyProof,
};

use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Signer, read_keypair_file},
    transaction::Transaction,
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
    } else {
        &trusted.current_authorities
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

fn main() {
    let program_id_str = env::var("PROGRAM_ID")
        .expect("set PROGRAM_ID env var (from `solana program deploy` output)");
    let program_id: Pubkey = program_id_str.parse().expect("valid program id");

    let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8899".to_string());
    let keypair_path = env::var("KEYPAIR").unwrap_or_else(|_| {
        let home = env::var("HOME").unwrap();
        format!("{}/.config/solana/id.json", home)
    });

    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());
    let payer = read_keypair_file(&keypair_path).expect("load keypair");

    let wire_bytes = wire_proof_bytes();
    let trusted_bytes = trusted_state_bytes();
    assert_eq!(wire_bytes[0], PROOF_TYPE_SP1);

    let trusted = ConsensusState::decode(&mut &trusted_bytes[..]).unwrap();
    let sp1_proof = Sp1BeefyProof::decode(&mut &wire_bytes[1..]).unwrap();

    let sp1_vkey_hash = sp1_vkey_hash();
    let public_inputs = build_public_inputs(&trusted, &sp1_proof);
    let proof_bytes = &sp1_proof.proof;

    let mut data = Vec::with_capacity(32 + 4 + proof_bytes.len() + public_inputs.len());
    data.extend_from_slice(&sp1_vkey_hash);
    data.extend_from_slice(&(proof_bytes.len() as u32).to_be_bytes());
    data.extend_from_slice(proof_bytes);
    data.extend_from_slice(&public_inputs);

    println!("program:           {}", program_id);
    println!("payer:             {}", payer.pubkey());
    println!("proof bytes:       {}", proof_bytes.len());
    println!("public inputs:     {}", public_inputs.len());
    println!("instruction data:  {}", data.len());

    let ix = Instruction {
        program_id,
        accounts: vec![AccountMeta::new_readonly(payer.pubkey(), true)],
        data,
    };

    let cu_budget = ComputeBudgetInstruction::set_compute_unit_limit(1_400_000);

    let blockhash = rpc.get_latest_blockhash().unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[cu_budget, ix],
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );

    println!("tx size:           {} bytes", bincode::serialize(&tx).unwrap().len());

    let sig = rpc.send_and_confirm_transaction(&tx).expect("send+confirm");
    println!("signature:         {}", sig);

    use solana_client::rpc_config::RpcTransactionConfig;
    use solana_sdk::commitment_config::CommitmentConfig as CC;
    let cfg = RpcTransactionConfig {
        encoding: Some(solana_transaction_status::UiTransactionEncoding::Json),
        commitment: Some(CC::confirmed()),
        max_supported_transaction_version: Some(0),
    };
    let parsed = rpc
        .get_transaction_with_config(&sig, cfg)
        .expect("fetch tx receipt");

    if let Some(meta) = parsed.transaction.meta {
        println!();
        if let solana_transaction_status::option_serializer::OptionSerializer::Some(logs) =
            &meta.log_messages
        {
            println!("logs:");
            for l in logs {
                println!("  {}", l);
            }
        }
        if let solana_transaction_status::option_serializer::OptionSerializer::Some(cu) =
            meta.compute_units_consumed
        {
            println!();
            println!("CONSUMED COMPUTE UNITS: {}", cu);
        }
    }
}
