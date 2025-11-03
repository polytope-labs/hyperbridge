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
use pallet_token_gateway::{
	impls::{convert_to_balance, convert_to_erc20},
	types::Body,
};
use pallet_token_governor::TokenGatewayParams;
use polkadot_sdk::*;

use frame_support::{
	ensure,
	traits::{
		fungibles::{self},
		Get,
	},
};
use polkadot_sdk::{
	cumulus_primitives_core::{AllCounted, DepositAsset, Parachain, Weight, Wild, Xcm},
	staging_xcm_executor::traits::TransferType,
};

use crate::xcm_utilities::ASSET_HUB_PARA_ID;
use ismp::{
	dispatcher::{DispatchPost, DispatchRequest, FeeMetadata, IsmpDispatcher},
	events::Meta,
	host::{IsmpHost, StateMachine},
	module::IsmpModule,
	router::{Request, Timeout},
};
pub use pallet::*;
use sp_core::{H160, H256, U256};
use sp_runtime::{traits::AccountIdConversion, Permill};
use staging_xcm::{
	prelude::Assets,
	v5::{Asset, AssetId, Fungibility, Junction, Location, WeightLimit},
	VersionedAssets, VersionedLocation, VersionedXcm,
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
		polkadot_sdk::frame_system::Config
		+ pallet_ismp::Config
		+ pallet_xcm::Config
		+ pallet_token_governor::Config
	{
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

		/// Origin for privileged actions
		type GatewayOrigin: EnsureOrigin<
			<Self as polkadot_sdk::frame_system::Config>::RuntimeOrigin,
		>;
	}

	#[pallet::storage]
	#[pallet::getter(fn params)]
	pub type Params<T> = StorageValue<_, AssetGatewayParams, OptionQuery>;

	/// The map of XCM location to asset Ids
	#[pallet::storage]
	pub type AssetIds<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		<T::Assets as fungibles::Inspect<T::AccountId>>::AssetId,
		Location,
		OptionQuery,
	>;

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
			/// XCM message hash
			message_id: H256,
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

	#[derive(
		Clone,
		Encode,
		Decode,
		DecodeWithMemTracking,
		scale_info::TypeInfo,
		Eq,
		PartialEq,
		RuntimeDebug,
	)]
	pub struct AssetGatewayParams {
		/// Percentage to be taken as protocol fees
		pub protocol_fee_percentage: Permill,
	}

	impl AssetGatewayParams {
		pub const fn from_parts(protocol_fee_percentage: Permill) -> Self {
			Self { protocol_fee_percentage }
		}
	}

	#[derive(
		Clone,
		Encode,
		Decode,
		DecodeWithMemTracking,
		scale_info::TypeInfo,
		Eq,
		PartialEq,
		RuntimeDebug,
	)]
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
			T::GatewayOrigin::ensure_origin(origin)?;
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
		identifier: H256,
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
				let bytes = convert_to_erc20(amount, 18, 10).to_big_endian();
				alloy_primitives::U256::from_be_bytes(bytes)
			},
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
			message_id: identifier,
		});

		Ok(())
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
	T::AccountId: Into<[u8; 32]> + From<[u8; 32]>,
{
	fn on_accept(&self, post: ismp::router::PostRequest) -> Result<Weight, anyhow::Error> {
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
					StateMachine::Substrate(_) |
					StateMachine::Relay { .. }
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

		let amount =
			convert_to_balance(U256::from_big_endian(&body.amount.to_be_bytes::<32>()), 18, 10)
				.map_err(|_| ismp::error::Error::ModuleDispatchError {
					msg: "Token Gateway: Trying to withdraw Invalid amount".to_string(),
					meta: Meta {
						source: request.source_chain(),
						dest: request.dest_chain(),
						nonce: request.nonce(),
					},
				})?;

		let asset_id = Location::parent();

		// We don't custody user funds, we send the dot back to assethub using xcm
		let xcm_beneficiary: Location =
			Junction::AccountId32 { network: None, id: body.to.0 }.into();

		let xcm_dest = VersionedLocation::V5(Location::new(1, [Parachain(ASSET_HUB_PARA_ID)]));
		let weight_limit = WeightLimit::Unlimited;
		let asset = Asset { id: AssetId(asset_id), fun: Fungibility::Fungible(amount) };

		let mut assets = Assets::new();
		assets.push(asset.clone());

		let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
			assets: Wild(AllCounted(assets.len() as u32)),
			beneficiary: xcm_beneficiary.clone(),
		}]);

		// Send xcm back to assethub
		pallet_xcm::Pallet::<T>::transfer_assets_using_type_and_then(
			frame_system::RawOrigin::Signed(Pallet::<T>::account_id()).into(),
			Box::new(xcm_dest),
			Box::new(VersionedAssets::V5(assets.clone())),
			Box::new(TransferType::DestinationReserve),
			Box::new(asset.id.into()),
			Box::new(TransferType::DestinationReserve),
			Box::new(VersionedXcm::from(custom_xcm_on_dest)),
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

		Ok(T::DbWeight::get().reads_writes(0, 0))
	}

	fn on_response(&self, response: ismp::router::Response) -> Result<Weight, anyhow::Error> {
		Err(ismp::error::Error::ModuleDispatchError {
			msg: "Token Gateway does not accept responses".to_string(),
			meta: Meta {
				source: response.source_chain(),
				dest: response.dest_chain(),
				nonce: response.nonce(),
			},
		}
		.into())
	}

	fn on_timeout(&self, request: Timeout) -> Result<Weight, anyhow::Error> {
		// We don't custody user funds, we send the dot back to the relaychain using xcm
		match request {
			Timeout::Request(Request::Post(post)) => {
				let request = Request::Post(post.clone());
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

				let beneficiary = body.from.0.into();
				// Send xcm back to relaychain
				let amount = convert_to_balance(
					U256::from_big_endian(&body.amount.to_be_bytes::<32>()),
					18,
					10,
				)
				.map_err(|_| ismp::error::Error::ModuleDispatchError {
					msg: "Token Gateway: Trying to withdraw Invalid amount".to_string(),
					meta: Meta {
						source: request.source_chain(),
						dest: request.dest_chain(),
						nonce: request.nonce(),
					},
				})?;
				// We do an xcm limited reserve transfer from the pallet custody account to the user
				// on assethub;
				let xcm_beneficiary: Location =
					Junction::AccountId32 { network: None, id: body.from.0 }.into();
				let asset_id = Location::parent();
				let xcm_dest =
					VersionedLocation::V5(Location::new(1, [Parachain(ASSET_HUB_PARA_ID)]));
				let weight_limit = WeightLimit::Unlimited;
				let asset = Asset { id: AssetId(asset_id), fun: Fungibility::Fungible(amount) };

				let mut assets = Assets::new();
				assets.push(asset.clone());

				let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
					assets: Wild(AllCounted(assets.len() as u32)),
					beneficiary: xcm_beneficiary.clone(),
				}]);

				pallet_xcm::Pallet::<T>::transfer_assets_using_type_and_then(
					frame_system::RawOrigin::Signed(Pallet::<T>::account_id()).into(),
					Box::new(xcm_dest),
					Box::new(VersionedAssets::V5(assets.clone())),
					Box::new(TransferType::DestinationReserve),
					Box::new(asset.id.into()),
					Box::new(TransferType::DestinationReserve),
					Box::new(VersionedXcm::from(custom_xcm_on_dest)),
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

				Ok(T::DbWeight::get().reads_writes(0, 0))
			},
			Timeout::Request(Request::Get(get)) => Err(ismp::error::Error::ModuleDispatchError {
				msg: "Tried to timeout unsupported request type".to_string(),
				meta: Meta { source: get.source, dest: get.dest, nonce: get.nonce },
			}
			.into()),

			Timeout::Response(response) => Err(ismp::error::Error::ModuleDispatchError {
				msg: "Tried to timeout unsupported request type".to_string(),
				meta: Meta {
					source: response.source_chain(),
					dest: response.dest_chain(),
					nonce: response.nonce(),
				},
			}
			.into()),
		}
	}
}
