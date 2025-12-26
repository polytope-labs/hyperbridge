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
pragma solidity ^0.8.24;

/**
 * @notice Tokens that must be received for a valid order fulfillment
 */
struct PaymentInfo {
    /// @dev The address to receive the output tokens
    bytes32 beneficiary;
    /// @dev The assets to be provided by the filler
    TokenInfo[] assets;
    /// @dev Optional calldata to be executed on the destination chain
    bytes call;
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

struct DispatchInfo {
    /// @dev Assets to execute a predispatch call with
    TokenInfo[] assets;
    /// @dev The actual call data to be executed
    bytes call;
}

/**
 * @dev Represents an order in the IntentGateway module.
 * @param Order The structure defining an order.
 */
struct Order {
    /// @dev The address of the user who is initiating the transfer
    bytes32 user;
    /// @dev The state machine identifier of the origin chain
    bytes source;
    /// @dev The state machine identifier of the destination chain
    bytes destination;
    /// @dev The block number by which the order must be filled on the destination chain
    uint256 deadline;
    /// @dev The nonce of the order
    uint256 nonce;
    /// @dev Represents the dispatch fees associated with the IntentGateway.
    uint256 fees;
    /// @dev Optional session key used to select winning solver.
    address session;
    /// @dev The predispatch information for the order
    /// This is used to encode any calls before the order is placed
    DispatchInfo predispatch;
    /// @dev The tokens that are escrowed for the filler.
    TokenInfo[] inputs;
    /// @dev The filler output, ie the tokens that the filler will provide
    PaymentInfo output;
}

/**
 * @dev Request from hyperbridge for sweeping accumulated dust
 */
struct SweepDust {
    /// @dev The address of the beneficiary of the protocol fee
    address beneficiary;
    /// @dev The tokens to be withdrawn
    TokenInfo[] outputs;
}

/**
 * @dev Struct to define the parameters for the IntentGateway module.
 */
struct Params {
    /// @dev The address of the host contract
    address host;
    /// @dev Address of the dispatcher contract responsible for handling intents.
    address dispatcher;
    /// @dev Flag indicating whether solver selection is enabled.
    bool solverSelection;
    /// @dev The percentage of surplus (in basis points) that goes to the protocol. The rest goes to beneficiary.
    /// 10000 = 100%, 5000 = 50%, etc.
    uint256 surplusShareBps;
}

/**
 * @dev Struct representing the body of a request.
 */
struct RequestBody {
    /// @dev Represents the commitment of an order. This is typically a hash that uniquely identifies the order.
    bytes32 commitment;
    /// @dev Stores the identifier for the beneficiary.
    bytes32 beneficiary;
    /// @dev An array of token identifiers. Each element in the array represents a unique token involved in the order.
    TokenInfo[] tokens;
}

/**
 * @notice A struct representing the options for filling an intent.
 * @dev This struct is used to specify various parameters and options
 *      when filling an intent in the IntentGateway contract.
 */
struct FillOptions {
    /// @dev The fee paid in feeTokens to the relayer for processing transactions.
    uint256 relayerFee;
    /// @dev The fee paid in native tokens for cross-chain dispatch.
    uint256 nativeDispatchFee;
    /// @dev The output tokens with amounts the solver is willing to give
    /// @dev Must be strictly >= the amounts requested in order.output.assets
    TokenInfo[] outputs;
}

/**
 * @notice A struct representing the options for selecting a solver
 * @dev This struct is used to specify various parameters and options
 *      when selecting a solver.
 */
struct SelectOptions {
    /// @dev The commitment hash of the order.
    bytes32 commitment;
    /// @dev The solver address to select.
    address solver;
    /// @dev The EIP-712 signature from the session key that signed SelectSolver(commitment, solver)
    bytes signature;
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
 * @dev Request from hyperbridge for adding a new deployment of IntentGateway
 */
struct NewDeployment {
    /// @dev Identifier for the state machine.
    bytes stateMachineId;
    /// @dev An address variable to store the gateway identifier.
    address gateway;
}

/**
 * @title IIntentGatewayV2
 * @author Polytope Labs (hello@polytope.technology)
 * @notice Interface for the IntentGatewayV2 contract
 * @dev Defines all external functions, events, and errors for cross-chain intent fulfillment
 */
interface IIntentGatewayV2 {
    // ============================================
    // Errors
    // ============================================

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

    // ============================================
    // Events
    // ============================================

    /**
     * @notice Emitted when an order is placed.
     * @param user The address of the user who is initiating the transfer
     * @param source The state machine identifier of the origin chain
     * @param destination The state machine identifier of the destination chain
     * @param deadline The block number by which the order must be filled
     * @param nonce The nonce of the order
     * @param fees The dispatch fees associated with the order
     * @param session Optional session key used to select winning solver
     * @param beneficiary The address to receive the output tokens
     * @param predispatch The predispatch assets for the order
     * @param inputs The tokens that are escrowed for the filler
     * @param outputs The tokens that the filler will provide
     */
    event OrderPlaced(
        bytes32 user,
        bytes source,
        bytes destination,
        uint256 deadline,
        uint256 nonce,
        uint256 fees,
        address session,
        bytes32 beneficiary,
        TokenInfo[] predispatch,
        TokenInfo[] inputs,
        TokenInfo[] outputs
    );

    /**
     * @notice Emitted when an order is filled.
     * @param commitment The unique identifier of the order
     * @param filler The address of the entity that filled the order
     */
    event OrderFilled(bytes32 indexed commitment, address filler);

    /**
     * @notice Emitted when an escrow is released.
     * @param commitment The unique identifier of the order
     */
    event EscrowReleased(bytes32 indexed commitment);

    /**
     * @notice Emitted when an escrow is refunded.
     * @param commitment The unique identifier of the order
     */
    event EscrowRefunded(bytes32 indexed commitment);

    /**
     * @notice Emitted when parameters are updated.
     * @param previous The previous parameters
     * @param current The current parameters
     */
    event ParamsUpdated(Params previous, Params current);

    /**
     * @notice Emitted when a new deployment is added.
     * @param stateMachineId The state machine identifier
     * @param gateway The gateway identifier
     */
    event NewDeploymentAdded(bytes stateMachineId, address gateway);

    /**
     * @notice Emitted when dust is collected.
     * @param token The token address
     * @param amount The amount of dust collected
     */
    event DustCollected(address token, uint256 amount);

    /**
     * @notice Emitted when dust is swept to a beneficiary.
     * @param token The token address
     * @param amount The amount swept
     * @param beneficiary The beneficiary of the funds
     */
    event DustSwept(address token, uint256 amount, address beneficiary);

    // ============================================
    // Constants
    // ============================================

    /**
     * @notice EIP-712 type hash for SelectSolver message
     */
    function SELECT_SOLVER_TYPEHASH() external view returns (bytes32);

    /**
     * @notice EIP-712 domain separator
     */
    function DOMAIN_SEPARATOR() external view returns (bytes32);

    // ============================================
    // Functions
    // ============================================

    /**
     * @notice Returns the host contract address.
     * @return address The address of the IsmpHost contract
     */
    function host() external view returns (address);

    /**
     * @notice Fetch the IntentGateway contract instance for a chain.
     * @param stateMachineId The state machine identifier
     * @return address The gateway address for the given state machine
     */
    function instance(bytes calldata stateMachineId) external view returns (address);

    /**
     * @notice Sets the parameters for the IntentGateway module.
     * @param p The parameters to be set, encapsulated in a Params struct
     */
    function setParams(Params memory p) external;

    /**
     * @notice Returns the current parameters of the module.
     * @return Params A struct containing the module's current parameters
     */
    function params() external view returns (Params memory);

    /**
     * @notice Calculates the commitment slot hash for storage proof verification.
     * @param commitment The commitment hash
     * @return bytes The calculated commitment slot hash
     */
    function calculateCommitmentSlotHash(bytes32 commitment) external pure returns (bytes memory);

    /**
     * @notice Places an order for cross-chain intent fulfillment.
     * @param order The order to be placed
     * @param graffiti The arbitrary data used for identification purposes
     */
    function placeOrder(Order memory order, bytes32 graffiti) external payable;

    /**
     * @notice Selects a solver for an order (when solver selection is enabled).
     * @param options The options for selecting a solver
     */
    function select(SelectOptions calldata options) external;

    /**
     * @notice Fills an order with the specified options.
     * @param order The order to be filled
     * @param options The options to be used when filling the order
     */
    function fillOrder(Order calldata order, FillOptions calldata options) external payable;

    /**
     * @notice Cancels an order after it has expired.
     * @param order The order to be cancelled
     * @param options The cancellation options
     */
    function cancelOrder(Order calldata order, CancelOptions calldata options) external payable;
}
