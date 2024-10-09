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

use crate::token_gateway_id;
use alloc::string::ToString;
use frame_support::PalletId;
use sp_core::U256;
use sp_runtime::traits::AccountIdConversion;

use crate::{Config, Pallet};

impl<T: Config> Pallet<T> {
	pub fn pallet_account() -> T::AccountId {
		let mut inner = [0u8; 8];
		inner.copy_from_slice(&token_gateway_id().0[0..8]);
		PalletId(inner).into_account_truncating()
	}

	pub fn is_token_gateway(id: &[u8]) -> bool {
		id == &token_gateway_id().0
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

	use super::{convert_to_balance, convert_to_erc20};

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
