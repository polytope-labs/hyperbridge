use bsc_verifier::{
	primitives::{compute_epoch, Testnet, EPOCH_LENGTH},
	verify_bsc_header, NextValidators,
};
use ethers::{
	prelude::{Middleware, ProviderExt, StreamExt},
	providers::{Http, Provider},
};
use geth_primitives::CodecHeader;
use ismp::messaging::Keccak256;
use std::time::Duration;

use crate::BscPosProver;

pub struct Host;

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
	let mut provider = Provider::<Http>::connect(&consensus_url).await;
	// Bsc block time is 3s we don't want to deal with missing authority set changes while polling
	// for blocks in our tests
	provider.set_interval(Duration::from_secs(3));
	BscPosProver::new(provider)
}

#[tokio::test]
#[ignore]
async fn verify_bsc_pos_headers() {
	let prover = setup_prover().await;
	let latest_block = prover.latest_header().await.unwrap();
	let (epoch_header, validators) = prover.fetch_finalized_state::<Host>().await.unwrap();
	if latest_block.number.low_u64() - epoch_header.number.low_u64() < 12 {
		// We want to ensure the current validators have been enacted before continuing
		tokio::time::sleep(Duration::from_secs(
			(latest_block.number.low_u64() - epoch_header.number.low_u64()) * 12,
		))
		.await;
	}
	let mut next_validators: Option<NextValidators> = None;
	let mut current_epoch = compute_epoch(latest_block.number.low_u64());
	let mut sub = prover.client.watch_blocks().await.unwrap();
	// Verify at least an epoch change until validator set is rotated
	while let Some(block) = sub.next().await {
		let header: CodecHeader = prover.fetch_header(block).await.unwrap().unwrap();
		let block_epoch = compute_epoch(header.number.low_u64());

		if let Some(mut update) = prover
			.fetch_bsc_update::<Host>(
				header.clone(),
				validators.len() as u64,
				current_epoch + 1,
				block_epoch > current_epoch,
			)
			.await
			.unwrap()
		{
			dbg!(block_epoch);
			dbg!(current_epoch);
			dbg!(header.number);

			if next_validators.is_some() {
				update.epoch_header_ancestry = Default::default();
			}

			if next_validators.is_some() &&
				update.attested_header.number.low_u64() % EPOCH_LENGTH >=
					(validators.len() as u64 / 2)
			{
				let result = verify_bsc_header::<Host, Testnet>(
					&next_validators.clone().unwrap().validators,
					update.clone(),
				);
				if result.is_ok() {
					println!("VALIDATOR SET ROTATED SUCCESSFULLY");
					return;
				} else {
					println!("VALIDATOR SET NOT YET ROTATED");
					continue;
				}
			}
			let result = verify_bsc_header::<Host, Testnet>(&validators, update.clone()).unwrap();
			dbg!(&result.hash);
			dbg!(result.next_validators.is_some());
			if let Some(validators) = result.next_validators {
				next_validators = Some(validators);
				current_epoch = block_epoch
			}
		}
	}
}
