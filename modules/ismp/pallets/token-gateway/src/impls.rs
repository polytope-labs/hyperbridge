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

use alloc::{format, string::ToString, vec::Vec};
use codec::{Decode, Encode};
use frame_support::ensure;
use ismp::{
	dispatcher::{FeeMetadata, IsmpDispatcher},
	events::Meta,
	module::IsmpModule,
	router::{PostRequest, PostResponse, Response},
};
use pallet_token_governor::TokenGatewayParams;
use sp_core::{ConstU32, U256};
use sp_runtime::{traits::AccountIdConversion, BoundedVec};

use crate::{
	Config, Pallet, TokenGatewayAddressRequest, TokenGatewayAddressResponse, TokenGatewayAddresses,
	TokenGatewayReverseMap, PALLET_ID,
};

impl<T: Config> Pallet<T> {
	pub fn pallet_account() -> T::AccountId {
		PALLET_ID.into_account_truncating()
	}

	pub fn is_token_gateway(id: Vec<u8>) -> bool {
		TokenGatewayReverseMap::<T>::contains_key(id)
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

/// An Ismp Module that receives requests from substrate chains requesting for the token gateway
/// addresses on EVM chains
pub struct AddressRequestModule<T>(core::marker::PhantomData<T>);

impl<T> Default for AddressRequestModule<T> {
	fn default() -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<T: pallet_token_governor::Config> IsmpModule for AddressRequestModule<T>
where
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
{
	fn on_accept(&self, post: PostRequest) -> Result<(), ismp::Error> {
		let PostRequest { body: data, from, source, dest, nonce, .. } = post.clone();
		// Check that source module is equal to the known token gateway deployment address
		ensure!(
			from == PALLET_ID.0.to_vec(),
			ismp::error::Error::ModuleDispatchError {
				msg: "Token Gateway: Unknown source contract address".to_string(),
				meta: Meta { source, dest, nonce },
			}
		);

		ensure!(
			source.is_substrate(),
			ismp::error::Error::ModuleDispatchError {
				msg: "Token Gateway: Illegal source chain".to_string(),
				meta: Meta { source, dest, nonce },
			}
		);

		let req = TokenGatewayAddressRequest::decode(&mut &*data).map_err(|err| {
			ismp::error::Error::Custom(format!("Invalid request from a substrate chain: {err}"))
		})?;
		let mut addresses = BoundedVec::<_, ConstU32<5>>::new();
		for state_machine in req.chains {
			if let Some(params) = TokenGatewayParams::<T>::get(&state_machine) {
				addresses.try_push((state_machine, params.address)).map_err(|err| {
					ismp::error::Error::Custom(alloc::format!(
						"Maximum of 5 state machines can be requested: {err:?}"
					))
				})?;
			}
		}

		if !addresses.is_empty() {
			let response = TokenGatewayAddressResponse { addresses };
			let dispatcher = <T as pallet_token_governor::Config>::Dispatcher::default();

			let post_response =
				PostResponse { post, response: response.encode(), timeout_timestamp: 0 };

			let _ = dispatcher.dispatch_response(
				post_response,
				FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
			)?;
		}
		return Ok(())
	}

	fn on_response(&self, _response: ismp::router::Response) -> Result<(), ismp::Error> {
		Err(ismp::error::Error::Custom("Module does not accept responses".to_string()))
	}

	fn on_timeout(&self, _request: ismp::router::Timeout) -> Result<(), ismp::Error> {
		Err(ismp::error::Error::Custom("Module does not accept timeouts".to_string()))
	}
}

pub struct AddressResponseModule<T>(core::marker::PhantomData<T>);

impl<T> Default for AddressResponseModule<T> {
	fn default() -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<T: Config> IsmpModule for AddressResponseModule<T> {
	fn on_accept(&self, _post: PostRequest) -> Result<(), ismp::Error> {
		Err(ismp::error::Error::Custom("Module does not accept requests".to_string()))
	}

	fn on_response(&self, response: Response) -> Result<(), ismp::error::Error> {
		let data = response.response().ok_or_else(|| ismp::error::Error::ModuleDispatchError {
			msg: "AddressResponseModule: Response has no body".to_string(),
			meta: Meta {
				source: response.source_chain(),
				dest: response.dest_chain(),
				nonce: response.nonce(),
			},
		})?;
		let resp = TokenGatewayAddressResponse::decode(&mut &*data).map_err(|_| {
			ismp::error::Error::ModuleDispatchError {
				msg: "AddressResponseModule: Failed to decode response body".to_string(),
				meta: Meta {
					source: response.source_chain(),
					dest: response.dest_chain(),
					nonce: response.nonce(),
				},
			}
		})?;
		for (state_machine, addr) in resp.addresses {
			TokenGatewayAddresses::<T>::insert(state_machine, addr);
			TokenGatewayReverseMap::<T>::insert(addr.0.to_vec(), state_machine)
		}
		Ok(())
	}

	fn on_timeout(&self, _request: ismp::router::Timeout) -> Result<(), ismp::Error> {
		Err(ismp::error::Error::Custom("Module does not accept timeouts".to_string()))
	}
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
