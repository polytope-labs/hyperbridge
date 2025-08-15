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
use anyhow::anyhow;
use frame_support::pallet_prelude::Weight;
use ismp::router::{PostRequest, Response, Timeout};
use polkadot_sdk::*;

pub use types::*;

use alloc::{format, vec};
use codec::Encode;
use ismp::module::IsmpModule;
use primitive_types::{H160, H256};
use token_gateway_primitives::{RemoteERC6160AssetRegistration, PALLET_TOKEN_GATEWAY_ID};

pub use token_gateway_primitives::TOKEN_GOVERNOR_ID as PALLET_ID;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {

	use super::*;
	use alloc::collections::{BTreeMap, BTreeSet};
	use frame_support::{
		pallet_prelude::*,
		traits::{fungible::Mutate, tokens::Preservation},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use ismp::{dispatcher::IsmpDispatcher, host::StateMachine};
	use sp_runtime::traits::AccountIdConversion;
	use token_gateway_primitives::AssetMetadata;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The pallet's configuration trait.
	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config
		+ pallet_ismp::Config
		+ pallet_ismp_host_executive::Config
	{
		/// The [`IsmpDispatcher`] for dispatching cross-chain requests
		type Dispatcher: IsmpDispatcher<Account = Self::AccountId, Balance = Self::Balance>;

		/// The account id for the treasury
		type TreasuryAccount: Get<PalletId>;

		/// Origin for privileged actions
		type GovernorOrigin: EnsureOrigin<Self::RuntimeOrigin>;
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

	/// TokenGateway protocol parameters per chain
	#[pallet::storage]
	pub type TokenGatewayParams<T: Config> =
		StorageMap<_, Twox64Concat, StateMachine, GatewayParams<H160>, OptionQuery>;

	/// IntentGateway protocol parameters per chain
	#[pallet::storage]
	pub type IntentGatewayParams<T: Config> =
		StorageMap<_, Twox64Concat, StateMachine, GatewayParams<H256>, OptionQuery>;

	/// Native asset ids for standalone chains connected to token gateway.
	#[pallet::storage]
	pub type StandaloneChainAssets<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, StateMachine, Twox64Concat, H256, bool, OptionQuery>;

	/// Pallet events that functions in this pallet can emit.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new asset has been registered
		AssetRegistered {
			/// The asset identifier
			asset_id: H256,
			/// Request commitment
			commitment: H256,
			/// Destination chain
			dest: StateMachine,
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
		/// The IntentGateway params have been updated for a state machine
		IntentGatewayParamsUpdated {
			/// The old params
			old: GatewayParams<H256>,
			/// The new params
			new: GatewayParams<H256>,
			/// The state machine it was updated for
			state_machine: StateMachine,
		},
		/// The TokenGateway params have been updated for a state machine
		TokenGatewayParamsUpdated {
			/// The old params
			old: GatewayParams<H160>,
			/// The new params
			new: GatewayParams<H160>,
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
		/// Native asset IDs have been deregistered
		NativeAssetsDeregistered { assets: BTreeMap<StateMachine, BTreeSet<H256>> },
		/// Native asset IDs have been registered
		NativeAssetsRegistered { assets: BTreeMap<StateMachine, BTreeSet<H256>> },
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
		/// Creates a multi-chain ERC6160 asset.
		///
		/// This works by dispatching a governance request to the TokenGateway contract on each
		/// requested chain to create the token contract for the asset
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
			T::GovernorOrigin::ensure_origin(origin)?;

			Self::update_registrar_params_impl(update)?;

			Ok(())
		}

		/// Records the update of the TokenGateway contract parameters and
		/// dispatches a request to update the TokenGateway contract parameters
		#[pallet::call_index(3)]
		#[pallet::weight(weight())]
		pub fn update_token_gateway_params(
			origin: OriginFor<T>,
			update: BTreeMap<StateMachine, GatewayParamsUpdate<H160>>,
		) -> DispatchResult {
			T::GovernorOrigin::ensure_origin(origin)?;

			Self::update_token_gateway_params_impl(update)?;

			Ok(())
		}

		/// Updates the TokenGovernor pallet parameters.
		#[pallet::call_index(4)]
		#[pallet::weight(weight())]
		pub fn update_params(
			origin: OriginFor<T>,
			update: ParamsUpdate<<T as pallet_ismp::Config>::Balance>,
		) -> DispatchResult {
			T::GovernorOrigin::ensure_origin(origin)?;
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

		/// Dispatches a governance request to the relevant TokenGateway contracts to map the
		/// provided token contract to an asset id
		#[pallet::call_index(6)]
		#[pallet::weight(weight())]
		pub fn create_asset_mapping(
			origin: OriginFor<T>,
			asset: ERC20AssetRegistration,
		) -> DispatchResult {
			T::GovernorOrigin::ensure_origin(origin)?;

			Self::create_erc20_asset_impl(asset)?;

			Ok(())
		}

		/// Adds a new token gateway contract instance to all existing instances
		#[pallet::call_index(7)]
		#[pallet::weight(weight())]
		pub fn new_token_gateway_instance(
			origin: OriginFor<T>,
			updates: BTreeMap<StateMachine, GatewayParamsUpdate<H160>>,
		) -> DispatchResult {
			T::GovernorOrigin::ensure_origin(origin)?;

			Self::new_token_gateway_instance_impl(updates)?;

			Ok(())
		}

		/// Deregister the native token asset ids for standalone chains
		#[pallet::call_index(8)]
		#[pallet::weight(weight())]
		pub fn deregister_standalone_chain_native_assets(
			origin: OriginFor<T>,
			assets: BTreeMap<StateMachine, BTreeSet<H256>>,
		) -> DispatchResult {
			T::GovernorOrigin::ensure_origin(origin)?;

			for (state_machine, new_asset_ids) in assets.clone() {
				new_asset_ids
					.into_iter()
					.for_each(|id| StandaloneChainAssets::<T>::remove(state_machine, id))
			}

			Self::deposit_event(Event::<T>::NativeAssetsDeregistered { assets });

			Ok(())
		}

		/// Register the native token asset ids for standalone chains
		#[pallet::call_index(9)]
		#[pallet::weight(weight())]
		pub fn register_standalone_chain_native_assets(
			origin: OriginFor<T>,
			assets: BTreeMap<StateMachine, BTreeSet<H256>>,
		) -> DispatchResult {
			T::GovernorOrigin::ensure_origin(origin)?;

			for (state_machine, new_asset_ids) in assets.clone() {
				new_asset_ids
					.into_iter()
					.for_each(|id| StandaloneChainAssets::<T>::insert(state_machine, id, true))
			}

			Self::deposit_event(Event::<T>::NativeAssetsRegistered { assets });

			Ok(())
		}

		/// Records the update of the IntentGateway contract parameters and
		/// dispatches a request to update the IntentGateway contract parameters
		#[pallet::call_index(10)]
		#[pallet::weight(weight())]
		pub fn update_intent_gateway_params(
			origin: OriginFor<T>,
			params: BTreeMap<StateMachine, GatewayParamsUpdate<H256>>,
		) -> DispatchResult {
			T::GovernorOrigin::ensure_origin(origin)?;

			Self::update_intent_gateway_params_impl(params)?;

			Ok(())
		}

		/// Adds a new intent gateway contract instance to all existing instances
		#[pallet::call_index(11)]
		#[pallet::weight(weight())]
		pub fn new_intent_gateway_instance(
			origin: OriginFor<T>,
			params: BTreeMap<StateMachine, NewIntentGatewayDeployment>,
		) -> DispatchResult {
			T::GovernorOrigin::ensure_origin(origin)?;

			Self::new_intent_gateway_instance_impl(params)?;

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

impl<T: Config> IsmpModule for Pallet<T>
where
	T::AccountId: From<[u8; 32]>,
{
	fn on_accept(
		&self,
		PostRequest { body: data, from, source, .. }: PostRequest,
	) -> Result<Weight, anyhow::Error> {
		// Only substrate chains are allowed to fully register assets remotely
		if source.is_substrate() && &from == &PALLET_TOKEN_GATEWAY_ID[..] {
			let remote_reg: RemoteERC6160AssetRegistration = codec::Decode::decode(&mut &*data)
				.map_err(|_| ismp::error::Error::Custom(format!("Failed to decode data")))?;
			match remote_reg {
				RemoteERC6160AssetRegistration::CreateAsset(asset) => {
					let asset_id: H256 = sp_io::hashing::keccak_256(asset.symbol.as_ref()).into();
					Pallet::<T>::register_asset(
						asset.into(),
						sp_io::hashing::keccak_256(&source.encode()).into(),
					)
					.map_err(|e| {
						ismp::error::Error::Custom(format!("Failed create asset {e:?}"))
					})?;
					StandaloneChainAssets::<T>::insert(source, asset_id, true);
				},
				RemoteERC6160AssetRegistration::UpdateAsset(asset) => {
					Pallet::<T>::update_erc6160_asset_impl(asset.into()).map_err(|e| {
						ismp::error::Error::Custom(format!("Failed create asset {e:?}"))
					})?;
				},
			}

			return Ok(weight());
		}
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

		Ok(weight())
	}

	fn on_response(&self, _response: Response) -> Result<Weight, anyhow::Error> {
		Err(anyhow!("Module does not expect responses"))
	}

	fn on_timeout(&self, _request: Timeout) -> Result<Weight, anyhow::Error> {
		// The request lives forever, it's not exactly time-sensitive.
		// There are no refunds for asset registration fees
		Err(anyhow!("Module does not expect timeouts"))
	}
}

/// Static weights because benchmarks suck, and we'll be getting PolkaVM soon anyways
fn weight() -> Weight {
	Weight::from_parts(300_000_000, 0)
}
