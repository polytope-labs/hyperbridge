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

use alloy::{eips::BlockNumberOrTag, providers::Provider};
use bsc_verifier::{
	primitives::{compute_epoch, parse_extra, Testnet, VALIDATOR_BIT_SET_SIZE},
	verify_bsc_header, NextValidators,
};
use geth_primitives::CodecHeader;
use ismp::messaging::Keccak256;
use polkadot_sdk::*;
use ssz_rs::{Bitvector, Deserialize};
use std::time::Duration;

use crate::BscPosProver;

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
	BscPosProver::new(url)
}

#[tokio::test]
#[ignore]
async fn verify_bsc_pos_headers() {
	let prover = setup_prover().await;
	let latest_block = prover.latest_header().await.unwrap();
	let (epoch_header, validators) =
		prover.fetch_finalized_state::<Host>(EPOCH_LENGTH).await.unwrap();
	if latest_block.number.low_u64() - epoch_header.number.low_u64() < 48 {
		// We want to ensure the current validators have been enacted before continuing
		tokio::time::sleep(Duration::from_secs(
			(latest_block.number.low_u64() - epoch_header.number.low_u64()) * 48,
		))
		.await;
	}
	let mut next_validators: Option<NextValidators> = None;
	let mut current_epoch = compute_epoch(latest_block.number.low_u64(), EPOCH_LENGTH);
	let mut last_block_number = latest_block.number.low_u64();
	// Verify at least an epoch change until validator set is rotated
	loop {
		// Poll for new blocks
		tokio::time::sleep(Duration::from_millis(750)).await;
		let block = match prover.client.get_block(BlockNumberOrTag::Latest.into()).await {
			Ok(Some(b)) => b,
			_ => continue,
		};
		let block_number = block.header.number;
		if block_number <= last_block_number {
			continue;
		}
		last_block_number = block_number;
		let header: CodecHeader = block.into();
		let block_epoch = compute_epoch(header.number.low_u64(), EPOCH_LENGTH);

		if let Some(mut update) = prover
			.fetch_bsc_update::<Host>(crate::UpdateParams {
				attested_header: header.clone(),
				validator_size: validators.len() as u64,
				epoch_length: EPOCH_LENGTH,
				epoch: current_epoch + 1,
				fetch_val_set_change: block_epoch > current_epoch,
			})
			.await
			.unwrap()
		{
			dbg!(block_epoch);
			dbg!(current_epoch);
			dbg!(header.number);

			if next_validators.is_some() {
				update.epoch_header_ancestry = Default::default();
			}

			let extra_data = match parse_extra::<Host, Testnet>(&update.attested_header) {
				Ok(extra) => extra,
				Err(e) => {
					println!("Failed to parse extra data for block {}: {}", header.number, e);
					continue;
				},
			};

			let validators_bit_set = match Bitvector::<VALIDATOR_BIT_SET_SIZE>::deserialize(
				extra_data.vote_address_set.to_le_bytes().to_vec().as_slice(),
			) {
				Ok(bits) => bits,
				Err(e) => {
					println!(
						"Failed to deserialize vote address set for block {}: {:?}",
						header.number, e
					);
					continue;
				},
			};

			// Determine which validator set to use for participant check
			let use_next_validators = next_validators.is_some() &&
				update.attested_header.number.low_u64() % EPOCH_LENGTH >=
					(validators.len() as u64 / 2);

			let validator_set_for_check = if use_next_validators {
				&next_validators.as_ref().unwrap().validators
			} else {
				&validators
			};

			// Skip blocks without enough participants (2/3 + 1 threshold)
			let participant_count = validators_bit_set.iter().as_bitslice().count_ones();
			let required_participants = (2 * validator_set_for_check.len() / 3) + 1;
			if participant_count < required_participants {
				println!(
					"Not enough participants in bsc update for block {} ({}/{}), skipping",
					header.number, participant_count, required_participants
				);
				continue;
			}

			if use_next_validators {
				let result = verify_bsc_header::<Host, Testnet>(
					&next_validators.clone().unwrap().validators,
					update.clone(),
					EPOCH_LENGTH,
				);
				if result.is_ok() {
					println!("VALIDATOR SET ROTATED SUCCESSFULLY");
					return;
				} else {
					println!("VALIDATOR SET NOT YET ROTATED");
					continue;
				}
			}

			// Skip blocks that fail verification
			let result =
				match verify_bsc_header::<Host, Testnet>(&validators, update.clone(), EPOCH_LENGTH)
				{
					Ok(r) => r,
					Err(e) => {
						println!(
							"Verification failed for block {}: {}, skipping",
							header.number, e
						);
						continue;
					},
				};

			dbg!(&result.hash);
			dbg!(result.next_validators.is_some());
			if let Some(new_vals) = result.next_validators {
				next_validators = Some(new_vals);
				current_epoch = block_epoch
			}
		}
	}
}
