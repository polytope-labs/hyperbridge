// Copyright (C) 2023 Polytope Labs.
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

use crate::runtime::{assert_last_event, new_test_ext, RuntimeOrigin, Test};
use ismp::{
    consensus::{StateCommitment, StateMachineHeight, StateMachineId},
    error::Error,
    host::{Ethereum, IsmpHost, StateMachine},
};
use pallet_ismp::host::Host;
use sp_core::{crypto::AccountId32, H256};
use sp_runtime::{DispatchError, ModuleError};

#[test]
fn test_can_veto_state_commitments() {
    new_test_ext().execute_with(|| {
        let account: AccountId32 = H256::random().0.into();

        // add a new fisherman
        pallet_fishermen::Pallet::<Test>::add(RuntimeOrigin::root(), account.clone()).unwrap();
        assert_eq!(pallet_fishermen::WhitelistedAccount::<Test>::get(account.clone()), Some(()));

        // sanity check, can't add it again
        let result = pallet_fishermen::Pallet::<Test>::add(RuntimeOrigin::root(), account.clone());
        assert_eq!(
            result,
            Err(DispatchError::Module(ModuleError {
                index: 5,
                error: [0, 0, 0, 0],
                message: Some("AlreadyAdded"),
            }))
        );

        let host = Host::<Test>::default();
        let height = StateMachineHeight {
            id: StateMachineId {
                state_id: StateMachine::Ethereum(Ethereum::Optimism),
                consensus_state_id: *b"ETH0",
            },
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
                index: 5,
                error: [2, 0, 0, 0],
                message: Some("UnauthorizedAction"),
            }))
        );

        // actual veto
        let result = pallet_fishermen::Pallet::<Test>::veto_state_commitment(
            RuntimeOrigin::signed(account.clone()),
            height,
        );
        assert_eq!(result, Ok(()));

        // should have been deleted
        let result = host.state_machine_commitment(height);
        assert!(matches!(result, Err(Error::StateCommitmentNotFound { .. })));

        pallet_fishermen::Pallet::<Test>::remove(RuntimeOrigin::root(), account.clone()).unwrap();
        assert_eq!(pallet_fishermen::WhitelistedAccount::<Test>::get(account), None);
    })
}
