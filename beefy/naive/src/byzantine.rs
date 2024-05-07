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

use std::sync::Arc;

use crate::BeefyHost;
use anyhow::anyhow;
use codec::{Decode, Encode};
use ismp::{
    consensus::{StateMachineHeight, StateMachineId},
    events::StateMachineUpdated
};
use sp_core::H256;
use subxt::{config::substrate::SubstrateHeader, Config};
use tesseract_primitives::{ByzantineHandler, IsmpHost};

#[async_trait::async_trait]
impl<R, P> ByzantineHandler for BeefyHost<R, P>
where
    R: subxt::Config + Send + Sync + Clone,
    P: subxt::Config + Send + Sync + Clone,
    H256: From<<P as Config>::Hash>,
{
    async fn check_for_byzantine_attack(
        &self,
        counterparty: Arc<dyn IsmpHost>,
        event: StateMachineUpdated,
    ) -> Result<(), anyhow::Error> {
        let height = StateMachineHeight {
            id: StateMachineId {
                state_id: self.provider.state_machine_id().state_id,
                consensus_state_id: self.consensus_state_id,
            },
            height: event.latest_height,
        };

        let header = match &self.prover {
            crate::prover::Prover::Naive(prover) =>  {
                let block_hash = prover.para.rpc().block_hash(Some(event.latest_height.into())).await?.ok_or_else(|| anyhow!("Failed to get block header in byzantine handler"))?;
                prover.para.rpc().header(Some(block_hash)).await?.ok_or_else(|| anyhow!("Failed to get block header in byzantine handler"))?
            },
            crate::prover::Prover::ZK(prover) => {
                let block_hash = prover.inner.para.rpc().block_hash(Some(event.latest_height.into())).await?.ok_or_else(|| anyhow!("Failed to get block header in byzantine handler"))?;
                prover.inner.para.rpc().header(Some(block_hash)).await?.ok_or_else(|| anyhow!("Failed to get block header in byzantine handler"))?
            },
        };

        let header = SubstrateHeader::<u32, P::Hasher>::decode(&mut &*header.encode())?;

        let counterparty_provider = counterparty.provider();
        let finalized_state_commitment = counterparty_provider
            .query_state_machine_commitment(height)
            .await?;

        if finalized_state_commitment.state_root != header.state_root.into() {
            log::info!(
                "Vetoing state commitment for {:?} on {:?}",
                self.provider.state_machine_id().state_id,
                counterparty_provider.state_machine_id().state_id
            );
            counterparty_provider.veto_state_commitment(height).await?;
        }

        Ok(())
    }
}
