// Copyright (C) 2022 Polytope Labs.
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

use bsc_verifier::{
	primitives::{compute_epoch, parse_extra, Testnet, VALIDATOR_BIT_SET_SIZE},
	verify_bsc_header,
};
use ismp::messaging::Keccak256;
use polkadot_sdk::*;
use ssz_rs::{Bitvector, Deserialize};
use std::time::Duration;

use crate::{get_rotation_block, BscPosProver, UpdateParams};

pub struct Host;

const EPOCH_LENGTH: u64 = 1000;

impl Keccak256 for Host {
	fn keccak256(bytes: &[u8]) -> primitive_types::H256
	where
		Self: Sized,
	{
		sp_core::keccak_256(bytes).into()
	}
}

async fn setup_prover() -> BscPosProver<Testnet> {
	dotenv::dotenv().ok();
	let consensus_url = std::env::var("BSC_URL").unwrap();
	let url = consensus_url.parse().expect("Invalid URL");
	BscPosProver::new(vec![url]).expect("Failed to create prover")
}

/// End-to-end test that mirrors the two-phase consensus-update loop used by
/// `tesseract::consensus::bsc::host::start_consensus`:
///
/// 1. **Sync phase** — starting at the first candidate attested block after an epoch boundary
///    (`epoch_block + 2`), walk block-by-block up to the latest position from which the previous
///    validator set can still sign (`rotation_block - 1 + epoch_length / 2`). For each header, ask
///    the prover for a `BscClientUpdate` that carries the new validator set (`fetch_val_set_change:
///    true`), drop updates with insufficient BLS participation, verify under the **current**
///    validator set, and accept the first one that either contains the epoch header ancestry or
///    whose source header *is* the epoch block. That update's `next_validators` becomes the pending
///    set.
///
/// 2. **Enactment phase** — starting at `get_rotation_block(...)`, walk up to `epoch_block +
///    epoch_length - 1` and pull an update with `fetch_val_set_change: false`. Drop
///    low-participation ones against the **next** validator set, then verify under the next set.
///    The first update that verifies proves rotation has taken effect.
///
/// Relative to the previous implementation which polled `latest_header` at
/// 750 ms and guessed at rotation, this version:
///   - walks specific deterministic block numbers rather than racing the chain tip, so it can't
///     accidentally skip or double-count blocks;
///   - waits for each header to materialize (sleeps 3 s and retries if the tip hasn't caught up),
///     which is the expected failure mode on a live chain rather than a reason to bail;
///   - uses the same `get_rotation_block` helper the production host uses, instead of re-deriving
///     the rotation boundary inline;
///   - distinguishes "update source header too old" from "update fails verification" so the sync
///     phase can fail fast on a truly bad update.
#[tokio::test]
#[ignore]
async fn verify_bsc_pos_headers() {
	let prover = setup_prover().await;

	// Start from the current epoch header + its validator set, exactly how
	// `host.rs` initializes `ConsensusState::current_validators`.
	let (epoch_header, current_validators) =
		prover.fetch_finalized_state::<Host>(EPOCH_LENGTH).await.unwrap();
	let current_epoch = compute_epoch(epoch_header.number.low_u64(), EPOCH_LENGTH);
	let mut finalized_height = epoch_header.number.low_u64();

	// ── Sync phase ──────────────────────────────────────────────────────────
	let next_epoch = current_epoch + 1;
	let epoch_block_number = next_epoch * EPOCH_LENGTH;
	let sync_end =
		get_rotation_block(epoch_block_number, current_validators.len() as u64, EPOCH_LENGTH) - 1 +
			EPOCH_LENGTH / 2;

	println!(
		"Sync phase: walking [{}, {}] against {}-validator set from epoch {}",
		epoch_block_number + 2,
		sync_end,
		current_validators.len(),
		current_epoch,
	);

	let mut block = epoch_block_number + 2;
	let next_validators = loop {
		assert!(
			block <= sync_end,
			"sync phase exhausted without finding a valid epoch-change update"
		);

		let Some(header) = prover.fetch_header(block).await.unwrap() else {
			// Chain tip hasn't reached `block` yet — wait and retry, same as host.rs.
			tokio::time::sleep(Duration::from_secs(3)).await;
			continue;
		};

		let maybe_update = prover
			.fetch_bsc_update::<Host>(UpdateParams {
				attested_header: header,
				validator_size: current_validators.len() as u64,
				epoch: next_epoch,
				epoch_length: EPOCH_LENGTH,
				fetch_val_set_change: true,
			})
			.await
			.unwrap();

		let Some(update) = maybe_update else {
			block += 1;
			continue;
		};

		// Reject updates with insufficient BLS participation from the current set.
		let extra_data = parse_extra::<Host, Testnet>(&update.attested_header)
			.expect("infallible: prover already parsed extra data");
		let validators_bit_set = Bitvector::<VALIDATOR_BIT_SET_SIZE>::deserialize(
			extra_data.vote_address_set.to_le_bytes().to_vec().as_slice(),
		)
		.expect("infallible: prover already parsed extra data");
		if validators_bit_set.iter().as_bitslice().count_ones() < (2 * current_validators.len() / 3)
		{
			println!("sync: not enough participants at block {block}, skipping");
			block += 1;
			continue;
		}

		// Skip updates whose source header was already finalized by an earlier update.
		if update.source_header.number.low_u64() <= finalized_height {
			block += 1;
			continue;
		}

		// In the sync phase a verification failure is terminal — if the current
		// validator set cannot sign an update at `block`, we cannot safely rotate.
		let result =
			verify_bsc_header::<Host, Testnet>(&current_validators, update.clone(), EPOCH_LENGTH)
				.expect("sync update failed to verify against current validator set");

		println!(
			"sync: verified update at block {block} against current set \
			(source_header={}, target_header={}, ancestry={}B)",
			update.source_header.number,
			update.target_header.number,
			update.epoch_header_ancestry.len(),
		);

		// Mirror `host.rs`: only accept a sync update that crosses the epoch
		// boundary (either by carrying ancestry back to the epoch block, or by
		// directly finalizing the epoch block itself).
		if update.epoch_header_ancestry.is_empty() &&
			update.source_header.number.low_u64() != epoch_block_number
		{
			println!(
				"sync: verified update at block {block} does not cross epoch boundary, skipping"
			);
			block += 1;
			continue;
		}

		let next = result.next_validators.expect("sync update must carry next validator set");
		finalized_height = update.source_header.number.low_u64();
		println!(
			"Sync accepted at block {block}: new {}-validator set staged, rotation at {}",
			next.validators.len(),
			next.rotation_block,
		);
		break next;
	};

	// ── Enactment phase ─────────────────────────────────────────────────────
	// Walk from the actual rotation block up to the end of the new epoch.
	// Verification must succeed under the *next* validator set before we
	// declare the rotation enacted.
	let rotation_start =
		get_rotation_block(epoch_block_number, current_validators.len() as u64, EPOCH_LENGTH);
	let enact_end = epoch_block_number + EPOCH_LENGTH - 1;

	println!(
		"Enactment phase: walking [{}, {}] against {}-validator next set",
		rotation_start,
		enact_end,
		next_validators.validators.len(),
	);

	let mut block = rotation_start;
	loop {
		assert!(
			block <= enact_end,
			"enactment phase exhausted without verifying a post-rotation update"
		);

		let Some(header) = prover.fetch_header(block).await.unwrap() else {
			tokio::time::sleep(Duration::from_secs(3)).await;
			continue;
		};

		let maybe_update = prover
			.fetch_bsc_update::<Host>(UpdateParams {
				attested_header: header,
				validator_size: current_validators.len() as u64,
				epoch: next_epoch,
				epoch_length: EPOCH_LENGTH,
				fetch_val_set_change: false,
			})
			.await
			.unwrap();

		let Some(mut update) = maybe_update else {
			block += 1;
			continue;
		};

		// Ancestry is only needed during the sync phase; once we've staged the
		// next set, drop it so verification runs purely against the new set.
		update.epoch_header_ancestry = Default::default();

		let extra_data = parse_extra::<Host, Testnet>(&update.attested_header)
			.expect("infallible: prover already parsed extra data");
		let validators_bit_set = Bitvector::<VALIDATOR_BIT_SET_SIZE>::deserialize(
			extra_data.vote_address_set.to_le_bytes().to_vec().as_slice(),
		)
		.expect("infallible: prover already parsed extra data");
		if validators_bit_set.iter().as_bitslice().count_ones() <
			(2 * next_validators.validators.len() / 3)
		{
			println!("enact: not enough participants at block {block}, skipping");
			block += 1;
			continue;
		}

		if update.source_header.number.low_u64() <= finalized_height {
			block += 1;
			continue;
		}

		// In the enactment phase verification failure is *expected* at first:
		// rotation might not have occurred yet at this block, so the next set
		// can't sign it. Log and keep walking.
		match verify_bsc_header::<Host, Testnet>(
			&next_validators.validators,
			update.clone(),
			EPOCH_LENGTH,
		) {
			Ok(_) => {
				println!(
					"enact: verified update at block {block} against next set \
					(source_header={}, target_header={})",
					update.source_header.number, update.target_header.number,
				);
				println!(
					"VALIDATOR SET ROTATED SUCCESSFULLY at block {block} \
					(source_header={})",
					update.source_header.number,
				);
				return;
			},
			Err(_) => {
				println!("enact: rotation not yet in effect at block {block}");
				block += 1;
			},
		}
	}
}
