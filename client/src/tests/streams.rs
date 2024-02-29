#![cfg(target_arch = "wasm32")]

use crate::{
    providers::{evm_chain::EvmClient, global::Client, hyperbridge::HyperBridgeClient},
    streams::timeout_stream,
};
use ethers::{
    prelude::{Provider, Ws},
    providers::{Http, ProviderExt},
    types::H160,
};
use futures::StreamExt;
use ismp::{
    consensus::{ConsensusStateId, StateMachineId},
    host::StateMachine,
};
use std::{str::FromStr, sync::Arc};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_subscribed_request_status() -> Result<(), anyhow::Error> {
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
