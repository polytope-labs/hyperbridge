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

/// This will wrap the provided implementations into an [`AnyClient`] that implement the required
/// traits [`IsmpHost`], [`IsmpProvider`] & [`ByzantineHandler`].
#[macro_export]
macro_rules! chain {
	($(
        $(#[$($meta:meta)*])*
		$name:ident($config:path, $client:path),
	)*) => {
		#[derive(Debug, Serialize, Deserialize, Clone)]
		#[serde(tag = "type", rename_all = "snake_case")]
		pub enum AnyConfig {
			$(
				$(#[$($meta)*])*
				$name($config),
			)*
		}

		#[derive(Clone)]
		pub enum AnyClient {
			$(
				$(#[$($meta)*])*
				$name($client),
			)*
		}

        #[async_trait::async_trait]
        impl primitives::IsmpHost for AnyClient {
            async fn consensus_notification<C>(
                &self,
                counterparty: C,
            ) -> Result<primitives::BoxStream<ismp::messaging::ConsensusMessage>, anyhow::Error>
            where
                C: primitives::IsmpHost + primitives::IsmpProvider + Clone + 'static
            {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.consensus_notification(counterparty).await,
					)*
				}
            }
        }

        #[async_trait::async_trait]
        impl primitives::IsmpProvider for AnyClient {
            async fn query_consensus_state(
                &self,
                at: Option<u64>,
                id: ismp::consensus::ConsensusClientId,
            ) -> Result<Vec<u8>, anyhow::Error> {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.query_consensus_state(at, id).await,
					)*
				}
            }

            async fn query_latest_state_machine_height(
                &self,
                id: ismp::consensus::StateMachineId,
            ) -> Result<u32, anyhow::Error> {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.query_latest_state_machine_height(id).await,
					)*
				}
            }

            async fn query_consensus_update_time(
                &self,
                id: ismp::consensus::ConsensusClientId,
            ) -> Result<core::time::Duration, anyhow::Error> {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.query_consensus_update_time(id).await,
					)*
				}
            }

			async fn query_challenge_period(
                &self,
                id: ismp::consensus::ConsensusClientId,
            ) -> Result<core::time::Duration, anyhow::Error> {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.query_challenge_period(id).await,
					)*
				}
            }

			async fn query_timestamp(
                &self,
            ) -> Result<core::time::Duration, anyhow::Error> {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.query_timestamp().await,
					)*
				}
            }

            async fn query_requests_proof(
                &self,
                at: u64,
                keys: Vec<primitives::Query>,
            ) -> Result<Vec<u8>, anyhow::Error> {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.query_requests_proof(at, keys).await,
					)*
				}
            }

            async fn query_responses_proof(
                &self,
                at: u64,
                keys: Vec<primitives::Query>,
            ) -> Result<Vec<u8>, anyhow::Error> {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.query_responses_proof(at, keys).await,
					)*
				}
            }

            async fn query_state_proof(
                &self,
                at: u64,
                keys: Vec<Vec<u8>>,
            ) -> Result<Vec<u8>, anyhow::Error> {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.query_state_proof(at, keys).await,
					)*
				}
            }

            async fn query_ismp_events(
                &self,
                event: primitives::StateMachineUpdated,
            ) -> Result<Vec<ismp::events::Event>, anyhow::Error> {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.query_ismp_events(event).await,
					)*
				}
            }

            async fn query_pending_get_requests(&self, height: u64) -> Result<Vec<ismp::router::Get>, anyhow::Error> {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.query_pending_get_requests(height).await,
					)*
				}
            }


            fn name(&self) -> String {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.name(),
					)*
				}
            }

            fn state_machine_id(&self) -> ismp::consensus::StateMachineId {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.state_machine_id(),
					)*
				}
            }

            fn block_max_gas(&self) -> u64 {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.block_max_gas(),
					)*
				}
            }

            async fn estimate_gas(&self, msg: Vec<ismp::messaging::Message>) -> Result<u64, anyhow::Error> {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.estimate_gas(msg).await,
					)*
				}
            }

            async fn state_machine_update_notification(
                &self,
                counterparty_state_id: ismp::consensus::StateMachineId,
            ) -> primitives::BoxStream<primitives::StateMachineUpdated> {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.state_machine_update_notification(counterparty_state_id).await,
					)*
				}
            }

            async fn submit(&self, messages: Vec<ismp::messaging::Message>) -> Result<(), anyhow::Error> {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.submit(messages).await,
					)*
				}
            }
        }

        #[async_trait::async_trait]
        impl primitives::ByzantineHandler for AnyClient {
            async fn query_consensus_message(
                &self,
                event: primitives::ChallengePeriodStarted,
            ) -> Result<ismp::messaging::ConsensusMessage, anyhow::Error> {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.query_consensus_message(event).await,
					)*
				}
            }

            async fn check_for_byzantine_attack<C: primitives::IsmpHost>(
                &self,
                counterparty: &C,
                consensus_message: ismp::messaging::ConsensusMessage,
            ) -> Result<(), anyhow::Error> {
                match self {
					$(
						$(#[$($meta)*])*
						Self::$name(chain) => chain.check_for_byzantine_attack(counterparty, consensus_message).await,
					)*
				}
            }
        }
    };
}
