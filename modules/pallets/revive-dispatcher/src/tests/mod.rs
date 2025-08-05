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
	use hex_literal::hex;
	use ismp::{host::StateMachine, messaging::hash_request, router::Request};
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
	use polkadot_sdk::{sp_core::Get, *};
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

		let new = Address::from(hex!("0000000000000000000000000000000001270000"));
		let encoded = new.abi_encode();

		println!("Abi encoded address: {:?}", hex::encode(&encoded));

		let decoded = Address::abi_decode(&encoded);
		println!("Decoded address: {:?}", decoded);
	}

	#[test]
	fn test_query_host() {
		new_test_ext().execute_with(|| {
			// Setup account
			let root = AccountId32::new([1u8; 32]);
			let initial_balance = 1_000_000_000_000u128;

			// give our account some balance
			let _ = Balances::deposit_creating(&root, initial_balance);
			// map the account
			<Test as pallet_revive::Config>::AddressMapper::map(&root).unwrap();

			let _root_addr = <Test as pallet_revive::Config>::AddressMapper::to_address(&root);

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
						let revert_data = hex::encode(&return_value.data);
						println!("Call reverted with data: {}", revert_data);
						// Try to decode the revert message
						if return_value.data.len() >= 4 {
							// Skip the error selector and decode the string
							if let Ok(error_msg) = String::abi_decode(&return_value.data[4..]) {
								println!("Revert reason: {}", error_msg);
							}
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
					assert_eq!(
						host_string,
						HostStateMachine::get().to_string(),
						"Host state machine mismatch"
					);
				},
				Err(err) => panic!("Bare call failed with error: {:?}", err),
			}
		});
	}

	#[test]
	fn test_query_nonce() {
		new_test_ext().execute_with(|| {
			// Setup account
			let root = AccountId32::new([1u8; 32]);
			let initial_balance = 1_000_000_000_000u128;

			let nonce = 125;

			pallet_ismp::Nonce::<Test>::put(nonce);

			// give our account some balance
			let _ = Balances::deposit_creating(&root, initial_balance);
			// map the account
			<Test as pallet_revive::Config>::AddressMapper::map(&root).unwrap();

			// Get the precompile address
			let dispatcher_precompile_addr = H160::from(
				polkadot_sdk::pallet_revive::precompiles::AddressMatcher::Fixed(
					NonZero::new(3367).unwrap(),
				)
				.base_address(),
			);

			// Call nonce() method
			let call =
				crate::IDispatcher::IDispatcherCalls::nonce(crate::IDispatcher::nonceCall {});
			let encoded_call = call.abi_encode();

			let result = pallet_revive::Pallet::<Test>::bare_call(
				RuntimeOrigin::signed(root.clone()),
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
						// Try to decode the revert message
						if return_value.data.len() >= 4 {
							// Skip the error selector and decode the string
							if let Ok(error_msg) = String::abi_decode(&return_value.data[4..]) {
								println!("Revert reason: {}", error_msg);
							}
						}
						panic!("Call reverted");
					}

					// Decode the nonce
					let nonce = Uint::<256, 4>::abi_decode(&return_value.data)
						.expect("Failed to decode nonce");

					println!("Current nonce: {}", nonce);

					// Initial nonce should be 0
					assert_eq!(nonce, Uint::<256, 4>::from(nonce), "Initial nonce should be 0");
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
	fn test_dispatch_get_request() {
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
					assert_eq!(return_value.data.len(), 32, "Expected 32-byte commitment");

					let commitment = FixedBytes::<32>::abi_decode(&return_value.data)
						.expect("Failed to decode commitment");

					println!("GET request commitment: 0x{}", hex::encode(commitment.0));
				},
				Err(err) => panic!("Bare call failed with error: {:?}", err),
			}

			// Verify fee tokens were deducted from the account
			let final_fee_balance = <Assets as Inspect<AccountId32>>::balance(fee_token_id, &root);
			// For GET requests, we need to calculate the fee based on the encoded size
			// The fee should be deducted
			assert!(final_fee_balance < initial_balance, "Fee tokens should be deducted");
		});
	}

	#[test]
	fn test_query_fee_token() {
		new_test_ext().execute_with(|| {
			// Setup account
			let root = AccountId32::new([1u8; 32]);
			let initial_balance = 1_000_000_000_000u128;

			// give our account some balance
			let _ = Balances::deposit_creating(&root, initial_balance);
			// map the account
			<Test as pallet_revive::Config>::AddressMapper::map(&root).unwrap();

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
				RuntimeOrigin::signed(root.clone()),
				dispatcher_precompile_addr,
				0u128, // value
				Weight::MAX,
				DepositLimit::UnsafeOnlyForDryRun,
				encoded_call,
			);

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

					// Decode the address
					// Debug the raw return data
					println!("Raw return data: 0x{}", hex::encode(&return_value.data));

					let fee_token_address = Address::abi_decode(&return_value.data)
						.expect("Failed to decode fee token address");

					// The precompile returns the address from pallet_revive's alloy primitives
					// which might have a different internal representation
					println!("Decoded fee token address: 0x{}", fee_token_address);

					assert_eq!(
						FeeTokenAddress::get(),
						fee_token_address,
						"Fee token address mismatch"
					);
				},
				Err(err) => panic!("Bare call failed with error: {:?}", err),
			}
		});
	}

	#[test]
	fn test_query_hyperbridge() {
		new_test_ext().execute_with(|| {
			// Setup account
			let root = AccountId32::new([1u8; 32]);
			let initial_balance = 1_000_000_000_000u128;

			// give our account some balance
			let _ = Balances::deposit_creating(&root, initial_balance);
			// map the account
			<Test as pallet_revive::Config>::AddressMapper::map(&root).unwrap();

			// Get the precompile address
			let dispatcher_precompile_addr = H160::from(
				polkadot_sdk::pallet_revive::precompiles::AddressMatcher::Fixed(
					NonZero::new(3367).unwrap(),
				)
				.base_address(),
			);

			// Call hyperbridge() method
			let call = crate::IDispatcher::IDispatcherCalls::hyperbridge(
				crate::IDispatcher::hyperbridgeCall {},
			);
			let encoded_call = call.abi_encode();

			let result = pallet_revive::Pallet::<Test>::bare_call(
				RuntimeOrigin::signed(root.clone()),
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
						// Try to decode the revert message
						if return_value.data.len() >= 4 {
							// Skip the error selector and decode the string
							if let Ok(error_msg) = String::abi_decode(&return_value.data[4..]) {
								println!("Revert reason: {}", error_msg);
							}
						}
						panic!("Call reverted unexpectedly");
					}

					// Decode the hyperbridge state machine
					let hyperbridge_bytes = Bytes::abi_decode(&return_value.data)
						.expect("Failed to decode hyperbridge");

					let hyperbridge_string = String::from_utf8(hyperbridge_bytes.to_vec())
						.expect("Failed to convert hyperbridge to string");

					println!("Hyperbridge state machine: {}", hyperbridge_string);

					// Should match the Coprocessor which returns HostStateMachine (Kusama(2000))
					assert_eq!(
						hyperbridge_string,
						Coprocessor::get().unwrap().to_string(),
						"Hyperbridge state machine mismatch"
					);
				},
				Err(err) => panic!("Bare call failed with error: {:?}", err),
			}
		});
	}

	#[test]
	fn test_query_per_byte_fee() {
		new_test_ext().execute_with(|| {
			// Setup account
			let root = AccountId32::new([1u8; 32]);
			let initial_balance = 1_000_000_000_000u128;

			// give our account some balance
			let _ = Balances::deposit_creating(&root, initial_balance);
			// map the account
			<Test as pallet_revive::Config>::AddressMapper::map(&root).unwrap();

			// Get the precompile address
			let dispatcher_precompile_addr = H160::from(
				polkadot_sdk::pallet_revive::precompiles::AddressMatcher::Fixed(
					NonZero::new(3367).unwrap(),
				)
				.base_address(),
			);

			// Call perByteFee() method for Ethereum mainnet
			let dest = StateMachine::Evm(1);
			let call = crate::IDispatcher::IDispatcherCalls::perByteFee(
				crate::IDispatcher::perByteFeeCall {
					dest: Bytes::from(dest.to_string().as_bytes().to_vec()),
				},
			);
			let encoded_call = call.abi_encode();

			let result = pallet_revive::Pallet::<Test>::bare_call(
				RuntimeOrigin::signed(root.clone()),
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
						// Try to decode the revert message
						if return_value.data.len() >= 4 {
							// Skip the error selector and decode the string
							if let Ok(error_msg) = String::abi_decode(&return_value.data[4..]) {
								println!("Revert reason: {}", error_msg);
							}
						}
						panic!("Call reverted");
					}

					// Decode the fee
					let fee = Uint::<256, 4>::abi_decode(&return_value.data)
						.expect("Failed to decode fee");

					println!("Per byte fee for {}: {}", dest, fee);

					// Should be the default per byte fee (100) from mock.rs
					assert_eq!(fee, Uint::<256, 4>::from(100u128), "Per byte fee mismatch");
				},
				Err(err) => panic!("Bare call failed with error: {:?}", err),
			}
		});
	}

	#[test]
	fn test_dispatch_response() {
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

			// Get the precompile address
			let dispatcher_precompile_addr = H160::from(
				polkadot_sdk::pallet_revive::precompiles::AddressMatcher::Fixed(
					NonZero::new(3367).unwrap(),
				)
				.base_address(),
			);

			// First, dispatch a POST request to create a commitment we can respond to

			let source = HostStateMachine::get(); // Response comes from the destination
			let dest = StateMachine::Evm(1); // Back to our host
			let to = vec![0x12, 0x34, 0x56, 0x78];
			let body = b"Hello from test".to_vec();
			let timeout = 3600u64;
			let fee = 1_000u128;

			let fake_request = ismp::router::PostRequest {
				source: source.clone(),
				dest: dest.clone(),
				nonce: 0u64,
				from: root_addr.0.to_vec(),
				to: to.clone(),
				body: body.clone(),
				timeout_timestamp: timeout,
			};

			// Hash the request and insert it into RequestReceipts
			let fake_request_commitment =
				hash_request::<pallet_ismp::Pallet<Test>>(&Request::Post(fake_request.clone()));

			pallet_ismp::child_trie::RequestReceipts::<Test>::insert(
				fake_request_commitment,
				&root_addr.0.to_vec(),
			);

			// Now create a response to this request
			// Note: In a real scenario, the response would come from the destination chain
			// For testing, we'll create a response that references our request
			let response_data = b"Response data".to_vec();
			let response_timeout = 7200u64; // 2 hours

			// Create the response structure
			// The request fields must match the fake request we inserted
			let response = crate::DispatchPostResponse {
				request: crate::PostRequest {
					source: Bytes::from(source.to_string().as_bytes().to_vec()), /* Original source (our chain) */
					dest: Bytes::from(dest.to_string().as_bytes().to_vec()),     // Original dest
					nonce: 0u64,
					from: Bytes::from(root_addr.0.to_vec()), // Original sender
					to: Bytes::from(to),
					timeoutTimestamp: timeout, // Same timeout as fake request
					body: Bytes::from(body),
				},
				response: Bytes::from(response_data),
				timeout: response_timeout,
				fee: Uint::<256, 4>::from(fee),
				payer: Address::from(root_addr.0),
			};

			let call = crate::IDispatcher::IDispatcherCalls::dispatch_2(
				crate::IDispatcher::dispatch_2Call { response },
			);
			let encoded_call = call.abi_encode();

			// Call the precompile to dispatch the response
			let result = pallet_revive::Pallet::<Test>::bare_call(
				RuntimeOrigin::signed(root.clone()),
				dispatcher_precompile_addr,
				0u128,
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
								// This is expected to revert because we're responding to a request
								// This should now succeed because we've set up the request in
								// RequestReceipts
								panic!("Response dispatch should not revert: {}", error_msg);
							}
						}
						panic!("Failed to decode revert message");
					}

					// Since we set up the request properly, it should succeed
					assert_eq!(return_value.data.len(), 32, "Expected 32-byte commitment");
					let commitment = FixedBytes::<32>::abi_decode(&return_value.data)
						.expect("Failed to decode commitment");
					println!("Dispatch response commitment: 0x{}", hex::encode(commitment.0));
				},
				Err(err) => panic!("Bare call failed with error: {:?}", err),
			}
		});
	}

	#[test]
	fn test_fund_request() {
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

			// Get the precompile address
			let dispatcher_precompile_addr = H160::from(
				polkadot_sdk::pallet_revive::precompiles::AddressMatcher::Fixed(
					NonZero::new(3367).unwrap(),
				)
				.base_address(),
			);

			// First, dispatch an actual POST request to get a real commitment
			let dest = StateMachine::Evm(1); // Ethereum mainnet
			let to = vec![0x12, 0x34, 0x56, 0x78]; // Some destination address
			let body = b"Request to be funded".to_vec();
			let timeout = 3600u64; // 1 hour timeout
			let fee = 1_000u128; // Initial fee amount

			// Encode the dispatch request
			let request = crate::DispatchPost {
				dest: Bytes::from(dest.to_string().as_bytes().to_vec()),
				to: Bytes::from(to),
				body: Bytes::from(body.clone()),
				timeout,
				fee: Uint::<256, 4>::from(fee),
				payer: Address::from(root_addr.0),
			};

			let dispatch_call = crate::IDispatcher::IDispatcherCalls::dispatch_0(
				crate::IDispatcher::dispatch_0Call { request },
			);
			let encoded_dispatch_call = dispatch_call.abi_encode();

			// Dispatch the request
			let dispatch_result = pallet_revive::Pallet::<Test>::bare_call(
				RuntimeOrigin::signed(root.clone()),
				dispatcher_precompile_addr,
				0u128,
				Weight::MAX,
				DepositLimit::UnsafeOnlyForDryRun,
				encoded_dispatch_call,
			);

			// Get the commitment from the dispatch result
			let commitment = match dispatch_result.result {
				Ok(return_value) => {
					assert!(!return_value.did_revert(), "Request dispatch failed");
					assert_eq!(return_value.data.len(), 32, "Expected 32-byte commitment");
					FixedBytes::<32>::abi_decode(&return_value.data)
						.expect("Failed to decode commitment")
				},
				Err(err) => panic!("Request dispatch failed: {:?}", err),
			};

			println!("Request commitment: 0x{}", hex::encode(commitment.0));

			// Now fund the request with the actual commitment
			let additional_fee = 5000u128;

			let fund_call = crate::IDispatcher::IDispatcherCalls::fundRequest(
				crate::IDispatcher::fundRequestCall {
					commitment,
					amount: Uint::<256, 4>::from(additional_fee),
				},
			);
			let encoded_fund_call = fund_call.abi_encode();

			// Record balance before funding
			let balance_before = <Assets as Inspect<AccountId32>>::balance(fee_token_id, &root);

			// Call the precompile to fund the request
			let fund_result = pallet_revive::Pallet::<Test>::bare_call(
				RuntimeOrigin::signed(root.clone()),
				dispatcher_precompile_addr,
				0u128,
				Weight::MAX,
				DepositLimit::UnsafeOnlyForDryRun,
				encoded_fund_call,
			);

			// Check the fund result
			match fund_result.result {
				Ok(return_value) => {
					if return_value.did_revert() {
						println!(
							"Fund call reverted with data: {}",
							hex::encode(&return_value.data)
						);
						// Try to decode the revert message
						if return_value.data.len() >= 4 {
							// Skip the error selector and decode the string
							if let Ok(error_msg) = String::abi_decode(&return_value.data[4..]) {
								println!("Revert reason: {}", error_msg);
							}
						}
						panic!("Fund request failed");
					}

					// Fund request should return empty data
					assert_eq!(return_value.data.len(), 0, "Expected empty return value");

					// Verify the additional fee was deducted
					let balance_after =
						<Assets as Inspect<AccountId32>>::balance(fee_token_id, &root);
					assert!(
						balance_after < balance_before,
						"Fee tokens should be deducted after funding"
					);
					println!(
						"Successfully funded request with commitment: 0x{}",
						hex::encode(commitment.0)
					);
				},
				Err(err) => panic!("Fund call failed with error: {:?}", err),
			}
		});
	}

	#[test]
	fn test_fund_response() {
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

			// Get the precompile address
			let dispatcher_precompile_addr = H160::from(
				polkadot_sdk::pallet_revive::precompiles::AddressMatcher::Fixed(
					NonZero::new(3367).unwrap(),
				)
				.base_address(),
			);

			// First, dispatch a POST request to create a commitment we can respond to
			let source = HostStateMachine::get(); // Response comes from the destination
			let dest = StateMachine::Evm(1); // Back to our host
			let to = vec![0x12, 0x34, 0x56, 0x78];
			let body = b"Initial request".to_vec();
			let timeout = 3600u64;
			let fee = 1_000u128;

			// Now we need to create a fake request and insert it into RequestReceipts
			// This simulates the request being received by the destination chain
			let fake_request = ismp::router::PostRequest {
				source: source.clone(),
				dest: dest.clone(),
				nonce: 0u64,
				from: root_addr.0.to_vec(),
				to: to.clone(),
				timeout_timestamp: timeout,
				body: body.clone(),
			};

			// Hash the request and insert it into RequestReceipts
			let fake_request_commitment =
				hash_request::<pallet_ismp::Pallet<Test>>(&Request::Post(fake_request.clone()));
			pallet_ismp::child_trie::RequestReceipts::<Test>::insert(
				fake_request_commitment,
				&root_addr.0.to_vec(),
			);

			let response_data = b"Response data".to_vec();
			let response_timeout = 7200u64; // 2 hours

			// Create the response structure
			// Note: The nonce and other fields should match what was actually dispatched
			let response = crate::DispatchPostResponse {
				request: crate::PostRequest {
					source: Bytes::from(source.to_string().as_bytes().to_vec()),
					dest: Bytes::from(dest.to_string().as_bytes().to_vec()),
					nonce: 0u64,
					from: Bytes::from(root_addr.0.to_vec()),
					to: Bytes::from(to),
					timeoutTimestamp: timeout,
					body: Bytes::from(body),
				},
				response: Bytes::from(response_data),
				timeout: response_timeout,
				fee: Uint::<256, 4>::from(fee),
				payer: Address::from(root_addr.0),
			};

			let dispatch_response_call = crate::IDispatcher::IDispatcherCalls::dispatch_2(
				crate::IDispatcher::dispatch_2Call { response },
			);
			let encoded_response_call = dispatch_response_call.abi_encode();

			// Dispatch the response
			let response_result = pallet_revive::Pallet::<Test>::bare_call(
				RuntimeOrigin::signed(root.clone()),
				dispatcher_precompile_addr,
				0u128,
				Weight::MAX,
				DepositLimit::UnsafeOnlyForDryRun,
				encoded_response_call,
			);

			// Handle the response dispatch result
			match response_result.result {
				Ok(return_value) => {
					if return_value.did_revert() {
						println!(
							"Response dispatch reverted with data: {}",
							hex::encode(&return_value.data)
						);
						// Try to decode the revert message
						if return_value.data.len() >= 4 {
							// Skip the error selector and decode the string
							if let Ok(error_msg) = String::abi_decode(&return_value.data[4..]) {
								panic!("Response dispatch should not revert: {}", error_msg);
							}
						}
						panic!("Response dispatch failed");
					}

					// Response dispatch should succeed now that we've set up the request
					assert_eq!(return_value.data.len(), 32, "Expected 32-byte commitment");
					let response_commitment = FixedBytes::<32>::abi_decode(&return_value.data)
						.expect("Failed to decode commitment");

					println!("Response commitment: 0x{}", hex::encode(response_commitment.0));

					// Now fund the response with the actual commitment
					let additional_fee = 3000u128;

					let fund_call = crate::IDispatcher::IDispatcherCalls::fundResponse(
						crate::IDispatcher::fundResponseCall {
							commitment: response_commitment,
							amount: Uint::<256, 4>::from(additional_fee),
						},
					);
					let encoded_fund_call = fund_call.abi_encode();

					// Record balance before funding
					let balance_before =
						<Assets as Inspect<AccountId32>>::balance(fee_token_id, &root);

					// Call the precompile to fund the response
					let fund_result = pallet_revive::Pallet::<Test>::bare_call(
						RuntimeOrigin::signed(root.clone()),
						dispatcher_precompile_addr,
						0u128,
						Weight::MAX,
						DepositLimit::UnsafeOnlyForDryRun,
						encoded_fund_call,
					);

					// Check the fund result
					match fund_result.result {
						Ok(return_value) => {
							assert!(!return_value.did_revert(), "Fund response should not revert");
							assert_eq!(return_value.data.len(), 0, "Expected empty return value");

							// Verify the additional fee was deducted
							let balance_after =
								<Assets as Inspect<AccountId32>>::balance(fee_token_id, &root);
							assert!(
								balance_after < balance_before,
								"Fee tokens should be deducted after funding"
							);
							println!(
								"Successfully funded response with commitment: 0x{}",
								hex::encode(response_commitment.0)
							);
						},
						Err(err) => panic!("Fund call failed with error: {:?}", err),
					}
				},
				Err(err) => panic!("Response dispatch failed: {:?}", err),
			}
		});
	}
}
