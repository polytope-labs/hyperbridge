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
use ismp::router::{Post, Response, Timeout};
pub use types::*;

use ismp::module::IsmpModule;
use primitive_types::{H160, H256};

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

/// The module id for this pallet
pub const PALLET_ID: [u8; 8] = *b"registry";

// All pallet logic is defined in its own module and must be annotated by the `pallet` attribute.
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		traits::{fungible::Mutate, tokens::Preservation},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use ismp::{
		dispatcher::{DispatchPost, DispatchRequest, FeeMetadata, IsmpDispatcher},
		host::StateMachine,
	};
	use sp_runtime::traits::AccountIdConversion;

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

		/// The account id for the treasury
		type TreasuryAccount: Get<PalletId>;
	}

	/// Maps a pending asset to it's owner. Enables asset registration without the native token by
	/// prviding a single-use execution ticket for asset creation through an unsigned transaction.
	#[pallet::storage]
	pub type PendingAsset<T: Config> = StorageMap<_, Identity, H256, H160, OptionQuery>;

	/// Mapping of AssetIds and supported chain to their metadata. Can only be updated by the asset
	/// owner
	#[pallet::storage]
	pub type Assets<T: Config> =
		StorageDoubleMap<_, Identity, H256, Twox64Concat, StateMachine, AssetMetadata, OptionQuery>;

	/// Mapping of AssetIds to their owners
	#[pallet::storage]
	pub type AssetOwners<T: Config> =
		StorageMap<_, Identity, H256, <T as frame_system::Config>::AccountId, OptionQuery>;

	/// TokenGovernor protocol parameters.
	#[pallet::storage]
	pub type ProtocolParams<T: Config> =
		StorageValue<_, Params<<T as pallet_ismp::Config>::Balance>, OptionQuery>;

	/// Pallet events that functions in this pallet can emit.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new asset has been registered
		AssetRegistered(ERC6160AssetRegistration),
		/// A new pending asset has been registered
		NewPendingAsset {
			/// The pending asset identifier
			asset_id: H256,
			/// Owner of the asset
			owner: H160,
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
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	{
		/// Registers a multi-chain ERC6160 asset. The asset should not already exist.
		///
		/// This works by dispatching a request to the TokenGateway module on each requested chain
		/// to create the asset.
		#[pallet::call_index(0)]
		#[pallet::weight(1_000_000_000)]
		pub fn register_erc6160_asset(
			origin: OriginFor<T>,
			asset: ERC6160AssetRegistration,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let asset_id: H256 = sp_io::hashing::keccak_256(asset.symbol.as_ref()).into();
			ensure!(!AssetOwners::<T>::contains_key(&asset_id), Error::<T>::AssetAlreadyExists);

			let Params { registration_fee, token_gateway_address, .. } =
				ProtocolParams::<T>::get().ok_or_else(|| Error::<T>::NotInitialized)?;
			T::Currency::transfer(
				&who,
				&T::TreasuryAccount::get().into_account_truncating(),
				registration_fee,
				Preservation::Preserve,
			)?;

			for ChainWithSupply { chain, supply } in asset.chains.clone() {
				// todo: hash bytecode with CREATE2 to get address
				let metadata = AssetMetadata {
					name: asset.name.clone(),
					symbol: asset.symbol.clone(),
					logo: asset.logo.clone(),
					..Default::default()
				};
				Assets::<T>::insert(asset_id, chain, metadata);

				let mut body = SolAssetMetadata {
					name: String::from_utf8(asset.name.as_slice().to_vec())
						.map_err(|_| Error::<T>::InvalidUtf8)?,
					symbol: String::from_utf8(asset.symbol.as_slice().to_vec())
						.map_err(|_| Error::<T>::InvalidUtf8)?,
					..Default::default()
				};

				if let Some(supply) = supply {
					body.beneficiary = supply.beneficiary.0.into();
					body.initialSupply =
						alloy_primitives::U256::from_limbs(supply.initial_supply.0);
				}

				let dispatcher = T::Dispatcher::default();
				dispatcher
					.dispatch_request(
						DispatchRequest::Post(DispatchPost {
							dest: chain,
							from: PALLET_ID.to_vec(),
							to: token_gateway_address.as_bytes().to_vec(),
							timeout: 0,
							body: body.encode(),
						}),
						FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
					)
					.map_err(|_| Error::<T>::DispatchFailed)?;
			}

			AssetOwners::<T>::insert(asset_id, who);

			Self::deposit_event(Event::<T>::AssetRegistered(asset));

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(1_000_000_000)]
		pub fn register_erc6160_asset_unsigned(
			origin: OriginFor<T>,
			asset: ERC6160AssetRegistration,
		) -> DispatchResult {
			ensure_none(origin)?;

			Ok(())
		}

		// todo: unsigned extrinsic

		// todo: updates to protocol params

		// todo: ERC20 asset registration

		// todo: updates to mult-chain asset
		// 1. token logo, erc6160 asset?
		// 2. supported chains
		// 3. changeAdmins
		// 4. deregister asset from TokenGateway
	}

	/// This allows users to create assets from any chain using the TokenGatewayRegistrar.
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

		fn validate_unsigned(
			_source: TransactionSource,
			_call: &Self::Call,
		) -> TransactionValidity {
			Ok(ValidTransaction {
				// they should all have the same priority so they can be rejected
				priority: 100,
				// they are all self-contained batches that have no dependencies
				requires: vec![],
				// provides this unique hash of transactions
				provides: vec![], // use asset_id here
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
	fn on_accept(&self, Post { data, from, .. }: Post) -> Result<(), ismp::error::Error> {
		let Params { token_registrar_address, .. } = ProtocolParams::<T>::get()
			.ok_or_else(|| ismp::error::Error::Custom(format!("Pallet is not initialized")))?;
		if from != token_registrar_address.as_bytes().to_vec() {
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
