use alloc::collections::BTreeMap;

use codec::{Decode, Encode};
use polkadot_sdk::{
	frame_support::traits::{fungible::Mutate, tokens::Preservation},
	sp_core::U256,
	sp_runtime::traits::*,
};
use sp_core::{H256, keccak_256, Pair, sr25519};

use ismp::{
	dispatcher::{DispatchPost, DispatchRequest, FeeMetadata},
	messaging::{hash_request, Message},
	router::{Request, RequestResponse, Response}
};
use pallet_hyperbridge::{VersionedHostParams, WithdrawalRequest};
use pallet_ismp_host_executive::HostParam::{EvmHostParam, SubstrateHostParam};

use crate::{*, types::IncentivizedMessage};

impl<T: Config> Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	u128: From<<T as pallet_ismp::Config>::Balance>,
	T::AccountId: AsRef<[u8]>,
{
	pub fn on_executed(messages: Vec<Message>) -> DispatchResultWithPostInfo {
		for message in &messages {
			let relayer_account = match message {
				Message::Request(msg) => {
					let data = keccak_256(&msg.requests.encode());
					Self::verify_and_get_relayer(&msg.signer, &data)
				},
				Message::Response(msg) => {
					let data = keccak_256(&msg.datagram.encode());
					Self::verify_and_get_relayer(&msg.signer, &data)
				},
				_ => None,
			};

			if let Some(relayer_account) = relayer_account {
				let _ = Self::accumulate_protocol_fees(message, &relayer_account);
				let _ = Self::process_bridge_rewards(message, relayer_account);
			}
		}

		Ok(PostDispatchInfo { actual_weight: None, pays_fee: Pays::No })
	}

	fn verify_and_get_relayer(signer: &Vec<u8>, signed_data: &[u8; 32]) -> Option<T::AccountId>
	where
		T::AccountId: From<[u8; 32]>,
	{
		type Sr25519Signature = (sr25519::Public, sr25519::Signature);

		if let Ok((pub_key, sig)) = Sr25519Signature::decode(&mut &signer[..]) {
			if sr25519::Pair::verify(&sig, signed_data, &pub_key) {
				return Some(pub_key.0.into());
			}
		}
		None
	}

	fn accumulate_protocol_fees(
		message: &Message,
		relayer_account: &T::AccountId,
	) -> Result<(), Error<T>> {
		let requests = match message {
			Message::Request(req) => req.requests.clone(),
			_ => return Ok(()),
		};

		for req in requests {
			let commitment =  hash_request::<<T as pallet::Config>::IsmpHost>(&Request::Post(req.clone()));
			let  source_chain = &req.source;

			if let Some(fee) = CommitmentFees::<T>::take(&commitment) {
				AccumulatedFees::<T>::mutate(source_chain, relayer_account, |total_fee| {
					*total_fee = total_fee.saturating_add(fee);
				});

				Self::deposit_event(Event::FeeAccumulated {
					relayer: relayer_account.clone(),
					source_chain: source_chain.clone(),
					amount: fee,
				});
			}
		}

		Ok(())
	}

	fn process_bridge_rewards(message: &Message, relayer: T::AccountId) -> Result<(), Error<T>> {
		let mut messages_by_chain: BTreeMap<StateMachine, Vec<IncentivizedMessage>> =
			BTreeMap::new();

		match message {
			Message::Request(msg) =>
				for req in &msg.requests {
					if IncentivizedRoutes::<T>::get((req.source, req.dest)).is_some() {
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
						if IncentivizedRoutes::<T>::get((source_chain, dest_chain)).is_some() {
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
						if IncentivizedRoutes::<T>::get((source_chain, dest_chain)).is_some() {
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

			if let Some(host_params) =
				pallet_ismp_host_executive::HostParams::<T>::get(&state_machine)
			{
				let per_byte_fee = match host_params {
					EvmHostParam(evm_host_param) => {
						let fee = evm_host_param
							.per_byte_fees
							.iter()
							.find(|fee| {
								let hashed_chain_id = keccak_256(&state_machine.encode());
								fee.state_id == H256(hashed_chain_id)
							})
							.map(|fee| fee.per_byte_fee)
							.unwrap_or(evm_host_param.default_per_byte_fee);

						fee
					},
					SubstrateHostParam(VersionedHostParams::V1(substrate_params)) => {
						let fee = substrate_params
							.per_byte_fees
							.get(&state_machine)
							.cloned()
							.unwrap_or(substrate_params.default_per_byte_fee);
						let fee_u128: u128 = fee.into();
						U256::from(fee_u128)
					},
				};

				let dollar_cost = per_byte_fee.saturating_mul(U256::from(bytes_processed));
				let dollar_cost: u128 =
					dollar_cost.try_into().map_err(|_| Error::<T>::CalculationOverflow)?;

				let bridge_usd_price = T::PriceOracle::get_bridge_price()
					.map_err(|_| Error::<T>::ErrorInPriceConversion)?;

				let base_reward_in_token =
					bridge_usd_price.saturating_mul(dollar_cost.saturated_into());

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
					Self::deposit_event(Event::FeeRewarded {
						relayer: relayer.clone(),
						amount: reward_amount.saturated_into(),
					});
				} else {
					T::Currency::transfer(
						&relayer,
						&T::TreasuryAccount::get().into_account_truncating(),
						base_reward_in_token,
						Preservation::Expendable,
					)
						.map_err(|_| Error::<T>::RewardTransferFailed)?;

					Self::deposit_event(Event::FeePaid {
						relayer: relayer.clone(),
						amount: base_reward_in_token.into(),
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

	pub fn do_withdraw_fees(
		relayer: T::AccountId,
		source_chain: StateMachine
	) -> Result<(), DispatchError> {
		let nonce = Nonce::<T>::get(&relayer, &source_chain);

		let available_amount =
			AccumulatedFees::<T>::take(source_chain, &relayer);
		ensure!(available_amount > T::Balance::from(0u32), Error::<T>::NotEnoughBalance);

		let body = pallet_hyperbridge::Message::WithdrawRelayerFees(WithdrawalRequest {
			amount: available_amount,
			account: relayer.clone()
		}).encode();

		let dispatcher = <T as Config>::IsmpHost::default();

		let post = DispatchPost {
			dest: source_chain,
			from: MODULE_ID.to_vec(),
			to: relayer.as_ref().to_vec(),
			timeout: 0,
			body
		};

		dispatcher
			.dispatch_request(
				DispatchRequest::Post(post),
				FeeMetadata { payer: [0u8; 32].into(),  fee: Default::default() },
			)
			.map_err(|_| Error::<T>::DispatchFailed)?;

		Nonce::<T>::try_mutate(relayer.clone(), source_chain, |value| {
			*value = value.saturating_add(1);
			Ok::<(), ()>(())
		})
			.map_err(|_| Error::<T>::ErrorCompletingCall)?;

		Self::deposit_event(Event::Withdrawn {
			relayer,
			source_chain,
			amount: available_amount,
		});

		Ok(())
	}

	pub fn note_request_fee(commitment: H256, fee: u128) {
		let fee_balance: T::Balance = fee.saturated_into();
		CommitmentFees::<T>::insert(commitment, fee_balance);
	}
}
