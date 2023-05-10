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

//! The ISMP consensus handler

use crate::{
    error::Error,
    handlers::{ConsensusClientCreatedResult, ConsensusUpdateResult, MessageResult},
    host::ISMPHost,
    messaging::{ConsensusMessage, CreateConsensusClient},
};
use alloc::collections::BTreeSet;

/// This function handles verification of consensus messages for consensus clients
pub fn handle<H>(host: &H, msg: ConsensusMessage) -> Result<MessageResult, Error>
where
    H: ISMPHost,
{
    let consensus_client = host.consensus_client(msg.consensus_client_id)?;
    let trusted_state = host.consensus_state(msg.consensus_client_id)?;

    let update_time = host.consensus_update_time(msg.consensus_client_id)?;
    let delay = host.challenge_period(msg.consensus_client_id);
    let now = host.timestamp();

    // Ensure client is not frozen
    consensus_client.is_frozen(&trusted_state)?;

    if (now - update_time) <= delay {
        Err(Error::ChallengePeriodNotElapsed {
            consensus_id: msg.consensus_client_id,
            current_time: now,
            update_time,
        })?
    }

    host.is_expired(msg.consensus_client_id)?;

    let (new_state, intermediate_states) =
        consensus_client.verify_consensus(host, trusted_state, msg.consensus_proof)?;
    host.store_consensus_state(msg.consensus_client_id, new_state)?;
    let timestamp = host.timestamp();
    host.store_consensus_update_time(msg.consensus_client_id, timestamp)?;
    let mut state_updates = BTreeSet::new();
    for intermediate_state in intermediate_states {
        // If a state machine is frozen, we skip it
        if host.is_frozen(intermediate_state.height)? {
            continue
        }

        let previous_latest_height = host.latest_commitment_height(intermediate_state.height.id)?;

        // Only allow heights greater than latest height
        if previous_latest_height > intermediate_state.height {
            continue
        }

        // Skip duplicate states
        if host.state_machine_commitment(intermediate_state.height).is_ok() {
            continue
        }

        host.store_state_machine_commitment(
            intermediate_state.height,
            intermediate_state.commitment,
        )?;

        state_updates.insert((previous_latest_height, intermediate_state.height));
        host.store_latest_commitment_height(intermediate_state.height)?;
    }

    let result =
        ConsensusUpdateResult { consensus_client_id: msg.consensus_client_id, state_updates };

    Ok(MessageResult::ConsensusMessage(result))
}

/// Handles the creation of consensus clients
pub fn create_consensus_client<H>(
    host: &H,
    message: CreateConsensusClient,
) -> Result<ConsensusClientCreatedResult, Error>
where
    H: ISMPHost,
{
    // Store the initial state for the consensus client
    host.store_consensus_state(message.consensus_client_id, message.consensus_state)?;

    // Store all intermedite state machine commitments
    for intermediate_state in message.state_machine_commitments {
        host.store_state_machine_commitment(
            intermediate_state.height,
            intermediate_state.commitment,
        )?;
        host.store_latest_commitment_height(intermediate_state.height)?;
    }

    host.store_consensus_update_time(message.consensus_client_id, host.timestamp())?;

    Ok(ConsensusClientCreatedResult { consensus_client_id: message.consensus_client_id })
}
