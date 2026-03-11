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

import {IHost} from "./IHost.sol";
import {
    PostRequestMessage,
    PostResponseMessage,
    GetResponseMessage,
    PostRequestTimeoutMessage,
    PostResponseTimeoutMessage,
    GetTimeoutMessage
} from "../libraries/Message.sol";

/**
 * @title IHandler
 * @author Polytope Labs (hello@polytope.technology)
 * @notice Interface for handling incoming cross-chain messages and consensus updates
 * @dev The Handler is responsible for verifying and processing all incoming ISMP protocol messages.
 * It validates consensus proofs, message proofs, and ensures messages are delivered to the correct applications.
 * This interface serves as the entry point for all cross-chain communication through Hyperbridge.
 */
interface IHandler {
    /**
     * @notice Process an incoming consensus update
     * @dev Verifies the consensus proof using the registered IConsensus implementation and updates the trusted state.
     * This enables the protocol to track state changes on remote chains.
     * @param host The Host contract that stores protocol state
     * @param proof Consensus proof data (format depends on the consensus mechanism)
     */
    function handleConsensus(IHost host, bytes memory proof) external;

    /**
     * @notice Process a batch of incoming POST requests
     * @dev Verifies request proofs, checks for timeouts, validates message delays, and dispatches valid requests to destination apps.
     * Ensures requests haven't expired and come from verified state commitments.
     * @param host The Host contract that stores protocol state
     * @param request Batch of POST requests with their merkle proofs
     */
    function handlePostRequests(IHost host, PostRequestMessage memory request) external;

    /**
     * @notice Process a batch of incoming POST responses
     * @dev Verifies response proofs, checks for timeouts, validates message delays, and dispatches valid responses to requesting apps.
     * Matches responses to their original requests and ensures they haven't expired.
     * @param host The Host contract that stores protocol state
     * @param response Batch of POST responses with their merkle proofs
     */
    function handlePostResponses(IHost host, PostResponseMessage memory response) external;

    /**
     * @notice Process a batch of GET responses (state queries)
     * @dev Verifies state proofs, checks for timeouts, and delivers the queried state data to requesting apps.
     * Ensures the state data comes from the requested height and hasn't expired.
     * @param host The Host contract that stores protocol state
     * @param message Batch of GET responses with their state proofs
     */
    function handleGetResponses(IHost host, GetResponseMessage memory message) external;

    /**
     * @notice Process POST request timeouts
     * @dev Verifies non-membership proofs to confirm requests were not processed before timeout.
     * Notifies source apps about timed-out requests and allows them to handle refunds or retries.
     * @param host The Host contract that stores protocol state
     * @param message Batch of timed-out POST requests with non-membership proofs
     */
    function handlePostRequestTimeouts(IHost host, PostRequestTimeoutMessage memory message) external;

    /**
     * @notice Process POST response timeouts
     * @dev Verifies non-membership proofs to confirm responses were not processed before timeout.
     * Notifies responding apps about timed-out responses.
     * @param host The Host contract that stores protocol state
     * @param message Batch of timed-out POST responses with non-membership proofs
     */
    function handlePostResponseTimeouts(IHost host, PostResponseTimeoutMessage memory message) external;

    /**
     * @notice Process GET request timeouts
     * @dev Verifies non-membership proofs to confirm queries were not answered before timeout.
     * Notifies requesting apps about timed-out state queries so they can implement fallback logic.
     * @param host The Host contract that stores protocol state
     * @param message Batch of timed-out GET requests with non-membership proofs
     */
    function handleGetRequestTimeouts(IHost host, GetTimeoutMessage memory message) external;
}
