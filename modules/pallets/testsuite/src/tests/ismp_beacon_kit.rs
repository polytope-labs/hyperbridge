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

use codec::Encode;
use ismp::consensus::ConsensusClient;
use ismp_beacon_kit::{
	BeaconKitClient, BeaconKitUpdate, ConsensusState, BERACHAIN_MAINNET_CHAIN_ID,
};
use polkadot_sdk::sp_runtime::BoundedVec;
use std::io::Read;
use tendermint_primitives::{
	Client, CodecConsensusProof, CodecTrustedState, ConsensusProof, TrustedState,
	VerificationOptions, Validator, PubKey,
};
use tendermint_prover::CometBFTClient;

use crate::runtime::{new_test_ext, set_timestamp, Ismp, Test};

/// Fetch all transactions from a block at the given height.
/// The first transaction (txs[0]) is the SSZ-encoded SignedBeaconBlock.
async fn fetch_block_txs(rpc_url: &str, height: u64) -> anyhow::Result<Vec<Vec<u8>>> {
	let request_body = serde_json::json!({
		"jsonrpc": "2.0",
		"id": "1",
		"method": "block",
		"params": {
			"height": height.to_string()
		}
	});

	let http_client = reqwest::Client::new();
	let response = http_client
		.post(rpc_url)
		.json(&request_body)
		.send()
		.await
		.map_err(|e| anyhow::anyhow!("Block fetch request failed: {}", e))?;

	if !response.status().is_success() {
		return Err(anyhow::anyhow!("HTTP error fetching block: {}", response.status()));
	}

	let rpc_response: serde_json::Value = response
		.json()
		.await
		.map_err(|e| anyhow::anyhow!("Failed to parse block response: {}", e))?;

	let txs = rpc_response
		.get("result")
		.and_then(|r| r.get("block"))
		.and_then(|b| b.get("data"))
		.and_then(|d| d.get("txs"))
		.and_then(|t| t.as_array())
		.ok_or_else(|| anyhow::anyhow!("Failed to extract txs from block response"))?;

	let mut decoded_txs = Vec::with_capacity(txs.len());
	for tx in txs {
		let tx_str = tx.as_str().ok_or_else(|| anyhow::anyhow!("Transaction is not a string"))?;
		let tx_bytes = base64_decode(tx_str)?;
		decoded_txs.push(tx_bytes);
	}

	Ok(decoded_txs)
}

fn base64_decode(s: &str) -> anyhow::Result<Vec<u8>> {
	use base64::{engine::general_purpose::STANDARD, read::DecoderReader};

	let mut decoder = DecoderReader::new(s.as_bytes(), &STANDARD);
	let mut decoded = Vec::new();
	decoder.read_to_end(&mut decoded)?;
	Ok(decoded)
}


#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn beaconkit_verify_consensus() -> anyhow::Result<()> {
	let _ = tracing_subscriber::fmt::try_init();
	dotenv::dotenv().ok();

	let rpc_url = std::env::var("BEACONKIT_COMETBFT_RPC").expect("BEACONKIT_COMETBFT_RPC must be set");
	let client = CometBFTClient::new(&rpc_url).await?;

	let chain_id = client.chain_id().await?;

	let latest_height = client.latest_height().await?;
	let trusted_height = latest_height.saturating_sub(2);
	let target_height = latest_height.saturating_sub(1);

	println!("Latest height: {}", latest_height);
	println!("Trusted height: {}", trusted_height);
	println!("Target height: {} (testing adjacent block)", target_height);

	let trusted_header = client.signed_header(trusted_height).await?;
	let trusted_validators = client.validators(trusted_height).await?;
	let trusted_next_validators = client.next_validators(trusted_height).await?;

	let trusted_state = TrustedState::new(
		chain_id,
		trusted_height,
		trusted_header.header.time.unix_timestamp() as u64,
		trusted_header.header.hash().as_bytes().try_into().unwrap(),
		trusted_validators,
		trusted_next_validators,
		trusted_header.header.next_validators_hash.as_bytes().try_into().unwrap(),
		82 * 3600,
		VerificationOptions::default(),
	);

	let consensus_state = ConsensusState {
		tendermint_state: CodecTrustedState::from(&trusted_state),
		chain_id: BERACHAIN_MAINNET_CHAIN_ID,
	};
	let encoded_consensus_state = consensus_state.encode();

	let target_header = client.signed_header(target_height).await?;
	let target_validators = client.validators(target_height).await?;
	let target_next_validators = client.next_validators(target_height).await?;
	let txs = fetch_block_txs(&rpc_url, target_height).await?;

	println!("Target block has {} transactions", txs.len());
	println!("Target header commit has {} signatures", target_header.commit.signatures.len());


	if txs.is_empty() {
		println!("Block has no transactions - skipping verify_consensus test");
		return Ok(());
	}

	let consensus_proof = ConsensusProof::new(target_header.clone(), Some(target_next_validators));

	println!("Total transactions: {}", txs.len());
	println!("SSZ beacon block size: {} bytes", txs[0].len());

	let beacon_kit_update = BeaconKitUpdate {
		tendermint_update: CodecConsensusProof::from(&consensus_proof),
		txs: BoundedVec::truncate_from(txs),
	};
	let encoded_proof = beacon_kit_update.encode();

	new_test_ext().execute_with(|| {
		let current_time = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_millis() as u64;
		set_timestamp::<Test>(current_time);

		let beacon_kit_client = BeaconKitClient::<Ismp, Test>::default();

		let result = beacon_kit_client.verify_consensus(
			&Ismp::default(),
			*b"BKIT",
			encoded_consensus_state.clone(),
			encoded_proof.clone(),
		);

		match result {
			Ok((new_consensus_state, verified_commitments)) => {
				println!("verify_consensus: PASSED");
				println!(
					"New consensus state size: {} bytes",
					new_consensus_state.len()
				);
				println!("Verified commitments: {:?}", verified_commitments.keys().collect::<Vec<_>>());

				// check that we have state commitments
				assert!(!verified_commitments.is_empty(), "Should have verified commitments");


				let new_state: ConsensusState =
					codec::Decode::decode(&mut &new_consensus_state[..])
						.expect("Should decode new consensus state");
				println!(
					"New trusted state height: {}",
					new_state.tendermint_state.height
				);

				// new height should be the target height
				assert_eq!(
					new_state.tendermint_state.height, target_height,
					"New state height should match target height"
				);
			},
			Err(e) => {
				panic!("verify_consensus failed: {:?}", e);
			},
		}
	});

	Ok(())
}