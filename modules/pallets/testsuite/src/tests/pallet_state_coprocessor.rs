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

//! Tests for the early-validation checks inside
//! `pallet_state_coprocessor::handle_get_requests`. These cover the
//! per-request loop and the post-loop height check — all of which run
//! before any proof verification.

#![cfg(test)]

use std::collections::BTreeSet;

use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	host::{IsmpHost, StateMachine},
	messaging::{hash_request, Proof},
	router::{GetRequest, Request},
	Error,
};
use pallet_state_coprocessor::impls::GetRequestsWithProof;

use crate::runtime::{
	new_test_ext, set_timestamp, setup_mock_client, Ismp, StateCoprocessor, Test,
	MOCK_CONSENSUS_STATE_ID,
};

/// The state machine the mock client is configured for. Requests in
/// these tests both originate from and target this chain so the
/// (source, dest, source-proof, response-proof) tuple aligns by default.
const SOURCE_CHAIN: StateMachine = StateMachine::Evm(1);
const DEST_CHAIN: StateMachine = StateMachine::Evm(1);
/// Block height at which the mock client's commitment is registered.
const PROOF_HEIGHT: u64 = 3;
/// Far-future timeout so requests are not considered expired by default.
const FAR_FUTURE_TIMEOUT: u64 = 2_000_000_000;

fn proof_at(state_id: StateMachine, height: u64) -> Proof {
	Proof {
		height: StateMachineHeight {
			id: StateMachineId { state_id, consensus_state_id: MOCK_CONSENSUS_STATE_ID },
			height,
		},
		proof: vec![],
	}
}

fn build_request(
	nonce: u64,
	source: StateMachine,
	dest: StateMachine,
	height: u64,
	timeout: u64,
) -> GetRequest {
	GetRequest {
		source,
		dest,
		nonce,
		from: vec![0xAB; 20],
		keys: vec![vec![1u8; 32]],
		height,
		context: Default::default(),
		timeout_timestamp: timeout,
	}
}

/// Standard message that would pass every per-request check; tweak one
/// field per test to assert the corresponding rejection.
fn valid_message() -> GetRequestsWithProof {
	let mut requests = BTreeSet::new();
	requests.insert(build_request(0, SOURCE_CHAIN, DEST_CHAIN, PROOF_HEIGHT, FAR_FUTURE_TIMEOUT));
	GetRequestsWithProof {
		requests,
		source: proof_at(SOURCE_CHAIN, PROOF_HEIGHT),
		response: proof_at(DEST_CHAIN, PROOF_HEIGHT),
		address: vec![0u8; 32],
	}
}

/// Returns a copy of the message with its single request mutated.
fn with_mutated_request(
	mut msg: GetRequestsWithProof,
	mutate: impl FnOnce(&mut GetRequest),
) -> GetRequestsWithProof {
	let mut req = msg.requests.iter().next().cloned().expect("valid_message has one request");
	mutate(&mut req);
	msg.requests = BTreeSet::from([req]);
	msg
}

#[test]
fn rejects_timed_out_get_requests() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let host = Ismp::default();
		setup_mock_client::<_, Test>(&host);

		// Host clock at 1s past the request timeout.
		let now_secs = 100u64;
		set_timestamp::<Test>(now_secs.saturating_mul(1_000));

		let msg = with_mutated_request(valid_message(), |req| req.timeout_timestamp = now_secs - 1);

		let err = StateCoprocessor::handle_get_requests(msg).expect_err("must fail");
		assert!(matches!(err, Error::RequestTimeout { .. }), "unexpected error: {err:?}");
	});
}

/// SCALE-encodes a `GetRequestsWithProof` payload that contains a
/// duplicate `GetRequest` (constructed by hand: a `Vec` length prefix
/// of 2 with the same request body twice), then decodes it into the
/// real `BTreeSet`-backed struct. The decoder's `BTreeSet::insert` call
/// drops the duplicate before the pallet sees it, so an adversary
/// cannot smuggle repeats through the SCALE layer — and the pallet
/// never has to perform a separate batch-dedup check.
#[test]
fn scale_decoding_duplicate_requests_into_btreeset_dedupes_before_dispatch() {
	use codec::{Compact, Decode, Encode};

	let req = build_request(7, SOURCE_CHAIN, DEST_CHAIN, PROOF_HEIGHT, FAR_FUTURE_TIMEOUT);

	// Hand-rolled SCALE payload: compact-encoded len=2, then the request bytes twice,
	// followed by the rest of the GetRequestsWithProof fields. This is the shape an
	// attacker would produce if they swapped a `BTreeSet` for a `Vec` on the wire.
	let mut encoded = Vec::new();
	Compact(2u32).encode_to(&mut encoded);
	req.encode_to(&mut encoded);
	req.encode_to(&mut encoded);
	proof_at(SOURCE_CHAIN, PROOF_HEIGHT).encode_to(&mut encoded);
	proof_at(DEST_CHAIN, PROOF_HEIGHT).encode_to(&mut encoded);
	(vec![0u8; 32]).encode_to(&mut encoded);

	let decoded = GetRequestsWithProof::decode(&mut &encoded[..]).expect("decode should succeed");

	// The duplicate collapsed at decode time — the BTreeSet holds one entry.
	assert_eq!(decoded.requests.len(), 1);
	assert!(decoded.requests.contains(&req));
}

#[test]
fn rejects_request_when_source_mismatches_source_proof() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let host = Ismp::default();
		setup_mock_client::<_, Test>(&host);

		// Source-chain on the request disagrees with the source proof's state machine.
		let msg = with_mutated_request(valid_message(), |req| req.source = StateMachine::Evm(42));

		let err = StateCoprocessor::handle_get_requests(msg).expect_err("must fail");
		assert!(
			matches!(err, Error::RequestProofMetadataNotValid { .. }),
			"unexpected error: {err:?}"
		);
	});
}

#[test]
fn rejects_request_when_dest_mismatches_response_proof() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let host = Ismp::default();
		setup_mock_client::<_, Test>(&host);

		// Response proof must come from the chain whose state was
		// being queried (the request's destination); pointing it
		// elsewhere should be rejected.
		let msg = with_mutated_request(valid_message(), |req| req.dest = StateMachine::Evm(42));

		let err = StateCoprocessor::handle_get_requests(msg).expect_err("must fail");
		assert!(
			matches!(err, Error::RequestProofMetadataNotValid { .. }),
			"unexpected error: {err:?}"
		);
	});
}

#[test]
fn rejects_request_already_processed_previously() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let host = Ismp::default();
		setup_mock_client::<_, Test>(&host);

		let msg = valid_message();
		// Pre-seed a receipt for the request so the coprocessor sees it as already serviced.
		let req = msg.requests.iter().next().cloned().expect("valid_message has one request");
		let full = Request::Get(req);
		let _ = host
			.store_request_receipt(&full, &vec![0u8; 32])
			.expect("seeding receipt should succeed");
		assert!(host.request_receipt(&full).is_some());
		// Sanity: commitment computation uses the configured hasher.
		let _ = hash_request::<Ismp>(&full);

		let err = StateCoprocessor::handle_get_requests(msg).expect_err("must fail");
		assert!(matches!(err, Error::DuplicateResponse { .. }), "unexpected error: {err:?}");
	});
}

#[test]
fn rejects_when_request_height_does_not_match_response_proof_height() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let host = Ismp::default();
		setup_mock_client::<_, Test>(&host);

		// Two requests at different heights — only one matches the proof.
		let mut msg = valid_message();
		msg.requests = BTreeSet::from([
			build_request(0, SOURCE_CHAIN, DEST_CHAIN, PROOF_HEIGHT, FAR_FUTURE_TIMEOUT),
			build_request(1, SOURCE_CHAIN, DEST_CHAIN, PROOF_HEIGHT + 1, FAR_FUTURE_TIMEOUT),
		]);

		let err = StateCoprocessor::handle_get_requests(msg).expect_err("must fail");
		assert!(matches!(err, Error::InsufficientProofHeight), "unexpected error: {err:?}");
	});
}
