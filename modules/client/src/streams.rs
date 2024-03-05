use crate::{
    providers::global::{Client, RequestOrResponse},
    types::{BoxStream, MessageStatus, PostStreamState},
    Keccak256,
};
use anyhow::{anyhow, Error};
use ethers::prelude::H160;
use futures::{stream, StreamExt};
use ismp::{
    router::{Post, Request},
    util::hash_request,
};
use std::time::Duration;

/// returns the query stream for a post
pub async fn query_request_status_stream(
    post: Post,
    source_client: impl Client,
    dest_client: impl Client,
    hyperbridge_client: impl Client,
    post_request_height: u64,
) -> BoxStream<MessageStatus> {
    let stream = stream::unfold(PostStreamState::Pending, move |post_request_status| {
        let dest_client = dest_client.clone();
        let hyperbridge_client = hyperbridge_client.clone();
        let source_client = source_client.clone();
        let req = Request::Post(post.clone());
        let hash = hash_request::<Keccak256>(&req);
        let post = post.clone();
        async move {
            let lambda = || async {
                match post_request_status {
                    PostStreamState::Pending => {
                        let destination_current_timestamp = dest_client.query_timestamp().await?;
                        let relayer_address = dest_client.query_request_receipt(hash).await?;

                        if relayer_address != H160::zero() {
                            // This means the message has gotten to the destination chain
                            return Ok::<Option<(Result<_, Error>, PostStreamState)>, Error>(Some((
                                Ok(MessageStatus::DestinationDelivered),
                                PostStreamState::End,
                            )))
                        }

                        if destination_current_timestamp.as_secs() >= post.timeout_timestamp {
                            // Checking to see if the message has timed-out
                            return Ok(Some((Ok(MessageStatus::Timeout), PostStreamState::End)))
                        }

                        let hyperbridge_current_timestamp =
                            hyperbridge_client.query_timestamp().await?;
                        let relayer = hyperbridge_client.query_request_receipt(hash).await?;

                        if relayer != H160::zero() {
                            // This means the message has gotten to the destination chain
                            return Ok::<Option<(Result<_, Error>, PostStreamState)>, Error>(Some((
                                Ok(MessageStatus::HyperbridgeDelivered),
                                PostStreamState::HyperbridgeDelivered(
                                    hyperbridge_client.query_latest_block_height().await?,
                                ),
                            )))
                        }

                        if hyperbridge_current_timestamp.as_secs() >= post.timeout_timestamp {
                            // Checking to see if the message has timed-out
                            return Ok(Some((Ok(MessageStatus::Timeout), PostStreamState::End)))
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
                                            Ok(MessageStatus::SourceFinalized),
                                            PostStreamState::SourceFinalized,
                                        )))
                                    }
                                },
                                Err(e) =>
                                    return Ok(Some((
                                        Err(anyhow!(
                                            "Encountered an error {:?}: in {:?}",
                                            PostStreamState::Pending,
                                            e
                                        )),
                                        post_request_status,
                                    ))),
                            };
                        }

                        Ok(None)
                    },

                    PostStreamState::SourceFinalized => {
                        let hyperbridge_request_response =
                            hyperbridge_client.query_request_receipt(hash).await?;
                        if hyperbridge_request_response != H160::zero() {
                            let hyperbridge_height =
                                hyperbridge_client.query_latest_block_height().await?;

                            return Ok(Some((
                                Ok(MessageStatus::HyperbridgeDelivered),
                                PostStreamState::HyperbridgeDelivered(hyperbridge_height.into()),
                            )));
                        }

                        let mut stream = hyperbridge_client
                            .ismp_events_stream(RequestOrResponse::Request(post.clone()))
                            .await?;
                        while let Some(event) = stream.next().await {
                            match event {
                                Ok(_) => {
                                    let hyperbridge_height =
                                        hyperbridge_client.query_latest_block_height().await?;
                                    return Ok(Some((
                                        Ok(MessageStatus::HyperbridgeDelivered),
                                        PostStreamState::HyperbridgeDelivered(
                                            hyperbridge_height.into(),
                                        ),
                                    )));
                                },
                                Err(e) =>
                                    return Ok(Some((
                                        Err(anyhow!(
                                            "Encountered an error {:?}: in {:?}",
                                            PostStreamState::SourceFinalized,
                                            e
                                        )),
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
                                Ok(MessageStatus::DestinationDelivered),
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
                                            Ok(MessageStatus::HyperbridgeFinalized),
                                            PostStreamState::HyperbridgeFinalized,
                                        )));
                                    } else {
                                        continue
                                    },
                                Err(e) =>
                                    return Ok(Some((
                                        Err(anyhow!(
                                            "Encountered an error {:?}: in {:?}",
                                            PostStreamState::HyperbridgeDelivered(height),
                                            e
                                        )),
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
                                Ok(MessageStatus::DestinationDelivered),
                                PostStreamState::DestinationDelivered,
                            )));
                        }
                        let mut stream = dest_client.post_request_handled_stream(hash).await?;

                        while let Some(event) = stream.next().await {
                            match event {
                                Ok(_) => {
                                    return Ok(Some((
                                        Ok(MessageStatus::DestinationDelivered),
                                        PostStreamState::DestinationDelivered,
                                    )));
                                },
                                Err(e) =>
                                    return Ok(Some((
                                        Err(anyhow!(
                                            "Encountered an error {:?}: in {:?}",
                                            PostStreamState::HyperbridgeFinalized,
                                            e
                                        )),
                                        post_request_status,
                                    ))),
                            }
                        }
                        Ok(None)
                    },

                    PostStreamState::DestinationDelivered | PostStreamState::End =>
                        Ok::<Option<(Result<_, _>, PostStreamState)>, Error>(None),
                }
            };

            let response = lambda().await;
            match response {
                Ok(res) => res,
                Err(e) => Some((
                    Err(anyhow!("Encountered an error in stream {e:?}")),
                    post_request_status,
                )),
            }
        }
    });

    Box::pin(stream)
}

/// This function returns a stream that yields when the timeout
/// time of a request is reached
pub async fn timeout_stream(timeout: u64, client: impl Client + Clone) -> BoxStream<MessageStatus> {
    let stream = stream::unfold((), move |_| {
        let client_moved = client.clone();

        async move {
            let lambda = || async {
                let current_timestamp = client_moved.query_timestamp().await?.as_secs();

                return if current_timestamp > timeout {
                    Ok(true)
                } else {
                    let sleep_time = timeout - current_timestamp;
                    let _ = wasm_timer::Delay::new(Duration::from_secs(sleep_time)).await;
                    Ok::<_, Error>(false)
                };
            };

            loop {
                let response = lambda().await;

                let value = match response {
                    Ok(val) if val => Some((Ok(MessageStatus::Timeout), ())),
                    Ok(val) if !val => continue,
                    Err(e) =>
                        Some((Err(anyhow!("Encountered an error in timeout stream: {:?}", e)), ())),
                    _ => None,
                };

                return value
            }
        }
    });

    Box::pin(stream)
}
