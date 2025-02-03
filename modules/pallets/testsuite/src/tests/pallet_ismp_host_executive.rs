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

use crate::runtime::{last_event, new_test_ext, RuntimeEvent, RuntimeOrigin, Test};
use ismp::host::StateMachine;
use pallet_ismp_host_executive::{EvmHostParam, EvmHostParamUpdate, HostParam, HostParamUpdate};
use sp_core::{crypto::AccountId32, H160, H256};
use sp_runtime::DispatchError;
use std::collections::BTreeMap;

#[test]
fn test_host_executive() {
	new_test_ext().execute_with(|| {
		let account: AccountId32 = H256::random().0.into();

		let handler = H160::random();
		let mut map = BTreeMap::new();
		let mut evm_host_params = EvmHostParam::default();
		evm_host_params.handler = handler;
		let params = HostParam::EvmHostParam(evm_host_params);
		map.insert(StateMachine::Polkadot(2000), params.clone());

		// sanity check non-root can't dispatch requests
		let result = pallet_ismp_host_executive::Pallet::<Test>::set_host_params(
			RuntimeOrigin::signed(account),
			map.clone(),
		);
		assert_eq!(result, Err(DispatchError::BadOrigin));

		pallet_ismp_host_executive::Pallet::<Test>::set_host_params(RuntimeOrigin::root(), map)
			.unwrap();

		let mut params = EvmHostParamUpdate::default();
		let new_handler = H160::random();
		params.handler = Some(new_handler);
		pallet_ismp_host_executive::Pallet::<Test>::update_host_params(
			RuntimeOrigin::root(),
			StateMachine::Polkadot(2000),
			HostParamUpdate::EvmHostParam(params),
		)
		.unwrap();

		let RuntimeEvent::HostExecutive(
			pallet_ismp_host_executive::Event::<Test>::HostParamsUpdated { state_machine, .. },
		) = last_event::<Test>()
		else {
			panic!("Ismp request not found")
		};

		assert_eq!(state_machine, StateMachine::Polkadot(2000))
	})
}
