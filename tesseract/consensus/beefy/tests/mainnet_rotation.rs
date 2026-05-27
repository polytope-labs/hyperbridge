// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0
//
//! Live mainnet relayer test: generate naive (ECDSA) BEEFY consensus proofs that rotate the
//! authority set and submit them to the deployed `HandlerV2.handleConsensus` on all 9 mainnet
//! chains, asserting each succeeds before moving on.
//!
//! All chains were initialized with the same BEEFY consensus state, so the loop reads the
//! genesis state from one chain, then walks Polkadot's authority-set handovers (exactly like
//! `BeefyProver::run`: `query_next_finalized_epoch` -> `epoch_justification_for`), builds the
//! rotation proof, and fans the same wire bytes out to every chain. The on-chain Handler reads
//! each host's own stored state, so identical genesis + identical proof keeps all 9 in lockstep.
//!
//! Config comes from `evm/.env.mainnet` (`PRIVATE_KEY`, per-chain `*_RPC_URL`, `PARA_ID`,
//! and `POLKADOT_URL` = the EVM RPC for chain 420420419 / Polkadot Hub). The Polkadot **relay**
//! and Nexus **parachain** substrate endpoints come from `RELAY_WS_URL` / `PARA_WS_URL`.
//!
//! Marked `#[ignore]` — it sends real mainnet transactions (real gas). Run with:
//!   cargo test -p tesseract-beefy --test mainnet_rotation -- --ignored --nocapture

use std::{collections::BTreeSet, sync::Arc, time::Duration};

use alloy::{
	eips::eip2718::Encodable2718,
	network::{EthereumWallet, TransactionBuilder},
	primitives::{address, Address, Bytes},
	providers::{DynProvider, Provider, ProviderBuilder},
	rpc::types::{TransactionReceipt, TransactionRequest},
	signers::local::PrivateKeySigner,
};
use alloy_sol_types::SolValue;
use anyhow::{anyhow, Context};
use beefy_prover::relay::fetch_latest_beefy_justification;
use ismp_abi::{ecdsa_beefy::BeefyConsensusState, evm_host::EvmHost, handler::handler_v2::HandlerV2};
use sp_consensus_beefy::{ecdsa_crypto::Signature, SignedCommitment};
use subxt::{backend::legacy::LegacyRpcMethods, config::Header as _};
use tesseract_beefy::{
	backend::{InMemoryProofBackend, ProofBackend},
	prover::{BeefyProver, BeefyProverConfig, Prover, ProverConfig, ProverConsensusState, ProofVariant},
	ConsensusState,
};
use tesseract_substrate::{
	config::{Blake2SubstrateChain, KeccakSubstrateChain},
	SubstrateClient, SubstrateConfig,
};

/// Shared CREATE2 addresses across every mainnet deployment (config.mainnet.toml).
const HOST: Address = address!("620128E2B19193d6Bd244a3AC8D3bBa0541B19c3");
const HANDLER: Address = address!("2a18AB35DEa43474882E05A661e2F20fe89c0535");

/// Arbitrum rejects the ~96 KB naive proof at the public RPC ("oversized data"); pushing the
/// signed tx straight to the sequencer bypasses that node-level txpool size check.
const ARBITRUM_SEQUENCER: &str = "https://arb1-sequencer.arbitrum.io/rpc";

/// The 9 mainnet chains: (chain id, label, env var holding the EVM RPC url, optional submit
/// endpoint). When the submit endpoint is set, the tx is filled+signed against the RPC and the
/// raw signed tx is sent there instead. Chain 420420419 (Polkadot Hub) uses `POLKADOT_URL`.
const CHAINS: &[(u64, &str, &str, Option<&str>)] = &[
	(1, "ethereum", "ETHEREUM_RPC_URL", None),
	(42161, "arbitrum", "ARBITRUM_RPC_URL", Some(ARBITRUM_SEQUENCER)),
	(10, "optimism", "OPTIMISM_RPC_URL", None),
	(8453, "base", "BASE_RPC_URL", None),
	(56, "bsc", "BSC_RPC_URL", None),
	(100, "gnosis", "GNOSIS_RPC_URL", None),
	(1868, "soneium", "SONEIUM_RPC_URL", None),
	(137, "polygon", "POLYGON_RPC_URL", None),
	(420420419, "polkadot-hub", "POLKADOT_URL", None),
];

fn env_or(key: &str, default: &str) -> String {
	std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Read a host's stored BEEFY consensus state and decode it to the prover's `ConsensusState`.
async fn read_consensus_state(provider: &DynProvider) -> anyhow::Result<ConsensusState> {
	let host = EvmHost::new(HOST, provider.clone());
	let bytes = host.consensusState().call().await.context("host.consensusState()")?;
	let sol = BeefyConsensusState::abi_decode(&bytes).context("decode BeefyConsensusState")?;
	Ok(sol.into())
}

/// Fill + sign a `handleConsensus` tx against `read`, push the raw signed tx to `sequencer`
/// (which doesn't enforce the public node's oversized-tx limit), then poll `read` for the
/// receipt (sequencer endpoints don't serve reads).
async fn submit_via_sequencer(
	read: &DynProvider,
	sequencer: &DynProvider,
	wallet: &EthereumWallet,
	from: Address,
	proof: Bytes,
) -> anyhow::Result<TransactionReceipt> {
	// `DynProvider` erases the filler layer, so populate the tx fields explicitly against the
	// read RPC, sign locally, and push the raw signed tx to the sequencer.
	let calldata =
		HandlerV2::new(HANDLER, read.clone()).handleConsensus(HOST, proof).calldata().clone();
	let base =
		TransactionRequest::default().with_from(from).with_to(HANDLER).with_input(calldata);
	let chain_id = read.get_chain_id().await.context("get_chain_id")?;
	let nonce = read.get_transaction_count(from).await.context("get_transaction_count")?;
	let gas = read.estimate_gas(base.clone()).await.context("estimate_gas")?;
	let fees = read.estimate_eip1559_fees().await.context("estimate_eip1559_fees")?;
	let request = base
		.with_chain_id(chain_id)
		.with_nonce(nonce)
		.with_gas_limit(gas)
		.with_max_fee_per_gas(fees.max_fee_per_gas)
		.with_max_priority_fee_per_gas(fees.max_priority_fee_per_gas);
	let envelope = request.build(wallet).await?;
	let raw = envelope.encoded_2718();
	let pending = sequencer
		.send_raw_transaction(&raw)
		.await
		.context("sequencer eth_sendRawTransaction")?;
	let hash = *pending.tx_hash();
	for _ in 0..90 {
		if let Some(receipt) = read.get_transaction_receipt(hash).await? {
			return Ok(receipt);
		}
		tokio::time::sleep(Duration::from_secs(2)).await;
	}
	Err(anyhow!("timed out waiting for receipt of {hash:?} after sequencer submit"))
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "sends real mainnet transactions; run with --ignored and a funded PRIVATE_KEY"]
async fn rotate_authorities_across_all_chains() -> anyhow::Result<()> {
	// Load EVM-submission config from evm/.env.mainnet (PRIVATE_KEY, *_RPC_URL, PARA_ID,
	// POLKADOT_URL). Relay/parachain substrate endpoints come from RELAY_WS_URL/PARA_WS_URL.
	let env_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../evm/.env.mainnet");
	let _ = dotenv::from_path(env_path);

	let relay_url = env_or("RELAY_WS_URL", "");
	let para_url = env_or("PARA_WS_URL", "");
	let para_id: u32 = env_or("PARA_ID", "3367").parse().context("PARA_ID")?;

	// --- EVM side: one signer (relayer), one provider per chain. --------------------------------
	let pk = std::env::var("PRIVATE_KEY").context("PRIVATE_KEY missing from env/.env.mainnet")?;
	let signer: PrivateKeySigner =
		pk.trim_start_matches("0x").parse().context("invalid PRIVATE_KEY")?;
	let from = signer.address();
	let wallet = EthereumWallet::from(signer);

	let mut chains: Vec<(String, DynProvider, Option<DynProvider>)> = Vec::with_capacity(CHAINS.len());
	println!("EVM RPC endpoints:");
	for (id, name, rpc_env, submit_url) in CHAINS {
		let url = std::env::var(rpc_env)
			.with_context(|| format!("missing {rpc_env} for chain {id} ({name})"))?;
		println!("  {name} (chain {id}) <- ${rpc_env} = {url}");
		let read = ProviderBuilder::new()
			.wallet(wallet.clone())
			.connect(&url)
			.await
			.with_context(|| format!("connect {name} @ {url}"))?
			.erased();
		let submit = match submit_url {
			Some(s) => {
				println!("    {name} raw-tx submit -> {s}");
				Some(
					ProviderBuilder::new()
						.connect(s)
						.await
						.with_context(|| format!("connect {name} sequencer @ {s}"))?
						.erased(),
				)
			},
			None => None,
		};
		chains.push((name.to_string(), read, submit));
	}

	// --- Starting anchor: the most-behind chain's consensus state. -----------------------------
	// Chains can diverge (a prior run/relayer advanced some and not others), so the loop always
	// drives from the laggard. Read every chain and log THAT anchor, not whichever is first.
	let start_anchor = {
		let mut best: Option<(String, ConsensusState)> = None;
		for (name, read, _submit) in &chains {
			let s = read_consensus_state(read)
				.await
				.with_context(|| format!("consensusState() on {name}"))?;
			println!(
				"  {name}: height={} current_set={} next_set={}",
				s.latest_beefy_height, s.current_authorities.id, s.next_authorities.id,
			);
			if best.as_ref().map_or(true, |(_, b)| s.latest_beefy_height < b.latest_beefy_height) {
				best = Some((name.clone(), s));
			}
		}
		best.expect("at least one chain")
	};
	println!(
		"starting anchor: most-behind chain {} @ height={} current_set={} next_set={}",
		start_anchor.0,
		start_anchor.1.latest_beefy_height,
		start_anchor.1.current_authorities.id,
		start_anchor.1.next_authorities.id,
	);
	let genesis = start_anchor.1;

	// --- Relay/prover side: build a BeefyProver (Ecdsa) over an in-memory backend. --------------
	let prover_config = ProverConfig {
		relay_rpc_ws: relay_url.clone(),
		para_rpc_ws: para_url.clone(),
		para_ids: vec![para_id],
		proof_variant: ProofVariant::Ecdsa,
		max_rpc_payload_size: None,
		query_batch_size: None,
	};
	// ECDSA proof submitted to the EVM handler, which does not enforce the SP1 committed-
	// nonce binding, so a zero account is fine here.
	let prover: Prover<Blake2SubstrateChain, KeccakSubstrateChain, zk_beefy::LocalProver> =
		Prover::new(prover_config, Default::default()).await?;

	let substrate = SubstrateClient::<KeccakSubstrateChain>::new(
		SubstrateConfig {
			state_machine: None,
			hashing: None,
			consensus_state_id: None,
			rpc_ws: para_url.clone(),
			max_rpc_payload_size: None,
			signer: None,
			initial_height: None,
			max_concurent_queries: None,
			poll_interval: None,
			fee_token_decimals: None,
		}
		.resolve()
		.await?,
	)
	.await?;

	// Seed the in-memory backend with the genesis state read from the chains.
	let backend: Arc<dyn ProofBackend> = Arc::new(InMemoryProofBackend::new(ProverConsensusState {
		inner: genesis.clone(),
		finalized_parachain_height: 0,
	}));

	let beefy_config = BeefyProverConfig {
		consensus_state_id: *b"DOT0",
		minimum_finalization_height: 0,
		state_machines: vec![],
		backend: Default::default(),
	};

	let beefy = BeefyProver::<Blake2SubstrateChain, KeccakSubstrateChain, zk_beefy::LocalProver, dyn ProofBackend>::new(
		beefy_config,
		substrate,
		prover,
		backend,
	)
	.await?;

	// A second relay connection for the auxiliary queries the helpers don't expose (resolving
	// the epoch-change block number, the live finalized head).
	let (_relay, relay_rpc_client) =
		subxt_utils::client::ws_client::<Blake2SubstrateChain>(&relay_url, u32::MAX).await?;
	let relay_rpc = LegacyRpcMethods::<Blake2SubstrateChain>::new(relay_rpc_client.clone());

	// --- The catch-up loop: one rotation proof per authority-set handover. ----------------------
	let _ = genesis; // genesis was only needed to seed the backend; per-iteration we read live state.

	// Chains that can't accept a naive proof (e.g. Arbitrum rejects the ~96 KB calldata as
	// "oversized data") are recorded here and excluded from *both* submission and the anchor —
	// otherwise a permanently-failing chain stays the most-behind chain forever and stalls the
	// rest. Submission errors never abort the run; the chain is skipped and we keep going.
	let mut failed: BTreeSet<String> = BTreeSet::new();
	let mut rotations = 0u32;
	loop {
		// Look up the consensus state on every chain up front. They may diverge (an earlier
		// run/relayer can advance some chains and not others), so anchor proof generation to the
		// most-behind *active* chain — that produces a proof whose validator set the laggards
		// still accept, while any chain already at/past the target height is skipped below.
		let mut states: Vec<(String, DynProvider, Option<DynProvider>, ConsensusState)> =
			Vec::with_capacity(chains.len());
		for (name, read, submit) in &chains {
			let s = read_consensus_state(read)
				.await
				.with_context(|| format!("consensusState() on {name}"))?;
			states.push((name.clone(), read.clone(), submit.clone(), s));
		}

		let Some(anchor) = states
			.iter()
			.filter(|(name, _, _, _)| !failed.contains(name))
			.min_by_key(|(_, _, _, s)| s.latest_beefy_height)
			.map(|(_, _, _, s)| s.clone())
		else {
			println!("\nNo active chains left to advance (all skipped).");
			break;
		};
		println!(
			"\n--- anchor (most-behind active chain): height={} current_set={} next_set={} ---",
			anchor.latest_beefy_height, anchor.current_authorities.id, anchor.next_authorities.id,
		);

		let pcs = ProverConsensusState { inner: anchor.clone(), finalized_parachain_height: 0 };
		let (update, live_header) = beefy.query_next_finalized_epoch(&pcs).await?;

		let commitment: SignedCommitment<u32, Signature> = match update {
			Some((epoch_hash, next_set_id)) => {
				assert_eq!(
					next_set_id, anchor.next_authorities.id,
					"next epoch must be the anchor's next set",
				);
				let epoch_header = relay_rpc
					.chain_get_header(Some(epoch_hash))
					.await?
					.ok_or_else(|| anyhow!("epoch-change header missing"))?;
				beefy
					.epoch_justification_for(epoch_header.number().into())
					.await?
					.ok_or_else(|| anyhow!("no BEEFY justification found for epoch {next_set_id}"))?
			},
			None => {
				// Sets are caught up. Do a final height advance to the live head, then stop.
				if live_header.number <= anchor.latest_beefy_height {
					println!("\nActive chains caught up at height {} set {}", anchor.latest_beefy_height, anchor.current_authorities.id);
					break;
				}
				let head = live_header.hash();
				let (sc, _) = fetch_latest_beefy_justification(&relay_rpc, head.into()).await?;
				sc
			},
		};

		let block = commitment.commitment.block_number;
		let set_id = commitment.commitment.validator_set_id;
		println!("=== proof @ block={block} set_id={set_id} ===");

		// Reuse the daemon's encoder: returns `0x00 ++ abi_encode_params(BeefyConsensusProof)`.
		let wire = beefy.consensus_proof(commitment.clone(), anchor.clone()).await?;
		let proof = Bytes::from(wire);

		let failed_before = failed.len();
		let mut advanced_any = false;
		for (name, read, submit, before) in &states {
			if failed.contains(name) {
				continue;
			}
			if before.latest_beefy_height >= block {
				println!("  - {name}: already at height {} (skip)", before.latest_beefy_height);
				continue;
			}
			// Submission errors (oversized tx, RPC rejection) and reverts are non-fatal: log,
			// mark the chain skipped, and continue with the rest. Chains with a dedicated submit
			// endpoint (Arbitrum sequencer) take the raw-tx path; the rest use a normal send.
			let result: anyhow::Result<TransactionReceipt> = match submit {
				Some(sequencer) => submit_via_sequencer(read, sequencer, &wallet, from, proof.clone()).await,
				None => {
					let handler = HandlerV2::new(HANDLER, read.clone());
					async {
						Ok(handler.handleConsensus(HOST, proof.clone()).send().await?.get_receipt().await?)
					}
					.await
				},
			};
			match result {
				Ok(receipt) if receipt.status() => {
					let after = read_consensus_state(read).await?;
					println!(
						"  ✓ {name}: height {} -> {} | set {} -> {} | tx {:?}",
						before.latest_beefy_height,
						after.latest_beefy_height,
						before.current_authorities.id,
						after.current_authorities.id,
						receipt.transaction_hash,
					);
					advanced_any = true;
				},
				Ok(receipt) => {
					println!("  ✗ {name}: reverted (tx {:?}) — skipping this chain.", receipt.transaction_hash);
					failed.insert(name.clone());
				},
				Err(e) => {
					println!("  ✗ {name}: submission failed — skipping this chain. {e:#}");
					failed.insert(name.clone());
				},
			}
		}

		// Progress guard: stop only if this round neither advanced a chain nor newly skipped one
		// (i.e. nothing changed and we'd loop forever). A fresh skip moves the anchor next round.
		if !advanced_any && failed.len() == failed_before {
			println!("\nNo progress this round; stopping.");
			break;
		}
		if advanced_any {
			rotations += 1;
		}
	}

	let advanced: Vec<&str> =
		chains.iter().map(|(n, _, _)| n.as_str()).filter(|n| !failed.contains(*n)).collect();
	println!("\nDone — {rotations} rotation(s).");
	println!("  advanced: {advanced:?}");
	if !failed.is_empty() {
		println!("  skipped (could not submit): {:?}", failed);
	}
	Ok(())
}
