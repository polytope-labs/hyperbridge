#![cfg(test)]
use polkadot_sdk::*;

use alloy_sol_types::SolValue;
use codec::Encode;
use ismp::{
	host::StateMachine,
	module::IsmpModule,
	router::{PostRequest, Request},
};
use pallet_hyper_fungible_token::{
	impls::convert_to_erc20,
	types::{Message, SendParams, SubstrateCalldata},
};

use frame_support::BoundedVec;
use sp_core::{ByteArray, Get, Pair, H160, H256};
use sp_runtime::{AccountId32, MultiSignature};

use crate::runtime::{
	new_test_ext, HftNativeAssetId, HyperFungibleToken, RuntimeOrigin, Test, ALICE, BOB,
	INITIAL_BALANCE,
};

const SEND_AMOUNT: u128 = 1_000_000_000_000;

fn hft_contract() -> Vec<u8> {
	vec![0xABu8; 20]
}

#[test]
fn should_send_asset_correctly() {
	new_test_ext().execute_with(|| {
		let params = SendParams {
			asset_id: HftNativeAssetId::get(),
			destination: StateMachine::Evm(1),
			recipient: BoundedVec::try_from(BOB.as_slice().to_vec()).unwrap(),
			timeout: 0,
			amount: SEND_AMOUNT,
			relayer_fee: Default::default(),
			call_data: None,
		};

		HyperFungibleToken::send(RuntimeOrigin::signed(ALICE), params).unwrap();

		let new_balance = pallet_balances::Pallet::<Test>::free_balance(ALICE);
		assert_eq!(new_balance, INITIAL_BALANCE - SEND_AMOUNT);
	})
}

#[test]
fn should_receive_asset_correctly() {
	new_test_ext().execute_with(|| {
		// First send to escrow funds in pallet account
		let params = SendParams {
			asset_id: HftNativeAssetId::get(),
			destination: StateMachine::Evm(1),
			recipient: BoundedVec::try_from(BOB.as_slice().to_vec()).unwrap(),
			timeout: 0,
			amount: SEND_AMOUNT,
			relayer_fee: Default::default(),
			call_data: None,
		};

		HyperFungibleToken::send(RuntimeOrigin::signed(ALICE), params).unwrap();
		let balance_after_send = pallet_balances::Pallet::<Test>::free_balance(ALICE);
		assert_eq!(balance_after_send, INITIAL_BALANCE - SEND_AMOUNT);

		// Simulate receiving tokens from EVM
		let module = HyperFungibleToken::default();
		let post = PostRequest {
			source: StateMachine::Evm(1),
			dest: StateMachine::Kusama(100),
			nonce: 0,
			from: hft_contract(),
			to: pallet_hyper_fungible_token::PALLET_ID.to_bytes(),
			timeout_timestamp: 1000,
			body: {
				let msg = Message {
					from: alloy_primitives::Bytes::from(vec![0x11u8; 20]),
					to: alloy_primitives::Bytes::from(ALICE.as_slice().to_vec()),
					amount: {
						let bytes = convert_to_erc20(SEND_AMOUNT, 18, 10).to_big_endian();
						alloy_primitives::U256::from_be_bytes(bytes)
					},
					data: alloy_primitives::Bytes::default(),
				};
				Message::abi_encode(&msg)
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
		// Send to escrow
		let params = SendParams {
			asset_id: HftNativeAssetId::get(),
			destination: StateMachine::Evm(1),
			recipient: BoundedVec::try_from(BOB.as_slice().to_vec()).unwrap(),
			timeout: 0,
			amount: SEND_AMOUNT,
			relayer_fee: Default::default(),
			call_data: None,
		};

		HyperFungibleToken::send(RuntimeOrigin::signed(ALICE), params).unwrap();
		let balance_after_send = pallet_balances::Pallet::<Test>::free_balance(ALICE);
		assert_eq!(balance_after_send, INITIAL_BALANCE - SEND_AMOUNT);

		// Simulate timeout — the `to` field is the token contract on the dest chain
		let module = HyperFungibleToken::default();
		let post = PostRequest {
			source: StateMachine::Kusama(100),
			dest: StateMachine::Evm(1),
			nonce: 0,
			from: pallet_hyper_fungible_token::PALLET_ID.to_bytes(),
			to: hft_contract(),
			timeout_timestamp: 1000,
			body: {
				let msg = Message {
					from: alloy_primitives::Bytes::from(ALICE.as_slice().to_vec()),
					to: alloy_primitives::Bytes::from(BOB.as_slice().to_vec()),
					amount: {
						let bytes = convert_to_erc20(SEND_AMOUNT, 18, 10).to_big_endian();
						alloy_primitives::U256::from_be_bytes(bytes)
					},
					data: alloy_primitives::Bytes::default(),
				};
				Message::abi_encode(&msg)
			},
		};

		module.on_timeout(Request::Post(post)).unwrap();
		let new_balance = pallet_balances::Pallet::<Test>::free_balance(ALICE);
		assert_eq!(new_balance, INITIAL_BALANCE);
	});
}

#[test]
fn should_reject_unknown_source_contract() {
	new_test_ext().execute_with(|| {
		let module = HyperFungibleToken::default();
		let post = PostRequest {
			source: StateMachine::Evm(1),
			dest: StateMachine::Kusama(100),
			nonce: 0,
			from: vec![0xFFu8; 20], // unknown contract
			to: pallet_hyper_fungible_token::PALLET_ID.to_bytes(),
			timeout_timestamp: 1000,
			body: {
				let msg = Message {
					from: alloy_primitives::Bytes::from(vec![0x11u8; 20]),
					to: alloy_primitives::Bytes::from(ALICE.as_slice().to_vec()),
					amount: alloy_primitives::U256::from(1000u64),
					data: alloy_primitives::Bytes::default(),
				};
				Message::abi_encode(&msg)
			},
		};

		let result = module.on_accept(post);
		assert!(result.is_err());
	});
}

#[test]
fn should_reject_unsupported_chain() {
	new_test_ext().execute_with(|| {
		let params = SendParams {
			asset_id: HftNativeAssetId::get(),
			destination: StateMachine::Evm(42), // not registered
			recipient: BoundedVec::try_from(BOB.as_slice().to_vec()).unwrap(),
			timeout: 0,
			amount: SEND_AMOUNT,
			relayer_fee: Default::default(),
			call_data: None,
		};

		let result = HyperFungibleToken::send(RuntimeOrigin::signed(ALICE), params);
		assert!(result.is_err());
	});
}

#[test]
fn should_register_and_update_token() {
	use pallet_hyper_fungible_token::types::{ChainConfig, TokenRegistration, TokenUpdate};
	use std::collections::BTreeMap;

	new_test_ext().execute_with(|| {
		let asset_id: H256 = sp_io::hashing::keccak_256(b"NEW_TOKEN").into();
		let contract = vec![0xEEu8; 20];

		let mut chains = BTreeMap::new();
		chains.insert(
			StateMachine::Evm(42),
			ChainConfig { token_contract: H160::from_slice(&contract), decimals: 6 },
		);

		let reg = TokenRegistration { local_id: asset_id, native: false, chains };

		HyperFungibleToken::register_token(RuntimeOrigin::signed(ALICE), reg).unwrap();

		// Verify storage
		assert_eq!(
			pallet_hyper_fungible_token::TokenContracts::<Test>::get(
				StateMachine::Evm(42),
				asset_id
			)
			.unwrap(),
			contract
		);
		assert_eq!(
			pallet_hyper_fungible_token::ContractToAsset::<Test>::get(
				StateMachine::Evm(42),
				&contract
			)
			.unwrap(),
			asset_id
		);
		assert_eq!(
			pallet_hyper_fungible_token::Precisions::<Test>::get(asset_id, StateMachine::Evm(42))
				.unwrap(),
			6
		);
		assert!(!pallet_hyper_fungible_token::NativeAssets::<Test>::get(asset_id));

		// Update: remove chain
		let update = TokenUpdate {
			asset_id,
			add_chains: BTreeMap::new(),
			remove_chains: vec![StateMachine::Evm(42)],
		};

		HyperFungibleToken::update_token(RuntimeOrigin::signed(ALICE), update).unwrap();

		assert!(pallet_hyper_fungible_token::TokenContracts::<Test>::get(
			StateMachine::Evm(42),
			asset_id
		)
		.is_none());
		assert!(pallet_hyper_fungible_token::ContractToAsset::<Test>::get(
			StateMachine::Evm(42),
			&contract
		)
		.is_none());
	});
}

#[test]
fn should_receive_asset_with_calldata() {
	new_test_ext().execute_with(|| {
		// First send to escrow funds
		let params = SendParams {
			asset_id: HftNativeAssetId::get(),
			destination: StateMachine::Evm(1),
			recipient: BoundedVec::try_from(BOB.as_slice().to_vec()).unwrap(),
			timeout: 0,
			amount: SEND_AMOUNT,
			relayer_fee: Default::default(),
			call_data: None,
		};

		HyperFungibleToken::send(RuntimeOrigin::signed(ALICE), params).unwrap();

		// Build a runtime call: transfer from beneficiary to a final recipient
		let final_recipient = AccountId32::new([5u8; 32]);
		let runtime_call =
			crate::runtime::RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
				dest: final_recipient.clone(),
				value: SEND_AMOUNT,
			})
			.encode();

		// Sign with sr25519
		let (pair, ..) = sp_core::sr25519::Pair::generate();
		let beneficiary_bytes = pair.public().0;
		let payload = (0u64, runtime_call.clone()).encode();
		let message_hash = sp_core::keccak_256(&payload);
		let raw_signature = pair.sign(&message_hash);
		let multisignature = MultiSignature::Sr25519(raw_signature).encode();

		let substrate_data = SubstrateCalldata { signature: Some(multisignature), runtime_call };

		let module = HyperFungibleToken::default();
		let post = PostRequest {
			source: StateMachine::Evm(1),
			dest: StateMachine::Kusama(100),
			nonce: 0,
			from: hft_contract(),
			to: pallet_hyper_fungible_token::PALLET_ID.to_bytes(),
			timeout_timestamp: 1000,
			body: {
				let msg = Message {
					from: alloy_primitives::Bytes::from(vec![0x11u8; 20]),
					to: alloy_primitives::Bytes::from(beneficiary_bytes.to_vec()),
					amount: {
						let bytes = convert_to_erc20(SEND_AMOUNT, 18, 10).to_big_endian();
						alloy_primitives::U256::from_be_bytes(bytes)
					},
					data: alloy_primitives::Bytes::from(substrate_data.encode()),
				};
				Message::abi_encode(&msg)
			},
		};

		module.on_accept(post.clone()).unwrap();

		// The calldata transferred tokens from beneficiary to final_recipient
		let final_balance = pallet_balances::Pallet::<Test>::free_balance(final_recipient);
		assert_eq!(final_balance, SEND_AMOUNT);

		// Replay should fail (nonce incremented)
		let result = module.on_accept(post);
		assert!(result.is_err());
	});
}
