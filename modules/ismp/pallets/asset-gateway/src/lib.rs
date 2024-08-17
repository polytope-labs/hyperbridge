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

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{boxed::Box, string::ToString, vec};
use alloy_sol_types::SolType;
use core::marker::PhantomData;
use pallet_token_governor::TokenGatewayParams;

use frame_support::{
	ensure,
	traits::{
		fungibles::{self},
		Get,
	},
};

use ismp::{
	dispatcher::{DispatchPost, DispatchRequest, FeeMetadata, IsmpDispatcher},
	events::Meta,
	host::{IsmpHost, StateMachine},
	messaging::hash_request,
	module::IsmpModule,
	router::{Request, Timeout},
};
pub use pallet::*;
use sp_core::{H160, H256, U256};
use sp_runtime::{traits::AccountIdConversion, Permill};
use staging_xcm::{
	prelude::Assets,
	v4::{Asset, AssetId, Fungibility, Junction, Location, WeightLimit},
	VersionedAssets, VersionedLocation,
};
use xcm_utilities::MultiAccount;

pub mod xcm_utilities;

#[frame_support::pallet]
pub mod pallet {
	use alloc::vec;

	use super::*;
	use frame_support::{pallet_prelude::*, traits::fungibles, PalletId};
	use frame_system::pallet_prelude::OriginFor;
	use pallet_ismp::ModuleId;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// ISMP module identifier
	pub const PALLET_ID: ModuleId = ModuleId::Pallet(PalletId(*b"assetgtw"));

	/// The config trait
	#[pallet::config]
	pub trait Config:
		frame_system::Config
		+ pallet_ismp::Config
		+ pallet_xcm::Config
		+ pallet_token_governor::Config
	{
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The asset tranfer's pallet id, used for deriving its sovereign account ID.
		/// All escrowed funds will be custodied by this account
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Pallet parameters
		#[pallet::constant]
		type Params: Get<AssetGatewayParams>;

		/// The [`IsmpDispatcher`] implementation to use for dispatching requests
		type IsmpHost: IsmpHost + IsmpDispatcher<Account = Self::AccountId, Balance = Self::Balance>;

		/// Fungible asset implementation
		type Assets: fungibles::Mutate<Self::AccountId> + fungibles::Inspect<Self::AccountId>;
	}

	#[pallet::storage]
	#[pallet::getter(fn params)]
	pub type Params<T> = StorageValue<_, AssetGatewayParams, OptionQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// Error encountered while dispatching post request
		DispatchPostError,
		/// Pallet has not been initialized
		NotInitialized,
	}

	/// Events emiited by the relayer pallet
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An XCM transfer from the relay chain has been transformed into a crosschain message
		AssetTeleported {
			/// Source account on the relaychain
			from: T::AccountId,
			/// beneficiary account on destination
			to: H160,
			/// Amount transferred
			amount: <T::Assets as fungibles::Inspect<T::AccountId>>::Balance,
			/// Destination chain
			dest: StateMachine,
			/// Request commitment
			commitment: H256,
		},

		/// An asset has been received and transferred to the beneficiary's account on the
		/// relaychain
		AssetReceived {
			/// beneficiary account on relaychain
			beneficiary: T::AccountId,
			/// Amount transferred
			amount: <T::Assets as fungibles::Inspect<T::AccountId>>::Balance,
			/// Destination chain
			source: StateMachine,
		},

		/// An asset has been refunded and transferred to the beneficiary's account on the
		/// relaychain
		AssetRefunded {
			/// beneficiary account on relaychain
			beneficiary: T::AccountId,
			/// Amount transferred
			amount: <T::Assets as fungibles::Inspect<T::AccountId>>::Balance,
			/// Destination chain
			source: StateMachine,
		},
	}

	#[derive(Clone, Encode, Decode, scale_info::TypeInfo, Eq, PartialEq, RuntimeDebug)]
	pub struct AssetGatewayParams {
		/// Percentage to be taken as protocol fees
		pub protocol_fee_percentage: Permill,
	}

	impl AssetGatewayParams {
		pub const fn from_parts(protocol_fee_percentage: Permill) -> Self {
			Self { protocol_fee_percentage }
		}
	}

	#[derive(Clone, Encode, Decode, scale_info::TypeInfo, Eq, PartialEq, RuntimeDebug)]
	pub struct TokenGatewayParamsUpdate {
		pub protocol_fee_percentage: Option<Permill>,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: From<[u8; 32]>,
	{
		#[pallet::weight(T::DbWeight::get().writes(1))]
		#[pallet::call_index(0)]
		pub fn set_params(
			origin: OriginFor<T>,
			update: TokenGatewayParamsUpdate,
		) -> DispatchResult {
			<T as pallet_ismp::Config>::AdminOrigin::ensure_origin(origin)?;
			let mut current_params = Params::<T>::get().unwrap_or(T::Params::get());

			if let Some(protocol_fee_percentage) = update.protocol_fee_percentage {
				current_params.protocol_fee_percentage = protocol_fee_percentage;
			}

			Params::<T>::put(current_params);
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T>
where
	u128: From<<T::Assets as fungibles::Inspect<T::AccountId>>::Balance>,
	T::AccountId: Into<[u8; 32]>,
{
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	pub fn protocol_account_id() -> T::AccountId {
		T::TreasuryAccount::get().into_account_truncating()
	}

	pub fn token_gateway_address(state_machine: &StateMachine) -> H160 {
		TokenGatewayParams::<T>::get(state_machine)
			.map(|p| p.address)
			.unwrap_or_default()
	}

	pub fn protocol_fee_percentage() -> Permill {
		Params::<T>::get()
			.unwrap_or(<T as Config>::Params::get())
			.protocol_fee_percentage
	}

	pub fn dot_asset_id() -> H256 {
		sp_io::hashing::keccak_256(b"DOT").into()
	}

	/// Dispatch ismp request to token gateway on destination chain
	pub fn dispatch_request(
		multi_account: MultiAccount<T::AccountId>,
		amount: <T::Assets as fungibles::Inspect<T::AccountId>>::Balance,
	) -> Result<(), Error<T>> {
		let dispatcher = <T as Config>::IsmpHost::default();

		let mut to = [0u8; 32];
		to[12..].copy_from_slice(&multi_account.evm_account.0);
		let from: [u8; 32] = multi_account.substrate_account.clone().into();
		let asset_id = Self::dot_asset_id().0.into();
		let body = Body {
			amount: {
				let amount: u128 = amount.into();
				let mut bytes = [0u8; 32];
				convert_to_erc20(amount).to_big_endian(&mut bytes);
				alloy_primitives::U256::from_be_bytes(bytes)
			},
			max_fee: Default::default(),
			asset_id,
			redeem: false,
			from: from.into(),
			to: to.into(),
		};

		let token_gateway_address = Self::token_gateway_address(&multi_account.dest_state_machine);

		let dispatch_post = DispatchPost {
			dest: multi_account.dest_state_machine,
			from: token_gateway_address.0.to_vec(),
			to: token_gateway_address.0.to_vec(),
			timeout: multi_account.timeout,
			body: {
				// Prefix with the handleIncomingAsset enum variant
				let mut encoded = vec![0];
				encoded.extend_from_slice(&Body::abi_encode(&body));
				encoded
			},
		};

		let metadata =
			FeeMetadata { payer: multi_account.substrate_account.clone(), fee: Default::default() };
		let commitment = dispatcher
			.dispatch_request(DispatchRequest::Post(dispatch_post), metadata)
			.map_err(|_| Error::<T>::DispatchPostError)?;

		Self::deposit_event(Event::<T>::AssetTeleported {
			from: multi_account.substrate_account,
			to: multi_account.evm_account,
			dest: multi_account.dest_state_machine,
			amount,
			commitment,
		});

		Ok(())
	}
}

alloy_sol_macro::sol! {
	#![sol(all_derives)]
	struct Body {
		// Amount of the asset to be sent
		uint256 amount;
		// Maximum amount to pay for liquidity fees
		uint256 max_fee;
		// The asset identifier
		bytes32 asset_id;
		// Flag to redeem the erc20 asset on the destination
		bool redeem;
		// Sender address
		bytes32 from;
		// Recipient address
		bytes32 to;
	}
}

#[derive(Clone)]
pub struct Module<T>(PhantomData<T>);

impl<T: Config> Default for Module<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<T: Config> IsmpModule for Module<T>
where
	<T::Assets as fungibles::Inspect<T::AccountId>>::Balance: From<u128>,
	u128: From<<T::Assets as fungibles::Inspect<T::AccountId>>::Balance>,
	<T::Assets as fungibles::Inspect<T::AccountId>>::AssetId: From<Location>,
	T::AccountId: Into<[u8; 32]> + From<[u8; 32]>,
{
	fn on_accept(&self, post: ismp::router::PostRequest) -> Result<(), ismp::error::Error> {
		let request = Request::Post(post.clone());
		// Check that source module is equal to the known token gateway deployment address
		ensure!(
			request.source_module() == Pallet::<T>::token_gateway_address(&post.source).0.to_vec(),
			ismp::error::Error::ModuleDispatchError {
				msg: "Token Gateway: Unknown source contract address".to_string(),
				meta: Meta {
					source: request.source_chain(),
					dest: request.dest_chain(),
					nonce: request.nonce(),
				},
			}
		);

		// parachains/solochains shouldn't be sending us a request.
		ensure!(
			!matches!(
				request.source_chain(),
				StateMachine::Kusama(_) |
					StateMachine::Polkadot(_) |
					StateMachine::Grandpa(_) |
					StateMachine::Beefy(_)
			),
			ismp::error::Error::ModuleDispatchError {
				msg: "Token Gateway: Illegal source chain".to_string(),
				meta: Meta {
					source: request.source_chain(),
					dest: request.dest_chain(),
					nonce: request.nonce(),
				},
			}
		);

		let body = Body::abi_decode(&mut &post.body[1..], true).map_err(|_| {
			ismp::error::Error::ModuleDispatchError {
				msg: "Token Gateway: Failed to decode request body".to_string(),
				meta: Meta {
					source: request.source_chain(),
					dest: request.dest_chain(),
					nonce: request.nonce(),
				},
			}
		})?;

		// Check that the asset id is equal to the known asset id
		ensure!(
			body.asset_id.0 == Pallet::<T>::dot_asset_id().0,
			ismp::error::Error::ModuleDispatchError {
				msg: "Token Gateway: AssetId is unknown".to_string(),
				meta: Meta {
					source: request.source_chain(),
					dest: request.dest_chain(),
					nonce: request.nonce(),
				},
			}
		);

		let amount = convert_to_balance(U256::from_big_endian(&body.amount.to_be_bytes::<32>()))
			.map_err(|_| ismp::error::Error::ModuleDispatchError {
				msg: "Token Gateway: Trying to withdraw Invalid amount".to_string(),
				meta: Meta {
					source: request.source_chain(),
					dest: request.dest_chain(),
					nonce: request.nonce(),
				},
			})?;

		let asset_id = Location::parent();

		// We don't custody user funds, we send the dot back to the relaychain using xcm
		let xcm_beneficiary: Location =
			Junction::AccountId32 { network: None, id: body.to.0 }.into();
		let xcm_dest = VersionedLocation::V4(Location::parent());
		let fee_asset_item = 0;
		let weight_limit = WeightLimit::Unlimited;
		let asset = Asset { id: AssetId(asset_id), fun: Fungibility::Fungible(amount) };

		let mut assets = Assets::new();
		assets.push(asset);

		// Send xcm back to relaychain
		pallet_xcm::Pallet::<T>::limited_reserve_transfer_assets(
			frame_system::RawOrigin::Signed(Pallet::<T>::account_id()).into(),
			Box::new(xcm_dest),
			Box::new(xcm_beneficiary.into()),
			Box::new(VersionedAssets::V4(assets)),
			fee_asset_item,
			weight_limit,
		)
		.map_err(|_| ismp::error::Error::ModuleDispatchError {
			msg: "Token Gateway: Failed execute xcm to relay chain".to_string(),
			meta: Meta {
				source: request.source_chain(),
				dest: request.dest_chain(),
				nonce: request.nonce(),
			},
		})?;

		Pallet::<T>::deposit_event(Event::<T>::AssetReceived {
			beneficiary: body.to.0.into(),
			amount: amount.into(),
			source: request.source_chain(),
		});

		Ok(())
	}

	fn on_response(&self, response: ismp::router::Response) -> Result<(), ismp::error::Error> {
		Err(ismp::error::Error::ModuleDispatchError {
			msg: "Token Gateway does not accept responses".to_string(),
			meta: Meta {
				source: response.source_chain(),
				dest: response.dest_chain(),
				nonce: response.nonce(),
			},
		})
	}

	fn on_timeout(&self, request: Timeout) -> Result<(), ismp::error::Error> {
		// We don't custody user funds, we send the dot back to the relaychain using xcm
		match request {
			Timeout::Request(Request::Post(post)) => {
				let request = Request::Post(post.clone());
				let commitment = hash_request::<<T as Config>::IsmpHost>(&request);
				let fee_metadata = pallet_ismp::child_trie::RequestCommitments::<T>::get(
					commitment,
				)
				.ok_or_else(|| ismp::error::Error::ModuleDispatchError {
					msg: "Token Gateway: Fee metadata could not be found for request".to_string(),
					meta: Meta {
						source: request.source_chain(),
						dest: request.dest_chain(),
						nonce: request.nonce(),
					},
				})?;
				let beneficiary = fee_metadata.fee.payer;
				let body = Body::abi_decode(&mut &post.body[1..], true).map_err(|_| {
					ismp::error::Error::ModuleDispatchError {
						msg: "Token Gateway: Failed to decode request body".to_string(),
						meta: Meta {
							source: request.source_chain(),
							dest: request.dest_chain(),
							nonce: request.nonce(),
						},
					}
				})?;
				// Send xcm back to relaychain

				let amount =
					convert_to_balance(U256::from_big_endian(&body.amount.to_be_bytes::<32>()))
						.map_err(|_| ismp::error::Error::ModuleDispatchError {
							msg: "Token Gateway: Trying to withdraw Invalid amount".to_string(),
							meta: Meta {
								source: request.source_chain(),
								dest: request.dest_chain(),
								nonce: request.nonce(),
							},
						})?;
				// We do an xcm limited reserve transfer from the pallet custody account to the user
				// on the relaychain;
				let xcm_beneficiary: Location =
					Junction::AccountId32 { network: None, id: beneficiary.clone().into() }.into();
				let xcm_dest = VersionedLocation::V4(Location::parent());
				let fee_asset_item = 0;
				let weight_limit = WeightLimit::Unlimited;
				let asset =
					Asset { id: AssetId(Location::parent()), fun: Fungibility::Fungible(amount) };

				let mut assets = Assets::new();
				assets.push(asset);
				pallet_xcm::Pallet::<T>::limited_reserve_transfer_assets(
					frame_system::RawOrigin::Signed(Pallet::<T>::account_id()).into(),
					Box::new(xcm_dest),
					Box::new(xcm_beneficiary.into()),
					Box::new(VersionedAssets::V4(assets)),
					fee_asset_item,
					weight_limit,
				)
				.map_err(|_| ismp::error::Error::ModuleDispatchError {
					msg: "Token Gateway: Failed to execute xcm to relay chain".to_string(),
					meta: Meta {
						source: request.source_chain(),
						dest: request.dest_chain(),
						nonce: request.nonce(),
					},
				})?;

				Pallet::<T>::deposit_event(Event::<T>::AssetRefunded {
					beneficiary,
					amount: amount.into(),
					source: request.dest_chain(),
				});

				Ok(())
			},
			Timeout::Request(Request::Get(get)) => Err(ismp::error::Error::ModuleDispatchError {
				msg: "Tried to timeout unsupported request type".to_string(),
				meta: Meta { source: get.source, dest: get.dest, nonce: get.nonce },
			}),

			Timeout::Response(response) => Err(ismp::error::Error::ModuleDispatchError {
				msg: "Tried to timeout unsupported request type".to_string(),
				meta: Meta {
					source: response.source_chain(),
					dest: response.dest_chain(),
					nonce: response.nonce(),
				},
			}),
		}
	}
}

/// Converts an ERC20 U256 to a DOT u128
pub fn convert_to_balance(value: U256) -> Result<u128, anyhow::Error> {
	let dec_str = (value / U256::from(100_000_000u128)).to_string();
	dec_str.parse().map_err(|e| anyhow::anyhow!("{e:?}"))
}

/// Converts a DOT u128 to an Erc20 denomination
pub fn convert_to_erc20(value: u128) -> U256 {
	U256::from(value) * U256::from(100_000_000u128)
}

#[cfg(test)]
mod tests {
	use sp_core::U256;
	use sp_runtime::Permill;
	use std::ops::Mul;

	use crate::{convert_to_balance, convert_to_erc20};
	#[test]
	fn test_per_mill() {
		let per_mill = Permill::from_parts(1_000);

		println!("{}", per_mill.mul(20_000_000u128));
	}

	#[test]
	fn balance_conversions() {
		let supposedly_small_u256 = U256::from_dec_str("1000000000000000000").unwrap();
		// convert erc20 value to dot value
		let converted_balance = convert_to_balance(supposedly_small_u256).unwrap();
		println!("{}", converted_balance);

		let dot = 10_000_000_000u128;

		assert_eq!(converted_balance, dot);

		// Convert 1 dot to erc20

		let dot = 10_000_000_000u128;
		let erc_20_val = convert_to_erc20(dot);
		assert_eq!(erc_20_val, U256::from_dec_str("1000000000000000000").unwrap());
	}

	#[test]
	fn max_value_check() {
		let max = U256::MAX;

		let converted_balance = convert_to_balance(max);
		assert!(converted_balance.is_err())
	}

	#[test]
	fn min_value_check() {
		let min = U256::from(1u128);

		let converted_balance = convert_to_balance(min).unwrap();
		assert_eq!(converted_balance, 0);
	}
}
