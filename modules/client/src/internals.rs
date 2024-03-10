use crate::{
    providers::interface::{wait_for_challenge_period, Client},
    types::{BoxStream, ClientConfig, MessageStatus, TimeoutStatus},
    Keccak256,
};
use anyhow::anyhow;
///! This module contains the internal implementation of HyperClient.
use ethers::prelude::H160;
use futures::{stream, StreamExt};
use ismp::{
    consensus::StateMachineHeight,
    messaging::{Message, Proof, TimeoutMessage},
    router::{Post, PostResponse, Request, Response},
    util::hash_request,
};

/// `query_request_status_internal` is an internal function that
/// checks the status of a message
pub async fn query_request_status_internal(
    post: Post,
    config: ClientConfig,
) -> Result<MessageStatus, anyhow::Error> {
    let dest_client = config.dest_chain().await?;
    let hyperbridge_client = config.hyperbridge_client().await?;

    let destination_current_timestamp = dest_client.query_timestamp().await?;
    let req = Request::Post(post.clone());
    let hash = hash_request::<Keccak256>(&req);
    let relayer_address = dest_client.query_request_receipt(hash).await?;

    if relayer_address != H160::zero() {
        // This means the message has gotten the destination chain
        return Ok(MessageStatus::DestinationDelivered { height: 0 });
    }

    // Checking to see if the messaging has timed-out
    if destination_current_timestamp.as_secs() >= post.timeout_timestamp {
        // request timed out before reaching the destination chain
        return Ok(MessageStatus::Timeout);
    }

    let hyperbridge_current_timestamp = hyperbridge_client.query_timestamp().await?;
    let relayer = hyperbridge_client.query_request_receipt(hash).await?;

    if relayer != H160::zero() {
        return Ok(MessageStatus::HyperbridgeDelivered { height: 0 });
    }

    if hyperbridge_current_timestamp.as_secs() > post.timeout_timestamp {
        // the request timed out before getting to hyper bridge
        return Ok(MessageStatus::Timeout);
    }

    Ok(MessageStatus::Pending)
}

/// `query_response_status_internal` function returns the status of a response
pub async fn query_response_status_internal(
    config: ClientConfig,
    post_response: PostResponse,
) -> Result<MessageStatus, anyhow::Error> {
    let dest_client = config.dest_chain().await?;
    let hyperbridge_client = config.hyperbridge_client().await?;

    let response_destination_timeout = dest_client.query_timestamp().await?;
    let res = Response::Post(post_response.clone());
    let req_hash = hash_request::<Keccak256>(&res.request());
    let response_receipt_relayer = dest_client.query_response_receipt(req_hash).await?;

    if response_receipt_relayer != H160::zero() {
        return Ok(MessageStatus::DestinationDelivered { height: 0 });
    }

    if response_destination_timeout.as_secs() > post_response.timeout_timestamp {
        // response timed out before reaching the destination chain
        return Ok(MessageStatus::Timeout);
    }

    let relayer = hyperbridge_client.query_response_receipt(req_hash).await?;

    if relayer != H160::zero() {
        return Ok(MessageStatus::HyperbridgeDelivered { height: 0 });
    }

    let hyperbridge_current_timestamp = hyperbridge_client.latest_timestamp().await?;

    if hyperbridge_current_timestamp.as_secs() > post_response.timeout_timestamp {
        // the request timed out before getting to hyper bridge
        return Ok(MessageStatus::Timeout);
    }

    Ok(MessageStatus::Pending)
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TimeoutStreamState {
    Pending,
    /// Destination state machine has been finalized on hyperbridge
    DestinationFinalized(u64),
    /// Message has been timed out on hyperbridge
    HyperbridgeTimedout(u64),
    /// Hyperbridge has been finalized on source chain
    HyperbridgeFinalized(u64),
    /// Stream has ended
    End,
}

/// Handles the timeout process internally and yields the encoded transaction data to be submitted
/// to the source chain This future does not check the request timeout status, only call it after
/// you have confirmed the request timeout status using `query_request_status`
pub async fn timeout_request_stream(
    post: Post,
    config: ClientConfig,
) -> Result<BoxStream<TimeoutStatus>, anyhow::Error> {
    let dest_client = config.dest_chain().await?;
    let hyperbridge_client = config.hyperbridge_client().await?;
    let source_client = config.source_chain().await?;
    let stream = stream::unfold(TimeoutStreamState::Pending, move |state| {
        let dest_client = dest_client.clone();
        let hyperbridge_client = hyperbridge_client.clone();
        let source_client = source_client.clone();
        let req = Request::Post(post.clone());
        let hash = hash_request::<Keccak256>(&req);
        async move {
            let lambda = || async {
                match state {
                    TimeoutStreamState::Pending => {
                        let relayer = hyperbridge_client.query_request_receipt(hash).await?;
                        if relayer != H160::zero() {
                            let mut stream = hyperbridge_client
                                .state_machine_update_notification(dest_client.state_machine_id())
                                .await?;
                            let mut valid_proof_height = None;
                            while let Some(event) = stream.next().await {
                                match event {
                                    Ok(ev) => {
                                        let state_machine_height = StateMachineHeight {
                                            id: ev.state_machine_id,
                                            height: ev.latest_height,
                                        };
                                        let commitment = hyperbridge_client
                                            .query_state_machine_commitment(state_machine_height)
                                            .await?;
                                        if commitment.timestamp > post.timeout_timestamp {
                                            valid_proof_height = Some(ev.latest_height);
                                            break
                                        }
                                    },
                                    Err(e) =>
                                        return Ok(Some((
                                            Err(anyhow!(
                                                "Encountered error in time out stream {e:?}"
                                            )),
                                            state,
                                        ))),
                                }
                            }
                            Ok(valid_proof_height.map(|height| {
                                (
                                    Ok(TimeoutStatus::DestinationFinalized { height }),
                                    TimeoutStreamState::DestinationFinalized(height),
                                )
                            }))
                        } else {
                            let height = hyperbridge_client.query_latest_block_height().await?;
                            Ok(Some((
                                Ok(TimeoutStatus::HyperbridgeTimedout { height }),
                                TimeoutStreamState::HyperbridgeTimedout(height),
                            )))
                        }
                    },
                    TimeoutStreamState::DestinationFinalized(proof_height) => {
                        let storage_key = dest_client.request_receipt_full_key(hash);
                        let proof =
                            dest_client.query_state_proof(proof_height, vec![storage_key]).await?;
                        let height = StateMachineHeight {
                            id: dest_client.state_machine_id(),
                            height: proof_height,
                        };
                        let message = Message::Timeout(TimeoutMessage::Post {
                            requests: vec![req.clone()],
                            timeout_proof: Proof { height, proof },
                        });
                        let challenge_period = hyperbridge_client
                            .query_challenge_period(
                                dest_client.state_machine_id().consensus_state_id,
                            )
                            .await?;
                        let update_time =
                            hyperbridge_client.query_state_machine_update_time(height).await?;
                        wait_for_challenge_period(
                            &hyperbridge_client,
                            update_time,
                            challenge_period,
                        )
                        .await?;
                        let height = hyperbridge_client.submit(message).await?;
                        Ok(Some((
                            Ok(TimeoutStatus::HyperbridgeTimedout { height }),
                            TimeoutStreamState::HyperbridgeTimedout(height),
                        )))
                    },
                    TimeoutStreamState::HyperbridgeTimedout(hyperbridge_height) => {
                        let mut state_machine_update_stream = source_client
                            .state_machine_update_notification(
                                hyperbridge_client.state_machine_id(),
                            )
                            .await?;

                        let mut valid_proof_height = None;
                        while let Some(event) = state_machine_update_stream.next().await {
                            match event {
                                Ok(ev) => {
                                    let state_machine_height = StateMachineHeight {
                                        id: ev.state_machine_id,
                                        height: ev.latest_height,
                                    };
                                    let commitment = source_client
                                        .query_state_machine_commitment(state_machine_height)
                                        .await?;
                                    if commitment.timestamp > post.timeout_timestamp &&
                                        ev.latest_height >= hyperbridge_height
                                    {
                                        valid_proof_height = Some(ev.latest_height);
                                        break
                                    }
                                },
                                Err(e) =>
                                    return Ok(Some((
                                        Err(anyhow!("Encountered error in time out stream {e:?}")),
                                        state,
                                    ))),
                            }
                        }

                        Ok(valid_proof_height.map(|height| {
                            (
                                Ok(TimeoutStatus::HyperbridgeFinalized { height }),
                                TimeoutStreamState::HyperbridgeFinalized(height),
                            )
                        }))
                    },
                    TimeoutStreamState::HyperbridgeFinalized(proof_height) => {
                        let storage_key = hyperbridge_client.request_receipt_full_key(hash);
                        let proof = hyperbridge_client
                            .query_state_proof(proof_height, vec![storage_key])
                            .await?;
                        let height = StateMachineHeight {
                            id: hyperbridge_client.state_machine,
                            height: proof_height,
                        };
                        let message = Message::Timeout(TimeoutMessage::Post {
                            requests: vec![req],
                            timeout_proof: Proof { height, proof },
                        });
                        let challenge_period = source_client
                            .query_challenge_period(
                                hyperbridge_client.state_machine_id().consensus_state_id,
                            )
                            .await?;
                        let update_time =
                            source_client.query_state_machine_update_time(height).await?;
                        wait_for_challenge_period(&source_client, update_time, challenge_period)
                            .await?;
                        let calldata = source_client.encode(message)?;

                        Ok(Some((
                            Ok(TimeoutStatus::TimeoutMessage { calldata }),
                            TimeoutStreamState::End,
                        )))
                    },
                    TimeoutStreamState::End => Ok::<_, anyhow::Error>(None),
                }
            };

            let response = lambda().await;
            match response {
                Ok(res) => res,
                Err(e) => Some((Err(anyhow!("Encountered an error in stream {e:?}")), state)),
            }
        }
    });

    Ok(Box::pin(stream))
}
