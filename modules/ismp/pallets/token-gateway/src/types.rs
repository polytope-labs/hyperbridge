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

//! Pallet types

use alloc::vec::Vec;
use anyhow::anyhow;
use frame_support::{pallet_prelude::*, traits::fungibles};
use ismp::host::StateMachine;
use pallet_token_governor::{AssetMetadata, ERC6160AssetRegistration};
use primitive_types::H256;

use crate::Config;

pub type AssetId<T> =
	<<T as Config>::Assets as fungibles::Inspect<<T as frame_system::Config>::AccountId>>::AssetId;

/// Asset teleportation parameters
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct TeleportParams<AssetId, Balance> {
	/// Asset Id registered on Hyperbridge
	pub asset_id: AssetId,
	/// Destination state machine
	pub destination: StateMachine,
	/// Receiving account on destination
	pub recepient: H256,
	/// Amount to be sent
	pub amount: Balance,
	/// Request timeout
	pub timeout: u64,
	/// Token gateway address
	pub token_gateway: Vec<u8>,
	/// Relayer fee
	pub relayer_fee: Balance,
}

/// Local asset Id and its corresponding token gateway asset id
#[derive(Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq, RuntimeDebug)]
pub struct AssetMap<AssetId> {
	/// Local Asset Id
	pub local_id: AssetId,
	/// MNT Asset registration details
	pub reg: ERC6160AssetRegistration,
}

/// A struct for registering some assets
#[derive(Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq, RuntimeDebug)]
#[scale_info(skip_type_params(T))]
pub struct AssetRegistration<AssetId> {
	pub assets: BoundedVec<AssetMap<AssetId>, ConstU32<10>>,
}

alloy_sol_macro::sol! {
	#![sol(all_derives)]
	struct Body {
		// Amount of the asset to be sent
		uint256 amount;
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

/// A trait that helps in creating new assets in the runtime
pub trait CreateAsset<AssetId> {
	/// Create an asset and return its local asset id
	fn create_asset(meta: AssetMetadata) -> Result<AssetId, anyhow::Error>;
}

impl<AssetId> CreateAsset<AssetId> for () {
	fn create_asset(_meta: AssetMetadata) -> Result<AssetId, anyhow::Error> {
		Err(anyhow!("Unimplemented"))
	}
}
