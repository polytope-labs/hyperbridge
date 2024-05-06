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

use crate::GrandpaHost;
use codec::{Decode, Encode};
use futures::stream;
use ismp::{host::StateMachine, messaging::ConsensusMessage};
use ismp_grandpa::messages::{RelayChainMessage, StandaloneChainMessage};
use ismp_grandpa_primitives::ConsensusState;

use ismp_grandpa_primitives::justification::GrandpaJustification;
use subxt::config::Header;

use subxt::{
    config::substrate::{BlakeTwo256, SubstrateHeader},
    ext::sp_runtime::traits::{One, Zero},
};
use tesseract_primitives::{BoxStream, IsmpHost, IsmpProvider};
use tokio::time::{interval, sleep};

pub type Justification = GrandpaJustification<polkadot_core_primitives::Header>;

#[async_trait::async_trait]
impl<T> IsmpHost for GrandpaHost<T>
where
    T: subxt::Config + Send + Sync + Clone,
    T::Header: Send + Sync,
    <T::Header as Header>::Number: Ord + Zero + finality_grandpa::BlockNumberOps + One,
    u32: From<<T::Header as Header>::Number>,
    sp_core::H256: From<T::Hash>,
    T::Header: codec::Decode,
    <T::Hasher as subxt::config::Hasher>::Output: From<T::Hash>,
    T::Hash: From<<T::Hasher as subxt::config::Hasher>::Output>,
    <T as subxt::Config>::Hash: From<sp_core::H256>,
{
    async fn consensus_notification<C>(
        &self,
        counterparty: C,
    ) -> Result<BoxStream<ConsensusMessage>, anyhow::Error>
    where
        C: IsmpHost + IsmpProvider + 'static,
    {
        let client = GrandpaHost::clone(&self);
        let challenge_period =
            counterparty.query_challenge_period(self.consensus_state_id.clone()).await?;
        let last_consensus_update =
            counterparty.query_state_machine_update_time(self.consensus_state_id.clone()).await?;
        let counterparty_timestamp = counterparty.query_timestamp().await?;
        if counterparty_timestamp - last_consensus_update < challenge_period {
            // We sleep until the current challenge period has elapsed before starting the interval
            // because the first tick of the interval completes instantly.
            // sleep(challenge_period - (counterparty_timestamp - last_consensus_update)).await;
            let sleep_duration =
                challenge_period - (counterparty_timestamp - last_consensus_update);
            sleep(sleep_duration).await;
        }

        let interval_stream = stream::try_unfold((), move |state| {
            let client = client.clone();
            let counterparty = counterparty.clone();
            let prover = client.prover.clone();
            // Let the interval be the length of the challenge period
            let mut interval = interval(challenge_period);
            async move {
                loop {
                    interval.tick().await;
                    let last_consensus_update = counterparty
                        .query_state_machine_update_time(client.consensus_state_id.clone())
                        .await?;
                    let counterparty_timestamp = counterparty.query_timestamp().await?;
                    // If onchain timestamp has not progressed wait for next tick of the interval
                    if counterparty_timestamp - last_consensus_update < challenge_period {
                        continue
                    } else {
                        break
                    }
                }
                return match client.state_machine {
                    StateMachine::Polkadot(_) | StateMachine::Kusama(_) => {
                        let consensus_state_bytes = counterparty
                            .query_consensus_state(None, client.consensus_state_id.clone())
                            .await?;

                        let consensus_state: ConsensusState =
                            codec::Decode::decode(&mut &consensus_state_bytes[..])?;

                        let next_relay_height = consensus_state.latest_height + 1;

                        let finality_proof = prover
                            .query_finality_proof::<SubstrateHeader<u32, BlakeTwo256>>(
                                consensus_state.latest_height,
                                next_relay_height,
                            )
                            .await?;

                        let justification =
                            Justification::decode(&mut &finality_proof.justification[..])?;

                        let parachain_headers_with_proof = prover
                                .query_finalized_parachain_headers_with_proof::<SubstrateHeader<u32, BlakeTwo256>>(
                                    consensus_state.latest_height.clone(),
                                    justification.commit.target_number,
                                    finality_proof.clone(),
                                )
                                .await?;
                        let relay_chain_message = RelayChainMessage {
                            finality_proof: codec::Decode::decode(
                                &mut &parachain_headers_with_proof.finality_proof.encode()[..],
                            )?,
                            parachain_headers: parachain_headers_with_proof.parachain_headers,
                        };
                        let message = ConsensusMessage {
                            consensus_proof:
                                ismp_grandpa::messages::ConsensusMessage::RelayChainMessage(
                                    relay_chain_message,
                                )
                                .encode(),
                            consensus_state_id: client.consensus_state_id.clone(),
                        };

                        Ok(Some((message, state)))
                    }
                    StateMachine::Grandpa(_) => {
                        // Query finality proof
                        let consensus_state_bytes = counterparty
                            .query_consensus_state(None, client.consensus_state_id)
                            .await?;

                        let consensus_state: ConsensusState =
                            codec::Decode::decode(&mut &consensus_state_bytes[..])?;

                        let next_relay_height = consensus_state.latest_height + 1;

                        let finality_proof = prover
                            .query_finality_proof::<SubstrateHeader<u32, BlakeTwo256>>(
                                consensus_state.latest_height,
                                next_relay_height,
                            )
                            .await?;
                        let standalone_message = StandaloneChainMessage {
                            finality_proof: codec::Decode::decode(
                                &mut &finality_proof.encode()[..],
                            )?,
                        };
                        let message = ConsensusMessage {
                            consensus_proof:
                                ismp_grandpa::messages::ConsensusMessage::StandaloneChainMessage(
                                    standalone_message,
                                )
                                .encode(),
                            consensus_state_id: client.consensus_state_id,
                        };

                        Ok(Some((message, state)))
                    }
                    _ => Ok(None),
                }
            }
        });

        Ok(Box::pin(interval_stream))
    }
}
