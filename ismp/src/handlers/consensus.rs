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
    consensus::{StateMachineHeight, StateMachineId},
    error::Error,
    handlers::{ConsensusClientCreatedResult, ConsensusUpdateResult, MessageResult},
    host::IsmpHost,
    messaging::{ConsensusMessage, CreateConsensusClient, FraudProofMessage},
};
use alloc::collections::BTreeSet;

/// This function handles verification of consensus messages for consensus clients
pub fn update_client<H>(host: &H, msg: ConsensusMessage) -> Result<MessageResult, Error>
where
    H: IsmpHost,
{
    let consensus_client = host.consensus_client(msg.consensus_client_id)?;
    let trusted_state = host.consensus_state(msg.consensus_client_id)?;

    let update_time = host.consensus_update_time(msg.consensus_client_id)?;
    let delay = host.challenge_period(msg.consensus_client_id);
    let now = host.timestamp();

    host.is_consensus_client_frozen(msg.consensus_client_id)?;

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
    for (id, commitment_height) in intermediate_states {
        let id = StateMachineId { state_id: id, consensus_client: msg.consensus_client_id };
        let state_height = StateMachineHeight { id, height: commitment_height.height };
        // If a state machine is frozen, we skip it
        if host.is_state_machine_frozen(state_height).is_err() {
            continue
        }

        let previous_latest_height = host.latest_commitment_height(id)?;

        // Only allow heights greater than latest height
        if previous_latest_height > commitment_height.height {
            continue
        }

        // Skip duplicate states
        if host.state_machine_commitment(state_height).is_ok() {
            continue
        }

        host.store_state_machine_commitment(state_height, commitment_height.commitment)?;

        state_updates
            .insert((StateMachineHeight { id, height: previous_latest_height }, state_height));
        host.store_latest_commitment_height(state_height)?;
    }

    let result =
        ConsensusUpdateResult { consensus_client_id: msg.consensus_client_id, state_updates };

    Ok(MessageResult::ConsensusMessage(result))
}

/// Handles the creation of consensus clients
pub fn create_client<H>(
    host: &H,
    message: CreateConsensusClient,
) -> Result<ConsensusClientCreatedResult, Error>
where
    H: IsmpHost,
{
    // check that we have an implementation of this client
    host.consensus_client(message.consensus_client_id)?;

    // Store the initial state for the consensus client
    host.store_consensus_state(message.consensus_client_id, message.consensus_state)?;

    // Store all intermedite state machine commitments
    for (id, state_commitment) in message.state_machine_commitments {
        let height = StateMachineHeight { id, height: state_commitment.height };
        host.store_state_machine_commitment(height, state_commitment.commitment)?;
        host.store_latest_commitment_height(height)?;
    }

    host.store_consensus_update_time(message.consensus_client_id, host.timestamp())?;

    Ok(ConsensusClientCreatedResult { consensus_client_id: message.consensus_client_id })
}

/// Freeze a consensus client by providing a valid fraud proof.
pub fn freeze_client<H>(host: &H, msg: FraudProofMessage) -> Result<MessageResult, Error>
where
    H: IsmpHost,
{
    let consensus_client = host.consensus_client(msg.consensus_client_id)?;
    let trusted_state = host.consensus_state(msg.consensus_client_id)?;

    consensus_client.verify_fraud_proof(host, trusted_state, msg.proof_1, msg.proof_2)?;

    host.freeze_consensus_client(msg.consensus_client_id)?;

    host.store_consensus_update_time(msg.consensus_client_id, host.timestamp())?;

    Ok(MessageResult::FrozenClient(msg.consensus_client_id))
}
