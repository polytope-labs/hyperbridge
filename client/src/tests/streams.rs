#![cfg(target_arch = "wasm32")]


use std::str::FromStr;
use std::sync::Arc;
use ethers::prelude::{Provider, Ws};
use ethers::providers::{Http, ProviderExt};
use ethers::types::H160;
use ismp::consensus::{ConsensusStateId, StateMachineId};
use crate::providers::evm_chain::EvmClient;
use crate::providers::global::Client;
use crate::streams::{timeout_stream};
use futures::StreamExt;
use wasm_bindgen_test::*;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen::prelude::*;
use ismp::host::StateMachine;
use crate::providers::hyperbridge::HyperBridgeClient;



wasm_bindgen_test_configure!(run_in_browser);




#[wasm_bindgen_test]
async fn test_block_timestamp_timeout_stream() -> Result<(), anyhow::Error> {
    let rpc_url = "https://polygon-mumbai.g.alchemy.com/v2/wd5DZRJgh7Ini3Ps_xbuLRDsgQsInF9f";
    let consensus_state_id = *b"ETH0";
    let host_address = H160::from_str("0xd2AC90f4c83B5dfbB66653bA7Ca60AA83C448604").unwrap();
    let handler_address = H160::from_str("0xD145F3387417BE46bD4e7668935236b6EF0C90d4").unwrap();
    let state_machine = "POLY";


    let evm_client = EvmClient::new(
        rpc_url.into(),
        consensus_state_id.into(),
        host_address,
        handler_address,
        state_machine.into(),
    ).await.unwrap();




    let current_time = evm_client.query_latest_block_timestamp().await?;
    let timeout = 10u64;

    let mut timeout_stream = timeout_stream(current_time + timeout, evm_client).await;
    let mut timeout_stream_boxed = Box::pin(timeout_stream);


    while let Some(_) = timeout_stream_boxed.next().await {
        console_log!("timeout_stream_boxed.next().await");
    }

    Ok::<(), anyhow::Error>(())
}


#[wasm_bindgen_test]
async fn test_event_stream_stream_evm() -> Result<(), anyhow::Error> {
    let rpc_url = "https://opt-sepolia.g.alchemy.com/v2/qzZKMgRJ7zHxeUPoEvjYCmuAsJnx0oVP";
    let consensus_state_id = *b"ETH0";
    let host_address = H160::from_str("0x1B58A47e61Ca7604b634CBB00b4e275cCd7c9E95")?;
    let handler_address = H160::from_str("0xa25151598Dc180fc03635858f37bDF8427f47845")?;
    let state_machine = "OPTI";

    let evm_client = EvmClient::new(
        rpc_url.into(),
        consensus_state_id.into(),
        host_address,
        handler_address,
        state_machine.into(),
    ).await?;

    let mut event_stream = evm_client.event_stream().await?;

    while let Some(t) = event_stream.next().await {
        console_log!("{:?}", t.unwrap());
    }

    Ok(())
}


#[wasm_bindgen_test]
async fn test_post_request_handled_stream_evm() -> Result<(), anyhow::Error> {
    let rpc_url = "https://opt-sepolia.g.alchemy.com/v2/qzZKMgRJ7zHxeUPoEvjYCmuAsJnx0oVP";
    let consensus_state_id = *b"ETH0";
    let host_address = H160::from_str("0x1B58A47e61Ca7604b634CBB00b4e275cCd7c9E95")?;
    let handler_address = H160::from_str("0xa25151598Dc180fc03635858f37bDF8427f47845")?;
    let state_machine = "OPTI";

    let evm_client = EvmClient::new(
        rpc_url.into(),
        consensus_state_id.into(),
        host_address,
        handler_address,
        state_machine.into(),
    ).await?;

    let mut post_request_handled_stream = evm_client.post_request_handled_stream().await?;

    while let Some(t) = post_request_handled_stream.next().await {
        console_log!("{:?}", t.unwrap());
    }

    Ok(())
}


#[wasm_bindgen_test]
async fn test_state_machine_updated_stream_evm() -> Result<(), anyhow::Error> {
    let rpc_url = "https://opt-sepolia.g.alchemy.com/v2/qzZKMgRJ7zHxeUPoEvjYCmuAsJnx0oVP";
    let consensus_state_id = *b"ETH0";
    let host_address = H160::from_str("0x1B58A47e61Ca7604b634CBB00b4e275cCd7c9E95")?;
    let handler_address = H160::from_str("0xa25151598Dc180fc03635858f37bDF8427f47845")?;
    let state_machine = "OPTI";

    let evm_client = EvmClient::new(
        rpc_url.into(),
        consensus_state_id.into(),
        host_address,
        handler_address,
        state_machine.into(),
    ).await?;

    let mut state_machine_update_notification_stream = evm_client.state_machine_update_notification().await?;

    while let Some(t) = state_machine_update_notification_stream.next().await {
        console_log!("{:?}", t.unwrap());
    }

    Ok(())
}


#[wasm_bindgen_test]
async fn test_state_machine_updated_stream_hyperbridge() -> Result<(), anyhow::Error> {
    let hyper_bridge_client = HyperBridgeClient::new("ws://192.168.1.197:9988".into()).await?;
    let state_machine_raw = "OPTI";
    let state_machine: StateMachine = StateMachine::from_str(&state_machine_raw).unwrap();

    let consensus_state_id = *b"ETH0";
    let counter_party_state_machine_id = StateMachineId {
        consensus_state_id: consensus_state_id,
        state_id: state_machine,
    };



    let mut state_machine_update_notification_stream = hyper_bridge_client.state_machine_update_notification(counter_party_state_machine_id).await?;

    while let Some(t) = state_machine_update_notification_stream.next().await {
        console_log!("{:?}", t.unwrap());
    }

    Ok(())
}



#[wasm_bindgen_test]
async fn test_query_request_status_stream() -> Result<(), anyhow::Error> {
    // create source chain instance 
    


    // send post request to destination using source host


    // make the call to the `query_reqeust_status_stream`


    // assert <>::<>
    
    Ok(())
}


#[wasm_bindgen_test]
async fn test_query_request_status() -> Result<(), anyhow::Error> {

    Ok(())
}

#[wasm_bindgen_test]
async fn test_query_response_status() -> Result<(), anyhow::Error> {

    Ok(())
}


#[wasm_bindgen_test]
async fn test_timeout_request() -> Result<(), anyhow::Error> {

    Ok(())
}


#[wasm_bindgen_test]
async fn test_timeout_response() -> Result<(), anyhow::Error> {

    Ok(())
}

#[wasm_bindgen_test]
async fn test_subscribed_query_request_status() -> Result<(), anyhow::Error> {

    Ok(())
}