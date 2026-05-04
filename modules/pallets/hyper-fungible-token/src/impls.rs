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

//! Helper implementations for the hyper-fungible-token pallet

use alloc::string::ToString;
use polkadot_sdk::*;
use sp_core::U256;

use crate::{Config, Pallet};

impl<T: Config> Pallet<T> {
	/// Returns the pallet's custodial account for holding native assets
	pub fn pallet_account() -> T::AccountId {
		use frame_support::PalletId;
		use sp_runtime::traits::AccountIdConversion;
		PalletId(*b"hft__acc").into_account_truncating()
	}
}

/// Converts an ERC20 U256 amount to a local u128 balance
///
/// Divides by 10^(erc_decimals - local_decimals) to scale down from ERC20 precision
pub fn convert_to_balance(
	value: U256,
	erc_decimals: u8,
	local_decimals: u8,
) -> Result<u128, anyhow::Error> {
	let dec_str = (value /
		U256::from(10u128.pow(erc_decimals.saturating_sub(local_decimals) as u32)))
	.to_string();
	dec_str.parse().map_err(|e| anyhow::anyhow!("{e:?}"))
}

/// Converts a local u128 balance to an ERC20 U256 amount
///
/// Multiplies by 10^(erc_decimals - local_decimals) to scale up to ERC20 precision
pub fn convert_to_erc20(value: u128, erc_decimals: u8, local_decimals: u8) -> U256 {
	U256::from(value) * U256::from(10u128.pow(erc_decimals.saturating_sub(local_decimals) as u32))
}
