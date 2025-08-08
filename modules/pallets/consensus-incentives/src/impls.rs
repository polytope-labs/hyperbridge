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

//! Implementation blocks for pallet-consensus-incentives.

use crate::*;
use alloc::collections::BTreeMap;
use frame_support::traits::tokens::Preservation;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	events::Event as IsmpEvent,
	host::IsmpHost,
	messaging::Message,
};
use pallet_ismp::fee_handler::FeeHandler;
use pallet_ismp_relayer::withdrawal::Signature;
use polkadot_sdk::{
	frame_support::traits::fungible::Mutate, sp_core::sr25519, sp_runtime::traits::*,
};

impl<T: Config> Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
{
	fn verify_and_get_relayer(signer: &Vec<u8>, signed_data: &[u8; 32]) -> Option<Vec<u8>> {
		if let Ok(signature_enum) = Signature::decode(&mut &signer[..]) {
			match signature_enum {
				Signature::Sr25519 { public_key, signature } => {
					if let (Ok(pub_key), Ok(sig)) = (
						sr25519::Public::decode(&mut &public_key[..]),
						sr25519::Signature::decode(&mut &signature[..]),
					) {
						if sp_io::crypto::sr25519_verify(&sig, signed_data, &pub_key) {
							return Some(pub_key.0.into());
						}
					}
				},
				_ => return None,
			}
		}
		None
	}
	/// Process a message and reward the relayer
	///
	/// This is an internal function used to handle relayer rewards for each
	/// processed message, this targets just ConsensusMessage for now.
	///  It extracts relayer information, calculates the
	/// appropriate reward, and and transfer the reward to the relayer.
	fn process_message(
		state_machine_height: StateMachineHeight,
		state_machine_id: StateMachineId,
		relayer_address: Vec<u8>,
	) -> Result<(), Error<T>> {
		if relayer_address.len() != 32 {
			return Err(Error::<T>::InvalidAddress);
		}

		let mut raw_address = [0u8; 32];
		raw_address.copy_from_slice(&relayer_address[..]);

		let relayer_account =
			T::AccountId::try_from(raw_address).map_err(|_| Error::<T>::InvalidAddress)?;

		if let Some(block_cost) = StateMachinesCostPerBlock::<T>::get(state_machine_id) {
			let reward = Self::calculate_reward(&state_machine_id, block_cost)?;

			T::Currency::transfer(
				&T::TreasuryAccount::get().into_account_truncating(),
				&relayer_account,
				reward,
				Preservation::Expendable,
			)
			.map_err(|_| Error::<T>::RewardTransferFailed)?;

			Self::deposit_event(Event::<T>::RelayerRewarded {
				relayer: relayer_account.clone(),
				amount: reward,
				state_machine_height,
			});

			T::ReputationAsset::mint_into(&relayer_account, reward.saturated_into())
				.map_err(|_| Error::<T>::ReputationMintFailed)?;
		}
		Ok(())
	}

	/// Calculate the reward for a message based on the state machine id
	fn calculate_reward(
		state_machine_id: &StateMachineId,
		block_cost: <T as pallet_ismp::Config>::Balance,
	) -> Result<<T as pallet_ismp::Config>::Balance, Error<T>> {
		let host = <T::IsmpHost>::default();
		let latest_height = host
			.latest_commitment_height(state_machine_id.clone())
			.map_err(|_| Error::<T>::CouldNotGetStateMachineHeight)?;
		let previous_height =
			host.previous_commitment_height(state_machine_id.clone()).unwrap_or_default();

		let blocks = latest_height.saturating_sub(previous_height);

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
					(update.state_machine_id.consensus_state_id.clone(), update.latest_height),
				);
			}
		}

		for message in messages {
			if let Message::Consensus(consensus_msg) = message {
				let data = sp_io::hashing::keccak_256(&consensus_msg.consensus_proof.encode());
				if let Some(relayer_account) =
					Self::verify_and_get_relayer(&consensus_msg.signer, &data)
				{
					let maybe_match =
						state_machine_map.iter().find(|(_, (consensus_state_id, _))| {
							*consensus_state_id == consensus_msg.consensus_state_id
						});

					if let Some((state_machine_id, (_, height))) = maybe_match {
						let state_machine_height =
							StateMachineHeight { id: state_machine_id.clone(), height: *height };

						let _ = Self::process_message(
							state_machine_height,
							state_machine_id.clone(),
							relayer_account,
						);
					}
				}
			}
		}

		// Return with actual weight information
		// We use Pays::No to indicate that someone (the message sender) doesn't pay for this
		// operation, though we're using this mechanism to reward relayers rather than charge fees
		Ok(PostDispatchInfo { actual_weight: None, pays_fee: Pays::No })
	}
}
