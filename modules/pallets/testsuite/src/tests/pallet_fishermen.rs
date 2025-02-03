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

use crate::runtime::{new_test_ext, Ismp, RuntimeOrigin, Test};
use ismp::{
	consensus::{StateCommitment, StateMachineHeight, StateMachineId},
	error::Error,
	host::{IsmpHost, StateMachine},
};
use sp_core::{crypto::AccountId32, H256};
use sp_runtime::{DispatchError, ModuleError};

#[test]
fn test_can_veto_state_commitments() {
	new_test_ext().execute_with(|| {
		let account: AccountId32 = H256::random().0.into();

		// add a new fisherman
		pallet_fishermen::Pallet::<Test>::add(RuntimeOrigin::root(), account.clone()).unwrap();
		assert_eq!(pallet_fishermen::Fishermen::<Test>::get(account.clone()), Some(()));

		// sanity check, can't add it again
		let result = pallet_fishermen::Pallet::<Test>::add(RuntimeOrigin::root(), account.clone());
		assert_eq!(
			result,
			Err(DispatchError::Module(ModuleError {
				index: 9,
				error: [0, 0, 0, 0],
				message: Some("AlreadyAdded"),
			}))
		);

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

		let result = host.state_machine_commitment(height).unwrap();
		assert_eq!(result, commitment);

		// sanity check, unauthorized veto
		let result = pallet_fishermen::Pallet::<Test>::veto_state_commitment(
			RuntimeOrigin::signed(H256::random().0.into()),
			height,
		);
		assert_eq!(
			result,
			Err(DispatchError::Module(ModuleError {
				index: 9,
				error: [2, 0, 0, 0],
				message: Some("UnauthorizedAction"),
			}))
		);

		// Add another fisherman

		let account_2: AccountId32 = H256::random().0.into();
		pallet_fishermen::Pallet::<Test>::add(RuntimeOrigin::root(), account_2.clone()).unwrap();
		assert_eq!(pallet_fishermen::Fishermen::<Test>::get(account_2.clone()), Some(()));

		// actual veto
		let result = pallet_fishermen::Pallet::<Test>::veto_state_commitment(
			RuntimeOrigin::signed(account.clone()),
			height,
		);
		assert_eq!(result, Ok(()));

		assert_eq!(pallet_fishermen::PendingVetoes::<Test>::get(height), Some(account.clone()));

		// veto with same account
		let result = pallet_fishermen::Pallet::<Test>::veto_state_commitment(
			RuntimeOrigin::signed(account.clone()),
			height,
		);

		assert!(matches!(
			result,
			Err(sp_runtime::DispatchError::Module(ModuleError {
				index: 9,
				error: [4, 0, 0, 0,],
				message: Some("InvalidVeto",),
			}))
		));

		let result = pallet_fishermen::Pallet::<Test>::veto_state_commitment(
			RuntimeOrigin::signed(account_2.clone()),
			height,
		);
		assert_eq!(result, Ok(()));

		// should have been deleted
		let result = host.state_machine_commitment(height);
		assert!(matches!(result, Err(Error::StateCommitmentNotFound { .. })));

		pallet_fishermen::Pallet::<Test>::remove(RuntimeOrigin::root(), account.clone()).unwrap();
		assert_eq!(pallet_fishermen::Fishermen::<Test>::get(account), None);
	})
}
