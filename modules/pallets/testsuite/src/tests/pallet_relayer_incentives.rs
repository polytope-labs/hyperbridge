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

use frame_support::traits::fungible::{Inspect, Mutate};
use frame_support::PalletId;
use polkadot_sdk::*;
use sp_core::{crypto::AccountId32, H256};
use sp_runtime::traits::AccountIdConversion;

use ismp::messaging::ConsensusMessage;
use ismp::{
	consensus::StateMachineId,
	host::{IsmpHost, StateMachine},
	messaging::Message,
};

use crate::runtime::*;
use crate::runtime::{new_test_ext, Ismp, RuntimeOrigin, Test};

#[test]
fn test_incentivize_relayer() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let host = Ismp::default();

		let mock_consensus_state_id = *b"mock";

		let state_machine_id = StateMachineId {
			state_id: StateMachine::Polkadot(1000),
			consensus_state_id: mock_consensus_state_id,
		};

		let relayer = H256::random().0;
		let relayer_account: AccountId32 = relayer.into();
		assert_eq!(Balances::balance(&relayer_account), Default::default());
		Balances::mint_into(&relayer_account, UNIT).unwrap();

		pallet_relayer_incentives::Pallet::<Test>::update_cost_per_block(
			RuntimeOrigin::root(),
			state_machine_id,
			100,
		)
		.unwrap();

		assert_eq!(Balances::balance(&relayer_account), UNIT);

		let treasury_account = PalletId(*b"treasury");

		assert_eq!(
			Balances::balance(&treasury_account.into_account_truncating()),
			Default::default()
		);
		Balances::mint_into(&treasury_account.into_account_truncating(), 20000 * UNIT).unwrap();

		let consensus_message = Message::Consensus(ConsensusMessage {
			consensus_proof: vec![],
			consensus_state_id: mock_consensus_state_id,
			signer: relayer.into(),
		});
		setup_mock_client::<_, Test>(&host);
		host.unbonding_period(mock_consensus_state_id).unwrap();
		host.store_consensus_update_time(mock_consensus_state_id, host.timestamp())
			.unwrap();

		pallet_ismp::Pallet::<Test>::handle_unsigned(
			RuntimeOrigin::none(),
			vec![consensus_message],
		)
		.unwrap();

		// check that relayer was rewarded
		assert_eq!(Balances::balance(&relayer_account), UNIT + 4200);
	})
}
