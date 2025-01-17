#![cfg(test)]

use alloy_sol_types::SolValue;
use codec::Encode;
use ismp::{
	host::StateMachine,
	router::{PostRequest, Request, Timeout},
};
use pallet_token_gateway::{
	impls::convert_to_erc20, AssetRegistration, Body, BodyWithCall, CreateAssetId,
	SubstrateCalldata, TeleportParams,
};

use sp_core::{ByteArray, Get, Pair, H160, H256, U256};

use sp_runtime::{AccountId32, MultiSignature};
use token_gateway_primitives::{
	token_gateway_id, token_governor_id, AssetMetadata, GatewayAssetRegistration,
};
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
			call_data: None,
			redeem: false,
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
			call_data: None,
			redeem: false,
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
			call_data: None,
			redeem: false,
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
			minimum_balance: None,
		};

		let post = PostRequest {
			source: StateMachine::Polkadot(3367),
			dest: StateMachine::Kusama(100),
			nonce: 0,
			from: token_governor_id(),
			to: token_gateway_id().0.to_vec(),
			timeout_timestamp: 0,
			body: asset_metadata.encode(),
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
		let local_asset_id = AssetIdFactory::create_asset_id("MDG".as_bytes().to_vec()).unwrap();
		let reg = AssetRegistration::<H256> {
			local_id: local_asset_id,
			reg: GatewayAssetRegistration {
				name: "MOODENG".as_bytes().to_vec().try_into().unwrap(),
				symbol: "MDG".as_bytes().to_vec().try_into().unwrap(),
				chains: vec![StateMachine::Evm(97)],
				minimum_balance: None,
			},
		};

		TokenGateway::create_erc6160_asset(RuntimeOrigin::signed(ALICE), reg, true).unwrap();

		let asset = pallet_token_gateway::SupportedAssets::<Test>::get(local_asset_id).unwrap();
		// For the test we use the same asset id construction for local and token gateway, they
		// should be equal
		assert_eq!(local_asset_id, asset);
	})
}

#[test]
fn should_receive_asset_with_call_correctly() {
	new_test_ext().execute_with(|| {
		let params = TeleportParams {
			asset_id: NativeAssetId::get(),
			destination: StateMachine::Evm(1),
			recepient: H256::random(),
			timeout: 0,
			amount: SEND_AMOUNT,
			token_gateway: H160::zero().0.to_vec(),
			relayer_fee: Default::default(),
			call_data: None,
			redeem: false,
		};

		TokenGateway::teleport(RuntimeOrigin::signed(ALICE), params).unwrap();

		let new_balance = pallet_balances::Pallet::<Test>::free_balance(ALICE);

		assert_eq!(new_balance, INITIAL_BALANCE - SEND_AMOUNT);

		let final_recepient = H256::random();

		let runtime_call =
			crate::runtime::RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
				dest: final_recepient.0.into(),
				value: SEND_AMOUNT.into(),
			})
			.encode();

		let (pair, ..) = sp_core::sr25519::Pair::generate();
		let beneficiary = pair.public().0;
		let payload = (0u64, runtime_call.clone()).encode();

		let message = sp_core::keccak_256(&payload);

		let raw_signature = pair.sign(&message);

		let multisignature = MultiSignature::Sr25519(raw_signature).encode();

		let substrate_data = SubstrateCalldata { signature: multisignature, runtime_call };

		let module = TokenGateway::default();
		let post = PostRequest {
			source: StateMachine::Evm(1),
			dest: StateMachine::Kusama(100),
			nonce: 0,
			from: H160::zero().0.to_vec(),
			to: H160::zero().0.to_vec(),
			timeout_timestamp: 1000,
			body: {
				let body = BodyWithCall {
					amount: {
						let mut bytes = [0u8; 32];
						// Module callback will convert to ten decimals
						convert_to_erc20(SEND_AMOUNT, 18, 10).to_big_endian(&mut bytes);
						alloy_primitives::U256::from_be_bytes(bytes)
					},
					asset_id: H256::zero().0.into(),
					redeem: false,
					from: alloy_primitives::B256::from_slice(ALICE.as_slice()),
					to: alloy_primitives::B256::from_slice(beneficiary.as_slice()),
					data: substrate_data.encode().into(),
				};

				let encoded = vec![vec![0], BodyWithCall::abi_encode(&body)].concat();
				encoded
			},
		};

		module.on_accept(post.clone()).unwrap();
		let recipient: AccountId32 = final_recepient.0.into();
		let new_balance = pallet_balances::Pallet::<Test>::free_balance(recipient);

		assert_eq!(new_balance, SEND_AMOUNT);

		// try to replay call
		let res = module.on_accept(post);
		assert!(res.is_err());
	});
}
