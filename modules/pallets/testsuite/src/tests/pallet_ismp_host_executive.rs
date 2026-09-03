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

use crate::runtime::{last_event, new_test_ext, Ismp, RuntimeEvent, RuntimeOrigin, Test};
use frame_support::traits::Get;
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

/// The request that rotates the host manager must be addressed to the manager the destination
/// host currently authorises, while its payload installs the new one. Once the rotation is
/// recorded locally, later governance traffic is addressed to the new manager.
#[test]
fn test_manager_rotation_is_addressed_to_the_current_manager() {
	use ismp::{
		messaging::hash_request,
		router::{PostRequest, Request},
	};
	use pallet_ismp_host_executive::{WithdrawalParams, PALLET_ID};
	use sp_core::U256;

	new_test_ext().execute_with(|| {
		let chain = StateMachine::Evm(1);
		let current_manager = H160::random();
		let new_manager = H160::random();

		let mut initial = EvmHostParam::default();
		initial.host_manager = current_manager;
		let mut map = BTreeMap::new();
		map.insert(chain, HostParam::EvmHostParam(initial.clone()));
		pallet_ismp_host_executive::Pallet::<Test>::set_host_params(RuntimeOrigin::root(), map)
			.unwrap();

		let mut update = EvmHostParamUpdate::default();
		update.host_manager = Some(new_manager);
		pallet_ismp_host_executive::Pallet::<Test>::update_host_params(
			RuntimeOrigin::root(),
			chain,
			HostParamUpdate::EvmHostParam(update),
		)
		.unwrap();

		let (commitment, nonce) = last_dispatched_request();
		let mut expected = initial.clone();
		expected.host_manager = new_manager;
		let addressed_to = |to: H160| {
			hash_request::<Ismp>(&Request::Post(PostRequest {
				source: <Test as pallet_ismp::Config>::HostStateMachine::get(),
				dest: chain,
				nonce,
				from: PALLET_ID.to_bytes(),
				to: to.0.to_vec(),
				timeout_timestamp: 0,
				body: expected.abi_encode_with_variant().unwrap(),
			}))
		};
		assert_eq!(
			commitment,
			addressed_to(current_manager),
			"rotation goes through the current manager"
		);
		assert_ne!(commitment, addressed_to(new_manager), "the new manager is not yet authorised");
		assert_eq!(
			pallet_ismp_host_executive::HostParams::<Test>::get(chain),
			Some(HostParam::EvmHostParam(expected.clone())),
			"the new manager is recorded locally"
		);

		// Everything after the rotation is addressed to the new manager.
		let withdrawal = WithdrawalParams {
			beneficiary_address: H160::random().0.to_vec(),
			amount: U256::from(1u64),
			token: H160::zero(),
		};
		pallet_ismp_host_executive::Pallet::<Test>::withdraw(
			RuntimeOrigin::root(),
			chain,
			withdrawal.clone(),
		)
		.unwrap();
		let (commitment, nonce) = last_dispatched_request();
		assert_eq!(
			commitment,
			hash_request::<Ismp>(&Request::Post(PostRequest {
				source: <Test as pallet_ismp::Config>::HostStateMachine::get(),
				dest: chain,
				nonce,
				from: PALLET_ID.to_bytes(),
				to: new_manager.0.to_vec(),
				timeout_timestamp: 0,
				body: withdrawal.abi_encode().unwrap(),
			}))
		);
	})
}

/// Commitment and nonce of the most recently dispatched ISMP request.
fn last_dispatched_request() -> (H256, u64) {
	frame_system::Pallet::<Test>::events()
		.into_iter()
		.rev()
		.find_map(|record| match record.event {
			RuntimeEvent::Ismp(pallet_ismp::Event::Request {
				commitment, request_nonce, ..
			}) => Some((commitment, request_nonce)),
			_ => None,
		})
		.expect("a request was dispatched")
}
