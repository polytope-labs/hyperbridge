mod internals;
mod mock;
mod providers;
mod runtime;
mod streams;
mod types;

#[cfg(test)]
mod tests;

extern crate alloc;

use crate::{
    internals::{query_request_status_internal, query_response_status_internal},
    providers::global::Client,
    streams::{query_request_status_stream, timeout_stream},
    types::ClientConfig,
};
use codec::Encode;
use ethers::{prelude::Middleware, types::H256, utils::keccak256};
use futures::{stream, StreamExt};
use ismp::router::{Post, PostResponse};
use std::future::Future;
use subxt::ext::codec;
use wasm_bindgen::prelude::*;
use wasm_streams::ReadableStream;

// Functions takes in a post request and returns the status of the request
// (Pending, Destination, Hyperbridge, Timeout)
#[wasm_bindgen]
pub async fn query_request_status(
    request: JsValue,
    config_js: JsValue,
) -> Result<JsValue, JsError> {
    let post: Post = serde_wasm_bindgen::from_value(request)
        .map_err(|_| JsError::new("deserialization error"))?;
    let config: ClientConfig = serde_wasm_bindgen::from_value(config_js)
        .map_err(|_| JsError::new("deserialization error"))?;

    let response = query_request_status_internal(post, config)
        .await
        .map_err(|e| JsError::new(e.to_string().as_str()))?;

    serde_wasm_bindgen::to_value(&response).map_err(|_| JsError::new("deserialization error"))
}

// Function takes in a post response and returns the status of the response
// (Pending, Destination, Hyperbridge, Timeout)
#[wasm_bindgen]
pub async fn query_response_status(
    response: JsValue,
    config_js: JsValue,
) -> Result<JsValue, JsError> {
    let post_response: PostResponse = serde_wasm_bindgen::from_value(response)
        .map_err(|_| JsError::new("deserialization error"))?;
    let config: ClientConfig = serde_wasm_bindgen::from_value(config_js)
        .map_err(|_| JsError::new("deserialization error"))?;
    let response = query_response_status_internal(config, post_response)
        .await
        .map_err(|e| JsError::new(e.to_string().as_str()))?;

    serde_wasm_bindgen::to_value(&response).map_err(|_| JsError::new("deserialization error"))
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
) -> Result<wasm_streams::readable::sys::ReadableStream, JsError> {
    let post: Post = serde_wasm_bindgen::from_value(request)
        .map_err(|_| JsError::new("deserialization error"))?;
    let config: ClientConfig = serde_wasm_bindgen::from_value(config_js)
        .map_err(|_| JsError::new("deserialization error"))?;
    let source_chain =
        config.source_chain().await.map_err(|e| JsError::new(e.to_string().as_str()))?;

    let stream = stream::unfold((), move |_| {
        let source_chain = source_chain.clone();
        let post = post.clone();
        let config = config.clone();
        async move {
            // Obtaining the request stream and the timeout stream
            let mut timed_out = timeout_stream(post.timeout_timestamp, source_chain).await;
            let mut request_status =
                query_request_status_stream(post, config, post_request_height).await;
            loop {
                tokio::select! {
                    result = timed_out.next() => {
                        return result.map(|val| (val.map(|status| serde_wasm_bindgen::to_value(&status).expect("Infallible")).map_err(|e| JsValue::from_str(alloc::format!("{e:?}").as_str())), ()))
                    }
                    result = request_status.next() => {
                        return result.map(|val| (val.map(|status| serde_wasm_bindgen::to_value(&status).expect("Infallible")).map_err(|e| JsValue::from_str(alloc::format!("{e:?}").as_str())), ()))
                    }
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
