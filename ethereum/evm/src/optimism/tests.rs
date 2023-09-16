use crate::{
    mock::Host,
    optimism::client::{OpConfig, OpHost},
};
use consensus_client::optimism::verify_optimism_payload;
use ethabi::ethereum_types::H160;
use ethers::providers::Middleware;
use hex_literal::hex;

const L2_ORACLE: [u8; 20] = hex!("dfe97868233d1aa22e815a266982f2cf17685a27");
const MESSAGE_PARSER: [u8; 20] = hex!("4200000000000000000000000000000000000016");

#[tokio::test]
async fn test_payload_proof_verification() {
    let config = OpConfig {
        beacon_execution_client: "wss://rpc.ankr.com/eth/ws/6875dedefed4afb05996bc795f89f9cd6f245f5117302f2c9214376ec1d96513".to_string(),
        op_execution: "wss://rpc.ankr.com/optimism/ws/6875dedefed4afb05996bc795f89f9cd6f245f5117302f2c9214376ec1d96513".to_string(),
        l2_oracle: H160::from(L2_ORACLE),
        message_parser: H160::from(MESSAGE_PARSER),
        evm_config: None
    };

    let op_client = OpHost::new(config).await.expect("Host creation failed");

    let event = op_client
        .latest_event(18022470, 18022470)
        .await
        .expect("Failed to fetch latest event")
        .expect("There should be an event");

    let payload_proof = op_client
        .fetch_optimism_payload(18022470, event)
        .await
        .expect("Error fetching payload proof");

    let l1_header = op_client
        .beacon_execution_client
        .get_block(18022470)
        .await
        .unwrap()
        .expect("Block should exist");

    let state_root = l1_header.state_root;

    let _ =
        verify_optimism_payload::<Host>(payload_proof, state_root.as_bytes(), op_client.l2_oracle)
            .expect("Payload proof verification should succeed");
}
