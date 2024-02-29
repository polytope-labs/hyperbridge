use crate::{
    providers::{global::Client, hyperbridge::HyperBridgeClient},
    types::{
        ClientConfig, HyperClientErrors, MessageStatus, ReturnRequestTimeoutData,
        ReturnRequestTimeoutMessage,
    },
    Keccak256,
};
use anyhow::anyhow;
///! This module contains the internal implementation of HyperClient.
use ethers::prelude::{Address, H160, H256};
use futures::StreamExt;
use ismp::{
    consensus::StateMachineHeight,
    messaging::{Message, TimeoutMessage},
    router::{Post, PostResponse, Request},
    util::hash_request,
};
use std::time::Duration;

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
    dest_client: impl Client,
    hyperbridge_client: HyperBridgeClient,
    post_response: PostResponse,
) -> Result<MessageStatus, anyhow::Error> {
    let response_destination_timeout = dest_client.query_timestamp().await?;
    let req = Request::Post(post_response.post.clone());
    let req_hash = hash_request::<Keccak256>(&req);
    let response_receipt_relayer = dest_client.query_response_receipt(req_hash).await?;

    if response_receipt_relayer != H160::zero() {
        return Ok(MessageStatus::DestinationDelivered);
    }

    if response_destination_timeout.as_secs() > post_response.timeout_timestamp {
        // response timed out before reaching the destination chain
        return Ok(MessageStatus::Timeout);
    }

    let hyper_bridge_response = hyperbridge_client.query_response(req_hash).await?;

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

// /// `timeout_request_internal` function is an internal function that is used to
// /// time out a request
// pub async fn timeout_request_internal(
//     post: Post,
//     source_client: impl Client,
//     dest_client: impl Client,
//     hyperbridge_client: HyperBridgeClient,
//     config: ClientConfig,
// ) -> Result<ReturnRequestTimeoutData, anyhow::Error> {
//     let req = Request::Post(post.clone());
//     let post_request_commitment = hash_request::<Keccak256>(&req);
//     let dest_state_machine_id = dest_client.state_machine_id()?;
//     let source_state_machine_id = source_client.state_machine_id()?;
//
//     let hyper_bridge_response =
// hyperbridge_client.query_request(&post_request_commitment).await?;
//
//     if let Some(request) = hyper_bridge_response.get(0) {
//         let dest_current_block_time = dest_client.host_timestamp().await?;
//
//         if !request.timed_out(Duration::from_secs(dest_current_block_time)) {
//             return Err(anyhow!("RequestIsNotDueForTimeOut"));
//         }
//
//         let mut dest_state_machine_update_stream = hyperbridge_client
//             .state_machine_update_notification(dest_state_machine_id.clone())
//             .await?;
//
//         let mut timeout_response;
//
//         while let Some(item) = dest_state_machine_update_stream.next().await {
//             match item {
//                 Ok(state_machine_update) => {
//                     let current_state_machine_height = StateMachineHeight {
//                         height: state_machine_update.latest_height,
//                         id: state_machine_update.state_machine_id,
//                     };
//
//                     let state_machine_commitment = hyperbridge_client
//                         .query_state_machine_commitment(current_state_machine_height)
//                         .await?;
//
//                     if state_machine_commitment.timestamp <= request.timeout().as_secs() {
//                         let proof = dest_client
//                             .query_request_proof(
//                                 &post_request_commitment,
//                                 state_machine_update.latest_height,
//                             )
//                             .await?;
//
//                         let timeout_message = Message::Timeout(TimeoutMessage::Post {
//                             requests: vec![req.clone()],
//                             timeout_proof: proof.clone(),
//                         });
//
//                         timeout_response =
//                             hyperbridge_client.send_message(proof, timeout_message).await?;
//
//                         break;
//                     }
//                 },
//                 Err(e) => {},
//             }
//         }
//
//         let mut source_state_machine_update_stream = hyperbridge_client
//             .state_machine_update_notification(source_state_machine_id.clone())
//             .await?;
//
//         while let Some(source_stream_item) = source_state_machine_update_stream.next().await {
//             match source_stream_item {
//                 Ok(source_state_machine_update) => {
//                     let source_current_state_machine_height = StateMachineHeight {
//                         height: source_state_machine_update.latest_height,
//                         id: source_state_machine_update.state_machine_id,
//                     };
//
//                     let state_machine_commitment = hyperbridge_client
//                         .query_state_machine_commitment(source_current_state_machine_height)
//                         .await?;
//
//                     if state_machine_commitment.timestamp <= request.timeout().as_secs() {
//                         let request_key =
//                             get_request_storage_key(Vec::from(post_request_commitment.0));
//                         let proof_from_hyperbridge = hyperbridge_client
//                             .get_state_proof(
//                                 source_state_machine_update.latest_height,
//                                 vec![request_key],
//                             )
//                             .await?;
//
//                         let timeout_return_message = ReturnRequestTimeoutMessage {
//                             timeouts: vec![post.clone()],
//                             height: source_current_state_machine_height,
//                             proof: vec![proof_from_hyperbridge],
//                         };
//
//                         let timeout_data = ReturnRequestTimeoutData {
//                             host: config.source_ismp_host_address,
//                             post_request_timeout_message: timeout_return_message,
//                         };
//
//                         return Ok(timeout_data);
//                     } else {
//                         continue;
//                     }
//                 },
//                 Err(e) => {},
//             }
//         }
//     }
//
//     Err(anyhow!("ErrorGettingSourceStateMachineUpdate"))
// }
