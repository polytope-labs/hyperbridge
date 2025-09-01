#![cfg(test)]
use polkadot_sdk::*;

use alloy_sol_types::SolValue;
use codec::Encode;
use ismp::{
	host::StateMachine,
	router::{PostRequest, Request, Timeout},
};
use pallet_token_gateway::{
	impls::convert_to_erc20,
	types::{AssetRegistration, Body, BodyWithCall, SubstrateCalldata, TeleportParams},
};

use sp_core::{ByteArray, Get, Pair, H160, H256, U256};

use crate::runtime::ALICE;
use sp_runtime::{AccountId32, MultiSignature};
use token_gateway_primitives::{GatewayAssetRegistration, PALLET_TOKEN_GATEWAY_ID};

use crate::runtime::{
	new_test_ext, NativeAssetId, RuntimeOrigin, Test, TokenGateway, TokenGatewayInspector,
	INITIAL_BALANCE,
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
						let bytes = convert_to_erc20(SEND_AMOUNT, 18, 10).to_big_endian();
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
			source: StateMachine::Evm(97),
			dest: StateMachine::Evm(1),
			nonce: 0,
			from: H160::zero().0.to_vec(),
			to: H160::zero().0.to_vec(),
			timeout_timestamp: 1000,
			body: {
				let body = Body {
					amount: {
						let bytes = convert_to_erc20(SEND_AMOUNT, 18, 10).to_big_endian();
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
			from: PALLET_TOKEN_GATEWAY_ID.to_vec(),
			to: H160::zero().0.to_vec(),
			timeout_timestamp: 1000,
			body: {
				let body = Body {
					amount: {
						let bytes = convert_to_erc20(SEND_AMOUNT, 18, 10).to_big_endian();
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
						let bytes = convert_to_erc20(SEND_AMOUNT, 18, 10).to_big_endian();
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
			from: PALLET_TOKEN_GATEWAY_ID.to_vec(),
			to: H160::zero().0.to_vec(),
			timeout_timestamp: 1000,
			body: {
				let body = Body {
					amount: {
						let bytes = convert_to_erc20(SEND_AMOUNT, 18, 10).to_big_endian();
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
fn dispatching_remote_asset_creation() {
	new_test_ext().execute_with(|| {
		let local_asset_id = sp_io::hashing::keccak_256("MDG".as_bytes()).into();
		let reg = AssetRegistration::<H256> {
			local_id: local_asset_id,
			reg: GatewayAssetRegistration {
				name: "MOODENG".as_bytes().to_vec().try_into().unwrap(),
				symbol: "MDG".as_bytes().to_vec().try_into().unwrap(),
				chains: vec![StateMachine::Evm(1)],
				minimum_balance: None,
			},
			native: true,
			precision: vec![(StateMachine::Evm(1), 18)].into_iter().collect(),
		};

		TokenGateway::create_erc6160_asset(RuntimeOrigin::signed(ALICE), reg).unwrap();

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

		let substrate_data = SubstrateCalldata { signature: Some(multisignature), runtime_call };

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
						let bytes = convert_to_erc20(SEND_AMOUNT, 18, 10).to_big_endian();
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

#[test]
fn should_register_asset_locally() {
	new_test_ext().execute_with(|| {
		let symbol = "LOCAL".as_bytes();
		let local_asset_id: H256 = sp_io::hashing::keccak_256(symbol).into();
		let asset_id: H256 = sp_io::hashing::keccak_256(symbol).into();

		let reg = AssetRegistration::<H256> {
			local_id: local_asset_id,
			reg: GatewayAssetRegistration {
				name: "LOCAL_ASSET".as_bytes().to_vec().try_into().unwrap(),
				symbol: symbol.to_vec().try_into().unwrap(),
				chains: vec![StateMachine::Evm(1)],
				minimum_balance: None,
			},
			native: true,
			precision: vec![(StateMachine::Evm(1), 18)].into_iter().collect(),
		};

		TokenGateway::register_asset_locally(RuntimeOrigin::signed(ALICE), reg).unwrap();

		let registered_asset_id =
			pallet_token_gateway::SupportedAssets::<Test>::get(local_asset_id).unwrap();
		assert_eq!(registered_asset_id, asset_id);

		let is_native = pallet_token_gateway::NativeAssets::<Test>::get(local_asset_id);
		assert!(is_native);

		let reverse_lookup_id = pallet_token_gateway::LocalAssets::<Test>::get(asset_id).unwrap();
		assert_eq!(reverse_lookup_id, local_asset_id);

		let precision =
			pallet_token_gateway::Precisions::<Test>::get(local_asset_id, StateMachine::Evm(1))
				.unwrap();
		assert_eq!(precision, 18);
	})
}
