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

//! Live tests against a Kaia RPC endpoint. Run with:
//! `cargo test -p kaia-prover -- --ignored --nocapture`
//! Override the endpoint with the `KAIA_URL` environment variable.

use crate::{Host, KaiaProver};
use codec::{Decode, Encode};
use kaia_verifier::{primitives::KaiaClientUpdate, verify_and_apply};

fn prover() -> KaiaProver {
	let url =
		std::env::var("KAIA_URL").unwrap_or_else(|_| "https://public-en.node.kaia.io".to_string());
	KaiaProver::new(vec![url]).unwrap()
}

#[tokio::test]
#[ignore]
async fn reconstructs_mainnet_headers() {
	let prover = prover();
	let latest = prover.latest_block_number().await.unwrap();
	// `fetch_header` internally asserts that the reconstructed RLP hashes to
	// the node-reported block hash; a run of consecutive recent headers is the
	// real encoding test.
	for number in (latest - 10)..=latest {
		let header = prover.fetch_header(number).await.unwrap();
		assert_eq!(header.number.low_u64(), number);
	}

	// A pre-Randao header (no randomReveal/mixHash/blob fields in the tail)
	// exercises the optional-tail handling. Randao activated at 147,534,000.
	let header = prover.fetch_header(147_000_000).await.unwrap();
	assert!(header.random_reveal.is_none());
	// And one from before the Magma fork (no baseFeePerGas either).
	let header = prover.fetch_header(99_000_000).await.unwrap();
	assert!(header.base_fee_per_gas.is_none());
}

#[tokio::test]
#[ignore]
async fn verifies_live_finality() {
	let prover = prover();
	let latest = prover.latest_block_number().await.unwrap();
	let trusted_height = latest - 60;
	let (mut state, _) = prover.fetch_consensus_state(trusted_height).await.unwrap();
	let initial_state = state.clone();
	println!(
		"trusted state at {trusted_height}: council={}, committee_size={}, quorum={}",
		state.council.len(),
		state.committee_size,
		kaia_verifier::required_seals(state.council.len() as u64, state.committee_size),
	);

	// Advance the client across a run of finalized headers one by one.
	for number in (trusted_height + 1)..=(trusted_height + 20) {
		let header = prover.fetch_header(number).await.unwrap();
		let update = KaiaClientUpdate { headers: vec![header] };
		let result = verify_and_apply::<Host>(&mut state, &update)
			.unwrap_or_else(|e| panic!("header {number} failed verification: {e:?}"));
		assert_eq!(result.height, number);
	}
	assert_eq!(state.finalized_height, trusted_height + 20);

	// And in one batched update with a gap.
	let mut state = initial_state;
	let update = prover.fetch_kaia_update(&state, trusted_height + 40).await.unwrap().unwrap();
	let result = verify_and_apply::<Host>(&mut state, &update).unwrap();
	assert_eq!(result.height, trusted_height + 40);

	// The update round-trips through SCALE unchanged.
	let decoded = KaiaClientUpdate::decode(&mut &update.encode()[..]).unwrap();
	assert_eq!(decoded, update);
}

#[tokio::test]
#[ignore]
async fn verifies_across_historical_vote_headers() {
	let prover = prover();
	// The council changed over the chain's history; verify the incremental
	// tracking on a historical range with a known validator vote, if the
	// endpoint retains it. This scans a small recent range as a smoke test.
	let latest = prover.latest_block_number().await.unwrap();
	let headers = prover.scan_for_council_updates(latest - 200, latest - 1, 604800).await.unwrap();
	println!("found {} council-relevant headers in the last 200 blocks", headers.len());
}
