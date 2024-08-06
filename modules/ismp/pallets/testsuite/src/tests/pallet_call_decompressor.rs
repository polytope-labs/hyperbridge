// Copyright (c) 2024 Polytope Labs.
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

use crate::{
	runtime::{
		new_test_ext, Ismp, RuntimeCall, RuntimeOrigin, Test, Timestamp, MOCK_CONSENSUS_STATE_ID,
	},
	tests::pallet_ismp_relayer::{encode_accumulate_fees_call, read_file_string},
};
use codec::Encode;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	host::{IsmpHost, StateMachine},
	messaging::{Message, Proof, RequestMessage, ResponseMessage, TimeoutMessage},
	router::{PostResponse, Request, RequestResponse},
};
use ruzstd::StreamingDecoder;
use sp_core::{H256, H512};
use sp_runtime::{DispatchError, ModuleError};
use std::{
	io::Read,
	time::{Duration, Instant},
};
use zstd_safe::WriteBuf;

#[test]
fn compress_benchmark_with_zstd_safe() {
	new_test_ext().execute_with(|| {
		let start_time = Instant::now();
		let mut buffer = [0u8; 256000];
		let proof_str = read_file_string("src/tests/proofs/accumulate_fee_proof.txt");
		let compressed_proof = zstd_safe::compress(&mut buffer, proof_str.as_bytes(), 3).unwrap();
		let compressed_proof = &buffer[..compressed_proof];
		let end_time = Instant::now();
		let duration = end_time - start_time;
		println!("time taken for compression with zstd_safe {:?}", duration);
		assert!(proof_str.as_bytes().to_vec().len() > compressed_proof.len());

		let start_time = Instant::now();
		let mut buffer = vec![0u8; 25600000000];
		let written = zstd_safe::decompress(&mut buffer[..], compressed_proof).unwrap();
		let decompressed_data = &buffer[..written];
		let end_time = Instant::now();

		let duration = end_time - start_time;
		println!("time taken for decompression with zstd_safe {:?}", duration);

		let start_time = Instant::now();
		let mut decoder = StreamingDecoder::new(compressed_proof.as_slice()).unwrap();
		let mut result = vec![0u8; 2000000];
		let read = decoder.read(&mut result).unwrap();
		dbg!(read / 1000);

		let end_time = Instant::now();
		let duration = end_time - start_time;
		println!("time taken for decompression with ruzstd {:?}", duration);

		dbg!(decompressed_data.len() / 1000);
		assert_eq!(proof_str.as_bytes().to_vec(), decompressed_data);
		assert_eq!(proof_str.as_bytes(), &result[..read]);
	});
}

#[test]
#[ignore]
fn decompress_and_execute_call() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let encoded_call = encode_accumulate_fees_call();

		let mut buffer = vec![0u8; 100000];
		let compressed_call =
			zstd_safe::compress(&mut buffer[..], encoded_call.as_slice(), 3).unwrap();
		let compressed_call = &buffer[..compressed_call];

		pallet_call_decompressor::Pallet::<Test>::decompress_call(
			RuntimeOrigin::none(),
			compressed_call.to_vec(),
			encoded_call.len() as u32,
		)
		.unwrap();
	});
}

#[test]
fn should_decompress_and_execute_pallet_ismp_get_response_calls_correctly() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let host = Ismp::default();
		let requests = (0..100)
			.into_iter()
			.map(|i| {
				let get = ismp::router::GetRequest {
					source: host.host_state_machine(),
					dest: StateMachine::Evm(1),
					nonce: i,
					from: H256::random().0.to_vec(),
					keys: { (0..256).into_iter().map(|_| H256::random().0.to_vec()).collect() },
					height: 3,
					timeout_timestamp: Duration::from_millis(Timestamp::now()).as_secs() +
						2_000_000_000,
				};
				Request::Get(get)
			})
			.collect::<Vec<_>>();

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
				index: 11,
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
			.map(|i| {
				let get = ismp::router::GetRequest {
					source: host.host_state_machine(),
					dest: StateMachine::Evm(1),
					nonce: i,
					from: H256::random().0.to_vec(),
					keys: { (0..256).into_iter().map(|_| H256::random().0.to_vec()).collect() },
					height: 3,
					timeout_timestamp: Duration::from_millis(Timestamp::now()).as_secs() +
						2_000_000_000,
				};
				Request::Get(get)
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
				index: 11,
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
				index: 11,
				error: [1, 0, 0, 0],
				message: Some("ErrorExecutingCall")
			})
		);
	})
}

#[test]
fn should_decompress_and_execute_pallet_ismp_post_response_calls_correctly() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let host = Ismp::default();
		let responses = (0..1000)
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
				ismp::router::Response::Post(PostResponse {
					post,
					response: H512::random().0.to_vec(),
					timeout_timestamp: 200000,
				})
			})
			.collect::<Vec<_>>();

		let msg = ResponseMessage {
			datagram: RequestResponse::Response(responses),
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
			messages: vec![Message::Response(msg)],
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
				index: 11,
				error: [1, 0, 0, 0],
				message: Some("ErrorExecutingCall")
			})
		);
	})
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

		let mut buffer = vec![0u8; 1000000];
		let compressed =
			zstd_safe::compress(&mut buffer[..], nested_calls.encode().as_slice(), 3).unwrap();
		let final_compressed_call = buffer[..compressed].to_vec();

		let res = pallet_call_decompressor::Pallet::<Test>::decompress_call(
			RuntimeOrigin::none(),
			final_compressed_call.to_vec(),
			1000000,
		)
		.err()
		.unwrap();

		assert_eq!(
			res,
			DispatchError::Module(ModuleError {
				index: 11,
				error: [3, 0, 0, 0],
				message: Some("ErrorDecodingCall")
			})
		);
	});
}
