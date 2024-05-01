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

#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

use std::sync::Arc;

use anyhow::anyhow;
use codec::{Decode, Encode};
use cumulus_relay_chain_interface::RelayChainInterface;
use sp_runtime::{
    generic::{BlockId, Header},
    traits::{BlakeTwo256, Block as BlockT},
};

use ismp::{consensus::StateMachineId, host::StateMachine, messaging::ConsensusMessage};
use ismp_parachain::{
    consensus::{parachain_header_storage_key, ParachainConsensusProof},
    PARACHAIN_CONSENSUS_ID,
};
use ismp_parachain_runtime_api::IsmpParachainApi;
use pallet_ismp_runtime_api::IsmpRuntimeApi;

/// Implements [`InherentDataProvider`](sp_inherents::InherentDataProvider) for providing parachain
/// consensus updates as inherents.
pub struct ConsensusInherentProvider(Option<ConsensusMessage>);

impl ConsensusInherentProvider {
    /// Create the [`ConsensusMessage`] for the latest height. Will be [`None`] if no para ids have
    /// been configured.
    pub async fn create<C, B>(
        parent: B::Hash,
        client: Arc<C>,
        relay_chain_interface: Arc<dyn RelayChainInterface>,
    ) -> Result<ConsensusInherentProvider, anyhow::Error>
    where
        C: sp_api::ProvideRuntimeApi<B> + sp_blockchain::HeaderBackend<B>,
        C::Api: IsmpParachainApi<B> + IsmpRuntimeApi<B, B::Hash>,
        B: BlockT,
    {
        let para_ids = client.runtime_api().para_ids(parent)?;

        if para_ids.is_empty() {
            return Ok(ConsensusInherentProvider(None));
        }

        let state = client.runtime_api().current_relay_chain_state(parent)?;

        // parachain is just starting
        if state.number == 0u32 {
            return Ok(ConsensusInherentProvider(None));
        }

        let relay_header = relay_chain_interface
            .header(BlockId::Number(state.number))
            .await?
            .ok_or_else(|| anyhow!("Relay chain header for height {} not found", state.number))?;

        let mut para_ids_to_fetch = vec![];
        for id in para_ids {
            let Some(head) = relay_chain_interface
                .get_storage_by_key(relay_header.hash(), parachain_header_storage_key(id).as_ref())
                .await?
            else {
                continue
            };

            let Ok(intermediate) = Vec::<u8>::decode(&mut &head[..]) else {
                continue;
            };

            let Ok(header) = Header::<u32, BlakeTwo256>::decode(&mut &intermediate[..]) else {
                continue;
            };

            let state_id = match client.runtime_api().host_state_machine(parent)? {
                StateMachine::Polkadot(_) => StateMachine::Polkadot(id),
                StateMachine::Kusama(_) => StateMachine::Kusama(id),
                id => Err(anyhow!("Unsupported state machine: {id:?}"))?,
            };
            let Some(height) = client.runtime_api().latest_state_machine_height(
                parent,
                StateMachineId { consensus_state_id: PARACHAIN_CONSENSUS_ID, state_id },
            )?
            else {
                continue
            };

            if height >= header.number as u64 {
                continue
            }

            para_ids_to_fetch.push(id);
        }

        if para_ids_to_fetch.is_empty() {
            return Ok(ConsensusInherentProvider(None));
        }

        let keys = para_ids_to_fetch.iter().map(|id| parachain_header_storage_key(*id).0).collect();
        let storage_proof = relay_chain_interface
            .prove_read(relay_header.hash(), &keys)
            .await?
            .into_iter_nodes()
            .collect();

        let consensus_proof = ParachainConsensusProof { relay_height: state.number, storage_proof };
        let message = ConsensusMessage {
            consensus_state_id: PARACHAIN_CONSENSUS_ID,
            consensus_proof: consensus_proof.encode(),
            signer: Default::default(),
        };

        Ok(ConsensusInherentProvider(Some(message)))
    }
}

#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for ConsensusInherentProvider {
    async fn provide_inherent_data(
        &self,
        inherent_data: &mut sp_inherents::InherentData,
    ) -> Result<(), sp_inherents::Error> {
        if let Some(ref message) = self.0 {
            inherent_data.put_data(ismp_parachain::INHERENT_IDENTIFIER, message)?;
        }

        Ok(())
    }

    async fn try_handle_error(
        &self,
        _: &sp_inherents::InherentIdentifier,
        _: &[u8],
    ) -> Option<Result<(), sp_inherents::Error>> {
        None
    }
}
