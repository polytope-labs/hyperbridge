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
    Deployment
} from "@hyperbridge/core/apps/IntentGatewayV2.sol";

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";

import {ICallDispatcher, Call} from "@hyperbridge/core/interfaces/ICallDispatcher.sol";

/**
 * @dev Minimal interface for the ArbSys precompile available on Arbitrum chains
 * at address(100). Exposes the L2 block number, since the `block.number` opcode
 * on Arbitrum returns the (approximate) L1 block number instead.
 */
interface IArbSys {
    function arbBlockNumber() external view returns (uint256);
}

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
     * @dev Sentinel key under which the Hyperbridge relayer fees are held in `_orders`. The low
     * 160 bits of keccak256("txFees"), far above any leg index, and the same slot the fee pot
     * occupied when `_orders` was keyed by token address.
     */
    uint256 internal constant TRANSACTION_FEES = uint160(uint256(keccak256("txFees")));

    /**
     * @dev Big-endian encoding of storage slot 2 (the `_filled` mapping slot).
     * Used to construct storage proof keys for cross-chain cancel verification.
     */
    bytes32 constant FILLED_SLOT_BIG_ENDIAN_BYTES =
        hex"0000000000000000000000000000000000000000000000000000000000000002";

    /**
     * @dev Big-endian encoding of storage slot 11 (the `_partialFills` mapping slot).
     * Used to construct storage proof keys for cross-chain partial-fill cancel verification.
     * Asserted against the compiled storage layout in the test suite to catch layout drift.
     */
    bytes32 constant PARTIAL_FILLS_SLOT_BIG_ENDIAN_BYTES =
        hex"000000000000000000000000000000000000000000000000000000000000000b";

    /**
     * @dev The ArbSys precompile address on Arbitrum chains.
     */
    address internal constant ARB_SYS = address(100);

    /**
     * @dev Chain ids for Arbitrum One, Arbitrum Nova, and Arbitrum Sepolia.
     */
    uint256 internal constant ARBITRUM_ONE = 42161;
    uint256 internal constant ARBITRUM_NOVA = 42170;
    uint256 internal constant ARBITRUM_SEPOLIA = 421614;

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
        RefundEscrow,
        /**
         * @dev Delegatecall the extrinsic module with the rest of the body as calldata, the host
         * still `msg.sender`. Governance's one door to the host-only functions:
         * `upgradeToAndCall` for upgrades, `setRelayer` for rotations. Same discriminator as the
         * `UpgradeContract` action of earlier implementations, whose `(address, bytes)` body
         * selects no function here and reverts.
         */
        Execute,
        /**
         * @dev Release a proportional slice of escrowed tokens to the solver after a
         * cross-chain partial fill, without finalizing the order. The completing fill
         * uses `RedeemEscrow` (which finalizes and forwards accumulated fees).
         */
        RedeemEscrowPartial
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
     * @dev Maps (commitment, leg index) to the escrow still held for `order.inputs[index]`, and
     * `TRANSACTION_FEES` to the relayer fee pot. Decremented as the leg is released via fills or
     * refunds. Keyed by leg rather than token so legs that repeat a token never share a balance.
     */
    mapping(bytes32 => mapping(uint256 => uint256)) public _orders;

    /**
     * @dev Maps keccak256(stateMachineId) to the registered gateway address for
     * that chain. Used for authenticating cross-chain messages and routing dispatches.
     * Read through `instance(bytes)`; the auto-generated getter was dropped for EIP-170 room.
     */
    mapping(bytes32 => address) internal _instances;

    /**
     * @dev Maps (commitment, leg index) to the cumulative amount of `order.output.assets[index]`
     * already filled, on the chain the order is filled on. Keyed by leg rather than token so legs
     * that repeat an output token track their progress independently. Proven cross-chain by
     * source-side cancellation, see `_calculatePartialFillSlotHash`.
     */
    mapping(bytes32 => mapping(uint256 => uint256)) public _partialFills;

    /**
     * @dev Maps keccak256(stateMachineId) to a destination-specific protocol fee
     * override in basis points. If zero, the global `_params.protocolFeeBps` is used.
     */
    mapping(bytes32 => uint256) public _destinationProtocolFees;

    /**
     * @dev Once set, the only relayer whose deliveries `onAccept` and `onGetResponse` accept. Slot 13
     * offset 0. Earlier implementations packed it at offset 1, behind an unused `bool _paused` that
     * has since been removed; `IntentGatewayV2.migrate` moves it.
     */
    address internal _relayer;

    /// @dev Exact placement fee and original post-fee input; absent for legacy and zero-fee orders.
    struct ProtocolFee {
        uint256 amount;
        uint256 committed;
    }

    /// @dev Appended accounting shared by the implementation and both delegatecall modules, keyed
    /// by (commitment, leg index) like `_orders`.
    mapping(bytes32 => mapping(uint256 => ProtocolFee)) public _protocolFees;

    /**
     * @dev This contract's own address. Under delegatecall `address(this)` is the proxy instead,
     * so a module uses this to refuse direct calls and to delegatecall itself for `Execute`.
     */
    address internal immutable __self = address(this);

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
     * @notice The fill's own validity window has passed (`options.validUntil`)
     * @dev Distinct from {Expired}: that one means the order is dead, this one means the
     *      solver's quote is stale.
     */
    error FillExpired();

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

    /*
     * @dev Thrown when the cross-chain peer is unknown.
    */
    error UnknownInstance();

    /*
     * @dev Thrown when a solver attempts to partially fill an order that carries
     * output calldata. Such orders must be filled completely in a single fill.
    */
    error PartialFillNotAllowed();

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
     * @param predispatchCall The calldata executed via the CallDispatcher before escrow.
     * @param outputCall The calldata executed on the destination chain during the fill.
     * @param graffiti The attribution tag supplied by the order placer.
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
     * @dev Emitted when an order is fully filled by a solver.
     * @param commitment The order commitment hash.
     * @param filler The address of the solver who filled the order.
     * @param outputs The output token amounts provided by the solver.
     * @param inputs The escrowed input tokens released to the solver.
     */
    event OrderFilled(bytes32 indexed commitment, address filler, TokenInfo[] outputs, TokenInfo[] inputs);

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
     * @dev Emitted when escrowed tokens are released to the solver after a successful (full or
     * partial) fill. For cross-chain partial fills this is the only source-chain signal of release.
     * @param commitment The order commitment hash.
     * @param solver The recipient of the released escrow.
     * @param tokens The tokens and amounts released.
     */
    event EscrowReleased(bytes32 indexed commitment, address solver, TokenInfo[] tokens);

    /**
     * @dev Emitted when escrowed tokens are refunded to the original user after cancellation.
     * @param commitment The order commitment hash.
     * @param tokens The tokens and amounts refunded.
     */
    event EscrowRefunded(bytes32 indexed commitment, TokenInfo[] tokens);

    /// @dev Protocol fee returned on cancellation, separate from principal in EscrowRefunded.
    event ProtocolFeeRefunded(bytes32 indexed commitment, address indexed token, uint256 amount);

    /**
     * @dev Emitted when an order's cancellation is initiated, on the chain it is initiated from.
     * For same-chain orders the refund is processed in the same transaction and `EscrowRefunded`
     * follows; for cross-chain orders `EscrowRefunded` follows on the source chain once the
     * cancellation has travelled through Hyperbridge.
     * @param commitment The order commitment hash.
     * @param canceller The account that initiated the cancellation. Destination-side cancellation
     * and expired same-chain cancellation are permissionless, so this may be a third party.
     */
    event OrderCancelled(bytes32 indexed commitment, address canceller);

    /**
     * @dev Emitted when the gateway's configuration parameters are updated via governance.
     * @param previous The previous parameter values.
     * @param current The new parameter values.
     */
    event ParamsUpdated(Params previous, Params current);

    /**
     * @dev Emitted when a new gateway instance is registered for a remote state machine.
     * @param chain The state machine identifier for the new deployment.
     * @param gateway The address of the deployed gateway on that chain.
     */
    event DeploymentAdded(string chain, address gateway);

    /**
     * @dev Emitted when surplus tokens are retained by the protocol. This includes
     * settled protocol fees, surplus shares from overpayment, and residual
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
     * @param chain The destination state machine identifier.
     * @param feeBps The protocol fee in basis points for orders targeting this destination.
     */
    event DestinationProtocolFeeUpdated(string chain, uint256 feeBps);

    /**
     * @dev Emitted when the authorised relayer is replaced.
     * @param previous The relayer that was authorised before this change.
     * @param current The relayer authorised from now on.
     */
    event RelayerUpdated(address previous, address current);

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
     * @notice The only relayer whose `onAccept` and `onGetResponse` deliveries are accepted, or
     * zero while the gate is open
     */
    function relayer() external view returns (address) {
        return _relayer;
    }

    /**
     * @dev Returns the current block number of the host chain. Order deadlines are
     * denominated in the block heights Hyperbridge tracks for each state machine —
     * for Arbitrum chains that is the L2 block number, but the `block.number` opcode
     * there returns the approximate L1 block number, so the ArbSys precompile is
     * queried instead. The chain id check keeps the bytecode identical across all
     * chains, preserving the deterministic CREATE2 deployment addresses.
     * @return The chain-appropriate current block number.
     */
    function _blockNumber() internal view returns (uint256) {
        if (block.chainid == ARBITRUM_ONE || block.chainid == ARBITRUM_NOVA || block.chainid == ARBITRUM_SEPOLIA) {
            return IArbSys(ARB_SYS).arbBlockNumber();
        }
        return block.number;
    }

    /**
     * @dev Resolves the IntentGateway instance address for a given state machine.
     * Reverts with `UnknownInstance` if no remote deployment has been registered for that chain.
     * @param stateMachineId The raw state machine identifier bytes.
     * @return The gateway address for the given state machine.
     */
    function _instance(bytes calldata stateMachineId) internal view returns (address) {
        address gateway = _instances[keccak256(stateMachineId)];
        if (gateway == address(0)) revert UnknownInstance();
        return gateway;
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

    /// @dev Native transfer that reverts with `InsufficientNativeToken` if refused.
    function _sendValue(address to, uint256 amount) internal {
        (bool sent,) = to.call{value: amount}("");
        if (!sent) revert InsufficientNativeToken();
    }

    /// @dev Splits overpayment between protocol and beneficiary. An order with output calldata
    /// gives the beneficiary nothing, since the surplus is not the caller's to give.
    function _splitSurplus(uint256 dust, bool hasOutputCall)
        internal
        view
        returns (uint256 protocolShare, uint256 beneficiaryShare)
    {
        if (hasOutputCall) return (dust, 0);
        protocolShare = (dust * _params.surplusShareBps) / 10_000;
        beneficiaryShare = dust - protocolShare;
    }

    /**
     * @dev Computes the storage slot hash for `_partialFills[commitment][index]` on a remote
     * chain. `_partialFills` is a nested mapping at slot 11, so the key is derived as
     * keccak256(index . keccak256(commitment . 11)) — the standard Solidity nested-mapping layout.
     * Used to construct GET storage-proof keys for cross-chain partial-fill cancel verification.
     * Keying by leg gives every leg its own proof key even when legs repeat an output token.
     * @param commitment The order commitment hash.
     * @param index The leg whose fill progress is being proven.
     * @return The ABI-encoded storage slot hash for the nested mapping entry.
     */
    function _calculatePartialFillSlotHash(bytes32 commitment, uint256 index) internal pure returns (bytes memory) {
        bytes32 innerSlot = keccak256(abi.encodePacked(commitment, PARTIAL_FILLS_SLOT_BIG_ENDIAN_BYTES));
        return abi.encodePacked(keccak256(abi.encodePacked(index, innerSlot)));
    }

    /**
     * @dev Computes the cumulative escrow released for an input token given how much of its
     * paired output has been filled. Defined as a single monotonic function so that the sum of
     * per-fill release deltas exactly equals `escrowTotal` once the output is fully filled, with
     * all integer-division rounding dust deterministically landing in the completing fill.
     *
     * Released(filled) = filled >= totalRequired ? escrowTotal : escrowTotal * filled / totalRequired
     *
     * This same function is used on the destination chain to size each `RedeemEscrow(Partial)`
     * message and on the source chain to size cancel refunds, guaranteeing that
     * (sum of redeems) + (cancel refund) == escrowTotal regardless of message arrival order.
     *
     * @param escrowTotal The full escrowed input amount for this token (order.inputs[i].amount).
     * @param filled The cumulative amount of the paired output filled so far.
     * @param totalRequired The total output amount required (order.output.assets[i].amount).
     * @return The cumulative escrow that should have been released to solvers at this fill level.
     */
    function _cumulativeReleased(uint256 escrowTotal, uint256 filled, uint256 totalRequired)
        internal
        pure
        returns (uint256)
    {
        if (totalRequired == 0 || filled >= totalRequired) return escrowTotal;
        return (escrowTotal * filled) / totalRequired;
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
     * `body.tokens[i]` is leg `i` of the order: every caller, and every gateway that posts a
     * `WithdrawalRequest`, lists one entry per leg in `order.inputs` order. The entry's token only
     * names what to transfer; the escrow drawn down is the leg's own, so legs that repeat a token
     * can never release each other's balance.
     *
     * @param body The withdrawal request containing the commitment, per-leg token amounts, and beneficiary.
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
            // A final redeem may carry zero principal after earlier slices were delivered.
            // Only finalize settles fees: fully-filled cancel proofs leave them for the solver redeem.
            uint256 refund = finalize ? _settleProtocolFee(body.commitment, i, token, isRefund ? amount : 0) : 0;
            if (amount > 0) {
                uint256 escrowed = _orders[body.commitment][i];
                if (escrowed == 0) revert UnknownOrder();
                _orders[body.commitment][i] = escrowed - amount;
            }

            uint256 transferAmount = amount + refund;
            if (transferAmount == 0) continue;
            if (token == address(0)) {
                _sendValue(beneficiary, transferAmount);
            } else {
                IERC20(token).safeTransfer(beneficiary, transferAmount);
            }
        }

        // Fees and the filled-marker are only settled on finalization; the release/refund event is
        // emitted for every withdrawal (including non-finalizing partial redeems and cancel refunds)
        // so escrow movement is always observable.
        if (finalize) {
            uint256 fees = _orders[body.commitment][TRANSACTION_FEES];
            if (fees > 0) {
                delete _orders[body.commitment][TRANSACTION_FEES];
                IERC20(IDispatcher(host()).feeToken()).safeTransfer(beneficiary, fees);
            }
        }

        if (isRefund) {
            emit EscrowRefunded({commitment: body.commitment, tokens: body.tokens});
        } else {
            emit EscrowReleased({commitment: body.commitment, solver: beneficiary, tokens: body.tokens});
        }
    }

    /// @dev Settles leg `index`'s held fee once, using authenticated refundable principal and the
    /// original commitment denominator. Floor rounding assigns the remaining fee unit to protocol
    /// revenue. `token` is the leg's input token, named in the emitted events.
    function _settleProtocolFee(bytes32 commitment, uint256 index, address token, uint256 principalRefund)
        internal
        returns (uint256 refund)
    {
        ProtocolFee memory fee = _protocolFees[commitment][index];
        if (fee.amount == 0) return 0;

        refund = Math.mulDiv(fee.amount, principalRefund, fee.committed);
        uint256 earned = fee.amount - refund;
        delete _protocolFees[commitment][index];

        if (refund > 0) emit ProtocolFeeRefunded(commitment, token, refund);
        if (earned > 0) emit DustCollected(token, earned);
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

            // Legs may repeat an output token. Sweep each token at its first leg only: a second
            // call for the same balance would fail the whole dispatch and report the dust twice.
            if (_isRepeatedToken(order.output.assets, i)) {
                unchecked {
                    ++i;
                }
                continue;
            }

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
     * @dev Whether `assets[i].token` already appears at a lower index.
     * @param assets The order's output assets.
     * @param i The leg to check.
     * @return True if an earlier leg carries the same token.
     */
    function _isRepeatedToken(TokenInfo[] calldata assets, uint256 i) internal pure returns (bool) {
        bytes32 token = assets[i].token;
        for (uint256 j; j < i;) {
            if (assets[j].token == token) return true;
            unchecked {
                ++j;
            }
        }
        return false;
    }

    /**
     * @dev Verifies an EIP-712 solver selection signature and stores a commitment to
     * `keccak256(abi.encode(solver, sessionKey))` in transient storage. The hash is
     * stored using `tstore` so it is only available within the same transaction —
     * this ensures atomicity between `select` and `fillOrder` calls.
     *
     * The session key is recovered from the EIP-712 signature over the (commitment, solver)
     * tuple. At fill time, `fillOrder` re-derives the same hash from `msg.sender` and
     * `order.session` and compares it against the value stored at the commitment slot.
     *
     * @param options The selection options containing the commitment, solver address, and signature.
     * @return The recovered session key address.
     */
    function _select(SelectOptions calldata options) internal returns (address) {
        bytes32 structHash = keccak256(abi.encode(SELECT_SOLVER_TYPEHASH, options.commitment, options.solver));
        bytes32 digest = _hashTypedDataV4(structHash);
        address sessionKey = ECDSA.recover(digest, options.signature);

        bytes32 commitment = options.commitment;
        bytes32 selectionHash = keccak256(abi.encode(options.solver, sessionKey));
        assembly {
            tstore(commitment, selectionHash)
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
    function _addDeployment(Deployment memory body) internal {
        _instances[keccak256(body.chain)] = body.gateway;
        emit DeploymentAdded({chain: string(body.chain), gateway: body.gateway});
    }

    /**
     * @dev The only writer of `_relayer`, behind `initialize` and `setRelayer`.
     */
    function _setRelayer(address relayer_) internal {
        emit RelayerUpdated({previous: _relayer, current: relayer_});
        _relayer = relayer_;
    }

    /**
     * @dev Validates gateway configuration parameters. Reverts with InvalidInput if any
     * value would brick the gateway or cause arithmetic errors in fee calculations.
     *
     * @param p The parameters to validate.
     */
    function _validateParams(Params memory p) internal view {
        if (p.host == address(0) || p.host.code.length == 0) revert InvalidInput();
        if (p.dispatcher == address(0) || p.dispatcher.code.length == 0) revert InvalidInput();
        if (p.surplusShareBps > 10_000) revert InvalidInput();
        if (p.protocolFeeBps >= 10_000) revert InvalidInput();
        if (p.priceOracle != address(0) && p.priceOracle.code.length == 0) revert InvalidInput();
    }

    /**
     * @dev Updates the gateway's configuration parameters and per-destination protocol fees.
     * Called by Hyperbridge governance to modify fee settings, host address, dispatcher,
     * price oracle, and other operational parameters.
     *
     * Validates all params before applying. Emits ParamsUpdated with the old and new params,
     * then iterates over any destination-specific fee overrides and applies them to
     * `_destinationProtocolFees`.
     *
     * @param update The parameter update containing new params and destination fee overrides.
     */
    function _updateParams(ParamsUpdate memory update) internal {
        _validateParams(update.params);

        emit ParamsUpdated({previous: _params, current: update.params});
        _params = update.params;

        for (uint256 i; i < update.destinationFees.length;) {
            bytes memory chain = update.destinationFees[i].chain;
            uint256 feeBps = update.destinationFees[i].destinationFeeBps;
            if (feeBps >= 10_000) revert InvalidInput();
            _destinationProtocolFees[keccak256(chain)] = feeBps;

            unchecked {
                ++i;
            }
            emit DestinationProtocolFeeUpdated(string(chain), feeBps);
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
                _sendValue(req.beneficiary, amount);
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
