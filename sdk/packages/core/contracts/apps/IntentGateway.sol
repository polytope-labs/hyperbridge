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

/**
 * @notice Tokens that must be received for a valid order fulfillment
 */
struct PaymentInfo {
	/// @dev The address of the ERC20 token on the destination chain
	/// @dev address(0) used as a sentinel for the native token
	bytes32 token;
	/// @dev The amount of the token to be sent
	uint256 amount;
	/// @dev The address to receive the output tokens
	bytes32 beneficiary;
}

/**
 * @notice Tokens that must be escrowed for an order
 */
struct TokenInfo {
	/// @dev The address of the ERC20 token on the destination chain
	/// @dev address(0) used as a sentinel for the native token
	bytes32 token;
	/// @dev The amount of the token to be sent
	uint256 amount;
}

/**
 * @dev Represents an order in the IntentGateway module.
 * @param Order The structure defining an order.
 */
struct Order {
	/// @dev The address of the user who is initiating the transfer
	bytes32 user;
	/// @dev The state machine identifier of the origin chain
	bytes sourceChain;
	/// @dev The state machine identifier of the destination chain
	bytes destChain;
	/// @dev The block number by which the order must be filled on the destination chain
	uint256 deadline;
	/// @dev The nonce of the order
	uint256 nonce;
	/// @dev Represents the dispatch fees associated with the IntentGateway.
	uint256 fees;
	/// @dev The tokens that the filler will provide.
	PaymentInfo[] outputs;
	/// @dev The tokens that are escrowed for the filler.
	TokenInfo[] inputs;
	/// @dev A bytes array to store the calls if any.
	bytes callData;
}

/**
 * @dev Struct to define the parameters for the IntentGateway module.
 */
struct Params {
	/// @dev The address of the host contract
	address host;
	/// @dev Address of the dispatcher contract responsible for handling intents.
	address dispatcher;
}

/**
 * @notice A struct representing the options for filling an intent.
 * @dev This struct is used to specify various parameters and options
 *      when filling an intent in the IntentGateway contract.
 */
struct FillOptions {
	/// @dev The fee paid to the relayer for processing transactions.
	uint256 relayerFee;
}

/**
 * @dev Struct representing the options for canceling an intent.
 */
struct CancelOptions {
	/// @dev The fee paid to the relayer for processing transactions.
	uint256 relayerFee;
	/// @dev Stores the height value.
	uint256 height;
}

/**
 * @title IIntentGateway
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @dev Interface for the IntentGateway that allows for the creation and fulfillment of cross-chain orders.
 */
interface IIntentGateway {
	/**
	 * @dev Emitted when an order is placed.
	 */
	event OrderPlaced(
		/// @dev The address of the user who is initiating the transfer
		bytes32 user,
		/// @dev The state machine identifier of the origin chain
		bytes sourceChain,
		/// @dev The state machine identifier of the destination chain
		bytes destChain,
		/// @dev The block number by which the order must be filled on the destination chain
		uint256 deadline,
		/// @dev The nonce of the order
		uint256 nonce,
		/// @dev Represents the dispatch fees associated with the IntentGateway.
		uint256 fees,
		/// @dev The tokens that the filler will provide.
		PaymentInfo[] outputs,
		/// @dev The tokens that are escrowed for the filler.
		TokenInfo[] inputs,
		/// @dev A bytes array to store the calls if any.
		bytes callData
	);

	/**
	 * @dev Emitted when an order is filled.
	 * @param commitment The unique identifier of the order.
	 * @param filler The address of the entity that filled the order.
	 */
	event OrderFilled(bytes32 indexed commitment, address filler);

	/**
	 * @dev Emitted when an escrow is released.
	 * @param commitment The unique identifier of the order.
	 */
	event EscrowReleased(bytes32 indexed commitment);

	/**
	 * @dev Emitted when an escrow is refunded.
	 * @param commitment The unique identifier of the order.
	 */
	event EscrowRefunded(bytes32 indexed commitment);

	/// @notice Thrown when an unauthorized action is attempted.
	error Unauthorized();

	/// @notice Thrown when an invalid input is provided.
	error InvalidInput();

	/// @notice Thrown when an action is attempted on an expired order.
	error Expired();

	/// @notice Thrown when there are insufficient native tokens to complete an action.
	error InsufficientNativeToken();

	/// @notice Thrown when an action is attempted on an order that has not yet expired.
	error NotExpired();

	/// @notice Thrown when an action is attempted on an order that has already been filled.
	error Filled();

	/// @notice Thrown when an action is attempted on the wrong chain.
	error WrongChain();

	/// @notice Thrown when an action is attempted on an unknown order.
	error UnknownOrder();

	/**
	 * @notice Fallback function to receive ether
	 * @dev This function is called when ether is sent to the contract without data
	 * @custom:note The function is marked payable to allow receiving ether
	 */
	receive() external payable;

	/**
	 * @dev Should return the `IsmpHost` address for the current chain.
	 * The `IsmpHost` is an immutable contract that will never change.
	 */
	function host() external view returns (address);

	/**
	 * @dev Fetch the IntentGateway contract instance for a chain.
	 */
	function instance(bytes calldata stateMachineId) external view returns (bytes32);

	/**
	 * @notice Retrieves the current parameter settings for the IntentGateway module
	 * @dev Returns a struct containing all configurable parameters
	 * @return Params A struct containing the module's current parameters
	 */
	function params() external view returns (Params memory);

	/**
	 * @notice Calculates the commitment slot hash required for storage queries.
	 * @dev The commitment slot hash is used as part of the proof verification process
	 * @param commitment The commitment value as a bytes32 hash
	 * @return bytes The calculated commitment slot hash
	 */
	function calculateCommitmentSlotHash(bytes32 commitment) external pure returns (bytes memory);

	/**
	 * @notice Places an order with the given order details.
	 * @dev This function allows users to place an order by providing the order details.
	 * @param order The order details to be placed.
	 * @param graffiti Additional data that can be attached to the order
	 */
	function placeOrder(Order memory order, bytes32 graffiti) external payable;

	/**
	 * @notice Fills an order with the specified options.
	 * @param order The order to be filled.
	 * @param options The options to be used when filling the order.
	 * @dev This function is payable and can accept Ether.
	 */
	function fillOrder(Order calldata order, FillOptions memory options) external payable;

	/**
	 * @notice Cancels an existing order.
	 * @param order The order to be canceled.
	 * @param options Additional options for the cancellation process.
	 * @dev This function can only be called by the order owner and requires a payment.
	 * It will initiate a storage query on the source chain to verify the order has not
	 * yet been filled. If the order has not been filled, the tokens will be released.
	 */
	function cancelOrder(Order calldata order, CancelOptions memory options) external payable;
}
