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

//! Implementation blocks for pallet-relayer-incentives.

use crate::*;
use alloc::collections::BTreeMap;
use ismp::{
	consensus::StateMachineId,
	events::Event as IsmpEvent,
	messaging::{
		ConsensusMessage, Message, Message::Request, RequestMessage, ResponseMessage,
		TimeoutMessage,
	},
};
use pallet_ismp::{fee_handler::FeeHandler, LatestStateMachineHeight, PreviousStateMachineHeight};
use polkadot_sdk::sp_runtime::traits::*;

impl<T: Config> Pallet<T> {
	/// Process a message and reward the relayer
	///
	/// This is an internal function used to handle relayer rewards for each
	/// processed message, this targets just ConsensusMessage for now.
	///  It extracts relayer information, calculates the
	/// appropriate reward, and updates the relayer's reward balance.
	fn process_message(
		message_id: &Vec<u8>,
		state_machine_id: StateMachineId,
		relayer_address: Vec<u8>,
	) -> Result<BalanceOf<T>, Error<T>> {
		// Check if message has already been processed
		if ProcessedMessages::<T>::get(&message_id) {
			return Err(Error::<T>::MessageAlreadyProcessed);
		}

		// Look up the relayer account
		let relayer_account = T::RelayerLookup::lookup_account(&relayer_address)
			.ok_or(Error::<T>::RelayerLookupFailed)?;

		let reward = Self::calculate_reward(&state_machine_id)?;

		// Update relayer rewards
		RelayerRewards::<T>::mutate(relayer_account.clone(), |balance| {
			*balance = balance.saturating_add(reward);
		});

		// Mark message as processed
		ProcessedMessages::<T>::insert(message_id.clone(), true);

		// Emit reward event
		Self::deposit_event(Event::<T>::RelayerRewarded {
			relayer: relayer_account,
			amount: reward,
			message_id: message_id.clone(),
		});

		Ok(reward)
	}

	/// Calculate the reward for a message based on the state machine id
	fn calculate_reward(state_machine_id: &StateMachineId) -> Result<BalanceOf<T>, Error<T>> {
		let latest_height =
			LatestStateMachineHeight::<T>::get(state_machine_id).unwrap_or_default();
		let previous_height =
			PreviousStateMachineHeight::<T>::get(state_machine_id).unwrap_or_default();

		let blocks = latest_height.saturating_sub(previous_height);
		let block_cost = StateMachinesCostPerBlock::<T>::get(state_machine_id);

		let blocks_as_balance: BalanceOf<T> = blocks.saturated_into();
		let reward = blocks_as_balance.saturating_mul(block_cost);

		Ok(reward)
	}
}

/// Implementation of the FeeHandler trait for the RelayerIncentives pallet
impl<T: Config> FeeHandler for Pallet<T> {
	fn on_executed(messages: Vec<Message>, events: Vec<IsmpEvent>) -> DispatchResultWithPostInfo {
		let mut state_machine_map = BTreeMap::new();

		for event in events {
			if let IsmpEvent::StateMachineUpdated(update) = event {
				state_machine_map.insert(
					update.state_machine_id.clone(),
					update.state_machine_id.consensus_state_id.clone(),
				);
			}
		}

		let mut total_rewards = BalanceOf::<T>::zero();
		let mut processed_count = 0u32;

		for message in messages {
			if let Message::Consensus(consensus_msg) = message {
				let matching_state_machine = state_machine_map
					.iter()
					.find(|(_, cid)| **cid == consensus_msg.consensus_state_id)
					.map(|(sm_id, _)| sm_id.clone());

				if let Some(state_machine_id) = matching_state_machine {
					match Self::process_message(
						&consensus_msg.consensus_proof,
						state_machine_id,
						consensus_msg.signer.clone(),
					) {
						Ok(reward) => {
							total_rewards = total_rewards.saturating_add(reward);
							processed_count += 1;
						},
						Err(_e) => {},
					}
				}
			}
		}
		// Emit batch processed event
		if processed_count > 0 {
			Self::deposit_event(Event::<T>::BatchProcessed {
				count: processed_count,
				total_rewards,
			});
		}

		// Calculate weight based on number of messages processed
		let weight = <T as Config>::WeightInfo::on_message_execution()
			.saturating_mul(processed_count.into());

		// Return with actual weight information
		// We use Pays::Yes to indicate that someone (the message sender) pays for this operation,
		// though we're using this mechanism to reward relayers rather than charge fees
		Ok(PostDispatchInfo { actual_weight: None, pays_fee: Pays::Yes })
	}
}
