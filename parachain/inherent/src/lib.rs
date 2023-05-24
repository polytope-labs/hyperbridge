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
#![deny(missing_docs)]

//! ISMP Parachain Consensus Inherent Provider
//!
//! This exports the inherent provider for including ISMP parachain consensus updates as block
//! inherents.

use codec::Encode;
use cumulus_primitives_core::PersistedValidationData;
use cumulus_relay_chain_interface::{PHash, RelayChainInterface};
use ismp::messaging::ConsensusMessage;
use ismp_parachain::consensus::{parachain_header_storage_key, ParachainConsensusProof};
use ismp_parachain_runtime_api::IsmpParachainApi;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

/// Implements [`InherentDataProvider`] for providing parachain consensus updates as inherents.
pub struct ConsensusInherentProvider(Option<ConsensusMessage>);

impl ConsensusInherentProvider {
    /// Create the [`ConsensusMessage`] at the given `relay_parent`. Will be [`None`] if no para ids
    /// have been confguired.
    pub async fn create<C, B>(
        client: Arc<C>,
        relay_parent: PHash,
        relay_chain_interface: &impl RelayChainInterface,
        validation_data: PersistedValidationData,
    ) -> Result<ConsensusInherentProvider, anyhow::Error>
    where
        C: sp_api::ProvideRuntimeApi<B> + sp_blockchain::HeaderBackend<B>,
        C::Api: IsmpParachainApi<B>,
        B: BlockT,
    {
        let head = client.info().best_hash;
        let para_ids = client.runtime_api().para_ids(head)?;

        if para_ids.is_empty() {
            return Ok(ConsensusInherentProvider(None))
        }

        let keys = para_ids.iter().map(|id| parachain_header_storage_key(*id).0).collect();
        let storage_proof = relay_chain_interface
            .prove_read(relay_parent, &keys)
            .await?
            .into_iter_nodes()
            .collect();

        let consensus_proof = ParachainConsensusProof {
            para_ids,
            relay_height: validation_data.relay_parent_number,
            storage_proof,
        };
        let message = ConsensusMessage {
            consensus_client_id: ismp_parachain::consensus::PARACHAIN_CONSENSUS_ID,
            consensus_proof: consensus_proof.encode(),
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
