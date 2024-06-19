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

// Pallet Implementations

use frame_support::{ensure, PalletId};
use frame_system::{pallet_prelude::OriginFor, RawOrigin};
use ismp::{
	dispatcher::{DispatchPost, DispatchRequest, FeeMetadata, IsmpDispatcher},
	host::StateMachine,
};
use sp_core::{H160, H256};
use sp_runtime::traits::AccountIdConversion;

use crate::{
	AssetFee, AssetFeeUpdate, AssetFees, AssetMetadata, AssetMetadatas, AssetOwners,
	ChainWithSupply, Config, ERC20AssetRegistration, ERC6160AssetRegistration, ERC6160AssetUpdate,
	Error, Event, Pallet, Params, PendingAsset, ProtocolParams, SolAssetFeeUpdate,
	SolAssetMetadata, SolChangeAssetAdmin, SolDeregsiterAsset, SolTokenGatewayParams,
	TokenGatewayParams, TokenGatewayParamsUpdate, TokenGatewayRequest,
	UnsignedERC6160AssetRegistration, PALLET_ID,
};

impl<T: Config> Pallet<T>
where
	T::AccountId: From<[u8; 32]>,
{
	/// Ensure the signer is the root account or asset owner
	pub fn ensure_root_or_owner(origin: OriginFor<T>, asset_id: H256) -> Result<(), Error<T>> {
		let raw_origin = origin.into().map_err(|_| Error::<T>::UnknownAsset)?;
		match raw_origin {
			RawOrigin::Signed(who) => {
				let owner =
					AssetOwners::<T>::get(&asset_id).ok_or_else(|| Error::<T>::UnknownAsset)?;

				ensure!(who == owner, Error::<T>::NotAssetOwner);
			},
			RawOrigin::Root => {},
			_ => Err(Error::<T>::UnknownAsset)?,
		};
		Ok(())
	}

	/// Registers the provided ERC6160 asset. Will check that the asset doesn't already exist
	pub fn register_asset(
		asset: ERC6160AssetRegistration,
		who: T::AccountId,
	) -> Result<(), Error<T>> {
		let asset_id: H256 = sp_io::hashing::keccak_256(asset.symbol.as_ref()).into();
		if AssetOwners::<T>::contains_key(&asset_id) {
			Err(Error::<T>::AssetAlreadyExists)?
		}
		let Params { token_gateway_address, .. } =
			ProtocolParams::<T>::get().ok_or_else(|| Error::<T>::NotInitialized)?;

		let metadata = AssetMetadata {
			name: asset.name.clone(),
			symbol: asset.symbol.clone(),
			logo: asset.logo.clone(),
			..Default::default()
		};

		for ChainWithSupply { chain, supply } in asset.chains.clone() {
			let mut body: SolAssetMetadata =
				metadata.clone().try_into().map_err(|_| Error::<T>::InvalidUtf8)?;

			if let Some(supply) = supply {
				body.beneficiary = supply.beneficiary.0.into();
				body.initialSupply = alloy_primitives::U256::from_limbs(supply.initial_supply.0);
			}

			let dispatcher = T::Dispatcher::default();
			dispatcher
				.dispatch_request(
					DispatchRequest::Post(DispatchPost {
						dest: chain.clone(),
						from: PALLET_ID.to_vec(),
						to: token_gateway_address.as_bytes().to_vec(),
						timeout: 0,
						body: body.encode_request(),
					}),
					FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
				)
				.map_err(|_| Error::<T>::DispatchFailed)?;
			// tracks which chains the asset is deployed on
			AssetFees::<T>::insert(asset_id, chain, AssetFee::default());
		}

		AssetMetadatas::<T>::insert(asset_id, metadata);
		AssetOwners::<T>::insert(asset_id, who);
		Self::deposit_event(Event::<T>::AssetRegistered { asset_id });

		Ok(())
	}

	/// Registers an asset that was paid for through the token registrar. The pallet must have
	/// previously received the asset to be created as a request from a TokenRegistrar otherwise
	/// this will fail
	pub fn register_asset_unsigned(
		registration: UnsignedERC6160AssetRegistration<T::AccountId>,
	) -> Result<(), Error<T>> {
		let UnsignedERC6160AssetRegistration { asset, signature, owner } = registration;
		let asset_id: H256 = sp_io::hashing::keccak_256(asset.symbol.as_ref()).into();

		let mut sig = [0u8; 65];
		sig.copy_from_slice(&signature);
		let pub_key = sp_io::crypto::secp256k1_ecdsa_recover(&sig, &asset_id.0)
			.map_err(|_| Error::<T>::InvalidSignature)?;
		let pub_key_hash = sp_io::hashing::keccak_256(&pub_key[..]);
		let address = H160::from_slice(&pub_key_hash[12..]);
		let asset_owner =
			PendingAsset::<T>::get(&asset_id).ok_or_else(|| Error::<T>::UnknownAsset)?;

		if address != asset_owner {
			Err(Error::<T>::NotAssetOwner)?;
		}

		Self::register_asset(asset, owner)?;

		PendingAsset::<T>::remove(&asset_id);

		Ok(())
	}

	/// This allows the asset owner to update their Multi-chain native asset.
	/// They are allowed to:
	/// 1. Change the logo
	/// 2. Dispatch a request to add the asset to any new chains
	/// 3. Dispatch a request to delist the asset from the TokenGateway contract on any previously
	///    supported chain (Should be used with caution)
	/// 4. Dispatch a request to change the asset admin to another address.
	pub fn update_erc6160_asset_impl(update: ERC6160AssetUpdate) -> Result<(), Error<T>> {
		let Params { token_gateway_address, .. } =
			ProtocolParams::<T>::get().ok_or_else(|| Error::<T>::NotInitialized)?;

		let metadata =
			AssetMetadatas::<T>::get(&update.asset_id).ok_or_else(|| Error::<T>::UnknownAsset)?;

		if let Some(logo) = update.logo {
			AssetMetadatas::<T>::mutate(&update.asset_id, |metadata| {
				metadata.as_mut().expect("Existence already checked; qed").logo = logo;
			});
		}

		let dispatcher = T::Dispatcher::default();

		for ChainWithSupply { chain, supply } in update.add_chains {
			// skip if it already was dispatched to the provided chain
			if AssetFees::<T>::get(&update.asset_id, &chain).is_some() {
				continue;
			}
			let mut body: SolAssetMetadata =
				metadata.clone().try_into().map_err(|_| Error::<T>::InvalidUtf8)?;

			if let Some(supply) = supply {
				body.beneficiary = supply.beneficiary.0.into();
				body.initialSupply = alloy_primitives::U256::from_limbs(supply.initial_supply.0);
			}

			dispatcher
				.dispatch_request(
					DispatchRequest::Post(DispatchPost {
						dest: chain.clone(),
						from: PALLET_ID.to_vec(),
						to: token_gateway_address.as_bytes().to_vec(),
						timeout: 0,
						body: body.encode_request(),
					}),
					FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
				)
				.map_err(|_| Error::<T>::DispatchFailed)?;
			// tracks which chains the asset is deployed on
			AssetFees::<T>::insert(update.asset_id, chain, AssetFee::default());
		}

		for chain in update.remove_chains {
			// skip if it already was dispatched to the provided chain
			if AssetFees::<T>::get(&update.asset_id, &chain).is_none() {
				continue;
			}

			let body = SolDeregsiterAsset { assetIds: vec![update.asset_id.0.into()] };
			dispatcher
				.dispatch_request(
					DispatchRequest::Post(DispatchPost {
						dest: chain.clone(),
						from: PALLET_ID.to_vec(),
						to: token_gateway_address.as_bytes().to_vec(),
						timeout: 0,
						body: body.encode_request(),
					}),
					FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
				)
				.map_err(|_| Error::<T>::DispatchFailed)?;
		}

		for (chain, admin) in update.new_admins {
			// skip if it doesn't exist on the provided chain
			if AssetFees::<T>::get(&update.asset_id, &chain).is_none() {
				continue;
			}

			let body =
				SolChangeAssetAdmin { assetId: update.asset_id.0.into(), newAdmin: admin.0.into() };
			dispatcher
				.dispatch_request(
					DispatchRequest::Post(DispatchPost {
						dest: chain.clone(),
						from: PALLET_ID.to_vec(),
						to: token_gateway_address.as_bytes().to_vec(),
						timeout: 0,
						body: body.encode_request(),
					}),
					FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
				)
				.map_err(|_| Error::<T>::DispatchFailed)?;
		}

		Ok(())
	}

	/// Dispatches a request to update the TokenRegistrar contract parameters
	pub fn update_gateway_params_impl(
		update: TokenGatewayParamsUpdate,
		state_machine: StateMachine,
	) -> Result<(), Error<T>> {
		let stored_params = TokenGatewayParams::<T>::get(&state_machine);
		let old_params = stored_params.clone().unwrap_or_default();
		let new_params = old_params.update(update);

		TokenGatewayParams::<T>::insert(state_machine.clone(), new_params.clone());

		// if the params already exists then we dispatch a request to update it
		if let Some(_) = stored_params {
			let Params { token_gateway_address, .. } =
				ProtocolParams::<T>::get().ok_or_else(|| Error::<T>::NotInitialized)?;
			let dispatcher = T::Dispatcher::default();
			let body: SolTokenGatewayParams = new_params.clone().into();
			dispatcher
				.dispatch_request(
					DispatchRequest::Post(DispatchPost {
						dest: state_machine.clone(),
						from: PALLET_ID.to_vec(),
						to: token_gateway_address.as_bytes().to_vec(),
						timeout: 0,
						body: body.encode_request(),
					}),
					FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
				)
				.map_err(|_| Error::<T>::DispatchFailed)?;
		}

		Self::deposit_event(Event::<T>::GatewayParamsUpdated {
			old: old_params,
			new: new_params,
			state_machine,
		});

		Ok(())
	}

	/// Dispatches a request to update the Asset fees on the provided chain
	pub fn update_asset_fees_impl(
		update: AssetFeeUpdate,
		state_machine: StateMachine,
	) -> Result<(), Error<T>> {
		let Params { token_gateway_address, .. } =
			ProtocolParams::<T>::get().ok_or_else(|| Error::<T>::NotInitialized)?;
		let fees = AssetFees::<T>::get(&update.asset_id, &state_machine)
			.ok_or_else(|| Error::<T>::UnknownAsset)?;

		let updated = fees.update(update.fee_update);
		let body = SolAssetFeeUpdate { assetId: update.asset_id.0.into(), fees: updated.into() };
		let dispatcher = T::Dispatcher::default();
		dispatcher
			.dispatch_request(
				DispatchRequest::Post(DispatchPost {
					dest: state_machine,
					from: PALLET_ID.to_vec(),
					to: token_gateway_address.as_bytes().to_vec(),
					timeout: 0,
					body: body.encode_request(),
				}),
				FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
			)
			.map_err(|_| Error::<T>::DispatchFailed)?;

		Ok(())
	}

	/// Dispatches a request to create list an ERC20 asset on TokenGateway
	pub fn create_erc20_asset_impl(asset: ERC20AssetRegistration) -> Result<(), Error<T>> {
		let asset_id: H256 = sp_io::hashing::keccak_256(asset.symbol.as_ref()).into();
		if AssetOwners::<T>::contains_key(&asset_id) {
			Err(Error::<T>::AssetAlreadyExists)?
		}
		let Params { token_gateway_address, .. } =
			ProtocolParams::<T>::get().ok_or_else(|| Error::<T>::NotInitialized)?;

		let metadata = AssetMetadata {
			name: asset.name.clone(),
			symbol: asset.symbol.clone(),
			logo: asset.logo.clone(),
			..Default::default()
		};

		for (chain, erc20_address) in asset.chains {
			let mut body: SolAssetMetadata =
				metadata.clone().try_into().map_err(|_| Error::<T>::InvalidUtf8)?;

			if let Some(erc20_address) = erc20_address {
				body.erc20 = erc20_address.0.into();
			}

			let dispatcher = T::Dispatcher::default();
			dispatcher
				.dispatch_request(
					DispatchRequest::Post(DispatchPost {
						dest: chain.clone(),
						from: PALLET_ID.to_vec(),
						to: token_gateway_address.as_bytes().to_vec(),
						timeout: 0,
						body: body.encode_request(),
					}),
					FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
				)
				.map_err(|_| Error::<T>::DispatchFailed)?;
			// tracks which chains the asset is deployed on
			AssetFees::<T>::insert(asset_id, chain, AssetFee::default());
		}

		AssetMetadatas::<T>::insert(asset_id, metadata);

		let who: T::AccountId = PalletId(PALLET_ID).into_account_truncating();
		AssetOwners::<T>::insert(asset_id, who);
		Self::deposit_event(Event::<T>::AssetRegistered { asset_id });
		Ok(())
	}
}
