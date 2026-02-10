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

//! Backend abstraction for consensus proof communication between prover and host.
//!
//! This module provides a unified trait and implementations for different backend systems,
//! allowing the prover and host to communicate without being tightly coupled to Redis.

mod memory;
mod redis;

pub use memory::InMemoryProofBackend;
pub use redis::{RedisConfig, RedisProofBackend};

use codec::{Decode, Encode};
use futures::Stream;
use ismp::{host::StateMachine, messaging::ConsensusMessage};
use std::pin::Pin;

/// Consensus proof message exchanged between prover and host
#[derive(Clone, Debug, Encode, Decode)]
pub struct ConsensusProof {
	/// The height that is now finalized by this consensus message
	pub finalized_height: u32,
	/// The validator set id responsible for signing this message
	pub set_id: u64,
	/// The consensus message in question
	pub message: ConsensusMessage,
}

/// Unified trait for proof communication backend between prover and host.
/// Combines queue operations, state storage, and notification mechanisms.
#[async_trait::async_trait]
pub trait ProofBackend: Send + Sync {
	// ============================================================================
	// Queue Operations (used by both prover and host)
	// ============================================================================

	/// Initialize the queues for the given state machines
	async fn init_queues(&self, state_machines: &[StateMachine]) -> Result<(), anyhow::Error>;

	// ============================================================================
	// Prover Operations (sending proofs and managing state)
	// ============================================================================

	/// Send a mandatory consensus proof (authority set changes)
	async fn send_mandatory_proof(
		&self,
		state_machine: &StateMachine,
		proof: ConsensusProof,
	) -> Result<(), anyhow::Error>;

	/// Send a messages consensus proof (new messages finalized)
	async fn send_messages_proof(
		&self,
		state_machine: &StateMachine,
		proof: ConsensusProof,
	) -> Result<(), anyhow::Error>;

	/// Save the prover's consensus state
	async fn save_state(
		&self,
		state: &crate::prover::ProverConsensusState,
	) -> Result<(), anyhow::Error>;

	/// Load the prover's consensus state
	async fn load_state(&self) -> Result<crate::prover::ProverConsensusState, anyhow::Error>;

	// ============================================================================
	// Host Operations (consuming proofs)
	// ============================================================================

	/// Subscribe to queue notifications for a specific state machine
	async fn queue_notifications(
		&self,
		state_machine: StateMachine,
	) -> Result<
		Pin<Box<dyn Stream<Item = Result<StreamMessage, anyhow::Error>> + Send>>,
		anyhow::Error,
	>;

	/// Receive a mandatory consensus proof from the queue
	async fn receive_mandatory_proof(
		&self,
		state_machine: &StateMachine,
	) -> Result<Option<QueueMessage>, anyhow::Error>;

	/// Receive a messages consensus proof from the queue
	async fn receive_messages_proof(
		&self,
		state_machine: &StateMachine,
	) -> Result<Option<QueueMessage>, anyhow::Error>;

	/// Delete a message from the queue after processing
	async fn delete_message(
		&self,
		state_machine: &StateMachine,
		message_id: &str,
		message_type: StreamMessage,
	) -> Result<(), anyhow::Error>;

	/// Attempt to recreate/reconnect the notification subscription (for connection recovery)
	/// Default implementation does nothing (used by backends that don't need reconnection)
	async fn reconnect_notifier(&self) -> Result<(), anyhow::Error> {
		Ok(())
	}
}

/// Types of notifications from the queue
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StreamMessage {
	/// The current authority set has handed over to the next
	EpochChanged,
	/// Some new messages can now be finalized
	NewMessages,
}

/// A message received from the queue
#[derive(Debug, Clone)]
pub struct QueueMessage {
	/// Unique identifier for this message in the queue
	pub id: String,
	/// The consensus proof
	pub proof: ConsensusProof,
}
