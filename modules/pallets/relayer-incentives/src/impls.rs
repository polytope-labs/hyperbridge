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
use frame_support::traits::tokens::Preservation;
use ismp::host::IsmpHost;
use ismp::{consensus::StateMachineId, events::Event as IsmpEvent, messaging::Message};
use pallet_ismp::fee_handler::FeeHandler;
use polkadot_sdk::frame_support::traits::fungible::Mutate;
use polkadot_sdk::sp_runtime::traits::*;
use sp_core::H256;
use sp_io::hashing::keccak_256;

impl<T: Config> Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
{
	/// Process a message and reward the relayer
	///
	/// This is an internal function used to handle relayer rewards for each
	/// processed message, this targets just ConsensusMessage for now.
	///  It extracts relayer information, calculates the
	/// appropriate reward, and updates the relayer's reward balance.
	fn process_message(
		message_id: H256,
		state_machine_id: StateMachineId,
		relayer_address: Vec<u8>,
	) -> Result<<T as pallet_ismp::Config>::Balance, Error<T>> {
		// Check if message has already been processed
		if ProcessedMessages::<T>::get(&message_id) {
			return Err(Error::<T>::MessageAlreadyProcessed);
		}

		if relayer_address.len() != 32 {
			return Err(Error::<T>::InvalidAddress);
		}

		let mut raw_address = [0u8; 32];
		raw_address.copy_from_slice(&relayer_address[..]);

		let relayer_account =
			T::AccountId::try_from(raw_address).map_err(|_| Error::<T>::InvalidAddress)?;

		let reward = Self::calculate_reward(&state_machine_id)?;

		RelayerRewards::<T>::mutate(relayer_account.clone(), |balance| {
			*balance = balance.saturating_add(reward);
		});

		T::Currency::transfer(
			&T::TreasuryAccount::get().into_account_truncating(),
			&relayer_account,
			reward,
			Preservation::Expendable,
		)
		.map_err(|_| Error::<T>::RewardTransferFailed)?;

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

	/// Calculate the reward for a message based on the state machine id
	fn calculate_reward(
		state_machine_id: &StateMachineId,
	) -> Result<<T as pallet_ismp::Config>::Balance, Error<T>> {
		let host = <T::IsmpHost>::default();
		let latest_height = host
			.latest_commitment_height(state_machine_id.clone())
			.map_err(|_| Error::<T>::CouldNotGetStateMachineHeight)?;
		let previous_height =
			host.previous_commitment_height(state_machine_id.clone()).unwrap_or_default();

		let blocks = latest_height.saturating_sub(previous_height);
		let block_cost = StateMachinesCostPerBlock::<T>::get(state_machine_id);

		let blocks_as_balance: <T as pallet_ismp::Config>::Balance = blocks.saturated_into();
		let reward = blocks_as_balance.saturating_mul(block_cost);

		Ok(reward)
	}
}

/// Implementation of the FeeHandler trait for the RelayerIncentives pallet
impl<T: Config> FeeHandler for Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
{
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

		for message in messages {
			if let Message::Consensus(consensus_msg) = message {
				let matching_state_machine = state_machine_map
					.iter()
					.find(|(_, cid)| **cid == consensus_msg.consensus_state_id)
					.map(|(sm_id, _)| sm_id.clone());

				if let Some(state_machine_id) = matching_state_machine {
					let message_hash = keccak_256(&consensus_msg.consensus_proof);
					Self::process_message(
						H256::from(message_hash),
						state_machine_id,
						consensus_msg.signer.clone(),
					)?;
				}
			}
		}

		// Return with actual weight information
		// We use Pays::No to indicate that someone (the message sender) doesn't pay for this operation,
		// though we're using this mechanism to reward relayers rather than charge fees
		Ok(PostDispatchInfo { actual_weight: None, pays_fee: Pays::No })
	}
}
