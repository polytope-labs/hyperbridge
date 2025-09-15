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

//! The host executive is tasked with managing the ISMP hosts on all connected chains.

#![cfg_attr(not(feature = "std"), no_std)]

mod params;
pub mod withdrawal;

extern crate alloc;

pub use pallet::*;
pub use params::*;
use polkadot_sdk::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::withdrawal::WithdrawalParams;
	use alloc::{collections::BTreeMap, vec};
	use frame_support::{
		pallet_prelude::{OptionQuery, *},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use ismp::{
		dispatcher::{DispatchPost, DispatchRequest, FeeMetadata, IsmpDispatcher},
		host::StateMachine,
	};
	use pallet_hyperbridge::{Message, PALLET_HYPERBRIDGE};
	use pallet_ismp::ModuleId;
	use primitive_types::{H160, U256};

	/// ISMP module identifier
	pub const PALLET_ID: ModuleId = ModuleId::Pallet(PalletId(*b"hostexec"));

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_ismp::Config {
		/// The [`IsmpDispatcher`] implementation to use for dispatching requests
		type IsmpHost: IsmpDispatcher<Account = Self::AccountId, Balance = Self::Balance>;

		/// Origin for privileged actions
		type HostExecutiveOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	/// Host Params for all connected chains
	#[pallet::storage]
	#[pallet::getter(fn host_params)]
	pub type HostParams<T: Config> = StorageMap<
		_,
		Twox64Concat,
		StateMachine,
		HostParam<<T as pallet_ismp::Config>::Balance>,
		OptionQuery,
	>;

	/// EvmHost addresses of all connected Evm chains
	#[pallet::storage]
	#[pallet::getter(fn evm_hosts)]
	pub type EvmHosts<T: Config> = StorageMap<_, Twox64Concat, StateMachine, H160, OptionQuery>;

	/// Stores the fee token decimals for only substrate based chains
	#[pallet::storage]
	#[pallet::getter(fn fee_token_decimals)]
	pub type FeeTokenDecimals<T: Config> =
		StorageMap<_, Blake2_128Concat, StateMachine, u8, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// `HostExecutiveOrigin` has initiated a host parameter update to the mentioned state
		/// machine
		HostParamsUpdated {
			/// State machine's whose host params should be updated
			state_machine: StateMachine,
			/// The old host params
			old: HostParam<<T as pallet_ismp::Config>::Balance>,
			/// The new host params
			new: HostParam<<T as pallet_ismp::Config>::Balance>,
		},
		/// `HostExecutiveOrigin` has set the initial host parameters for the mentioned state
		/// machine
		HostParamsSet {
			/// State machine's whose host params should be updated
			state_machine: StateMachine,
			/// The new host params
			params: HostParam<<T as pallet_ismp::Config>::Balance>,
		},
		/// The address for some EvmHost has been set
		HostAddressSet {
			/// State machine's whose host EvmHost address was just added
			state_machine: StateMachine,
			/// The address of the IsmpHost
			address: H160,
		},
		/// The host address for some EvmHost has been udpated
		HostAddressUpdated {
			/// State machine's whose host EvmHost address was just added
			state_machine: StateMachine,
			/// The old address of the IsmpHost
			old_address: H160,
			/// The updated address of the IsmpHost
			new_address: H160,
		},
		/// Fee token decimals updated for a particular StateMachine
		FeeTokenDecimalsUpdated {
			/// StateMachine updated
			state_machine: StateMachine,
			/// Decimals updated to
			decimals: u8,
		},
		/// A call to withdraw protocol fees was executed
		Withdraw {
			/// Beneficiary address
			address: BoundedVec<u8, ConstU32<32>>,
			/// destination state machine
			state_machine: StateMachine,
			/// Amount withdrawn
			amount: U256,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Could not commit the outgoing request
		DispatchFailed,
		/// The requested state machine was unrecognized
		UnknownStateMachine,
		/// Mismatched state machine and HostParams
		MismatchedHostParams,
		/// The provided state machine is not a Substrate-based chain
		UnsupportedStateMachine,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: From<[u8; 32]>,
	{
		/// Initialize the host params for all the different state machines
		#[pallet::weight(T::DbWeight::get().writes(1))]
		#[pallet::call_index(0)]
		pub fn set_host_params(
			origin: OriginFor<T>,
			params: BTreeMap<StateMachine, HostParam<<T as pallet_ismp::Config>::Balance>>,
		) -> DispatchResult {
			T::HostExecutiveOrigin::ensure_origin(origin)?;

			for (state_machine, params) in params {
				HostParams::<T>::insert(state_machine.clone(), params.clone());
				Self::deposit_event(Event::<T>::HostParamsSet { state_machine, params });
			}

			Ok(())
		}

		/// Update the host params for the provided state machine
		#[pallet::weight(T::DbWeight::get().writes(1))]
		#[pallet::call_index(1)]
		pub fn update_host_params(
			origin: OriginFor<T>,
			state_machine: StateMachine,
			update: HostParamUpdate<T::Balance>,
		) -> DispatchResult {
			T::HostExecutiveOrigin::ensure_origin(origin)?;

			let params = HostParams::<T>::get(&state_machine)
				.ok_or_else(|| Error::<T>::UnknownStateMachine)?;

			let (post, updated) = match (params.clone(), update) {
				(HostParam::EvmHostParam(mut inner), HostParamUpdate::EvmHostParam(update)) => {
					inner.update(update);

					let body = EvmHostParamsAbi::try_from(inner.clone())
						.expect("u128 will always fit inside a U256; qed")
						.encode();

					let post = DispatchPost {
						dest: state_machine,
						from: PALLET_ID.to_bytes(),
						to: inner.host_manager.0.to_vec(),
						timeout: 0,
						body,
					};

					(post, HostParam::EvmHostParam(inner))
				},
				(HostParam::SubstrateHostParam(_), HostParamUpdate::SubstrateHostParam(update)) => {
					let body =
						Message::<T::AccountId, T::Balance>::UpdateHostParams(update.clone())
							.encode();

					let post = DispatchPost {
						dest: state_machine,
						from: PALLET_ID.to_bytes(),
						to: PALLET_HYPERBRIDGE.0.to_vec(),
						timeout: 0,
						body,
					};

					(post, HostParam::SubstrateHostParam(update))
				},
				_ => return Err(Error::<T>::MismatchedHostParams.into()),
			};

			let dispatcher = <T as Config>::IsmpHost::default();
			dispatcher
				.dispatch_request(
					DispatchRequest::Post(post),
					FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
				)
				.map_err(|_| Error::<T>::DispatchFailed)?;

			HostParams::<T>::insert(state_machine, updated.clone());

			Self::deposit_event(Event::<T>::HostParamsUpdated {
				state_machine,
				old: params,
				new: updated,
			});

			Ok(())
		}

		/// Set or update the addresses for the specified evm hosts
		#[pallet::weight(T::DbWeight::get().writes(1))]
		#[pallet::call_index(2)]
		pub fn update_evm_hosts(
			origin: OriginFor<T>,
			params: BTreeMap<StateMachine, H160>,
		) -> DispatchResult {
			T::HostExecutiveOrigin::ensure_origin(origin)?;

			for (state_machine, address) in params {
				let old = EvmHosts::<T>::get(&state_machine);
				EvmHosts::<T>::insert(state_machine.clone(), address);
				if let Some(old_address) = old {
					Self::deposit_event(Event::<T>::HostAddressUpdated {
						state_machine,
						old_address,
						new_address: address,
					});
				} else {
					Self::deposit_event(Event::<T>::HostAddressSet { state_machine, address });
				}
			}

			Ok(())
		}

		/// Sets the fee token decimals for substrate based chains
		#[pallet::weight(T::DbWeight::get().writes(1))]
		#[pallet::call_index(3)]
		pub fn set_fee_token_decimals(
			origin: OriginFor<T>,
			state_machine: StateMachine,
			decimals: u8,
		) -> DispatchResult {
			T::HostExecutiveOrigin::ensure_origin(origin)?;

			FeeTokenDecimals::<T>::insert(state_machine, decimals);

			Self::deposit_event(Event::FeeTokenDecimalsUpdated { state_machine, decimals });

			Ok(())
		}

		/// Issues a call to withdraw the protocol fees from an evm chain
		#[pallet::weight(T::DbWeight::get().writes(1))]
		#[pallet::call_index(4)]
		pub fn withdraw_protocol_fees(
			origin: OriginFor<T>,
			state_machine: StateMachine,
			withdrawal_params: WithdrawalParams,
		) -> DispatchResult {
			T::HostExecutiveOrigin::ensure_origin(origin)?;

			ensure!(state_machine.is_evm(), Error::<T>::UnsupportedStateMachine);

			let HostParam::EvmHostParam(params) = HostParams::<T>::get(state_machine)
				.ok_or_else(|| Error::<T>::UnknownStateMachine)?
			else {
				Err(Error::<T>::UnknownStateMachine)?
			};

			let data = withdrawal_params.abi_encode();

			let post = DispatchPost {
				dest: state_machine,
				from: PALLET_ID.to_bytes(),
				to: params.host_manager.0.to_vec(),
				timeout: 0,
				body: data,
			};

			let dispatcher = <T as Config>::IsmpHost::default();

			// Account is not useful in this case
			dispatcher
				.dispatch_request(
					DispatchRequest::Post(post),
					FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
				)
				.map_err(|_| Error::<T>::DispatchFailed)?;

			Self::deposit_event(Event::<T>::Withdraw {
				address: sp_runtime::BoundedVec::truncate_from(
					withdrawal_params.beneficiary_address,
				),
				state_machine,
				amount: withdrawal_params.amount,
			});

			Ok(())
		}
	}
}
