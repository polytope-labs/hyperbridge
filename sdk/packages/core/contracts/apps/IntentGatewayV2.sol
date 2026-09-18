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

/**
 * @notice Information for executing calls before an order is placed on the destination chain
 * @dev Used to specify pre-dispatch operations with associated assets
 */
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
    /// @dev The protocol fee in basis points charged on order inputs.
    /// 10000 = 100%, 100 = 1%, etc.
    uint256 protocolFeeBps;
    /// @dev The address of the price oracle contract.
    address priceOracle;
}

/**
 * @dev Arguments to `IntentGatewayV2.initialize`. All of it is part of the proxy's init data, so the
 * same values on every chain keep the proxy address identical across chains.
 */
struct InitParams {
    /// @dev The initial gateway configuration.
    Params params;
    /// @dev State-machine ids of the cross-chain peers to register, each bound to the gateway's own
    /// address so no peer address is carried in the init data.
    bytes[] peerChains;
    /// @dev The only relayer whose deliveries are accepted. Zero leaves the gate open.
    address relayer;
    /// @dev The owner, who may pause the gateway. Must be non-zero.
    address owner;
}

/**
 * @dev Struct to define the destination fee parameters.
 */
struct DestinationFee {
    /// @dev The percentage of fee (in basis points) charged for the destination chain.
    /// 10000 = 100%, 5000 = 50%, etc.
    uint256 destinationFeeBps;
    /// @dev The state machine ID associated with the destination fee.
    bytes chain;
}

/**
 * @dev Struct to define a parameter update request from Hyperbridge
 * @notice Contains both general parameters and destination-specific fee configurations
 */
struct ParamsUpdate {
    /// @dev The general parameters for the IntentGateway module
    Params params;
    /// @dev The destination fee parameters for specific chains
    DestinationFee[] destinationFees;
}

/**
 * @dev Struct representing the body of a withdrawal request.
 */
struct WithdrawalRequest {
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
    /// @dev Last block number at which this fill may be executed. Zero means no bound.
    ///
    /// @dev A solver bidding through the coprocessor signs this calldata and then has no
    /// further say in when it is used: the order's `deadline` is chosen by the placer with
    /// no upper limit, and retracting the bid on Hyperbridge does not reach this chain. The
    /// placer holds the session key, so without a bound here they may sit on a signed bid
    /// and execute it whenever the price has moved in their favour. Setting this caps how
    /// long the quoted price stands.
    ///
    /// @dev Denominated in blocks, matching `order.deadline`, so both are read against the
    /// same clock (`_blockNumber()`, which is the L2 block number where that differs).
    uint256 validUntil;
    /// @dev The most output the solver pays per leg, indexed like `order.output.assets`.
    /// `outputs[i] / inputs[i]` is the solver's rate for leg `i`; the leg pays the input it actually
    /// releases at that rate, rounded up, so payment never exceeds this budget.
    TokenInfo[] outputs;
    /// @dev The most input the solver takes per leg, indexed like `order.inputs`. One entry per
    /// leg is required; zero here and in `outputs[i]` skips the leg.
    TokenInfo[] inputs;
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
    uint64 height;
}

/**
 * @dev Request from hyperbridge for adding a new deployment of IntentGateway
 */
struct Deployment {
    /// @dev Identifier for the state machine.
    bytes chain;
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

    /// @notice The fill's own validity window has passed (`options.validUntil`).
    /// @dev Distinct from {Expired}, which is the order's deadline. This one means the
    ///      solver's quote has gone stale, not that the order has.
    error FillExpired();

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

    /// @notice Thrown when the cross-chain peer is unknown.
    error UnknownInstance();

    /// @notice Thrown when a solver attempts to partially fill an order that carries output
    ///         calldata. Such orders must be filled completely in a single fill.
    error PartialFillNotAllowed();
    /// @notice Thrown when a leg's quoted output over quoted input is below the order's own rate.
    error RateBelowOrder();
    /// @notice Thrown when a fill credits no output or releases no input on any leg, including
    ///         fills whose quotes are all zero or too small to move a leg by one unit.
    error RateFillTooSmall();

    /// @notice Thrown by `placeOrder`, `fillOrder` and escrow deliveries while the gateway is paused,
    ///         and by `pause` when already paused.
    error EnforcedPause();

    /// @notice Thrown by `unpause` when the gateway is not paused.
    error ExpectedPause();

    /// @notice Thrown when an owner-only function is called by anyone but the owner or the host.
    error OwnableUnauthorizedAccount(address account);

    /// @notice Thrown when `initialize` or `migrate` is given a zero owner.
    error OwnableInvalidOwner(address owner);

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
     * @param inputs The tokens that are escrowed for the filler (amounts reflect values after protocol fee deduction)
     * @param outputs The tokens that the filler will provide
     * @param predispatchCall The calldata executed via the CallDispatcher before escrow
     * @param outputCall The calldata executed on the destination chain during the fill
     * @param graffiti The attribution tag supplied by the order placer
     */
    event OrderPlaced(
        bytes32 user,
        string source,
        string destination,
        uint256 deadline,
        uint256 nonce,
        uint256 fees,
        address session,
        bytes32 beneficiary,
        TokenInfo[] predispatch,
        TokenInfo[] inputs,
        TokenInfo[] outputs,
        bytes predispatchCall,
        bytes outputCall,
        bytes32 graffiti
    );

    /**
     * @notice Emitted when an order is fully filled.
     * @param commitment The unique identifier of the order
     * @param filler The address of the entity that filled the order
     * @param outputs The credited output amounts, excluding surplus
     * @param inputs The escrowed input tokens released to the filler
     */
    event OrderFilled(bytes32 indexed commitment, address filler, TokenInfo[] outputs, TokenInfo[] inputs);

    /**
     * @notice Emitted when an order is partially filled. Only same-chain orders
     *         support incremental fills.
     * @param commitment The unique identifier of the order
     * @param filler The address of the entity that provided this partial fill
     * @param outputs The credited output amounts in this fill, excluding surplus
     * @param inputs The proportional escrowed input tokens released to the filler
     */
    event PartialFill(bytes32 indexed commitment, address filler, TokenInfo[] outputs, TokenInfo[] inputs);

    /**
     * @notice Emitted when an order's cancellation is initiated, on the chain it is
     *         initiated from. `EscrowRefunded` remains the terminal event: it follows in
     *         the same transaction for a same-chain cancel, and on the source chain once
     *         the cancellation has travelled through Hyperbridge for a cross-chain one.
     * @param commitment The unique identifier of the order
     * @param canceller The account that initiated the cancellation. Destination-side cancellation
     *        and expired same-chain cancellation are permissionless, so this may be a third party.
     */
    event OrderCancelled(bytes32 indexed commitment, address canceller);

    /**
     * @notice Emitted when an escrow is released to the filler.
     * @param commitment The unique identifier of the order
     * @param tokens The tokens and amounts released
     */
    event EscrowReleased(bytes32 indexed commitment, address solver, TokenInfo[] tokens);

    /**
     * @notice Emitted when an escrow is refunded to the original user.
     * @param commitment The unique identifier of the order
     * @param tokens The tokens and amounts refunded
     */
    event EscrowRefunded(bytes32 indexed commitment, TokenInfo[] tokens);

    /// @dev Protocol fee returned on cancellation, separate from principal in EscrowRefunded.
    event ProtocolFeeRefunded(bytes32 indexed commitment, address indexed token, uint256 amount);

    /**
     * @notice Emitted when parameters are updated.
     * @param previous The previous parameters
     * @param current The current parameters
     */
    event ParamsUpdated(Params previous, Params current);

    /**
     * @notice Emitted when a gateway instance is registered for a remote state machine.
     * @param chain The state machine identifier for the new deployment
     * @param gateway The address of the deployed gateway on that chain
     */
    event DeploymentAdded(string chain, address gateway);

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

    /**
     * @notice Emitted when a destination-specific protocol fee override is set via governance.
     * @param chain The destination state machine identifier
     * @param feeBps The protocol fee in basis points for orders targeting this destination
     */
    event DestinationProtocolFeeUpdated(string chain, uint256 feeBps);

    /**
     * @notice Emitted when the relayer authorised to deliver cross-chain messages is replaced.
     * @param previous The relayer that was authorised before this change
     * @param current The relayer authorised from now on
     */
    event RelayerUpdated(address previous, address current);

    /**
     * @notice Emitted when the owner or the host proposes a new owner (OpenZeppelin
     *         `Ownable2StepUpgradeable`); the transfer completes when `newOwner` calls
     *         `acceptOwnership`. A proposal of zero withdraws a pending one.
     * @param previousOwner The current owner
     * @param newOwner The proposed owner
     */
    event OwnershipTransferStarted(address indexed previousOwner, address indexed newOwner);

    /**
     * @notice Emitted when the owner is set, by `initialize`, `migrate`, `acceptOwnership` or
     *         `renounceOwnership`.
     * @param previousOwner The owner before this change
     * @param newOwner The owner from now on
     */
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);

    /**
     * @notice Emitted when the owner pauses the gateway.
     * @param account The owner that paused
     */
    event Paused(address account);

    /**
     * @notice Emitted when the owner resumes the gateway.
     * @param account The owner that resumed
     */
    event Unpaused(address account);

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
     * @notice The module that runs same-chain fills and cancels under delegatecall
     * @return address The `IntrinsicModule` this implementation was deployed with
     */
    function intrinsicModule() external view returns (address);

    /**
     * @notice The module that runs cross-chain fills, cancels and settlement under delegatecall
     * @return address The `ExtrinsicModule` this implementation was deployed with
     */
    function extrinsicModule() external view returns (address);

    /**
     * @notice Returns the current parameters of the module.
     * @return Params A struct containing the module's current parameters
     */
    function params() external view returns (Params memory);

    /// @notice Held placement fee and original post-fee principal of leg `index`
    /// (`order.inputs[index]`); zero for zero-fee or settled legs.
    function _protocolFees(bytes32 commitment, uint256 index) external view returns (uint256 amount, uint256 committed);

    /**
     * @notice The only relayer whose `onAccept` and `onGetResponse` deliveries are accepted.
     * @return address The authorised relayer, or zero while every relayer is accepted
     */
    function relayer() external view returns (address);

    /**
     * @notice Takes a proxy from an earlier implementation to the current version, where
     *         `initialize` puts a fresh one. Host-only and one-shot; emits `Initialized`. It is the
     *         only way up for a proxy already at a version: `initialize` is refused on anything but
     *         a bare proxy. Moves the relayer from slot 13 offset 1 to offset 0 and sets the owner.
     * @param owner The owner, who may pause the gateway; must be non-zero
     */
    function migrate(address owner) external;

    /**
     * @notice The owner, who may pause and resume the gateway.
     * @return address The owner
     */
    function owner() external view returns (address);

    /**
     * @notice The account a proposed ownership transfer is waiting on.
     * @return address The pending owner, or zero
     */
    function pendingOwner() external view returns (address);

    /**
     * @notice Proposes a new owner, who takes over on `acceptOwnership`. Callable by the owner and
     *         by the host, so governance can replace the owner. Zero withdraws a pending proposal.
     * @param newOwner The proposed owner
     */
    function transferOwnership(address newOwner) external;

    /// @notice Completes a proposed ownership transfer. Callable only by the pending owner.
    function acceptOwnership() external;

    /// @notice Clears the owner. Owner or host; governance can propose a new one afterwards.
    function renounceOwnership() external;

    /**
     * @notice Whether the gateway is paused: `placeOrder`, `fillOrder`, and escrow redemptions,
     *         refunds and cancel proofs delivered by Hyperbridge revert `EnforcedPause`. Governance
     *         deliveries and `cancelOrder` are never paused; a refused delivery can be resubmitted
     *         once the gateway resumes.
     * @return bool True while paused
     */
    function paused() external view returns (bool);

    /// @notice Pauses the gateway. Callable by the owner or the host; reverts `EnforcedPause` if already paused.
    function pause() external;

    /// @notice Resumes the gateway. Callable by the owner or the host; reverts `ExpectedPause` if not paused.
    function unpause() external;

    /**
     * @notice The `Initializable` version: 3 once `initialize` or `migrate` has run on the
     *         module-split implementation with an owner, 2 on the armed implementation before it, 1
     *         before the relayer gate. Reverts on implementations that predate the gate.
     * @return uint64 The initialized version
     */
    function version() external view returns (uint64);

    /**
     * @notice Calculates the commitment slot hash for storage proof verification.
     * @param commitment The commitment hash
     * @return bytes The calculated commitment slot hash
     */
    function calculateCommitmentSlotHash(bytes32 commitment) external pure returns (bytes memory);

    /**
     * @notice Places an order for cross-chain intent fulfillment.
     * @dev If protocolFeeBps is configured, a protocol fee is deducted from each input token amount.
     *      The full input amounts are escrowed, but the OrderPlaced event emits reduced amounts (after fee).
     *      Protocol fees stay reserved until final settlement. Cancellation refunds the fee
     *      attributable to unfilled principal; only the earned remainder becomes sweepable dust.
     * @param order The order to be placed
     * @param graffiti The arbitrary data used for identification purposes
     */
    function placeOrder(Order memory order, bytes32 graffiti) external payable;

    /**
     * @notice Selects a solver for an order (when solver selection is enabled).
     * @param options The options for selecting a solver
     * @return sessionKey The recovered session key address
     */
    function select(SelectOptions calldata options) external returns (address sessionKey);

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
