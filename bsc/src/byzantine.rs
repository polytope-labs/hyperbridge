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

use anyhow::anyhow;
use ismp::{
    consensus::{StateMachineHeight, StateMachineId},
    events::StateMachineUpdated,
};
use tesseract_primitives::{ByzantineHandler, IsmpHost};

use crate::BscPosHost;

#[async_trait::async_trait]
impl ByzantineHandler for BscPosHost {
    async fn check_for_byzantine_attack(
        &self,
        counterparty: Arc<dyn IsmpHost>,
        event: StateMachineUpdated,
    ) -> Result<(), anyhow::Error> {
        let header = self
            .prover
            .fetch_header(event.latest_height)
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Failed to fetch header in {:?} byzantine handler",
                    self.state_machine
                )
            })?;
        let counterparty_provider = counterparty.provider();
        let height = StateMachineHeight {
            id: StateMachineId {
                state_id: self.state_machine,
                consensus_state_id: self.consensus_state_id,
            },
            height: event.latest_height,
        };
        let state_machine_commitment = counterparty_provider
            .query_state_machine_commitment(height)
            .await?;
        if header.state_root != state_machine_commitment.state_root {
            log::info!(
                "Vetoing State Machine Update for {:?} on {:?}",
                self.state_machine,
                counterparty_provider.state_machine_id().state_id
            );
            counterparty_provider.veto_state_commitment(height).await?;
        }

		Ok(())
	}
}
