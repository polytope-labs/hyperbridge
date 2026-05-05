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

//! # Hyper Fungible Token Pallet
//!
//! This pallet enables cross-chain token transfers via HyperFungibleToken and
//! WrappedHyperFungibleToken Solidity contracts. Each registered token has its own
//! EVM contract address per chain, and the pallet sets its `from` address to match
//! what each contract expects.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod error;
pub mod impls;
pub mod module;
pub mod types;
pub mod weights;

pub use weights::WeightInfo;

use crate::impls::convert_to_erc20;
use alloy_sol_types::SolValue;
use frame_support::{
	traits::fungibles::{self, Mutate},
	PalletId,
};
use pallet_ismp::ModuleId;
use polkadot_sdk::*;
use primitive_types::H256;
use types::{AssetId, EvmToSubstrate, Message, SendParams, TokenRegistration, TokenUpdate};

use alloc::{vec, vec::Vec};

pub use pallet::*;

/// The well-known module ID for the hyper-fungible-token pallet.
/// EVM contracts should set this as the destination address when sending to this pallet.
pub const PALLET_ID: ModuleId = ModuleId::Pallet(PalletId(*b"pall_hft"));

const ETHEREUM_MESSAGE_PREFIX: &str = "\x19Ethereum Signed Message:\n";

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		traits::{
			tokens::{Fortitude, Precision, Preservation},
			Currency, ExistenceRequirement,
		},
	};
	use frame_system::pallet_prelude::*;
	use ismp::{
		dispatcher::{DispatchPost, DispatchRequest, FeeMetadata, IsmpDispatcher},
		host::StateMachine,
	};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_ismp::Config {
		/// The [`IsmpDispatcher`] for dispatching cross-chain requests
		type Dispatcher: IsmpDispatcher<Account = Self::AccountId, Balance = Self::Balance>;

		/// A currency implementation for interacting with the native asset
		type NativeCurrency: Currency<Self::AccountId>;

		/// Account that is authorized to register and update tokens
		type CreateOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Fungible asset implementation
		type Assets: fungibles::Mutate<Self::AccountId>
			+ fungibles::metadata::Inspect<Self::AccountId>;

		/// The native asset ID
		type NativeAssetId: Get<AssetId<Self>>;

		/// The decimals of the native currency
		#[pallet::constant]
		type Decimals: Get<u8>;

		/// Converts an EVM address to a substrate account.
		/// Used for authenticating incoming cross-chain runtime calls.
		type EvmToSubstrate: EvmToSubstrate<Self>;

		/// Weight information for extrinsics in this pallet
		type WeightInfo: WeightInfo;
	}

	/// Maps (StateMachine, AssetId) → EVM contract address of the token on that chain.
	/// Used as the `to` field in outgoing DispatchPost.
	#[pallet::storage]
	pub type TokenContracts<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		StateMachine,
		Blake2_128Concat,
		AssetId<T>,
		Vec<u8>,
		OptionQuery,
	>;

	/// Reverse lookup: (StateMachine, contract address bytes) → local asset ID.
	/// Used in on_accept to find which local asset an incoming message corresponds to.
	#[pallet::storage]
	pub type ContractToAsset<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		StateMachine,
		Blake2_128Concat,
		Vec<u8>,
		AssetId<T>,
		OptionQuery,
	>;

	/// Whether this asset is native to this chain (custody model) or non-native (mint/burn)
	#[pallet::storage]
	pub type NativeAssets<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetId<T>, bool, ValueQuery>;

	/// EVM decimals per (AssetId, StateMachine) for precision conversion
	#[pallet::storage]
	pub type Precisions<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AssetId<T>,
		Blake2_128Concat,
		StateMachine,
		u8,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A cross-chain token transfer has been dispatched
		TokenSent {
			/// Source account
			from: T::AccountId,
			/// Recipient on destination chain
			to: BoundedVec<u8, sp_core::ConstU32<32>>,
			/// Amount transferred (local denomination)
			amount: <T::NativeCurrency as Currency<T::AccountId>>::Balance,
			/// Destination chain
			dest: StateMachine,
			/// Request commitment
			commitment: H256,
		},

		/// Tokens received from a cross-chain transfer
		TokenReceived {
			/// Beneficiary on this chain
			beneficiary: T::AccountId,
			/// Amount received (local denomination)
			amount: <<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance,
			/// Source chain
			source: StateMachine,
		},

		/// Tokens refunded after a timeout
		TokenRefunded {
			/// Refund beneficiary
			beneficiary: T::AccountId,
			/// Amount refunded (local denomination)
			amount: <<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance,
			/// The chain the original transfer was destined for
			dest: StateMachine,
		},

		/// A token has been registered
		TokenRegistered {
			/// Local asset ID
			asset_id: AssetId<T>,
			/// Whether this asset is native (custody) or non-native (mint/burn)
			native: bool,
			/// The chains this token was registered on
			chains: Vec<StateMachine>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Asset is not registered in this pallet
		UnregisteredAsset,
		/// Token contract not configured for this chain
		TokenContractNotFound,
		/// Pallet address not configured for this chain
		PalletAddressNotFound,
		/// Asset decimals not configured for this chain
		DecimalsNotFound,
		/// Error while transferring or burning assets
		AssetTransferError,
		/// Error dispatching the cross-chain request
		DispatchError,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
		u128: From<<<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance>,
		<T as pallet_ismp::Config>::Balance:
			From<<<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance>,
		<<T as Config>::Assets as fungibles::Inspect<T::AccountId>>::Balance:
			From<<<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance>,
		<<T as Config>::Assets as fungibles::Inspect<T::AccountId>>::Balance: From<u128>,
		[u8; 32]: From<<T as frame_system::Config>::AccountId>,
	{
		/// Sends tokens cross-chain to a HyperFungibleToken or WrappedHyperFungibleToken contract
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::send())]
		pub fn send(
			origin: OriginFor<T>,
			params: SendParams<
				AssetId<T>,
				<<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance,
			>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let dispatcher = <T as Config>::Dispatcher::default();

			let token_contract =
				TokenContracts::<T>::get(params.destination, params.asset_id.clone())
					.ok_or(Error::<T>::TokenContractNotFound)?;
			let erc_decimals = Precisions::<T>::get(params.asset_id.clone(), params.destination)
				.ok_or(Error::<T>::DecimalsNotFound)?;

			// Lock or burn the local asset
			let decimals = if params.asset_id == T::NativeAssetId::get() {
				// escrow the native asset
				<T as Config>::NativeCurrency::transfer(
					&who,
					&Self::pallet_account(),
					params.amount,
					ExistenceRequirement::AllowDeath,
				)?;
				T::Decimals::get()
			} else {
				let is_native = NativeAssets::<T>::get(params.asset_id.clone());
				if is_native {
					<T as Config>::Assets::transfer(
						params.asset_id.clone(),
						&who,
						&Self::pallet_account(),
						params.amount.into(),
						Preservation::Expendable,
					)?;
				} else {
					<T as Config>::Assets::burn_from(
						params.asset_id.clone(),
						&who,
						params.amount.into(),
						Preservation::Expendable,
						Precision::Exact,
						Fortitude::Polite,
					)?;
				}
				<T::Assets as fungibles::metadata::Inspect<T::AccountId>>::decimals(
					params.asset_id.clone(),
				)
			};

			// Encode the Message body
			let sender: [u8; 32] = who.clone().into();
			let amount: u128 = params.amount.into();
			let erc20_amount = convert_to_erc20(amount, erc_decimals, decimals);

			let token_message = Message {
				from: sender.to_vec().into(),
				to: params.recipient.to_vec().into(),
				amount: alloy_primitives::U256::from_be_bytes(erc20_amount.to_big_endian()),
				data: params.call_data.unwrap_or_default().into(),
			};

			let dispatch_post = DispatchPost {
				dest: params.destination,
				from: PALLET_ID.to_bytes(),
				to: token_contract,
				timeout: params.timeout,
				body: Message::abi_encode(&token_message),
			};

			let metadata = FeeMetadata { payer: who.clone(), fee: params.relayer_fee.into() };
			let commitment = dispatcher
				.dispatch_request(DispatchRequest::Post(dispatch_post), metadata)
				.map_err(|_| Error::<T>::DispatchError)?;

			Self::deposit_event(Event::<T>::TokenSent {
				from: who,
				to: params.recipient,
				dest: params.destination,
				amount: params.amount,
				commitment,
			});
			Ok(())
		}

		/// Registers a new token with per-chain contract configuration
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::register_token(registration.chains.len() as u32))]
		pub fn register_token(
			origin: OriginFor<T>,
			registration: TokenRegistration<AssetId<T>>,
		) -> DispatchResult {
			T::CreateOrigin::ensure_origin(origin)?;

			NativeAssets::<T>::insert(registration.local_id.clone(), registration.native);

			let chains: Vec<StateMachine> = registration.chains.keys().cloned().collect();
			for (chain, config) in registration.chains {
				TokenContracts::<T>::insert(
					chain,
					registration.local_id.clone(),
					config.token_contract.clone(),
				);
				ContractToAsset::<T>::insert(
					chain,
					config.token_contract,
					registration.local_id.clone(),
				);
				Precisions::<T>::insert(registration.local_id.clone(), chain, config.decimals);
			}

			Self::deposit_event(Event::<T>::TokenRegistered {
				asset_id: registration.local_id,
				native: registration.native,
				chains,
			});
			Ok(())
		}

		/// Updates chain configuration for an existing token
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::update_token(update.add_chains.len() as u32))]
		pub fn update_token(
			origin: OriginFor<T>,
			update: TokenUpdate<AssetId<T>>,
		) -> DispatchResult {
			T::CreateOrigin::ensure_origin(origin)?;

			for (chain, config) in update.add_chains {
				// Remove old reverse mapping if it exists
				if let Some(old_contract) = TokenContracts::<T>::get(chain, update.asset_id.clone())
				{
					ContractToAsset::<T>::remove(chain, old_contract);
				}

				TokenContracts::<T>::insert(
					chain,
					update.asset_id.clone(),
					config.token_contract.clone(),
				);
				ContractToAsset::<T>::insert(chain, config.token_contract, update.asset_id.clone());
				Precisions::<T>::insert(update.asset_id.clone(), chain, config.decimals);
			}

			for chain in update.remove_chains {
				if let Some(old_contract) = TokenContracts::<T>::get(chain, update.asset_id.clone())
				{
					ContractToAsset::<T>::remove(chain, old_contract);
				}
				TokenContracts::<T>::remove(chain, update.asset_id.clone());
				Precisions::<T>::remove(update.asset_id.clone(), chain);
			}

			Ok(())
		}
	}

	impl<T> Default for Pallet<T> {
		fn default() -> Self {
			Self(PhantomData)
		}
	}
}

