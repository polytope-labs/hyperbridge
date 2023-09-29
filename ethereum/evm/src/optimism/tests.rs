use crate::{
    mock::Host,
    optimism::client::{OpConfig, OpHost},
    EvmConfig,
};
use consensus_client::optimism::verify_optimism_payload;
use ethabi::ethereum_types::H160;
use ethers::providers::Middleware;
use hex_literal::hex;

const L2_ORACLE: [u8; 20] = hex!("E6Dfba0953616Bacab0c9A8ecb3a9BBa77FC15c0");
const MESSAGE_PARSER: [u8; 20] = hex!("4200000000000000000000000000000000000016");

#[tokio::test]
async fn test_payload_proof_verification() {
    dotenv::dotenv().ok();
    let op_orl = std::env::var("OP_URL").expect("OP_URL must be set.");
    let geth_url = std::env::var("GETH_URL").expect("GETH_URL must be set.");
    let config = OpConfig {
        beacon_execution_ws: geth_url,
        l2_oracle: H160::from(L2_ORACLE),
        message_parser: H160::from(MESSAGE_PARSER),
        evm_config: EvmConfig { execution_ws: op_orl, ..Default::default() },
    };

    let op_client = OpHost::new(&config).await.expect("Host creation failed");

    let event = op_client
        .latest_event(9779635, 9779635)
        .await
        .expect("Failed to fetch latest event")
        .expect("There should be an event");

    let payload_proof =
        op_client.fetch_op_payload(9779635, event).await.expect("Error fetching payload proof");

    let l1_header = op_client
        .beacon_execution_client
        .get_block(9779635)
        .await
        .unwrap()
        .expect("Block should exist");

    let state_root = l1_header.state_root;

    let _ =
        verify_optimism_payload::<Host>(payload_proof, state_root.as_bytes(), op_client.l2_oracle)
            .expect("Payload proof verification should succeed");
}
