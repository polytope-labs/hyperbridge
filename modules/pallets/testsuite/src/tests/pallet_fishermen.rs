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
use polkadot_sdk::*;

use crate::runtime::{new_test_ext, CollatorSet, Ismp, RuntimeOrigin, Test};
use ismp::{
	consensus::{StateCommitment, StateMachineHeight, StateMachineId},
	error::Error,
	host::{IsmpHost, StateMachine},
};
use pallet_fishermen::FishermanBlacklist;
use sp_core::{crypto::AccountId32, H160, H256};
use sp_runtime::{DispatchError, ModuleError};

#[test]
fn test_can_veto_state_commitments() {
	new_test_ext().execute_with(|| {
		let collator: AccountId32 = H256::random().0.into();
		let outsider: AccountId32 = H256::random().0.into();

		// Only the collator is in the active set.
		CollatorSet::set(vec![collator.clone()]);

		let host = Ismp::default();
		let height = StateMachineHeight {
			id: StateMachineId { state_id: StateMachine::Evm(97), consensus_state_id: *b"ETH0" },
			height: 225,
		};
		let commitment = StateCommitment {
			timestamp: 0,
			overlay_root: Some(H256::random()),
			state_root: H256::random(),
		};
		host.store_state_machine_commitment(height, commitment).unwrap();
		assert_eq!(host.state_machine_commitment(height).unwrap(), commitment);

		// A non-collator cannot veto.
		let result = pallet_fishermen::Pallet::<Test>::veto_state_commitment(
			RuntimeOrigin::signed(outsider),
			height,
		);
		assert!(matches!(
			result,
			Err(DispatchError::Module(ModuleError { message: Some("UnauthorizedAction"), .. }))
		));

		// A single collator's call deletes the commitment.
		assert_eq!(
			pallet_fishermen::Pallet::<Test>::veto_state_commitment(
				RuntimeOrigin::signed(collator),
				height,
			),
			Ok(()),
		);
		let result = host.state_machine_commitment(height);
		assert!(matches!(result, Err(Error::StateCommitmentNotFound { .. })));
	})
}

#[test]
fn test_can_blacklist_dispute_games() {
	new_test_ext().execute_with(|| {
		let collator: AccountId32 = H256::random().0.into();
		let outsider: AccountId32 = H256::random().0.into();
		CollatorSet::set(vec![collator.clone()]);

		let state_machine_id =
			StateMachineId { state_id: StateMachine::Evm(10), consensus_state_id: *b"OPTI" };
		let proxy = H160::repeat_byte(0xab);

		// Outsider rejected.
		let result = pallet_fishermen::Pallet::<Test>::blacklist_dispute_game(
			RuntimeOrigin::signed(outsider),
			state_machine_id,
			proxy,
		);
		assert!(matches!(
			result,
			Err(DispatchError::Module(ModuleError { message: Some("UnauthorizedAction"), .. }))
		));

		// Not blacklisted yet.
		assert!(!<pallet_fishermen::Pallet<Test> as FishermanBlacklist>::is_dispute_game_blacklisted(
			state_machine_id, proxy,
		));

		// A single collator's call finalizes the blacklist.
		assert_eq!(
			pallet_fishermen::Pallet::<Test>::blacklist_dispute_game(
				RuntimeOrigin::signed(collator.clone()),
				state_machine_id,
				proxy,
			),
			Ok(()),
		);
		assert!(<pallet_fishermen::Pallet<Test> as FishermanBlacklist>::is_dispute_game_blacklisted(
			state_machine_id, proxy,
		));
		// The submitting fisherman is recorded.
		assert_eq!(
			pallet_fishermen::BlacklistedDisputeGames::<Test>::get(state_machine_id, proxy),
			Some(collator.clone()),
		);

		// Idempotent: a second call is silently Ok and doesn't overwrite the recorded fisherman.
		let second_collator: AccountId32 = H256::random().0.into();
		CollatorSet::set(vec![collator.clone(), second_collator.clone()]);
		assert_eq!(
			pallet_fishermen::Pallet::<Test>::blacklist_dispute_game(
				RuntimeOrigin::signed(second_collator),
				state_machine_id,
				proxy,
			),
			Ok(()),
		);
		assert_eq!(
			pallet_fishermen::BlacklistedDisputeGames::<Test>::get(state_machine_id, proxy),
			Some(collator),
		);

		// A different proxy on the same chain is still un-blacklisted.
		let other_proxy = H160::repeat_byte(0xcd);
		assert!(!<pallet_fishermen::Pallet<Test> as FishermanBlacklist>::is_dispute_game_blacklisted(
			state_machine_id, other_proxy,
		));
	})
}

#[test]
fn test_can_blacklist_arbitrum_claims() {
	new_test_ext().execute_with(|| {
		let collator: AccountId32 = H256::random().0.into();
		CollatorSet::set(vec![collator.clone()]);

		let state_machine_id =
			StateMachineId { state_id: StateMachine::Evm(42161), consensus_state_id: *b"ARBC" };
		let claim = H256::repeat_byte(0x55);

		// A single collator's call finalizes the blacklist.
		assert_eq!(
			pallet_fishermen::Pallet::<Test>::blacklist_arbitrum_claim(
				RuntimeOrigin::signed(collator.clone()),
				state_machine_id,
				claim,
			),
			Ok(()),
		);
		assert!(<pallet_fishermen::Pallet<Test> as FishermanBlacklist>::is_arbitrum_claim_blacklisted(
			state_machine_id, claim,
		));
		assert_eq!(
			pallet_fishermen::BlacklistedArbitrumClaims::<Test>::get(state_machine_id, claim),
			Some(collator),
		);
	})
}
