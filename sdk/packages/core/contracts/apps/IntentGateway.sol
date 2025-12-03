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
 * @dev Struct for predispatch information
 */
struct PredispatchInfo {
    /// @dev Assets to execute a predispatch call with
    TokenInfo[] assets;
    /// @dev The actual call data to be executed
    bytes call;
}

/**
 * @dev Represents an order in the IntentGateway module.
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
    /// @dev The predispatch information for the order
    PredispatchInfo predispatch;
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
    /// @dev Protocol fee in basis points (BPS) deducted from filler-provided tokens
    uint256 protocolFeeBps;
}

/**
 * @notice A struct representing the options for filling an intent.
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
 * @title IIntentGatewayV2
 * @author Polytope Labs (hello@polytope.technology)
 * @notice Interface for the IntentGatewayV2 contract
 * @dev The IntentGateway allows for the creation and fulfillment of cross-chain orders.
 */
interface IIntentGateway {
    /**
     * @dev Enum representing the different kinds of incoming requests that can be executed.
     */
    enum RequestKind {
        /// @dev Identifies a request for redeeming an escrow.
        RedeemEscrow,
        /// @dev Identifies a request for recording new contract deployments
        NewDeployment,
        /// @dev Identifies a request for updating parameters.
        UpdateParams,
        /// @dev Identifies a request for refunding an escrow.
        RefundEscrow,
        /// @dev Identifies a request for collecting fees.
        CollectFees
    }

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

    /// @notice Thrown when an action is attempted on an order that has been cancelled.
    error Cancelled();

    /// @notice Thrown when an action is attempted on the wrong chain.
    error WrongChain();

    /// @notice Thrown when an action is attempted on an unknown order.
    error UnknownOrder();

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
        /// @dev Assets that were used to execute a predispatch call
        TokenInfo[] predispatch,
        /// @dev The tokens that are escrowed for the filler.
        TokenInfo[] inputs,
        /// @dev The tokens that the filler will provide.
        PaymentInfo[] outputs
    );

    /**
     * @notice Event emitted when an order is filled.
     * @param commitment The order commitment hash
     * @param filler The address of the filler
     */
    event OrderFilled(bytes32 commitment, address filler);

    /**
     * @notice Event emitted when escrow is released.
     * @param commitment The order commitment hash
     */
    event EscrowReleased(bytes32 commitment);

    /**
     * @notice Event emitted when escrow is refunded.
     * @param commitment The order commitment hash
     */
    event EscrowRefunded(bytes32 commitment);

    /**
     * @notice Event emitted when parameters are updated.
     * @param previous The previous parameters
     * @param current The new parameters
     */
    event ParamsUpdated(Params previous, Params current);

    /**
     * @notice Event emitted when a new deployment is added.
     * @param stateMachineId The state machine identifier
     * @param gateway The gateway address
     */
    event NewDeploymentAdded(bytes stateMachineId, bytes32 gateway);

    /**
     * @dev Emitted when dust is collected from predispatch swaps.
     * @param token The token contract address of the dust, address(0) for native currency.
     * @param amount The amount of dust collected.
     */
    event DustCollected(address token, uint256 amount);

    /**
     * @dev Emitted when protocol fee is collected from a filler.
     * @param token The token contract address of the fee, address(0) for native currency.
     * @param amount The amount of protocol fee collected.
     * @param chain The chain where the funds are stored.
     */
    event FeeCollected(address token, uint256 amount, bytes chain);

    /**
     * @dev Emitted when protocol revenue is withdrawn.
     * @param token The token contract address of the fee, address(0) for native currency.
     * @param amount The amount of protocol revenue collected.
     * @param beneficiary The beneficiary of the funds
     */
    event RevenueWithdrawn(address token, uint256 amount, address beneficiary);

    /**
     * @notice Returns the host address.
     * @return The host contract address
     */
    function host() external view returns (address);

    /**
     * @notice Returns the instance address for a given state machine.
     * @param stateMachineId The state machine identifier
     * @return The instance address
     */
    function instance(bytes calldata stateMachineId) external view returns (bytes32);

    /**
     * @notice Returns the current gateway parameters.
     * @return The current parameters
     */
    function params() external view returns (Params memory);

    /**
     * @notice Calculates the commitment slot hash for an order.
     * @param commitment The order commitment
     * @return The slot hash
     */
    function calculateCommitmentSlotHash(bytes32 commitment) external pure returns (bytes32);

    /**
     * @notice Places a new order.
     * @param order The order details
     * @param graffiti Additional data
     */
    function placeOrder(Order memory order, bytes32 graffiti) external payable;

    /**
     * @notice Fills an existing order.
     * @param order The order to fill
     * @param options Fill options including relayer fee
     */
    function fillOrder(Order calldata order, FillOptions memory options) external payable;

    /**
     * @notice Cancels an order (for expired orders).
     * @param order The order to cancel
     * @param options Cancel options including height and relayer fee
     */
    function cancelOrder(Order calldata order, CancelOptions memory options) external payable;

    /**
     * @notice Cancels a limit order (for orders with deadline = 0).
     * @param order The order to cancel
     * @param options Cancel options including relayer fee
     */
    function cancelLimitOrder(Order calldata order, CancelOptions memory options) external payable;
}
