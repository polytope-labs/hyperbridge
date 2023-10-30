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

//! ISMP handler definitions
use crate::{
    consensus::{ConsensusClientId, StateMachineClient, StateMachineHeight},
    error::Error,
    host::IsmpHost,
    messaging::Message,
};

use crate::{consensus::ConsensusStateId, module::DispatchResult};
use alloc::{boxed::Box, collections::BTreeSet, vec::Vec};
pub use consensus::create_client;

mod consensus;
mod request;
mod response;
mod timeout;

/// The result of successfully processing a [`ConsensusMessage`]
#[derive(Debug)]
pub struct ConsensusUpdateResult {
    /// Consensus client Id
    pub consensus_client_id: ConsensusClientId,
    /// Consensus state Id
    pub consensus_state_id: ConsensusStateId,
    /// Tuple of previous latest height and new latest height for a state machine
    pub state_updates: BTreeSet<(StateMachineHeight, StateMachineHeight)>,
}

/// The result of successfully processing a [`CreateConsensusClient`] message
pub struct ConsensusClientCreatedResult {
    /// Consensus client Id
    pub consensus_client_id: ConsensusClientId,
    /// Consensus state Id
    pub consensus_state_id: ConsensusStateId,
}

/// Result returned when ismp messages are handled successfully
#[derive(Debug)]
pub enum MessageResult {
    /// The [`ConsensusMessage`] result
    ConsensusMessage(ConsensusUpdateResult),
    /// Result of freezing a consensus state.
    FrozenClient(ConsensusStateId),
    /// The [`DispatchResult`] for requests
    Request(Vec<DispatchResult>),
    /// The [`DispatchResult`] for responses
    Response(Vec<DispatchResult>),
    /// The [`DispatchResult`] for timeouts
    Timeout(Vec<DispatchResult>),
}

/// This function serves as an entry point to handle the message types provided by the ISMP protocol
pub fn handle_incoming_message<H>(host: &H, message: Message) -> Result<MessageResult, Error>
where
    H: IsmpHost,
{
    match message {
        Message::Consensus(consensus_message) => consensus::update_client(host, consensus_message),
        Message::FraudProof(fraud_proof) => consensus::freeze_client(host, fraud_proof),
        Message::Request(req) => request::handle(host, req),
        Message::Response(resp) => response::handle(host, resp),
        Message::Timeout(timeout) => timeout::handle(host, timeout),
    }
}

/// This function checks to see that the delay period configured on the host chain
/// for the state machine has elasped.
pub fn verify_delay_passed<H>(host: &H, proof_height: &StateMachineHeight) -> Result<bool, Error>
where
    H: IsmpHost,
{
    let update_time = host.state_machine_update_time(*proof_height)?;
    let delay_period = host.challenge_period(proof_height.id.consensus_state_id).ok_or(
        Error::ChallengePeriodNotConfigured {
            consensus_state_id: proof_height.id.consensus_state_id,
        },
    )?;
    let current_timestamp = host.timestamp();
    Ok(current_timestamp - update_time > delay_period)
}

/// This function does the preliminary checks for a request or response message
/// - It ensures the consensus client is not frozen
/// - It ensures the state machine is not frozen
/// - Checks that the delay period configured for the state machine has elaspsed.
fn validate_state_machine<H>(
    host: &H,
    proof_height: StateMachineHeight,
) -> Result<Box<dyn StateMachineClient>, Error>
where
    H: IsmpHost,
{
    // Ensure consensus client is not frozen
    let consensus_client_id = host.consensus_client_id(proof_height.id.consensus_state_id).ok_or(
        Error::ConsensusStateIdNotRecognized {
            consensus_state_id: proof_height.id.consensus_state_id,
        },
    )?;
    let consensus_client = host.consensus_client(consensus_client_id)?;
    // Ensure client is not frozen
    host.is_consensus_client_frozen(proof_height.id.consensus_state_id)?;

    // Ensure state machine is not frozen
    host.is_state_machine_frozen(proof_height)?;

    // Ensure delay period has elapsed
    // if !verify_delay_passed(host, &proof_height)? {
    //     return Err(Error::ChallengePeriodNotElapsed {
    //         consensus_state_id: proof_height.id.consensus_state_id,
    //         current_time: host.timestamp(),
    //         update_time: host.state_machine_update_time(proof_height)?,
    //     })
    // }

    consensus_client.state_machine(proof_height.id.state_id)
}
