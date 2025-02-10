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

use ismp::host::StateMachine;
use primitive_types::H256;
use subxt_utils::{BlakeSubstrateChain, Hyperbridge};

use crate::providers::{evm::EvmClient, interface::Client, substrate::SubstrateClient};

#[derive(Clone)]
pub enum AnyClient {
	Evm(EvmClient),
	BlakeSubstrateChain(SubstrateClient<BlakeSubstrateChain>),
	KeccakSubstrateChain(SubstrateClient<Hyperbridge>),
}

impl Client for AnyClient {
	async fn query_latest_block_height(&self) -> Result<u64, anyhow::Error> {
		match self {
			AnyClient::Evm(inner) => inner.query_latest_block_height().await,
			AnyClient::BlakeSubstrateChain(inner) => inner.query_latest_block_height().await,
			AnyClient::KeccakSubstrateChain(inner) => inner.query_latest_block_height().await,
		}
	}

	fn state_machine_id(&self) -> ismp::consensus::StateMachineId {
		match self {
			AnyClient::Evm(inner) => inner.state_machine_id(),
			AnyClient::BlakeSubstrateChain(inner) => inner.state_machine_id(),
			AnyClient::KeccakSubstrateChain(inner) => inner.state_machine_id(),
		}
	}

	async fn query_timestamp(&self) -> Result<std::time::Duration, anyhow::Error> {
		match self {
			AnyClient::Evm(inner) => inner.query_timestamp().await,
			AnyClient::BlakeSubstrateChain(inner) => inner.query_timestamp().await,
			AnyClient::KeccakSubstrateChain(inner) => inner.query_timestamp().await,
		}
	}

	async fn query_request_receipt(
		&self,
		request_hash: sp_core::H256,
	) -> Result<sp_core::H160, anyhow::Error> {
		match self {
			AnyClient::Evm(inner) => inner.query_request_receipt(request_hash).await,
			AnyClient::BlakeSubstrateChain(inner) =>
				inner.query_request_receipt(request_hash).await,
			AnyClient::KeccakSubstrateChain(inner) =>
				inner.query_request_receipt(request_hash).await,
		}
	}

	async fn query_state_proof(
		&self,
		at: u64,
		key: Vec<Vec<u8>>,
	) -> Result<Vec<u8>, anyhow::Error> {
		match self {
			AnyClient::Evm(inner) => inner.query_state_proof(at, key).await,
			AnyClient::BlakeSubstrateChain(inner) => inner.query_state_proof(at, key).await,
			AnyClient::KeccakSubstrateChain(inner) => inner.query_state_proof(at, key).await,
		}
	}

	async fn query_requests_proof(
		&self,
		at: u64,
		keys: Vec<crate::providers::interface::Query>,
		counterparty: StateMachine,
	) -> Result<Vec<u8>, anyhow::Error> {
		match self {
			AnyClient::Evm(inner) => inner.query_requests_proof(at, keys, counterparty).await,
			AnyClient::BlakeSubstrateChain(inner) =>
				inner.query_requests_proof(at, keys, counterparty).await,
			AnyClient::KeccakSubstrateChain(inner) =>
				inner.query_requests_proof(at, keys, counterparty).await,
		}
	}

	async fn query_responses_proof(
		&self,
		at: u64,
		keys: Vec<crate::providers::interface::Query>,
		counterparty: StateMachine,
	) -> Result<Vec<u8>, anyhow::Error> {
		match self {
			AnyClient::Evm(inner) => inner.query_responses_proof(at, keys, counterparty).await,
			AnyClient::BlakeSubstrateChain(inner) =>
				inner.query_responses_proof(at, keys, counterparty).await,
			AnyClient::KeccakSubstrateChain(inner) =>
				inner.query_responses_proof(at, keys, counterparty).await,
		}
	}

	async fn query_response_receipt(
		&self,
		request_commitment: sp_core::H256,
	) -> Result<sp_core::H160, anyhow::Error> {
		match self {
			AnyClient::Evm(inner) => inner.query_response_receipt(request_commitment).await,
			AnyClient::BlakeSubstrateChain(inner) =>
				inner.query_response_receipt(request_commitment).await,
			AnyClient::KeccakSubstrateChain(inner) =>
				inner.query_response_receipt(request_commitment).await,
		}
	}

	async fn ismp_events_stream(
		&self,
		commitment: H256,
		initial_height: u64,
	) -> Result<
		crate::types::BoxStream<crate::providers::interface::WithMetadata<ismp::events::Event>>,
		anyhow::Error,
	> {
		match self {
			AnyClient::Evm(inner) => inner.ismp_events_stream(commitment, initial_height).await,
			AnyClient::BlakeSubstrateChain(inner) =>
				inner.ismp_events_stream(commitment, initial_height).await,
			AnyClient::KeccakSubstrateChain(inner) =>
				inner.ismp_events_stream(commitment, initial_height).await,
		}
	}

	async fn query_ismp_event(
		&self,
		range: std::ops::RangeInclusive<u64>,
	) -> Result<Vec<crate::providers::interface::WithMetadata<ismp::events::Event>>, anyhow::Error>
	{
		match self {
			AnyClient::Evm(inner) => inner.query_ismp_event(range).await,
			AnyClient::BlakeSubstrateChain(inner) => inner.query_ismp_event(range).await,
			AnyClient::KeccakSubstrateChain(inner) => inner.query_ismp_event(range).await,
		}
	}

	async fn post_request_handled_stream(
		&self,
		commitment: sp_core::H256,
		initial_height: u64,
	) -> Result<
		crate::types::BoxStream<
			crate::providers::interface::WithMetadata<ismp::events::RequestResponseHandled>,
		>,
		anyhow::Error,
	> {
		match self {
			AnyClient::Evm(inner) =>
				inner.post_request_handled_stream(commitment, initial_height).await,
			AnyClient::BlakeSubstrateChain(inner) =>
				inner.post_request_handled_stream(commitment, initial_height).await,
			AnyClient::KeccakSubstrateChain(inner) =>
				inner.post_request_handled_stream(commitment, initial_height).await,
		}
	}

	async fn query_latest_state_machine_height(
		&self,
		state_machine: ismp::consensus::StateMachineId,
	) -> Result<u64, anyhow::Error> {
		match self {
			AnyClient::Evm(inner) => inner.query_latest_state_machine_height(state_machine).await,
			AnyClient::BlakeSubstrateChain(inner) =>
				inner.query_latest_state_machine_height(state_machine).await,
			AnyClient::KeccakSubstrateChain(inner) =>
				inner.query_latest_state_machine_height(state_machine).await,
		}
	}

	async fn query_state_machine_commitment(
		&self,
		id: ismp::consensus::StateMachineHeight,
	) -> Result<ismp::consensus::StateCommitment, anyhow::Error> {
		match self {
			AnyClient::Evm(inner) => inner.query_state_machine_commitment(id).await,
			AnyClient::BlakeSubstrateChain(inner) => inner.query_state_machine_commitment(id).await,
			AnyClient::KeccakSubstrateChain(inner) =>
				inner.query_state_machine_commitment(id).await,
		}
	}

	async fn state_machine_update_notification(
		&self,
		counterparty_state_id: ismp::consensus::StateMachineId,
	) -> Result<
		crate::types::BoxStream<
			crate::providers::interface::WithMetadata<ismp::events::StateMachineUpdated>,
		>,
		anyhow::Error,
	> {
		match self {
			AnyClient::Evm(inner) =>
				inner.state_machine_update_notification(counterparty_state_id).await,
			AnyClient::BlakeSubstrateChain(inner) =>
				inner.state_machine_update_notification(counterparty_state_id).await,
			AnyClient::KeccakSubstrateChain(inner) =>
				inner.state_machine_update_notification(counterparty_state_id).await,
		}
	}

	fn request_commitment_full_key(&self, commitment: sp_core::H256) -> Vec<u8> {
		match self {
			AnyClient::Evm(inner) => inner.request_commitment_full_key(commitment),
			AnyClient::BlakeSubstrateChain(inner) => inner.request_commitment_full_key(commitment),
			AnyClient::KeccakSubstrateChain(inner) => inner.request_commitment_full_key(commitment),
		}
	}

	fn request_receipt_full_key(&self, commitment: sp_core::H256) -> Vec<u8> {
		match self {
			AnyClient::Evm(inner) => inner.request_receipt_full_key(commitment),
			AnyClient::BlakeSubstrateChain(inner) => inner.request_receipt_full_key(commitment),
			AnyClient::KeccakSubstrateChain(inner) => inner.request_receipt_full_key(commitment),
		}
	}

	fn response_commitment_full_key(&self, commitment: sp_core::H256) -> Vec<u8> {
		match self {
			AnyClient::Evm(inner) => inner.response_commitment_full_key(commitment),
			AnyClient::BlakeSubstrateChain(inner) => inner.response_commitment_full_key(commitment),
			AnyClient::KeccakSubstrateChain(inner) =>
				inner.response_commitment_full_key(commitment),
		}
	}

	fn response_receipt_full_key(&self, commitment: sp_core::H256) -> Vec<u8> {
		match self {
			AnyClient::Evm(inner) => inner.response_receipt_full_key(commitment),
			AnyClient::BlakeSubstrateChain(inner) => inner.response_receipt_full_key(commitment),
			AnyClient::KeccakSubstrateChain(inner) => inner.response_receipt_full_key(commitment),
		}
	}

	fn encode(&self, msg: ismp::messaging::Message) -> Result<Vec<u8>, anyhow::Error> {
		match self {
			AnyClient::Evm(inner) => inner.encode(msg),
			AnyClient::BlakeSubstrateChain(inner) => inner.encode(msg),
			AnyClient::KeccakSubstrateChain(inner) => inner.encode(msg),
		}
	}

	async fn submit(
		&self,
		msg: ismp::messaging::Message,
	) -> Result<crate::types::EventMetadata, anyhow::Error> {
		match self {
			AnyClient::Evm(inner) => inner.submit(msg).await,
			AnyClient::BlakeSubstrateChain(inner) => inner.submit(msg).await,
			AnyClient::KeccakSubstrateChain(inner) => inner.submit(msg).await,
		}
	}

	async fn query_state_machine_update_time(
		&self,
		height: ismp::consensus::StateMachineHeight,
	) -> Result<std::time::Duration, anyhow::Error> {
		match self {
			AnyClient::Evm(inner) => inner.query_state_machine_update_time(height).await,
			AnyClient::BlakeSubstrateChain(inner) =>
				inner.query_state_machine_update_time(height).await,
			AnyClient::KeccakSubstrateChain(inner) =>
				inner.query_state_machine_update_time(height).await,
		}
	}

	async fn query_challenge_period(
		&self,
		id: ismp::consensus::StateMachineId,
	) -> Result<std::time::Duration, anyhow::Error> {
		match self {
			AnyClient::Evm(inner) => inner.query_challenge_period(id).await,
			AnyClient::BlakeSubstrateChain(inner) => inner.query_challenge_period(id).await,
			AnyClient::KeccakSubstrateChain(inner) => inner.query_challenge_period(id).await,
		}
	}
}
