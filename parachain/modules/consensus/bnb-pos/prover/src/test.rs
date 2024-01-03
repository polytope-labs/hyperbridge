use bnb_pos_verifier::{
    primitives::{compute_epoch, parse_extra, EPOCH_LENGTH},
    verify_bnb_header,
};
use ethers::providers::{Provider, Ws};
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
#[ignore]
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

    let result = verify_bnb_header::<Host>(&validators, update).unwrap();
    dbg!(result);
}
