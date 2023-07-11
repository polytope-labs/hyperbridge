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
use primitives::{BoxStream, ByzantineHandler, ChallengePeriodStarted, IsmpHost, IsmpProvider};
use std::sync::Arc;

#[async_trait::async_trait]
impl<I, C> ByzantineHandler for SubstrateClient<I, C>
where
    I: IsmpHost,
    C: subxt::Config,
{
    async fn query_consensus_message(
        &self,
        challenge_event: ChallengePeriodStarted,
    ) -> Result<ismp::messaging::ConsensusMessage, anyhow::Error> {
        self.host.query_consensus_message(challenge_event).await
    }

    async fn check_for_byzantine_attack<T: IsmpHost>(
        &self,
        counterparty: &T,
        consensus_message: ismp::messaging::ConsensusMessage,
    ) -> Result<(), anyhow::Error> {
        self.host.check_for_byzantine_attack(counterparty, consensus_message).await
    }
}

#[async_trait::async_trait]
impl<T, C> IsmpHost for SubstrateClient<T, C>
where
    T: IsmpHost + Clone,
    C: subxt::Config,
{
    async fn consensus_notification<I>(
        &self,
        counterparty: I,
    ) -> Result<BoxStream<ismp::messaging::ConsensusMessage>, anyhow::Error>
    where
        I: IsmpHost + IsmpProvider + Clone + 'static,
    {
        self.consensus_notification(counterparty).await
    }
}

impl<T: IsmpHost + Clone, C: subxt::Config> Clone for SubstrateClient<T, C> {
    fn clone(&self) -> Self {
        Self {
            host: self.host.clone(),
            client: self.client.clone(),
            consensus_client: self.consensus_client.clone(),
            state_machine: self.state_machine.clone(),
            hashing: self.hashing.clone(),
            signer: self.signer.clone(),
            latest_state_machine_height: Arc::clone(&self.latest_state_machine_height),
        }
    }
}
