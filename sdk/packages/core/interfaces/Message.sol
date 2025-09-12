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
pragma solidity ^0.8.17;

import {StorageValue} from "@polytope-labs/solidity-merkle-trees/src/Types.sol";

// Various frozen states of the IIsmpHost
enum FrozenStatus {
	// Host is operating normally
	None,
	// Host is currently disallowing incoming datagrams
	Incoming,
	// Host is currently disallowing outgoing messages
	Outgoing,
	// All actions have been frozen
	All
}

// Identifies some state machine height. We allow for a state machine identifier here
// as some consensus clients may track multiple, concurrent state machines.
struct StateMachineHeight {
	// the state machine identifier
	uint256 stateMachineId;
	// height of this state machine
	uint256 height;
}

struct PostRequest {
	// the source state machine of this request
	bytes source;
	// the destination state machine of this request
	bytes dest;
	// request nonce
	uint64 nonce;
	// Module Id of this request origin
	bytes from;
	// destination module id
	bytes to;
	// timestamp by which this request times out.
	uint64 timeoutTimestamp;
	// request body
	bytes body;
}

struct GetRequest {
	// the source state machine of this request
	bytes source;
	// the destination state machine of this request
	bytes dest;
	// request nonce
	uint64 nonce;
	// Module Id of this request origin
	address from;
	// timestamp by which this request times out.
	uint64 timeoutTimestamp;
	// Storage keys to read.
	bytes[] keys;
	// height at which to read destination state machine
	uint64 height;
	// Some application-specific metadata relating to this request
	bytes context;
}

struct GetResponse {
	// The request that initiated this response
	GetRequest request;
	// storage values for get response
	StorageValue[] values;
}

struct PostResponse {
	// The request that initiated this response
	PostRequest request;
	// bytes for post response
	bytes response;
	// timestamp by which this response times out.
	uint64 timeoutTimestamp;
}

// A post request as a leaf in a merkle tree
struct PostRequestLeaf {
	// The request
	PostRequest request;
	// It's index in the mmr leaves
	uint256 index;
	// it's k-index
	uint256 kIndex;
}

// A post response as a leaf in a merkle tree
struct PostResponseLeaf {
	// The response
	PostResponse response;
	// It's index in the mmr leaves
	uint256 index;
	// it's k-index
	uint256 kIndex;
}

// A get response as a leaf in a merkle mountain range tree
struct GetResponseLeaf {
	// The response
	GetResponse response;
	// It's index in the mmr leaves
	uint256 index;
	// it's k-index
	uint256 kIndex;
}

// A merkle mountain range proof.
struct Proof {
	// height of the state machine
	StateMachineHeight height;
	// the multi-proof
	bytes32[] multiproof;
	// The total number of leaves in the mmr for this proof.
	uint256 leafCount;
}

// A message for handling incoming requests
struct PostRequestMessage {
	// proof for the requests
	Proof proof;
	// The requests, contained in the merkle mountain range tree
	PostRequestLeaf[] requests;
}

// A message for handling incoming GET responses
struct GetResponseMessage {
	// proof for the responses
	Proof proof;
	// The responses, contained in the merkle mountain range tree
	GetResponseLeaf[] responses;
}

struct GetTimeoutMessage {
	// requests which have timed-out
	GetRequest[] timeouts;
	// the height of the state machine proof
	StateMachineHeight height;
	// non-membership proof of the requests
	bytes[] proof;
}

struct PostRequestTimeoutMessage {
	// requests which have timed-out
	PostRequest[] timeouts;
	// the height of the state machine proof
	StateMachineHeight height;
	// non-membership proof of the requests
	bytes[] proof;
}

struct PostResponseTimeoutMessage {
	// responses which have timed-out
	PostResponse[] timeouts;
	// the height of the state machine proof
	StateMachineHeight height;
	// non-membership proof of the requests
	bytes[] proof;
}

// A message for handling incoming responses
struct PostResponseMessage {
	// proof for the responses
	Proof proof;
	// the responses, contained in a merkle tree leaf
	PostResponseLeaf[] responses;
}

library Message {
	/**
	 * @dev Calculates the absolute timeout value for a PostRequest
	 */
	function timeout(PostRequest memory req) internal pure returns (uint64) {
		if (req.timeoutTimestamp == 0) {
			return type(uint64).max;
		} else {
			return req.timeoutTimestamp;
		}
	}

	/**
	 * @dev Calculates the absolute timeout value for a GetRequest
	 */
	function timeout(GetRequest memory req) internal pure returns (uint64) {
		if (req.timeoutTimestamp == 0) {
			return type(uint64).max;
		} else {
			return req.timeoutTimestamp;
		}
	}

	/**
	 * @dev Calculates the absolute timeout value for a PostResponse
	 */
	function timeout(PostResponse memory res) internal pure returns (uint64) {
		if (res.timeoutTimestamp == 0) {
			return type(uint64).max;
		} else {
			return res.timeoutTimestamp;
		}
	}

	/**
	 * @dev Encode the given post request for commitment
	 */
	function encode(PostRequest memory req) internal pure returns (bytes memory) {
		return abi.encodePacked(req.source, req.dest, req.nonce, req.timeoutTimestamp, req.from, req.to, req.body);
	}

	/**
	 * @dev Encode the given get request for commitment
	 */
	function encode(GetRequest memory req) internal pure returns (bytes memory) {
		bytes memory keysEncoding = bytes("");
		uint256 len = req.keys.length;
		for (uint256 i = 0; i < len; i++) {
			keysEncoding = bytes.concat(keysEncoding, req.keys[i]);
		}

		return
			abi.encodePacked(
				req.source,
				req.dest,
				req.nonce,
				req.height,
				req.timeoutTimestamp,
				abi.encodePacked(req.from),
				keysEncoding,
				req.context
			);
	}

	/**
	 * @dev Returns the commitment for the given post response
	 */
	function hash(PostResponse memory res) internal pure returns (bytes32) {
		return keccak256(bytes.concat(encode(res.request), abi.encodePacked(res.response, res.timeoutTimestamp)));
	}

	/**
	 * @dev Returns the commitment for the given post request
	 */
	function hash(PostRequest memory req) internal pure returns (bytes32) {
		return keccak256(encode(req));
	}

	/**
	 * @dev Returns the commitment for the given get request
	 */
	function hash(GetRequest memory req) internal pure returns (bytes32) {
		return keccak256(encode(req));
	}

	/**
	 * @dev Returns the commitment for the given get response
	 */
	function hash(GetResponse memory res) internal pure returns (bytes32) {
		bytes memory response = bytes("");
		uint256 len = res.values.length;
		for (uint256 i = 0; i < len; i++) {
			response = bytes.concat(response, bytes.concat(res.values[i].key, res.values[i].value));
		}
		return keccak256(bytes.concat(encode(res.request), response));
	}
}
