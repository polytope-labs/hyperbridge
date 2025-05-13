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
use ismp::messaging::{Message, Request, Response, Timeout};
use pallet_ismp::fee_handler::FeeHandler;

impl<T: Config> Pallet<T> {
	/// Process a message and reward the relayer
	///
	/// This is an internal function used to handle relayer rewards for each
	/// processed message. It extracts relayer information, calculates the
	/// appropriate reward, and updates the relayer's reward balance.
	fn process_message(message: &Message) -> Result<BalanceOf<T>, Error<T>> {
		// Extract message identifier and relayer address
		let (message_id, relayer_addr) = match message {
			Message::Request(Request { request_identifier, relayer, .. }) => {
				(request_identifier.to_vec(), relayer.clone())
			},
			Message::Response(Response { response_commitment, relayer, .. }) => {
				(response_commitment.0.to_vec(), relayer.clone())
			},
			Message::Timeout(Timeout { request_identifier, relayer, .. }) => {
				(request_identifier.to_vec(), relayer.clone())
			},
			// Other message types might not have relayer info
			_ => return Ok(BalanceOf::<T>::zero()),
		};

		// Check if message has already been processed
		if ProcessedMessages::<T>::get(&message_id) {
			return Err(Error::<T>::MessageAlreadyProcessed);
		}

		// Look up the relayer account
		let relayer_account = T::RelayerLookup::lookup_account(&relayer_addr)
			.ok_or(Error::<T>::RelayerLookupFailed)?;

		// Calculate reward based on message type and parameters
		let parameters = IncentiveParams::<T>::get();
		let reward = Self::calculate_reward(message, &parameters)?;

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
			message_id,
		});

		Ok(reward)
	}

	/// Calculate the reward for a message based on the incentive parameters
	fn calculate_reward(
		message: &Message,
		params: &IncentiveParameters,
	) -> Result<BalanceOf<T>, Error<T>> {
		// Base reward for all messages
		let base: BalanceOf<T> = params.base_reward.saturated_into();

		// Apply priority multiplier for certain message types
		let multiplier = match message {
			Message::Request(Request { priority, .. }) => {
				if *priority == 0 {
					1u32
				} else {
					params.priority_multiplier
				}
			},
			// Other message types use the base reward
			_ => 1u32,
		};

		// Calculate final reward
		Ok(base.saturating_mul(multiplier.saturated_into()))
	}
}

/// Implementation of the FeeHandler trait for the RelayerIncentives pallet
impl<T: Config> FeeHandler for Pallet<T> {
	fn on_executed(messages: Vec<Message>) -> DispatchResultWithPostInfo {
		// Process each message and record the results
		let mut total_rewards = BalanceOf::<T>::zero();
		let mut processed_count = 0u32;

		for message in &messages {
			match Self::process_message(message) {
				Ok(reward) => {
					total_rewards = total_rewards.saturating_add(reward);
					processed_count += 1;
				},
				Err(e) => {
					// Log error but continue processing other messages
					log::warn!(
						target: "relayer-incentives",
						"Failed to process message for rewards: {:?}",
						e
					);
				},
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
