#![cfg(test)]

use polkadot_sdk::frame_support::traits::Get;
use sp_core::{H256, U256};
use polkadot_sdk::frame_support::traits::fungible::Inspect;
use polkadot_sdk::sp_runtime::traits::AccountIdConversion;
use polkadot_sdk::frame_support::traits::Hooks;
use codec::Encode;
use polkadot_sdk::frame_support::traits::fungible::Mutate;

use ismp::{
    host::StateMachine,
    messaging::Message,
    router::PostRequest,
};
use ismp::consensus::{StateMachineHeight, StateMachineId};
use ismp::messaging::{Proof, RequestMessage};
use pallet_ismp_host_executive::{EvmHostParam, HostParam, PerByteFee};
use sp_core::{crypto::AccountId32};
use pallet_messaging_relayer_incentives::Epoch;
use pallet_messaging_relayer_incentives::TotalBytesProcessed;

use crate::runtime::{Balances, Ismp, new_test_ext, RuntimeOrigin, Test, TreasuryAccount, UNIT, System};

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

fn create_request_message(dest_chain: StateMachine, signer: [u8; 32]) -> Message {
    let post_request = PostRequest {
        source: StateMachine::Evm(2000),
        dest: dest_chain,
        nonce: 0,
        from: vec![1; 32],
        to: vec![2; 32],
        timeout_timestamp: 100,
        body: vec![0; 100],
    };

    let request_message = RequestMessage {
        requests: vec![post_request],
        proof: Proof {
            height: StateMachineHeight {
                id: StateMachineId {
                    state_id: StateMachine::Evm(2000),
                    consensus_state_id: *b"mock",
                },
                height: 1,
            },
            proof: vec![],
        },
        signer: signer.to_vec(),
    };

    Message::Request(request_message)
}

#[test]
fn test_incentivize_relayer_for_request_message() {
    new_test_ext().execute_with(|| {
        let relayer_signer = H256::random().0;
        let relayer_account: AccountId32 = relayer_signer.into();
        let treasury_account = TreasuryAccount::get().into_account_truncating();
        let dest_chain = StateMachine::Evm(2000);

        setup_balances(&relayer_account, &treasury_account);
        setup_host_params(dest_chain);

        pallet_messaging_relayer_incentives::Pallet::<Test>::set_supported_state_machines(
            RuntimeOrigin::root(),
            dest_chain,
        )
            .unwrap();

        let request_message = create_request_message(dest_chain, relayer_signer);

        assert_eq!(TotalBytesProcessed::<Test>::get(), 0);

        pallet_messaging_relayer_incentives::Pallet::<Test>::on_executed(vec![request_message], vec![]).unwrap();

        let bytes_processed = 1;

        let base_reward = 100_000u128;

        let total_bytes = 1;
        let target_size = <Test as pallet_messaging_relayer_incentives::Config>::TargetMessageSize::get();

        let expected_reward =  pallet_messaging_relayer_incentives::Pallet::<Test>::calculate_reward(
            total_bytes,
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
        let relayer_signer = H256::random().0;
        let relayer_account: AccountId32 = relayer_signer.into();
        let treasury_account = TreasuryAccount::get().into_account_truncating();
        let dest_chain = StateMachine::Evm(2000);

        setup_balances(&relayer_account, &treasury_account);
        setup_host_params(dest_chain);

        pallet_messaging_relayer_incentives::Pallet::<Test>::set_supported_state_machines(
            RuntimeOrigin::root(),
            dest_chain,
        )
            .unwrap();

        let target_size: u32 =  <Test as pallet_messaging_relayer_incentives::Config>::TargetMessageSize::get();
        TotalBytesProcessed::<Test>::put(target_size + 1);

        let request_message = create_request_message(dest_chain, relayer_signer);

        pallet_messaging_relayer_incentives::Pallet::<Test>::on_executed(vec![request_message], vec![]).unwrap();

        let fee_charged = 100_000u128;

        assert_eq!(Balances::balance(&relayer_account), 1000 * UNIT - fee_charged);
        assert_eq!(TotalBytesProcessed::<Test>::get(), target_size + 2);
    });
}

#[test]
fn test_skip_incentivizing_for_unsupported_state_machine() {
    new_test_ext().execute_with(|| {
        let relayer_signer = H256::random().0;
        let relayer_account: AccountId32 = relayer_signer.into();
        let treasury_account = TreasuryAccount::get().into_account_truncating();
        let dest_chain = StateMachine::Evm(2000);

        setup_balances(&relayer_account, &treasury_account);
        setup_host_params(dest_chain);

        let request_message = create_request_message(dest_chain, relayer_signer);

        pallet_messaging_relayer_incentives::Pallet::<Test>::on_executed(vec![request_message], vec![]).unwrap();

        assert_eq!(Balances::balance(&relayer_account), 1000 * UNIT);
        assert_eq!(TotalBytesProcessed::<Test>::get(), 0);
    });
}

#[test]
fn test_skip_incentivizing_when_host_params_not_set() {
    new_test_ext().execute_with(|| {
        let relayer_signer = H256::random().0;
        let relayer_account: AccountId32 = relayer_signer.into();
        let treasury_account = TreasuryAccount::get().into_account_truncating();
        let dest_chain = StateMachine::Evm(2000);

        setup_balances(&relayer_account, &treasury_account);

        pallet_messaging_relayer_incentives::Pallet::<Test>::set_supported_state_machines(
            RuntimeOrigin::root(),
            dest_chain,
        )
            .unwrap();

        let request_message = create_request_message(dest_chain, relayer_signer);

        pallet_messaging_relayer_incentives::Pallet::<Test>::on_executed(vec![request_message], vec![]).unwrap();

        assert_eq!(Balances::balance(&relayer_account), 1000 * UNIT);
        assert_eq!(TotalBytesProcessed::<Test>::get(), 1);
    });
}

#[test]
fn test_epoch_finalization() {
    new_test_ext().execute_with(|| {
        let epoch_length = <Test as pallet_messaging_relayer_incentives::Config>::EpochLength::get();

       TotalBytesProcessed::<Test>::put(500);
        assert_eq!(Epoch::<Test>::get().index, 0);
        assert_eq!(TotalBytesProcessed::<Test>::get(), 500);

        for i in 1..=epoch_length {
            System::set_block_number(i);
            pallet_messaging_relayer_incentives::Pallet::<Test>::on_finalize(i);
        }

        assert_eq!(Epoch::<Test>::get().index, 1);
        assert_eq!(TotalBytesProcessed::<Test>::get(), 0);
    });
}

#[test]
fn test_reward_decreases_as_messages_increase() {
    new_test_ext().execute_with(|| {
        let relayer_signer = H256::random().0;
        let relayer_account: AccountId32 = relayer_signer.into();
        setup_balances(&relayer_account, &TreasuryAccount::get().into_account_truncating());
        let dest_chain = StateMachine::Evm(2000);
        setup_host_params(dest_chain);
        pallet_messaging_relayer_incentives::Pallet::<Test>::set_supported_state_machines(
            RuntimeOrigin::root(),
            dest_chain,
        ).unwrap();

        let mut last_reward = u128::MAX;
        let mut previous_balance = Balances::balance(&relayer_account);
        let number_of_messages = 5;

        println!("Initial Balance: {}", previous_balance);

        for i in 0..number_of_messages {
            let request_message = create_request_message(dest_chain, relayer_signer);
            pallet_messaging_relayer_incentives::Pallet::<Test>::on_executed(vec![request_message], vec![]).unwrap();

            let current_balance = Balances::balance(&relayer_account);
            let reward_received = current_balance - previous_balance;

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