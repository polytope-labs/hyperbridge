use alloy_sol_types::SolValue;
use ismp::{
	host::StateMachine,
	router::{PostRequest, Request, Timeout},
};
use pallet_token_gateway::{impls::convert_to_erc20, Body, TeleportParams};
use sp_core::{ByteArray, H160, H256, U256};
use staging_xcm::prelude::Location;
use xcm_simulator_example::ALICE;

use crate::runtime::{
	new_test_ext, RuntimeOrigin, Test, TokenGateway, TokenGatewayInspector, INITIAL_BALANCE,
};
use ismp::module::IsmpModule;

const SEND_AMOUNT: u128 = 1000_000_000_0000;

#[test]
fn should_teleport_asset_correctly() {
	new_test_ext().execute_with(|| {
		let params = TeleportParams {
			asset_id: Location::here(),
			destination: StateMachine::Evm(1),
			recepient: H160::random(),
			timeout: 0,
			amount: SEND_AMOUNT,
		};

		TokenGateway::teleport(RuntimeOrigin::signed(ALICE), params).unwrap();

		let new_balance = pallet_balances::Pallet::<Test>::free_balance(ALICE);

		assert_eq!(new_balance, INITIAL_BALANCE - SEND_AMOUNT);
	})
}

#[test]
fn should_receive_asset_correctly() {
	new_test_ext().execute_with(|| {
		let params = TeleportParams {
			asset_id: Location::here(),
			destination: StateMachine::Evm(1),
			recepient: H160::random(),
			timeout: 0,
			amount: SEND_AMOUNT,
		};

		TokenGateway::teleport(RuntimeOrigin::signed(ALICE), params).unwrap();

		let new_balance = pallet_balances::Pallet::<Test>::free_balance(ALICE);

		assert_eq!(new_balance, INITIAL_BALANCE - SEND_AMOUNT);

		let module = TokenGateway::default();
		let post = PostRequest {
			source: StateMachine::Evm(1),
			dest: StateMachine::Kusama(100),
			nonce: 0,
			from: H160::zero().0.to_vec(),
			to: H160::zero().0.to_vec(),
			timeout_timestamp: 1000,
			body: {
				let body = Body {
					amount: {
						let mut bytes = [0u8; 32];
						// Module callback will convert to ten decimals
						convert_to_erc20(SEND_AMOUNT).to_big_endian(&mut bytes);
						alloy_primitives::U256::from_be_bytes(bytes)
					},
					asset_id: H256::zero().0.into(),
					redeem: false,
					from: alloy_primitives::B256::from_slice(ALICE.as_slice()),
					to: alloy_primitives::B256::from_slice(ALICE.as_slice()),
				};

				let encoded = vec![vec![0], Body::abi_encode(&body)].concat();
				encoded
			},
		};

		module.on_accept(post).unwrap();
		let new_balance = pallet_balances::Pallet::<Test>::free_balance(ALICE);

		assert_eq!(new_balance, INITIAL_BALANCE);
	});
}

#[test]
fn should_timeout_request_correctly() {
	new_test_ext().execute_with(|| {
		let params = TeleportParams {
			asset_id: Location::here(),
			destination: StateMachine::Evm(1),
			recepient: H160::random(),
			timeout: 0,
			amount: SEND_AMOUNT,
		};

		TokenGateway::teleport(RuntimeOrigin::signed(ALICE), params).unwrap();

		let new_balance = pallet_balances::Pallet::<Test>::free_balance(ALICE);

		assert_eq!(new_balance, INITIAL_BALANCE - SEND_AMOUNT);

		let module = TokenGateway::default();
		let post = PostRequest {
			source: StateMachine::Evm(1),
			dest: StateMachine::Kusama(100),
			nonce: 0,
			from: H160::zero().0.to_vec(),
			to: H160::zero().0.to_vec(),
			timeout_timestamp: 1000,
			body: {
				let body = Body {
					amount: {
						let mut bytes = [0u8; 32];
						// Module callback will convert to ten decimals
						convert_to_erc20(SEND_AMOUNT).to_big_endian(&mut bytes);
						alloy_primitives::U256::from_be_bytes(bytes)
					},
					asset_id: H256::zero().0.into(),
					redeem: false,
					from: alloy_primitives::B256::from_slice(ALICE.as_slice()),
					to: alloy_primitives::B256::from_slice(ALICE.as_slice()),
				};

				let encoded = vec![vec![0], Body::abi_encode(&body)].concat();
				encoded
			},
		};

		module.on_timeout(Timeout::Request(Request::Post(post))).unwrap();
		let new_balance = pallet_balances::Pallet::<Test>::free_balance(ALICE);

		assert_eq!(new_balance, INITIAL_BALANCE);
	});
}

#[test]
fn inspector_should_intercept_illegal_request() {
	new_test_ext().execute_with(|| {
		let asset_id: H256 = [1u8; 32].into();
		let post = PostRequest {
			source: StateMachine::Kusama(100),
			dest: StateMachine::Evm(1),
			nonce: 0,
			from: H160::zero().0.to_vec(),
			to: H160::zero().0.to_vec(),
			timeout_timestamp: 1000,
			body: {
				let body = Body {
					amount: {
						let mut bytes = [0u8; 32];
						// Module callback will convert to ten decimals
						convert_to_erc20(SEND_AMOUNT).to_big_endian(&mut bytes);
						alloy_primitives::U256::from_be_bytes(bytes)
					},
					asset_id: asset_id.0.into(),
					redeem: false,
					from: alloy_primitives::B256::from_slice(ALICE.as_slice()),
					to: alloy_primitives::B256::from_slice(ALICE.as_slice()),
				};

				let encoded = vec![vec![0], Body::abi_encode(&body)].concat();
				encoded
			},
		};

		let result = TokenGatewayInspector::inspect_request(&post);
		println!("{result:?}");
		assert!(result.is_err());

		pallet_token_gateway_inspector::InflowBalances::<Test>::insert(
			asset_id,
			convert_to_erc20(SEND_AMOUNT),
		);

		let result = TokenGatewayInspector::inspect_request(&post);
		assert!(result.is_ok());
		let inflow = pallet_token_gateway_inspector::InflowBalances::<Test>::get(asset_id);
		assert_eq!(inflow, U256::zero());
	});
}

#[test]
fn inspector_should_record_non_native_asset_inflow() {
	new_test_ext().execute_with(|| {
		let asset_id: H256 = [1u8; 32].into();
		let post = PostRequest {
			source: StateMachine::Evm(1),
			dest: StateMachine::Kusama(100),
			nonce: 0,
			from: H160::zero().0.to_vec(),
			to: H160::zero().0.to_vec(),
			timeout_timestamp: 1000,
			body: {
				let body = Body {
					amount: {
						let mut bytes = [0u8; 32];
						// Module callback will convert to ten decimals
						convert_to_erc20(SEND_AMOUNT).to_big_endian(&mut bytes);
						alloy_primitives::U256::from_be_bytes(bytes)
					},
					asset_id: asset_id.0.into(),
					redeem: false,
					from: alloy_primitives::B256::from_slice(ALICE.as_slice()),
					to: alloy_primitives::B256::from_slice(ALICE.as_slice()),
				};

				let encoded = vec![vec![0], Body::abi_encode(&body)].concat();
				encoded
			},
		};

		let result = TokenGatewayInspector::inspect_request(&post);
		println!("{result:?}");
		assert!(result.is_ok());

		let inflow = pallet_token_gateway_inspector::InflowBalances::<Test>::get(asset_id);

		assert_eq!(convert_to_erc20(SEND_AMOUNT), inflow);
	});
}

#[test]
fn inspector_should_handle_timeout_correctly() {
	new_test_ext().execute_with(|| {
		let asset_id: H256 = [1u8; 32].into();
		let post = PostRequest {
			source: StateMachine::Kusama(100),
			dest: StateMachine::Evm(1),
			nonce: 0,
			from: H160::zero().0.to_vec(),
			to: H160::zero().0.to_vec(),
			timeout_timestamp: 1000,
			body: {
				let body = Body {
					amount: {
						let mut bytes = [0u8; 32];
						// Module callback will convert to ten decimals
						convert_to_erc20(SEND_AMOUNT).to_big_endian(&mut bytes);
						alloy_primitives::U256::from_be_bytes(bytes)
					},
					asset_id: asset_id.0.into(),
					redeem: false,
					from: alloy_primitives::B256::from_slice(ALICE.as_slice()),
					to: alloy_primitives::B256::from_slice(ALICE.as_slice()),
				};

				let encoded = vec![vec![0], Body::abi_encode(&body)].concat();
				encoded
			},
		};

		let inflow = pallet_token_gateway_inspector::InflowBalances::<Test>::get(asset_id);

		assert_eq!(inflow, U256::zero());

		let result = TokenGatewayInspector::handle_timeout(&post);
		println!("{result:?}");
		assert!(result.is_ok());

		let inflow = pallet_token_gateway_inspector::InflowBalances::<Test>::get(asset_id);

		assert_eq!(convert_to_erc20(SEND_AMOUNT), inflow);
	});
}
