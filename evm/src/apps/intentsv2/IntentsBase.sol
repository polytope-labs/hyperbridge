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

import {IDispatcher} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {
    TokenInfo,
    Order,
    Params,
    ParamsUpdate,
    SweepDust,
    WithdrawalRequest,
    SelectOptions,
    NewDeployment
} from "@hyperbridge/core/apps/IntentGatewayV2.sol";

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";

import {ICallDispatcher, Call} from "../../interfaces/ICallDispatcher.sol";

/**
 * @title IntentsBase
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @dev Abstract base contract for the IntentGateway. Contains all shared state,
 * constants, errors, events, and chain-agnostic utility functions.
 */
abstract contract IntentsBase is EIP712 {
    using SafeERC20 for IERC20;

    /**
     * @dev EIP-712 typehash for solver selection signatures.
     * Encodes the struct: SelectSolver(bytes32 commitment, address solver).
     */
    bytes32 public constant SELECT_SOLVER_TYPEHASH = keccak256("SelectSolver(bytes32 commitment,address solver)");

    /**
     * @dev Sentinel address used as the key for storing Hyperbridge relayer fees
     * in the `_orders` mapping. Derived from keccak256("txFees") to avoid
     * collisions with real token addresses.
     */
    address internal constant TRANSACTION_FEES = address(uint160(uint256(keccak256("txFees"))));

    /**
     * @dev Big-endian encoding of storage slot 2 (the `_filled` mapping slot).
     * Used to construct storage proof keys for cross-chain cancel verification.
     */
    bytes32 constant FILLED_SLOT_BIG_ENDIAN_BYTES =
        hex"0000000000000000000000000000000000000000000000000000000000000002";

    /**
     * @dev Discriminator for cross-chain request types dispatched via Hyperbridge.
     * Encoded as the first byte of the request body in onAccept.
     */
    enum RequestKind {
        /**
         * @dev Release escrowed tokens to the solver after a successful cross-chain fill.
         */
        RedeemEscrow,
        /**
         * @dev Register a new gateway deployment for a remote state machine.
         */
        NewDeployment,
        /**
         * @dev Update gateway configuration parameters and destination fees.
         */
        UpdateParams,
        /**
         * @dev Sweep accumulated protocol dust to a beneficiary.
         */
        SweepDust,
        /**
         * @dev Refund escrowed tokens to the user after a cross-chain cancellation.
         */
        RefundEscrow
    }

    /**
     * @dev Maps order commitment hashes to the address that filled or refunded the order.
     * A non-zero value indicates the order has been finalized and cannot be filled again.
     */
    mapping(bytes32 => address) public _filled;

    /**
     * @dev Monotonically increasing counter used to assign unique nonces to orders.
     * Each call to `placeOrder` consumes and increments this value.
     */
    uint256 public _nonce;

    /**
     * @dev Gateway configuration parameters including host address, dispatcher,
     * fee settings, price oracle, and solver selection toggle.
     */
    Params internal _params;

    /**
     * @dev One-time admin address set in the constructor. Has permission to call
     * `setParams` exactly once, after which it is burned to address(0).
     */
    address internal _admin;

    /**
     * @dev Maps (commitment, token address) to the escrowed amount for that token.
     * Decremented as tokens are released via fills or refunds.
     */
    mapping(bytes32 => mapping(address => uint256)) public _orders;

    /**
     * @dev Maps keccak256(stateMachineId) to the registered gateway address for
     * that chain. Used for authenticating cross-chain messages and routing dispatches.
     */
    mapping(bytes32 => address) public _instances;

    /**
     * @dev Maps (commitment, output token) to the cumulative amount already filled.
     * Used to track partial fill progress for same-chain orders.
     */
    mapping(bytes32 => mapping(bytes32 => uint256)) public _partialFills;

    /**
     * @dev Maps keccak256(stateMachineId) to a destination-specific protocol fee
     * override in basis points. If zero, the global `_params.protocolFeeBps` is used.
     */
    mapping(bytes32 => uint256) public _destinationProtocolFees;

    /**
     * @dev Thrown when the caller is not authorized to perform the action.
     */
    error Unauthorized();

    /**
     * @dev Thrown when function arguments fail validation.
     */
    error InvalidInput();

    /**
     * @dev Thrown when attempting to fill an order past its deadline.
     */
    error Expired();

    /**
     * @dev Thrown when insufficient native token (ETH) is provided or a transfer fails.
     */
    error InsufficientNativeToken();

    /**
     * @dev Thrown when attempting to cancel an order that has not yet expired.
     */
    error NotExpired();

    /**
     * @dev Thrown when attempting to fill or cancel an order that has already been finalized.
     */
    error Filled();

    /**
     * @dev Thrown when attempting to act on an already cancelled order.
     */
    error Cancelled();

    /**
     * @dev Thrown when an operation is invoked on the wrong chain for the given order.
     */
    error WrongChain();

    /**
     * @dev Thrown when no escrow exists for the given commitment and token.
     */
    error UnknownOrder();

    /**
     * @dev Emitted when a new intent order is placed and input tokens are escrowed.
     * @param user The order creator's address encoded as bytes32.
     * @param source The source chain's state machine identifier.
     * @param destination The destination chain's state machine identifier.
     * @param deadline The block number after which the order expires.
     * @param nonce The unique order nonce assigned by the gateway.
     * @param fees The Hyperbridge relayer fees paid by the user.
     * @param session The session key address authorized to select a solver.
     * @param beneficiary The recipient of the output tokens on the destination chain.
     * @param predispatch The tokens sent to the CallDispatcher for pre-order execution.
     * @param inputs The escrowed input tokens (after protocol fee deduction).
     * @param outputs The desired output tokens on the destination chain.
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
     * @dev Emitted when an order is fully filled by a solver.
     * @param commitment The order commitment hash.
     * @param filler The address of the solver who filled the order.
     */
    event OrderFilled(bytes32 indexed commitment, address filler);

    /**
     * @dev Emitted when an order is partially filled by a solver. Only applicable
     * to same-chain orders which support incremental fills.
     * @param commitment The order commitment hash.
     * @param filler The address of the solver who provided this partial fill.
     * @param outputs The output token amounts provided in this fill.
     * @param inputs The proportional escrowed input tokens released to the solver.
     */
    event PartialFill(bytes32 indexed commitment, address filler, TokenInfo[] outputs, TokenInfo[] inputs);

    /**
     * @dev Emitted when escrowed tokens are released to the solver after a successful fill.
     * @param commitment The order commitment hash.
     */
    event EscrowReleased(bytes32 indexed commitment);

    /**
     * @dev Emitted when escrowed tokens are refunded to the original user after cancellation.
     * @param commitment The order commitment hash.
     */
    event EscrowRefunded(bytes32 indexed commitment);

    /**
     * @dev Emitted when the gateway's configuration parameters are updated via governance.
     * @param previous The previous parameter values.
     * @param current The new parameter values.
     */
    event ParamsUpdated(Params previous, Params current);

    /**
     * @dev Emitted when a new gateway instance is registered for a remote state machine.
     * @param stateMachineId The state machine identifier for the new deployment.
     * @param gateway The address of the deployed gateway on that chain.
     */
    event NewDeploymentAdded(bytes stateMachineId, address gateway);

    /**
     * @dev Emitted when surplus tokens are retained by the protocol. This includes
     * protocol fee deductions, surplus shares from overpayment, and residual
     * balances swept from the CallDispatcher after calldata execution.
     * @param token The token address (address(0) for native token).
     * @param amount The amount collected.
     */
    event DustCollected(address token, uint256 amount);

    /**
     * @dev Emitted when accumulated protocol dust is swept to a beneficiary via governance.
     * @param token The token address (address(0) for native token).
     * @param amount The amount swept.
     * @param beneficiary The recipient of the swept tokens.
     */
    event DustSwept(address token, uint256 amount, address beneficiary);

    /**
     * @dev Emitted when a destination-specific protocol fee override is set via governance.
     * @param stateMachineId The destination state machine identifier.
     * @param feeBps The protocol fee in basis points for orders targeting this destination.
     */
    event DestinationProtocolFeeUpdated(bytes32 indexed stateMachineId, uint256 feeBps);

    /**
     * @dev Returns the address of the Hyperbridge host contract. This function is virtual
     * to allow derived contracts to resolve diamond inheritance conflicts.
     * @return The host contract address from stored params.
     */
    function host() public view virtual returns (address) {
        return _params.host;
    }

    /**
     * @dev Returns the EIP-712 domain separator for this contract. Used by off-chain
     * signers to construct typed data hashes for solver selection signatures.
     * @return The EIP-712 domain separator hash.
     */
    function DOMAIN_SEPARATOR() public view returns (bytes32) {
        return _domainSeparatorV4();
    }

    /**
     * @dev Resolves the IntentGateway instance address for a given state machine.
     * Falls back to `address(this)` if no remote deployment has been registered,
     * meaning this contract is the canonical gateway for that chain.
     * @param stateMachineId The raw state machine identifier bytes.
     * @return The gateway address for the given state machine.
     */
    function _instance(bytes calldata stateMachineId) internal view returns (address) {
        address gateway = _instances[keccak256(stateMachineId)];
        return gateway == address(0) ? address(this) : gateway;
    }

    /**
     * @dev Computes the storage slot hash for a given commitment in the `_filled` mapping.
     * This is used to construct storage proof keys for cross-chain cancellation verification
     * via Hyperbridge GET requests.
     * @param commitment The order commitment hash.
     * @return The ABI-encoded storage slot hash.
     */
    function _calculateCommitmentSlotHash(bytes32 commitment) internal pure returns (bytes memory) {
        return abi.encodePacked(keccak256(abi.encodePacked(commitment, FILLED_SLOT_BIG_ENDIAN_BYTES)));
    }

    /**
     * @dev Releases escrowed tokens to a beneficiary. Iterates over the withdrawal request's
     * token list, decrements the escrow balance for each, and transfers tokens out.
     *
     * When `finalize` is true, the order is marked as filled in the `_filled` mapping,
     * any accumulated transaction fees (in the protocol fee token) are forwarded to the
     * beneficiary, and the appropriate event (EscrowReleased or EscrowRefunded) is emitted.
     *
     * When `finalize` is false (partial fills), only the proportional token amounts are
     * released without finalizing the order.
     *
     * @param body The withdrawal request containing the commitment, token amounts, and beneficiary.
     * @param isRefund If true, emits EscrowRefunded instead of EscrowReleased on finalization.
     * @param finalize If true, marks the order as complete and releases accumulated fees.
     */
    function _withdraw(WithdrawalRequest memory body, bool isRefund, bool finalize) internal {
        address beneficiary = address(uint160(uint256(body.beneficiary)));
        if (finalize) _filled[body.commitment] = beneficiary;

        uint256 len = body.tokens.length;
        for (uint256 i; i < len; i++) {
            address token = address(uint160(uint256(body.tokens[i].token)));
            uint256 amount = body.tokens[i].amount;
            if (amount == 0) continue;

            uint256 escrowed = _orders[body.commitment][token];
            if (escrowed == 0) revert UnknownOrder();

            _orders[body.commitment][token] = escrowed - amount;
            if (token == address(0)) {
                (bool sent,) = beneficiary.call{value: amount}("");
                if (!sent) revert InsufficientNativeToken();
            } else {
                IERC20(token).safeTransfer(beneficiary, amount);
            }
        }

        if (finalize) {
            uint256 fees = _orders[body.commitment][TRANSACTION_FEES];
            if (fees > 0) {
                delete _orders[body.commitment][TRANSACTION_FEES];
                IERC20(IDispatcher(host()).feeToken()).safeTransfer(beneficiary, fees);
            }

            if (isRefund) {
                emit EscrowRefunded({commitment: body.commitment});
            } else {
                emit EscrowReleased({commitment: body.commitment});
            }
        }
    }

    /**
     * @dev Executes arbitrary calldata attached to an order's output via the CallDispatcher.
     * After dispatching the calls, any residual token balances left on the dispatcher
     * are swept back to this contract and accounted for as protocol dust.
     *
     * This enables composable order fulfillment — solvers can route through DEXes,
     * lending protocols, or other DeFi primitives as part of filling an order.
     *
     * @param order The order containing the output calldata to execute.
     * @param outputsLen The number of output assets to sweep after execution.
     */
    function _execute(Order calldata order, uint256 outputsLen) internal {
        if (order.output.call.length == 0) return;

        address dispatcher = _params.dispatcher;
        ICallDispatcher(dispatcher).dispatch(order.output.call);

        Call[] memory sweepCalls = new Call[](outputsLen);
        uint256 sweepCount = 0;

        for (uint256 i; i < outputsLen;) {
            address token = address(uint160(uint256(order.output.assets[i].token)));

            if (token == address(0)) {
                uint256 balance = dispatcher.balance;
                if (balance > 0) {
                    sweepCalls[sweepCount] = Call({to: address(this), value: balance, data: ""});
                    sweepCount++;
                    emit DustCollected(token, balance);
                }
            } else {
                uint256 balance = IERC20(token).balanceOf(dispatcher);
                if (balance > 0) {
                    sweepCalls[sweepCount] = Call({
                        to: token,
                        value: 0,
                        data: abi.encodeWithSelector(IERC20.transfer.selector, address(this), balance)
                    });
                    sweepCount++;
                    emit DustCollected(token, balance);
                }
            }

            unchecked {
                ++i;
            }
        }

        if (sweepCount > 0) {
            Call[] memory finalCalls = new Call[](sweepCount);
            for (uint256 i; i < sweepCount;) {
                finalCalls[i] = sweepCalls[i];
                unchecked {
                    ++i;
                }
            }
            ICallDispatcher(dispatcher).dispatch(abi.encode(finalCalls));
        }
    }

    /**
     * @dev Verifies an EIP-712 solver selection signature and stores the selected solver
     * and session key in transient storage. The solver and session key are stored using
     * `tstore` so they are only available within the same transaction — this ensures
     * atomicity between `select` and `fillOrder` calls.
     *
     * The session key is recovered from the EIP-712 signature over the (commitment, solver)
     * tuple. The recovered address is stored at `commitment + 1` in transient storage,
     * while the solver address is stored at the commitment slot itself.
     *
     * @param options The selection options containing the commitment, solver address, and signature.
     * @return The recovered session key address.
     */
    function _select(SelectOptions calldata options) internal returns (address) {
        bytes32 structHash = keccak256(abi.encode(SELECT_SOLVER_TYPEHASH, options.commitment, options.solver));
        bytes32 digest = _hashTypedDataV4(structHash);
        address sessionKey = ECDSA.recover(digest, options.signature);

        bytes32 commitment = options.commitment;
        bytes32 solver = bytes32(uint256(uint160(options.solver)));
        bytes32 sessionKeyBytes = bytes32(uint256(uint160(sessionKey)));
        bytes32 sessionSlot = bytes32(uint256(commitment) + 1);
        assembly {
            tstore(commitment, solver)
            tstore(sessionSlot, sessionKeyBytes)
        }

        return sessionKey;
    }

    /**
     * @dev Registers a new IntentGateway deployment for a remote state machine.
     * Called when Hyperbridge governance adds support for a new chain. The gateway
     * address is stored in `_instances` keyed by the hash of the state machine ID.
     *
     * @param body The deployment info containing the state machine ID and gateway address.
     */
    function _addDeployment(NewDeployment memory body) internal {
        _instances[keccak256(body.stateMachineId)] = body.gateway;
        emit NewDeploymentAdded({stateMachineId: body.stateMachineId, gateway: body.gateway});
    }

    /**
     * @dev Updates the gateway's configuration parameters and per-destination protocol fees.
     * Called by Hyperbridge governance to modify fee settings, host address, dispatcher,
     * price oracle, and other operational parameters.
     *
     * Emits ParamsUpdated with the old and new params, then iterates over any destination-
     * specific fee overrides and applies them to `_destinationProtocolFees`.
     *
     * @param update The parameter update containing new params and destination fee overrides.
     */
    function _updateParams(ParamsUpdate memory update) internal {
        emit ParamsUpdated({previous: _params, current: update.params});
        _params = update.params;

        for (uint256 i; i < update.destinationFees.length;) {
            bytes32 stateMachineId = update.destinationFees[i].stateMachineId;
            uint256 feeBps = update.destinationFees[i].destinationFeeBps;
            _destinationProtocolFees[stateMachineId] = feeBps;

            unchecked {
                ++i;
            }
            emit DestinationProtocolFeeUpdated(stateMachineId, feeBps);
        }
    }

    /**
     * @dev Transfers accumulated protocol dust (surplus tokens) to a specified beneficiary.
     * Called by Hyperbridge governance to sweep protocol-owned tokens that have accumulated
     * from fees, surplus splits, and calldata execution residuals.
     *
     * Supports both native tokens and ERC-20 tokens.
     *
     * @param req The sweep request containing the beneficiary address and token amounts.
     */
    function _sweepDust(SweepDust memory req) internal {
        uint256 outputsLen = req.outputs.length;
        for (uint256 i; i < outputsLen;) {
            TokenInfo memory info = req.outputs[i];
            address token = address(uint160(uint256(info.token)));
            uint256 amount = info.amount;

            if (token == address(0)) {
                (bool sent,) = req.beneficiary.call{value: amount}("");
                if (!sent) revert InsufficientNativeToken();
            } else {
                IERC20(token).safeTransfer(req.beneficiary, amount);
            }
            unchecked {
                ++i;
            }
            emit DustSwept(token, amount, req.beneficiary);
        }
    }
}
