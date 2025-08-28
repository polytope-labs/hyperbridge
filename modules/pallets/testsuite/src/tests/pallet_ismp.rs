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

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use codec::Encode;
use frame_support::traits::{
	fungible::{Inspect, Mutate},
	Time,
};
use frame_system::Origin;
use polkadot_sdk::{sp_crypto_hashing::keccak_256, *};
use sp_core::{crypto::AccountId32, ByteArray, Pair, H256};
use sp_runtime::traits::AccountIdConversion;

use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	dispatcher::{DispatchGet, DispatchRequest, FeeMetadata, IsmpDispatcher},
	host::{IsmpHost, StateMachine},
	messaging::{hash_request, Message, Proof, RequestMessage, ResponseMessage, TimeoutMessage},
	router::{GetResponse, PostRequest, Request, RequestResponse, Response, Timeout},
};
use ismp_testsuite::{
	check_challenge_period, check_client_expiry, create_relayer_signer,
	missing_state_commitment_check, post_request_timeout_check, post_response_timeout_check,
	write_outgoing_commitments,
};
use pallet_ismp::{
	child_trie::{RequestCommitments, RequestReceipts},
	offchain::Leaf,
	FundMessageParams, MessageCommitment, RELAYER_FEE_ACCOUNT,
};
use pallet_ismp_relayer::withdrawal::Signature;

use crate::runtime::*;

fn set_timestamp(now: Option<u64>) {
	Timestamp::set_timestamp(
		now.unwrap_or(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64),
	);
}

#[test]
fn dispatcher_should_write_receipts_for_outgoing_requests_and_responses() {
	let mut ext = new_test_ext();

	ext.execute_with(|| {
		set_timestamp(Some(1));
		let host = Ismp::default();
		let post = PostRequest {
			source: StateMachine::Kusama(2000),
			dest: host.host_state_machine(),
			nonce: 0,
			from: vec![0u8; 32],
			to: vec![0u8; 32],
			timeout_timestamp: 0,
			body: vec![0u8; 64],
		};

		let request_commitment = hash_request::<Ismp>(&Request::Post(post.clone()));
		RequestReceipts::<Test>::insert(request_commitment, &vec![0u8; 32]);
		write_outgoing_commitments(&host).unwrap();
	})
}

#[test]
fn should_reject_updates_within_challenge_period() {
	let mut ext = new_test_ext();

	ext.execute_with(|| {
		set_timestamp(None);
		let host = Ismp::default();

		let id = StateMachineId {
			state_id: StateMachine::Evm(11155111),
			consensus_state_id: MOCK_CONSENSUS_STATE_ID,
		};
		host.store_challenge_period(id, 1_000_000).unwrap();
		check_challenge_period(&host).unwrap()
	})
}

#[test]
fn should_reject_messages_for_frozen_state_machines() {
	let mut ext = new_test_ext();

	ext.execute_with(|| {
		set_timestamp(None);
		let host = Ismp::default();
		let id = StateMachineId {
			state_id: StateMachine::Evm(11155111),
			consensus_state_id: MOCK_CONSENSUS_STATE_ID,
		};
		host.store_challenge_period(id, 1_000_000).unwrap();
		missing_state_commitment_check(&host).unwrap()
	})
}

#[test]
fn should_reject_expired_check_clients() {
	let mut ext = new_test_ext();

	ext.execute_with(|| {
		set_timestamp(None);
		let host = Ismp::default();
		host.store_unbonding_period(MOCK_CONSENSUS_STATE_ID, 1_000_000).unwrap();
		let id = StateMachineId {
			state_id: StateMachine::Evm(11155111),
			consensus_state_id: MOCK_CONSENSUS_STATE_ID,
		};
		host.store_challenge_period(id, 1_000_000).unwrap();
		check_client_expiry(&host).unwrap()
	})
}

#[test]
fn should_handle_post_request_timeouts_correctly() {
	let mut ext = new_test_ext();

	ext.execute_with(|| {
		set_timestamp(Some(0));
		let host = Ismp::default();
		let id = StateMachineId {
			state_id: StateMachine::Evm(11155111),
			consensus_state_id: MOCK_CONSENSUS_STATE_ID,
		};
		host.store_challenge_period(id, 0).unwrap();
		post_request_timeout_check(&host).unwrap()
	})
}

#[test]
fn should_handle_post_response_timeouts_correctly() {
	let mut ext = new_test_ext();

	ext.execute_with(|| {
		set_timestamp(None);
		let host = Ismp::default();
		let id = StateMachineId {
			state_id: StateMachine::Evm(11155111),
			consensus_state_id: MOCK_CONSENSUS_STATE_ID,
		};
		host.store_challenge_period(id, 1_000_000).unwrap();
		post_response_timeout_check(&host).unwrap()
	})
}

#[test]
fn should_handle_get_request_timeouts_correctly() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let host = Ismp::default();
		setup_mock_client::<_, Test>(&host);
		let id = StateMachineId {
			state_id: StateMachine::Evm(11155111),
			consensus_state_id: MOCK_CONSENSUS_STATE_ID,
		};
		host.store_challenge_period(id, 0).unwrap();
		let requests = (0..2)
			.into_iter()
			.map(|i| {
				let msg = DispatchGet {
					dest: StateMachine::Evm(1),
					from: vec![0u8; 32],
					keys: vec![vec![1u8; 32], vec![1u8; 32]],
					context: Default::default(),
					height: 2,
					timeout: 1000,
				};

				host.dispatch_request(
					DispatchRequest::Get(msg),
					FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
				)
				.unwrap();
				let get = ismp::router::GetRequest {
					source: host.host_state_machine(),
					dest: StateMachine::Evm(1),
					nonce: i,
					from: vec![0u8; 32],
					keys: vec![vec![1u8; 32], vec![1u8; 32]],
					height: 2,
					context: Default::default(),

					timeout_timestamp: Duration::from_millis(Timestamp::now()).as_secs() + 1000,
				};
				ismp::router::Request::Get(get)
			})
			.collect::<Vec<_>>();

		let timeout_msg = TimeoutMessage::Get { requests: requests.clone() };

		set_timestamp(Some(Duration::from_secs(100_000_000).as_millis() as u64));
		pallet_ismp::Pallet::<Test>::execute(vec![Message::Timeout(timeout_msg)]).unwrap();
		for request in requests {
			// commitments should not be found in storage after timeout has been processed
			let commitment = hash_request::<Ismp>(&request);
			assert!(host.request_commitment(commitment).is_err())
		}
	})
}

#[test]
fn should_handle_get_request_responses_correctly() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let host = Ismp::default();
		setup_mock_client::<_, Test>(&host);
		let id = StateMachineId {
			state_id: StateMachine::Evm(11155111),
			consensus_state_id: MOCK_CONSENSUS_STATE_ID,
		};
		host.store_challenge_period(id, 0).unwrap();
		let requests = (0..2)
			.into_iter()
			.map(|i| {
				let msg = DispatchGet {
					dest: StateMachine::Evm(1),
					from: vec![0u8; 32],
					context: Default::default(),

					keys: vec![vec![1u8; 32], vec![1u8; 32]],
					height: 3,
					timeout: 2_000_000_000,
				};

				host.dispatch_request(
					DispatchRequest::Get(msg),
					FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
				)
				.unwrap();
				let get = ismp::router::GetRequest {
					source: host.host_state_machine(),
					dest: StateMachine::Evm(1),
					nonce: i,
					from: vec![0u8; 32],
					keys: vec![vec![1u8; 32], vec![1u8; 32]],
					height: 3,
					context: Default::default(),

					timeout_timestamp: Duration::from_millis(Timestamp::now()).as_secs() +
						2_000_000_000,
				};
				Request::Get(get)
			})
			.collect::<Vec<_>>();

		set_timestamp(Some(Duration::from_secs(100_000_000).as_millis() as u64));

		let (signature, public_key) =
			create_relayer_signer(RequestResponse::Request(requests.clone()).encode(), &[1u8; 32]);
		let initial_balance = 1000 * UNIT;
		let public_key_array: [u8; 32] =
			public_key.try_into().expect("Public key should be 32 bytes");
		Balances::mint_into(&public_key_array.into(), initial_balance).unwrap();

		let response = ResponseMessage {
			datagram: RequestResponse::Request(requests.clone()),
			proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId {
						state_id: StateMachine::Evm(1),
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: 3,
				},
				proof: vec![],
			},
			signer: signature,
		};

		pallet_ismp::Pallet::<Test>::execute(vec![Message::Response(response)]).unwrap();

		for request in requests {
			let Request::Get(get) = request else { panic!("Shouldn't be possible") };
			let response = Response::Get(GetResponse { get, values: Default::default() });
			assert!(host.response_receipt(&response).is_some())
		}
	})
}

#[test]
fn test_dispatch_fees_and_refunds() {
	let mut ext = new_test_ext();
	let account: AccountId32 = H256::random().0.into();
	let host = Ismp::default();

	ext.execute_with(|| {
		let msg = DispatchGet {
			dest: StateMachine::Evm(1),
			from: vec![0u8; 32],
			keys: vec![vec![1u8; 32], vec![1u8; 32]],
			context: Default::default(),
			height: 3,
			timeout: 2_000_000_000,
		};

		assert_eq!(Balances::balance(&account), Default::default());
		Balances::mint_into(&account, 10 * UNIT).unwrap();
		assert_eq!(Balances::balance(&account), 10 * UNIT);

		host.dispatch_request(
			DispatchRequest::Get(msg.clone()),
			// lets pay 10 units
			FeeMetadata { payer: account.clone().into(), fee: 10 * UNIT },
		)
		.unwrap();

		// we should no longer have it
		assert_eq!(Balances::balance(&account), Default::default());

		// now pallet-ismp has it
		assert_eq!(Balances::balance(&RELAYER_FEE_ACCOUNT.into_account_truncating()), 10 * UNIT);

		// fetch directly from pallet-mmr's buffer
		let Leaf::Request(request) = Mmr::intermediate_leaves(0).unwrap() else {
			panic!("Leaf not found!")
		};

		// ask the host to timeout the request
		host.ismp_router()
			.module_for_id(vec![])
			.unwrap()
			.on_timeout(Timeout::Request(request.clone()))
			.unwrap();

		// money should've been refunded to the account
		assert_eq!(Balances::balance(&account), 10 * UNIT);

		// unhappy case
		host.dispatch_request(
			DispatchRequest::Get(msg),
			// lets pay 10 units
			FeeMetadata { payer: account.clone().into(), fee: 10 * UNIT },
		)
		.unwrap();

		// we should no longer have it
		assert_eq!(Balances::balance(&account), Default::default());

		// now pallet-ismp has it
		assert_eq!(Balances::balance(&RELAYER_FEE_ACCOUNT.into_account_truncating()), 10 * UNIT);

		// ask the host to timeout the request, using the error module
		host.ismp_router()
			.module_for_id(ERROR_MODULE_ID.to_vec())
			.unwrap()
			.on_timeout(Timeout::Request(request.clone()))
			.unwrap_err();

		// pallet-ismp still has it
		assert_eq!(Balances::balance(&RELAYER_FEE_ACCOUNT.into_account_truncating()), 10 * UNIT);
	});
}

#[test]
fn test_fund_message() {
	let mut ext = new_test_ext();
	let account: AccountId32 = H256::random().0.into();
	let host = Ismp::default();

	ext.execute_with(|| {
		let msg = DispatchGet {
			dest: StateMachine::Evm(1),
			from: vec![0u8; 32],
			keys: vec![vec![1u8; 32], vec![1u8; 32]],
			context: Default::default(),
			height: 3,
			timeout: 2_000_000_000,
		};

		assert_eq!(Balances::balance(&account), Default::default());
		Balances::mint_into(&account, 20 * UNIT).unwrap();
		assert_eq!(Balances::balance(&account), 20 * UNIT);

		let commitment = host
			.dispatch_request(
				DispatchRequest::Get(msg.clone()),
				// lets pay 10 units
				FeeMetadata { payer: account.clone().into(), fee: 10 * UNIT },
			)
			.unwrap();

		// pallet-ismp now has it
		assert_eq!(Balances::balance(&RELAYER_FEE_ACCOUNT.into_account_truncating()), 10 * UNIT);

		// fund the request
		Ismp::fund_message(
			Origin::<Test>::Signed(account).into(),
			FundMessageParams {
				commitment: MessageCommitment::Request(commitment),
				amount: 10 * UNIT,
			},
		)
		.unwrap();

		assert_eq!(Balances::balance(&RELAYER_FEE_ACCOUNT.into_account_truncating()), 20 * UNIT);

		let metadata = RequestCommitments::<Test>::get(commitment).unwrap();
		assert_eq!(metadata.fee.fee, 20 * UNIT);
	});
}

#[test]
fn should_charge_fee_for_request() {
	new_test_ext().execute_with(|| {
		let host = Ismp::default();
		setup_mock_client::<_, Test>(&host);
		let id = StateMachineId {
			state_id: StateMachine::Evm(1),
			consensus_state_id: MOCK_CONSENSUS_STATE_ID,
		};

		let signer_pair = sp_core::sr25519::Pair::from_string("//Alice", None).unwrap();
		let signer_account: AccountId32 = signer_pair.public().into();
		let initial_balance = 1000 * UNIT;
		Balances::mint_into(&signer_account, initial_balance).unwrap();

		let treasury_pallet_id = TreasuryAccount::get();
		let treasury_account = treasury_pallet_id.into_account_truncating();
		let initial_treasury_balance = Balances::balance(&treasury_account);

		let post_request = PostRequest {
			source: id.state_id,
			dest: host.host_state_machine(),
			nonce: 0,
			from: vec![1; 32],
			to: vec![2; 32],
			timeout_timestamp: 0,
			body: b"body".to_vec(),
		};

		let requests = vec![post_request];
		let signed_data = keccak_256(&requests.encode());
		let signature = signer_pair.sign(&signed_data);
		let signature = Signature::Sr25519 {
			public_key: signer_pair.public().to_raw_vec(),
			signature: signature.to_raw_vec(),
		};

		let request_message = RequestMessage {
			requests,
			proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId { state_id: id.state_id, consensus_state_id: *b"mock" },
					height: 3,
				},
				proof: vec![],
			},
			signer: signature.encode(),
		};

		let message = Message::Request(request_message);

		let expected_fee = 50 * UNIT;

		pallet_ismp::Pallet::<Test>::handle_unsigned(RuntimeOrigin::none(), vec![message]).unwrap();

		let final_signer_balance = Balances::balance(&signer_account);
		let final_treasury_balance = Balances::balance(&treasury_account);

		assert_eq!(final_signer_balance, initial_balance - expected_fee);
		assert_eq!(final_treasury_balance, initial_treasury_balance + expected_fee);
	});
}
