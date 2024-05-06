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

use crate::BeefyHost;
use anyhow::Error;
use beefy_verifier_primitives::ConsensusState;
use codec::Decode;

use ethers::abi::AbiEncode;
use futures::stream;
use ismp::messaging::{ConsensusMessage, CreateConsensusState};

use futures::StreamExt;
use ismp_solidity_abi::beefy::BeefyConsensusState;
use std::sync::Arc;
use subxt::{config::Header, ext::sp_runtime::traits::Zero};
use tesseract_primitives::{BoxStream, IsmpHost, IsmpProvider};
use tokio::sync::broadcast;

#[async_trait::async_trait]
impl<R, P> IsmpHost for BeefyHost<R, P>
where
    R: subxt::Config + Send + Sync + Clone,
    P: subxt::Config + Send + Sync + Clone,

    <R::Header as Header>::Number: Ord + Zero,
    u32: From<<R::Header as Header>::Number>,
    sp_core::H256: From<R::Hash>,
    R::Header: codec::Decode,

    <P::Header as Header>::Number: Ord + Zero,
    u32: From<<P::Header as Header>::Number>,
    sp_core::H256: From<P::Hash>,
    P::Header: codec::Decode,
{
    async fn consensus_notification(
        &self,
        counterparty: Arc<dyn IsmpProvider>,
    ) -> Result<BoxStream<ConsensusMessage>, anyhow::Error> {
        let receiver = self.sender.subscribe();
        let consensus_state_id = self.consensus_state_id;
        let stream = stream::unfold(receiver, move |mut receiver| {
            let counterparty = counterparty.clone();
            async move {
                let (message, latest_beefy_height, set_id) = match receiver.recv().await {
                    Ok(m) => m,
                    Err(err) => {
                        return match err {
                            broadcast::error::RecvError::Closed => None,
                            broadcast::error::RecvError::Lagged(_) => {
                                Some((Ok::<_, anyhow::Error>(None), receiver))
                            }
                        }
                    }
                };

                match counterparty
                    .query_consensus_state(None, consensus_state_id)
                    .await
                {
                    Ok(consensus_state) => {
                        let consensus_state = ConsensusState::decode(&mut &consensus_state[..])
                            .expect("Infallible, consensus state was encoded correctly");

                        if latest_beefy_height > consensus_state.latest_beefy_height
                            && (set_id == consensus_state.current_authorities.id
                                || set_id == consensus_state.next_authorities.id)
                        {
                            return Some((Ok(Some(message)), receiver));
                        }

                        Some((Ok(None), receiver))
                    }
                    Err(err) => Some((Err(err), receiver)),
                }
            }
        })
        .filter_map(|res| async move {
            match res {
                Ok(Some(update)) => Some(Ok(update)),
                Ok(None) => None,
                Err(err) => Some(Err(err)),
            }
        });

        Ok(Box::pin(stream))
    }

    async fn query_initial_consensus_state(&self) -> Result<Option<CreateConsensusState>, Error> {
        let consensus_state: BeefyConsensusState = self
            .prover
            .inner()
            .get_initial_consensus_state()
            .await?
            .into();
        Ok(Some(CreateConsensusState {
            consensus_state: consensus_state.encode(),
            consensus_client_id: *b"BEEF",
            consensus_state_id: self.consensus_state_id,
            unbonding_period: 60 * 60 * 60 * 27,
            challenge_period: 5 * 60,
            state_machine_commitments: vec![],
        }))
    }

    fn provider(&self) -> Arc<dyn IsmpProvider> {
        self.provider.clone()
    }
}
