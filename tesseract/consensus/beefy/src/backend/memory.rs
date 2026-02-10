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

//! In-memory implementation of the ProofBackend trait

use super::{ConsensusProof, ProofBackend, QueueMessage, StreamMessage};
use futures::Stream;
use ismp::host::StateMachine;
use std::{
	collections::{HashMap, VecDeque},
	pin::Pin,
	sync::Arc,
};
use tokio::sync::RwLock;

/// In-memory implementation of the unified proof backend (for testing or single-process use)
pub struct InMemoryProofBackend {
	mandatory_queues: Arc<RwLock<HashMap<StateMachine, VecDeque<ConsensusProof>>>>,
	messages_queues: Arc<RwLock<HashMap<StateMachine, VecDeque<ConsensusProof>>>>,
	notifier: Arc<tokio::sync::broadcast::Sender<(StateMachine, StreamMessage)>>,
	state: Arc<RwLock<Option<crate::prover::ProverConsensusState>>>,
}

impl InMemoryProofBackend {
	pub fn new(initial_state: crate::prover::ProverConsensusState) -> Self {
		let (tx, _) = tokio::sync::broadcast::channel(1000);
		Self {
			mandatory_queues: Arc::new(RwLock::new(HashMap::new())),
			messages_queues: Arc::new(RwLock::new(HashMap::new())),
			notifier: Arc::new(tx),
			state: Arc::new(RwLock::new(Some(initial_state))),
		}
	}
}

#[async_trait::async_trait]
impl ProofBackend for InMemoryProofBackend {
	async fn init_queues(&self, state_machines: &[StateMachine]) -> Result<(), anyhow::Error> {
		let mut mandatory = self.mandatory_queues.write().await;
		let mut messages = self.messages_queues.write().await;

		for state_machine in state_machines {
			mandatory.entry(*state_machine).or_insert_with(VecDeque::new);
			messages.entry(*state_machine).or_insert_with(VecDeque::new);
		}

		Ok(())
	}

	async fn send_mandatory_proof(
		&self,
		state_machine: &StateMachine,
		proof: ConsensusProof,
	) -> Result<(), anyhow::Error> {
		let mut queues = self.mandatory_queues.write().await;
		queues.entry(*state_machine).or_insert_with(VecDeque::new).push_back(proof);

		// Notify consumers
		let _ = self.notifier.send((*state_machine, StreamMessage::EpochChanged));

		Ok(())
	}

	async fn send_messages_proof(
		&self,
		state_machine: &StateMachine,
		proof: ConsensusProof,
	) -> Result<(), anyhow::Error> {
		let mut queues = self.messages_queues.write().await;
		queues.entry(*state_machine).or_insert_with(VecDeque::new).push_back(proof);

		// Notify consumers
		let _ = self.notifier.send((*state_machine, StreamMessage::NewMessages));

		Ok(())
	}

	async fn save_state(
		&self,
		state: &crate::prover::ProverConsensusState,
	) -> Result<(), anyhow::Error> {
		*self.state.write().await = Some(state.clone());
		Ok(())
	}

	async fn load_state(&self) -> Result<crate::prover::ProverConsensusState, anyhow::Error> {
		self.state
			.read()
			.await
			.clone()
			.ok_or_else(|| anyhow::anyhow!("No consensus state stored in memory"))
	}

	async fn queue_notifications(
		&self,
		state_machine: StateMachine,
	) -> Result<
		Pin<Box<dyn Stream<Item = Result<StreamMessage, anyhow::Error>> + Send>>,
		anyhow::Error,
	> {
		use futures::StreamExt;

		let mut rx = self.notifier.subscribe();

		// Yield once immediately to check for existing messages
		let initial = futures::stream::iter(vec![Ok(StreamMessage::EpochChanged)]);

		let notifications = async_stream::stream! {
			loop {
				match rx.recv().await {
					Ok((sm, msg)) if sm == state_machine => yield Ok(msg),
					Ok(_) => continue, // Message for different state machine
					Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
						// We missed some messages, but that's okay - we'll catch up
						continue;
					}
					Err(tokio::sync::broadcast::error::RecvError::Closed) => {
						yield Err(anyhow::anyhow!("Notification channel closed"));
						break;
					}
				}
			}
		};

		Ok(Box::pin(initial.chain(notifications)))
	}

	async fn receive_mandatory_proof(
		&self,
		state_machine: &StateMachine,
	) -> Result<Option<QueueMessage>, anyhow::Error> {
		let queues = self.mandatory_queues.read().await;
		let queue = queues.get(state_machine);

		Ok(queue.and_then(|q| {
			q.front().map(|proof| QueueMessage {
				id: format!("mandatory-{}-{}", state_machine, proof.finalized_height),
				proof: proof.clone(),
			})
		}))
	}

	async fn receive_messages_proof(
		&self,
		state_machine: &StateMachine,
	) -> Result<Option<QueueMessage>, anyhow::Error> {
		let queues = self.messages_queues.read().await;
		let queue = queues.get(state_machine);

		Ok(queue.and_then(|q| {
			q.front().map(|proof| QueueMessage {
				id: format!("messages-{}-{}", state_machine, proof.finalized_height),
				proof: proof.clone(),
			})
		}))
	}

	async fn delete_message(
		&self,
		state_machine: &StateMachine,
		message_id: &str,
		message_type: StreamMessage,
	) -> Result<(), anyhow::Error> {
		let queues = match message_type {
			StreamMessage::EpochChanged => &self.mandatory_queues,
			StreamMessage::NewMessages => &self.messages_queues,
		};

		let mut queues_write = queues.write().await;
		let queue = queues_write.entry(*state_machine).or_insert_with(VecDeque::new);

		// Parse the message_id to extract the finalized_height
		// Format is "mandatory-{state_machine}-{height}" or "messages-{state_machine}-{height}"
		if let Some(height_str) = message_id.rsplit('-').next() {
			if let Ok(target_height) = height_str.parse::<u32>() {
				// Find and remove the message with matching finalized_height
				if let Some(pos) = queue.iter().position(|p| p.finalized_height == target_height) {
					queue.remove(pos);
				}
			}
		}

		Ok(())
	}
}
