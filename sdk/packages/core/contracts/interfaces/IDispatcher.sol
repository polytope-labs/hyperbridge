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

import {PostRequest, FrozenStatus} from "../libraries/Message.sol";

/**
 * @title DispatchPost
 * @notice Parameters for dispatching a POST request through Hyperbridge
 * @dev Used to send arbitrary data to applications on other chains
 */
struct DispatchPost {
    /// @notice Destination chain identifier (e.g., "POLKADOT-1000", "EVM-1")
    /// @dev Must be a valid state machine identifier recognized by the protocol
    bytes dest;
    /// @notice Destination application address or identifier
    /// @dev The receiving application on the destination chain
    bytes to;
    /// @notice The request payload
    /// @dev Arbitrary bytes that will be delivered to the destination application
    bytes body;
    /// @notice Timeout duration in seconds from the current timestamp
    /// @dev Request will be considered timed out after this duration
    uint64 timeout;
    /// @notice Fee paid to relayers for delivery & execution
    /// @dev Paid in the fee token specified by IHost.feeToken()
    uint256 fee;
    /// @notice Account responsible for paying the fees
    /// @dev If different from msg.sender, must have approved the Host contract
    address payer;
}

/**
 * @title DispatchGet
 * @notice Parameters for dispatching a GET request to query state on another chain
 * @dev Used to read storage values from other chains at a specific height
 */
struct DispatchGet {
    /// @notice Destination chain identifier
    /// @dev Must be a valid state machine identifier
    bytes dest;
    /// @notice Block height at which to read the state
    /// @dev Use 0 for latest available state
    uint64 height;
    /// @notice Storage keys to query
    /// @dev Array of storage keys whose values will be retrieved
    bytes[] keys;
    /// @notice Timeout duration in seconds
    /// @dev Query will be considered timed out after this duration
    uint64 timeout;
    /// @notice Fee amount for the query operation
    /// @dev Covers both protocol fees and relayer incentives
    uint256 fee;
    /// @notice Application-specific metadata
    /// @dev Can be used to track the query purpose or pass additional context
    bytes context;
}

/**
 * @title DispatchPostResponse
 * @notice Parameters for dispatching a response to a previously received POST request
 * @dev Used by applications to respond to cross-chain requests
 */
struct DispatchPostResponse {
    /// @notice The original request being responded to
    /// @dev Must be a valid request that was previously received
    PostRequest request;
    /// @notice Response payload
    /// @dev Data to send back to the requesting application
    bytes response;
    /// @notice Timeout duration in seconds for the response
    /// @dev Response will be considered timed out after this duration
    uint64 timeout;
    /// @notice Fee paid to relayers for delivery & execution
    /// @dev Paid in the fee token specified by IHost.feeToken()
    uint256 fee;
    /// @notice Account responsible for paying the fees
    /// @dev If different from msg.sender, must have approved the Host contract
    address payer;
}

/**
 * @title IDispatcher
 * @author Polytope Labs (hello@polytope.technology)
 * @notice Interface for dispatching cross-chain messages through Hyperbridge
 * @dev This interface provides methods for sending POST requests, GET queries, and responses.
 * All dispatch methods support both native token and fee token payments for relayer fees.
 * The dispatcher handles message routing, fee collection, and timeout management.
 */
interface IDispatcher {
    /**
     * @return the host state machine id
     */
    function host() external view returns (bytes memory);

    /**
     * @return the state machine identifier for the connected hyperbridge instance
     */
    function hyperbridge() external view returns (bytes memory);

    /**
     * @return the `frozen` status
     */
    function frozen() external view returns (FrozenStatus);

    /**
     * @dev Returns the address for the Uniswap V2 Router implementation used for swaps
     * @return routerAddress - The address to the in-use RouterV02 implementation
     */
    function uniswapV2Router() external view returns (address);

    /**
     * @dev Returns the nonce immediately available for requests
     * @return the `nonce`
     */
    function nonce() external view returns (uint256);

    /**
     * @dev Returns the address of the ERC-20 fee token contract configured for this state machine.
     *
     * @notice Hyperbridge collects it's dispatch fees in the provided token denomination. This will typically be in stablecoins.
     *
     * @return feeToken - The ERC20 contract address for fees.
     */
    function feeToken() external view returns (address);

    /**
     * @dev Returns the address of the per byte fee configured for the destination state machine.
     *
     * @notice Hyperbridge collects it's dispatch fees per every byte of the outgoing message.
     *
     * @param dest - The destination chain for the per byte fee.
     * @return perByteFee - The per byte fee for outgoing messages.
     */
    function perByteFee(bytes memory dest) external view returns (uint256);

    /**
     * @dev Dispatch a POST request to Hyperbridge
     *
     * @notice Payment for the request can be made with either the native token or the IHost.feeToken.
     * If native tokens are supplied, it will perform a swap under the hood using the local uniswap router.
     * Will revert if enough native tokens are not provided.
     *
     * If no native tokens are provided then it will try to collect payment from the calling contract in
     * the IHost.feeToken.
     *
     * @param request - post request
     * @return commitment - the request commitment
     */
    function dispatch(DispatchPost memory request) external payable returns (bytes32 commitment);

    /**
     * @dev Dispatch a GET request to Hyperbridge
     *
     * @notice Payment for the request can be made with either the native token or the IHost.feeToken.
     * If native tokens are supplied, it will perform a swap under the hood using the local uniswap router.
     * Will revert if enough native tokens are not provided.
     *
     * If no native tokens are provided then it will try to collect payment from the calling contract in
     * the IHost.feeToken.
     *
     * @param request - get request
     * @return commitment - the request commitment
     */
    function dispatch(DispatchGet memory request) external payable returns (bytes32 commitment);

    /**
     * @dev Dispatch a POST response to Hyperbridge
     *
     * @notice Payment for the request can be made with either the native token or the IHost.feeToken.
     * If native tokens are supplied, it will perform a swap under the hood using the local uniswap router.
     * Will revert if enough native tokens are not provided.
     *
     * If no native tokens are provided then it will try to collect payment from the calling contract in
     * the IHost.feeToken.
     *
     * @param response - post response
     * @return commitment - the request commitment
     */
    function dispatch(DispatchPostResponse memory response) external payable returns (bytes32 commitment);

    /**
     * @dev Increase the relayer fee for a previously dispatched request.
     * This is provided for use only on pending requests, such that when they timeout,
     * the user can recover the entire relayer fee.
     *
     * @notice Payment can be made with either the native token or the IHost.feeToken.
     * If native tokens are supplied, it will perform a swap under the hood using the local uniswap router.
     * Will revert if enough native tokens are not provided.
     *
     * If no native tokens are provided then it will try to collect payment from the calling contract in
     * the IHost.feeToken.
     *
     * If called on an already delivered request, these funds will be seen as a donation to the hyperbridge protocol.
     * @param commitment - The request commitment
     * @param amount - The amount provided in `IHost.feeToken()`
     */
    function fundRequest(bytes32 commitment, uint256 amount) external payable;

    /**
     * @dev Increase the relayer fee for a previously dispatched response.
     * This is provided for use only on pending responses, such that when they timeout,
     * the user can recover the entire relayer fee.
     *
     * @notice Payment can be made with either the native token or the IHost.feeToken.
     * If native tokens are supplied, it will perform a swap under the hood using the local uniswap router.
     * Will revert if enough native tokens are not provided.
     *
     * If no native tokens are provided then it will try to collect payment from the calling contract in
     * the IHost.feeToken.
     *
     * If called on an already delivered response, these funds will be seen as a donation to the hyperbridge protocol.
     * @param commitment - The response commitment
     * @param amount - The amount to be provided in `IHost.feeToken()`
     */
    function fundResponse(bytes32 commitment, uint256 amount) external payable;
}
