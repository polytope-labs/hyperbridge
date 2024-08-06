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

//! The token governor handles asset registration as well as tracks the metadata of multi-chain
//! native tokens across all connected chains
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod impls;
mod types;
use alloy_sol_types::SolValue;
use frame_support::pallet_prelude::Weight;
use ismp::router::{PostRequest, Response, Timeout};
pub use types::*;

use alloc::{format, vec};
use ismp::module::IsmpModule;
use primitive_types::{H160, H256};

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

/// The module id for this pallet
pub const PALLET_ID: [u8; 8] = *b"registry";

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::collections::BTreeMap;
	use frame_support::{
		pallet_prelude::*,
		traits::{fungible::Mutate, tokens::Preservation},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use ismp::{dispatcher::IsmpDispatcher, host::StateMachine};
	use sp_runtime::traits::AccountIdConversion;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The pallet's configuration trait.
	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_ismp::Config + pallet_ismp_host_executive::Config
	{
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The [`IsmpDispatcher`] for dispatching cross-chain requests
		type Dispatcher: IsmpDispatcher<Account = Self::AccountId, Balance = Self::Balance>;

		/// The account id for the treasury
		type TreasuryAccount: Get<PalletId>;
	}

	/// Maps a pending asset to it's owner. Enables asset registration without the native token by
	/// prviding a single-use execution ticket for asset creation through an unsigned transaction.
	#[pallet::storage]
	pub type PendingAsset<T: Config> = StorageMap<_, Identity, H256, H160, OptionQuery>;

	/// Tracks which assets have been deployed to which chains. Uses bool to ensure its persisted to
	/// the underlying trie
	#[pallet::storage]
	pub type SupportedChains<T: Config> =
		StorageDoubleMap<_, Identity, H256, Twox64Concat, StateMachine, bool, OptionQuery>;

	/// Mapping of AssetId to their metadata
	#[pallet::storage]
	pub type AssetMetadatas<T: Config> = StorageMap<_, Identity, H256, AssetMetadata, OptionQuery>;

	/// Mapping of AssetIds to their owners
	#[pallet::storage]
	pub type AssetOwners<T: Config> =
		StorageMap<_, Identity, H256, <T as frame_system::Config>::AccountId, OptionQuery>;

	/// TokenGovernor protocol parameters.
	#[pallet::storage]
	pub type ProtocolParams<T: Config> =
		StorageValue<_, Params<<T as pallet_ismp::Config>::Balance>, OptionQuery>;

	/// TokenRegistrar protocol parameters.
	#[pallet::storage]
	pub type TokenRegistrarParams<T: Config> =
		StorageMap<_, Twox64Concat, StateMachine, RegistrarParams, OptionQuery>;

	/// TokenGateway protocol parameters.
	#[pallet::storage]
	pub type TokenGatewayParams<T: Config> =
		StorageMap<_, Twox64Concat, StateMachine, GatewayParams, OptionQuery>;

	/// Pallet events that functions in this pallet can emit.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new asset has been registered
		AssetRegistered {
			/// The asset identifier
			asset_id: H256,
		},
		/// A new pending asset has been registered
		NewPendingAsset {
			/// The pending asset identifier
			asset_id: H256,
			/// Owner of the asset
			owner: H160,
		},
		/// The TokenRegistrar params have been updated for a state machine
		RegistrarParamsUpdated {
			/// The old params
			old: RegistrarParams,
			/// The new params
			new: RegistrarParams,
			/// The state machine it was updated for
			state_machine: StateMachine,
		},
		/// The TokenGateway params have been updated for a state machine
		GatewayParamsUpdated {
			/// The old params
			old: GatewayParams,
			/// The new params
			new: GatewayParams,
			/// The state machine it was updated for
			state_machine: StateMachine,
		},
		/// The TokenGovernor parameters have been updated
		ParamsUpdated {
			/// The old parameters
			old: Params<<T as pallet_ismp::Config>::Balance>,
			/// The new parameters
			new: Params<<T as pallet_ismp::Config>::Balance>,
		},
	}

	/// Errors that can be returned by this pallet.
	#[pallet::error]
	pub enum Error<T> {
		/// An asset with the same identifier already exists
		AssetAlreadyExists,
		/// The pallet has not yet been initialized
		NotInitialized,
		/// Failed to dispatch a request
		DispatchFailed,
		/// Provided name or symbol isn't valid utf-8
		InvalidUtf8,
		/// Provided asset identifier was unknown
		UnknownAsset,
		/// The account signer is not authorized to perform this action
		NotAssetOwner,
		/// Provided signature was invalid
		InvalidSignature,
		/// Unknown token gateway instance
		UnknownTokenGateway,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
		<T as pallet_ismp::Config>::Balance: Default,
	{
		/// Registers a multi-chain ERC6160 asset. The asset should not already exist.
		///
		/// This works by dispatching a request to the TokenGateway module on each requested chain
		/// to create the asset.
		#[pallet::call_index(0)]
		#[pallet::weight(weight())]
		pub fn create_erc6160_asset(
			origin: OriginFor<T>,
			asset: ERC6160AssetRegistration,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let Params { registration_fee, .. } =
				ProtocolParams::<T>::get().ok_or_else(|| Error::<T>::NotInitialized)?;
			T::Currency::transfer(
				&who,
				&T::TreasuryAccount::get().into_account_truncating(),
				registration_fee,
				Preservation::Preserve,
			)?;

			Self::register_asset(asset, who)?;

			Ok(())
		}

		/// Registers a multi-chain ERC6160 asset. The asset should not already exist.
		///
		/// Registration fees are paid through the TokenRegistrar. The pallet must have
		/// previously received the asset to be created as a request from a TokenRegistrar otherwise
		/// this will fail
		#[pallet::call_index(1)]
		#[pallet::weight(weight())]
		pub fn create_erc6160_asset_unsigned(
			origin: OriginFor<T>,
			registration: UnsignedERC6160AssetRegistration<T::AccountId>,
		) -> DispatchResult {
			ensure_none(origin)?;

			Self::register_asset_unsigned(registration)?;

			Ok(())
		}

		/// Dispatches a request to update the TokenRegistrar contract parameters
		#[pallet::call_index(2)]
		#[pallet::weight(weight())]
		pub fn update_registrar_params(
			origin: OriginFor<T>,
			update: BTreeMap<StateMachine, RegistrarParamsUpdate>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			Self::update_registrar_params_impl(update)?;

			Ok(())
		}

		/// Dispatches a request to update the TokenRegistrar contract parameters
		#[pallet::call_index(3)]
		#[pallet::weight(weight())]
		pub fn update_gateway_params(
			origin: OriginFor<T>,
			update: BTreeMap<StateMachine, TokenGatewayParamsUpdate>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			Self::update_gateway_params_impl(update)?;

			Ok(())
		}

		/// Updates the TokenGovernor pallet parameters.
		#[pallet::call_index(4)]
		#[pallet::weight(weight())]
		pub fn update_params(
			origin: OriginFor<T>,
			update: ParamsUpdate<<T as pallet_ismp::Config>::Balance>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			let stored_params = ProtocolParams::<T>::get();

			let old_params = stored_params.unwrap_or_default();
			let new_params = old_params.update(update);

			ProtocolParams::<T>::set(Some(new_params));

			Ok(())
		}

		/// This allows the asset owner to update their Multi-chain native asset.
		/// They are allowed to:
		/// 1. Change the logo
		/// 2. Dispatch a request to create the asset to any new chains
		/// 3. Dispatch a request to delist the asset from the TokenGateway contract on any
		///    previously supported chain (Should be used with caution)
		/// 4. Dispatch a request to change the asset admin to another address.
		#[pallet::call_index(5)]
		#[pallet::weight(weight())]
		pub fn update_erc6160_asset(
			origin: OriginFor<T>,
			update: ERC6160AssetUpdate,
		) -> DispatchResult {
			Self::ensure_root_or_owner(origin, update.asset_id)?;

			Self::update_erc6160_asset_impl(update)?;

			Ok(())
		}

		/// Dispatches a request to update the Asset fees on the provided chain
		#[pallet::call_index(6)]
		#[pallet::weight(weight())]
		pub fn create_erc20_asset(
			origin: OriginFor<T>,
			asset: ERC20AssetRegistration,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			Self::create_erc20_asset_impl(asset)?;

			Ok(())
		}

		/// Adds a new token gateway contract instance to all existing instances
		#[pallet::call_index(7)]
		#[pallet::weight(weight())]
		pub fn new_contract_instance(
			origin: OriginFor<T>,
			updates: BTreeMap<StateMachine, TokenGatewayParamsUpdate>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			Self::add_new_gateway_instance(updates)?;

			Ok(())
		}
	}

	/// This allows users to create assets from any chain using the TokenRegistrar.
	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	{
		type Call = Call<T>;

		// empty pre-dispatch do we don't modify storage
		fn pre_dispatch(_call: &Self::Call) -> Result<(), TransactionValidityError> {
			Ok(())
		}

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			let (res, asset_id) = match call {
				Call::create_erc6160_asset_unsigned { registration } => {
					let asset_id: H256 =
						sp_io::hashing::keccak_256(registration.asset.symbol.as_ref()).into();

					(Self::register_asset_unsigned(registration.clone()), asset_id)
				},
				_ => Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?,
			};

			if let Err(err) = res {
				log::error!(target: "ismp", "TokenGovernor Validation error {err:?}");
				Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?
			}

			Ok(ValidTransaction {
				// they should all have the same priority so they can be rejected
				priority: 1,
				// they are all self-contained batches that have no dependencies
				requires: vec![],
				// provides this unique hash of transactions
				provides: vec![asset_id.0.to_vec()],
				// should only live for at most 10 blocks
				longevity: 25,
				// always propagate
				propagate: true,
			})
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

impl<T: Config> IsmpModule for Pallet<T> {
	fn on_accept(
		&self,
		PostRequest { body: data, from, source, .. }: PostRequest,
	) -> Result<(), ismp::error::Error> {
		let RegistrarParams { address, .. } = TokenRegistrarParams::<T>::get(&source)
			.ok_or_else(|| ismp::error::Error::Custom(format!("Pallet is not initialized")))?;
		if from != address.as_bytes().to_vec() {
			Err(ismp::error::Error::Custom(format!("Unauthorized action")))?
		}
		let body = SolRequestBody::abi_decode(&data[..], true)
			.map_err(|err| ismp::error::Error::Custom(format!("Decode error: {err}")))?;
		let asset_id: H256 = body.assetId.0.into();
		let owner: H160 = body.owner.0 .0.into();

		// asset must not already exist
		if AssetOwners::<T>::contains_key(&asset_id) || PendingAsset::<T>::contains_key(&asset_id) {
			Err(ismp::error::Error::Custom(format!("Asset already exists")))?
		}

		PendingAsset::<T>::insert(asset_id, owner);

		Self::deposit_event(Event::<T>::NewPendingAsset { asset_id, owner });

		Ok(())
	}

	fn on_response(&self, _response: Response) -> Result<(), ismp::error::Error> {
		Err(ismp::error::Error::Custom(format!("Module does not expect responses")))
	}

	fn on_timeout(&self, _request: Timeout) -> Result<(), ismp::error::Error> {
		// The request lives forever, it's not exactly time-sensitive.
		// There are no refunds for asset registration fees
		Err(ismp::error::Error::Custom(format!("Module does not expect timeouts")))
	}
}

/// Static weights because benchmarks suck, and we'll be getting PolkaVM soon anyways
fn weight() -> Weight {
	Weight::from_parts(300_000_000, 0)
}
