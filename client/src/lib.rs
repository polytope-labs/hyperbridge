mod providers;
mod runtime;
mod types;
mod internals;
mod streams;
mod mock;


#[cfg(test)]
mod tests;


use std::future::Future;
use crate::{
    providers::{
        global::Client,
        hyperbridge::{get_request_storage_key, get_response_storage_key},
    },
    types::{
        ClientConfig, HyperClientErrors, MessageStatus, PostStreamState, ReturnRequestTimeoutData,
        ReturnRequestTimeoutMessage, ReturnResponseTimeoutData, ReturnResponseTimeoutMessage,
    },
};
use codec::Encode;
use ethers::{
    middleware::Middleware,
    types::{Address, H160, H256},
    utils::keccak256,
};
use futures::{stream, StreamExt};
use ismp::{
    consensus::StateMachineHeight,
    events::Event,
    messaging::{Message, TimeoutMessage},
    router::{Post, PostResponse, Request},
    util::{hash_post_response, hash_request},
};
use std::time::Duration;
use subxt::ext::codec;
use wasm_bindgen::prelude::*;
use wasm_streams::ReadableStream;
use crate::internals::{query_request_status_internal, query_response_status_internal, timeout_request_internal};
use crate::streams::{query_request_status_stream, timeout_stream};


// Functions takes in a post request and returns the status of the request
// (Pending, Destination, Hyperbridge, Timeout)
#[wasm_bindgen]
pub async fn query_request_status(
    request: JsValue,
    config_js: JsValue,
) -> Result<JsValue, JsValue>
{
    let post: Post = serde_wasm_bindgen::from_value(request)?;
    let config: ClientConfig = serde_wasm_bindgen::from_value(config_js)?;

    let response = query_request_status_internal(post, config).await.expect("Could not get request status");

    Ok(serde_wasm_bindgen::to_value(&response)?)
}



// Function takes in a post response and returns the status of the response
// (Pending, Destination, Hyperbridge, Timeout)
#[wasm_bindgen]
pub async fn query_response_status(
    response: JsValue,
    config_js: JsValue,
) -> Result<JsValue, JsValue>
{
    let post_response: PostResponse = serde_wasm_bindgen::from_value(response)?;
    let config: ClientConfig = serde_wasm_bindgen::from_value(config_js)?;
    let dest_client = config.dest_chain().await.map_err(|_| {
        serde_wasm_bindgen::to_value(&HyperClientErrors::FailedToCreateDestClient).expect("Failed to parse error message")
    })?;
    let hyperbridge_client = config.hyperbridge_client().await.map_err(|_| {
        serde_wasm_bindgen::to_value(&HyperClientErrors::FailedToCreateHyperbridgeClient).expect("Failed to parse error message")
    })?;
    let hash = hash_post_response::<Keccak256>(&post_response);


    let response = query_response_status_internal(dest_client, hyperbridge_client, hash, post_response).await.expect("Could not get response status");

    Ok(serde_wasm_bindgen::to_value(&response)?)
}


#[wasm_bindgen]
pub async fn timeout_request(
    post_request: JsValue,
    config_js: JsValue,
) -> Result<JsValue, JsValue>
{
    let post: Post = serde_wasm_bindgen::from_value(post_request)?;
    let config: ClientConfig = serde_wasm_bindgen::from_value(config_js)?;

    // setting up the clients
    let dest_client = config.dest_chain().await.map_err(|_| {
        serde_wasm_bindgen::to_value(&HyperClientErrors::FailedToCreateDestClient).unwrap()
    })?;
    let hyperbridge_client = config.hyperbridge_client().await.map_err(|_| {
        serde_wasm_bindgen::to_value(&HyperClientErrors::FailedToCreateHyperbridgeClient).unwrap()
    })?;
    let source_client = config.source_chain().await.map_err(|_| {
        serde_wasm_bindgen::to_value(&HyperClientErrors::FailedToCreateSourceClient).unwrap()
    })?;

    let response = timeout_request_internal(post, source_client, dest_client, hyperbridge_client, config).await.expect("Could not get request status");

    Ok(serde_wasm_bindgen::to_value(&response)?)
}


#[wasm_bindgen]
pub async fn timeout_response(
    post_response: JsValue,
    config_js: JsValue,
) -> Result<JsValue, JsValue>
{
    let post_response: PostResponse = serde_wasm_bindgen::from_value(post_response)?;
    let config: ClientConfig = serde_wasm_bindgen::from_value(config_js)?;
    let dest_client = config.dest_chain().await.map_err(|_| {
        serde_wasm_bindgen::to_value(&HyperClientErrors::FailedToCreateDestClient).expect("Failed to parse error")
    })?;
    let hyperbridge_client = config.hyperbridge_client().await.map_err(|_| {
        serde_wasm_bindgen::to_value(&HyperClientErrors::FailedToCreateHyperbridgeClient).expect("Failed to parse error")
    })?;
    let source_client = config.source_chain().await.map_err(|_| {
        serde_wasm_bindgen::to_value(&HyperClientErrors::FailedToCreateSourceClient).expect("Failed to parse error")
    })?;

    let hash = hash_post_response::<Keccak256>(&post_response);


    let dest_state_machine_id = dest_client.state_machine_id().unwrap();
    let source_state_machine_id = source_client.state_machine_id().unwrap();

    let hyper_bridge_response = hyperbridge_client
        .query_response(
            &post_response.source_chain(),
            &post_response.dest_chain(),
            post_response.nonce(),
        )
        .await
        .unwrap();

    if let Some(_) = hyper_bridge_response.get(0) {
        let dest_current_block_time = dest_client.host_timestamp().await.unwrap();

        if !post_response.timed_out(Duration::from_secs(dest_current_block_time)) {
            return Err(serde_wasm_bindgen::to_value(
                &HyperClientErrors::ResponseIsNotDueForTimeOut,
            )?);
        }

        let mut dest_state_machine_update_stream = hyperbridge_client
            .state_machine_update_notification(dest_state_machine_id.clone())
            .await
            .unwrap();

        let mut timeout_response;

        while let Some(item) = dest_state_machine_update_stream.next().await {
            match item {
                Ok(state_machine_update) => {
                    let current_state_machine_height = StateMachineHeight {
                        height: state_machine_update.latest_height,
                        id: state_machine_update.state_machine_id,
                    };

                    let state_machine_commitment = hyperbridge_client
                        .query_state_machine_commitment(current_state_machine_height)
                        .await
                        .unwrap();

                    if state_machine_commitment.timestamp <= post_response.timeout().as_secs() {
                        let proof = dest_client
                            .query_response_proof(&hash, state_machine_update.latest_height)
                            .await
                            .unwrap();

                        let timeout_message = Message::Timeout(TimeoutMessage::PostResponse {
                            responses: vec![post_response.clone()],
                            timeout_proof: proof.clone(),
                        });

                        timeout_response =
                            hyperbridge_client.send_message(proof, timeout_message).await.unwrap();
                        break;
                    }
                },
                Err(e) => {},
            }
        }

        let mut source_state_machine_update_stream = hyperbridge_client
            .state_machine_update_notification(source_state_machine_id.clone())
            .await
            .unwrap();

        while let Some(source_stream_item) = source_state_machine_update_stream.next().await {
            match source_stream_item {
                Ok(source_state_machine_update) => {
                    let source_current_state_machine_height = StateMachineHeight {
                        height: source_state_machine_update.latest_height,
                        id: source_state_machine_update.state_machine_id,
                    };

                    let state_machine_commitment = hyperbridge_client
                        .query_state_machine_commitment(source_current_state_machine_height)
                        .await
                        .unwrap();

                    if state_machine_commitment.timestamp <= post_response.timeout().as_secs() {
                        let request_key = get_response_storage_key(Vec::from(hash.0));
                        let proof_from_hyperbridge = hyperbridge_client
                            .get_state_proof(
                                source_state_machine_update.latest_height,
                                vec![request_key],
                            )
                            .await
                            .unwrap();

                        let timeout_return_message = ReturnResponseTimeoutMessage {
                            timeouts: vec![post_response.clone()],
                            height: source_current_state_machine_height,
                            proof: vec![proof_from_hyperbridge],
                        };

                        let timeout_data = ReturnResponseTimeoutData {
                            host: config.source_ismp_host_address,
                            post_response_timeout_message: timeout_return_message,
                        };

                        return Ok(serde_wasm_bindgen::to_value(&timeout_data)?);
                    }
                },
                Err(e) => {},
            }
        }
    }

    Ok(serde_wasm_bindgen::to_value(&MessageStatus::Pending)?)
}







// =====================================
// Stream Functions
// =====================================


/// This function is a subscribed version of the query_request_status_stream and
/// yield timeout if timeout was reached first
#[wasm_bindgen]
pub async fn subscribed_query_request_status(
    request: JsValue,
    config_js: JsValue,
    post_request_height: u64,
) -> Result<wasm_streams::readable::sys::ReadableStream, JsValue>
{
    let post: Post = serde_wasm_bindgen::from_value(request)?;
    let config: ClientConfig = serde_wasm_bindgen::from_value(config_js)?;


    let lambda =  || async {
        let source_chain = config.source_chain().await?;
        // Obtaining the request stream and the timeout stream
        let timed_out =
            timeout_stream(post.timeout_timestamp, source_chain).await;
        let request_status =
            query_request_status_stream(post, config, post_request_height).await?;


        // Merging the two streams and returns the first stream to yield
        Ok::<_, anyhow::Error>(stream::select(request_status, timed_out))
    };


    let main_stream = lambda().await.expect("Failed to select stream");

    // Wrapping the main stream in a readable stream
    let js_stream = ReadableStream::from_stream(main_stream);

    Ok(js_stream.into_raw())
}



































#[derive(Clone, Default)]
pub struct Keccak256;

impl ismp::util::Keccak256 for Keccak256 {
    fn keccak256(bytes: &[u8]) -> H256
    where
        Self: Sized,
    {
        keccak256(bytes).into()
    }
}