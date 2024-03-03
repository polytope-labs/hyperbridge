mod internals;
#[cfg(test)]
mod mock;
mod providers;
mod runtime;
mod streams;
mod types;

mod interfaces;
#[cfg(test)]
mod tests;

extern crate alloc;
extern crate core;

use crate::{
    internals::{query_request_status_internal, query_response_status_internal},
    streams::{query_request_status_stream, timeout_stream},
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

/// Functions takes in a post request and returns one of the following json strings variants
/// Status variants: `Pending`, `SourceFinalized`, `HyperbridgeDelivered`, `HyperbridgeFinalized`,
/// `DestinationDelivered`, `Timeout`
#[wasm_bindgen]
pub async fn query_request_status(
    request: JsPost,
    config_js: JsClientConfig,
) -> Result<JsValue, JsError> {
    let post: Post = request.try_into().map_err(|_| JsError::new("deserialization error"))?;
    let config: ClientConfig =
        config_js.try_into().map_err(|_| JsError::new("deserialization error"))?;

    let response = query_request_status_internal(post, config)
        .await
        .map_err(|e| JsError::new(e.to_string().as_str()))?;

    serde_wasm_bindgen::to_value(&response).map_err(|_| JsError::new("deserialization error"))
}

/// Function takes in a post response and returns one of the following json strings variants
/// Status Variants: `Pending`, `SourceFinalized`, `HyperbridgeDelivered`, `HyperbridgeFinalized`,
/// `DestinationDelivered`, `Timeout`
#[wasm_bindgen]
pub async fn query_response_status(
    response: JsResponse,
    config_js: JsClientConfig,
) -> Result<JsValue, JsError> {
    let post_response: PostResponse =
        response.try_into().map_err(|_| JsError::new("deserialization error"))?;
    let config: ClientConfig =
        config_js.try_into().map_err(|_| JsError::new("deserialization error"))?;
    let response = query_response_status_internal(config, post_response)
        .await
        .map_err(|e| JsError::new(e.to_string().as_str()))?;

    serde_wasm_bindgen::to_value(&response).map_err(|_| JsError::new("deserialization error"))
}

/// Accepts a post request that has timed out returns a stream that yields the following json
/// strings variants Status Variants: `Pending`, `DestinationFinalized`, `HyperbridgeTimedout`,
/// `HyperbridgeFinalized`, `{ "TimeoutMessage": [...] }`. This function will not check if the
/// request has timed out, only call it when sure that the request has timed out after calling
/// `query_request_status`
#[wasm_bindgen]
pub async fn timeout_post_request(
    request: JsPost,
    config_js: JsClientConfig,
) -> Result<wasm_streams::readable::sys::ReadableStream, JsError> {
    let post: Post = request.try_into().map_err(|_| JsError::new("deserialization error"))?;
    let config: ClientConfig =
        config_js.try_into().map_err(|_| JsError::new("deserialization error"))?;

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

/// Races between a timeout stream and request processing stream, and yields the following json
/// strings variants Status Variants: `Pending`, `SourceFinalized`, `HyperbridgeDelivered`,
/// `HyperbridgeFinalized`, `DestinationDelivered`, `Timeout`
#[wasm_bindgen]
pub async fn subscribe_to_request_status(
    request: JsPost,
    config_js: JsClientConfig,
    post_request_height: u64,
) -> Result<wasm_streams::readable::sys::ReadableStream, JsError> {
    let post: Post = request.try_into().map_err(|_| JsError::new("deserialization error"))?;
    let config: ClientConfig =
        config_js.try_into().map_err(|_| JsError::new("deserialization error"))?;
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
            let mut timed_out = timeout_stream(post.timeout_timestamp, source_client.clone()).await;
            let mut request_status = query_request_status_stream(
                post,
                source_client.clone(),
                dest_client,
                hyperbridge_client,
                post_request_height,
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
