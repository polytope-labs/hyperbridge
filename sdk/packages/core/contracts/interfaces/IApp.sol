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

import {PostRequest, PostResponse, GetResponse, GetRequest} from "../libraries/Message.sol";

/**
 * @title IncomingPostRequest
 * @notice Encapsulates an incoming POST request with relayer information
 * @dev Used by the Host to deliver POST requests to applications
 */
struct IncomingPostRequest {
    // The Post request containing source, dest, nonce, from, to, timeout, and body
    PostRequest request;
    // Relayer responsible for delivering the request
    address relayer;
}

/**
 * @title IncomingPostResponse
 * @notice Encapsulates an incoming POST response with relayer information
 * @dev Used by the Host to deliver POST responses to applications
 */
struct IncomingPostResponse {
    // The Post response containing the original request, response data, and timeout
    PostResponse response;
    // Relayer responsible for delivering the response
    address relayer;
}

/**
 * @title IncomingGetResponse
 * @notice Encapsulates an incoming GET response with relayer information
 * @dev Used by the Host to deliver GET responses containing state data to applications
 */
struct IncomingGetResponse {
    // The Get response containing the request and retrieved state values
    GetResponse response;
    // Relayer responsible for delivering the response
    address relayer;
}

/**
 * @title IApp
 * @author Polytope Labs (hello@polytope.technology)
 * @notice Interface for cross-chain applications built on Hyperbridge
 * @dev Applications must implement this interface to receive and handle cross-chain messages.
 * The Host calls these methods to deliver messages, responses, and timeout notifications.
 * All methods are permissioned and should only be callable by the Host contract.
 */
interface IApp {
    /**
     * @dev Called by the `Host` to notify an app of a new request the app may choose to respond immediately, or in a later block
     * @param incoming post request
     */
    function onAccept(IncomingPostRequest memory incoming) external;

    /**
     * @dev Called by the `Host` to notify an app of a post response to a previously sent out request
     * @param incoming post response
     */
    function onPostResponse(IncomingPostResponse memory incoming) external;

    /**
     * @dev Called by the `Host` to notify an app of a get response to a previously sent out request
     * @param incoming get response
     */
    function onGetResponse(IncomingGetResponse memory incoming) external;

    /**
     * @dev Called by the `Host` to notify an app of post requests that were previously sent but have now timed-out
     * @param request post request
     */
    function onPostRequestTimeout(PostRequest memory request) external;

    /**
     * @dev Called by the `Host` to notify an app of post responses that were previously sent but have now timed-out
     * @param response post response
     */
    function onPostResponseTimeout(PostResponse memory response) external;

    /**
     * @dev Called by the `Host` to notify an app of get requests that were previously sent but have now timed-out
     * @param request get request
     */
    function onGetTimeout(GetRequest memory request) external;
}
