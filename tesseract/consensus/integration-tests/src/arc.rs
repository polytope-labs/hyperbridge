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

//! Relayer level tests for the Arc consensus host.
//!
//! `arc_prover`'s own tests already cover the prover and verifier against the
//! live testnet. What these cover is the tesseract host on top of them: the
//! initial consensus state the relayer hands to `ismp.createConsensusState`,
//! and the update loop that keeps it moving.
//!
//! Configuration:
//! - `ARC_RPC_URL`: execution JSON-RPC for headers, proofs and storage. Defaults to the public drpc
//!   endpoint, since the official rpc.testnet.arc.network doesn't serve `eth_getProof`.
//! - `ARC_CERT_RPC_URL`: endpoint serving `arc_getCertificate`, when the execution RPC doesn't
//!   proxy Arc's custom namespace.
//! - `ARC_ISMP_HOST`: the `EvmHost` address on Arc. Only the messaging paths read it, so the
//!   consensus tests here run fine against the zero address until one is deployed.
//! - `HYPERBRIDGE_WS`: the hyperbridge node `arc_consensus_updates` relays into.

use std::{
	sync::Arc,
	time::{Duration, Instant},
};

use arc_primitives::{ARC_CONSENSUS_ID, ARC_TESTNET_CHAIN_ID};
use codec::Decode;
use ismp::host::StateMachine;
use ismp_arc::{ConsensusState, ARC_CONSENSUS_CLIENT_ID};
use sp_core::{H160, H256};
use substrate_state_machine::HashAlgorithm;
use subxt_utils::Hyperbridge;
use tesseract_arc::{ArcHost, ArcHostConfig};
use tesseract_evm::EvmConfig;
use tesseract_primitives::{IsmpHost, IsmpProvider};
use tesseract_substrate::{SubstrateClient, SubstrateConfig};

use crate::util::setup_logging;

/// Public Arc testnet JSON-RPC endpoint serving both `eth_getProof` (at
/// `"latest"` only) and `arc_getCertificate`.
const DEFAULT_ARC_TESTNET_RPC: &str = "https://arc-testnet.drpc.org";

/// Read an endpoint override, treating an unset or empty variable as absent so
/// that CI passing through an unconfigured secret still falls back to the
/// public endpoint.
fn endpoint(var: &str, default: &str) -> String {
	std::env::var(var)
		.ok()
		.filter(|url| !url.trim().is_empty())
		.unwrap_or_else(|| default.to_string())
}

/// The `EvmHost` address to run against. Zero until one is deployed on Arc.
fn ismp_host() -> Result<H160, anyhow::Error> {
	match std::env::var("ARC_ISMP_HOST").ok().filter(|addr| !addr.trim().is_empty()) {
		Some(addr) => Ok(addr.trim().trim_start_matches("0x").parse()?),
		None => Ok(H160::zero()),
	}
}

async fn arc_host() -> Result<ArcHost, anyhow::Error> {
	let rpc_url = endpoint("ARC_RPC_URL", DEFAULT_ARC_TESTNET_RPC);

	let evm_config = EvmConfig {
		rpc_urls: vec![rpc_url.clone()],
		state_machine: Some(StateMachine::Evm(ARC_TESTNET_CHAIN_ID)),
		consensus_state_id: Some(String::from_utf8(ARC_CONSENSUS_ID.to_vec())?),
		// Resolved explicitly rather than through the registry: Arc has no
		// EvmHost deployment yet, and the consensus paths never read it.
		ismp_host: Some(ismp_host()?),
		signer: Some(
			"2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622".to_string(),
		),
		tracing_batch_size: None,
		query_batch_size: None,
		poll_interval: None,
		gas_price_buffer: None,
		client_type: Default::default(),
		initial_height: None,
		transport: Default::default(),
	};

	let host_config = ArcHostConfig {
		consensus_update_frequency: Some(10),
		rpc_url,
		certificate_rpc_url: Some(endpoint("ARC_CERT_RPC_URL", DEFAULT_ARC_TESTNET_RPC)),
		unbonding_period_secs: None,
	};

	ArcHost::new(&host_config, &evm_config).await
}

/// The initial consensus state the relayer would submit must describe the live
/// chain: the Arc chain id, a non empty validator set, and a state commitment
/// pinned to the same finalized header the state trusts.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires network access to the Arc testnet — set ARC_RPC_URL / ARC_CERT_RPC_URL to override the default RPC and run with --ignored"]
async fn arc_initial_consensus_state() -> Result<(), anyhow::Error> {
	setup_logging();
	dotenv::dotenv().ok();

	let host = arc_host().await?;

	let create = host
		.query_initial_consensus_state()
		.await?
		.expect("Arc host always builds an initial consensus state");

	assert_eq!(create.consensus_client_id, ARC_CONSENSUS_CLIENT_ID);
	assert_eq!(create.consensus_state_id, ARC_CONSENSUS_ID);
	assert!(create.unbonding_period > 0);

	let state = ConsensusState::decode(&mut &create.consensus_state[..])?;
	assert_eq!(state.chain_id, ARC_TESTNET_CHAIN_ID);
	assert!(state.finalized_height > 0);
	assert_ne!(state.finalized_hash, H256::zero());
	assert!(!state.current_validators.is_empty());

	let (id, commitment) = create
		.state_machine_commitments
		.first()
		.cloned()
		.expect("an initial state commitment is bundled with the consensus state");
	assert_eq!(id.state_id, StateMachine::Evm(ARC_TESTNET_CHAIN_ID));
	assert_eq!(id.consensus_state_id, ARC_CONSENSUS_ID);
	// The commitment must describe the header the consensus state trusts,
	// otherwise the first update is verified against a state root nothing
	// vouched for.
	assert_eq!(commitment.height, state.finalized_height);
	assert_ne!(commitment.commitment.state_root, H256::zero());
	assert!(commitment.commitment.timestamp > 0);

	log::info!(
		"initial consensus state at height {} with {} validators (total power {})",
		state.finalized_height,
		state.current_validators.len(),
		state.current_validators.total_voting_power,
	);

	Ok(())
}

/// Mirrors `pharos_consensus_updates`: bootstrap the client on hyperbridge from
/// the live chain, run the relayer's consensus task, and wait for the on chain
/// consensus state to actually advance.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires a block producing hyperbridge node at HYPERBRIDGE_WS plus network access to the Arc testnet"]
async fn arc_consensus_updates() -> Result<(), anyhow::Error> {
	setup_logging();
	dotenv::dotenv().ok();

	let arc_host = arc_host().await?;

	let config_a = SubstrateConfig {
		state_machine: Some(StateMachine::Kusama(4009)),
		hashing: Some(HashAlgorithm::Keccak),
		consensus_state_id: Some("PARA".to_string()),
		rpc_ws: endpoint("HYPERBRIDGE_WS", "ws://localhost:9990"),
		max_rpc_payload_size: None,
		signer: Some(
			"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
		),
		initial_height: None,
		max_concurent_queries: None,
		poll_interval: None,
		fee_token_decimals: None,
	};
	let chain_a = SubstrateClient::<Hyperbridge>::new(config_a).await?;

	let create = arc_host
		.query_initial_consensus_state()
		.await?
		.expect("Arc host always builds an initial consensus state");
	let initial = ConsensusState::decode(&mut &create.consensus_state[..])?;

	log::info!("creating consensus state at height {}", initial.finalized_height);
	chain_a.create_consensus_state(create).await?;

	let counterparty: Arc<dyn IsmpProvider> = Arc::new(chain_a);
	let consensus_task = tokio::spawn({
		let arc_host = arc_host.clone();
		let counterparty = counterparty.clone();
		async move { arc_host.start_consensus(counterparty).await }
	});

	// Wait for the client to move past the height it was bootstrapped at. The
	// host ticks every 10s, so this allows for a handful of ticks plus the
	// challenge period.
	let advanced = crate::util::timeout_future(
		async {
			loop {
				let encoded = counterparty
					.query_consensus_state(None, ARC_CONSENSUS_ID)
					.await
					.expect("consensus state was just created");
				let state = ConsensusState::decode(&mut &encoded[..])
					.expect("the pallet stores what the relayer submitted");
				if state.finalized_height > initial.finalized_height {
					break state;
				}
				tokio::time::sleep(Duration::from_secs(5)).await;
			}
		},
		60 * 5,
		"Arc consensus state never advanced past its initial height".to_string(),
	)
	.await;

	log::info!(
		"consensus state advanced from {} to {}",
		initial.finalized_height,
		advanced.finalized_height
	);
	assert!(!advanced.current_validators.is_empty());

	// Soak: keep the relayer running so repeated updates, and any validator
	// set rotation they walk through, are exercised rather than just the
	// first hop off the bootstrap height.
	let soak = std::env::var("ARC_SOAK_SECS")
		.ok()
		.and_then(|secs| secs.parse::<u64>().ok())
		.unwrap_or_default();
	if soak > 0 {
		let deadline = Instant::now() + Duration::from_secs(soak);
		let mut last = advanced;
		let mut updates = 1u64;
		while Instant::now() < deadline {
			tokio::time::sleep(Duration::from_secs(10)).await;

			assert!(!consensus_task.is_finished(), "the consensus task exited early");

			let encoded = counterparty.query_consensus_state(None, ARC_CONSENSUS_ID).await?;
			let state = ConsensusState::decode(&mut &encoded[..])?;
			assert!(
				state.finalized_height >= last.finalized_height,
				"consensus state went backwards: {} -> {}",
				last.finalized_height,
				state.finalized_height
			);
			if state.finalized_height > last.finalized_height {
				updates += 1;
				if state.current_validators != last.current_validators {
					log::info!(
						"validator set rotated at {}: {} validators, total power {}",
						state.finalized_height,
						state.current_validators.len(),
						state.current_validators.total_voting_power,
					);
				}
				last = state;
			}
		}
		log::info!("soaked {soak}s: {updates} updates, now at height {}", last.finalized_height);
		assert!(updates > 1, "the relayer stopped updating after the first hop");
	}

	consensus_task.abort();

	Ok(())
}
