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

use crate::runtime::{
	new_test_ext, Ismp, RuntimeCall, RuntimeOrigin, Test, Timestamp, MOCK_CONSENSUS_STATE_ID,
};
use codec::Encode;
use frame_support::traits::Time;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	host::{IsmpHost, StateMachine},
	messaging::{Message, Proof, RequestMessage, ResponseMessage, TimeoutMessage},
};
use sp_core::{H256, H512};
use sp_runtime::{DispatchError, ModuleError};
use std::time::Duration;

#[test]
fn should_decompress_and_execute_pallet_ismp_get_response_calls_correctly() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let host = Ismp::default();
		let requests = (0..100)
			.into_iter()
			.map(|i| ismp::router::GetRequest {
				source: host.host_state_machine(),
				dest: StateMachine::Evm(1),
				nonce: i,
				from: H256::random().0.to_vec(),
				keys: { (0..256).into_iter().map(|_| H256::random().0.to_vec()).collect() },
				height: 3,
				context: Default::default(),

				timeout_timestamp: Duration::from_millis(Timestamp::now()).as_secs() +
					2_000_000_000,
			})
			.collect::<Vec<_>>();

		let response = ResponseMessage {
			requests: requests.clone(),
			proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId {
						state_id: StateMachine::Evm(1),
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: 3,
				},
				proof: H512::random().0.to_vec(),
			},
			signer: H512::random().0.to_vec(),
		};

		let call = RuntimeCall::Ismp(pallet_ismp::Call::handle_unsigned {
			messages: vec![Message::Response(response)],
		})
		.encode();
		let mut buffer = vec![0u8; 1_000_000];
		let compressed = zstd_safe::compress(&mut buffer[..], &call, 3).unwrap();
		let final_compressed_call = buffer[..compressed].to_vec();

		let res = pallet_call_decompressor::Pallet::<Test>::decompress_call(
			RuntimeOrigin::none(),
			final_compressed_call.to_vec(),
			call.len() as u32,
		)
		.err()
		.unwrap();

		// Decoding the call was completed without errors
		assert_eq!(
			res,
			DispatchError::Module(ModuleError {
				index: 10,
				error: [1, 0, 0, 0],
				message: Some("ErrorExecutingCall")
			})
		);
	})
}

#[test]
fn should_decompress_and_execute_pallet_ismp_get_time_out_calls_correctly() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let host = Ismp::default();
		let requests = (0..100)
			.into_iter()
			.map(|i| ismp::router::GetRequest {
				source: host.host_state_machine(),
				dest: StateMachine::Evm(1),
				nonce: i,
				from: H256::random().0.to_vec(),
				keys: { (0..256).into_iter().map(|_| H256::random().0.to_vec()).collect() },
				height: 3,
				context: Default::default(),

				timeout_timestamp: Duration::from_millis(Timestamp::now()).as_secs() +
					2_000_000_000,
			})
			.collect::<Vec<_>>();

		let msg = TimeoutMessage::Get { requests };

		let call = RuntimeCall::Ismp(pallet_ismp::Call::handle_unsigned {
			messages: vec![Message::Timeout(msg)],
		})
		.encode();
		let mut buffer = vec![0u8; 1_000_000];
		let compressed = zstd_safe::compress(&mut buffer[..], &call, 3).unwrap();
		let final_compressed_call = buffer[..compressed].to_vec();

		let res = pallet_call_decompressor::Pallet::<Test>::decompress_call(
			RuntimeOrigin::none(),
			final_compressed_call.to_vec(),
			call.len() as u32,
		)
		.err()
		.unwrap();

		// Decoding the call was completed without errors
		assert_eq!(
			res,
			DispatchError::Module(ModuleError {
				index: 10,
				error: [1, 0, 0, 0],
				message: Some("ErrorExecutingCall")
			})
		);
	})
}

#[test]
fn should_decompress_and_execute_pallet_ismp_post_request_calls_correctly() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let host = Ismp::default();
		let requests = (0..1000)
			.into_iter()
			.map(|i| {
				let post = ismp::router::PostRequest {
					source: host.host_state_machine(),
					dest: StateMachine::Evm(1),
					nonce: i,
					from: H256::random().0.to_vec(),
					to: H256::random().0.to_vec(),
					timeout_timestamp: Duration::from_millis(Timestamp::now()).as_secs() +
						2_000_000_000,
					body: H512::random().0.to_vec(),
				};
				post
			})
			.collect::<Vec<_>>();

		let msg = RequestMessage {
			requests,
			proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId {
						state_id: StateMachine::Evm(1),
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: 3,
				},
				proof: H512::random().0.to_vec(),
			},
			signer: H512::random().0.to_vec(),
		};

		let call = RuntimeCall::Ismp(pallet_ismp::Call::handle_unsigned {
			messages: vec![Message::Request(msg)],
		})
		.encode();
		let mut buffer = vec![0u8; 1_000_000];
		let compressed = zstd_safe::compress(&mut buffer[..], &call, 3).unwrap();
		let final_compressed_call = buffer[..compressed].to_vec();

		let res = pallet_call_decompressor::Pallet::<Test>::decompress_call(
			RuntimeOrigin::none(),
			final_compressed_call.to_vec(),
			call.len() as u32,
		)
		.err()
		.unwrap();

		// Decoding the call was completed without errors
		assert_eq!(
			res,
			DispatchError::Module(ModuleError {
				index: 10,
				error: [1, 0, 0, 0],
				message: Some("ErrorExecutingCall")
			})
		);
	})
}

#[test]
fn should_reject_decompression_when_actual_size_diverges_from_claim() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let host = Ismp::default();
		let post = ismp::router::PostRequest {
			source: host.host_state_machine(),
			dest: StateMachine::Evm(1),
			nonce: 0,
			from: H256::random().0.to_vec(),
			to: H256::random().0.to_vec(),
			timeout_timestamp: Duration::from_millis(Timestamp::now()).as_secs() + 2_000_000_000,
			body: H512::random().0.to_vec(),
		};

		let msg = RequestMessage {
			requests: vec![post],
			proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId {
						state_id: StateMachine::Evm(1),
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: 3,
				},
				proof: H512::random().0.to_vec(),
			},
			signer: H512::random().0.to_vec(),
		};

		let call = RuntimeCall::Ismp(pallet_ismp::Call::handle_unsigned {
			messages: vec![Message::Request(msg)],
		})
		.encode();

		let mut buffer = vec![0u8; 1_000_000];
		let compressed = zstd_safe::compress(&mut buffer[..], &call, 3).unwrap();
		let final_compressed_call = buffer[..compressed].to_vec();

		let inflated_size = (call.len() as u32) + 100_000;

		let res = pallet_call_decompressor::Pallet::<Test>::decompress_call(
			RuntimeOrigin::none(),
			final_compressed_call,
			inflated_size,
		)
		.err()
		.unwrap();

		assert_eq!(
			res,
			DispatchError::Module(ModuleError {
				index: 10,
				error: [2, 0, 0, 0],
				message: Some("DecompressionFailed")
			})
		);
	})
}

#[test]
fn decompress_rejects_oversize_claim_before_decompressing() {
	use frame_support::pallet_prelude::{InvalidTransaction, TransactionSource, ValidateUnsigned};

	let mut ext = new_test_ext();
	ext.execute_with(|| {
		// A tiny, perfectly valid zstd frame; the inflated *claim* is the attack.
		let payload = b"a small decompressible payload".to_vec();
		let mut buffer = vec![0u8; 1_000];
		let compressed = zstd_safe::compress(&mut buffer[..], &payload, 3).unwrap();
		let compressed = buffer[..compressed].to_vec();

		// `u32::MAX` is far above `MaxCallSize * ONE_MB`. The gate inside
		// `decompress` must reject it up-front (`CallSizeOutOfBound`) before the
		// decompression loop expands anything — this is the mempool zstd-bomb guard.
		let res =
			pallet_call_decompressor::Pallet::<Test>::decompress(compressed.clone(), u32::MAX);
		assert!(
			matches!(
				res,
				Err(DispatchError::Module(ModuleError { message: Some("CallSizeOutOfBound"), .. }))
			),
			"oversize claim must be rejected by the size gate, got {res:?}",
		);

		// And the unsigned mempool path (`validate_unsigned`) must reject it too,
		// without performing the decompression first.
		let call = pallet_call_decompressor::Call::<Test>::decompress_call {
			compressed,
			encoded_call_size: u32::MAX,
		};
		let validity =
			<pallet_call_decompressor::Pallet<Test> as ValidateUnsigned>::validate_unsigned(
				TransactionSource::External,
				&call,
			);
		assert_eq!(validity, Err(InvalidTransaction::Call.into()));
	});
}

#[test]
fn decompress_stack_exhaustion_poc() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		use crate::runtime::RuntimeCall;
		use codec::Encode;

		let inner_call = RuntimeCall::System(frame_system::Call::remark { remark: Vec::new() });

		let mut nested_calls =
			RuntimeCall::Sudo(pallet_sudo::Call::sudo { call: Box::new(inner_call) });

		for _ in 1..1000 {
			nested_calls =
				RuntimeCall::Sudo(pallet_sudo::Call::sudo { call: Box::new(nested_calls) });
		}

		let encoded = nested_calls.encode();
		let mut buffer = vec![0u8; 1000000];
		let compressed = zstd_safe::compress(&mut buffer[..], encoded.as_slice(), 3).unwrap();
		let final_compressed_call = buffer[..compressed].to_vec();

		let res = pallet_call_decompressor::Pallet::<Test>::decompress_call(
			RuntimeOrigin::none(),
			final_compressed_call.to_vec(),
			encoded.len() as u32,
		)
		.err()
		.unwrap();

		assert_eq!(
			res,
			DispatchError::Module(ModuleError {
				index: 10,
				error: [3, 0, 0, 0],
				message: Some("ErrorDecodingCall")
			})
		);
	});
}
