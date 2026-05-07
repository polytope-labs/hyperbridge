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

//! ISMP module implementation for the hyper-fungible-token pallet

use crate::{
	error::HftError, impls::convert_to_balance, pallet::*, types::SubstrateCalldata, Config,
	Message, Pallet, ETHEREUM_MESSAGE_PREFIX,
};
use alloc::{format, vec};
use alloy_sol_types::SolValue;
use codec::{Decode, Encode};
use frame_support::traits::{
	fungibles::{self, Mutate},
	tokens::Preservation,
	Currency, ExistenceRequirement,
};
use frame_system::RawOrigin;
use ismp::{
	module::IsmpModule,
	router::{PostRequest, Request, Response, Timeout},
};
use polkadot_sdk::*;
use sp_core::{Get, H160, U256};
use sp_runtime::{MultiSignature, Weight};

use crate::types::EvmToSubstrate;

impl<T: Config> IsmpModule for Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	<<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance: core::str::FromStr,
	<<<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance as core::str::FromStr>::Err:
		core::error::Error + Send + Sync + 'static,
	<<T as Config>::Assets as fungibles::Inspect<T::AccountId>>::Balance:
		From<<<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance>,
{
	fn on_accept(
		&self,
		PostRequest { body, from, source, .. }: PostRequest,
	) -> Result<Weight, anyhow::Error> {
		// Authenticate: look up which local asset this contract address maps to
		let local_asset_id = ContractToAsset::<T>::get(source, &from)
			.ok_or(HftError::UnknownSourceContract(source))?;

		// Decode the Message
		let message = Message::abi_decode(&body)
			.map_err(HftError::DecodeError)?;

		// Convert recipient bytes to substrate AccountId
		// If 32 bytes: use directly. If 20 bytes: left-pad with zeros.
		let mut beneficiary_bytes = [0u8; 32];
		let to_bytes = message.to.as_ref();
		if to_bytes.len() == 32 {
			beneficiary_bytes.copy_from_slice(to_bytes);
		} else if to_bytes.len() == 20 {
			beneficiary_bytes[12..].copy_from_slice(to_bytes);
		} else {
			Err(HftError::InvalidRecipientLength(to_bytes.len()))?;
		}
		let beneficiary: T::AccountId = beneficiary_bytes.into();

		// Convert amount from ERC20 denomination to local
		let decimals = if local_asset_id == T::NativeAssetId::get() {
			T::Decimals::get()
		} else {
			<T::Assets as fungibles::metadata::Inspect<T::AccountId>>::decimals(
				local_asset_id.clone(),
			)
		};
		let erc_decimals = Precisions::<T>::get(local_asset_id.clone(), source)
			.ok_or(HftError::DecimalsNotConfigured(source))?;
		let amount = convert_to_balance::<<<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance>(
			U256::from_big_endian(&message.amount.to_be_bytes::<32>()),
			erc_decimals,
			decimals,
		)
		.map_err(|e| HftError::InvalidAmountConversion(format!("{e:?}")))?;

		// Mint or transfer to beneficiary
		if local_asset_id == T::NativeAssetId::get() {
			<T as Config>::NativeCurrency::transfer(
				&Pallet::<T>::pallet_account(),
				&beneficiary,
				amount,
				ExistenceRequirement::AllowDeath,
			)
			.map_err(|e| HftError::TransferFailed(e.into()))?;
		} else {
			let is_native = NativeAssets::<T>::get(local_asset_id.clone());
			if is_native {
				<T as Config>::Assets::transfer(
					local_asset_id,
					&Pallet::<T>::pallet_account(),
					&beneficiary,
					amount.into(),
					Preservation::Expendable,
				)
				.map_err(|e| HftError::TransferFailed(e.into()))?;
			} else {
				<T as Config>::Assets::mint_into(local_asset_id, &beneficiary, amount.into())
					.map_err(|e| HftError::MintFailed(e.into()))?;
			}
		}

		// Execute optional calldata
		if !message.data.is_empty() {
			let substrate_data = SubstrateCalldata::decode(&mut &message.data[..])
				.map_err(HftError::CalldataDecodeError)?;

			let origin = if let Some(signature) = substrate_data.signature {
				let multi_signature = MultiSignature::decode(&mut &*signature)
					.map_err(HftError::SignatureDecodeError)?;

				let nonce = frame_system::Pallet::<T>::account_nonce(beneficiary.clone());

				match multi_signature {
					MultiSignature::Ed25519(sig) => {
						let payload = (nonce, substrate_data.runtime_call.clone()).encode();
						let msg = sp_io::hashing::keccak_256(&payload);
						let pub_key = beneficiary_bytes.as_slice().try_into().map_err(|_| {
							HftError::SignatureVerificationFailed
						})?;
						if !sp_io::crypto::ed25519_verify(&sig, msg.as_ref(), &pub_key) {
							Err(HftError::SignatureVerificationFailed)?
						}
					},
					MultiSignature::Sr25519(sig) => {
						let payload = (nonce, substrate_data.runtime_call.clone()).encode();
						let msg = sp_io::hashing::keccak_256(&payload);
						let pub_key = beneficiary_bytes.as_slice().try_into().map_err(|_| {
							HftError::SignatureVerificationFailed
						})?;
						if !sp_io::crypto::sr25519_verify(&sig, msg.as_ref(), &pub_key) {
							Err(HftError::SignatureVerificationFailed)?
						}
					},
					MultiSignature::Ecdsa(sig) => {
						let payload = (nonce, substrate_data.runtime_call.clone()).encode();
						let preimage = vec![
							format!("{ETHEREUM_MESSAGE_PREFIX}{}", payload.len())
								.as_bytes()
								.to_vec(),
							payload,
						]
						.concat();
						let msg = sp_io::hashing::keccak_256(&preimage);
						let pub_key = sp_io::crypto::secp256k1_ecdsa_recover(&sig.0, &msg)
							.map_err(|_| HftError::EcdsaRecoveryFailed)?;
						let eth_address =
							H160::from_slice(&sp_io::hashing::keccak_256(&pub_key[..])[12..]);
						let substrate_account = T::EvmToSubstrate::convert(eth_address);
						if substrate_account != beneficiary {
							Err(HftError::SignatureVerificationFailed)?
						}
					},
					MultiSignature::Eth(_) =>
						Err(HftError::EthSignatureUnsupported)?,
				};

				beneficiary.clone()
			} else {
				let from_bytes = message.from.as_ref();
				if source.is_evm() {
					T::EvmToSubstrate::convert(H160::from_slice(
						&from_bytes[from_bytes.len() - 20..],
					))
				} else {
					let mut account = [0u8; 32];
					account.copy_from_slice(from_bytes);
					account.into()
				}
			};

			let runtime_call = T::RuntimeCall::decode(&mut &*substrate_data.runtime_call)
				.map_err(HftError::RuntimeCallDecodeError)?;
			use sp_runtime::traits::Dispatchable;
			runtime_call
				.dispatch(RawOrigin::Signed(origin.clone()).into())
				.map_err(|e| HftError::CallDispatchError(e.error))?;

			frame_system::Pallet::<T>::inc_account_nonce(origin);
		}

		Self::deposit_event(Event::<T>::TokenReceived {
			beneficiary,
			amount: amount.into(),
			source,
		});

		Ok(T::DbWeight::get().reads_writes(5, 2))
	}

	fn on_response(&self, _response: Response) -> Result<Weight, anyhow::Error> {
		Err(HftError::ResponsesNotSupported)?
	}

	fn on_timeout(&self, request: Timeout) -> Result<Weight, anyhow::Error> {
		match request {
			Timeout::Request(Request::Post(PostRequest {
				body, to, dest, ..
			})) => {
				let message = Message::abi_decode(&body)
					.map_err(HftError::DecodeError)?;

				// Refund the original sender
				let from_bytes = message.from.as_ref();
				let mut sender_bytes = [0u8; 32];
				if from_bytes.len() == 32 {
					sender_bytes.copy_from_slice(from_bytes);
				} else if from_bytes.len() == 20 {
					sender_bytes[12..].copy_from_slice(from_bytes);
				} else {
					Err(HftError::InvalidSenderLength(from_bytes.len()))?
				}
				let beneficiary: T::AccountId = sender_bytes.into();

				// Look up the asset from the destination contract address
				let local_asset_id = ContractToAsset::<T>::get(dest, &to)
					.ok_or(HftError::UnknownContractOnTimeout)?;

				let decimals = if local_asset_id == T::NativeAssetId::get() {
					T::Decimals::get()
				} else {
					<T::Assets as fungibles::metadata::Inspect<T::AccountId>>::decimals(
						local_asset_id.clone(),
					)
				};
				let erc_decimals = Precisions::<T>::get(local_asset_id.clone(), dest)
					.ok_or(HftError::DecimalsNotConfigured(dest))?;
				let amount = convert_to_balance::<<<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance>(
					U256::from_big_endian(&message.amount.to_be_bytes::<32>()),
					erc_decimals,
					decimals,
				)
				.map_err(|e| HftError::InvalidAmountConversion(format!("{e:?}")))?;

				// Refund: release escrowed tokens back to the original sender
				if local_asset_id == T::NativeAssetId::get() {
					<T as Config>::NativeCurrency::transfer(
						&Pallet::<T>::pallet_account(),
						&beneficiary,
						amount.into(),
						ExistenceRequirement::AllowDeath,
					)
					.map_err(|e| HftError::TransferFailed(e.into()))?;
				} else {
					let is_native = NativeAssets::<T>::get(local_asset_id.clone());
					if is_native {
						<T as Config>::Assets::transfer(
							local_asset_id,
							&Pallet::<T>::pallet_account(),
							&beneficiary,
							amount.into(),
							Preservation::Expendable,
						)
						.map_err(|e| HftError::TransferFailed(e.into()))?;
					} else {
						<T as Config>::Assets::mint_into(
							local_asset_id,
							&beneficiary,
							amount.into(),
						)
						.map_err(|e| HftError::MintFailed(e.into()))?;
					}
				}

				Pallet::<T>::deposit_event(Event::<T>::TokenRefunded {
					beneficiary,
					amount: amount.into(),
					dest,
				});
				Ok(T::DbWeight::get().reads_writes(5, 2))
			},
			Timeout::Request(Request::Get(_)) => Err(HftError::UnsupportedTimeoutType)?,
			Timeout::Response(_) => Err(HftError::UnsupportedTimeoutType)?,
		}
	}
}
