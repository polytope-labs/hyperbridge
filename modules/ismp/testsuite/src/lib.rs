// Copyright (C) Polytope Labs Ltd.
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

//! ISMP Testsuite

use codec::Encode;
use crypto_utils::verification::Signature;
use polkadot_sdk::{
	sp_core::{sr25519, Pair},
	sp_io,
};
use std::vec;

use ismp::{
	consensus::{
		ConsensusStateId, IntermediateState, StateCommitment, StateMachineHeight, StateMachineId,
	},
	dispatcher::{DispatchPost, DispatchRequest, FeeMetadata, IsmpDispatcher},
	error::Error,
	handlers::handle_incoming_message,
	host::{IsmpHost, StateMachine},
	messaging::{
		hash_post_response, hash_request, ConsensusMessage, FraudProofMessage, Message, Proof,
		RequestMessage, ResponseMessage, TimeoutMessage,
	},
	router::{PostRequest, PostResponse, Request, RequestResponse, Response},
};

use crate::mocks::{Host, MOCK_CONSENSUS_CLIENT_ID, MOCK_PROXY_CONSENSUS_CLIENT_ID};

pub mod mocks;
#[cfg(test)]
mod tests;

fn mock_consensus_state_id() -> ConsensusStateId {
	*b"mock"
}

fn mock_proxy_consensus_state_id() -> ConsensusStateId {
	*b"prox"
}

fn setup_mock_client<H: IsmpHost>(host: &H) -> IntermediateState {
	let intermediate_state = IntermediateState {
		height: StateMachineHeight {
			id: StateMachineId {
				state_id: StateMachine::Evm(11155111),
				consensus_state_id: mock_consensus_state_id(),
			},
			height: 1,
		},
		commitment: StateCommitment {
			timestamp: 1000,
			overlay_root: None,
			state_root: Default::default(),
		},
	};

	host.store_consensus_state(mock_consensus_state_id(), vec![]).unwrap();
	host.store_consensus_state_id(mock_consensus_state_id(), MOCK_CONSENSUS_CLIENT_ID)
		.unwrap();
	host.store_state_machine_commitment(intermediate_state.height, intermediate_state.commitment)
		.unwrap();

	intermediate_state
}

fn setup_mock_proxy_client<H: IsmpHost>(
	host: &H,
	proxy_state_machine: StateMachine,
) -> IntermediateState {
	let proxy = IntermediateState {
		height: StateMachineHeight {
			id: StateMachineId {
				state_id: proxy_state_machine,
				consensus_state_id: mock_proxy_consensus_state_id(),
			},
			height: 1,
		},
		commitment: StateCommitment {
			timestamp: 1000,
			overlay_root: None,
			state_root: Default::default(),
		},
	};

	host.store_consensus_state(mock_proxy_consensus_state_id(), vec![]).unwrap();
	host.store_consensus_state_id(mock_proxy_consensus_state_id(), MOCK_PROXY_CONSENSUS_CLIENT_ID)
		.unwrap();
	host.store_state_machine_commitment(proxy.height, proxy.commitment).unwrap();
	proxy
}

/*
	Consensus Client and State Machine checks
*/

/// Ensure challenge period rules are followed in all handlers
pub fn check_challenge_period<H: IsmpHost>(host: &H) -> Result<(), &'static str> {
	let intermediate_state = setup_mock_client(host);
	// Set the previous update time
	let challenge_period = host.challenge_period(intermediate_state.height.id).unwrap();
	let previous_update_time = host.timestamp() - (challenge_period / 2);
	host.store_consensus_update_time(mock_consensus_state_id(), previous_update_time)
		.unwrap();
	host.store_state_machine_update_time(intermediate_state.height, previous_update_time)
		.unwrap();

	let post = PostRequest {
		source: intermediate_state.height.id.state_id,
		dest: host.host_state_machine(),
		nonce: 0,
		from: vec![0u8; 32],
		to: vec![0u8; 32],
		timeout_timestamp: 0,
		body: vec![0u8; 64],
	};
	let request = Request::Post(post.clone());

	let (signature, ..) = create_relayer_signer(vec![post.clone()].encode(), &[1u8; 32]);

	// Request message handling check
	let request_message = Message::Request(RequestMessage {
		requests: vec![post.clone()],
		proof: Proof { height: intermediate_state.height, proof: vec![] },
		signer: signature,
	});

	let res = handle_incoming_message(host, request_message).map_err(|e| e.downcast().unwrap());

	assert!(matches!(res, Err(ismp::error::Error::ChallengePeriodNotElapsed { .. })));

	let response_message = RequestResponse::Response(vec![Response::Post(PostResponse {
		post,
		response: vec![],
		timeout_timestamp: 0,
	})]);

	let (signature, ..) = create_relayer_signer(response_message.encode(), &[1u8; 32]);

	// Response message handling check
	let response_message = Message::Response(ResponseMessage {
		datagram: response_message,
		proof: Proof { height: intermediate_state.height, proof: vec![] },
		signer: signature,
	});

	let res = handle_incoming_message(host, response_message).map_err(|e| e.downcast().unwrap());
	assert!(matches!(res, Err(ismp::error::Error::ChallengePeriodNotElapsed { .. })));

	// Timeout mesaage handling check
	let timeout_message = Message::Timeout(TimeoutMessage::Post {
		requests: vec![request],
		timeout_proof: Proof { height: intermediate_state.height, proof: vec![] },
	});

	let res = handle_incoming_message(host, timeout_message).map_err(|e| e.downcast().unwrap());
	assert!(matches!(res, Err(ismp::error::Error::ChallengePeriodNotElapsed { .. })));
	Ok(())
}

/// Ensure expired client rules are followed in consensus update
pub fn check_client_expiry<H: IsmpHost>(host: &H) -> Result<(), &'static str> {
	let consensus_message = Message::Consensus(ConsensusMessage {
		consensus_proof: vec![],
		consensus_state_id: mock_consensus_state_id(),
		signer: vec![],
	});
	setup_mock_client(host);
	// Set the previous update time
	let unbonding_period = host.unbonding_period(mock_consensus_state_id()).unwrap();
	let previous_update_time = host.timestamp() - unbonding_period;
	host.store_consensus_update_time(mock_consensus_state_id(), previous_update_time)
		.unwrap();

	let res =
		handle_incoming_message::<H>(host, consensus_message).map_err(|e| e.downcast().unwrap());
	assert!(matches!(res, Err(ismp::error::Error::UnbondingPeriodElapsed { .. })));

	Ok(())
}

pub fn frozen_consensus_client_check<H: IsmpHost>(host: &H) -> Result<(), &'static str> {
	let intermediate_state = setup_mock_client(host);
	// Set the previous update time
	let challenge_period = host.challenge_period(intermediate_state.height.id).unwrap();
	let previous_update_time = host.timestamp() - (challenge_period * 2);
	host.store_consensus_update_time(mock_consensus_state_id(), previous_update_time)
		.unwrap();
	host.store_state_machine_update_time(intermediate_state.height, previous_update_time)
		.unwrap();
	host.freeze_consensus_client(mock_consensus_state_id()).unwrap();

	let post = PostRequest {
		source: intermediate_state.height.id.state_id,
		dest: host.host_state_machine(),
		nonce: 0,
		from: vec![0u8; 32],
		to: vec![0u8; 32],
		timeout_timestamp: 0,
		body: vec![0u8; 64],
	};
	let (signature, ..) = create_relayer_signer(vec![post.clone()].encode(), &[1u8; 32]);

	// Request message handling check
	let request_message = Message::Request(RequestMessage {
		requests: vec![post.clone()],
		proof: Proof { height: intermediate_state.height, proof: vec![] },
		signer: signature,
	});

	let res = handle_incoming_message(host, request_message).map_err(|e| e.downcast().unwrap());
	dbg!(&res);
	assert!(matches!(res, Err(ismp::error::Error::FrozenConsensusClient { .. })));
	Ok(())
}

/// Missing state commitments
pub fn missing_state_commitment_check<H: IsmpHost>(host: &H) -> Result<(), &'static str> {
	let intermediate_state = setup_mock_client(host);
	// Set the previous update time
	let challenge_period = host.challenge_period(intermediate_state.height.id).unwrap();
	let previous_update_time = host.timestamp() - (challenge_period * 2);
	host.store_consensus_update_time(mock_consensus_state_id(), previous_update_time)
		.unwrap();
	host.store_state_machine_update_time(intermediate_state.height, previous_update_time)
		.unwrap();
	host.delete_state_commitment(intermediate_state.height).unwrap();

	let post = PostRequest {
		source: intermediate_state.height.id.state_id,
		dest: host.host_state_machine(),
		nonce: 0,
		from: vec![0u8; 32],
		to: vec![0u8; 32],
		timeout_timestamp: 0,
		body: vec![0u8; 64],
	};
	let request = Request::Post(post.clone());

	let (signature, ..) = create_relayer_signer(vec![post.clone()].encode(), &[1u8; 32]);

	// Request message handling check
	let request_message = Message::Request(RequestMessage {
		requests: vec![post.clone()],
		proof: Proof { height: intermediate_state.height, proof: vec![] },
		signer: signature,
	});

	let res = handle_incoming_message(host, request_message).map_err(|e| e.downcast().unwrap());

	assert!(matches!(res, Err(ismp::error::Error::StateCommitmentNotFound { .. })));

	let response_message = RequestResponse::Response(vec![Response::Post(PostResponse {
		post,
		response: vec![],
		timeout_timestamp: 0,
	})]);
	let (signature, ..) = create_relayer_signer(response_message.encode(), &[1u8; 32]);

	// Response message handling check
	let response_message = Message::Response(ResponseMessage {
		datagram: response_message,
		proof: Proof { height: intermediate_state.height, proof: vec![] },
		signer: signature,
	});

	let res = handle_incoming_message(host, response_message).map_err(|e| e.downcast().unwrap());
	assert!(matches!(res, Err(ismp::error::Error::StateCommitmentNotFound { .. })));

	// Timeout mesaage handling check
	let timeout_message = Message::Timeout(TimeoutMessage::Post {
		requests: vec![request],
		timeout_proof: Proof { height: intermediate_state.height, proof: vec![] },
	});

	let res = handle_incoming_message(host, timeout_message).map_err(|e| e.downcast().unwrap());
	assert!(matches!(res, Err(ismp::error::Error::StateCommitmentNotFound { .. })));

	Ok(())
}

/// Ensure post request timeouts are handled properly
pub fn post_request_timeout_check<H>(host: &H) -> Result<(), &'static str>
where
	H: IsmpHost + IsmpDispatcher,
	H::Account: From<[u8; 32]>,
	H::Balance: From<u32> + Default,
{
	let intermediate_state = setup_mock_client(host);
	let challenge_period = host.challenge_period(intermediate_state.height.id).unwrap();
	let previous_update_time = host.timestamp().saturating_sub(challenge_period * 2);
	host.store_consensus_update_time(mock_consensus_state_id(), previous_update_time)
		.unwrap();
	host.store_state_machine_update_time(intermediate_state.height, previous_update_time)
		.unwrap();
	let dispatch_post = DispatchPost {
		dest: intermediate_state.height.id.state_id,
		from: vec![0u8; 32],
		to: vec![0u8; 32],
		timeout: intermediate_state.commitment.timestamp,
		body: vec![0u8; 64],
	};
	let post = PostRequest {
		source: host.host_state_machine(),
		dest: intermediate_state.height.id.state_id,
		nonce: 0,
		from: vec![0u8; 32],
		to: vec![0u8; 32],
		timeout_timestamp: intermediate_state.commitment.timestamp,
		body: vec![0u8; 64],
	};
	let request = Request::Post(post);
	let dispatch_request = DispatchRequest::Post(dispatch_post);
	host.dispatch_request(
		dispatch_request,
		FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
	)
	.unwrap();

	// Timeout message handling check
	let timeout_message = Message::Timeout(TimeoutMessage::Post {
		requests: vec![request.clone()],
		timeout_proof: Proof { height: intermediate_state.height, proof: vec![] },
	});

	handle_incoming_message(host, timeout_message).unwrap();

	// Assert that request commitment was deleted
	let commitment = hash_request::<H>(&request);
	let res = host.request_commitment(commitment);
	assert!(matches!(res, Err(..)));
	Ok(())
}

pub fn fraud_proof_checks<H>(host: &H)
where
	H: IsmpHost,
{
	setup_mock_client(host);

	let fraud_proof = FraudProofMessage {
		proof_1: vec![],
		proof_2: vec![],
		consensus_state_id: mock_consensus_state_id(),
		signer: vec![],
	};

	assert!(handle_incoming_message(host, Message::FraudProof(fraud_proof.clone())).is_ok());

	assert!(matches!(
		handle_incoming_message(host, Message::FraudProof(fraud_proof))
			.map_err(|e| e.downcast().unwrap()),
		Err(Error::FrozenConsensusClient { .. })
	));
}

/// Ensure post request timeouts are handled properly
pub fn post_response_timeout_check<H>(host: &H) -> Result<(), &'static str>
where
	H: IsmpHost + IsmpDispatcher,
	H::Account: From<[u8; 32]>,
	H::Balance: From<u32> + Default,
{
	let intermediate_state = setup_mock_client(host);
	let challenge_period = host.challenge_period(intermediate_state.height.id).unwrap();
	let previous_update_time = host.timestamp() - (challenge_period * 2);
	host.store_consensus_update_time(mock_consensus_state_id(), previous_update_time)
		.unwrap();
	host.store_state_machine_update_time(intermediate_state.height, previous_update_time)
		.unwrap();

	let request = PostRequest {
		source: intermediate_state.height.id.state_id,
		dest: host.host_state_machine(),
		nonce: 0,
		from: vec![0u8; 32],
		to: vec![0u8; 32],
		timeout_timestamp: 0,
		body: vec![0u8; 64],
	};

	let (signature, ..) = create_relayer_signer(vec![request.clone()].encode(), &[1u8; 32]);

	let request_message = Message::Request(RequestMessage {
		requests: vec![request.clone()],
		proof: Proof { height: intermediate_state.height, proof: vec![] },
		signer: signature,
	});

	handle_incoming_message(host, request_message).unwrap();
	// Assert that request was acknowledged
	assert!(matches!(host.request_receipt(&Request::Post(request.clone())), Some(_)));

	let response = PostResponse { post: request, response: vec![], timeout_timestamp: 100 };
	host.dispatch_response(
		response.clone(),
		FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
	)
	.unwrap();

	let timeout_message = Message::Timeout(TimeoutMessage::PostResponse {
		responses: vec![response.clone()],
		timeout_proof: Proof { height: intermediate_state.height, proof: vec![] },
	});

	handle_incoming_message(host, timeout_message).unwrap();

	// Assert that response commitment was deleted
	let commitment = hash_post_response::<H>(&response);
	let res = host.response_commitment(commitment);
	assert!(matches!(res, Err(..)));
	Ok(())
}

/*
	Check correctness of router implementation
*/

/// Check that dispatcher stores commitments for outgoing requests and responses and rejects
/// duplicate responses
pub fn write_outgoing_commitments<H>(host: &H) -> Result<(), &'static str>
where
	H: IsmpHost + IsmpDispatcher,
	H::Account: From<[u8; 32]>,
	H::Balance: From<u32> + Default,
{
	let post = DispatchPost {
		dest: StateMachine::Kusama(2000),
		from: vec![0u8; 32],
		to: vec![0u8; 32],
		timeout: 0,
		body: vec![0u8; 64],
	};
	let dispatch_request = DispatchRequest::Post(post);
	// Dispatch the request the first time
	host.dispatch_request(
		dispatch_request,
		FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
	)
	.map_err(|_| "Dispatcher failed to dispatch request")?;
	// Fetch commitment from storage
	let post = PostRequest {
		source: host.host_state_machine(),
		dest: StateMachine::Kusama(2000),
		nonce: 0,
		from: vec![0u8; 32],
		to: vec![0u8; 32],
		timeout_timestamp: 0,
		body: vec![0u8; 64],
	};
	let request = Request::Post(post);
	let commitment = hash_request::<H>(&request);
	host.request_commitment(commitment)
		.map_err(|_| "Expected Request commitment to be found in storage")?;
	let post = PostRequest {
		source: StateMachine::Kusama(2000),
		dest: host.host_state_machine(),
		nonce: 0,
		from: vec![0u8; 32],
		to: vec![0u8; 32],
		timeout_timestamp: 0,
		body: vec![0u8; 64],
	};
	let response = PostResponse { post, response: vec![], timeout_timestamp: 0 };
	// Dispatch the outgoing response for the first time
	host.dispatch_response(
		response.clone(),
		FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
	)
	.map_err(|_| "Router failed to dispatch request")?;
	// Dispatch the same response a second time
	let err = host.dispatch_response(
		response,
		FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
	);
	assert!(err.is_err(), "Expected router to return error for duplicate response");

	Ok(())
}

/// This should prevent a request from timing out on a proxy when there exists a consensus client
/// for the request destination
/// The State machine for the proxy is assumed in this test to be ``StateMachine::Kusama(2000);``
/// the State machine for the host is assumed in this test to be ``StateMachine::Polkadot(1000)``
/// The destination state machine for the test is assumed to be
/// ``StateMachine::Evm(*b"EXEC")``
pub fn prevent_request_timeout_on_proxy_with_known_state_machine(
	direct_conn_state_machine: StateMachine,
) -> Result<(), &'static str> {
	let proxy_state_machine = StateMachine::Kusama(2000);
	let mut host = Host::default();
	host.proxy = Some(proxy_state_machine);

	let proxy = setup_mock_proxy_client(&host, proxy_state_machine);
	let challenge_period = host.challenge_period(proxy.height.id).unwrap();
	let previous_update_time = host.timestamp() - (challenge_period * 2);

	host.store_consensus_update_time(mock_proxy_consensus_state_id(), previous_update_time)
		.unwrap();
	host.store_state_machine_update_time(proxy.height, previous_update_time)
		.unwrap();

	// check for the two consensus clients and also add the clinet the other one
	//assert that one consensus client is for the proxy and the other is for the destination chain

	let consensus_clients = host.consensus_clients();
	assert!(consensus_clients.len() > 1);

	// assert that destination chain concensus client is in the Host list of clients
	// destination chain concensus in this test is assumed to be MOCK_CONSENSUS_CLIENT_ID

	let proxy_consensus_client_id = consensus_clients
		.iter()
		.find(|client| client.state_machine(proxy_state_machine).ok().is_some())
		.expect("The proxy consensus client should be set for this test")
		.consensus_client_id();
	let destination_consensus_client_id = consensus_clients
		.iter()
		.find(|client| client.state_machine(direct_conn_state_machine).ok().is_some())
		.expect("The directly connected chain's consensus client should be set for this test")
		.consensus_client_id();

	// For our test case we assert that there exists distinct consensus clients for the proxy and
	// the direct connection

	assert_ne!(proxy_consensus_client_id, destination_consensus_client_id);

	let dispatch_post = DispatchPost {
		dest: direct_conn_state_machine,
		from: vec![0u8; 32],
		to: vec![0u8; 32],
		timeout: proxy.commitment.timestamp,
		body: vec![0u8; 64],
	};

	let post = PostRequest {
		source: host.host_state_machine(),
		dest: direct_conn_state_machine,
		nonce: 0,
		from: vec![0u8; 32],
		to: vec![0u8; 32],
		timeout_timestamp: proxy.commitment.timestamp,
		body: vec![0u8; 64],
	};

	let request = Request::Post(post.clone());

	let dispatch_request = DispatchRequest::Post(dispatch_post);
	host.dispatch_request(
		dispatch_request,
		FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
	)
	.unwrap();

	let timeout_message = Message::Timeout(TimeoutMessage::Post {
		requests: vec![request.clone()],
		timeout_proof: Proof { height: proxy.height, proof: vec![] },
	});

	let res = handle_incoming_message(&host, timeout_message).map_err(|e| e.downcast().unwrap());

	assert!(matches!(res, Err(Error::RequestProxyProhibited { .. })));

	Ok(())
}

/// This should prevent a response from timing out on a proxy when there exists a consensus client
/// for the request destination
pub fn prevent_response_timeout_on_proxy_with_known_state_machine(
	direct_conn_state_machine: StateMachine,
) -> Result<(), &'static str> {
	let proxy_state_machine = StateMachine::Kusama(2000);
	let mut host = Host::default();
	host.proxy = Some(proxy_state_machine);

	let intermediate_state = setup_mock_client(&host);
	let challenge_period = host.challenge_period(intermediate_state.height.id).unwrap();
	let previous_update_time = host.timestamp() - (challenge_period * 2);
	host.store_consensus_update_time(mock_consensus_state_id(), previous_update_time)
		.unwrap();
	host.store_state_machine_update_time(intermediate_state.height, previous_update_time)
		.unwrap();

	let proxy = setup_mock_proxy_client(&host, proxy_state_machine);

	host.store_consensus_update_time(mock_proxy_consensus_state_id(), previous_update_time)
		.unwrap();
	host.store_state_machine_update_time(proxy.height, previous_update_time)
		.unwrap();

	// check for the two consensus clients and also add the clinet the other one
	//assert that one consensus client is for the proxy and the other is for the destination chain

	let consensus_clients = host.consensus_clients();
	assert!(consensus_clients.len() > 1);

	// assert that destination chain concensus client is in the Host list of clients
	// destination chain concensus in this test is assumed to be MOCK_CONSENSUS_CLIENT_ID

	let proxy_consensus_client_id = consensus_clients
		.iter()
		.find(|client| client.state_machine(proxy_state_machine).ok().is_some())
		.expect("The proxy consensus client should be set for this test")
		.consensus_client_id();
	let destination_consensus_client_id = consensus_clients
		.iter()
		.find(|client| client.state_machine(direct_conn_state_machine).ok().is_some())
		.expect("The proxy destination chain's consensus client should be set for this test")
		.consensus_client_id();

	// For our test case we assert that there exists distinct consensus clients for the proxy and
	// the direct connection

	assert_ne!(proxy_consensus_client_id, destination_consensus_client_id);

	let request = PostRequest {
		source: direct_conn_state_machine,
		dest: host.host_state_machine(),
		nonce: 0,
		from: vec![0u8; 32],
		to: vec![0u8; 32],
		timeout_timestamp: 0,
		body: vec![0u8; 64],
	};

	let (signature, ..) = create_relayer_signer(vec![request.clone()].encode(), &[1u8; 32]);

	let request_message = Message::Request(RequestMessage {
		requests: vec![request.clone()],
		proof: Proof { height: intermediate_state.height, proof: vec![] },
		signer: signature,
	});

	handle_incoming_message(&host, request_message).unwrap();
	// Assert that request was acknowledged
	assert!(matches!(host.request_receipt(&Request::Post(request.clone())), Some(_)));

	let response = PostResponse { post: request, response: vec![], timeout_timestamp: 100 };
	host.dispatch_response(response.clone(), Default::default()).unwrap();

	let timeout_message = Message::Timeout(TimeoutMessage::PostResponse {
		responses: vec![response.clone()],
		timeout_proof: Proof { height: proxy.height, proof: vec![] },
	});

	let res = handle_incoming_message(&host, timeout_message).map_err(|e| e.downcast().unwrap());

	assert!(matches!(res, Err(Error::ResponseProxyProhibited { .. })));

	Ok(())
}

/// Check that proxies can dispatch requests & responses.
// check that state machine can dispatch request and responses
pub fn sanity_check_for_proxies() {}

/// This should prevent a request from being processed through a proxy when there exists a state
/// machine client for the request source
pub fn prevent_request_processing_on_proxy_with_known_state_machine(
	direct_conn_state_machine: StateMachine,
) -> Result<(), &'static str> {
	let proxy_state_machine = StateMachine::Kusama(2000);
	let mut host = Host::default();
	host.proxy = Some(proxy_state_machine);
	let proxy = setup_mock_proxy_client(&host, proxy_state_machine);
	let challenge_period = host.challenge_period(proxy.height.id).unwrap();
	let previous_update_time = host.timestamp() - (challenge_period * 2);

	host.store_consensus_update_time(mock_proxy_consensus_state_id(), previous_update_time)
		.unwrap();
	host.store_state_machine_update_time(proxy.height, previous_update_time)
		.unwrap();

	// check for the two consensus clients and also add the clinet the other one
	//assert that one consensus client is for the proxy and the other is for the destination chain

	let consensus_clients = host.consensus_clients();
	assert!(consensus_clients.len() > 1);

	// assert that destination chain concensus client is in the Host list of clients
	// destination chain concensus in this test is assumed to be MOCK_CONSENSUS_CLIENT_ID

	let proxy_consensus_client_id = consensus_clients
		.iter()
		.find(|client| client.state_machine(proxy_state_machine).ok().is_some())
		.expect("The proxy consensus client should be set for this test")
		.consensus_client_id();
	let destination_consensus_client_id = consensus_clients
		.iter()
		.find(|client| client.state_machine(direct_conn_state_machine).ok().is_some())
		.expect("The directly connected chain's consensus client should be set for this test")
		.consensus_client_id();

	// For our test case we assert that there exists distinct consensus clients for the proxy and
	// the direct connection

	assert_ne!(proxy_consensus_client_id, destination_consensus_client_id);

	let request = PostRequest {
		source: direct_conn_state_machine,
		dest: host.host_state_machine(),
		nonce: 0,
		from: vec![0u8; 32],
		to: vec![0u8; 32],
		timeout_timestamp: 0,
		body: vec![0u8; 64],
	};

	let (signature, ..) = create_relayer_signer(vec![request.clone()].encode(), &[1u8; 32]);
	let request_message = Message::Request(RequestMessage {
		requests: vec![request.clone()],
		proof: Proof { height: proxy.height, proof: vec![] },
		signer: signature,
	});

	let res = handle_incoming_message(&host, request_message).map_err(|e| e.downcast().unwrap());

	assert!(matches!(res, Err(Error::RequestProxyProhibited { .. })));

	Ok(())
}

/// This should check that if a proxy isn't configured, requests are not valid if they don't come
/// from the state machine claimed in the proof as well as check that the request destination
/// matches the host state machine.
pub fn check_request_source_and_destination() -> Result<(), &'static str> {
	let host = Host::default();
	let intermediate_state = setup_mock_client(&host);
	let challenge_period = host.challenge_period(intermediate_state.height.id).unwrap();
	let previous_update_time = host.timestamp() - (challenge_period * 2);
	host.store_consensus_update_time(mock_consensus_state_id(), previous_update_time)
		.unwrap();
	host.store_state_machine_update_time(intermediate_state.height, previous_update_time)
		.unwrap();
	//  Assert that No proxy is configured
	assert!(host.allowed_proxy().is_none());

	let request = PostRequest {
		source: StateMachine::Kusama(13000),
		dest: host.host_state_machine(),
		nonce: 0,
		from: vec![0u8; 32],
		to: vec![0u8; 32],
		timeout_timestamp: 0,
		body: vec![0u8; 64],
	};

	let (signature, ..) = create_relayer_signer(vec![request.clone()].encode(), &[1u8; 32]);

	let request_message = Message::Request(RequestMessage {
		requests: vec![request.clone()],
		proof: Proof { height: intermediate_state.height, proof: vec![] },
		signer: signature,
	});

	let res = handle_incoming_message(&host, request_message).map_err(|e| e.downcast().unwrap());

	assert!(matches!(res, Err(Error::RequestProxyProhibited { .. })));

	Ok(())
}

/// This should check that if a proxy isn't configured, responses are not valid if they don't come
/// from the state machine claimed in the proof
pub fn check_response_source() -> Result<(), &'static str> {
	let host = Host::default();
	let intermediate_state = setup_mock_client(&host);
	let challenge_period = host.challenge_period(intermediate_state.height.id).unwrap();
	let previous_update_time = host.timestamp() - (challenge_period * 2);
	host.store_consensus_update_time(mock_consensus_state_id(), previous_update_time)
		.unwrap();
	host.store_state_machine_update_time(intermediate_state.height, previous_update_time)
		.unwrap();

	// We assert that no proxy is configured
	assert!(host.allowed_proxy().is_none());

	let post = PostRequest {
		source: host.host_state_machine(),
		dest: StateMachine::Polkadot(900),
		nonce: 0,
		from: vec![0u8; 32],
		to: vec![0u8; 32],
		timeout_timestamp: 0,
		body: vec![0u8; 64],
	};

	let dispatch_post = DispatchPost {
		dest: StateMachine::Polkadot(900),
		from: vec![0u8; 32],
		to: vec![0u8; 32],
		timeout: 0,
		body: vec![0u8; 64],
	};

	let dispatch_request = DispatchRequest::Post(dispatch_post);
	host.dispatch_request(
		dispatch_request,
		FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
	)
	.unwrap();

	let response = PostResponse { post, response: vec![], timeout_timestamp: 0 };

	let response_message = RequestResponse::Response(vec![Response::Post(response)]);

	let (signature, ..) = create_relayer_signer(response_message.encode(), &[1u8; 32]);

	let timeout_message = Message::Response(ResponseMessage {
		datagram: response_message,
		proof: Proof { height: intermediate_state.height, proof: vec![] },
		signer: signature,
	});

	let res = handle_incoming_message(&host, timeout_message).map_err(|e| e.downcast().unwrap());

	dbg!(&res);
	assert!(matches!(res, Err(Error::ResponseProxyProhibited { .. })));

	Ok(())
}

pub fn create_relayer_signer(data: Vec<u8>, seed: &[u8; 32]) -> (Vec<u8>, Vec<u8>) {
	let signer_keypair = sr25519::Pair::from_seed(seed);
	let data = sp_io::hashing::keccak_256(&data);
	let signature = signer_keypair.sign(&data);
	let signer = Signature::Sr25519 {
		public_key: signer_keypair.public().to_vec(),
		signature: signature.0.to_vec(),
	};
	(signer.encode(), signer_keypair.public().to_vec())
}
