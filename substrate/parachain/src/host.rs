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

use crate::{
    relay_chain, relay_chain::api::runtime_types::polkadot_parachain::primitives::Id, ParachainHost,
};
use anyhow::anyhow;
use codec::{Decode, Encode};
use futures::stream;
use ismp::{consensus::StateMachineId, host::StateMachine, messaging::ConsensusMessage};
use ismp_parachain::consensus::{ParachainConsensusProof, PARACHAIN_CONSENSUS_ID};
use subxt::ext::sp_runtime::{generic::Header, traits::BlakeTwo256, DigestItem};
use tesseract_primitives::{BoxStream, IsmpHost, IsmpProvider};

#[async_trait::async_trait]
impl<T> IsmpHost for ParachainHost<T>
where
    T: subxt::Config + Send + Sync + Clone,
    T::Header: Send + Sync,
{
    async fn consensus_notification<C>(
        &self,
        counterparty: C,
    ) -> Result<BoxStream<ConsensusMessage>, anyhow::Error>
    where
        C: IsmpHost + IsmpProvider + 'static,
    {
        let client = ParachainHost::clone(&self);

        let stream = stream::try_unfold((), move |state| {
            let client = client.clone();
            let counterparty = counterparty.clone();

            async move {
                match client.state_machine {
                    StateMachine::Polkadot(id) | StateMachine::Kusama(id) => {
                        // we know there's no challenge period
                        let mut subscription =
                            client.parachain.rpc().subscribe_best_block_headers().await?;

                        while let Some(Ok(header)) = subscription.next().await {
                            let header =
                                Header::<u32, BlakeTwo256>::decode(&mut &*header.encode())?;

                            let digest = header.digest.logs.iter().find(|d| match d {
                                DigestItem::Consensus(id, _) if *id == PARACHAIN_CONSENSUS_ID => {
                                    true
                                }
                                _ => false,
                            });

                            let relay_height =
                                if let Some(DigestItem::Consensus(_, height)) = digest {
                                    u32::decode(&mut &height[..])?
                                } else {
                                    continue
                                };

                            let relay_block_hash = client
                                .relay_chain
                                .rpc()
                                .block_hash(Some(relay_height.into()))
                                .await?
                                .ok_or_else(|| {
                                    anyhow!(
                                        "Can't find relay chain block for height {relay_height}"
                                    )
                                })?;

                            let key = relay_chain::api::storage().paras().heads(Id(id));
                            let header_bytes = client
                                .relay_chain
                                .storage()
                                .at(relay_block_hash)
                                .fetch(&key)
                                .await?
                                .ok_or_else(|| {
                                    anyhow!(
                                        "Parachain with ParaId({id}) not found on the relay chain"
                                    )
                                })?
                                .0;
                            let header = Header::<u32, BlakeTwo256>::decode(&mut &*header_bytes)?;

                            if header.number == 0 {
                                // skip the genesis block
                                continue
                            }

                            let latest_height = counterparty
                                .query_latest_state_machine_height(StateMachineId {
                                    state_id: client.state_machine,
                                    consensus_state_id: PARACHAIN_CONSENSUS_ID,
                                })
                                .await?;

                            // check header height
                            if header.number <= latest_height {
                                continue
                            }

                            // todo: check for any ismp::{Request, Response} events

                            let full_key = client.relay_chain.storage().address_bytes(&key)?;
                            let storage_proof = client
                                .relay_chain
                                .rpc()
                                .read_proof(vec![full_key.as_slice()], Some(relay_block_hash))
                                .await?
                                .proof
                                .into_iter()
                                .map(|b| b.0)
                                .collect();

                            let proof = ParachainConsensusProof {
                                para_ids: vec![id],
                                relay_height,
                                storage_proof,
                            };

                            let message = ConsensusMessage {
                                consensus_proof: proof.encode(),
                                consensus_state_id: PARACHAIN_CONSENSUS_ID,
                            };

                            return Ok(Some((message, state)))
                        }
                    }
                    state_machine => panic!("Unsupported state machine: {state_machine:?}"),
                };

                return Ok(None)
            }
        });

        Ok(Box::pin(stream))
    }
}
