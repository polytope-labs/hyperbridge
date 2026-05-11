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
use sp_core::{crypto::AccountId32, H256};
use sp_runtime::{DispatchError, ModuleError};

#[test]
fn test_can_veto_state_commitments() {
	new_test_ext().execute_with(|| {
		let collator_a: AccountId32 = H256::random().0.into();
		let collator_b: AccountId32 = H256::random().0.into();
		let outsider: AccountId32 = H256::random().0.into();

		// Seed the active collator set. Outsider deliberately not included.
		CollatorSet::set(vec![collator_a.clone(), collator_b.clone()]);

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
			RuntimeOrigin::signed(outsider.clone()),
			height,
		);
		assert_eq!(
			result,
			Err(DispatchError::Module(ModuleError {
				index: 9,
				error: [0, 0, 0, 0],
				message: Some("UnauthorizedAction"),
			}))
		);

		// First collator records a pending veto.
		let result = pallet_fishermen::Pallet::<Test>::veto_state_commitment(
			RuntimeOrigin::signed(collator_a.clone()),
			height,
		);
		assert_eq!(result, Ok(()));
		assert_eq!(pallet_fishermen::PendingVetoes::<Test>::get(height), Some(collator_a.clone()));

		// Same collator submitting again is rejected.
		let result = pallet_fishermen::Pallet::<Test>::veto_state_commitment(
			RuntimeOrigin::signed(collator_a.clone()),
			height,
		);
		assert!(matches!(
			result,
			Err(sp_runtime::DispatchError::Module(ModuleError {
				index: 9,
				error: [2, 0, 0, 0,],
				message: Some("InvalidVeto",),
			}))
		));

		// Second distinct collator finalizes the veto and the commitment is gone.
		let result = pallet_fishermen::Pallet::<Test>::veto_state_commitment(
			RuntimeOrigin::signed(collator_b.clone()),
			height,
		);
		assert_eq!(result, Ok(()));

		let result = host.state_machine_commitment(height);
		assert!(matches!(result, Err(Error::StateCommitmentNotFound { .. })));
	})
}
