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

use crate::{
	providers::interface::Client, types::MessageStatusWithMetadata, HyperClient, Keccak256,
};
use anyhow::anyhow;
use ismp::{
	messaging::hash_request,
	router::{PostResponse, Response},
};
use primitive_types::H160;

/// `query_response_status_internal` function returns the status of a response
pub async fn query_response_status_internal(
	hyperclient: &HyperClient,
	post_response: PostResponse,
) -> Result<MessageStatusWithMetadata, anyhow::Error> {
	let dest_client = if post_response.dest_chain() == hyperclient.dest.state_machine_id().state_id
	{
		&hyperclient.dest
	} else if post_response.dest_chain() == hyperclient.source.state_machine_id().state_id {
		&hyperclient.source
	} else {
		Err(anyhow!("Unknown client for {}", post_response.dest_chain()))?
	};
	let response_destination_timeout = dest_client.query_timestamp().await?;
	let res = Response::Post(post_response.clone());
	let req_hash = hash_request::<Keccak256>(&res.request());
	let response_receipt_relayer = dest_client.query_response_receipt(req_hash).await?;

	if response_receipt_relayer != H160::zero() {
		return Ok(MessageStatusWithMetadata::DestinationDelivered { meta: Default::default() });
	}

	if response_destination_timeout.as_secs() > post_response.timeout_timestamp {
		// response timed out before reaching the destination chain
		return Ok(MessageStatusWithMetadata::Timeout);
	}

	let relayer = hyperclient.hyperbridge.query_response_receipt(req_hash).await?;

	if relayer != H160::zero() {
		return Ok(MessageStatusWithMetadata::HyperbridgeVerified { meta: Default::default() });
	}

	let hyperbridge_current_timestamp = hyperclient.hyperbridge.latest_timestamp().await?;

	if hyperbridge_current_timestamp.as_secs() > post_response.timeout_timestamp {
		// the request timed out before getting to hyper bridge
		return Ok(MessageStatusWithMetadata::Timeout);
	}

	Ok(MessageStatusWithMetadata::Pending)
}
