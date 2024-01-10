use bnb_pos_verifier::{
    primitives::{compute_epoch, parse_extra, EPOCH_LENGTH},
    verify_bnb_header, NextValidators,
};
use ethers::{
    prelude::{Middleware, StreamExt},
    providers::{Provider, Ws},
};
use geth_primitives::CodecHeader;
use ismp::util::Keccak256;
use sync_committee_primitives::constants::BlsPublicKey;

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
    let latest_block = prover.latest_header().await.unwrap();
    let epoch_1 = compute_epoch(latest_block.number.low_u64()) - 1;
    let epoch_1_header = prover.fetch_header(epoch_1 * EPOCH_LENGTH).await.unwrap();
    let epoch_2_header = prover.fetch_header((epoch_1 + 1) * EPOCH_LENGTH).await.unwrap();

    let epoch_1_extra = parse_extra::<Host>(&epoch_1_header.extra_data).unwrap();
    let validators = epoch_1_extra
        .validators
        .into_iter()
        .map(|val| val.bls_public_key.as_slice().try_into().expect("Infallible"))
        .collect::<Vec<BlsPublicKey>>();

    let update = prover.fetch_bnb_update::<Host>(epoch_2_header.clone()).await.unwrap();

    let result = verify_bnb_header::<Host>(&validators, update.unwrap()).unwrap();
    dbg!(result);
}

#[tokio::test]
#[ignore]
async fn verify_bnb_pos_headers() {
    let prover = setup_prover().await;
    let latest_block = prover.latest_header().await.unwrap();
    let epoch_header = prover
        .fetch_header((compute_epoch(latest_block.number.low_u64())) * EPOCH_LENGTH)
        .await
        .unwrap();

    let epoch_extra = parse_extra::<Host>(&epoch_header.extra_data).unwrap();
    let mut validators = epoch_extra
        .validators
        .into_iter()
        .map(|val| val.bls_public_key.as_slice().try_into().expect("Infallible"))
        .collect::<Vec<BlsPublicKey>>();
    let mut next_validators: Option<NextValidators> = None;
    let mut current_epoch = compute_epoch(latest_block.number.low_u64());

    let mut sub = prover.client.subscribe_blocks().await.unwrap();
    while let Some(block) = sub.next().await {
        let header: CodecHeader = block.into();
        let block_epoch = compute_epoch(header.number.low_u64());

        if let Some(update) = prover.fetch_bnb_update::<Host>(header.clone()).await.unwrap() {
            dbg!(block_epoch);
            dbg!(current_epoch);
            dbg!(header.number);
            if next_validators.is_some() &&
                update.attested_header.number.low_u64() >=
                    next_validators.as_ref().unwrap().rotation_block
            {
                println!("VALIDATOR SET ROTATED SUCCESSFULLY");
                validators = next_validators.as_ref().unwrap().validators.clone();
                next_validators = None;
            }
            let result = verify_bnb_header::<Host>(&validators, update).unwrap();
            dbg!(&result.hash);
            dbg!(result.next_validators.is_some());
            if let Some(validators) = result.next_validators {
                dbg!(validators.rotation_block);
                next_validators = Some(validators);
                current_epoch = block_epoch
            }
        }
    }
}
