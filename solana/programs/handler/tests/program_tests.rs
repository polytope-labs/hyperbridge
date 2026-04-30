//! BPF smoke tests for the host program. Requires `cargo build-sbf`
//! to have produced `target/deploy/host.so`.

use std::path::PathBuf;

use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use anchor_lang::solana_program::{
    instruction::AccountMeta as AMeta,
    pubkey::Pubkey as APubkey,
    system_program as a_system_program,
};
use solana_program_test::{BanksClient, ProgramTest};
use solana_sdk::{
    instruction::{AccountMeta as SMeta, Instruction as SIx},
    pubkey::Pubkey as SPubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

const BEFY: [u8; 4] = *b"BEFY";

// Anchor 1.0 and solana-program-test 2.1.6 pin different solana
// sub-crates, so `Pubkey` / `AccountMeta` / `Instruction` exist as
// parallel types. We build via Anchor's flavour and convert at the
// `BanksClient` boundary.

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

fn host_config_pda() -> APubkey {
    APubkey::find_program_address(&[host::state::HostConfig::SEED], &host::ID).0
}

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

fn default_init_params() -> host::instructions::InitializeHostParams {
    host::instructions::InitializeHostParams {
        host_state_machine: u32::from_be_bytes(*b"sola"),
        hyperbridge_id: 4009,
        consensus_client_id: BEFY,
        challenge_period: 0,
        unbonding_period: 60 * 60 * 24 * 365 * 100,
        default_timeout: 3600,
        fee_token_mint: APubkey::default(),
        per_byte_fee: 100,
    }
}

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
