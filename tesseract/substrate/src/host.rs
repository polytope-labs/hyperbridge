// Copyright (C) 2023 Polytope Labs.
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

//! [`IsmpHost`] implementation

use crate::SubstrateClient;
use anyhow::{anyhow, Error};
use ismp::{events::StateMachineUpdated, messaging::CreateConsensusState};
use primitives::{BoxStream, ByzantineHandler, IsmpHost, IsmpProvider};
use subxt::{
	config::{extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams},
	ext::sp_runtime::MultiSignature,
};

#[async_trait::async_trait]
impl<I, C> ByzantineHandler for SubstrateClient<I, C>
where
	I: IsmpHost,
	C: subxt::Config,
{
	async fn query_consensus_message(
		&self,
		challenge_event: StateMachineUpdated,
	) -> Result<ismp::messaging::ConsensusMessage, anyhow::Error> {
		self.host
			.as_ref()
			.ok_or_else(|| anyhow!("Host not initialized"))?
			.query_consensus_message(challenge_event)
			.await
	}

	async fn check_for_byzantine_attack<T: IsmpHost + IsmpProvider>(
		&self,
		counterparty: &T,
		consensus_message: ismp::messaging::ConsensusMessage,
	) -> Result<(), anyhow::Error> {
		self.host
			.as_ref()
			.ok_or_else(|| anyhow!("Host not initialized"))?
			.check_for_byzantine_attack(counterparty, consensus_message)
			.await
	}
}

#[async_trait::async_trait]
impl<T, C> IsmpHost for SubstrateClient<T, C>
where
	T: IsmpHost + Clone,
	C: subxt::Config + Send + Sync + Clone,
	<C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<C, PlainTip>>,
	C::Signature: From<MultiSignature> + Send + Sync,
	C::AccountId:
		From<sp_core::crypto::AccountId32> + Into<C::Address> + Clone + 'static + Send + Sync,
{
	async fn consensus_notification<I>(
		&self,
		counterparty: I,
	) -> Result<BoxStream<ismp::messaging::ConsensusMessage>, anyhow::Error>
	where
		I: IsmpHost + IsmpProvider + Clone + 'static,
	{
		self.host
			.as_ref()
			.ok_or_else(|| anyhow!("Host not initialized"))?
			.consensus_notification(counterparty)
			.await
	}

	async fn query_initial_consensus_state(&self) -> Result<Option<CreateConsensusState>, Error> {
		self.host
			.as_ref()
			.ok_or_else(|| anyhow!("Host not initialized"))?
			.query_initial_consensus_state()
			.await
	}
}

impl<T: IsmpHost + Clone, C: subxt::Config> Clone for SubstrateClient<T, C> {
	fn clone(&self) -> Self {
		Self {
			host: self.host.clone(),
			client: self.client.clone(),
			consensus_state_id: self.consensus_state_id,
			state_machine: self.state_machine,
			hashing: self.hashing.clone(),
			signer: self.signer.clone(),
			address: self.address.clone(),
			initial_height: self.initial_height,
			config: self.config.clone(),
		}
	}
}
