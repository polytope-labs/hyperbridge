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

mod mock;

#[cfg(test)]
mod test {
	use super::mock::*;

	use core::num::NonZero;
	use frame_support::{
		traits::{
			fungibles::{Create, Inspect, Mutate},
			Currency,
		},
		weights::Weight,
	};
	use ismp::host::StateMachine;
	use pallet_revive::{
		precompiles::{
			alloy::{
				primitives::{Address, Bytes, FixedBytes, Uint},
				sol_types::{SolInterface, SolValue},
			},
			AddressMapper, AddressMatcher, H160,
		},
		DepositLimit,
	};
	use polkadot_sdk::*;
	use sp_runtime::AccountId32;

	#[test]
	fn runtime_compiles() {
		// Just verify the runtime compiles
		new_test_ext().execute_with(|| {
			// Runtime compiled successfully
		});
	}

	#[test]
	fn test_address_matcher_prefix() {
		// Create the AddressMatcher::Prefix with value 0x127
		let matcher = AddressMatcher::Prefix(NonZero::new(0x127).unwrap());

		// Get the base address
		let base_address = matcher.base_address();

		// Convert the byte array to hex string
		let hex_string =
			base_address.iter().map(|byte| format!("{:02x}", byte)).collect::<String>();

		// Print the base address in hex format
		println!("Base address in hex: 0x{}", hex_string);

		// Also print the raw bytes for clarity
		println!("Base address bytes: {:?}", base_address);
	}

	#[test]
	fn test_query_host_through_bare_call() {
		new_test_ext().execute_with(|| {
			// Get the precompile address for the ISMP dispatcher
			let dispatcher_precompile_addr = H160::from(
				polkadot_sdk::pallet_revive::precompiles::AddressMatcher::Fixed(
					NonZero::new(3367).unwrap(),
				)
				.base_address(),
			);

			// Call host() method - this is a view function that doesn't require fees
			let call = crate::IDispatcher::IDispatcherCalls::host(crate::IDispatcher::hostCall {});
			let encoded_call = call.abi_encode();

			// Call the precompile through bare_call
			// The precompile address is the destination, not the origin
			let result = pallet_revive::Pallet::<Test>::bare_call(
				RuntimeOrigin::root(), // Use root to bypass signer checks
				dispatcher_precompile_addr,
				0u128, // value
				Weight::MAX,
				DepositLimit::UnsafeOnlyForDryRun,
				encoded_call,
			);

			// Check the result
			match result.result {
				Ok(return_value) => {
					if return_value.did_revert() {
						let revert_data = hex::encode(&return_value.data);
						println!("Call reverted with data: {}", revert_data);
						// Try to decode as string if it's a revert message
						if let Ok(decoded) =
							String::from_utf8(hex::decode(&revert_data).unwrap_or_default())
						{
							println!("Decoded revert message: {}", decoded);
						}
						panic!("Call reverted");
					}

					// Decode the host state machine
					let host_bytes =
						Bytes::abi_decode(&return_value.data).expect("Failed to decode host");

					let host_string = String::from_utf8(host_bytes.to_vec())
						.expect("Failed to convert host to string");

					println!("Host state machine: {}", host_string);

					// Should match the HostStateMachine from mock.rs (Kusama(2000))
					assert_eq!(host_string, "KUSAMA-2000", "Host state machine mismatch");
				},
				Err(err) => panic!("Bare call failed with error: {:?}", err),
			}
		});
	}

	#[test]
	fn test_query_nonce_through_bare_call() {
		new_test_ext().execute_with(|| {
			// Get the precompile address
			let dispatcher_precompile_addr = H160::from(
				polkadot_sdk::pallet_revive::precompiles::AddressMatcher::Fixed(
					NonZero::new(3367).unwrap(),
				)
				.base_address(),
			);

			pallet_ismp::Nonce::<Test>::put(100);

			// Call nonce() method
			let call =
				crate::IDispatcher::IDispatcherCalls::nonce(crate::IDispatcher::nonceCall {});
			let encoded_call = call.abi_encode();

			let result = pallet_revive::Pallet::<Test>::bare_call(
				RuntimeOrigin::root(),
				dispatcher_precompile_addr,
				0u128,
				Weight::MAX,
				DepositLimit::UnsafeOnlyForDryRun,
				encoded_call,
			);

			match result.result {
				Ok(return_value) => {
					if return_value.did_revert() {
						println!("Call reverted with data: {}", hex::encode(&return_value.data));
						panic!("Call reverted");
					}

					// Decode the nonce
					let nonce = Uint::<256, 4>::abi_decode(&return_value.data)
						.expect("Failed to decode nonce");

					println!("Current nonce: {}", nonce);

					// Initial nonce should be 0
					assert_eq!(nonce, Uint::<256, 4>::from(100u64), "Initial nonce should be 0");
				},
				Err(err) => panic!("Bare call failed with error: {:?}", err),
			}
		});
	}

	#[test]
	fn test_dispatch_post_request_with_proper_setup() {
		new_test_ext().execute_with(|| {
			// Setup: Create and fund the fee token asset
			let fee_token_id = 0x127u32;
			let initial_balance = 1_000_000_000_000u128;

			let root = AccountId32::new([1u8; 32]);
			// give our account some balance
			let _ = Balances::deposit_creating(&root, initial_balance);
			// map the account
			<Test as pallet_revive::Config>::AddressMapper::map(&root).unwrap();

			let root_addr = <Test as pallet_revive::Config>::AddressMapper::to_address(&root);
			println!("Root H160 address: 0x{}", hex::encode(root_addr.0));
			println!("Root AccountId: 0x{}", hex::encode(&root));

			// Create the asset
			let _ = <Assets as Create<AccountId32>>::create(
				fee_token_id,
				root.clone(),
				true,
				1u128, // min_balance
			);

			// Fund the account with fee tokens
			let _ =
				<Assets as Mutate<AccountId32>>::mint_into(fee_token_id, &root, initial_balance)
					.unwrap();

			println!(
				"Account balance: {}",
				<Assets as Inspect<AccountId32>>::balance(fee_token_id, &root)
			);

			let fee_balance = <Assets as Inspect<AccountId32>>::balance(fee_token_id, &root);
			println!("Precompile account fee token balance: {}", fee_balance);

			// Get the precompile address & map it to an account ID
			let dispatcher_precompile_addr = H160::from(
				polkadot_sdk::pallet_revive::precompiles::AddressMatcher::Fixed(
					NonZero::new(3367).unwrap(),
				)
				.base_address(),
			);
			let precompile_account = <Test as pallet_revive::Config>::AddressMapper::to_account_id(
				&dispatcher_precompile_addr,
			);

			// Debug the account mapping
			println!("Precompile H160 address: 0x{}", hex::encode(dispatcher_precompile_addr.0));
			println!("Precompile AccountId: 0x{}", hex::encode(precompile_account));

			// Create a POST request
			let dest = StateMachine::Evm(1); // Ethereum mainnet
			let to = vec![0x12, 0x34, 0x56, 0x78]; // Some destination address
			let body = b"Hello from test".to_vec();
			let timeout = 3600u64; // 1 hour timeout
			let fee = 1_000u128; // Fee amount

			// Encode the dispatch request
			let request = crate::DispatchPost {
				dest: Bytes::from(dest.to_string().as_bytes().to_vec()),
				to: Bytes::from(to),
				body: Bytes::from(body.clone()),
				timeout,
				fee: Uint::<256, 4>::from(fee),
				payer: Address::from(root_addr.0), // Will be replaced by the actual caller address
			};

			let call = crate::IDispatcher::IDispatcherCalls::dispatch_0(
				crate::IDispatcher::dispatch_0Call { request },
			);
			let encoded_call = call.abi_encode();

			// Call the precompile through bare_call
			let result = pallet_revive::Pallet::<Test>::bare_call(
				RuntimeOrigin::signed(root.clone()),
				dispatcher_precompile_addr,
				0u128, // value
				Weight::MAX,
				DepositLimit::UnsafeOnlyForDryRun,
				encoded_call,
			);

			// Check the result
			match result.result {
				Ok(return_value) => {
					if return_value.did_revert() {
						println!("Call reverted with data: {}", hex::encode(&return_value.data));
						// Try to decode the revert message
						if return_value.data.len() >= 4 {
							// Skip the error selector and decode the string
							if let Ok(error_msg) = String::abi_decode(&return_value.data[4..]) {
								println!("Revert reason: {}", error_msg);
							}
						}
						panic!("Call reverted");
					}

					// The return value should be a 32-byte commitment hash
					assert_eq!(return_value.data.len(), 32, "Expected 32-byte commitment");

					let commitment = FixedBytes::<32>::abi_decode(&return_value.data)
						.expect("Failed to decode commitment");

					println!("Dispatch request commitment: 0x{}", hex::encode(commitment.0));
				},
				Err(err) => panic!("Bare call failed with error: {:?}", err),
			}

			// Verify fee tokens were deducted from the precompile account
			let final_fee_balance = <Assets as Inspect<AccountId32>>::balance(fee_token_id, &root);
			let dispatch_fee = 100 * 32 as u128;
			assert_eq!(
				final_fee_balance + fee + dispatch_fee,
				initial_balance,
				"Fee tokens should be deducted"
			);
		});
	}

	#[test]
	fn test_dispatch_get_request_through_bare_call() {
		new_test_ext().execute_with(|| {
			// Setup: Create and fund the fee token asset
			let fee_token_id = 0x127u32;
			let root = AccountId32::new([0u8; 32]); // Root account for asset creation

			// Create the asset
			let _ = <Assets as Create<AccountId32>>::create(
				fee_token_id,
				root.clone(),
				true,
				1u128, // min_balance
			);

			// Get the precompile address
			let dispatcher_precompile_addr = H160::from(
				polkadot_sdk::pallet_revive::precompiles::AddressMatcher::Fixed(
					NonZero::new(3367).unwrap(),
				)
				.base_address(),
			);

			// Fund the precompile account itself (which will be the payer when using Root origin)
			let precompile_account = <Test as pallet_revive::Config>::AddressMapper::to_account_id(
				&dispatcher_precompile_addr,
			);

			// Debug the account mapping
			println!(
				"GET test - Precompile H160 address: 0x{}",
				hex::encode(dispatcher_precompile_addr.0)
			);
			println!("GET test - Precompile account (mapped): {:?}", precompile_account);

			// Fund the precompile account with native tokens and fee tokens
			let _ = Balances::deposit_creating(&precompile_account, 1_000_000_000_000u128);
			let _ = <Assets as Mutate<AccountId32>>::mint_into(
				fee_token_id,
				&precompile_account,
				1_000_000u128,
			);
			let fee_balance =
				<Assets as Inspect<AccountId32>>::balance(fee_token_id, &precompile_account);
			println!("GET test - Precompile account fee token balance: {}", fee_balance);

			// Create a GET request
			let dest = StateMachine::Evm(1);
			let keys = vec![Bytes::from(b"key1".to_vec()), Bytes::from(b"key2".to_vec())];
			let height = 1000u64;
			let context = b"test-context".to_vec();
			let timeout = 3600u64;
			let fee = 1000u128;

			// Encode the GET request
			let request = crate::DispatchGet {
				dest: Bytes::from(dest.to_string().as_bytes().to_vec()),
				keys,
				height,
				context: Bytes::from(context),
				timeout,
				fee: Uint::<256, 4>::from(fee),
			};

			let call = crate::IDispatcher::IDispatcherCalls::dispatch_1(
				crate::IDispatcher::dispatch_1Call { request },
			);
			let encoded_call = call.abi_encode();

			// Call the precompile
			let result = pallet_revive::Pallet::<Test>::bare_call(
				RuntimeOrigin::root(),
				dispatcher_precompile_addr,
				0u128, // value
				Weight::MAX,
				DepositLimit::UnsafeOnlyForDryRun,
				encoded_call,
			);

			// Check the result
			match result.result {
				Ok(return_value) => {
					if return_value.did_revert() {
						println!("Call reverted with data: {}", hex::encode(&return_value.data));
						panic!("Call reverted");
					}
					assert_eq!(return_value.data.len(), 32, "Expected 32-byte commitment");

					let commitment = FixedBytes::<32>::abi_decode(&return_value.data)
						.expect("Failed to decode commitment");

					println!("GET request commitment: 0x{}", hex::encode(commitment.0));
				},
				Err(err) => panic!("Bare call failed with error: {:?}", err),
			}

			// Verify fee tokens were deducted from the precompile account
			let final_fee_balance =
				<Assets as Inspect<AccountId32>>::balance(fee_token_id, &precompile_account);
			assert!(final_fee_balance < 1_000_000u128, "Fee tokens should be deducted");
		});
	}

	#[test]
	fn test_query_fee_token() {
		new_test_ext().execute_with(|| {
			// Get the precompile address
			let dispatcher_precompile_addr = H160::from(
				polkadot_sdk::pallet_revive::precompiles::AddressMatcher::Fixed(
					NonZero::new(3367).unwrap(),
				)
				.base_address(),
			);

			// Call feeToken() method
			let call =
				crate::IDispatcher::IDispatcherCalls::feeToken(crate::IDispatcher::feeTokenCall {});
			let encoded_call = call.abi_encode();

			let result = pallet_revive::Pallet::<Test>::bare_call(
				RuntimeOrigin::root(),
				dispatcher_precompile_addr,
				0u128, // value
				Weight::MAX,
				DepositLimit::UnsafeOnlyForDryRun,
				encoded_call,
			);

			match result.result {
				Ok(return_value) => {
					if return_value.did_revert() {
						println!("Call reverted with data: {:?}", hex::encode(&return_value.data));
						panic!("Call reverted");
					}

					// Decode the address
					// Debug the raw return data
					println!("Raw return data: 0x{}", hex::encode(&return_value.data));
					println!("Return data length: {}", return_value.data.len());

					let fee_token_address = Address::abi_decode(&return_value.data)
						.expect("Failed to decode fee token address");

					// The precompile returns the address from pallet_revive's alloy primitives
					// which might have a different internal representation
					println!(
						"Decoded fee token address: 0x{}",
						hex::encode(fee_token_address.0 .0)
					);

					// Verify it's using the correct bytes from FeeTokenAddress
					// The first 8 bytes should be 0x42 based on the actual output
					let fee_token_bytes = fee_token_address.0 .0;
					assert_eq!(
						&fee_token_bytes[0..8],
						&[0x42u8; 8],
						"Fee token address should start with 8 bytes of 0x42"
					);
				},
				Err(err) => panic!("Bare call failed with error: {:?}", err),
			}
		});
	}
}
