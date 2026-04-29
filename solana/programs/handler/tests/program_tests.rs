//! Integration tests against the actual host + handler BPF binaries
//! via `solana-program-test`. Same code path as deployment.
//!
//! Prerequisite: `cargo build-sbf` must produce `target/deploy/host.so`
//! and `target/deploy/handler.so` before `cargo test` runs.
//!
//! Anchor 1.0 (solana-instruction 3.x) and solana-program-test 2.1.6
//! (solana-instruction 2.x) pin different solana sub-crate versions, so
//! `Pubkey` / `AccountMeta` / `Instruction` exist as parallel Rust
//! types. We build instructions in anchor's flavor (driven by
//! `host::accounts::*` + `host::instruction::*`) and convert to
//! solana-sdk's flavor at the BanksClient boundary.

use std::path::PathBuf;

use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use anchor_lang::solana_program::{
    instruction::AccountMeta as AMeta,
    pubkey::Pubkey as APubkey,
    system_program as a_system_program,
};
use parity_scale_codec::Decode;
use solana_program_test::{BanksClient, ProgramTest};
use solana_sdk::{
    instruction::{AccountMeta as SMeta, Instruction as SIx},
    pubkey::Pubkey as SPubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

use sp1_beefy_verifier::{
    fixtures::{sp1_vkey_hash, trusted_state_bytes, wire_proof_bytes},
    Sp1BeefyProof, PROOF_TYPE_SP1,
};

const BEFY: [u8; 4] = *b"BEFY";

// =========================================================================
// Cross-version Pubkey bridging
// =========================================================================

fn s(p: APubkey) -> SPubkey {
    SPubkey::new_from_array(p.to_bytes())
}

fn a(p: SPubkey) -> APubkey {
    APubkey::new_from_array(p.to_bytes())
}

fn to_sdk_ix<A: ToAccountMetas, D: InstructionData>(
    program_id: APubkey,
    accounts: A,
    data: D,
) -> SIx {
    let metas: Vec<SMeta> = accounts
        .to_account_metas(None)
        .into_iter()
        .map(|m: AMeta| SMeta {
            pubkey: s(m.pubkey),
            is_signer: m.is_signer,
            is_writable: m.is_writable,
        })
        .collect();
    SIx { program_id: s(program_id), accounts: metas, data: data.data() }
}

// =========================================================================
// PDA derivation (anchor-side Pubkey)
// =========================================================================

fn host_config_pda() -> APubkey {
    APubkey::find_program_address(&[host::state::HostConfig::SEED], &host::ID).0
}

fn consensus_state_pda(id: [u8; 4]) -> APubkey {
    APubkey::find_program_address(
        &[host::state::ConsensusState::SEED_PREFIX, &id],
        &host::ID,
    )
    .0
}

fn state_commitment_pda(state_machine: u32, height: u64) -> APubkey {
    APubkey::find_program_address(
        &[
            host::state::StateCommitment::SEED_PREFIX,
            &state_machine.to_le_bytes(),
            &height.to_le_bytes(),
        ],
        &host::ID,
    )
    .0
}

fn handler_state_pda() -> APubkey {
    APubkey::find_program_address(&[handler::state::HandlerState::SEED], &handler::ID).0
}

fn epoch_record_pda(authority_set_id: u64) -> APubkey {
    APubkey::find_program_address(
        &[
            handler::state::EpochRecord::SEED_PREFIX,
            &authority_set_id.to_le_bytes(),
        ],
        &handler::ID,
    )
    .0
}

fn handler_authority_pda() -> APubkey {
    APubkey::find_program_address(&[b"handler_authority"], &handler::ID).0
}

// =========================================================================
// Test bank setup
// =========================================================================

fn ensure_bpf_out_dir() {
    let solana_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("manifest dir resolves to solana/")
        .to_path_buf();
    std::env::set_var("BPF_OUT_DIR", solana_root.join("target/deploy"));
}

fn pt() -> ProgramTest {
    ensure_bpf_out_dir();
    let mut p = ProgramTest::default();
    p.add_program("host", s(host::ID), None);
    p.add_program("handler", s(handler::ID), None);
    p
}

async fn send(
    banks: &mut BanksClient,
    payer: &Keypair,
    blockhash: solana_sdk::hash::Hash,
    ix: SIx,
    extra: &[&Keypair],
) -> Result<(), solana_program_test::BanksClientError> {
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend(extra);
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &signers, blockhash);
    banks.process_transaction(tx).await
}

async fn read_account<T: AccountDeserialize>(
    banks: &mut BanksClient,
    addr: APubkey,
) -> Option<T> {
    let acct = banks.get_account(s(addr)).await.ok().flatten()?;
    T::try_deserialize(&mut &acct.data[..]).ok()
}

// =========================================================================
// Instruction builders
// =========================================================================

fn ix_initialize_host(payer: &Keypair, params: host::instructions::InitializeHostParams) -> SIx {
    to_sdk_ix(
        host::ID,
        host::accounts::InitializeHost {
            admin: a(payer.pubkey()),
            host_config: host_config_pda(),
            system_program: a_system_program::ID,
        },
        host::instruction::InitializeHost { params },
    )
}

fn ix_set_handler(admin: &Keypair, new_handler: APubkey) -> SIx {
    to_sdk_ix(
        host::ID,
        host::accounts::SetHandler {
            admin: a(admin.pubkey()),
            host_config: host_config_pda(),
        },
        host::instruction::SetHandler { new_handler },
    )
}

fn ix_set_frozen_state(admin: &Keypair, frozen: bool) -> SIx {
    to_sdk_ix(
        host::ID,
        host::accounts::SetFrozenState {
            admin: a(admin.pubkey()),
            host_config: host_config_pda(),
        },
        host::instruction::SetFrozenState { frozen },
    )
}

fn ix_set_consensus_state(admin: &Keypair, id: [u8; 4], state: Vec<u8>) -> SIx {
    to_sdk_ix(
        host::ID,
        host::accounts::SetConsensusState {
            admin: a(admin.pubkey()),
            host_config: host_config_pda(),
            consensus_state: consensus_state_pda(id),
            system_program: a_system_program::ID,
        },
        host::instruction::SetConsensusState {
            params: host::instructions::SetConsensusStateParams { id, state },
        },
    )
}

fn ix_initialize_handler(payer: &Keypair) -> SIx {
    to_sdk_ix(
        handler::ID,
        handler::accounts::InitializeHandler {
            payer: a(payer.pubkey()),
            handler_state: handler_state_pda(),
            system_program: a_system_program::ID,
        },
        handler::instruction::InitializeHandler {},
    )
}

fn ix_handle_consensus(
    relayer: &Keypair,
    state_commitment: APubkey,
    params: handler::instructions::HandleConsensusParams,
) -> SIx {
    let auth_set_id = params.authority_set_id;
    to_sdk_ix(
        handler::ID,
        handler::accounts::HandleConsensus {
            relayer: a(relayer.pubkey()),
            handler_state: handler_state_pda(),
            epoch_record: epoch_record_pda(auth_set_id),
            handler_authority: handler_authority_pda(),
            host_program: host::ID,
            host_config: host_config_pda(),
            consensus_state: consensus_state_pda(BEFY),
            state_commitment,
            system_program: a_system_program::ID,
        },
        handler::instruction::HandleConsensus { params },
    )
}

fn default_init_params() -> host::instructions::InitializeHostParams {
    host::instructions::InitializeHostParams {
        host_state_machine: u32::from_be_bytes(*b"sola"),
        hyperbridge_id: 4009,
        consensus_client_id: BEFY,
        challenge_period: 0,
        unbonding_period: 60 * 60 * 24 * 365 * 100, // ~100 years; fixture is from a stale block
        default_timeout: 3600,
        fee_token_mint: APubkey::default(),
        per_byte_fee: 100,
    }
}

fn fixture_first_header_meta() -> (u32, u64, u64) {
    let bytes = wire_proof_bytes();
    assert_eq!(bytes[0], PROOF_TYPE_SP1);
    let mut input = &bytes[1..];
    let proof = Sp1BeefyProof::decode(&mut input).unwrap();
    let header = &proof.headers[0];
    let (number, _) = handler::verifier::beefy::extract_header_prefix(&header.header).unwrap();
    (header.para_id, number as u64, proof.validator_set_id)
}

// =========================================================================
// Tests
// =========================================================================

#[tokio::test]
async fn initialize_host_creates_config() {
    let (mut banks, payer, blockhash) = pt().start().await;
    send(&mut banks, &payer, blockhash, ix_initialize_host(&payer, default_init_params()), &[])
        .await
        .unwrap();

    let cfg: host::state::HostConfig = read_account(&mut banks, host_config_pda())
        .await
        .expect("host_config should exist");
    assert_eq!(cfg.admin.to_bytes(), payer.pubkey().to_bytes());
    assert_eq!(cfg.consensus_client_id, BEFY);
    assert_eq!(cfg.handler_program, APubkey::default());
    assert!(!cfg.frozen);
}

#[tokio::test]
async fn set_handler_updates_host_config() {
    let (mut banks, payer, blockhash) = pt().start().await;
    send(&mut banks, &payer, blockhash, ix_initialize_host(&payer, default_init_params()), &[])
        .await
        .unwrap();
    send(&mut banks, &payer, blockhash, ix_set_handler(&payer, handler::ID), &[])
        .await
        .unwrap();

    let cfg: host::state::HostConfig = read_account(&mut banks, host_config_pda()).await.unwrap();
    assert_eq!(cfg.handler_program, handler::ID);
}

#[tokio::test]
async fn unauthorized_set_handler_rejects() {
    let (mut banks, payer, blockhash) = pt().start().await;
    send(&mut banks, &payer, blockhash, ix_initialize_host(&payer, default_init_params()), &[])
        .await
        .unwrap();

    let stranger = Keypair::new();
    let ix = to_sdk_ix(
        host::ID,
        host::accounts::SetHandler {
            admin: a(stranger.pubkey()),
            host_config: host_config_pda(),
        },
        host::instruction::SetHandler { new_handler: handler::ID },
    );
    let result = send(&mut banks, &payer, blockhash, ix, &[&stranger]).await;
    assert!(result.is_err(), "non-admin must be rejected");
}

// Blocked on a version-skew between Anchor 1.0 (solana-instruction 3.x)
// and solana-program-test 2.1.6 (solana-instruction 2.x): the older
// runtime silently rejects handler.so with `InvalidAccountData` before
// it ever invokes the entry function. Run with `--ignored` once the
// workspace can move to a newer solana-program-test that aligns with
// Anchor 1.0's modular crates.
#[tokio::test]
#[ignore]
async fn frozen_host_rejects_consensus_update() {
    let (mut banks, payer, blockhash) = pt().start().await;
    let trusted = trusted_state_bytes();
    let sp1_vkey = sp1_vkey_hash();
    let (para_id, height, set_id) = fixture_first_header_meta();

    for (label, ix) in [
        ("init_host", ix_initialize_host(&payer, default_init_params())),
        ("set_handler", ix_set_handler(&payer, handler::ID)),
        ("set_consensus_state", ix_set_consensus_state(&payer, BEFY, trusted)),
        ("init_handler", ix_initialize_handler(&payer)),
        ("set_frozen", ix_set_frozen_state(&payer, true)),
    ] {
        send(&mut banks, &payer, blockhash, ix, &[])
            .await
            .unwrap_or_else(|e| panic!("setup '{label}' failed: {e:?}"));
    }

    let params = handler::instructions::HandleConsensusParams {
        message: wire_proof_bytes(),
        sp1_vkey_hash: sp1_vkey,
        commit_header_index: 0,
        authority_set_id: set_id,
    };
    let result = send(
        &mut banks,
        &payer,
        blockhash,
        ix_handle_consensus(&payer, state_commitment_pda(para_id, height), params),
        &[],
    )
    .await;
    assert!(result.is_err(), "frozen host must reject handle_consensus");
}

// See note on `frozen_host_rejects_consensus_update`.
#[tokio::test]
#[ignore]
async fn handle_consensus_with_real_fixture_advances_state() {
    let (mut banks, payer, blockhash) = pt().start().await;
    let trusted = trusted_state_bytes();
    let sp1_vkey = sp1_vkey_hash();
    let (para_id, height, set_id) = fixture_first_header_meta();

    for (label, ix) in [
        ("init_host", ix_initialize_host(&payer, default_init_params())),
        ("set_handler", ix_set_handler(&payer, handler::ID)),
        ("set_consensus_state", ix_set_consensus_state(&payer, BEFY, trusted)),
        ("init_handler", ix_initialize_handler(&payer)),
    ] {
        send(&mut banks, &payer, blockhash, ix, &[])
            .await
            .unwrap_or_else(|e| panic!("setup '{label}' failed: {e:?}"));
    }

    let params = handler::instructions::HandleConsensusParams {
        message: wire_proof_bytes(),
        sp1_vkey_hash: sp1_vkey,
        commit_header_index: 0,
        authority_set_id: set_id,
    };
    let sc_pda = state_commitment_pda(para_id, height);
    send(
        &mut banks,
        &payer,
        blockhash,
        ix_handle_consensus(&payer, sc_pda, params),
        &[],
    )
    .await
    .expect("handle_consensus should succeed against real fixture");

    let sc: host::state::StateCommitment = read_account(&mut banks, sc_pda)
        .await
        .expect("StateCommitment PDA should be initialized");
    assert_eq!(sc.state_machine, para_id);
    assert_eq!(sc.height, height);
    assert!(!sc.vetoed);

    let er: handler::state::EpochRecord = read_account(&mut banks, epoch_record_pda(set_id))
        .await
        .expect("EpochRecord should be initialized on epoch advance");
    assert_eq!(er.authority_set_id, set_id);
    assert_eq!(er.relayer.to_bytes(), payer.pubkey().to_bytes());

    let hs: handler::state::HandlerState =
        read_account(&mut banks, handler_state_pda()).await.unwrap();
    assert_eq!(hs.current_epoch, set_id);
}
