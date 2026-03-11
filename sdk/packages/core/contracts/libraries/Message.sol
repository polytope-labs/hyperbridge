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
import {StateMachineHeight} from "../interfaces/IConsensus.sol";

/**
 * @title FrozenStatus
 * @notice Represents the frozen state of the Host for security and emergency situations
 * @dev Used to halt specific protocol operations during security incidents or upgrades
 */
enum FrozenStatus {
    /// @notice Normal operation - all functions are enabled
    None,
    /// @notice Incoming messages are blocked - prevents receiving cross-chain messages
    Incoming,
    /// @notice Outgoing messages are blocked - prevents sending cross-chain messages
    Outgoing,
    /// @notice All operations are frozen - complete protocol halt
    All
}

/**
 * @title PostRequest
 * @notice Represents a cross-chain message request
 * @dev Contains all necessary information for routing and processing a cross-chain message
 */
struct PostRequest {
    /// @notice Source chain identifier (e.g., "POLKADOT-1000", "EVM-1")
    bytes source;
    /// @notice Destination chain identifier
    bytes dest;
    /// @notice Unique nonce for this request on the source chain
    uint64 nonce;
    /// @notice Source application address that initiated this request
    bytes from;
    /// @notice Destination application address to receive this request
    bytes to;
    /// @notice Unix timestamp when this request expires
    uint64 timeoutTimestamp;
    /// @notice Request payload to be delivered to the destination
    bytes body;
}

/**
 * @title GetRequest
 * @notice Represents a cross-chain state query request
 * @dev Used to read storage values from other chains at specific heights
 */
struct GetRequest {
    /// @notice Source chain identifier where the query originated
    bytes source;
    /// @notice Destination chain identifier to query
    bytes dest;
    /// @notice Unique nonce for this request on the source chain
    uint64 nonce;
    /// @notice Address of the application that initiated this query
    address from;
    /// @notice Unix timestamp when this query expires
    uint64 timeoutTimestamp;
    /// @notice Storage keys to retrieve from the destination chain
    bytes[] keys;
    /// @notice Block height at which to read the state (0 for latest)
    uint64 height;
    /// @notice Application-specific metadata for tracking the query
    bytes context;
}

/**
 * @title GetResponse
 * @notice Represents a response to a cross-chain state query
 * @dev Contains the original query request and the retrieved storage values
 */
struct GetResponse {
    /// @notice The original GET request being responded to
    GetRequest request;
    /// @notice Retrieved storage values from the queried chain
    StorageValue[] values;
}

/**
 * @title PostResponse
 * @notice Represents a response to a cross-chain POST request
 * @dev Contains the original request and the response data
 */
struct PostResponse {
    /// @notice The original request being responded to
    PostRequest request;
    /// @notice Response payload to send back to the requester
    bytes response;
    /// @notice Unix timestamp when this response expires
    uint64 timeoutTimestamp;
}

/**
 * @title PostRequestLeaf
 * @notice Represents a POST request as a leaf in a Merkle Mountain Range tree
 * @dev Used for generating and verifying merkle proofs of requests
 */
struct PostRequestLeaf {
    /// @notice The POST request data
    PostRequest request;
    /// @notice Position in the MMR leaves array
    uint256 index;
    /// @notice K-index for MMR proof generation
    uint256 kIndex;
}

/**
 * @title PostResponseLeaf
 * @notice Represents a POST response as a leaf in a Merkle Mountain Range tree
 * @dev Used for generating and verifying merkle proofs of responses
 */
struct PostResponseLeaf {
    /// @notice The POST response data
    PostResponse response;
    /// @notice Position in the MMR leaves array
    uint256 index;
    /// @notice K-index for MMR proof generation
    uint256 kIndex;
}

// A get response as a leaf in a merkle mountain range tree
/**
 * @title GetResponseLeaf
 * @notice Represents a GET response as a leaf in a Merkle Mountain Range tree
 * @dev Used for generating and verifying merkle proofs of state query responses
 */
struct GetResponseLeaf {
    /// @notice The GET response data
    GetResponse response;
    /// @notice Position in the MMR leaves array
    uint256 index;
    /// @notice K-index for MMR proof generation
    uint256 kIndex;
}

/**
 * @title Proof
 * @notice Merkle Mountain Range proof for message verification
 * @dev Used to prove inclusion of messages in the protocol's merkle tree
 */
struct Proof {
    /// @notice State machine height where this proof is anchored
    StateMachineHeight height;
    /// @notice Array of merkle tree nodes forming the inclusion proof
    bytes32[] multiproof;
    /// @notice Total number of leaves in the MMR at this height
    uint256 leafCount;
}

// A message for handling incoming requests
/**
 * @title PostRequestMessage
 * @notice Batch of POST requests with their merkle proof
 * @dev Used by the Handler to verify and process incoming POST requests
 */
struct PostRequestMessage {
    /// @notice Merkle proof for verifying the requests
    Proof proof;
    /// @notice Array of POST requests as MMR leaves
    PostRequestLeaf[] requests;
}

// A message for handling incoming GET responses
struct GetResponseMessage {
    // proof for the responses
    Proof proof;
    // The responses, contained in the merkle mountain range tree
    GetResponseLeaf[] responses;
}

/**
 * @title GetTimeoutMessage
 * @notice Batch of timed-out GET requests with their non-membership proof
 * @dev Used to prove that GET requests were not responded to before timeout
 */
struct GetTimeoutMessage {
    /// @notice Array of GET requests that have timed out
    GetRequest[] timeouts;
    /// @notice Height at which the timeout proof is generated
    StateMachineHeight height;
    /// @notice Non-membership proof showing requests were not processed
    bytes[] proof;
}

/**
 * @title PostRequestTimeoutMessage
 * @notice Batch of timed-out POST requests with their non-membership proof
 * @dev Used to prove that POST requests were not responded to before timeout
 */
struct PostRequestTimeoutMessage {
    /// @notice Array of POST requests that have timed out
    PostRequest[] timeouts;
    /// @notice Height at which the timeout proof is generated
    StateMachineHeight height;
    /// @notice Non-membership proof showing requests were not processed
    bytes[] proof;
}

/**
 * @title PostResponseTimeoutMessage
 * @notice Batch of timed-out POST responses with their non-membership proof
 * @dev Used to prove that POST responses were not acknowledged before timeout
 */
struct PostResponseTimeoutMessage {
    /// @notice Array of POST responses that have timed out
    PostResponse[] timeouts;
    /// @notice Height at which the timeout proof is generated
    StateMachineHeight height;
    /// @notice Non-membership proof showing responses were not processed
    bytes[] proof;
}

// A message for handling incoming responses
/**
 * @title PostResponseMessage
 * @notice Batch of POST responses with their merkle proof
 * @dev Used by the Handler to verify and process incoming POST responses
 */
struct PostResponseMessage {
    /// @notice Merkle proof for verifying the responses
    Proof proof;
    /// @notice Array of POST responses as MMR leaves
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

        return abi.encodePacked(
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
