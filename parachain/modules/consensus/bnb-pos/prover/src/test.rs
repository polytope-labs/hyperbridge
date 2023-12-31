use std::time::Duration;

use bnb_pos_verifier::{
    primitives::{compute_epoch, Header, EPOCH_LENGTH},
    verify_bnb_header,
};
use ethers::providers::{Provider, Ws};
use ismp::util::Keccak256;
use sp_core::H160;
use tokio::time::interval;

use crate::BnbPosProver;

pub struct Host;

impl Keccak256 for Host {
    fn keccak256(bytes: &[u8]) -> primitive_types::H256
    where
        Self: Sized,
    {
        sp_core::keccak_256(bytes).into()
    }
}

async fn setup_prover() -> BnbPosProver {
    dotenv::dotenv().ok();
    let consensus_url = std::env::var("BNB_RPC").unwrap();
    let provider = Provider::<Ws>::connect_with_reconnects(consensus_url, 1000).await.unwrap();

    BnbPosProver::new(provider)
}

#[tokio::test]
async fn verify_bnb_pos_header() {
    let prover = setup_prover().await;

    let mut interval = interval(Duration::from_secs(2));

    let initial_header = prover.latest_header().await.unwrap();
    let initial_header_epoch = compute_epoch(initial_header.number.low_u64());

    println!(
        "Initial header {:?} in epoch {:?}",
        initial_header,
        initial_header_epoch
    );

    let validator_change_block_number = initial_header_epoch * EPOCH_LENGTH;
    let validator_change_header = prover.fetch_header(validator_change_block_number).await.unwrap();

    println!(
        "Initial header {:?} in epoch {:?}",
        validator_change_header,
        validator_change_block_number
    );

    let (_, validators_data) = prover
        .fetch_proofs_and_validators::<Host>(validator_change_header)
        .await
        .unwrap();
    let mut validators = validators_data.unwrap();

    loop {
        interval.tick().await;
        let latest_header = prover.latest_header().await.unwrap();
        let latest_header_epoch = compute_epoch(latest_header.number.low_u64());

        if initial_header_epoch == latest_header_epoch {
            verify_bnb_header::<Host>(&validators, latest_header.clone()).unwrap();
            println!(
                "Successfully verified header {:?} in epoch {:?}",
                latest_header.number.low_u64(),
                latest_header_epoch
            );
        }

        if initial_header_epoch > latest_header_epoch {
            for epoch in (initial_header_epoch.clone() + 1)..latest_header_epoch {
                let last_header_number_epoch = (epoch * EPOCH_LENGTH) + EPOCH_LENGTH - 1;
                let last_header_in_epoch =
                    prover.fetch_header(last_header_number_epoch.clone()).await.unwrap();

                let previous_epoch = last_header_number_epoch - EPOCH_LENGTH;
                let previous_epoch_header =
                    prover.fetch_header(previous_epoch.clone()).await.unwrap();
                let (previous_epoch_bnb_proof, _previous_epoch_validators_data) = prover
                    .fetch_proofs_and_validators::<Host>(previous_epoch_header)
                    .await
                    .unwrap();
                let previous_epoch_validator_set_size = previous_epoch_bnb_proof.validator_set_size;

                let validator_set_change_epoch: u64 =
                    previous_epoch_validator_set_size as u64 / 2 + previous_epoch;

                if epoch == validator_set_change_epoch {
                    let validator_set_change_epoch_header =
                        prover.fetch_header(epoch.clone()).await.unwrap();
                    let (_, validator_set_change_validators_data) = prover
                        .fetch_proofs_and_validators::<Host>(validator_set_change_epoch_header)
                        .await
                        .unwrap();

                    validators = validator_set_change_validators_data.unwrap();
                }

                verify_bnb_header::<Host>(&validators, last_header_in_epoch.clone()).unwrap();

                println!(
                    "Successfully verified header {:?} in new epoch {:?}",
                    latest_header.number.low_u64(),
                    last_header_number_epoch
                );
            }
        }
    }
}
