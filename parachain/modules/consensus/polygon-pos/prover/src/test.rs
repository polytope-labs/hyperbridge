use ethers::providers::{Provider, Ws};
use geth_primitives::Header;
use ismp::util::Keccak256;
use polygon_pos_verifier::verify_polygon_header;

use crate::PolygonPosProver;

pub struct Host;

impl Keccak256 for Host {
    fn keccak256(bytes: &[u8]) -> primitive_types::H256
    where
        Self: Sized,
    {
        sp_core::keccak_256(bytes).into()
    }
}

async fn setup_prover() -> PolygonPosProver {
    dotenv::dotenv().ok();
    let consensus_url = std::env::var("POLYGON_RPC").unwrap();
    let provider = Provider::<Ws>::connect_with_reconnects(consensus_url, 1000).await.unwrap();

    PolygonPosProver::new(provider)
}

#[tokio::test]
#[ignore]
async fn verify_polygon_pos_headers() {
    let prover = setup_prover().await;

    let (mut finalized_header, mut validators) = prover.fetch_finalized_state().await.unwrap();

    let mut parent_hash = Header::from(&finalized_header).hash::<Host>();
    for number in (finalized_header.number.low_u64() + 1)..=(finalized_header.number.low_u64() + 10)
    {
        let header = prover.fetch_header(number).await.unwrap().unwrap();
        if parent_hash == header.parent_hash {
            parent_hash = Header::from(&header).hash::<Host>();
            let result = verify_polygon_header::<Host>(&validators, header).unwrap();
            finalized_header = result.header;
            if let Some(next_validators) = result.next_validators {
                validators = next_validators;
            }
            println!("Successfully verified header {:?}", finalized_header.number.low_u64());
        } else {
            println!("Header not verified");
            break
        }
    }
}
