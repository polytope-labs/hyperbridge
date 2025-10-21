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

struct TeleportParams {
	// amount to be sent
	uint256 amount;
	// Relayer fee
	uint256 relayerFee;
	// The token identifier to send
	bytes32 assetId;
	// Redeem ERC20 on the destination?
	bool redeem;
	// recipient address
	bytes32 to;
	// recipient state machine
	bytes dest;
	// request timeout in seconds
	uint64 timeout;
	// Amount of native token to pay for dispatching the request
	// if 0 will use the `IIsmpHost.feeToken`
	uint256 nativeCost;
	// destination contract call data
	bytes data;
}

// Params for the TokenGateway contract
struct TokenGatewayParams {
	// address of the IsmpHost contract on this chain
	address host;
	// dispatcher for delegating external calls
	address dispatcher;
}

/**
 * @title ITokenGateway
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice Interface for the TokenGateway contract that allows users to send either ERC20 or hyper-fungible tokens
 * using Hyperbridge as a message-passing layer.
 *
 * @dev ERC20 tokens are custodied in exchange for hyper-fungible tokens to be minted on the destination chain,
 * Otherwise if hyper-fungible tokens are sent, then it simply performs a burn-and-mint.
 */
interface ITokenGateway {
	// User has received some assets
	event AssetReceived(
		// The amount that was provided to the user
		uint256 amount,
		// The associated request commitment
		bytes32 commitment,
		// The source of the funds
		bytes32 indexed from,
		// The beneficiary of the funds
		address indexed beneficiary,
		// The provided assetId
		bytes32 indexed assetId
	);

	// User has sent some assets
	event AssetTeleported(
		// The beneficiary of the funds
		bytes32 to,
		// The destination chain
		string dest,
		// The amount that was requested to be sent
		uint256 amount,
		// The associated request commitment
		bytes32 commitment,
		// The source of the funds
		address indexed from,
		// The provided assetId
		bytes32 indexed assetId,
		// Flag to redeem funds from the TokenGateway
		bool redeem
	);

	// User assets could not be delivered and have been refunded.
	event AssetRefunded(
		// The amount that was requested to be sent
		uint256 amount,
		// The associated request commitment
		bytes32 commitment,
		// The beneficiary of the funds
		address indexed beneficiary,
		// The provided assetId
		bytes32 indexed assetId
	);

	// @dev Unexpected zero address
	error ZeroAddress();

	// @dev Provided amount was invalid
	error InvalidAmount();

	// @dev Provided token was unknown
	error UnknownAsset();

	// @dev Protocol invariant violated
	error InconsistentState();

	/**
	 * @dev Read the protocol parameters
	 */
	function params() external view returns (TokenGatewayParams memory);

	/**
	 * @dev Fetch the address for an ERC20 asset
	 */
	function erc20(bytes32 assetId) external view returns (address);

	/**
	 * @dev Fetch the address for a hyper-fungible asset
	 */
	function erc6160(bytes32 assetId) external view returns (address);

	/**
	 * @dev Fetch the TokenGateway instance for a destination.
	 */
	function instance(bytes calldata destination) external view returns (address);

	/**
	 * @dev Teleports a local ERC20/hyper-fungible asset to the destination chain. Allows users to pay
	 * the Hyperbridge fees in the native token or `IIsmpHost.feeToken`
	 *
	 * @notice If a request times out, users can request a refund permissionlessly through
	 * `HandlerV1.handlePostRequestTimeouts`.
	 */
	function teleport(TeleportParams calldata teleportParams) external payable;
}
