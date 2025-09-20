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

use codec::Encode;
use frame_support::traits::fungible::{Inspect, Mutate};
use sp_core::{crypto::AccountId32, H256};
use sp_runtime::traits::AccountIdConversion;

use ismp::{
	dispatcher::{DispatchPost, DispatchRequest, FeeMetadata, IsmpDispatcher},
	host::StateMachine,
	module::IsmpModule,
	router::PostRequest,
};
use pallet_hyperbridge::{Message, SubstrateHostParams, VersionedHostParams, WithdrawalRequest};
use pallet_ismp::RELAYER_FEE_ACCOUNT;

use crate::runtime::{new_test_ext, Balances, Coprocessor, Hyperbridge, UNIT};

#[test]
fn test_dispatch_fees() {
	let mut ext = new_test_ext();
	let account: AccountId32 = H256::random().0.into();
	let hyperbridge = Hyperbridge::default();

	ext.execute_with(|| {
		hyperbridge
			.on_accept(PostRequest {
				// not the coprocessor so this should fail
				source: StateMachine::Polkadot(3368),
				dest: StateMachine::Polkadot(2001),
				nonce: 0,
				from: vec![],
				to: vec![],
				timeout_timestamp: 0,
				body: vec![],
			})
			.unwrap_err();

		// lets set the protocol fees
		let params = VersionedHostParams::V1(SubstrateHostParams {
			default_per_byte_fee: 10 * UNIT,
			..Default::default()
		});
		let data = Message::<AccountId32, u128>::UpdateHostParams(params.clone()).encode();
		hyperbridge
			.on_accept(PostRequest {
				//
				source: Coprocessor::get().unwrap(),
				dest: StateMachine::Polkadot(2001),
				nonce: 0,
				from: vec![],
				to: vec![],
				timeout_timestamp: 0,
				body: data,
			})
			.unwrap();

		// params was successfully set
		assert_eq!(Hyperbridge::host_params(), params);

		assert_eq!(Balances::balance(&account), Default::default());
		// cost of request
		Balances::mint_into(&account, 65 * 10 * UNIT).unwrap();
		assert_eq!(Balances::balance(&account), 65 * 10 * UNIT);

		let msg = DispatchPost {
			dest: StateMachine::Evm(1),
			from: vec![0u8; 32],
			to: vec![0u8; 32],
			timeout: 2_000_000_000,
			body: vec![0u8; 64],
		};
		hyperbridge
			.dispatch_request(
				DispatchRequest::Post(msg.clone()),
				// lets pay 10 units
				FeeMetadata { payer: account.clone().into(), fee: 10 * UNIT },
			)
			.unwrap();

		// we should no longer have it
		assert_eq!(Balances::balance(&account), Default::default());

		// now pallet-ismp has it
		assert_eq!(
			Balances::balance(&RELAYER_FEE_ACCOUNT.into_account_truncating()),
			65 * 10 * UNIT
		);
	});
}

#[test]
fn test_can_withdraw_relayer_and_protocol_revenue() {
	let mut ext = new_test_ext();
	let account: AccountId32 = H256::random().0.into();
	let hyperbridge = Hyperbridge::default();

	ext.execute_with(|| {
		Balances::mint_into(&RELAYER_FEE_ACCOUNT.into_account_truncating(), 65 * 10 * UNIT)
			.unwrap();

		let withdrawal = WithdrawalRequest { amount: 65 * 10 * UNIT, account: account.clone() };

		let data = Message::<AccountId32, u128>::WithdrawRelayerFees(withdrawal.clone()).encode();
		hyperbridge
			.on_accept(PostRequest {
				source: Coprocessor::get().unwrap(),
				dest: StateMachine::Polkadot(2001),
				nonce: 0,
				from: vec![],
				to: vec![],
				timeout_timestamp: 0,
				body: data,
			})
			.unwrap();

		// relayer fees withdrawn
		assert_eq!(Balances::balance(&account), 65 * 10 * UNIT);
	});
}
