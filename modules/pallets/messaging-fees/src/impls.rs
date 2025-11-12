use alloc::{collections::BTreeMap, string::ToString, vec};

use codec::{Decode, Encode};
use hyperbridge_client_machine::OnRequestProcessed;
use polkadot_sdk::{
	frame_support::traits::{
		fungible::{Inspect, Mutate},
		tokens::Preservation,
	},
	sp_core::U256,
	sp_runtime::traits::*,
};
use sp_core::H256;

use crate::{types::IncentivizedMessage, *};
use crypto_utils::verification::Signature;
use ismp::{
	events::Event as IsmpEvent,
	messaging::{hash_request, hash_response, Message, MessageWithWeight},
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
	fn accumulate_protocol_fees(message: &Message, relayer_account: &T::AccountId) {
		let mut fee_data: Vec<(usize, H256, StateMachine, StateMachine)> = vec![];
		match message {
			Message::Request(req) =>
				for post in &req.requests {
					let commitment = hash_request::<<T as pallet::Config>::IsmpHost>(
						&Request::Post(post.clone()),
					);
					fee_data.push((
						post.body.len(),
						commitment,
						post.source.clone(),
						post.dest.clone(),
					));
				},
			Message::Response(res) => match &res.datagram {
				RequestResponse::Response(responses) =>
					for r in responses {
						if let Response::Post(post_response) = r {
							let response = Response::Post(post_response.clone());
							let commitment =
								hash_response::<<T as pallet::Config>::IsmpHost>(&response);
							fee_data.push((
								post_response.response.len(),
								commitment,
								response.source_chain(),
								response.dest_chain(),
							));
						}
					},
				RequestResponse::Request(_) => return,
			},
			_ => return,
		};

		let relayer_address = relayer_account.as_ref().to_vec();

		for (size, commitment, source_chain, dest_chain) in fee_data {
			if source_chain.is_evm() {
				if let Some(per_byte_fee) = Self::get_per_byte_fee(&source_chain, &dest_chain) {
					let fee = per_byte_fee.saturating_mul(U256::from(size));
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
					if fee_u256 > U256::zero() {
						pallet_ismp_relayer::Pallet::<T>::accumulate_fee_and_deposit_event(
							source_chain.clone(),
							relayer_address.clone(),
							fee_u256,
						);
					}
				}
			}
		}
	}

	fn get_per_byte_fee(
		source_chain: &StateMachine,
		destination_chain: &StateMachine,
	) -> Option<U256> {
		let host_params = pallet_ismp_host_executive::HostParams::<T>::get(source_chain)?;

		let per_byte_fee = match host_params {
			EvmHostParam(evm_host_param) => {
				let fee = evm_host_param
					.per_byte_fees
					.iter()
					.find(|fee| {
						let hashed_chain_id =
							sp_io::hashing::keccak_256(&destination_chain.to_string().as_bytes());
						fee.state_id == H256(hashed_chain_id)
					})
					.map(|fee| fee.per_byte_fee)
					.unwrap_or(evm_host_param.default_per_byte_fee);

				let Some(decimals) =
					pallet_ismp_host_executive::FeeTokenDecimals::<T>::get(source_chain)
				else {
					return None;
				};
				let scaling_power = 18u8.saturating_sub(decimals); // assumption that the decimals will always be less than 18
				let scaling_factor = U256::from(10u128.pow(scaling_power as u32));
				fee.saturating_mul(scaling_factor)
			},
			SubstrateHostParam(VersionedHostParams::V1(substrate_params)) => {
				let fee = substrate_params
					.per_byte_fees
					.get(destination_chain)
					.cloned()
					.unwrap_or(substrate_params.default_per_byte_fee);
				let fee_u128: u128 = fee.into();
				let fee_u256 = U256::from(fee_u128);

				let Some(decimals) =
					pallet_ismp_host_executive::FeeTokenDecimals::<T>::get(source_chain)
				else {
					return None;
				};

				let scaling_power = 18u8.saturating_sub(decimals); // assumption that the decimals will always be less than 18
				let scaling_factor = U256::from(10u128.pow(scaling_power as u32));
				fee_u256.saturating_mul(scaling_factor)
			},
		};

		Some(per_byte_fee)
	}

	fn process_bridge_rewards(message: &Message, relayer: T::AccountId) -> Result<(), Error<T>> {
		let mut messages_by_chain: BTreeMap<
			(StateMachine, StateMachine),
			Vec<IncentivizedMessage>,
		> = BTreeMap::new();

		match message {
			Message::Request(msg) =>
				for req in &msg.requests {
					let is_incentivized = IncentivizedRoutes::<T>::get(&req.source).is_some() &&
						IncentivizedRoutes::<T>::get(&req.dest).is_some();
					let request = Request::Post(req.clone());
					messages_by_chain
						.entry((req.source.clone(), req.dest.clone()))
						.or_default()
						.push(IncentivizedMessage::Request(request, is_incentivized));
				},
			Message::Response(msg) => match msg.datagram.clone() {
				RequestResponse::Request(_) => return Ok(()),
				RequestResponse::Response(responses) =>
					for res in responses {
						let source_chain = res.source_chain();
						let dest_chain = res.dest_chain();
						let is_incentivized = IncentivizedRoutes::<T>::get(source_chain).is_some() &&
							IncentivizedRoutes::<T>::get(dest_chain).is_some();
						messages_by_chain
							.entry((source_chain, dest_chain))
							.or_default()
							.push(IncentivizedMessage::Response(res, is_incentivized));
					},
			},
			_ => return Ok(()),
		};

		for ((source_chain, destination_chain), messages) in &messages_by_chain {
			for msg in messages {
				let (mut bytes_processed, is_incentivized) = match msg {
					IncentivizedMessage::Request(req, is_incentivized) => {
						let size = match req {
							Request::Post(post) => post.body.len() as u32,
							Request::Get(_) => 0,
						};
						(size, *is_incentivized)
					},
					IncentivizedMessage::Response(res, is_incentivized) => {
						let size = match res {
							Response::Post(post) => post.response.len() as u32,
							Response::Get(get) => get
								.values
								.iter()
								.filter_map(|storage_val| storage_val.value.as_ref())
								.map(|bytes| bytes.len())
								.sum::<usize>() as u32,
						};
						(size, *is_incentivized)
					},
				};

				bytes_processed = core::cmp::max(bytes_processed, 32);
				TotalBytesProcessed::<T>::mutate(|total| {
					*total = total.saturating_add(bytes_processed)
				});

				if let Some(per_byte_fee) = Self::get_per_byte_fee(source_chain, destination_chain)
				{
					let cost = per_byte_fee.saturating_mul(U256::from(bytes_processed));
					if cost.is_zero() {
						continue;
					}
					let bridge_price = T::PriceOracle::get_bridge_price()
						.map_err(|_| Error::<T>::ErrorInPriceConversion)?;

					// Get reward in bridge tokens and scale down to 12 decimals
					let base_reward: u128 = cost
						.checked_div(bridge_price)
						.ok_or(Error::<T>::CalculationOverflow)?
						.checked_mul(DECIMALS_12.into())
						.ok_or(Error::<T>::CalculationOverflow)?
						.try_into()
						.map_err(|_| Error::<T>::CalculationOverflow)?;

					if is_incentivized {
						let current_total_bytes = TotalBytesProcessed::<T>::get();
						let target_message_size = Self::get_target_message_size();

						if current_total_bytes < target_message_size {
							let reward_amount = Self::calculate_reward(
								current_total_bytes,
								target_message_size,
								base_reward,
							)?;
							if reward_amount >= T::Currency::minimum_balance().into() {
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
								log::info!(target: "ismp", "Reward amount {reward_amount:?} below minimum balance");
							}
							T::ReputationAsset::mint_into(&relayer, reward_amount.saturated_into())
								.map_err(|_| Error::<T>::ReputationMintFailed)?;
						} else {
							Self::pay_fee(&relayer, base_reward.saturated_into()).map_err(|e| {
								log::error!(target: "ismp", "Failed to pay fee {e:?}");
								e
							})?;
							T::ReputationAsset::mint_into(&relayer, base_reward.saturated_into())
								.map_err(|_| Error::<T>::ReputationMintFailed)?;
						}
					} else {
						Self::pay_fee(&relayer, base_reward.saturated_into()).map_err(|e| {
							log::error!(target: "ismp", "Failed to pay fee {e:?}");
							e
						})?;
						T::ReputationAsset::mint_into(&relayer, base_reward.saturated_into())
							.map_err(|_| Error::<T>::ReputationMintFailed)?;
					}
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

	fn pay_fee(relayer: &T::AccountId, fee: T::Balance) -> Result<(), Error<T>> {
		if fee >= T::Currency::minimum_balance().into() {
			T::Currency::transfer(
				relayer,
				&T::TreasuryAccount::get().into_account_truncating(),
				fee,
				Preservation::Expendable,
			)
			.map_err(|_| Error::<T>::RewardTransferFailed)?;

			Self::deposit_event(Event::FeePaid { relayer: relayer.clone(), amount: fee });
		} else {
			log::info!(target: "ismp", "Fee amount {fee:?} below minimum balance");
		}

		Ok(())
	}

	fn get_target_message_size() -> u32 {
		Self::target_message_size().unwrap_or(200000)
	}
}

impl<T: Config> FeeHandler for Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	u128: From<<T as pallet_ismp::Config>::Balance>,
	T::AccountId: AsRef<[u8]>,
{
	fn on_executed(
		messages: Vec<MessageWithWeight>,
		_events: Vec<IsmpEvent>,
	) -> DispatchResultWithPostInfo {
		for message in &messages {
			let message = message.message.clone();
			let relayer_account = match &message {
				Message::Request(msg) => {
					let data = sp_io::hashing::keccak_256(&msg.requests.encode());
					Signature::decode(&mut &msg.signer[..])
						.ok()
						.and_then(|sig| sig.verify_and_get_sr25519_pubkey(&data, None).ok())
				},
				Message::Response(msg) => {
					let data = sp_io::hashing::keccak_256(&msg.datagram.encode());
					Signature::decode(&mut &msg.signer[..])
						.ok()
						.and_then(|sig| sig.verify_and_get_sr25519_pubkey(&data, None).ok())
				},
				_ => None,
			};

			if let Some(relayer_account) = relayer_account {
				let _ = Self::accumulate_protocol_fees(&message, &relayer_account.into());
				let _ = Self::process_bridge_rewards(&message, relayer_account.into())?;
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
