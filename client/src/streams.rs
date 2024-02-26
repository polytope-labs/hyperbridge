use ethers::prelude::H160;
use futures::stream;
use wasm_bindgen::JsValue;
use ismp::events::Event;
use ismp::router::{Post, Request};
use ismp::util::hash_request;
use crate::Keccak256;
use crate::providers::global::Client;
use crate::types::{ClientConfig, MessageStatus, PostStreamState};
use futures::StreamExt;
use gloo_timers::future::TimeoutFuture;



/// returns the query stream for a post
pub async fn query_request_status_stream(
    post: Post,
    config: ClientConfig,
    post_request_height: u64,
) -> Result<impl futures::Stream<Item = Result<JsValue, JsValue>>, anyhow::Error>
{
    let request_stream =
        stream::unfold(PostStreamState::Pending, move |mut post_request_status| {
            let config_inner = config.clone();
            let post_inner = post.clone();
            let req = Request::Post(post.clone());
            let hash = hash_request::<Keccak256>(&req);


            async move {
                let lambda  = || async {
                    let source_client = config_inner.source_chain().await?;
                    let dest_client = config_inner.dest_chain().await?;
                    let hyperbridge_client = config_inner.hyperbridge_client().await?;


                    match post_request_status {
                        PostStreamState::Pending => {
                            let destination_current_timestamp =
                                dest_client.host_timestamp().await?;
                            let relayer_address =
                                dest_client.query_request_receipts(&hash).await?;

                            return if relayer_address != H160::zero() {
                                // This means the message has gotten the destination chain
                                Ok::<Option<(Result<JsValue, JsValue>, PostStreamState)>, anyhow::Error>(Some((
                                    Ok(serde_wasm_bindgen::to_value(&MessageStatus::Destination).expect("Failed to serialize message status")),
                                    PostStreamState::DestinationFinalized,
                                )))
                            } else if destination_current_timestamp >= post.timeout_timestamp.into() {
                                // Checking to see if the messaging has timed-out
                                Ok(Some((
                                    Ok(serde_wasm_bindgen::to_value(&MessageStatus::Timeout).expect("Failed to serialize message status")),
                                    PostStreamState::DestinationFinalized,
                                )))
                            } else {
                                Ok(Some((Ok(serde_wasm_bindgen::to_value(&MessageStatus::Pending).expect("Failed to serialize message status")),
                                         PostStreamState::SourceFinalized)))
                            };
                        },
                        PostStreamState::SourceFinalized => {
                            let mut state_machine_updated_stream = hyperbridge_client
                                .state_machine_update_notification(
                                    source_client.state_machine_id()?,
                                )
                                .await?;

                            while let Some(item) = state_machine_updated_stream.next().await {
                                return match item {
                                    Ok(state_machine_update) => {
                                        if state_machine_update.latest_height >= post_request_height {
                                            Ok(Some((
                                                Ok(serde_wasm_bindgen::to_value(&MessageStatus::Hyperbridge).expect("Failed to serialize message status")),
                                                PostStreamState::HyperbridgeDelivered,
                                            )))
                                        } else {
                                            Ok(Some((
                                                Ok(serde_wasm_bindgen::to_value(&MessageStatus::Hyperbridge).expect("Failed to serialize message status")),
                                                PostStreamState::HyperbridgeDelivered,
                                            )))
                                        }
                                    },
                                    Err(e) => {
                                        Ok(None)
                                    },
                                };
                            }

                            Ok(None)
                        },
                        PostStreamState::HyperbridgeDelivered => {
                            let hyperbridge_request_response = hyperbridge_client
                                .query_request(&post_inner.source, &post_inner.dest, post_inner.nonce)
                                .await?;

                            if let Some(_) = hyperbridge_request_response.get(0) {
                                let hyperbridge_height = hyperbridge_client.client.blocks().at_latest().await?.number();

                                return Ok(Some((
                                    Ok(serde_wasm_bindgen::to_value(&MessageStatus::Hyperbridge).expect("Failed to serialize message status")),
                                    PostStreamState::HyperbridgeFinalized(hyperbridge_height as u64),
                                )));
                            } else {
                                let mut dest_request_stream = dest_client.event_stream().await?;

                                while let Some(event) = dest_request_stream.next().await {
                                    match event {
                                        Ok(state_machine_update) => match state_machine_update {
                                            Event::PostRequest(current_post) => {
                                                let current_post_req =
                                                    Request::Post(current_post.clone());
                                                let current_post_hash =
                                                    hash_request::<Keccak256>(&current_post_req);
                                                let hyperbridge_height = hyperbridge_client.client.blocks().at_latest().await?.number();


                                                if current_post_hash == hash {
                                                    return Ok(Some((
                                                        Ok(serde_wasm_bindgen::to_value(&MessageStatus::Destination).expect("Failed to serialize message status")),
                                                        PostStreamState::HyperbridgeFinalized(hyperbridge_height as u64),
                                                    )));
                                                } else {
                                                    continue;
                                                }
                                            },
                                            _ => {
                                                continue;
                                            },
                                        },
                                        Err(e) => return Ok(None),
                                    }
                                }
                            }

                            return Ok(None);
                        },
                        PostStreamState::HyperbridgeFinalized(at) => {
                            let mut dest_state_machine_update_stream = dest_client.state_machine_update_notification().await?;

                            while let Some(item) = dest_state_machine_update_stream.next().await {
                                match item {
                                    Ok(state_machine_update) => {
                                        if state_machine_update.latest_height >= at {
                                            return Ok(Some((
                                                // this should be HyperbridgeFinalized
                                                Ok(serde_wasm_bindgen::to_value(&MessageStatus::HyperbridgeFinalized).expect("Failed to serialize message status")),
                                                PostStreamState::DestinationDelivered,
                                            )));
                                        }
                                    },
                                    Err(e) => return Ok(None),
                                }
                            }

                            Ok(None)
                        },
                        PostStreamState::DestinationDelivered => {
                            // listen for post request handled event
                            let mut dest_request_stream =
                                dest_client.post_request_handled_stream().await?;

                            while let Some(event) = dest_request_stream.next().await {
                                match event {
                                    Ok(post_request_handled) => {
                                        if post_request_handled.commitment == hash.0 {
                                            return Ok(Some((
                                                Ok(serde_wasm_bindgen::to_value(&MessageStatus::Destination).expect("Failed to serialize message status")),
                                                PostStreamState::DestinationFinalized,
                                            )));
                                        }
                                    },
                                    Err(e) => return Ok(None),
                                }
                            }

                            Ok(None)
                        },
                        PostStreamState::DestinationFinalized =>
                            Ok::<Option<(Result<JsValue, JsValue>, PostStreamState)>, anyhow::Error>(None)
                    }
                };

                let response = lambda().await.expect("Failed to get stream response");
                response
            }
        });

    Ok(request_stream)
}


/// This function returns a stream that yields when the timeout
/// time of a request is reached
pub async fn timeout_stream(
    timeout: u64,
    client: impl Client + Clone,
) -> impl futures::Stream<Item = Result<JsValue, JsValue>>
{
    let state_machine_update_steam = stream::unfold(0u64, move |mut message_status| {
        let client_moved = client.clone();

        async move {
            let lambda = || async {
                let current_timestamp = client_moved.host_timestamp().await.expect("Failed to get the current timestamp");

                return if current_timestamp > timeout {
                    (MessageStatus::Timeout, current_timestamp)
                } else {
                    let sleep_time = (timeout - current_timestamp) * 1000;
                    TimeoutFuture::new(sleep_time as u32).await;


                    (MessageStatus::NotTimedOut, current_timestamp)
                };
            };

            let (response, time) = lambda().await;

            match response {
                MessageStatus::Timeout => {
                    None
                },
                MessageStatus::NotTimedOut => {
                    Some((Ok(serde_wasm_bindgen::to_value(&MessageStatus::NotTimedOut).unwrap()), time))
                },
                _ => {
                    None
                }
            }
        }
    });


    state_machine_update_steam
}