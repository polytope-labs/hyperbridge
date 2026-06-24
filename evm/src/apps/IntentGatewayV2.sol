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

import {IntentsBase} from "./intentsv2/IntentsBase.sol";
import {IntrinsicIntents} from "./intentsv2/IntrinsicIntents.sol";
import {ExtrinsicIntents} from "./intentsv2/ExtrinsicIntents.sol";

import {ICallDispatcher, Call} from "@hyperbridge/core/interfaces/ICallDispatcher.sol";
import {IDispatcher} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {IIntentPriceOracle} from "@hyperbridge/core/apps/IntentPriceOracle.sol";
import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import {ReentrancyGuardTransient} from "@openzeppelin/contracts/utils/ReentrancyGuardTransient.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IUniswapV2Router02} from "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";
import {
    PaymentInfo,
    TokenInfo,
    DispatchInfo,
    Order,
    SweepDust,
    Params,
    ParamsUpdate,
    DestinationFee,
    WithdrawalRequest,
    FillOptions,
    SelectOptions,
    CancelOptions,
    Deployment
} from "@hyperbridge/core/apps/IntentGatewayV2.sol";

/**
 * @title IntentGatewayV2
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @dev The IntentGateway allows for the creation and fulfillment of same-chain & cross-chain orders.
 * This is the concrete entry-point contract that composes all intent logic via inheritance:
 *
 *            EIP712
 *              |
 *          IntentsBase
 *           /       \
 *  IntrinsicIntents  ExtrinsicIntents
 *           \       /
 *        IntentGatewayV2
 */
contract IntentGatewayV2 is IntrinsicIntents, ExtrinsicIntents, ReentrancyGuardTransient {
    using SafeERC20 for IERC20;

    /**
     * @dev Initializes the EIP-712 domain with name "IntentGateway" and version "2".
     * Sets the initial admin who has one-time authority to call `setParams`.
     * @param admin The address that will have permission to set initial parameters.
     */
    constructor(address admin) EIP712("IntentGateway", "2") {
        _admin = admin;
    }

    /**
     * @dev Allows the contract to receive native tokens (ETH/DOT/etc).
     * Required for escrow deposits with native tokens and for receiving
     * swept balances from the CallDispatcher.
     */
    receive() external payable {}

    /**
     * @dev Returns the Hyperbridge host contract address. Overrides both IntentsBase
     * and ExtrinsicIntents to resolve the diamond inheritance conflict at the
     * final concrete contract level.
     * @return The host contract address from stored params.
     */
    function host() public view override(IntentsBase, ExtrinsicIntents) returns (address) {
        return _params.host;
    }

    /**
     * @dev One-time parameter initialization. Can only be called by the admin set in
     * the constructor. After successful execution, the admin is burned (set to address(0)),
     * preventing any further calls.
     *
     * Subsequent parameter updates must come through Hyperbridge governance via the `onAccept` callback.
     *
     * @param p The initial gateway configuration parameters.
     * @param deployments The initial gateway cross-chain peers
     */
    function init(Params memory p, Deployment[] memory deployments) public {
        if (msg.sender != _admin) revert Unauthorized();

        uint256 deploymentsLength = deployments.length;
        for (uint256 i = 0; i < deploymentsLength; i++) {
            _addDeployment(deployments[i]);
        }
        _validateParams(p);
        _params = p;
        _admin = address(0);
    }

    /**
     * @dev Returns the current gateway configuration parameters.
     * @return The full Params struct containing host, dispatcher, fee settings, etc.
     */
    function params() external view returns (Params memory) {
        return _params;
    }

    /**
     * @dev Returns the registered gateway address for a given state machine.
     * Falls back to this contract's address if no remote deployment is registered.
     * @param stateMachineId The raw state machine identifier bytes.
     * @return The gateway address for the given state machine.
     */
    function instance(bytes calldata stateMachineId) public view returns (address) {
        return _instance(stateMachineId);
    }

    /**
     * @dev Computes the storage slot hash used for cross-chain cancel verification.
     * External callers (e.g., relayers) can use this to construct storage proof keys.
     * @param commitment The order commitment hash.
     * @return The ABI-encoded storage slot hash for the commitment in the `_filled` mapping.
     */
    function calculateCommitmentSlotHash(bytes32 commitment) public pure returns (bytes memory) {
        return _calculateCommitmentSlotHash(commitment);
    }

    /**
     * @dev Places a new intent order by escrowing the user's input tokens.
     *
     * The caller specifies the desired output tokens and destination chain. The function:
     * 1. Stamps the order with the caller's address, source chain, and a unique nonce.
     * 2. Deducts a protocol fee (in basis points) from each input amount. The commitment
     *    hash is computed over the fee-reduced inputs so solvers only need to match
     *    the post-fee amounts.
     * 3. If the order includes predispatch calldata, executes it via the CallDispatcher
     *    (e.g., unwrapping LP tokens) before escrowing the resulting balances.
     * 4. Otherwise, transfers input tokens directly from the caller into escrow.
     * 5. If the order includes solver fees, collects them in the protocol
     *    fee token — swapping from native token via Uniswap V2 if necessary.
     *
     * @param order The order struct. `user`, `source`, and `nonce` are overwritten by this function.
     * @param graffiti Unused on-chain; available for off-chain indexing or solver metadata.
     */
    function placeOrder(Order memory order, bytes32 graffiti) public payable nonReentrant {
        if (order.inputs.length == 0) revert InvalidInput();
        // Inputs and outputs pair 1:1 by index; reject mismatched orders that could never be filled.
        if (order.inputs.length != order.output.assets.length) revert InvalidInput();

        // Reject duplicate output tokens
        uint256 outputsLen_ = order.output.assets.length;
        for (uint256 i; i < outputsLen_;) {
            // A zero-amount output would strand its paired input escrow
            if (order.output.assets[i].amount == 0) revert InvalidInput();
            bytes32 token = order.output.assets[i].token;
            assembly ("memory-safe") {
                if tload(token) {
                    mstore(0, 0xb4fa3fb3) // InvalidInput.selector
                    revert(0x1c, 0x04)
                }
                tstore(token, 1)
            }
            unchecked {
                ++i;
            }
        }
        // Clean up transient storage so repeated placeOrder calls in the same tx don't false-positive.
        for (uint256 i; i < outputsLen_;) {
            bytes32 token = order.output.assets[i].token;
            assembly ("memory-safe") {
                tstore(token, 0)
            }
            unchecked {
                ++i;
            }
        }

        address hostAddr = host();
        order.user = bytes32(uint256(uint160(msg.sender)));
        order.source = IDispatcher(hostAddr).host();
        order.nonce = _nonce++;

        uint256 inputsLen = order.inputs.length;

        // Phase 1: Transfer tokens and record actual received amounts.
        // For fee-on-transfer tokens, the gateway receives less than the requested amount.
        // We mutate order.inputs to reflect actual received so the commitment and escrow
        // are consistent with what the gateway holds.
        uint256 msgValue = msg.value;
        if (order.predispatch.call.length > 0 && order.predispatch.assets.length > 0) {
            address dispatcher = _params.dispatcher;

            uint256 assetsLen = order.predispatch.assets.length;
            for (uint256 i; i < assetsLen;) {
                address token = address(uint160(uint256(order.predispatch.assets[i].token)));
                uint256 amount = order.predispatch.assets[i].amount;
                if (amount == 0) revert InvalidInput();

                if (token == address(0)) {
                    if (amount > msgValue) revert InsufficientNativeToken();
                    msgValue -= amount;

                    (bool sent,) = dispatcher.call{value: amount}("");
                    if (!sent) revert InsufficientNativeToken();
                } else {
                    IERC20(token).safeTransferFrom(msg.sender, dispatcher, amount);
                }

                unchecked {
                    ++i;
                }
            }

            ICallDispatcher(dispatcher).dispatch(order.predispatch.call);

            // Build sweep calls and snapshot gateway balances before the sweep.
            Call[] memory transferCalls = new Call[](inputsLen);
            uint256[] memory balancesBefore = new uint256[](inputsLen);
            for (uint256 i; i < inputsLen;) {
                if (order.inputs[i].amount == 0) revert InvalidInput();
                address token = address(uint160(uint256(order.inputs[i].token)));
                uint256 requiredAmount = order.inputs[i].amount;

                if (token == address(0)) {
                    uint256 balance = address(dispatcher).balance;
                    if (balance < requiredAmount) revert InsufficientNativeToken();
                    transferCalls[i] = Call({to: address(this), value: balance, data: ""});
                    balancesBefore[i] = address(this).balance;
                } else {
                    uint256 balance = IERC20(token).balanceOf(dispatcher);
                    if (balance < requiredAmount) revert InvalidInput();
                    transferCalls[i] = Call({
                        to: token,
                        value: 0,
                        data: abi.encodeWithSelector(IERC20.transfer.selector, address(this), balance)
                    });
                    balancesBefore[i] = IERC20(token).balanceOf(address(this));
                }

                unchecked {
                    ++i;
                }
            }

            ICallDispatcher(dispatcher).dispatch(abi.encode(transferCalls));

            // Measure actual received, emit dust for excess, update order.inputs.
            for (uint256 i; i < inputsLen;) {
                address token = address(uint160(uint256(order.inputs[i].token)));
                uint256 received;
                if (token == address(0)) {
                    received = address(this).balance - balancesBefore[i];
                } else {
                    received = IERC20(token).balanceOf(address(this)) - balancesBefore[i];
                }

                if (received > order.inputs[i].amount) {
                    uint256 dust = received - order.inputs[i].amount;
                    emit DustCollected(token, dust);
                } else {
                    order.inputs[i].amount = received;
                }

                unchecked {
                    ++i;
                }
            }
        } else {
            for (uint256 i; i < inputsLen;) {
                if (order.inputs[i].amount == 0) revert InvalidInput();
                address token = address(uint160(uint256(order.inputs[i].token)));
                if (token == address(0)) {
                    if (msgValue < order.inputs[i].amount) revert InsufficientNativeToken();
                    msgValue -= order.inputs[i].amount;
                } else {
                    uint256 balBefore = IERC20(token).balanceOf(address(this));
                    IERC20(token).safeTransferFrom(msg.sender, address(this), order.inputs[i].amount);
                    order.inputs[i].amount = IERC20(token).balanceOf(address(this)) - balBefore;
                }

                unchecked {
                    ++i;
                }
            }
        }

        // Phase 2: Compute protocol fees and commitment from actual received amounts.
        bytes32 destinationHash = keccak256(order.destination);
        uint256 protocolFeeBps = _destinationProtocolFees[destinationHash];
        if (protocolFeeBps == 0) {
            protocolFeeBps = _params.protocolFeeBps;
        }
        TokenInfo[] memory reducedInputs;
        bytes32 commitment;

        if (protocolFeeBps > 0) {
            reducedInputs = new TokenInfo[](inputsLen);
            for (uint256 i; i < inputsLen;) {
                uint256 originalAmount = order.inputs[i].amount;
                if (originalAmount == 0) revert InvalidInput();
                uint256 protocolFee = (originalAmount * protocolFeeBps) / 10_000;
                uint256 reducedAmount = originalAmount - protocolFee;
                address token = address(uint160(uint256(order.inputs[i].token)));

                if (protocolFee > 0) emit DustCollected(token, protocolFee);

                reducedInputs[i] = TokenInfo({token: order.inputs[i].token, amount: reducedAmount});
                unchecked {
                    ++i;
                }
            }

            order.inputs = reducedInputs;
            commitment = keccak256(abi.encode(order));
        } else {
            reducedInputs = order.inputs;
            commitment = keccak256(abi.encode(order));
        }

        // Phase 3: Credit escrow.
        for (uint256 i; i < inputsLen;) {
            address token = address(uint160(uint256(order.inputs[i].token)));
            // Reject duplicate input tokens
            if (_orders[commitment][token] != 0) revert InvalidInput();
            _orders[commitment][token] = reducedInputs[i].amount;

            unchecked {
                ++i;
            }
        }

        if (order.fees > 0) {
            address feeToken = IDispatcher(hostAddr).feeToken();
            if (msgValue > 0) {
                address uniswapV2 = IDispatcher(hostAddr).uniswapV2Router();
                address WETH = IUniswapV2Router02(uniswapV2).WETH();
                address[] memory path = new address[](2);
                path[0] = WETH;
                path[1] = IDispatcher(hostAddr).feeToken();
                uint256[] memory amounts = IUniswapV2Router02(uniswapV2).swapETHForExactTokens{value: msgValue}(
                    order.fees, path, address(this), block.timestamp
                );
                msgValue -= amounts[0];
            } else {
                IERC20(feeToken).safeTransferFrom(msg.sender, address(this), order.fees);
            }

            _orders[commitment][TRANSACTION_FEES] = order.fees;
        }

        // Refund any unspent native tokens to the user.
        if (msgValue > 0) {
            (bool sent,) = msg.sender.call{value: msgValue}("");
            if (!sent) revert InsufficientNativeToken();
        }

        emit OrderPlaced({
            user: order.user,
            source: string(order.source),
            destination: string(order.destination),
            deadline: order.deadline,
            nonce: order.nonce,
            fees: order.fees,
            session: order.session,
            predispatch: order.predispatch.assets,
            inputs: reducedInputs,
            beneficiary: order.output.beneficiary,
            outputs: order.output.assets
        });
    }

    /**
     * @dev Verifies and stores a solver selection for a given order commitment. Must be
     * called in the same transaction as `fillOrder` when solver selection is enabled.
     * Uses transient storage to atomically bind the solver to the commitment.
     * @param options The selection options containing commitment, solver address, and EIP-712 signature.
     * @return The recovered session key address from the signature.
     */
    function select(SelectOptions calldata options) public returns (address) {
        return _select(options);
    }

    /**
     * @dev Fills an existing order by providing the requested output tokens. Routes to
     * either same-chain or cross-chain fill logic based on the order's source and
     * destination chains.
     *
     * Shared validation performed before routing:
     * 1. Checks the order has not expired (deadline >= current block).
     * 2. Verifies the order has not already been filled.
     * 3. If solver selection is enabled, validates the caller matches the selected
     *    solver stored in transient storage (set by a prior `select` call).
     * 4. Validates input/output array length consistency.
     *
     * After fill completion, records the price spread with the oracle if configured.
     *
     * @param order The order to fill. Must match the exact order that was placed.
     * @param options Fill options including output token amounts and fee parameters.
     */
    function fillOrder(Order calldata order, FillOptions calldata options) public payable nonReentrant {
        if (order.deadline < block.number) revert Expired();
        bytes32 commitment = keccak256(abi.encode(order));

        address hostAddr = host();
        bytes32 currentChain = keccak256(IDispatcher(hostAddr).host());
        bytes32 orderSource = keccak256(order.source);
        bytes32 orderDest = keccak256(order.destination);
        bool isSameChain = orderSource == orderDest;

        if (isSameChain && orderSource != currentChain) revert WrongChain();
        if (!isSameChain && orderDest != currentChain) revert WrongChain();

        if (_filled[commitment] != address(0)) revert Filled();

        if (_params.solverSelection) {
            bytes32 storedSelectionHash;
            assembly {
                storedSelectionHash := tload(commitment)
            }

            bytes32 expectedSelectionHash = keccak256(abi.encode(msg.sender, order.session));
            if (storedSelectionHash != expectedSelectionHash) revert Unauthorized();
        }

        uint256 outputsLen = order.output.assets.length;
        if (options.outputs.length != outputsLen) revert InvalidInput();
        if (order.inputs.length != outputsLen) revert InvalidInput();

        if (isSameChain) {
            _fillSameChain(order, options, commitment);
        } else {
            _fillCrossChain(order, options, commitment);
        }

        if (_params.priceOracle != address(0)) {
            IIntentPriceOracle(_params.priceOracle)
                .recordSpread(commitment, order.source, order.inputs, options.outputs);
        }
    }

    /**
     * @dev Cancels an existing order and initiates the refund of escrowed tokens.
     * Routes to the appropriate cancellation logic based on the order type and
     * the current chain:
     *
     * - Same-chain orders: Refunds escrow directly on this chain.
     * - Cross-chain, called from source: Dispatches a Hyperbridge GET request to
     *   verify the order was not filled on the destination chain.
     * - Cross-chain, called from destination: Marks the order as filled (preventing
     *   future fills) and dispatches a RefundEscrow message to the source chain.
     *
     * Reverts if the order has already been filled or if called from the wrong chain.
     *
     * @param order The order to cancel. Must match the exact order that was placed.
     * @param options Cancel options including proof height and relayer fee for cross-chain cancels.
     */
    function cancelOrder(Order calldata order, CancelOptions calldata options) public payable nonReentrant {
        bytes32 commitment = keccak256(abi.encode(order));

        if (_filled[commitment] != address(0)) revert Filled();

        address hostAddr = host();
        bytes32 currentChain = keccak256(IDispatcher(hostAddr).host());
        bytes32 orderSource = keccak256(order.source);
        bytes32 orderDest = keccak256(order.destination);
        bool isSameChain = orderSource == orderDest;

        if (isSameChain) {
            _cancelSameChain(order, commitment);
        } else if (currentChain == orderSource) {
            _cancelFromSource(order, options, commitment);
        } else if (currentChain == orderDest) {
            _cancelFromDest(order, options, commitment);
        } else {
            revert WrongChain();
        }
    }
}
