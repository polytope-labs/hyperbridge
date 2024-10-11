#![cfg(test)]

use alloy_sol_types::SolValue;
use ismp::{
	host::StateMachine,
	router::{PostRequest, Request, Timeout},
};
use pallet_token_gateway::{
	impls::convert_to_erc20, AssetMap, AssetRegistration, Body, CreateAssetId, TeleportParams,
};
use pallet_token_governor::{
	token_gateway_id, AssetMetadata, ChainWithSupply, ERC6160AssetRegistration, SolAssetMetadata,
	TokenGatewayRequest,
};
use sp_core::{ByteArray, Get, H160, H256, U256};

use xcm_simulator_example::ALICE;

use crate::runtime::{
	new_test_ext, AssetIdFactory, NativeAssetId, RuntimeOrigin, Test, TokenGateway,
	TokenGatewayInspector, INITIAL_BALANCE,
};
use ismp::module::IsmpModule;

const SEND_AMOUNT: u128 = 1000_000_000_0000;

#[test]
fn should_teleport_asset_correctly() {
	new_test_ext().execute_with(|| {
		let params = TeleportParams {
			asset_id: NativeAssetId::get(),
			destination: StateMachine::Evm(1),
			recepient: H256::random(),
			timeout: 0,
			amount: SEND_AMOUNT,
			token_gateway: H160::zero().0.to_vec(),
			relayer_fee: Default::default(),
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
			asset_id: NativeAssetId::get(),
			destination: StateMachine::Evm(1),
			recepient: H256::random(),
			timeout: 0,
			amount: SEND_AMOUNT,
			token_gateway: H160::zero().0.to_vec(),
			relayer_fee: Default::default(),
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
						convert_to_erc20(SEND_AMOUNT, 18, 10).to_big_endian(&mut bytes);
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
			asset_id: NativeAssetId::get(),
			destination: StateMachine::Evm(1),
			recepient: H256::random(),
			timeout: 0,
			amount: SEND_AMOUNT,
			token_gateway: H160::zero().0.to_vec(),
			relayer_fee: Default::default(),
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
						convert_to_erc20(SEND_AMOUNT, 18, 10).to_big_endian(&mut bytes);
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
			from: token_gateway_id().0.to_vec(),
			to: H160::zero().0.to_vec(),
			timeout_timestamp: 1000,
			body: {
				let body = Body {
					amount: {
						let mut bytes = [0u8; 32];
						// Module callback will convert to ten decimals
						convert_to_erc20(SEND_AMOUNT, 18, 10).to_big_endian(&mut bytes);
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
		assert!(result.is_err());

		pallet_token_gateway_inspector::InflowBalances::<Test>::insert(
			StateMachine::Kusama(100),
			asset_id,
			convert_to_erc20(SEND_AMOUNT, 18, 10),
		);

		let result = TokenGatewayInspector::inspect_request(&post);
		assert!(result.is_ok());
		let inflow = pallet_token_gateway_inspector::InflowBalances::<Test>::get(
			StateMachine::Kusama(100),
			asset_id,
		);
		assert_eq!(inflow, U256::zero());
	});
}

#[test]
fn inspector_should_record_asset_inflow() {
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
						convert_to_erc20(SEND_AMOUNT, 18, 10).to_big_endian(&mut bytes);
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
		assert!(result.is_ok());

		let inflow = pallet_token_gateway_inspector::InflowBalances::<Test>::get(
			StateMachine::Kusama(100),
			asset_id,
		);

		assert_eq!(convert_to_erc20(SEND_AMOUNT, 18, 10), inflow);
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
			from: token_gateway_id().0.to_vec(),
			to: H160::zero().0.to_vec(),
			timeout_timestamp: 1000,
			body: {
				let body = Body {
					amount: {
						let mut bytes = [0u8; 32];
						// Module callback will convert to ten decimals
						convert_to_erc20(SEND_AMOUNT, 18, 10).to_big_endian(&mut bytes);
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

		let inflow = pallet_token_gateway_inspector::InflowBalances::<Test>::get(
			StateMachine::Kusama(100),
			asset_id,
		);

		assert_eq!(inflow, U256::zero());

		pallet_token_gateway_inspector::InflowBalances::<Test>::insert(
			StateMachine::Evm(1),
			asset_id,
			convert_to_erc20(SEND_AMOUNT, 18, 10),
		);

		let result = TokenGatewayInspector::handle_timeout(&post);
		println!("{result:?}");
		assert!(result.is_ok());

		let inflow = pallet_token_gateway_inspector::InflowBalances::<Test>::get(
			StateMachine::Kusama(100),
			asset_id,
		);

		assert_eq!(convert_to_erc20(SEND_AMOUNT, 18, 10), inflow);
	});
}

#[test]
fn receiving_remote_asset_creation() {
	new_test_ext().execute_with(|| {
		let asset_metadata = AssetMetadata {
			name: "USDC".as_bytes().to_vec().try_into().unwrap(),
			symbol: "USDC".as_bytes().to_vec().try_into().unwrap(),
			decimals: 6,
		};

		let body: SolAssetMetadata = asset_metadata.clone().try_into().unwrap();

		let post = PostRequest {
			source: StateMachine::Polkadot(3367),
			dest: StateMachine::Kusama(100),
			nonce: 0,
			from: pallet_token_governor::PALLET_ID.to_vec(),
			to: token_gateway_id().0.to_vec(),
			timeout_timestamp: 0,
			body: body.encode_request(),
		};

		let module = TokenGateway::default();
		let res = module.on_accept(post);
		println!("{res:?}");
		assert!(res.is_ok());
		let local_asset_id =
			AssetIdFactory::create_asset_id(asset_metadata.symbol.to_vec()).unwrap();
		let asset = pallet_token_gateway::SupportedAssets::<Test>::get(local_asset_id).unwrap();
		// For the test we use the same asset id construction for local and token gateway, they
		// should be equal
		assert_eq!(local_asset_id, asset);
	})
}

#[test]
fn dispatching_remote_asset_creation() {
	new_test_ext().execute_with(|| {
		let asset_map = AssetMap::<H256> {
			local_id: None,
			reg: ERC6160AssetRegistration {
				name: "MOODENG".as_bytes().to_vec().try_into().unwrap(),
				symbol: "MDG".as_bytes().to_vec().try_into().unwrap(),
				chains: vec![ChainWithSupply { chain: StateMachine::Evm(97), supply: None }],
			},
		};

		let reg = AssetRegistration { assets: vec![asset_map].try_into().unwrap() };

		TokenGateway::create_erc6160_asset(RuntimeOrigin::root(), reg).unwrap();
		let local_asset_id = AssetIdFactory::create_asset_id("MDG".as_bytes().to_vec()).unwrap();
		let asset = pallet_token_gateway::SupportedAssets::<Test>::get(local_asset_id).unwrap();
		// For the test we use the same asset id construction for local and token gateway, they
		// should be equal
		assert_eq!(local_asset_id, asset);
	})
}
