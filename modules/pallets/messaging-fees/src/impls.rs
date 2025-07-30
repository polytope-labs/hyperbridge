use alloc::collections::BTreeMap;

use codec::{Decode, Encode};
use hyperbridge_client_machine::OnRequestProcessed;
use polkadot_sdk::{
	frame_support::traits::{fungible::Mutate, tokens::Preservation},
	sp_core::U256,
	sp_runtime::traits::*,
};
use sp_core::{sr25519, H256};

use crate::{types::IncentivizedMessage, *};
use ismp::{
	events::Event as IsmpEvent,
	messaging::{hash_request, Message},
	router::{Request, RequestResponse, Response},
};
use pallet_hyperbridge::VersionedHostParams;
use pallet_ismp::fee_handler::FeeHandler;
use pallet_ismp_host_executive::HostParam::{EvmHostParam, SubstrateHostParam};

impl<T: Config> Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	u128: From<<T as pallet_ismp::Config>::Balance>,
	T::AccountId: AsRef<[u8]>,
{
	fn verify_and_get_relayer(signer: &Vec<u8>, signed_data: &[u8; 32]) -> Option<T::AccountId>
	where
		T::AccountId: From<[u8; 32]>,
	{
		type Sr25519Signature = (sr25519::Public, sr25519::Signature);

		if let Ok((pub_key, sig)) = Sr25519Signature::decode(&mut &signer[..]) {
			if sp_io::crypto::sr25519_verify(&sig, signed_data, &pub_key) {
				return Some(pub_key.0.into());
			}
		}
		None
	}

	fn accumulate_protocol_fees(message: &Message, relayer_account: &T::AccountId) {
		let requests = match message {
			Message::Request(req) => req.requests.clone(),
			_ => return,
		};

		let relayer_address = relayer_account.as_ref().to_vec();

		for req in requests {
			let commitment =
				hash_request::<<T as pallet::Config>::IsmpHost>(&Request::Post(req.clone()));
			let source_chain = &req.source;

			if source_chain.is_evm() {
				if let Some(per_byte_fee) = Self::get_per_byte_fee(&req.dest) {
					let fee = per_byte_fee.saturating_mul(U256::from(req.body.len()));
					if fee > U256::zero() {
						pallet_ismp_relayer::Pallet::<T>::accumulate_fee_and_deposit_event(
							source_chain.clone(),
							relayer_address.clone(),
							fee,
						);
					}
				}
			} else if source_chain.is_substrate() {
				if let Some(fee) = CommitmentFees::<T>::take(&commitment) {
					let fee_u256: U256 = u128::from(fee).into();

					pallet_ismp_relayer::Pallet::<T>::accumulate_fee_and_deposit_event(
						source_chain.clone(),
						relayer_address.clone(),
						fee_u256,
					);
				}
			}
		}
	}

	fn get_per_byte_fee(state_machine: &StateMachine) -> Option<U256> {
		let host_params = pallet_ismp_host_executive::HostParams::<T>::get(state_machine)?;

		let per_byte_fee = match host_params {
			EvmHostParam(evm_host_param) => {
				let fee = evm_host_param
					.per_byte_fees
					.iter()
					.find(|fee| {
						let hashed_chain_id = sp_io::hashing::keccak_256(&state_machine.encode());
						fee.state_id == H256(hashed_chain_id)
					})
					.map(|fee| fee.per_byte_fee)
					.unwrap_or(evm_host_param.default_per_byte_fee);
				fee
			},
			SubstrateHostParam(VersionedHostParams::V1(substrate_params)) => {
				let fee = substrate_params
					.per_byte_fees
					.get(state_machine)
					.cloned()
					.unwrap_or(substrate_params.default_per_byte_fee);
				let fee_u128: u128 = fee.into();
				let fee_u256 = U256::from(fee_u128);

				let decimals =
					pallet_ismp_host_executive::FeeTokenDecimals::<T>::get(state_machine)
						.unwrap_or(10);
				let scaling_power = 18 - decimals; // assumption that the decimals will always be less than 18
				let scaling_factor = U256::from(10u128.pow(scaling_power as u32));
				fee_u256.saturating_mul(scaling_factor)
			},
		};

		Some(per_byte_fee)
	}

	fn process_bridge_rewards(message: &Message, relayer: T::AccountId) -> Result<(), Error<T>> {
		let mut messages_by_chain: BTreeMap<StateMachine, Vec<IncentivizedMessage>> =
			BTreeMap::new();

		match message {
			Message::Request(msg) =>
				for req in &msg.requests {
					if IncentivizedRoutes::<T>::get(req.source).is_some() &&
						IncentivizedRoutes::<T>::get(req.dest).is_some()
					{
						let request = Request::Post(req.clone());
						messages_by_chain
							.entry(req.dest)
							.or_default()
							.push(IncentivizedMessage::Request(request));
					}
				},
			Message::Response(msg) => match msg.datagram.clone() {
				RequestResponse::Request(requests) =>
					for req in requests {
						let source_chain = req.source_chain();
						let dest_chain = req.dest_chain();
						if IncentivizedRoutes::<T>::get(source_chain).is_some() &&
							IncentivizedRoutes::<T>::get(dest_chain).is_some()
						{
							messages_by_chain
								.entry(dest_chain)
								.or_default()
								.push(IncentivizedMessage::Request(req));
						}
					},
				RequestResponse::Response(responses) =>
					for res in responses {
						let source_chain = res.source_chain();
						let dest_chain = res.dest_chain();
						if IncentivizedRoutes::<T>::get(source_chain).is_some() &&
							IncentivizedRoutes::<T>::get(dest_chain).is_some()
						{
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
			let bytes_processed: u32 = messages
				.iter()
				.map(|msg| match msg {
					IncentivizedMessage::Request(req) => match req {
						Request::Post(post) => post.body.len() as u32,
						Request::Get(_) => 0,
					},
					IncentivizedMessage::Response(res) => match res {
						Response::Post(post) => post.response.len() as u32,
						Response::Get(get) => get
							.values
							.iter()
							.filter_map(|storage_val| storage_val.value.as_ref())
							.map(|bytes| bytes.len())
							.sum::<usize>() as u32,
					},
				})
				.sum();

			TotalBytesProcessed::<T>::mutate(|total| {
				*total = total.saturating_add(bytes_processed)
			});
			let current_total_bytes = TotalBytesProcessed::<T>::get();

			if let Some(per_byte_fee) = Self::get_per_byte_fee(&state_machine) {
				let cost = per_byte_fee.saturating_mul(U256::from(bytes_processed));
				let bridge_price = T::PriceOracle::get_bridge_price()
					.map_err(|_| Error::<T>::ErrorInPriceConversion)?;

				let cost_bridge_price_18_decimals =
					cost.checked_mul(bridge_price).ok_or(Error::<T>::CalculationOverflow)?;

				let cost_bridge_price_12_decimals_u256 = cost_bridge_price_18_decimals
					.checked_div(SCALING_FACTOR_18_TO_12.into())
					.ok_or(Error::<T>::CalculationOverflow)?;

				let base_reward_12_decimals: u128 = cost_bridge_price_12_decimals_u256
					.try_into()
					.map_err(|_| Error::<T>::CalculationOverflow)?;

				let base_reward_as_balance: T::Balance = base_reward_12_decimals.saturated_into();
				let target_message_size = T::TargetMessageSize::get();

				if current_total_bytes <= target_message_size {
					let reward_amount = Self::calculate_reward(
						current_total_bytes,
						target_message_size,
						base_reward_12_decimals.into(),
					)?;

					T::Currency::transfer(
						&T::TreasuryAccount::get().into_account_truncating(),
						&relayer,
						reward_amount.saturated_into(),
						Preservation::Expendable,
					)
					.map_err(|_| Error::<T>::RewardTransferFailed)?;
					Self::deposit_event(Event::FeeRewarded {
						relayer: relayer.clone(),
						amount: reward_amount.saturated_into(),
					});
				} else {
					T::Currency::transfer(
						&relayer,
						&T::TreasuryAccount::get().into_account_truncating(),
						base_reward_as_balance,
						Preservation::Expendable,
					)
					.map_err(|_| Error::<T>::RewardTransferFailed)?;

					Self::deposit_event(Event::FeePaid {
						relayer: relayer.clone(),
						amount: base_reward_as_balance,
					});
				}
			}
		}

		Ok(())
	}

	/// A curve for calculating reward
	/// Reward=BaseReward×((TargetSize−TotalBytes)/TargetSize)^2
	pub fn calculate_reward(
		total_bytes: u32,
		target_size: u32,
		base_reward: u128,
	) -> Result<u128, Error<T>> {
		if total_bytes >= target_size || target_size.is_zero() {
			return Ok(u128::zero());
		}

		let total_bytes_u128 = total_bytes as u128;
		let target_size_u128 = target_size as u128;

		let decay_numerator = target_size_u128.saturating_sub(total_bytes_u128);

		let decay_numerator_sq = decay_numerator
			.checked_mul(decay_numerator)
			.ok_or(Error::<T>::CalculationOverflow)?;

		let target_size_sq = target_size_u128
			.checked_mul(target_size_u128)
			.ok_or(Error::<T>::CalculationOverflow)?;

		let final_reward_numerator = base_reward
			.checked_mul(decay_numerator_sq)
			.ok_or(Error::<T>::CalculationOverflow)?;

		Ok(final_reward_numerator.saturating_div(target_size_sq))
	}
	pub fn note_request_fee(commitment: H256, fee: u128) {
		let fee_balance: T::Balance = fee.saturated_into();
		CommitmentFees::<T>::insert(commitment, fee_balance);
	}
}

impl<T: Config> FeeHandler for Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	u128: From<<T as pallet_ismp::Config>::Balance>,
	T::AccountId: AsRef<[u8]>,
{
	fn on_executed(messages: Vec<Message>, _events: Vec<IsmpEvent>) -> DispatchResultWithPostInfo {
		for message in &messages {
			let relayer_account = match message {
				Message::Request(msg) => {
					let data = sp_io::hashing::keccak_256(&msg.requests.encode());
					Self::verify_and_get_relayer(&msg.signer, &data)
				},
				Message::Response(msg) => {
					let data = sp_io::hashing::keccak_256(&msg.datagram.encode());
					Self::verify_and_get_relayer(&msg.signer, &data)
				},
				_ => None,
			};

			if let Some(relayer_account) = relayer_account {
				let _ = Self::accumulate_protocol_fees(message, &relayer_account);
				let _ = Self::process_bridge_rewards(message, relayer_account)?;
			}
		}

		Ok(PostDispatchInfo { actual_weight: None, pays_fee: Pays::No })
	}
}

impl<T: Config> OnRequestProcessed for Pallet<T> {
	fn note_request_fee(commitment: H256, fee: u128) {
		let fee_balance: T::Balance = fee.saturated_into();
		CommitmentFees::<T>::insert(commitment, fee_balance);
	}
}
