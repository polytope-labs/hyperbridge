pub mod internals;
pub mod providers;
pub mod runtime;
pub mod types;

pub mod interfaces;

extern crate alloc;
extern crate core;

use crate::{
    internals::{query_request_status_internal, query_response_status_internal},
    types::ClientConfig,
};

use crate::{
    interfaces::{JsClientConfig, JsPost, JsResponse},
    internals::timeout_request_stream,
};
use ethers::{types::H256, utils::keccak256};
use futures::{stream, StreamExt};
use ismp::router::{Post, PostResponse};
use wasm_bindgen::prelude::*;
use wasm_streams::ReadableStream;

/// Accepts a post request and returns a `MessageStatus` where
/// type MessageStatus =  SourceFinalized | HyperbridgeDelivered | HyperbridgeFinalized |
/// DestinationDelivered | Timeout;
///
/// // This event is emitted on hyperbridge
/// interface SourceFinalized {
///     kind: "SourceFinalized";
/// }
///
/// // This event is emitted on hyperbridge
/// interface HyperbridgeDelivered {
///     kind: "HyperbridgeDelivered";
/// }
///
/// // This event is emitted on the destination chain
/// interface HyperbridgeFinalized {
///     kind: "HyperbridgeFinalized";
/// }
///
/// // This event is emitted on the destination chain
/// interface DestinationDelivered {
///     kind: "DestinationDelivered";
/// }
///
/// // The request has now timed out
/// interface Timeout {
///     kind: "Timeout";
/// }
#[wasm_bindgen]
pub async fn query_request_status(
    request: JsPost,
    config: JsClientConfig,
) -> Result<JsValue, JsError> {
    let post: Post = request.try_into().map_err(|_| JsError::new("deserialization error"))?;
    let config: ClientConfig =
        config.try_into().map_err(|_| JsError::new("deserialization error"))?;

    let response = query_request_status_internal(post, config)
        .await
        .map_err(|e| JsError::new(e.to_string().as_str()))?;

    serde_wasm_bindgen::to_value(&response).map_err(|_| JsError::new("deserialization error"))
}

/// Accepts a post response and returns a `MessageStatus` where
/// type MessageStatus =  SourceFinalized | HyperbridgeDelivered | HyperbridgeFinalized |
/// DestinationDelivered | Timeout;
///
/// // This event is emitted on hyperbridge
/// interface SourceFinalized {
///     kind: "SourceFinalized";
/// }
///
/// // This event is emitted on hyperbridge
/// interface HyperbridgeDelivered {
///     kind: "HyperbridgeDelivered";
/// }
///
/// // This event is emitted on the destination chain
/// interface HyperbridgeFinalized {
///     kind: "HyperbridgeFinalized";
/// }
///
/// // This event is emitted on the destination chain
/// interface DestinationDelivered {
///     kind: "DestinationDelivered";
/// }
///
/// // The request has now timed out
/// interface Timeout {
///     kind: "Timeout";
/// }
#[wasm_bindgen]
pub async fn query_response_status(
    response: JsResponse,
    config: JsClientConfig,
) -> Result<JsValue, JsError> {
    let post_response: PostResponse =
        response.try_into().map_err(|_| JsError::new("deserialization error"))?;
    let config: ClientConfig =
        config.try_into().map_err(|_| JsError::new("deserialization error"))?;
    let response = query_response_status_internal(config, post_response)
        .await
        .map_err(|e| JsError::new(e.to_string().as_str()))?;

    serde_wasm_bindgen::to_value(&response).map_err(|_| JsError::new("deserialization error"))
}

/// Accepts a post request that has timed out returns a `ReadableStream` that yields a
/// `TimeoutStatus` where type TimeoutStatus =  DestinationFinalized | HyperbridgeTimedout |
/// HyperbridgeFinalized | TimeoutMessage;
///
/// // This event is emitted on hyperbridge
/// interface DestinationFinalized {
///     kind: "DestinationFinalized";
///     // The hash of the block where the event was emitted
///     block_hash: H256,
///     // The hash of the extrinsic responsible for the event
///     transaction_hash: H256,
///     // The block number where the event was emitted
///     block_number: u64,
/// }
///
/// // This event is emitted on hyperbridge
/// interface HyperbridgeTimedout {
///     kind: "HyperbridgeTimedout";
///     // The hash of the block where the event was emitted
///     block_hash: H256,
///     // The hash of the extrinsic responsible for the event
///     transaction_hash: H256,
///     // The block number where the event was emitted
///     block_number: u64,
/// }
///
/// // This event is emitted on the source chain
/// interface HyperbridgeFinalized {
///     kind: "HyperbridgeFinalized";
///     // The hash of the block where the event was emitted
///     block_hash: H256,
///     // The hash of the extrinsic responsible for the event
///     transaction_hash: H256,
///     // The block number where the event was emitted
///     block_number: u64,
/// }
///
/// // This event is emitted on the destination chain
/// interface TimeoutMessage {
///     kind: "TimeoutMessage";
///     // encoded call for HandlerV1.handlePostRequestTimeouts
///     calldata: Vec<u8>,
/// }
///
/// This function will not check if the request has timed out, only call it when sure that the
/// request has timed out after calling `query_request_status`
#[wasm_bindgen]
pub async fn timeout_post_request(
    request: JsPost,
    config: JsClientConfig,
) -> Result<wasm_streams::readable::sys::ReadableStream, JsError> {
    let post: Post = request.try_into().map_err(|_| JsError::new("deserialization error"))?;
    let config: ClientConfig =
        config.try_into().map_err(|_| JsError::new("deserialization error"))?;

    let stream = timeout_request_stream(post, config)
        .await
        .map_err(|e| JsError::new(e.to_string().as_str()))?
        .map(|value| {
            value
                .map(|status| serde_wasm_bindgen::to_value(&status).expect("Infallible"))
                .map_err(|e| JsValue::from_str(alloc::format!("{e:?}").as_str()))
        });

    let js_stream = ReadableStream::from_stream(stream);
    Ok(js_stream.into_raw())
}

// =====================================
// Stream Functions
// =====================================

/// Accepts a PostRequest and returns a `ReadableStream` that yields a `MessageStatus` where:
/// type MessageStatus =  SourceFinalized | HyperbridgeDelivered | HyperbridgeFinalized |
/// DestinationDelivered | Timeout;
///
/// // This event is emitted on hyperbridge
/// interface SourceFinalized {
///     kind: "SourceFinalized";
///     // The hash of the block where the event was emitted
///     block_hash: H256,
///     // The hash of the extrinsic responsible for the event
///     transaction_hash: H256,
///     // The block number where the event was emitted
///     block_number: u64,
/// }
///
/// // This event is emitted on hyperbridge
/// interface HyperbridgeDelivered {
///     kind: "HyperbridgeDelivered";
///     // The hash of the block where the event was emitted
///     block_hash: H256,
///     // The hash of the extrinsic responsible for the event
///     transaction_hash: H256,
///     // The block number where the event was emitted
///     block_number: u64,
/// }
///
/// // This event is emitted on the destination chain
/// interface HyperbridgeFinalized {
///     kind: "HyperbridgeFinalized";
///     // The hash of the block where the event was emitted
///     block_hash: H256,
///     // The hash of the extrinsic responsible for the event
///     transaction_hash: H256,
///     // The block number where the event was emitted
///     block_number: u64,
/// }
///
/// // This event is emitted on the destination chain
/// interface DestinationDelivered {
///     kind: "DestinationDelivered";
///     // The hash of the block where the event was emitted
///     block_hash: H256,
///     // The hash of the extrinsic responsible for the event
///     transaction_hash: H256,
///     // The block number where the event was emitted
///     block_number: u64,
/// }
///
/// // The request has now timed out
/// interface Timeout {
///     kind: "Timeout";
/// }
#[wasm_bindgen]
pub async fn request_status_stream(
    request: JsPost,
    config: JsClientConfig,
) -> Result<wasm_streams::readable::sys::ReadableStream, JsError> {
    let post: Post = request
        .clone()
        .try_into()
        .map_err(|_err| JsError::new("deserialization error: {_err:?}"))?;
    let config: ClientConfig = config
        .try_into()
        .map_err(|_err| JsError::new("deserialization error: {_err:?}"))?;
    let source_client =
        config.source_chain().await.map_err(|e| JsError::new(e.to_string().as_str()))?;
    let dest_client =
        config.dest_chain().await.map_err(|e| JsError::new(e.to_string().as_str()))?;
    let hyperbridge_client = config
        .hyperbridge_client()
        .await
        .map_err(|e| JsError::new(e.to_string().as_str()))?;

    let stream = stream::unfold((), move |_| {
        let dest_client = dest_client.clone();
        let hyperbridge_client = hyperbridge_client.clone();
        let source_client = source_client.clone();
        let post = post.clone();
        async move {
            // Obtaining the request stream and the timeout stream
            let mut timed_out =
                internals::request_timeout_stream(post.timeout_timestamp, source_client.clone())
                    .await;
            let mut request_status = internals::request_status_stream(
                post,
                source_client.clone(),
                dest_client,
                hyperbridge_client,
                request.height,
            )
            .await;

            tokio::select! {
                result = timed_out.next() => {
                    return result.map(|val| (val.map(|status| serde_wasm_bindgen::to_value(&status).expect("Infallible")).map_err(|e| JsValue::from_str(alloc::format!("{e:?}").as_str())), ()))
                }
                result = request_status.next() => {
                    return result.map(|val| (val.map(|status| serde_wasm_bindgen::to_value(&status).expect("Infallible")).map_err(|e| JsValue::from_str(alloc::format!("{e:?}").as_str())), ()))
                }
            }
        }
    });

    // Wrapping the main stream in a readable stream
    let js_stream = ReadableStream::from_stream(stream);

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
