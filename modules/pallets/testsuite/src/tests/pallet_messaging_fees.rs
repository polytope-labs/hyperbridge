#![cfg(test)]

use codec::{Decode, Encode};
use polkadot_sdk::{
	frame_support::{
		self,
		dispatch::PerDispatchClass,
		pallet_prelude::StorageVersion,
		traits::{
			fungible::{Inspect, Mutate},
			Get, OnRuntimeUpgrade,
		},
	},
	frame_system::{self, limits::BlockWeights},
	pallet_session::SessionHandler,
	sp_io,
	sp_runtime::{
		traits::{AccountIdConversion, OpaqueKeys},
		KeyTypeId, Weight,
	},
};

use scale_info::TypeInfo;
use sp_core::{crypto::AccountId32, keccak_256, sr25519, ByteArray, Pair, H256, U256};

use hyperbridge_client_machine::OnRequestProcessed;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	host::StateMachine,
	messaging::{hash_request, Message, MessageWithWeight, Proof, RequestMessage},
	router::{PostRequest, Request},
};
use pallet_ismp::fee_handler::FeeHandler;
use pallet_ismp_host_executive::{EvmHostParam, HostParam, PerByteFee};
use pallet_ismp_relayer::withdrawal::Signature;
use pallet_messaging_fees::{migrations, IncentivesManager, TotalBytesProcessed};

use crate::{
	runtime::{
		new_test_ext, Assets, Balances, ReputationAssetId, RuntimeOrigin, Test, TreasuryAccount,
		UNIT,
	},
	tests::common::setup_relayer_and_asset,
};
use pallet_messaging_fees::types::WeightInfo;
use std::cmp::Ordering;

fn setup_balances(relayer_account: &AccountId32, treasury_account: &AccountId32) {
	setup_relayer_and_asset(&relayer_account);

	assert_eq!(Balances::balance(relayer_account), 0);
	Balances::mint_into(relayer_account, 1000 * UNIT).unwrap();
	assert_eq!(Balances::balance(relayer_account), 1000 * UNIT);

	assert_eq!(Balances::balance(treasury_account), 0);
	Balances::mint_into(treasury_account, 20000 * UNIT).unwrap();
}

fn setup_host_params(source_chain: StateMachine, dest_chain: StateMachine) {
	let host_params = HostParam::EvmHostParam(EvmHostParam {
		per_byte_fees: vec![PerByteFee {
			state_id: H256(keccak_256(&dest_chain.to_string().as_bytes())),
			per_byte_fee: U256::from(10_000_000_000_000_000u128),
		}]
		.try_into()
		.unwrap(),
		..Default::default()
	});

	pallet_ismp_host_executive::HostParams::<Test>::insert(source_chain, host_params);
	pallet_ismp_host_executive::FeeTokenDecimals::<Test>::insert(source_chain, 18);
}

fn create_request_message(
	source_chain: StateMachine,
	dest_chain: StateMachine,
	relayer_pair: &sr25519::Pair,
	body: &Vec<u8>,
) -> MessageWithWeight {
	let post_request = PostRequest {
		source: source_chain,
		dest: dest_chain,
		nonce: 0,
		from: vec![1; 32],
		to: vec![2; 32],
		timeout_timestamp: 100,
		body: body.clone(),
	};

	let requests = vec![post_request];
	let signed_data = keccak_256(&requests.encode());
	let signature = relayer_pair.sign(&signed_data);
	let signature = Signature::Sr25519 {
		public_key: relayer_pair.public().to_raw_vec(),
		signature: signature.to_raw_vec(),
	};

	let request_message = RequestMessage {
		requests,
		proof: Proof {
			height: StateMachineHeight {
				id: StateMachineId { state_id: source_chain, consensus_state_id: *b"mock" },
				height: 1,
			},
			proof: vec![],
		},
		signer: signature.encode(),
	};

	let request_message =
		MessageWithWeight { message: Message::Request(request_message), weight: Weight::zero() };

	request_message
}

fn create_bad_request_message(
	source_chain: StateMachine,
	dest_chain: StateMachine,
	relayer_pair: &sr25519::Pair,
	evil_pair: &sr25519::Pair,
) -> MessageWithWeight {
	let post_request = PostRequest {
		source: source_chain,
		dest: dest_chain,
		nonce: 0,
		from: vec![1; 32],
		to: vec![2; 32],
		timeout_timestamp: 100,
		body: vec![0; 100],
	};

	let requests = vec![post_request];
	let signed_data = keccak_256(&requests.encode());
	let signature = relayer_pair.sign(&signed_data);
	let signer_tuple = (evil_pair.public(), signature);

	let request_message = RequestMessage {
		requests,
		proof: Proof {
			height: StateMachineHeight {
				id: StateMachineId { state_id: source_chain, consensus_state_id: *b"mock" },
				height: 1,
			},
			proof: vec![],
		},
		signer: signer_tuple.encode(),
	};

	let request_message =
		MessageWithWeight { message: Message::Request(request_message), weight: Weight::zero() };

	request_message
}

#[test]
fn test_incentivize_relayer_for_request_message() {
	new_test_ext().execute_with(|| {
		let relayer_pair = sr25519::Pair::from_seed(&H256::random().0);
		let relayer_account: AccountId32 = relayer_pair.public().into();
		let treasury_account = TreasuryAccount::get().into_account_truncating();
		let source_chain = StateMachine::Evm(2000);
		let dest_chain = StateMachine::Evm(3000);

		setup_balances(&relayer_account, &treasury_account);
		setup_host_params(source_chain, dest_chain);

		pallet_messaging_fees::Pallet::<Test>::set_supported_route(
			RuntimeOrigin::root(),
			source_chain,
		)
		.unwrap();

		pallet_messaging_fees::Pallet::<Test>::set_supported_route(
			RuntimeOrigin::root(),
			dest_chain,
		)
		.unwrap();

		let body = vec![0; 100];
		let request_message =
			create_request_message(source_chain, dest_chain, &relayer_pair, &body);

		assert_eq!(TotalBytesProcessed::<Test>::get(), 0);

		let initial_relayer_balance = Balances::balance(&relayer_account);
		let initial_relayer_reputation_asset_balance =
			Assets::balance(ReputationAssetId::get(), &relayer_account);

		let _ = pallet_messaging_fees::Pallet::<Test>::on_executed(vec![request_message], vec![])
			.unwrap();
		dbg!(initial_relayer_balance);
		dbg!(Balances::balance(&relayer_account));

		assert!(Balances::balance(&relayer_account) > initial_relayer_balance);
		assert_eq!(TotalBytesProcessed::<Test>::get(), body.len() as u32);
		assert!(
			Assets::balance(ReputationAssetId::get(), &relayer_account) >
				initial_relayer_reputation_asset_balance
		);
	});
}

#[test]
fn test_charge_relayer_when_target_size_is_exceeded() {
	new_test_ext().execute_with(|| {
		let relayer_pair = sr25519::Pair::from_seed(&H256::random().0);
		let relayer_account: AccountId32 = relayer_pair.public().into();
		let treasury_account = TreasuryAccount::get().into_account_truncating();
		let source_chain = StateMachine::Evm(2000);
		let dest_chain = StateMachine::Evm(3000);

		setup_balances(&relayer_account, &treasury_account);
		setup_host_params(source_chain, dest_chain);

		pallet_messaging_fees::Pallet::<Test>::set_supported_route(
			RuntimeOrigin::root(),
			source_chain,
		)
		.unwrap();

		pallet_messaging_fees::Pallet::<Test>::set_supported_route(
			RuntimeOrigin::root(),
			dest_chain,
		)
		.unwrap();

		pallet_messaging_fees::Pallet::<Test>::set_target_message_size(
			RuntimeOrigin::root(),
			20000u32,
		)
		.unwrap();

		let initial_relayer_balance = Balances::balance(&relayer_account);
		let initial_bytes_processed = TotalBytesProcessed::<Test>::get();
		let target_size: u32 = pallet_messaging_fees::TargetMessageSize::<Test>::get().unwrap();
		TotalBytesProcessed::<Test>::put(target_size + 1);

		let body = vec![0; 100];
		let request_message =
			create_request_message(source_chain, dest_chain, &relayer_pair, &body);

		let _ = pallet_messaging_fees::Pallet::<Test>::on_executed(vec![request_message], vec![]);
		let current_relayer_balance = Balances::balance(&relayer_account);
		dbg!(initial_relayer_balance);
		dbg!(current_relayer_balance);
		assert!(current_relayer_balance < initial_relayer_balance);
		assert!(initial_bytes_processed < TotalBytesProcessed::<Test>::get());
	});
}
#[test]
fn test_skip_incentivizing_for_unsupported_route_but_fees_should_still_be_paid() {
	new_test_ext().execute_with(|| {
		let relayer_pair = sr25519::Pair::from_seed(&H256::random().0);
		let relayer_account: AccountId32 = relayer_pair.public().into();
		let treasury_account = TreasuryAccount::get().into_account_truncating();
		let source_chain = StateMachine::Evm(2000);
		let dest_chain = StateMachine::Evm(3000);

		setup_balances(&relayer_account, &treasury_account);
		setup_host_params(source_chain, dest_chain);

		let body = vec![0; 100];
		let request_message =
			create_request_message(source_chain, dest_chain, &relayer_pair, &body);

		let initial_relayer_balance = Balances::balance(&relayer_account);
		let _ = pallet_messaging_fees::Pallet::<Test>::on_executed(vec![request_message], vec![]);
		let current_relayer_balance = Balances::balance(&relayer_account);

		assert!(current_relayer_balance < initial_relayer_balance);
		assert_eq!(TotalBytesProcessed::<Test>::get(), 100);
	});
}

#[test]
fn test_skip_incentivizing_for_invalid_signature() {
	new_test_ext().execute_with(|| {
		let relayer_pair = sr25519::Pair::from_seed(&H256::random().0);
		let relayer_account: AccountId32 = relayer_pair.public().into();
		let evil_pair = sr25519::Pair::from_seed(&H256::random().0);
		let treasury_account = TreasuryAccount::get().into_account_truncating();
		let source_chain = StateMachine::Evm(2000);
		let dest_chain = StateMachine::Evm(3000);

		setup_balances(&relayer_account, &treasury_account);
		setup_host_params(source_chain, dest_chain);

		pallet_messaging_fees::Pallet::<Test>::set_supported_route(
			RuntimeOrigin::root(),
			source_chain,
		)
		.unwrap();

		pallet_messaging_fees::Pallet::<Test>::set_supported_route(
			RuntimeOrigin::root(),
			dest_chain,
		)
		.unwrap();

		let request_message =
			create_bad_request_message(source_chain, dest_chain, &relayer_pair, &evil_pair);

		let _ = pallet_messaging_fees::Pallet::<Test>::on_executed(vec![request_message], vec![]);

		assert_eq!(Balances::balance(&relayer_account), 1000 * UNIT);
		assert_eq!(TotalBytesProcessed::<Test>::get(), 0);
	});
}

#[test]
fn test_reward_decreases_as_messages_increase() {
	new_test_ext().execute_with(|| {
		let relayer_pair = sr25519::Pair::from_seed(&H256::random().0);
		let relayer_account: AccountId32 = relayer_pair.public().into();
		let treasury_account = TreasuryAccount::get().into_account_truncating();
		let source_chain = StateMachine::Evm(2000);
		let dest_chain = StateMachine::Evm(3000);

		setup_balances(&relayer_account, &treasury_account);
		setup_host_params(source_chain, dest_chain);

		pallet_messaging_fees::Pallet::<Test>::set_supported_route(
			RuntimeOrigin::root(),
			source_chain,
		)
		.unwrap();

		pallet_messaging_fees::Pallet::<Test>::set_supported_route(
			RuntimeOrigin::root(),
			dest_chain,
		)
		.unwrap();

		let mut last_reward = u128::MAX;
		let mut previous_balance = Balances::balance(&relayer_account);
		let number_of_messages = 5;

		for i in 0..number_of_messages {
			let body = vec![0; 100];
			let request_message =
				create_request_message(source_chain, dest_chain, &relayer_pair, &body);
			let _ =
				pallet_messaging_fees::Pallet::<Test>::on_executed(vec![request_message], vec![]);

			let current_balance = Balances::balance(&relayer_account);
			let reward_received = current_balance.saturating_sub(previous_balance);

			println!(
				"Message {}: TotalBytes={}, Reward Received={}",
				i + 1,
				TotalBytesProcessed::<Test>::get(),
				reward_received
			);

			assert!(reward_received < last_reward);

			last_reward = reward_received;
			previous_balance = current_balance;
		}
	});
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Encode, Decode, TypeInfo)]
pub struct MockOpaqueKeys;

impl OpaqueKeys for MockOpaqueKeys {
	type KeyTypeIdProviders = ();

	fn key_ids() -> &'static [KeyTypeId] {
		todo!()
	}

	fn get_raw(&self, _i: KeyTypeId) -> &[u8] {
		todo!()
	}
}

#[test]
fn test_on_new_session_resets_state() {
	new_test_ext().execute_with(|| {
		TotalBytesProcessed::<Test>::put(500);
		assert_eq!(TotalBytesProcessed::<Test>::get(), 500);

		pallet_messaging_fees::Pallet::<Test>::reset_incentives();

		assert_eq!(TotalBytesProcessed::<Test>::get(), 0);
	});
}

#[test]
fn test_reward_curve_visualization_to_one_megabyte() {
	new_test_ext().execute_with(|| {
		const ONE_MEGABYTE: u32 = 1_048_576;
		const BASE_REWARD: u128 = 1_000_000_000;
		const TARGET_SIZE: u32 = ONE_MEGABYTE;

		println!("\n--- Reward Curve Visualization ---");
		println!("Base Reward: {}, Target Size: {} bytes (1 MB)", BASE_REWARD, TARGET_SIZE);
		println!("{:<20} | {:<20} | {}", "Progress", "Total Bytes", "Calculated Reward");
		println!("{:-<22}|{:-<22}|{:-<22}", "", "", "");

		let mut last_reward = u128::MAX;

		for i in 0..=10 {
			let percentage = i * 10;
			let total_bytes = (TARGET_SIZE as u64 * percentage as u64 / 100) as u32;

			let reward = pallet_messaging_fees::Pallet::<Test>::calculate_reward(
				total_bytes,
				TARGET_SIZE,
				BASE_REWARD,
			)
			.unwrap();

			println!("{:<20} | {:<20} | {}", format!("{}%", percentage), total_bytes, reward);

			assert!(reward <= last_reward);
			last_reward = reward;
		}
	});
}

#[test]
fn test_protocol_fee_accumulation() {
	new_test_ext().execute_with(|| {
		let relayer_pair = sr25519::Pair::from_seed(&H256::random().0);
		let source_chain = StateMachine::Substrate(*b"dock");
		let dest_chain = StateMachine::Evm(1000);
		let request = PostRequest {
			source: source_chain,
			dest: dest_chain,
			nonce: 0,
			from: vec![1; 32],
			to: vec![2; 32],
			timeout_timestamp: 100,
			body: vec![0; 100],
		};
		let commitment = hash_request::<<Test as pallet_messaging_fees::Config>::IsmpHost>(
			&Request::Post(request.clone()),
		);
		let body = vec![0; 100];
		let request_message =
			create_request_message(source_chain, dest_chain, &relayer_pair, &body);
		let fee = 1_000_000u128;

		setup_host_params(source_chain, dest_chain);

		pallet_messaging_fees::Pallet::<Test>::note_request_fee(commitment, fee);
		assert!(pallet_messaging_fees::CommitmentFees::<Test>::get(commitment).is_some());

		let _ = pallet_messaging_fees::Pallet::<Test>::on_executed(vec![request_message], vec![]);

		assert!(pallet_messaging_fees::CommitmentFees::<Test>::get(commitment).is_none());

		let relayer_address: Vec<u8> = relayer_pair.public().0.into();
		let expected_fee_u256 = U256::from(fee);
		assert_eq!(
			pallet_ismp_relayer::Fees::<Test>::get(source_chain, relayer_address),
			expected_fee_u256
		);
	});
}

#[cfg(test)]
mod migration_tests {
	use super::*;
	use frame_support::weights::WeightMeter;
	use polkadot_sdk::{frame_support::migrations::SteppedMigration, sp_runtime::Saturating};
	use pallet_messaging_fees::migrations::v1::Migration;
	#[test]
	fn migration_scales_evm_fees_for_32_byte_address_multi_block() {
		new_test_ext().execute_with(|| {
			let evm_chain_1 = StateMachine::Evm(1);
			let relayer_32_byte_1 = vec![1u8; 32];
			let fee_1 = U256::from(100_000_000_000_000_000_000u128);

			let evm_chain_2 = StateMachine::Evm(2);
			let relayer_32_byte_2 = vec![2u8; 32];
			let fee_2 = U256::from(200_000_000_000_000_000_000u128);

			pallet_ismp_relayer::Fees::<Test>::insert(
				evm_chain_1,
				relayer_32_byte_1.clone(),
				fee_1,
			);
			pallet_ismp_host_executive::FeeTokenDecimals::<Test>::insert(&evm_chain_1, 6u8);

			pallet_ismp_relayer::Fees::<Test>::insert(
				evm_chain_2,
				relayer_32_byte_2.clone(),
				fee_2,
			);
			pallet_ismp_host_executive::FeeTokenDecimals::<Test>::insert(&evm_chain_2, 8u8);

			StorageVersion::new(0).put::<pallet_messaging_fees::Pallet<Test>>();

			let one_item_weight =
				<Test as pallet_messaging_fees::Config>::WeightInfo::migrate_evm_fees();

			let mut meter = WeightMeter::with_limit(one_item_weight);
			let cursor_1 = Migration::<Test>::step(None, &mut meter).unwrap();
			assert!(cursor_1.is_some());


			let mut meter = WeightMeter::with_limit(one_item_weight);
			let cursor_2 = Migration::<Test>::step(cursor_1.clone(), &mut meter).unwrap();
			assert!(cursor_2.is_some());
			assert_ne!(cursor_1, cursor_2);


			let mut meter = WeightMeter::with_limit(one_item_weight);
			let cursor_3 = Migration::<Test>::step(cursor_2.clone(), &mut meter).unwrap();
			assert!(cursor_3.is_none());


			let scaling_power_1 = 18u32.saturating_sub(6u32);
			let divisor_1 = U256::from(10u128).pow(U256::from(scaling_power_1));
			let expected_fee_1 = fee_1.checked_div(divisor_1).unwrap();

			let scaling_power_2 = 18u32.saturating_sub(8u32);
			let divisor_2 = U256::from(10u128).pow(U256::from(scaling_power_2));
			let expected_fee_2 = fee_2.checked_div(divisor_2).unwrap();

			assert_eq!(
				pallet_ismp_relayer::Fees::<Test>::get(evm_chain_1, relayer_32_byte_1),
				expected_fee_1
			);
			assert_eq!(
				pallet_ismp_relayer::Fees::<Test>::get(evm_chain_2, relayer_32_byte_2),
				expected_fee_2
			);
		});
	}
}
