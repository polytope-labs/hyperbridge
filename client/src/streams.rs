use crate::{
    providers::global::{Client, RequestOrResponse},
    types::{BoxStreamJs, ClientConfig, HyperClientErrors, MessageStatus, PostStreamState},
    Keccak256,
};
use anyhow::{anyhow, Error};
use ethers::prelude::H160;
use futures::{stream, StreamExt};
use gloo_timers::future::TimeoutFuture;
use ismp::{
    router::{Post, Request},
    util::hash_request,
};
use wasm_bindgen::{JsError, JsValue};

/// returns the query stream for a post
pub async fn query_request_status_stream(
    post: Post,
    config: ClientConfig,
    post_request_height: u64,
) -> BoxStreamJs<JsValue> {
    let stream = stream::unfold(PostStreamState::Pending, move |post_request_status| {
        let config_inner = config.clone();
        let req = Request::Post(post.clone());
        let hash = hash_request::<Keccak256>(&req);
        let post = post.clone();
        async move {
            let lambda = || async {
                let source_client = config_inner.source_chain().await?;
                let dest_client = config_inner.dest_chain().await?;
                let hyperbridge_client = config_inner.hyperbridge_client().await?;

                match post_request_status {
                    PostStreamState::Pending => {
                        let destination_current_timestamp = dest_client.query_timestamp().await?;
                        let relayer_address = dest_client.query_request_receipt(hash).await?;

                        if relayer_address != H160::zero() {
                            // This means the message has gotten the destination chain
                            return Ok::<Option<(Result<JsValue, JsError>, PostStreamState)>, Error>(
                                Some((
                                    Ok(serde_wasm_bindgen::to_value(
                                        &MessageStatus::DestinationDelivered,
                                    )
                                    .expect("Failed to serialize message status")),
                                    PostStreamState::End,
                                )),
                            )
                        }

                        if destination_current_timestamp.as_secs() >= post.timeout_timestamp {
                            // Checking to see if the message has timed-out
                            return Ok(Some((
                                Ok(serde_wasm_bindgen::to_value(&MessageStatus::Timeout)
                                    .expect("Failed to serialize message status")),
                                PostStreamState::End,
                            )))
                        }

                        let mut state_machine_updated_stream = hyperbridge_client
                            .state_machine_update_notification(source_client.state_machine_id())
                            .await?;

                        while let Some(item) = state_machine_updated_stream.next().await {
                            match item {
                                Ok(state_machine_update) => {
                                    if state_machine_update.latest_height >= post_request_height &&
                                        state_machine_update.state_machine_id.state_id ==
                                            post.source
                                    {
                                        return Ok(Some((
                                            Ok(serde_wasm_bindgen::to_value(
                                                &MessageStatus::SourceFinalized,
                                            )
                                            .expect("Failed to serialize message status")),
                                            PostStreamState::SourceFinalized,
                                        )))
                                    }
                                },
                                Err(_) =>
                                    return Ok(Some((
                                        Err(JsError::new("stream encountered an error")),
                                        post_request_status,
                                    ))),
                            };
                        }

                        Ok(None)
                    },

                    PostStreamState::SourceFinalized => {
                        let hyperbridge_request_response =
                            hyperbridge_client.query_request(hash).await?;
                        if let Some(_) = hyperbridge_request_response {
                            let hyperbridge_height =
                                hyperbridge_client.client.blocks().at_latest().await?.number();

                            return Ok(Some((
                                Ok(serde_wasm_bindgen::to_value(
                                    &MessageStatus::HyperbridgeDelivered,
                                )
                                .expect("Failed to serialize message status")),
                                PostStreamState::HyperbridgeDelivered(hyperbridge_height.into()),
                            )));
                        }

                        let mut stream = hyperbridge_client
                            .ismp_events_stream(RequestOrResponse::Request(post.clone()))
                            .await?;
                        while let Some(event) = stream.next().await {
                            match event {
                                Ok(_) => {
                                    let hyperbridge_height = hyperbridge_client
                                        .client
                                        .blocks()
                                        .at_latest()
                                        .await?
                                        .number();
                                    return Ok(Some((
                                        Ok(serde_wasm_bindgen::to_value(
                                            &MessageStatus::HyperbridgeDelivered,
                                        )
                                        .expect("Failed to serialize message status")),
                                        PostStreamState::HyperbridgeDelivered(
                                            hyperbridge_height.into(),
                                        ),
                                    )));
                                },
                                Err(_) =>
                                    return Ok(Some((
                                        Err(JsError::new("stream encountered an error")),
                                        post_request_status,
                                    ))),
                            }
                        }

                        Ok(None)
                    },
                    PostStreamState::HyperbridgeDelivered(height) => {
                        let res = dest_client.query_request_receipt(hash).await?;
                        if res != H160::zero() {
                            return Ok(Some((
                                Ok(serde_wasm_bindgen::to_value(
                                    &MessageStatus::DestinationDelivered,
                                )
                                .expect("Failed to serialize message status")),
                                PostStreamState::End,
                            )));
                        }

                        let mut stream = dest_client
                            .state_machine_update_notification(
                                hyperbridge_client.state_machine_id(),
                            )
                            .await?;
                        while let Some(update) = stream.next().await {
                            match update {
                                Ok(event) =>
                                    if event.latest_height >= height {
                                        return Ok(Some((
                                            Ok(serde_wasm_bindgen::to_value(
                                                &MessageStatus::HyperbridgeFinalized,
                                            )
                                            .expect("Failed to serialize message status")),
                                            PostStreamState::HyperbridgeFinalized,
                                        )));
                                    } else {
                                        continue
                                    },
                                Err(_) =>
                                    return Ok(Some((
                                        Err(JsError::new("stream encountered an error")),
                                        post_request_status,
                                    ))),
                            }
                        }
                        Ok(None)
                    },
                    PostStreamState::HyperbridgeFinalized => {
                        let res = dest_client.query_request_receipt(hash).await?;
                        if res != H160::zero() {
                            return Ok(Some((
                                Ok(serde_wasm_bindgen::to_value(
                                    &MessageStatus::DestinationDelivered,
                                )
                                .expect("Failed to serialize message status")),
                                PostStreamState::DestinationDelivered,
                            )));
                        }
                        let mut stream = dest_client.post_request_handled_stream(hash).await?;

                        while let Some(event) = stream.next().await {
                            match event {
                                Ok(_) => {
                                    return Ok(Some((
                                        Ok(serde_wasm_bindgen::to_value(
                                            &MessageStatus::DestinationDelivered,
                                        )
                                        .expect("Failed to serialize message status")),
                                        PostStreamState::DestinationDelivered,
                                    )));
                                },
                                Err(_) =>
                                    return Ok(Some((
                                        Err(JsError::new("stream encountered an error")),
                                        post_request_status,
                                    ))),
                            }
                        }
                        Ok(None)
                    },

                    PostStreamState::DestinationDelivered | PostStreamState::End =>
                        Ok::<Option<(Result<JsValue, JsError>, PostStreamState)>, Error>(None),
                }
            };

            let response = lambda().await;
            match response {
                Ok(res) => res,
                Err(_) =>
                    Some((Err(JsError::new("Encountered an error in stream")), post_request_status)),
            }
        }
    });

    Box::pin(stream)
}

/// This function returns a stream that yields when the timeout
/// time of a request is reached
pub async fn timeout_stream(timeout: u64, client: impl Client + Clone) -> BoxStreamJs<JsValue> {
    let stream = stream::unfold((), move |_| {
        let client_moved = client.clone();

        async move {
            let lambda = || async {
                let current_timestamp = client_moved.query_timestamp().await?.as_secs();

                return if current_timestamp > timeout {
                    Ok(MessageStatus::Timeout)
                } else {
                    let sleep_time = (timeout - current_timestamp) * 1000;
                    TimeoutFuture::new(sleep_time as u32).await;
                    Ok::<_, Error>(MessageStatus::NotTimedOut)
                };
            };

            loop {
                let response = lambda().await;

                let value = match response {
                    Ok(MessageStatus::Timeout) => Some((
                        Ok(serde_wasm_bindgen::to_value(&MessageStatus::Timeout)
                            .expect("Infallible")),
                        (),
                    )),
                    Ok(MessageStatus::NotTimedOut) => continue,
                    _ => Some((Err(JsError::new("stream encountered an error")), ())),
                };

                return value
            }
        }
    });

    Box::pin(stream)
}
