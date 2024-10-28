// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod get_request;
mod post_request;
mod post_response;

pub use get_request::*;
pub use post_request::*;
pub use post_response::*;

///! This module contains the internal implementation of HyperClient.
use crate::{
	any_client::AnyClient,
	providers::{
		interface::{wait_for_challenge_period, Client, Query},
		substrate::SubstrateClient,
	},
	types::{BoxStream, MessageStatusWithMetadata},
	Keccak256,
};
use anyhow::anyhow;
use ethers::prelude::H160;
use futures::{stream, StreamExt};
use ismp::{
	consensus::StateMachineHeight,
	messaging::{hash_request, Message, Proof, RequestMessage, ResponseMessage},
	router::{PostRequest, Request, Response},
};
use sp_core::H256;
use subxt_utils::Hyperbridge;

use ismp::messaging::hash_response;
use std::time::Duration;

#[cfg(all(target_arch = "wasm32", feature = "web"))]
use gloo_timers::future::*;
#[cfg(not(target_arch = "wasm32"))]
use tokio::time::*;
#[cfg(all(target_arch = "wasm32", feature = "nodejs"))]
use wasmtimer::tokio::*;

/// This returns a stream that yields when the provided timeout value is reached on the chain for
/// the provided [`Client`]
pub async fn message_timeout_stream(
	timeout: u64,
	client: impl Client + Clone,
	request: Request,
) -> BoxStream<MessageStatusWithMetadata> {
	if timeout == 0 {
		// since it doesn't timeout, use stream pending here
		return Box::pin(stream::pending());
	}

	let commitment = hash_request::<Keccak256>(&request);
	let stream = stream::unfold(client, move |client| async move {
		let lambda = || async {
			let relayer = client.query_request_receipt(commitment).await?;
			if relayer != Default::default() {
				return Ok(None);
			}

			let current_timestamp = client.query_timestamp().await?.as_secs();
			return if current_timestamp > timeout {
				Ok(Some(true))
			} else {
				let sleep_time = timeout - current_timestamp;
				tracing::trace!("Sleeping for {sleep_time}s");
				let _ = sleep(Duration::from_secs(sleep_time)).await;
				Ok::<_, anyhow::Error>(Some(false))
			};
		};

		let response = lambda().await;

		let value = match response {
			Ok(Some(true)) => Some((Ok(Some(MessageStatusWithMetadata::Timeout)), client)),
			Ok(Some(false)) => Some((Ok(None), client)),
			Ok(None) => None,
			Err(e) =>
				Some((Err(anyhow!("Encountered an error in timeout stream: {:?}", e)), client)),
		};

		return value;
	})
	.filter_map(|item| async move {
		match item {
			Ok(None) => None,
			Ok(Some(event)) => Some(Ok(event)),
			Err(err) => Some(Err(err)),
		}
	});

	Box::pin(stream)
}

pub async fn encode_request_call_data(
	hyperbridge: &SubstrateClient<Hyperbridge>,
	dest_client: &AnyClient,
	post: PostRequest,
	commitment: H256,
	height: u64,
) -> Result<Vec<u8>, anyhow::Error> {
	let proof = hyperbridge
		.query_requests_proof(
			height,
			vec![Query { commitment }],
			dest_client.state_machine_id().state_id,
		)
		.await?;
	let proof_height = StateMachineHeight { id: hyperbridge.state_machine, height };

	let message = Message::Request(RequestMessage {
		requests: vec![post.clone()],
		proof: Proof { height: proof_height, proof },
		signer: H160::zero().0.to_vec(),
	});
	let calldata = dest_client.encode(message)?;
	Ok(calldata)
}

pub async fn encode_response_call_data(
	hyperbridge: &SubstrateClient<Hyperbridge>,
	dest_client: &AnyClient,
	response: Response,
	height: u64,
) -> Result<Vec<u8>, anyhow::Error> {
	let commitment = hash_response::<Keccak256>(&response);
	let proof = hyperbridge
		.query_responses_proof(
			height,
			vec![Query { commitment }],
			dest_client.state_machine_id().state_id,
		)
		.await?;
	let proof_height = StateMachineHeight { id: hyperbridge.state_machine, height };

	let message = Message::Response(ResponseMessage {
		datagram: ismp::router::RequestResponse::Response(vec![response]),
		proof: Proof { height: proof_height, proof },
		signer: H160::zero().0.to_vec(),
	});
	let calldata = dest_client.encode(message)?;
	Ok(calldata)
}
// Encodes the call data for the message but waits for the challenge period before yielding
pub async fn encode_request_message_and_wait_for_challenge_period(
	hyperbridge: &SubstrateClient<Hyperbridge>,
	dest_client: &AnyClient,
	post: PostRequest,
	commitment: H256,
	height: u64,
) -> Result<Vec<u8>, anyhow::Error> {
	let calldata =
		encode_request_call_data(hyperbridge, dest_client, post, commitment, height).await?;
	let proof_height = StateMachineHeight { id: hyperbridge.state_machine, height };
	let challenge_period =
		dest_client.query_challenge_period(hyperbridge.state_machine_id()).await?;
	let update_time = dest_client.query_state_machine_update_time(proof_height).await?;
	wait_for_challenge_period(dest_client, update_time, challenge_period).await?;

	Ok(calldata)
}
// Encodes the call data for the message but waits for the challenge period before yielding
pub async fn encode_response_message_and_wait_for_challenge_period(
	hyperbridge: &SubstrateClient<Hyperbridge>,
	dest_client: &AnyClient,
	response: Response,
	height: u64,
) -> Result<Vec<u8>, anyhow::Error> {
	let calldata = encode_response_call_data(hyperbridge, dest_client, response, height).await?;
	let proof_height = StateMachineHeight { id: hyperbridge.state_machine, height };
	let challenge_period =
		dest_client.query_challenge_period(hyperbridge.state_machine_id()).await?;
	let update_time = dest_client.query_state_machine_update_time(proof_height).await?;
	wait_for_challenge_period(dest_client, update_time, challenge_period).await?;

	Ok(calldata)
}
