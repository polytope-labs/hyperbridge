#![cfg(test)]

use codec::{Decode, Encode};
use polkadot_sdk::{
	frame_support::traits::{
		fungible::{Inspect, Mutate},
		Get,
	},
	pallet_session::SessionHandler,
	sp_runtime::{
		traits::{AccountIdConversion, OpaqueKeys},
		KeyTypeId,
	},
};

use scale_info::TypeInfo;
use sp_core::{crypto::AccountId32, keccak_256, sr25519, Pair, H256, U256};

use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	host::StateMachine,
	messaging::{Message, Proof, RequestMessage},
	router::PostRequest,
};
use pallet_ismp_host_executive::{EvmHostParam, HostParam, PerByteFee};
use pallet_messaging_fees::{Epoch, TotalBytesProcessed};

use crate::runtime::{new_test_ext, Balances, RuntimeOrigin, Test, TreasuryAccount, UNIT};

fn setup_balances(relayer_account: &AccountId32, treasury_account: &AccountId32) {
	assert_eq!(Balances::balance(relayer_account), 0);
	Balances::mint_into(relayer_account, 1000 * UNIT).unwrap();
	assert_eq!(Balances::balance(relayer_account), 1000 * UNIT);

	assert_eq!(Balances::balance(treasury_account), 0);
	Balances::mint_into(treasury_account, 20000 * UNIT).unwrap();
}

fn setup_host_params(dest_chain: StateMachine) {
	let host_params = HostParam::EvmHostParam(EvmHostParam {
		per_byte_fees: vec![PerByteFee {
			state_id: H256(sp_core::keccak_256(&dest_chain.encode())),
			per_byte_fee: U256::from(100_000),
		}]
		.try_into()
		.unwrap(),
		..Default::default()
	});

	pallet_ismp_host_executive::HostParams::<Test>::insert(dest_chain, host_params);
}

fn create_request_message(
	source_chain: StateMachine,
	dest_chain: StateMachine,
	relayer_pair: &sr25519::Pair,
) -> Message {
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
	let signer_tuple = (relayer_pair.public(), signature);

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

	Message::Request(request_message)
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
		setup_host_params(dest_chain);

		pallet_messaging_fees::Pallet::<Test>::set_supported_route(
			RuntimeOrigin::root(),
			source_chain,
			dest_chain,
		)
		.unwrap();

		let request_message = create_request_message(source_chain, dest_chain, &relayer_pair);

		assert_eq!(TotalBytesProcessed::<Test>::get(), 0);

		let _ = pallet_messaging_fees::Pallet::<Test>::on_executed(vec![request_message]);

		let bytes_processed = 1;
		let base_reward = 100_000u128;
		let total_bytes_for_calc = 1;
		let target_size: u32 = <Test as pallet_messaging_fees::Config>::TargetMessageSize::get();

		let expected_reward = pallet_messaging_fees::Pallet::<Test>::calculate_reward(
			total_bytes_for_calc,
			target_size,
			base_reward,
		)
		.unwrap();

		assert_eq!(Balances::balance(&relayer_account), 1000 * UNIT + expected_reward);
		assert_eq!(TotalBytesProcessed::<Test>::get(), bytes_processed);
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
		setup_host_params(dest_chain);

		pallet_messaging_fees::Pallet::<Test>::set_supported_route(
			RuntimeOrigin::root(),
			source_chain,
			dest_chain,
		)
		.unwrap();

		let target_size: u32 = <Test as pallet_messaging_fees::Config>::TargetMessageSize::get();
		TotalBytesProcessed::<Test>::put(target_size + 1);

		let request_message = create_request_message(source_chain, dest_chain, &relayer_pair);

		let _ = pallet_messaging_fees::Pallet::<Test>::on_executed(vec![request_message]);

		let fee_charged = 100_000u128;

		assert_eq!(Balances::balance(&relayer_account), 1000 * UNIT - fee_charged);
		assert_eq!(TotalBytesProcessed::<Test>::get(), target_size + 2);
	});
}

#[test]
fn test_skip_incentivizing_for_unsupported_route() {
	new_test_ext().execute_with(|| {
		let relayer_pair = sr25519::Pair::from_seed(&H256::random().0);
		let relayer_account: AccountId32 = relayer_pair.public().into();
		let treasury_account = TreasuryAccount::get().into_account_truncating();
		let source_chain = StateMachine::Evm(2000);
		let dest_chain = StateMachine::Evm(3000);

		setup_balances(&relayer_account, &treasury_account);
		setup_host_params(dest_chain);

		let request_message = create_request_message(source_chain, dest_chain, &relayer_pair);

		let _ = pallet_messaging_fees::Pallet::<Test>::on_executed(vec![request_message]);

		assert_eq!(Balances::balance(&relayer_account), 1000 * UNIT);
		assert_eq!(TotalBytesProcessed::<Test>::get(), 0);
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
		setup_host_params(dest_chain);

		pallet_messaging_fees::Pallet::<Test>::set_supported_route(
			RuntimeOrigin::root(),
			source_chain,
			dest_chain,
		)
		.unwrap();

		let request_message = create_request_message(source_chain, dest_chain, &evil_pair);

		let _ = pallet_messaging_fees::Pallet::<Test>::on_executed(vec![request_message]);

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
		setup_host_params(dest_chain);

		pallet_messaging_fees::Pallet::<Test>::set_supported_route(
			RuntimeOrigin::root(),
			source_chain,
			dest_chain,
		)
		.unwrap();

		let mut last_reward = u128::MAX;
		let mut previous_balance = Balances::balance(&relayer_account);
		let number_of_messages = 5;

		for i in 0..number_of_messages {
			let request_message = create_request_message(source_chain, dest_chain, &relayer_pair);
			let _ = pallet_messaging_fees::Pallet::<Test>::on_executed(vec![request_message]);

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
		assert_eq!(Epoch::<Test>::get().index, 0);
		assert_eq!(TotalBytesProcessed::<Test>::get(), 500);

		let validators: Vec<(AccountId32, MockOpaqueKeys)> = vec![];
		let queued_validators: Vec<(AccountId32, MockOpaqueKeys)> = vec![];
		pallet_messaging_fees::Pallet::<Test>::on_new_session(
			true,
			&validators,
			&queued_validators,
		);

		assert_eq!(Epoch::<Test>::get().index, 1);
		assert_eq!(TotalBytesProcessed::<Test>::get(), 0);
	});
}
