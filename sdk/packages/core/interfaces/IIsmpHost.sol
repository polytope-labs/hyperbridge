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

import {StateCommitment, StateMachineHeight} from "./IConsensusClient.sol";
import {IDispatcher} from "./IDispatcher.sol";
import {PostRequest, PostResponse, GetResponse, GetRequest, FrozenStatus} from "./Message.sol";

// Some metadata about the request fee
struct FeeMetadata {
	// the relayer fee
	uint256 fee;
	// user who initiated the request
	address sender;
}

struct ResponseReceipt {
	// commitment of the response object
	bytes32 responseCommitment;
	// address of the relayer responsible for this response delivery
	address relayer;
}

/**
 * @title The Ismp Host Interface
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice The Ismp Host interface sits at the core of the interoperable state machine protocol,
 * It which encapsulates the interfaces required for ISMP datagram handlers and modules.
 *
 * @dev The IsmpHost provides the necessary storage interface for the ISMP handlers to process
 * ISMP messages, the IsmpDispatcher provides the interfaces applications use for dispatching requests
 * and responses. This host implementation delegates all verification logic to the IHandler contract.
 * It is only responsible for dispatching incoming & outgoing messages as well as managing
 * the state of the ISMP protocol.
 */
interface IIsmpHost is IDispatcher {
	/**
	 * @return the host admin
	 */
	function admin() external returns (address);

	/**
	 * @return the host timestamp
	 */
	function timestamp() external view returns (uint256);

	/**
	 * @dev Returns the fisherman responsible for vetoing the given state machine height.
	 * @return the `fisherman` address
	 */
	function vetoes(uint256 paraId, uint256 height) external view returns (address);

	/**
	 * @dev Returns the fee required for 3rd party applications to access hyperbridge state commitments.
	 * @return the `stateCommitmentFee`
	 */
	function stateCommitmentFee() external view returns (uint256);

	/**
	 * @notice Charges the stateCommitmentFee to 3rd party applications.
	 * If native tokens are provided, will attempt to swap them for the stateCommitmentFee.
	 * If not enough native tokens are supplied, will revert.
	 *
	 * If no native tokens are provided then it will try to collect payment from the calling contract in
	 * the IIsmpHost.feeToken.
	 *
	 * @param height - state machine height
	 * @return the state commitment at `height`
	 */
	function stateMachineCommitment(StateMachineHeight memory height) external payable returns (StateCommitment memory);

	/**
	 * @param height - state machine height
	 * @return the state machine commitment update time at `height`
	 */
	function stateMachineCommitmentUpdateTime(StateMachineHeight memory height) external returns (uint256);

	/**
	 * @return the consensus client contract
	 */
	function consensusClient() external view returns (address);

	/**
	 * @return the last updated time of the consensus client
	 */
	function consensusUpdateTime() external view returns (uint256);

	/**
	 * @return the latest state machine height for the given stateMachineId. If it returns 0, the state machine is unsupported.
	 */
	function latestStateMachineHeight(uint256 stateMachineId) external view returns (uint256);

	/**
	 * @return the state of the consensus client
	 */
	function consensusState() external view returns (bytes memory);

	/**
	 * @dev Check the response status for a given request.
	 * @return `response` status
	 */
	function responded(bytes32 commitment) external view returns (bool);

	/**
	 * @param commitment - commitment to the request
	 * @return relayer address
	 */
	function requestReceipts(bytes32 commitment) external view returns (address);

	/**
	 * @param commitment - commitment to the request of the response
	 * @return response receipt
	 */
	function responseReceipts(bytes32 commitment) external view returns (ResponseReceipt memory);

	/**
	 * @param commitment - commitment to the request
	 * @return existence status of an outgoing request commitment
	 */
	function requestCommitments(bytes32 commitment) external view returns (FeeMetadata memory);

	/**
	 * @param commitment - commitment to the response
	 * @return existence status of an outgoing response commitment
	 */
	function responseCommitments(bytes32 commitment) external view returns (FeeMetadata memory);

	/**
	 * @return the challenge period
	 */
	function challengePeriod() external view returns (uint256);

	/**
	 * @return the unstaking period
	 */
	function unStakingPeriod() external view returns (uint256);

	/**
	 * @dev set the new frozen state of the host, only the admin or handler can call this.
	 * @param newState - the new frozen state
	 */
	function setFrozenState(FrozenStatus newState) external;

	/**
	 * @dev Store an encoded consensus state
	 * @param state new consensus state
	 */
	function storeConsensusState(bytes memory state) external;

	/**
	 * @dev Store the commitment at `state height`
	 * @param height state machine height
	 * @param commitment state commitment
	 */
	function storeStateMachineCommitment(StateMachineHeight memory height, StateCommitment memory commitment) external;

	/**
	 * @dev Delete the state commitment at given state height.
	 */
	function deleteStateMachineCommitment(StateMachineHeight memory height, address fisherman) external;

	/**
	 * @dev Dispatch an incoming request to destination module
	 * @param request - post request
	 */
	function dispatchIncoming(PostRequest memory request, address relayer) external;

	/**
	 * @dev Dispatch an incoming post response to source module
	 * @param response - post response
	 */
	function dispatchIncoming(PostResponse memory response, address relayer) external;

	/**
	 * @dev Dispatch an incoming get response to source module
	 * @param response - get response
	 */
	function dispatchIncoming(GetResponse memory response, address relayer) external;

	/**
	 * @dev Dispatch an incoming get timeout to source module
	 * @param timeout - timed-out get request
	 */
	function dispatchTimeOut(GetRequest memory timeout, FeeMetadata memory meta, bytes32 commitment) external;

	/**
	 * @dev Dispatch an incoming post timeout to source module
	 * @param timeout - timed-out post request
	 */
	function dispatchTimeOut(PostRequest memory timeout, FeeMetadata memory meta, bytes32 commitment) external;

	/**
	 * @dev Dispatch an incoming post response timeout to source module
	 * @param timeout - timed-out post response
	 */
	function dispatchTimeOut(PostResponse memory timeout, FeeMetadata memory meta, bytes32 commitment) external;
}
