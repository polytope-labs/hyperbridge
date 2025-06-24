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

use frame_support::{
	traits::fungible::{Inspect, Mutate},
	PalletId,
};
use polkadot_sdk::*;
use sp_core::{crypto::AccountId32, H256};
use sp_runtime::traits::AccountIdConversion;

use ismp::{
	consensus::StateMachineId,
	host::{IsmpHost, StateMachine},
	messaging::{ConsensusMessage, Message},
};

use crate::runtime::{new_test_ext, Ismp, RuntimeOrigin, Test, *};

fn setup_state_machine() -> StateMachineId {
	StateMachineId { state_id: StateMachine::Polkadot(1000), consensus_state_id: *b"mock" }
}

fn setup_balances(relayer_account: &AccountId32, treasury_account: &AccountId32) {
	assert_eq!(Balances::balance(relayer_account), 0);
	Balances::mint_into(relayer_account, UNIT).unwrap();
	assert_eq!(Balances::balance(relayer_account), UNIT);

	assert_eq!(Balances::balance(treasury_account), 0);
	Balances::mint_into(treasury_account, 20000 * UNIT).unwrap();
}

fn setup_host_and_message(relayer: H256, host: &Ismp) -> Message {
	let message = Message::Consensus(ConsensusMessage {
		consensus_proof: vec![],
		consensus_state_id: *b"mock",
		signer: relayer.0.into(),
	});
	setup_mock_client::<_, Test>(host);
	host.unbonding_period(*b"mock").unwrap();
	host.store_consensus_update_time(*b"mock", host.timestamp()).unwrap();
	message
}

#[test]
fn test_incentivize_relayer() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let host = Ismp::default();
		let state_machine_id = setup_state_machine();

		let relayer = H256::random().0;
		let relayer_account: AccountId32 = relayer.into();
		let treasury_account = PalletId(*b"treasury").into_account_truncating();

		setup_balances(&relayer_account, &treasury_account);

		pallet_relayer_incentives::Pallet::<Test>::update_cost_per_block(
			RuntimeOrigin::root(),
			state_machine_id,
			100,
		)
		.unwrap();

		let consensus_message = setup_host_and_message(relayer.into(), &host);

		pallet_ismp::Pallet::<Test>::handle_unsigned(
			RuntimeOrigin::none(),
			vec![consensus_message],
		)
		.unwrap();

		assert_eq!(Balances::balance(&relayer_account), UNIT + 4200);
	})
}

#[test]
fn skip_incentivizing_of_relayer_when_cost_per_block_is_not_set() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let host = Ismp::default();

		let relayer = H256::random().0;
		let relayer_account: AccountId32 = relayer.into();
		let treasury_account = PalletId(*b"treasury").into_account_truncating();

		setup_balances(&relayer_account, &treasury_account);

		let consensus_message = setup_host_and_message(relayer.into(), &host);

		pallet_ismp::Pallet::<Test>::handle_unsigned(
			RuntimeOrigin::none(),
			vec![consensus_message],
		)
		.unwrap();

		assert_eq!(Balances::balance(&relayer_account), UNIT);
	})
}
