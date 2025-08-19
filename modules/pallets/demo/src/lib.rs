// Copyright (c) 2025 Polytope Labs.
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

//! ISMP Assets
//! Simple Demo for Asset transfer over ISMP
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

use alloc::{
	format,
	string::{String, ToString},
};
use frame_support::{traits::fungible::Mutate, PalletId};
use ismp::{
	error::Error as IsmpError,
	host::StateMachine,
	module::IsmpModule,
	router::{PostRequest, Request, Response, Timeout},
};
pub use pallet::*;
use pallet_ismp::ModuleId;
use polkadot_sdk::{cumulus_primitives_core::Weight, *};
use sp_core::H160;

/// Constant Pallet ID
pub const PALLET_ID: ModuleId = ModuleId::Pallet(PalletId(*b"ismp-ast"));

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::{vec, vec::Vec};
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungible::{Inspect, Mutate},
			tokens::{Balance, Fortitude, Precision},
		},
	};
	use frame_system::pallet_prelude::*;
	use ismp::{
		consensus::{StateCommitment, StateMachineHeight},
		dispatcher::{DispatchGet, DispatchPost, DispatchRequest, FeeMetadata, IsmpDispatcher},
		host::{IsmpHost, StateMachine},
	};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Pallet Configuration
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_ismp::Config {
		/// Native balance
		type Balance: Balance
			+ Into<<Self::NativeCurrency as Inspect<Self::AccountId>>::Balance>
			+ From<u32>;
		/// Native currency implementation
		type NativeCurrency: Mutate<Self::AccountId>;
		/// Ismp message disptacher
		type IsmpHost: IsmpHost
			+ IsmpDispatcher<Account = Self::AccountId, Balance = <Self as Config>::Balance>
			+ Default;
	}

	/// Pallet events
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Some balance has been transferred
		BalanceTransferred {
			/// Source account
			from: T::AccountId,
			/// Destination account
			to: T::AccountId,
			/// Amount being transferred
			amount: <T as Config>::Balance,
			/// Destination chain's Id
			dest_chain: StateMachine,
		},
		/// Some balance has been received
		BalanceReceived {
			/// Source account
			from: T::AccountId,
			/// Receiving account
			to: T::AccountId,
			/// Amount that was received
			amount: <T as Config>::Balance,
			/// Source chain's Id
			source_chain: StateMachine,
		},

		/// Request data receieved
		Request {
			/// Source of the request
			source: StateMachine,
			/// utf-8 decoded data
			data: String,
		},

		/// Get response recieved
		GetResponse(Vec<Option<Vec<u8>>>),
	}

	/// Pallet Errors
	#[pallet::error]
	pub enum Error<T> {
		/// Error encountered when initializing transfer
		TransferFailed,
		/// Failed to dispatch get request
		GetDispatchFailed,
	}

	// Pallet implements [`Hooks`] trait to define some logic to execute in some context.
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Transfer some funds over ISMP
		#[pallet::weight(Weight::from_parts(1_000_000, 0))]
		#[pallet::call_index(0)]
		pub fn transfer(
			origin: OriginFor<T>,
			params: TransferParams<T::AccountId, <T as Config>::Balance>,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			// first, burn the requested amount
			<T::NativeCurrency as Mutate<T::AccountId>>::burn_from(
				&origin,
				params.amount.into(),
				frame_support::traits::tokens::Preservation::Expendable,
				Precision::Exact,
				Fortitude::Force,
			)?;

			// next, construct the request to be sent out
			let payload = Payload { to: params.to, from: origin.clone(), amount: params.amount };
			let dest = match <T as pallet_ismp::Config>::HostStateMachine::get() {
				StateMachine::Kusama(_) => StateMachine::Kusama(params.para_id),
				StateMachine::Polkadot(_) => StateMachine::Polkadot(params.para_id),
				_ => Err(DispatchError::Other("Pallet only supports parachain hosts"))?,
			};
			let post = DispatchPost {
				dest,
				from: PALLET_ID.to_bytes(),
				to: PALLET_ID.to_bytes(),
				timeout: params.timeout,
				body: payload.encode(),
			};

			// dispatch the request
			let dispatcher = T::IsmpHost::default();
			dispatcher
				.dispatch_request(
					DispatchRequest::Post(post),
					FeeMetadata { payer: origin, fee: Default::default() },
				)
				.map_err(|_| Error::<T>::TransferFailed)?;

			// let the user know, they've successfully sent the funds
			Self::deposit_event(Event::<T>::BalanceTransferred {
				from: payload.from,
				to: payload.to,
				amount: payload.amount,
				dest_chain: dest,
			});

			Ok(())
		}

		/// Get the total issuance of the native token in a counterparty
		/// parachain
		#[pallet::weight(Weight::from_parts(1_000_000, 0))]
		#[pallet::call_index(1)]
		pub fn get_request(origin: OriginFor<T>, params: GetRequest) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			let dest = match <T as pallet_ismp::Config>::HostStateMachine::get() {
				StateMachine::Kusama(_) => StateMachine::Kusama(params.para_id),
				StateMachine::Polkadot(_) => StateMachine::Polkadot(params.para_id),
				_ => Err(DispatchError::Other("Pallet only supports parachain hosts"))?,
			};

			let get = DispatchGet {
				dest,
				from: PALLET_ID.to_bytes(),
				keys: params.keys,
				height: params.height as u64,
				timeout: params.timeout,
				context: Default::default(),
			};

			let dispatcher = T::IsmpHost::default();
			dispatcher
				.dispatch_request(
					DispatchRequest::Get(get),
					FeeMetadata { payer: origin, fee: Default::default() },
				)
				.map_err(|_| Error::<T>::GetDispatchFailed)?;
			Ok(())
		}

		/// Dispatch request to a connected EVM chain.
		#[pallet::weight(Weight::from_parts(1_000_000, 0))]
		#[pallet::call_index(2)]
		pub fn dispatch_to_evm(origin: OriginFor<T>, params: EvmParams) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			let post = DispatchPost {
				dest: StateMachine::Evm(params.destination),
				from: PALLET_ID.to_bytes(),
				to: params.module.0.to_vec(),
				timeout: params.timeout,
				body: b"Hello from polkadot".to_vec(),
			};
			let dispatcher = T::IsmpHost::default();
			for _ in 0..params.count {
				// dispatch the request
				dispatcher
					.dispatch_request(
						DispatchRequest::Post(post.clone()),
						FeeMetadata { payer: origin.clone(), fee: Default::default() },
					)
					.map_err(|_| Error::<T>::TransferFailed)?;
			}
			Ok(())
		}

		/// Insert an unverified state commitment into the host, this is for testing purposes only.
		#[pallet::weight(Weight::from_parts(1_000_000, 0))]
		#[pallet::call_index(3)]
		pub fn set_state_commitment(
			origin: OriginFor<T>,
			height: StateMachineHeight,
			commitment: StateCommitment,
		) -> DispatchResult {
			use ismp::events::{Event, StateMachineUpdated};
			ensure_root(origin)?;
			let host = T::IsmpHost::default();

			// shouldn't return an error
			host.store_state_machine_commitment(height, commitment).unwrap();
			host.store_state_machine_update_time(height, host.timestamp()).unwrap();

			// deposit the event
			pallet_ismp::Pallet::<T>::deposit_event(
				Event::StateMachineUpdated(StateMachineUpdated {
					state_machine_id: height.id,
					latest_height: height.height,
				})
				.into(),
			);

			Ok(())
		}
	}

	/// Transfer payload
	/// This would be encoded to bytes as the request data
	#[derive(
		Clone, codec::Encode, codec::Decode, scale_info::TypeInfo, PartialEq, Eq, RuntimeDebug,
	)]
	pub struct Payload<AccountId, Balance> {
		/// Destination account
		pub to: AccountId,
		/// Source account
		pub from: AccountId,
		/// Amount to be transferred
		pub amount: Balance,
	}

	/// The get request payload
	#[derive(
		Clone,
		codec::Encode,
		codec::Decode,
		DecodeWithMemTracking,
		scale_info::TypeInfo,
		PartialEq,
		Eq,
		RuntimeDebug,
	)]
	pub struct GetRequest {
		/// Destination parachain
		pub para_id: u32,
		/// Height at which to read state
		pub height: u32,
		/// request timeout
		pub timeout: u64,
		/// Storage keys to read
		pub keys: Vec<Vec<u8>>,
	}

	/// Extrinsic Parameters for initializing a cross chain transfer
	#[derive(
		Clone,
		codec::Encode,
		codec::Decode,
		DecodeWithMemTracking,
		scale_info::TypeInfo,
		PartialEq,
		Eq,
		RuntimeDebug,
	)]
	pub struct TransferParams<AccountId, Balance> {
		/// Destination account
		pub to: AccountId,

		/// Amount to transfer
		pub amount: Balance,

		/// Destination parachain Id
		pub para_id: u32,

		/// Timeout timestamp on destination chain in seconds
		pub timeout: u64,
	}

	/// Extrisnic params for evm dispatch
	#[derive(
		Clone,
		codec::Encode,
		codec::Decode,
		DecodeWithMemTracking,
		scale_info::TypeInfo,
		PartialEq,
		Eq,
		RuntimeDebug,
	)]
	pub struct EvmParams {
		/// Destination module
		pub module: H160,

		/// Destination EVM host
		pub destination: u32,

		/// Timeout timestamp on destination chain in seconds
		pub timeout: u64,

		/// Request count
		pub count: u64,
	}
}

/// Module callback for the pallet
pub struct IsmpModuleCallback<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> Default for IsmpModuleCallback<T> {
	fn default() -> Self {
		Self(core::marker::PhantomData)
	}
}

fn weight() -> Weight {
	Weight::from_parts(0, 0)
}

impl<T: Config> IsmpModule for IsmpModuleCallback<T> {
	fn on_accept(&self, request: PostRequest) -> Result<Weight, anyhow::Error> {
		let source_chain = request.source;

		match source_chain {
			StateMachine::Evm(_) => Pallet::<T>::deposit_event(Event::Request {
				source: source_chain,
				data: unsafe { String::from_utf8_unchecked(request.body) },
			}),
			StateMachine::Polkadot(_) | StateMachine::Kusama(_) => {
				let payload =
					<Payload<T::AccountId, <T as Config>::Balance> as codec::Decode>::decode(
						&mut &*request.body,
					)
					.map_err(|_| IsmpError::Custom("Failed to decode request data".to_string()))?;
				<T::NativeCurrency as Mutate<T::AccountId>>::mint_into(
					&payload.to,
					payload.amount.into(),
				)
				.map_err(|_| IsmpError::Custom("Failed to mint funds".to_string()))?;
				Pallet::<T>::deposit_event(Event::<T>::BalanceReceived {
					from: payload.from,
					to: payload.to,
					amount: payload.amount,
					source_chain,
				});
			},
			source => Err(IsmpError::Custom(format!("Unsupported source {source:?}")))?,
		}

		Ok(weight())
	}

	fn on_response(&self, response: Response) -> Result<Weight, anyhow::Error> {
		match response {
			Response::Post(_) => Err(IsmpError::Custom(
				"Balance transfer protocol does not accept post responses".to_string(),
			))?,
			Response::Get(res) => Pallet::<T>::deposit_event(Event::<T>::GetResponse(
				res.values.into_iter().map(|storage_value| storage_value.value).collect(),
			)),
		};

		Ok(weight())
	}

	fn on_timeout(&self, timeout: Timeout) -> Result<Weight, anyhow::Error> {
		let request = match timeout {
			Timeout::Request(Request::Post(post)) => Request::Post(post),
			_ => Err(IsmpError::Custom("Only Post requests allowed, found Get".to_string()))?,
		};
		let source_chain = request.source_chain();

		let payload = <Payload<T::AccountId, <T as Config>::Balance> as codec::Decode>::decode(
			&mut &*request.body().expect("Request has been checked; qed"),
		)
		.map_err(|_| IsmpError::Custom("Failed to decode request data".to_string()))?;
		<T::NativeCurrency as Mutate<T::AccountId>>::mint_into(
			&payload.from,
			payload.amount.into(),
		)
		.map_err(|_| IsmpError::Custom("Failed to mint funds".to_string()))?;
		Pallet::<T>::deposit_event(Event::<T>::BalanceReceived {
			from: payload.from,
			to: payload.to,
			amount: payload.amount,
			source_chain,
		});
		Ok(weight())
	}
}
