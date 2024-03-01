use crate::{
    providers::global::Client,
    types::{ClientConfig, MessageStatus},
    Keccak256,
};
use anyhow::anyhow;
///! This module contains the internal implementation of HyperClient.
use ethers::prelude::H160;
use futures::StreamExt;
use ismp::{
    consensus::StateMachineHeight,
    messaging::{Message, Proof, TimeoutMessage},
    router::{Post, PostResponse, Request, Response},
    util::{hash_request, hash_response},
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
        return Ok(MessageStatus::DestinationDelivered);
    }

    // Checking to see if the messaging has timed-out
    if destination_current_timestamp.as_secs() >= post.timeout_timestamp {
        // request timed out before reaching the destination chain
        return Ok(MessageStatus::Timeout);
    }

    let hyperbridge_current_timestamp = hyperbridge_client.query_timestamp().await?;
    let request = hyperbridge_client.query_request(hash).await?;

    if let Some(_) = request {
        return Ok(MessageStatus::HyperbridgeDelivered);
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
    let res_hash = hash_response::<Keccak256>(&res);
    let response_receipt_relayer = dest_client.query_response_receipt(res_hash).await?;

    if response_receipt_relayer != H160::zero() {
        return Ok(MessageStatus::DestinationDelivered);
    }

    if response_destination_timeout.as_secs() > post_response.timeout_timestamp {
        // response timed out before reaching the destination chain
        return Ok(MessageStatus::Timeout);
    }

    let hyper_bridge_response = hyperbridge_client.query_response(res_hash).await?;

    if let Some(_) = hyper_bridge_response {
        return Ok(MessageStatus::HyperbridgeDelivered);
    }

    let hyperbridge_current_timestamp = hyperbridge_client.latest_timestamp().await?;

    if hyperbridge_current_timestamp.as_secs() > post_response.timeout_timestamp {
        // the request timed out before getting to hyper bridge
        return Ok(MessageStatus::Timeout);
    }

    Ok(MessageStatus::Pending)
}

/// Handles the timeout process internally and yields the encoded transaction data to be submitted
/// to the source chain This future does not check the request timeout status, only call it after
/// you have confirmed the request timeout status using `query_request_status`
pub async fn timeout_request(post: Post, config: ClientConfig) -> Result<Vec<u8>, anyhow::Error> {
    let dest_client = config.dest_chain().await?;
    let hyperbridge_client = config.hyperbridge_client().await?;
    let source_client = config.source_chain().await?;
    let req = Request::Post(post.clone());
    let hash = hash_request::<Keccak256>(&req);
    let hyper_bridge_response = hyperbridge_client.query_response(hash).await?;

    let timeout_height = if hyper_bridge_response.is_some() {
        let mut stream = hyperbridge_client
            .state_machine_update_notification(dest_client.state_machine_id())
            .await?;
        let mut valid_proof_height = None;
        while let Some(event) = stream.next().await {
            match event {
                Ok(ev) => {
                    let state_machine_height =
                        StateMachineHeight { id: ev.state_machine_id, height: ev.latest_height };
                    let commitment = hyperbridge_client
                        .query_state_machine_commitment(state_machine_height)
                        .await?;
                    if commitment.timestamp > post.timeout_timestamp {
                        valid_proof_height = Some(ev.latest_height);
                        break
                    }
                },
                Err(_) => {},
            }
        }

        if let Some(proof_height) = valid_proof_height {
            let storage_key = dest_client.request_receipt_full_key(hash);
            let proof = dest_client.query_state_proof(proof_height, vec![storage_key]).await?;
            let message = Message::Timeout(TimeoutMessage::Post {
                requests: vec![req],
                timeout_proof: Proof {
                    height: StateMachineHeight {
                        id: dest_client.state_machine_id(),
                        height: proof_height,
                    },
                    proof,
                },
            });

            hyperbridge_client.submit(message).await?;
            hyperbridge_client.query_latest_block_height().await?
        } else {
            Err(anyhow!("Encountered an error wile trying to timeout request on hyperbridge"))?
        }
    } else {
        hyperbridge_client.query_latest_block_height().await?
    };

    let mut state_machine_update_stream = source_client
        .state_machine_update_notification(hyperbridge_client.state_machine)
        .await?;

    let mut valid_proof_height = None;
    while let Some(event) = state_machine_update_stream.next().await {
        match event {
            Ok(ev) => {
                let state_machine_height =
                    StateMachineHeight { id: ev.state_machine_id, height: ev.latest_height };
                let commitment =
                    dest_client.query_state_machine_commitment(state_machine_height).await?;
                if commitment.timestamp > post.timeout_timestamp &&
                    ev.latest_height >= timeout_height
                {
                    valid_proof_height = Some(ev.latest_height);
                    break
                }
            },
            Err(_) => {
                // An error occured in stream
            },
        }
    }

    let message = if let Some(proof_height) = valid_proof_height {
        let storage_key = source_client.request_receipt_full_key(hash);
        let proof = hyperbridge_client.query_state_proof(proof_height, vec![storage_key]).await?;
        let message = Message::Timeout(TimeoutMessage::Post {
            requests: vec![req],
            timeout_proof: Proof {
                height: StateMachineHeight {
                    id: hyperbridge_client.state_machine,
                    height: proof_height,
                },
                proof,
            },
        });

        source_client.encode(message)?
    } else {
        Err(anyhow!(
            "Failed to complete timeout request successfully on {:?}",
            source_client.state_machine_id().state_id
        ))?
    };

    Ok(message)
}
