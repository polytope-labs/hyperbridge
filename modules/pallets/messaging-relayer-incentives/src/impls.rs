use alloc::collections::BTreeMap;

use polkadot_sdk::{frame_support::traits::fungible::Mutate, sp_runtime::traits::*};
use polkadot_sdk::frame_support::traits::tokens::Preservation;
use polkadot_sdk::sp_core::U256;
use sp_core::{H256, keccak_256};

use ismp::{
	events::Event as IsmpEvent
	,
	messaging::Message,
	router::RequestResponse,
};
use pallet_ismp_host_executive::HostParam::EvmHostParam;

use crate::*;
use crate::types::IncentivizedMessage;

impl<T: Config> Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	u128: From<<T as pallet_ismp::Config>::Balance>
{
	fn process_message(message: &Message, relayer: T::AccountId) -> Result<(), Error<T>> {
		let mut messages_by_chain: BTreeMap<StateMachine, Vec<IncentivizedMessage>> =
			BTreeMap::new();

		match message {
			Message::Request(msg) =>
				for req in &msg.requests {
					if let Some(_) = SupportedStateMachines::<T>::get(req.dest) {
						messages_by_chain
							.entry(req.dest)
							.or_default()
							.push(IncentivizedMessage::Post(req.clone()));
					}
				},
			Message::Response(msg) => match msg.datagram.clone() {
				RequestResponse::Request(requests) =>
					for req in requests {
						let dest_chain = req.dest_chain();
						if let Some(_) = SupportedStateMachines::<T>::get(dest_chain) {
							messages_by_chain
								.entry(dest_chain)
								.or_default()
								.push(IncentivizedMessage::Request(req));
						}
					},
				RequestResponse::Response(responses) =>
					for res in responses {
						let dest_chain = res.dest_chain();
						if SupportedStateMachines::<T>::get(dest_chain).is_some() {
							messages_by_chain
								.entry(dest_chain)
								.or_default()
								.push(IncentivizedMessage::Response(res));
						}
					},
			},
			_ => return Ok(()),
		};

		for (state_machine, messages) in &messages_by_chain {
			let bytes_processed = messages.len() as u32;
			let current_total_bytes = TotalBytesProcessed::<T>::get();

			TotalBytesProcessed::<T>::mutate(|total| {
				*total = total.saturating_add(bytes_processed)
			});

			if let Some(host_params) = pallet_ismp_host_executive::HostParams::<T>::get(&state_machine) {
				match host_params {
					EvmHostParam(evm_host_param) => {
						let per_byte_fee = evm_host_param
							.per_byte_fees
							.iter()
							.find(|fee| {
								let hashed_chain_id = keccak_256(&state_machine.encode());
								H256(hashed_chain_id) == fee.state_id
							})
							.map(|fee| fee.per_byte_fee)
							.ok_or(Error::<T>::PerByteFeeNotFound)?;

						let dollar_cost = per_byte_fee.saturating_mul(U256::from(bytes_processed));
						let dollar_cost:u128 = dollar_cost.try_into().map_err(|_| Error::<T>::CalculationOverflow)?;

						let base_reward_in_token = T::PriceOracle::convert_to_usd(state_machine.clone(), dollar_cost.saturated_into())
							.map_err(|_| Error::<T>::ErrorInPriceConversion)?;

						let target_message_size = T::TargetMessageSize::get();

						if current_total_bytes <= target_message_size {
							let reward_amount = Self::calculate_reward(
								current_total_bytes,
								target_message_size,
								base_reward_in_token.into(),
							)?;

							T::Currency::transfer(
								&T::TreasuryAccount::get().into_account_truncating(),
								&relayer,
								reward_amount.saturated_into(),
								Preservation::Expendable,
							)
								.map_err(|_| Error::<T>::RewardTransferFailed)?;
							Self::deposit_event(Event::RelayerRewarded { relayer: relayer.clone(), amount: reward_amount.saturated_into() });
						} else {
							T::Currency::transfer(
								&relayer,
								&T::TreasuryAccount::get().into_account_truncating(),
								base_reward_in_token,
								Preservation::Expendable,
							)
								.map_err(|_| Error::<T>::RewardTransferFailed)?;

							Self::deposit_event(Event::RelayerCharged {
								relayer: relayer.clone(),
								amount: base_reward_in_token.into(),
							});
						}
					},
					_ => {}
				}
			}
		}

		Ok(())
	}

	/// A curve for calculating reward
	/// Reward=BaseReward×((TargetSize−TotalBytes)/TargetSize)^2
	fn calculate_reward(
		total_bytes: u32,
		target_size: u32,
		base_reward: u128,
	) -> Result<u128, Error<T>> {
		if total_bytes >= target_size || target_size.is_zero() {
			return Ok(u128::zero());
		}

		let decay_numerator = target_size.saturating_sub(total_bytes);
		let decay_numerator_sq = decay_numerator
			.checked_mul(decay_numerator)
			.ok_or(Error::<T>::CalculationOverflow)?;
		let final_reward_numerator = base_reward
			.checked_mul(decay_numerator_sq as u128)
			.ok_or(Error::<T>::CalculationOverflow)?;
		let target_size_sq =
			target_size.checked_mul(target_size).ok_or(Error::<T>::CalculationOverflow)? as u128;

		Ok(final_reward_numerator / target_size_sq)
	}
}

impl<T: Config> Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	u128: From<<T as pallet_ismp::Config>::Balance>
{
	fn on_executed(messages: Vec<Message>, _events: Vec<IsmpEvent>) -> DispatchResultWithPostInfo {
		for message in &messages {
			let relayer_address = match message {
				Message::Request(msg) => Some(msg.signer.clone()),
				Message::Response(msg) => Some(msg.signer.clone()),
				_ => None,
			};

			if let Some(address_bytes) = relayer_address {
				if address_bytes.len() == 32 {
					let mut raw_address = [0u8; 32];
					raw_address.copy_from_slice(&address_bytes[..]);
					let relayer_account = T::AccountId::from(raw_address);
					let _ = Self::process_message(message, relayer_account);
				}
			}
		}

		Ok(PostDispatchInfo { actual_weight: None, pays_fee: Pays::No })
	}
}
