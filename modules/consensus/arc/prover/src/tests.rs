// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Live integration tests against the Arc testnet.
//!
//! Configuration:
//! - `ARC_RPC_URL`: execution JSON-RPC for headers/proofs/storage (defaults to the public Arc
//!   testnet RPC). Third-party endpoints (e.g. Alchemy) work.
//! - `ARC_CERT_RPC_URL`: endpoint serving `arc_getCertificate` (defaults to the public Arc testnet
//!   RPC, which proxies the consensus node).

use crate::{header_hash, ArcProver, Keccak256Hasher};
use arc_verifier::verify_arc_update;
use std::time::Duration;

/// Public Arc testnet JSON-RPC endpoint (serves `arc_getCertificate`).
const DEFAULT_ARC_TESTNET_RPC: &str = "https://rpc.testnet.arc.network";

/// How many consecutive updates to follow.
const UPDATES_TO_FOLLOW: u64 = 5;

fn prover() -> Result<ArcProver, anyhow::Error> {
	let primary =
		std::env::var("ARC_RPC_URL").unwrap_or_else(|_| DEFAULT_ARC_TESTNET_RPC.to_string());
	let certificates =
		std::env::var("ARC_CERT_RPC_URL").unwrap_or_else(|_| DEFAULT_ARC_TESTNET_RPC.to_string());
	Ok(ArcProver::with_certificate_endpoint(primary, certificates)?)
}

fn init_tracing() {
	let _ = tracing_subscriber::fmt()
		.with_env_filter(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.unwrap_or_else(|_| "arc_prover=trace,info".into()),
		)
		.try_init();
}

/// Follows Arc testnet consensus, mirroring `test_beefy_consensus_client`:
/// bootstrap a trusted state from a storage proof, then repeatedly fetch
/// finalized updates (commit certificate + header + validator set proof) and
/// run them through the exact verification the on-chain client performs.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires network access to the Arc testnet — set ARC_RPC_URL / ARC_CERT_RPC_URL to override the default RPC and run with --ignored"]
async fn test_arc_consensus_client() -> Result<(), anyhow::Error> {
	init_tracing();

	let prover = prover()?;

	let mut trusted = prover.fetch_latest_verifier_state().await?;
	tracing::info!(
		"bootstrapped at height {} with {} validators (total power {})",
		trusted.finalized_height,
		trusted.current_validators.len(),
		trusted.current_validators.total_voting_power,
	);
	assert!(!trusted.current_validators.is_empty());

	let mut verified = 0u64;
	let mut attempts = 0u64;
	while verified < UPDATES_TO_FOLLOW {
		attempts += 1;
		anyhow::ensure!(attempts <= 5 * UPDATES_TO_FOLLOW, "too many failed update attempts");

		let update = match prover.fetch_latest_update().await {
			Ok(update) => update,
			Err(e) => {
				tracing::warn!("transient update fetch failure: {e}");
				tokio::time::sleep(Duration::from_secs(1)).await;
				continue;
			},
		};
		let target = update.certificate.height;
		if target <= trusted.finalized_height {
			tokio::time::sleep(Duration::from_secs(1)).await;
			continue;
		}

		assert_eq!(update.certificate.block_hash, header_hash(&update.header));

		let new_state = verify_arc_update::<Keccak256Hasher>(trusted.clone(), update.clone())?;
		assert_eq!(new_state.finalized_height, target);
		assert_eq!(new_state.finalized_hash, update.certificate.block_hash);
		assert!(!new_state.current_validators.is_empty());

		tracing::info!(
			"verified certificate for height {} ({} signatures, {} validators)",
			target,
			update.certificate.commit_signatures.len(),
			new_state.current_validators.len(),
		);

		trusted = new_state;
		verified += 1;
	}

	Ok(())
}

/// The verifier must reject a certificate whose signatures were produced for a
/// different block hash.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires network access to the Arc testnet — set ARC_RPC_URL / ARC_CERT_RPC_URL to override the default RPC and run with --ignored"]
async fn test_arc_rejects_tampered_certificate() -> Result<(), anyhow::Error> {
	init_tracing();

	let prover = prover()?;

	let trusted = prover.fetch_latest_verifier_state().await?;

	// Grab an update at least two blocks past the trusted height so the
	// tampered certificate (re-pointed at the update's parent) still advances
	// the trusted state.
	let update = loop {
		let update = prover.fetch_latest_update().await?;
		if update.certificate.height > trusted.finalized_height + 1 {
			break update;
		}
		tokio::time::sleep(Duration::from_secs(1)).await;
	};
	let target = update.certificate.height;

	// Re-point the certificate at the parent block, leaving the signatures
	// over the original block hash. The height and header-binding checks all
	// pass, so verification must fail on the signatures themselves.
	let mut update = update;
	let parent = prover.rpc.get_block_by_number(target - 1).await?;
	update.certificate.height = target - 1;
	update.certificate.block_hash = header_hash(&parent);
	update.header = parent;

	let result = verify_arc_update::<Keccak256Hasher>(trusted, update);
	assert!(
		matches!(result, Err(arc_verifier::error::Error::InvalidSignature { .. })),
		"tampered certificate must fail signature verification, got {result:?}"
	);

	Ok(())
}
