//! Simnode regression test for the pool-aware nonce used by the BEEFY prover's on-chain
//! proof backend (`tesseract/consensus/beefy/src/backend/onchain.rs`).
//!
//! The prover submits proofs but only waits for **in-block** (not finalization) before its
//! next submission, for speed. subxt sources its automatic nonce from the latest **finalized**
//! block; on the parachain, finality trails the best chain by several blocks, so the auto nonce
//! is frequently already spent by the previous proof and the node rejects the resubmission with
//! `Invalid Transaction (1010)` (`InvalidTransaction::Stale` in the node's txpool). The fix
//! fetches a pool-aware nonce via `system_accountNextIndex` and submits it explicitly through
//! `subxt_utils::send_extrinsic_with_nonce`, which uses `create_partial_offline` so subxt does
//! not overwrite the nonce from the finalized view.
//!
//! This test reproduces the exact condition deterministically on a manual-seal simnode: it
//! lands an extrinsic in a best-but-unfinalized block, shows the finalized-sourced nonce is now
//! stale while the pool-aware nonce leads it, confirms that submitting with the stale nonce is
//! rejected (the production bug), and that submitting with the pool-aware nonce lands cleanly
//! (the fix). Requires a running gargantua simnode; run with `--ignored`.

#![cfg(test)]

use std::{env, time::Duration};

use anyhow::{anyhow, Context};
use polkadot_sdk::*;
use primitive_types::H256;
use sc_consensus_manual_seal::CreatedBlock;
use sp_core::{crypto::Ss58Codec, Bytes};
use sp_keyring::sr25519::Keyring;
use subxt::{
	dynamic::Value,
	ext::subxt_rpcs::{rpc_params, RpcClient},
	OnlineClient,
};
use subxt_utils::{send_extrinsic_with_nonce, Hyperbridge, InMemorySigner};

/// Create a block on the manual-seal simnode. `finalize = false` advances the best chain
/// without moving the finalized head — precisely the state that used to make the prover
/// reuse a stale nonce.
async fn create_block(rpc: &RpcClient, finalize: bool) -> Result<H256, anyhow::Error> {
	let block: CreatedBlock<H256> = rpc
		.request("engine_createBlock", rpc_params![true, finalize])
		.await
		.map_err(|e| anyhow!("engine_createBlock failed: {e:?}"))?;
	Ok(block.hash)
}

/// Block until the submitted extrinsic is sitting in the node's pool, so the block we seal
/// actually contains it rather than racing an empty block in ahead of submission.
async fn wait_until_pending(rpc: &RpcClient) -> Result<(), anyhow::Error> {
	for _ in 0..200 {
		let pending: Vec<Bytes> =
			rpc.request("author_pendingExtrinsics", rpc_params![]).await.unwrap_or_default();
		if !pending.is_empty() {
			return Ok(());
		}
		tokio::time::sleep(Duration::from_millis(25)).await;
	}
	Err(anyhow!("extrinsic never entered the pool"))
}

/// Submit `payload` signed by `signer` with an explicit `nonce`, sealing one non-finalized
/// block so the manual-seal node includes it. `send_extrinsic_with_nonce` waits for in-block,
/// so the seal has to happen concurrently — we drive both on the same task with `join!`.
async fn submit_with_nonce_sealing(
	client: &OnlineClient<Hyperbridge>,
	rpc: &RpcClient,
	signer: &InMemorySigner<Hyperbridge>,
	payload: &subxt::tx::DynamicPayload,
	nonce: u64,
) -> Result<H256, anyhow::Error> {
	let submit_fut = send_extrinsic_with_nonce(client, signer, payload, nonce, false);
	let seal_fut = async {
		wait_until_pending(rpc).await?;
		create_block(rpc, false).await
	};
	let (submit_res, seal_res) = tokio::join!(submit_fut, seal_fut);
	let block = seal_res?;
	submit_res?;
	Ok(block)
}

/// A distinct `System::remark` call per submission, so successive extrinsics hash differently.
fn remark(tag: &[u8]) -> subxt::tx::DynamicPayload {
	subxt::dynamic::tx("System", "remark", vec![Value::from_bytes(tag)])
}

#[tokio::test]
#[ignore]
async fn pool_aware_nonce_survives_finalization_lag() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or_else(|_| "9990".into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;

	// Bob is endowed in the gargantua dev genesis, so a client-side signed extrinsic is valid
	// and funded. Signing client-side (rather than via `simnode_authorExtrinsic`) is the point:
	// it exercises the prover's own nonce sourcing.
	let signer = InMemorySigner::<Hyperbridge>::new(Keyring::Bob.pair());
	let bob_ss58 = Keyring::Bob.to_account_id().to_ss58check();

	// Baseline next nonce. Prior tiers finalize their blocks, so best == finalized here and the
	// pool-aware nonce equals the finalized nonce at the start.
	let next0: u64 = rpc_client
		.request("system_accountNextIndex", rpc_params![bob_ss58.clone()])
		.await
		.map_err(|e| anyhow!("system_accountNextIndex failed: {e:?}"))?;

	// 1. Land a first extrinsic in a *best but unfinalized* block — the exact condition the prover
	//    hits: the proof is in-block, but finality (subxt's auto-nonce source) lags.
	submit_with_nonce_sealing(&client, &rpc_client, &signer, &remark(b"nonce-test-1"), next0)
		.await
		.context("first submission should land in a block")?;

	// 2. Finality has not advanced, so subxt's finalized-sourced nonce is stale while the
	//    pool-aware nonce reflects the best chain.
	let finalized_nonce = client.tx().account_nonce(&signer.account_id).await?;
	let pool_nonce: u64 = rpc_client
		.request("system_accountNextIndex", rpc_params![bob_ss58.clone()])
		.await
		.map_err(|e| anyhow!("system_accountNextIndex failed: {e:?}"))?;
	assert_eq!(finalized_nonce, next0, "finalized nonce must not advance without finalization");
	assert_eq!(pool_nonce, next0 + 1, "pool-aware nonce must reflect the unfinalized best block");

	// 3. Bug reproduction: submitting with the stale finalized nonce — what subxt's `create_signed`
	//    auto path would pick — is rejected by the node (surfaces to the prover as the production
	//    `Invalid Transaction (1010)`). It fails at submission, so no block needs sealing.
	let stale_res = send_extrinsic_with_nonce(
		&client,
		&signer,
		&remark(b"nonce-test-stale"),
		finalized_nonce,
		false,
	)
	.await;
	assert!(
		stale_res.is_err(),
		"submitting with the stale finalized nonce ({finalized_nonce}) must be rejected, got {stale_res:?}",
	);

	// 4. The fix: submitting with the pool-aware nonce lands cleanly despite the finality lag.
	let last_block = submit_with_nonce_sealing(
		&client,
		&rpc_client,
		&signer,
		&remark(b"nonce-test-2"),
		pool_nonce,
	)
	.await
	.context("submission with the pool-aware nonce should succeed despite finality lag")?;

	// Finalize the last sealed block (and its ancestors) so we don't leave `best > finalized`
	// for subsequent `--test-threads=1` simnode tests.
	let finalized: bool = rpc_client
		.request("engine_finalizeBlock", rpc_params![last_block])
		.await
		.map_err(|e| anyhow!("engine_finalizeBlock failed: {e:?}"))?;
	assert!(finalized, "finalizing the last sealed block should succeed");

	Ok(())
}
