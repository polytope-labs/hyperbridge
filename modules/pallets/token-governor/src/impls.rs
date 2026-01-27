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

use alloc::{collections::BTreeMap, vec};
use alloy_sol_types::SolValue;
use frame_support::{ensure, PalletId};
use frame_system::{pallet_prelude::OriginFor, RawOrigin};
use ismp::{
	dispatcher::{DispatchPost, DispatchRequest, FeeMetadata, IsmpDispatcher},
	host::StateMachine,
};
use polkadot_sdk::*;
use sp_core::{H160, H256};
use sp_runtime::traits::AccountIdConversion;

use crate::{
	AssetMetadatas, AssetOwners, AssetRegistration, ChainWithSupply, Config, ContractInstance,
	ERC20AssetRegistration, ERC6160AssetRegistration, ERC6160AssetUpdate, Error, Event,
	GatewayParams, GatewayParamsUpdate, IntentGatewayParams, NewIntentGatewayDeployment, Pallet,
	PendingAsset, RegistrarParamsUpdate, SolAssetMetadata, SolChangeAssetAdmin,
	SolContractInstance, SolDeregsiterAsset, SolGatewayParams, SolNewIntentGatewayDeployment,
	SolRegistrarParams, SupportedChains, TokenGatewayParams, TokenGatewayRequest,
	TokenRegistrarParams, UnsignedERC6160AssetRegistration, PALLET_ID,
};

use token_gateway_primitives::AssetMetadata;

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

		let metadata = AssetMetadata { name: asset.name.clone(), symbol: asset.symbol.clone() };

		for ChainWithSupply { chain, supply } in asset.chains.clone() {
			let address = if chain.is_substrate() {
				continue;
			} else {
				let GatewayParams { address, .. } = TokenGatewayParams::<T>::get(&chain)
					.ok_or_else(|| Error::<T>::UnknownTokenGateway)?;
				address
			};

			let dispatcher = T::Dispatcher::default();
			let commitment = dispatcher
				.dispatch_request(
					DispatchRequest::Post(DispatchPost {
						dest: chain.clone(),
						from: PALLET_ID.to_vec(),
						to: address.as_bytes().to_vec(),
						timeout: 0,
						body: {
							let mut body: SolAssetMetadata =
								metadata.clone().try_into().map_err(|_| Error::<T>::InvalidUtf8)?;

							if let Some(supply) = supply {
								body.beneficiary = supply.beneficiary.0.into();
								body.initialSupply =
									alloy_primitives::U256::from_limbs(supply.initial_supply.0);
							}
							body.encode_request()
						},
					}),
					FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
				)
				.map_err(|_| Error::<T>::DispatchFailed)?;
			// tracks which chains the asset is deployed on
			SupportedChains::<T>::insert(asset_id, chain, true);
			Self::deposit_event(Event::<T>::AssetRegistered { asset_id, commitment, dest: chain });
		}

		AssetMetadatas::<T>::insert(asset_id, metadata);
		AssetOwners::<T>::insert(asset_id, who);

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
	///
	/// 1. Dispatch a request to add the asset to any new chains
	/// 2. Dispatch a request to delist the asset from the TokenGateway contract on any previously
	///    supported chain (Should be used with caution)
	/// 3. Dispatch a request to change the asset admin to another address.
	pub fn update_erc6160_asset_impl(update: ERC6160AssetUpdate) -> Result<(), Error<T>> {
		let metadata =
			AssetMetadatas::<T>::get(&update.asset_id).ok_or_else(|| Error::<T>::UnknownAsset)?;

		let dispatcher = T::Dispatcher::default();

		for ChainWithSupply { chain, supply } in update.add_chains {
			// skip if it already was dispatched to the provided chain
			if SupportedChains::<T>::get(&update.asset_id, &chain).is_some() || chain.is_substrate()
			{
				continue;
			}

			let GatewayParams { address, .. } = TokenGatewayParams::<T>::get(&chain)
				.ok_or_else(|| Error::<T>::UnknownTokenGateway)?;

			dispatcher
				.dispatch_request(
					DispatchRequest::Post(DispatchPost {
						dest: chain.clone(),
						from: PALLET_ID.to_vec(),
						to: address.as_bytes().to_vec(),
						timeout: 0,
						body: {
							let mut body: SolAssetMetadata =
								metadata.clone().try_into().map_err(|_| Error::<T>::InvalidUtf8)?;

							if let Some(supply) = supply {
								body.beneficiary = supply.beneficiary.0.into();
								body.initialSupply =
									alloy_primitives::U256::from_limbs(supply.initial_supply.0);
							}
							body.encode_request()
						},
					}),
					FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
				)
				.map_err(|_| Error::<T>::DispatchFailed)?;
			// tracks which chains the asset is deployed on
			SupportedChains::<T>::insert(update.asset_id, chain, true);
		}

		for chain in update.remove_chains {
			// skip if it already was dispatched to the provided chain
			if SupportedChains::<T>::get(&update.asset_id, &chain).is_none() || chain.is_substrate()
			{
				continue;
			}

			let GatewayParams { address, .. } = TokenGatewayParams::<T>::get(&chain)
				.ok_or_else(|| Error::<T>::UnknownTokenGateway)?;

			dispatcher
				.dispatch_request(
					DispatchRequest::Post(DispatchPost {
						dest: chain.clone(),
						from: PALLET_ID.to_vec(),
						to: address.as_bytes().to_vec(),
						timeout: 0,
						body: {
							let body =
								SolDeregsiterAsset { assetIds: vec![update.asset_id.0.into()] };
							body.encode_request()
						},
					}),
					FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
				)
				.map_err(|_| Error::<T>::DispatchFailed)?;

			// skip if it already was dispatched to the provided chain
			SupportedChains::<T>::remove(&update.asset_id, &chain)
		}

		for (chain, admin) in update.new_admins {
			// skip if it doesn't exist on the provided chain
			if SupportedChains::<T>::get(&update.asset_id, &chain).is_none() || chain.is_substrate()
			{
				continue;
			}

			let GatewayParams { address, .. } = TokenGatewayParams::<T>::get(&chain)
				.ok_or_else(|| Error::<T>::UnknownTokenGateway)?;

			let body =
				SolChangeAssetAdmin { assetId: update.asset_id.0.into(), newAdmin: admin.0.into() };
			dispatcher
				.dispatch_request(
					DispatchRequest::Post(DispatchPost {
						dest: chain.clone(),
						from: PALLET_ID.to_vec(),
						to: address.as_bytes().to_vec(),
						timeout: 0,
						body: body.encode_request(),
					}),
					FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
				)
				.map_err(|_| Error::<T>::DispatchFailed)?;
		}

		Ok(())
	}

	/// Dispatches a request to update the TokenGateway contract parameters
	pub fn update_token_gateway_params_impl(
		updates: BTreeMap<StateMachine, GatewayParamsUpdate<H160>>,
	) -> Result<(), Error<T>> {
		for (state_machine, update) in updates {
			let stored_params = TokenGatewayParams::<T>::get(&state_machine);
			let old_params = stored_params.clone().unwrap_or_default();
			let new_params = old_params.update::<T>(&state_machine, update);

			TokenGatewayParams::<T>::insert(state_machine.clone(), new_params.clone());

			// if the params already exists then we dispatch a request to update it
			if let Some(old) = stored_params {
				let dispatcher = T::Dispatcher::default();
				let body: SolGatewayParams = new_params.clone().into();
				dispatcher
					.dispatch_request(
						DispatchRequest::Post(DispatchPost {
							dest: state_machine.clone(),
							from: PALLET_ID.to_vec(),
							to: old.address.as_bytes().to_vec(),
							timeout: 0,
							body: body.encode_request(),
						}),
						FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
					)
					.map_err(|_| Error::<T>::DispatchFailed)?;
			}

			Self::deposit_event(Event::<T>::TokenGatewayParamsUpdated {
				old: old_params,
				new: new_params,
				state_machine,
			});
		}

		Ok(())
	}

	/// Introduce a new instance of the token gateway which has a different address
	pub fn new_token_gateway_instance_impl(
		updates: BTreeMap<StateMachine, GatewayParamsUpdate<H160>>,
	) -> Result<(), Error<T>> {
		// first set them all
		for (state_machine, update) in updates.iter() {
			let new_params = GatewayParams::default().update::<T>(&state_machine, update.clone());
			TokenGatewayParams::<T>::insert(state_machine.clone(), new_params);
		}

		// now dispatch cross-chain governance actions
		let dispatcher = T::Dispatcher::default();
		for (state_machine, _) in updates {
			let GatewayParams { address, .. } = TokenGatewayParams::<T>::get(&state_machine)
				.expect("Params set in previous loop; qed");
			let body: SolContractInstance =
				ContractInstance { chain: state_machine, module_id: address }.into();

			for (chain, GatewayParams { address, .. }) in TokenGatewayParams::<T>::iter() {
				if chain == state_machine || chain.is_substrate() {
					continue;
				}
				dispatcher
					.dispatch_request(
						DispatchRequest::Post(DispatchPost {
							dest: chain.clone(),
							from: PALLET_ID.to_vec(),
							to: address.as_bytes().to_vec(),
							timeout: 0,
							body: body.encode_request(),
						}),
						FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
					)
					.map_err(|_| Error::<T>::DispatchFailed)?;
			}
		}

		Ok(())
	}

	/// Dispatch a request to update the params of the TokenRegistrar
	pub fn update_registrar_params_impl(
		updates: BTreeMap<StateMachine, RegistrarParamsUpdate>,
	) -> Result<(), Error<T>> {
		for (state_machine, update) in updates {
			let stored_params = TokenRegistrarParams::<T>::get(&state_machine);
			let old_params = stored_params.clone().unwrap_or_default();
			let new_params = old_params.update::<T>(&state_machine, update);

			TokenRegistrarParams::<T>::insert(state_machine.clone(), new_params.clone());

			// if the params already exists then we dispatch a request to update it
			if let Some(old) = stored_params {
				let dispatcher = T::Dispatcher::default();
				dispatcher
					.dispatch_request(
						DispatchRequest::Post(DispatchPost {
							dest: state_machine.clone(),
							from: PALLET_ID.to_vec(),
							to: old.address.as_bytes().to_vec(),
							timeout: 0,
							body: SolRegistrarParams::from(new_params.clone()).abi_encode(),
						}),
						FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
					)
					.map_err(|_| Error::<T>::DispatchFailed)?;
			}

			Self::deposit_event(Event::<T>::RegistrarParamsUpdated {
				old: old_params,
				new: new_params,
				state_machine,
			});
		}

		Ok(())
	}

	/// Dispatches a request to create list an ERC20 asset on TokenGateway
	pub fn create_erc20_asset_impl(asset: ERC20AssetRegistration) -> Result<(), Error<T>> {
		let asset_id: H256 = sp_io::hashing::keccak_256(asset.symbol.as_ref()).into();

		let metadata = AssetMetadata { name: asset.name.clone(), symbol: asset.symbol.clone() };

		for AssetRegistration { chain, erc20, erc6160 } in asset.chains {
			if chain.is_substrate() {
				continue;
			}
			// Set the parent ERC20 asset's decimals value
			let mut body: SolAssetMetadata =
				metadata.clone().try_into().map_err(|_| Error::<T>::InvalidUtf8)?;

			if let Some(erc20) = erc20 {
				body.erc20 = erc20.0.into();
			}

			if let Some(erc6160) = erc6160 {
				body.erc6160 = erc6160.0.into();
			}

			let GatewayParams { address, .. } = TokenGatewayParams::<T>::get(&chain)
				.ok_or_else(|| Error::<T>::UnknownTokenGateway)?;

			let dispatcher = T::Dispatcher::default();
			let commitment = dispatcher
				.dispatch_request(
					DispatchRequest::Post(DispatchPost {
						dest: chain.clone(),
						from: PALLET_ID.to_vec(),
						to: address.as_bytes().to_vec(),
						timeout: 0,
						body: body.encode_request(),
					}),
					FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
				)
				.map_err(|_| Error::<T>::DispatchFailed)?;
			// tracks which chains the asset is deployed on
			SupportedChains::<T>::insert(asset_id, chain, true);
			Self::deposit_event(Event::<T>::AssetRegistered { asset_id, commitment, dest: chain });
		}

		AssetMetadatas::<T>::insert(asset_id, metadata);

		let who: T::AccountId = PalletId(PALLET_ID).into_account_truncating();
		AssetOwners::<T>::insert(asset_id, who);
		Ok(())
	}

	/// Register a new intent gateway instance.
	pub fn new_intent_gateway_instance_impl(
		updates: BTreeMap<StateMachine, NewIntentGatewayDeployment>,
	) -> Result<(), Error<T>> {
		// first set them all
		for (state_machine, update) in updates.iter() {
			let new_params = GatewayParams::default().update::<T>(
				&state_machine,
				GatewayParamsUpdate {
					address: Some(update.module_id.clone()),
					call_dispatcher: None,
				},
			);
			IntentGatewayParams::<T>::insert(state_machine.clone(), new_params);
		}

		// now dispatch cross-chain governance actions
		let dispatcher = T::Dispatcher::default();
		for (state_machine, new_deployment) in updates {
			let body: SolNewIntentGatewayDeployment = new_deployment.into();

			for (chain, GatewayParams { address, .. }) in IntentGatewayParams::<T>::iter() {
				if chain == state_machine || chain.is_substrate() {
					continue;
				}

				dispatcher
					.dispatch_request(
						DispatchRequest::Post(DispatchPost {
							dest: chain.clone(),
							from: PALLET_ID.to_vec(),
							// Contract address is the last 20 bytes
							to: address.0[12..].to_vec(),
							timeout: 0,
							body: vec![vec![1], SolNewIntentGatewayDeployment::abi_encode(&body)]
								.concat(),
						}),
						FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
					)
					.map_err(|_| Error::<T>::DispatchFailed)?;
			}
		}

		Ok(())
	}

	/// Updates the intent gateway parameters for the given state machine.
	pub fn update_intent_gateway_params_impl(
		params: BTreeMap<StateMachine, GatewayParamsUpdate<H256>>,
	) -> Result<(), Error<T>> {
		for (state_machine, update) in params {
			let stored_params = IntentGatewayParams::<T>::get(&state_machine);
			let old_params = stored_params.clone().unwrap_or_default();

			let new_params = old_params.update::<T>(&state_machine, update);

			IntentGatewayParams::<T>::insert(state_machine.clone(), new_params.clone());

			if !state_machine.is_evm() {
				continue;
			}

			// if the params already exists then we dispatch a request to update it
			if let Some(old) = stored_params {
				let dispatcher = T::Dispatcher::default();
				let body: SolGatewayParams = GatewayParams {
					host: new_params.host,
					call_dispatcher: new_params.call_dispatcher,
					..Default::default()
				}
				.into();
				dispatcher
					.dispatch_request(
						DispatchRequest::Post(DispatchPost {
							dest: state_machine.clone(),
							from: PALLET_ID.to_vec(),
							// Contract address is the last 20 bytes
							to: old.address.0[12..].to_vec(),
							timeout: 0,
							body: vec![vec![2], SolGatewayParams::abi_encode(&body)].concat(),
						}),
						FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
					)
					.map_err(|_| Error::<T>::DispatchFailed)?;
			}

			Self::deposit_event(Event::<T>::IntentGatewayParamsUpdated {
				old: old_params,
				new: new_params,
				state_machine,
			});
		}

		Ok(())
	}
}
