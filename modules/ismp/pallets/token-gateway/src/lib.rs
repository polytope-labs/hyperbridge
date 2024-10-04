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

//! The token gateway enables asset transfers to EVM instances of token gateway
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod impls;
pub mod types;
use crate::impls::{convert_to_balance, convert_to_erc20};
use alloy_sol_types::SolValue;
use codec::Decode;
use frame_support::{
	ensure,
	pallet_prelude::Weight,
	traits::{
		fungibles::{self, Mutate},
		tokens::Preservation,
		Currency, ExistenceRequirement,
	},
	PalletId,
};
use ismp::{
	events::Meta,
	router::{PostRequest, Request, Response, Timeout},
};
use sp_core::{Get, U256};
pub use types::*;

use alloc::{string::ToString, vec};
use ismp::module::IsmpModule;
use primitive_types::{H160, H256};

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

/// The module id for this pallet
pub const PALLET_ID: PalletId = PalletId(*b"tokengtw");

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		traits::{tokens::Preservation, Currency, ExistenceRequirement},
	};
	use frame_system::pallet_prelude::*;
	use ismp::{
		dispatcher::{DispatchPost, DispatchRequest, FeeMetadata, IsmpDispatcher},
		host::StateMachine,
	};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The pallet's configuration trait.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_ismp::Config {
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The [`IsmpDispatcher`] for dispatching cross-chain requests
		type Dispatcher: IsmpDispatcher<Account = Self::AccountId, Balance = Self::Balance>;

		/// A currency implementation for interacting with the native asset
		type Currency: Currency<Self::AccountId>;

		/// Fungible asset implementation
		type Assets: fungibles::Mutate<Self::AccountId> + fungibles::Inspect<Self::AccountId>;

		/// The native asset ID
		type NativeAssetId: Get<AssetId<Self>>;
	}

	/// Assets supported by this instance of token gateway
	/// A map of the local asset id to the token gateway asset id
	#[pallet::storage]
	pub type SupportedAssets<T: Config> = StorageMap<_, Identity, AssetId<T>, H256, OptionQuery>;

	/// Assets supported by this instance of token gateway
	/// A map of the token gateway asset id to the local asset id
	#[pallet::storage]
	pub type LocalAssets<T: Config> = StorageMap<_, Identity, H256, AssetId<T>, OptionQuery>;

	/// The token gateway adresses on different chains
	#[pallet::storage]
	pub type TokenGatewayAddresses<T: Config> =
		StorageMap<_, Identity, StateMachine, H160, OptionQuery>;

	/// Pallet events that functions in this pallet can emit.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An asset has been teleported
		AssetTeleported {
			/// Source account on the relaychain
			from: T::AccountId,
			/// beneficiary account on destination
			to: H160,
			/// Amount transferred
			amount: <<T as Config>::Currency as Currency<T::AccountId>>::Balance,
			/// Destination chain
			dest: StateMachine,
			/// Request commitment
			commitment: H256,
		},

		/// An asset has been received and transferred to the beneficiary's account
		AssetReceived {
			/// beneficiary account on relaychain
			beneficiary: T::AccountId,
			/// Amount transferred
			amount: <<T as Config>::Currency as Currency<T::AccountId>>::Balance,
			/// Destination chain
			source: StateMachine,
		},

		/// An asset has been refunded and transferred to the beneficiary's account
		AssetRefunded {
			/// beneficiary account on relaychain
			beneficiary: T::AccountId,
			/// Amount transferred
			amount: <<T as Config>::Currency as Currency<T::AccountId>>::Balance,
			/// Destination chain
			source: StateMachine,
		},
		/// Token Gateway address enquiry dispatched
		AddressEnquiryDispatched { commitment: H256 },
	}

	/// Errors that can be returned by this pallet.
	#[pallet::error]
	pub enum Error<T> {
		/// A asset that has not been registered
		UnregisteredAsset,
		/// A state machine does not have the token gateway address registered
		UnregisteredDestinationChain,
		/// Error while teleporting asset
		AssetTeleportError,
		/// Coprocessor was not configured in the runtime
		CoprocessorNotConfigured,
		/// A request to query the token gateway addresses failed to dispatch
		AddressEnquiryDispatchFailed,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
		u128: From<<<T as Config>::Currency as Currency<T::AccountId>>::Balance>,
		<<T as Config>::Assets as fungibles::Inspect<T::AccountId>>::Balance:
			From<<<T as Config>::Currency as Currency<T::AccountId>>::Balance>,
		[u8; 32]: From<<T as frame_system::Config>::AccountId>,
	{
		/// Teleports a registered asset
		/// locks the asset and dispatches a request to token gateway on the destination
		#[pallet::call_index(0)]
		#[pallet::weight(weight())]
		pub fn teleport(
			origin: OriginFor<T>,
			params: TeleportParams<
				AssetId<T>,
				<<T as Config>::Currency as Currency<T::AccountId>>::Balance,
			>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let dispatcher = <T as Config>::Dispatcher::default();
			let asset_id = SupportedAssets::<T>::get(params.asset_id.clone())
				.ok_or_else(|| Error::<T>::UnregisteredAsset)?;
			if params.asset_id == T::NativeAssetId::get() {
				// Custody funds in pallet
				<T as Config>::Currency::transfer(
					&who,
					&Self::pallet_account(),
					params.amount,
					ExistenceRequirement::KeepAlive,
				)?;
			} else {
				<T as Config>::Assets::transfer(
					params.asset_id,
					&who,
					&Self::pallet_account(),
					params.amount.into(),
					Preservation::Protect,
				)?;
			}

			// Dispatch Ismp request
			// Token gateway expected abi encoded address
			let mut to = [0u8; 32];
			to[12..].copy_from_slice(&params.recepient.0);
			let from: [u8; 32] = who.clone().into();

			let body = Body {
				amount: {
					let amount: u128 = params.amount.into();
					let mut bytes = [0u8; 32];
					convert_to_erc20(amount).to_big_endian(&mut bytes);
					alloy_primitives::U256::from_be_bytes(bytes)
				},
				asset_id: asset_id.0.into(),
				redeem: false,
				from: from.into(),
				to: to.into(),
			};

			let token_gateway_address = TokenGatewayAddresses::<T>::get(params.destination)
				.ok_or_else(|| Error::<T>::UnregisteredDestinationChain)?;

			let dispatch_post = DispatchPost {
				dest: params.destination,
				from: token_gateway_address.0.to_vec(),
				to: token_gateway_address.0.to_vec(),
				timeout: params.timeout,
				body: {
					// Prefix with the handleIncomingAsset enum variant
					let mut encoded = vec![0];
					encoded.extend_from_slice(&Body::abi_encode(&body));
					encoded
				},
			};

			let metadata = FeeMetadata { payer: who.clone(), fee: Default::default() };
			let commitment = dispatcher
				.dispatch_request(DispatchRequest::Post(dispatch_post), metadata)
				.map_err(|_| Error::<T>::AssetTeleportError)?;

			Self::deposit_event(Event::<T>::AssetTeleported {
				from: who,
				to: params.recepient,
				dest: params.destination,
				amount: params.amount,
				commitment,
			});
			Ok(())
		}

		/// Request the token gateway address from Hyperbridge for specified chains
		#[pallet::call_index(1)]
		#[pallet::weight(weight())]
		pub fn request_token_gateway_address(
			origin: OriginFor<T>,
			chains: BoundedVec<StateMachine, ConstU32<5>>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			let request = TokenGatewayAddressRequest { chains };
			let dispatcher = <T as Config>::Dispatcher::default();
			let dispatch_post = DispatchPost {
				dest: T::Coprocessor::get().ok_or_else(|| Error::<T>::CoprocessorNotConfigured)?,
				from: PALLET_ID.0.to_vec(),
				to: PALLET_ID.0.to_vec(),
				timeout: 0,
				body: request.encode(),
			};

			let metadata = FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() };
			let commitment = dispatcher
				.dispatch_request(DispatchRequest::Post(dispatch_post), metadata)
				.map_err(|_| Error::<T>::AddressEnquiryDispatchFailed)?;
			Self::deposit_event(Event::<T>::AddressEnquiryDispatched { commitment });
			Ok(())
		}

		/// Map some local assets to their token gateway asset ids
		#[pallet::call_index(2)]
		#[pallet::weight(weight())]
		pub fn register_assets(
			origin: OriginFor<T>,
			assets: AssetRegistration<AssetId<T>>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			for asset_map in assets.assets {
				SupportedAssets::<T>::insert(
					asset_map.local_id.clone(),
					asset_map.token_gateway_asset_id.clone(),
				);
				LocalAssets::<T>::insert(asset_map.token_gateway_asset_id, asset_map.local_id);
			}
			Ok(())
		}
	}

	// Hack for implementing the [`Default`] bound needed for
	// [`IsmpDispatcher`](ismp::dispatcher::IsmpDispatcher) and
	// [`IsmpModule`](ismp::module::IsmpModule)
	impl<T> Default for Pallet<T> {
		fn default() -> Self {
			Self(PhantomData)
		}
	}
}

impl<T: Config> IsmpModule for Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	<<T as Config>::Currency as Currency<T::AccountId>>::Balance: From<u128>,
	<<T as Config>::Assets as fungibles::Inspect<T::AccountId>>::Balance: From<u128>,
{
	fn on_accept(
		&self,
		PostRequest { body, from, source, dest, nonce, .. }: PostRequest,
	) -> Result<(), ismp::error::Error> {
		ensure!(
			from == TokenGatewayAddresses::<T>::get(source).unwrap_or_default().0.to_vec(),
			ismp::error::Error::ModuleDispatchError {
				msg: "Token Gateway: Unknown source contract address".to_string(),
				meta: Meta { source, dest, nonce },
			}
		);

		let body = Body::abi_decode(&mut &body[1..], true).map_err(|_| {
			ismp::error::Error::ModuleDispatchError {
				msg: "Token Gateway: Failed to decode request body".to_string(),
				meta: Meta { source, dest, nonce },
			}
		})?;
		let amount = convert_to_balance(U256::from_big_endian(&body.amount.to_be_bytes::<32>()))
			.map_err(|_| ismp::error::Error::ModuleDispatchError {
				msg: "Token Gateway: Trying to withdraw Invalid amount".to_string(),
				meta: Meta { source, dest, nonce },
			})?;

		let local_asset_id =
			LocalAssets::<T>::get(H256::from(body.asset_id.0)).ok_or_else(|| {
				ismp::error::Error::ModuleDispatchError {
					msg: "Token Gateway: Unknown asset".to_string(),
					meta: Meta { source, dest, nonce },
				}
			})?;
		let beneficiary: T::AccountId = body.to.0.into();
		if local_asset_id == T::NativeAssetId::get() {
			<T as Config>::Currency::transfer(
				&Pallet::<T>::pallet_account(),
				&beneficiary,
				amount.into(),
				ExistenceRequirement::AllowDeath,
			)
			.map_err(|_| ismp::error::Error::ModuleDispatchError {
				msg: "Token Gateway: Failed to complete asset transfer".to_string(),
				meta: Meta { source, dest, nonce },
			})?;
		} else {
			<T as Config>::Assets::transfer(
				local_asset_id,
				&Pallet::<T>::pallet_account(),
				&beneficiary,
				amount.into(),
				Preservation::Protect,
			)
			.map_err(|_| ismp::error::Error::ModuleDispatchError {
				msg: "Token Gateway: Failed to complete asset transfer".to_string(),
				meta: Meta { source, dest, nonce },
			})?;
		}

		Self::deposit_event(Event::<T>::AssetReceived {
			beneficiary,
			amount: amount.into(),
			source,
		});
		Ok(())
	}

	fn on_response(&self, response: Response) -> Result<(), ismp::error::Error> {
		let data = response.response().ok_or_else(|| ismp::error::Error::ModuleDispatchError {
			msg: "Token Gateway: Response has no body".to_string(),
			meta: Meta {
				source: response.source_chain(),
				dest: response.dest_chain(),
				nonce: response.nonce(),
			},
		})?;
		let resp = TokenGatewayAddressResponse::decode(&mut &*data).map_err(|_| {
			ismp::error::Error::ModuleDispatchError {
				msg: "Token Gateway: Failed to decode response body".to_string(),
				meta: Meta {
					source: response.source_chain(),
					dest: response.dest_chain(),
					nonce: response.nonce(),
				},
			}
		})?;
		for (state_machine, addr) in resp.addresses {
			TokenGatewayAddresses::<T>::insert(state_machine, addr)
		}
		Ok(())
	}

	fn on_timeout(&self, request: Timeout) -> Result<(), ismp::error::Error> {
		match request {
			Timeout::Request(Request::Post(PostRequest { body, source, dest, nonce, .. })) => {
				let body = Body::abi_decode(&mut &body[1..], true).map_err(|_| {
					ismp::error::Error::ModuleDispatchError {
						msg: "Token Gateway: Failed to decode request body".to_string(),
						meta: Meta { source, dest, nonce },
					}
				})?;
				let beneficiary = body.from.0.into();

				let amount =
					convert_to_balance(U256::from_big_endian(&body.amount.to_be_bytes::<32>()))
						.map_err(|_| ismp::error::Error::ModuleDispatchError {
							msg: "Token Gateway: Trying to withdraw Invalid amount".to_string(),
							meta: Meta { source, dest, nonce },
						})?;
				let local_asset_id = LocalAssets::<T>::get(H256::from(body.asset_id.0))
					.ok_or_else(|| ismp::error::Error::ModuleDispatchError {
						msg: "Token Gateway: Unknown asset".to_string(),
						meta: Meta { source, dest, nonce },
					})?;

				if local_asset_id == T::NativeAssetId::get() {
					<T as Config>::Currency::transfer(
						&Pallet::<T>::pallet_account(),
						&beneficiary,
						amount.into(),
						ExistenceRequirement::AllowDeath,
					)
					.map_err(|_| ismp::error::Error::ModuleDispatchError {
						msg: "Token Gateway: Failed to complete asset transfer".to_string(),
						meta: Meta { source, dest, nonce },
					})?;
				} else {
					<T as Config>::Assets::transfer(
						local_asset_id,
						&Pallet::<T>::pallet_account(),
						&beneficiary,
						amount.into(),
						Preservation::Protect,
					)
					.map_err(|_| ismp::error::Error::ModuleDispatchError {
						msg: "Token Gateway: Failed to complete asset transfer".to_string(),
						meta: Meta { source, dest, nonce },
					})?;
				}

				Pallet::<T>::deposit_event(Event::<T>::AssetRefunded {
					beneficiary,
					amount: amount.into(),
					source: dest,
				});
			},
			Timeout::Request(Request::Get(get)) => Err(ismp::error::Error::ModuleDispatchError {
				msg: "Tried to timeout unsupported request type".to_string(),
				meta: Meta { source: get.source, dest: get.dest, nonce: get.nonce },
			})?,

			Timeout::Response(response) => Err(ismp::error::Error::ModuleDispatchError {
				msg: "Tried to timeout unsupported request type".to_string(),
				meta: Meta {
					source: response.source_chain(),
					dest: response.dest_chain(),
					nonce: response.nonce(),
				},
			})?,
		}
		Ok(())
	}
}

/// Static weights because benchmarks suck, and we'll be getting PolkaVM soon anyways
fn weight() -> Weight {
	Weight::from_parts(300_000_000, 0)
}
