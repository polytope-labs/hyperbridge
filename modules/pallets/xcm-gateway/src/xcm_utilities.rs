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

use crate::{AssetIds, Config, Pallet};
use alloc::vec::Vec;
use codec::{Decode, Encode};
use core::{cmp::min, marker::PhantomData};
use frame_support::traits::{
	fungibles::{self, Mutate},
	Contains,
};
use ismp::host::StateMachine;
use polkadot_sdk::*;
use sp_core::{Get, H160};
use sp_runtime::traits::MaybeEquivalence;
use staging_xcm::v5::{
	Asset, Error as XcmError, Fungibility::Fungible, Junction, Junctions, Location, NetworkId,
	Result as XcmResult, XcmContext,
};
use staging_xcm_builder::{AssetChecking, FungiblesMutateAdapter};
use staging_xcm_executor::{
	traits::{ConvertLocation, Error as MatchError, MatchesFungibles, TransactAsset},
	AssetsInHolding,
};
pub const ASSET_HUB_PARA_ID: u32 = 1000;
pub struct WrappedNetworkId(pub NetworkId);

impl TryFrom<WrappedNetworkId> for StateMachine {
	type Error = ();

	fn try_from(value: WrappedNetworkId) -> Result<Self, Self::Error> {
		match value.0 {
			NetworkId::Ethereum { chain_id } => Ok(StateMachine::Evm(chain_id as u32)),
			// Only transforms ethereum network ids
			_ => Err(()),
		}
	}
}

/// Converts a MutiLocation to a substrate account and an evm account if the multilocation
/// description matches a supported Ismp State machine
pub struct MultilocationToMultiAccount<A>(PhantomData<A>);

#[derive(Encode, Decode, Clone)]
pub struct MultiAccount<A> {
	/// Origin substrate account on the relaychain
	pub substrate_account: A,
	/// Destination evm account
	pub evm_account: H160,
	/// Destination state machine
	pub dest_state_machine: StateMachine,
	/// Request time out in seconds
	pub timeout: u64,
	/// Nonce of the substrate account that sent the tx on the relaychain
	pub account_nonce: u64,
}

// Supports a Multilocation interior of Junctions::X3
// Junctions::X4(AccountId32 { .. }, AccountKey20 { .. }, GeneralIndex(..), GeneralIndex(..))
// The value specified in the first GeneralIndex will be used as the timeout in seconds for the ismp
// request that will be dispatched
// The value in the second GeneralIndex should be the nonce of the substrate account on the
// relaychain
impl<A> ConvertLocation<MultiAccount<A>> for MultilocationToMultiAccount<A>
where
	A: From<[u8; 32]> + Into<[u8; 32]> + Clone,
{
	fn convert_location(location: &Location) -> Option<MultiAccount<A>> {
		match location {
			Location { parents: 0, interior: Junctions::X4(arc_junctions) } => {
				// Dereference the Arc to access the underlying array
				match arc_junctions.as_ref() {
					[Junction::AccountId32 { id, .. }, Junction::AccountKey20 { network: Some(network), key }, Junction::GeneralIndex(timeout), Junction::GeneralIndex(nonce)] =>
					{
						// Ensure that the network Id is one of the supported ethereum networks
						// If it transforms correctly we return the ethereum account
						let dest_state_machine =
							StateMachine::try_from(WrappedNetworkId(network.clone())).ok()?;
						Some(MultiAccount {
							substrate_account: A::from(*id),
							evm_account: H160::from(*key),
							dest_state_machine,
							timeout: *timeout as u64,
							account_nonce: *nonce as u64,
						})
					},
					_ => None,
				}
			},
			// Any other multilocation format is unsupported
			_ => None,
		}
	}
}

pub struct ConvertAssetId<T>(core::marker::PhantomData<T>);

impl<T: Config, AssetId: Clone> MaybeEquivalence<Location, AssetId> for ConvertAssetId<T>
where
	AssetId: From<[u8; 32]>,
	<T::Assets as fungibles::Inspect<T::AccountId>>::AssetId: From<AssetId>,
{
	fn convert(a: &Location) -> Option<AssetId> {
		let asset_id: AssetId = sp_io::hashing::keccak_256(&a.encode()).into();
		let converted: <T::Assets as fungibles::Inspect<T::AccountId>>::AssetId =
			asset_id.clone().into();
		if !AssetIds::<T>::contains_key(converted.clone()) {
			AssetIds::<T>::insert(converted, a.clone());
		}
		Some(asset_id)
	}

	fn convert_back(b: &AssetId) -> Option<Location> {
		let converted: <T::Assets as fungibles::Inspect<T::AccountId>>::AssetId = b.clone().into();
		AssetIds::<T>::get(converted)
	}
}

pub struct ReserveTransferFilter;

impl Contains<(Location, Vec<Asset>)> for ReserveTransferFilter {
	fn contains(t: &(Location, Vec<Asset>)) -> bool {
		let native = Location::parent();
		t.1.iter().all(|asset| {
			if let Asset { id: asset_id, fun: Fungible(_) } = asset {
				asset_id.0 == native
			} else {
				false
			}
		})
	}
}

pub struct HyperbridgeAssetTransactor<T, Matcher, AccountIdConverter, CheckAsset, CheckingAccount>(
	PhantomData<(T, Matcher, AccountIdConverter, CheckAsset, CheckingAccount)>,
);

impl<
		T: Config,
		Matcher: MatchesFungibles<
			<T::Assets as fungibles::Inspect<T::AccountId>>::AssetId,
			<T::Assets as fungibles::Inspect<T::AccountId>>::Balance,
		>,
		AccountIdConverter: ConvertLocation<T::AccountId>,
		CheckAsset: AssetChecking<<T::Assets as fungibles::Inspect<T::AccountId>>::AssetId>,
		CheckingAccount: Get<T::AccountId>,
	> TransactAsset
	for HyperbridgeAssetTransactor<T, Matcher, AccountIdConverter, CheckAsset, CheckingAccount>
where
	<T::Assets as fungibles::Inspect<T::AccountId>>::Balance: Into<u128> + From<u128>,
	u128: From<<T::Assets as fungibles::Inspect<T::AccountId>>::Balance>,
	T::AccountId: Eq + Clone + From<[u8; 32]> + Into<[u8; 32]>,
{
	fn can_check_in(origin: &Location, what: &Asset, context: &XcmContext) -> XcmResult {
		FungiblesMutateAdapter::<
			T::Assets,
			Matcher,
			AccountIdConverter,
			T::AccountId,
			CheckAsset,
			CheckingAccount,
		>::can_check_in(origin, what, context)
	}

	fn check_in(origin: &Location, what: &Asset, context: &XcmContext) {
		FungiblesMutateAdapter::<
			T::Assets,
			Matcher,
			AccountIdConverter,
			T::AccountId,
			CheckAsset,
			CheckingAccount,
		>::check_in(origin, what, context)
	}

	fn can_check_out(dest: &Location, what: &Asset, context: &XcmContext) -> XcmResult {
		FungiblesMutateAdapter::<
			T::Assets,
			Matcher,
			AccountIdConverter,
			T::AccountId,
			CheckAsset,
			CheckingAccount,
		>::can_check_out(dest, what, context)
	}

	fn check_out(dest: &Location, what: &Asset, context: &XcmContext) {
		FungiblesMutateAdapter::<
			T::Assets,
			Matcher,
			AccountIdConverter,
			T::AccountId,
			CheckAsset,
			CheckingAccount,
		>::check_out(dest, what, context)
	}

	fn deposit_asset(what: &Asset, who: &Location, _context: Option<&XcmContext>) -> XcmResult {
		// Check we handle this asset.
		let (asset_id, amount) = Matcher::matches_fungibles(what)?;
		// Ismp xcm transaction
		if let Some(who) = MultilocationToMultiAccount::<T::AccountId>::convert_location(who) {
			// We would remove the protocol fee at this point

			let protocol_account = Pallet::<T>::protocol_account_id();
			let pallet_account = Pallet::<T>::account_id();
			let protocol_percentage = Pallet::<T>::protocol_fee_percentage();

			// If destination is ETH mainnet charge a base fee of 2 DOT to cover expensive consensus
			// messages
			let base_fee =
				if who.dest_state_machine == StateMachine::Evm(1) { 20_000_000_000u128 } else { 0 };
			// Cap protocol fees at 10 DOT
			let protocol_fees =
				min(protocol_percentage * u128::from(amount) + base_fee, 100_000_000_000u128);
			let remainder = u128::from(amount)
				.checked_sub(protocol_fees.into())
				.ok_or_else(|| XcmError::Overflow)?
				.into();
			// Mint protocol fees
			T::Assets::mint_into(asset_id.clone(), &protocol_account, protocol_fees.into())
				.map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;
			// We custody the funds in the pallet account
			T::Assets::mint_into(asset_id, &pallet_account, remainder)
				.map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;
			// We dispatch an ismp request to the destination chain
			let identifier = {
				let encoded = who.encode();
				sp_io::hashing::keccak_256(&encoded).into()
			};
			Pallet::<T>::dispatch_request(who, identifier, remainder)
				.map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;
		} else {
			Err(MatchError::AccountIdConversionFailed)?
		}

		Ok(())
	}

	fn withdraw_asset(
		what: &Asset,
		who: &Location,
		maybe_context: Option<&XcmContext>,
	) -> Result<AssetsInHolding, XcmError> {
		FungiblesMutateAdapter::<
			T::Assets,
			Matcher,
			AccountIdConverter,
			T::AccountId,
			CheckAsset,
			CheckingAccount,
		>::withdraw_asset(what, who, maybe_context)
	}

	fn internal_transfer_asset(
		asset: &Asset,
		from: &Location,
		to: &Location,
		context: &XcmContext,
	) -> Result<AssetsInHolding, XcmError> {
		FungiblesMutateAdapter::<
			T::Assets,
			Matcher,
			AccountIdConverter,
			T::AccountId,
			CheckAsset,
			CheckingAccount,
		>::internal_transfer_asset(asset, from, to, context)
	}

	fn transfer_asset(
		asset: &Asset,
		from: &Location,
		to: &Location,
		context: &XcmContext,
	) -> Result<AssetsInHolding, XcmError> {
		FungiblesMutateAdapter::<
			T::Assets,
			Matcher,
			AccountIdConverter,
			T::AccountId,
			CheckAsset,
			CheckingAccount,
		>::transfer_asset(asset, from, to, context)
	}
}
