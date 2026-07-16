// Copyright (c) 2025 Polytope Labs.
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

#![cfg(test)]

use codec::Encode;
use frame_support::{
	traits::fungible::{Inspect, Mutate},
	PalletId,
};
use polkadot_sdk::*;
use sp_core::{crypto::AccountId32, keccak_256, sr25519, ByteArray, Pair, H256};
use sp_runtime::traits::AccountIdConversion;

use ismp::{
	consensus::{StateCommitment, StateMachineHeight, StateMachineId},
	events::{Event as IsmpEvent, StateMachineUpdated},
	host::{IsmpHost, StateMachine},
	messaging::{ConsensusMessage, Message, MessageWithWeight},
};
use pallet_ismp::fee_handler::FeeHandler;
use pallet_ismp_relayer::withdrawal::Signature;
use polkadot_sdk::sp_runtime::Weight;

use crate::{
	runtime::{new_test_ext, Assets, Ismp, RuntimeOrigin, Test, *},
	tests::common::setup_relayer_and_asset,
};

fn setup_state_machine() -> StateMachineId {
	StateMachineId { state_id: StateMachine::Polkadot(1000), consensus_state_id: *b"mock" }
}

fn setup_balances(relayer_account: &AccountId32, treasury_account: &AccountId32) {
	setup_relayer_and_asset(&relayer_account);

	assert_eq!(Balances::balance(relayer_account), 0);
	Balances::mint_into(relayer_account, UNIT).unwrap();
	assert_eq!(Balances::balance(relayer_account), UNIT);

	assert_eq!(Balances::balance(treasury_account), 0);
	Balances::mint_into(treasury_account, 20000 * UNIT).unwrap();
}

fn setup_host_and_message(host: &Ismp) -> (Message, AccountId32) {
	let relayer_pair = sr25519::Pair::from_seed(&H256::random().0);
	let treasury_account = PalletId(*b"treasury").into_account_truncating();

	let relayer_account: AccountId32 = relayer_pair.public().into();

	setup_balances(&relayer_account.clone().into(), &treasury_account);

	let consensus_proof: Vec<u8> = vec![0];
	let signed_data = keccak_256(&consensus_proof);
	let signature = relayer_pair.sign(&signed_data);
	let signature = Signature::Sr25519 {
		public_key: relayer_pair.public().to_raw_vec(),
		signature: signature.to_raw_vec(),
	};

	dbg!(relayer_pair.public());

	let message = Message::Consensus(ConsensusMessage {
		consensus_proof,
		consensus_state_id: *b"mock",
		signer: signature.encode(),
	});
	setup_mock_client::<_, Test>(host);
	host.unbonding_period(*b"mock").unwrap();
	host.store_consensus_update_time(*b"mock", host.timestamp()).unwrap();
	(message, relayer_account)
}

#[test]
fn test_incentivize_relayer() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let host = Ismp::default();
		let state_machine_id = setup_state_machine();

		pallet_consensus_incentives::Pallet::<Test>::update_cost_per_block(
			RuntimeOrigin::root(),
			state_machine_id,
			100,
		)
		.unwrap();

		let (consensus_message, relayer_account) = setup_host_and_message(&host);

		pallet_ismp::Pallet::<Test>::handle_unsigned(
			RuntimeOrigin::none(),
			vec![consensus_message],
		)
		.unwrap();

		assert_eq!(Balances::balance(&relayer_account), UNIT + 4200);
		assert_eq!(Assets::balance(ReputationAssetId::get(), &relayer_account), 4200);
	})
}

fn commitment() -> StateCommitment {
	StateCommitment { timestamp: 0, overlay_root: Some(H256::random()), state_root: H256::random() }
}

// A relayer is paid once for advancing a state machine across a span of heights. When the latest
// height is rolled back and later resubmitted, the reward should still only cover the new blocks.
// The `LastRewardedHeight` watermark keeps each payout scoped to the span that has not been paid.
#[test]
fn reward_covers_only_unpaid_heights_after_rollback() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		const BLOCK_COST: u128 = 100;
		let host = Ismp::default();
		let state_machine_id = setup_state_machine();
		let treasury_account: AccountId32 = PalletId(*b"treasury").into_account_truncating();

		pallet_consensus_incentives::Pallet::<Test>::update_cost_per_block(
			RuntimeOrigin::root(),
			state_machine_id,
			BLOCK_COST,
		)
		.unwrap();

		let (consensus_message, relayer_account) = setup_host_and_message(&host);
		let message = MessageWithWeight { message: consensus_message, weight: Weight::zero() };
		let updated = |height: u64| {
			vec![IsmpEvent::StateMachineUpdated(StateMachineUpdated {
				state_machine_id,
				latest_height: height,
			})]
		};

		// The chain has already advanced to 1025 and every block up to it has been rewarded once.
		host.store_state_machine_commitment(
			StateMachineHeight { id: state_machine_id, height: 1024 },
			commitment(),
		)
		.unwrap();
		host.store_latest_commitment_height(StateMachineHeight {
			id: state_machine_id,
			height: 1024,
		})
		.unwrap();
		host.store_state_machine_commitment(
			StateMachineHeight { id: state_machine_id, height: 1025 },
			commitment(),
		)
		.unwrap();
		host.store_latest_commitment_height(StateMachineHeight {
			id: state_machine_id,
			height: 1025,
		})
		.unwrap();

		let treasury_before_first = Balances::balance(&treasury_account);
		<pallet_consensus_incentives::Pallet<Test> as FeeHandler>::on_executed(
			vec![message.clone()],
			updated(1025),
		)
		.unwrap();

		assert_eq!(Balances::balance(&treasury_account), treasury_before_first - BLOCK_COST);
		assert_eq!(
			pallet_consensus_incentives::LastRewardedHeight::<Test>::get(state_machine_id),
			Some(1025)
		);

		// The previous-height pointer references an older height whose commitment is no longer
		// retained in the bounded map.
		pallet_ismp::PreviousStateMachineHeight::<Test>::insert(state_machine_id, 1);

		// Deleting the latest commitment rolls the latest height back to that previous pointer.
		host.delete_state_commitment(StateMachineHeight { id: state_machine_id, height: 1025 })
			.unwrap();
		assert_eq!(host.latest_commitment_height(state_machine_id).unwrap(), 1);

		// The next honest consensus update advances to 1030, carrying the stale pointer forward as
		// the new previous height.
		host.store_state_machine_commitment(
			StateMachineHeight { id: state_machine_id, height: 1030 },
			commitment(),
		)
		.unwrap();
		host.store_latest_commitment_height(StateMachineHeight {
			id: state_machine_id,
			height: 1030,
		})
		.unwrap();
		assert_eq!(host.previous_commitment_height(state_machine_id), Some(1));

		let treasury_before_second = Balances::balance(&treasury_account);
		<pallet_consensus_incentives::Pallet<Test> as FeeHandler>::on_executed(
			vec![message],
			updated(1030),
		)
		.unwrap();

		// The real advance is 1025 -> 1030, so only the 5 new blocks are paid rather than the full
		// span back to the previous pointer.
		assert_eq!(Balances::balance(&treasury_account), treasury_before_second - 5 * BLOCK_COST);
		assert_eq!(
			pallet_consensus_incentives::LastRewardedHeight::<Test>::get(state_machine_id),
			Some(1030)
		);
	})
}

#[test]
fn skip_incentivizing_of_relayer_when_cost_per_block_is_not_set() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let host = Ismp::default();
		let (consensus_message, relayer_account) = setup_host_and_message(&host);

		pallet_ismp::Pallet::<Test>::handle_unsigned(
			RuntimeOrigin::none(),
			vec![consensus_message],
		)
		.unwrap();

		assert_eq!(Balances::balance(&relayer_account), UNIT);
	})
}
