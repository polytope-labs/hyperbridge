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
import {ICallDispatcher, Call} from "../interfaces/ICallDispatcher.sol";

import {IDispatcher} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {IIntentPriceOracle} from "@hyperbridge/core/apps/IntentPriceOracle.sol";
import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
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
    NewDeployment
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
contract IntentGatewayV2 is IntrinsicIntents, ExtrinsicIntents {
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
     */
    function setParams(Params memory p) public {
        if (msg.sender != _admin) revert Unauthorized();

        _admin = address(0);
        _params = p;
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
    function placeOrder(Order memory order, bytes32 graffiti) public payable {
        if (order.inputs.length == 0) revert InvalidInput();

        address hostAddr = host();
        order.user = bytes32(uint256(uint160(msg.sender)));
        order.source = IDispatcher(hostAddr).host();
        order.nonce = _nonce++;

        uint256 inputsLen = order.inputs.length;
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

            TokenInfo[] memory originalInputs = order.inputs;
            order.inputs = reducedInputs;
            commitment = keccak256(abi.encode(order));
            order.inputs = originalInputs;
        } else {
            reducedInputs = order.inputs;
            commitment = keccak256(abi.encode(order));
        }

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

            Call[] memory transferCalls = new Call[](inputsLen);
            for (uint256 i; i < inputsLen;) {
                if (order.inputs[i].amount == 0) revert InvalidInput();
                address token = address(uint160(uint256(order.inputs[i].token)));
                uint256 requiredAmount = order.inputs[i].amount;
                uint256 balance;

                if (token == address(0)) {
                    balance = address(dispatcher).balance;
                    if (balance < requiredAmount) revert InsufficientNativeToken();
                    transferCalls[i] = Call({to: address(this), value: balance, data: ""});
                } else {
                    balance = IERC20(token).balanceOf(dispatcher);
                    if (balance < requiredAmount) revert InvalidInput();
                    transferCalls[i] = Call({
                        to: token,
                        value: 0,
                        data: abi.encodeWithSelector(IERC20.transfer.selector, address(this), balance)
                    });
                }

                uint256 dust = balance - requiredAmount;
                if (dust > 0) emit DustCollected(token, dust);

                _orders[commitment][token] += reducedInputs[i].amount;

                unchecked {
                    ++i;
                }
            }

            ICallDispatcher(dispatcher).dispatch(abi.encode(transferCalls));
        } else {
            for (uint256 i; i < inputsLen;) {
                if (order.inputs[i].amount == 0) revert InvalidInput();
                address token = address(uint160(uint256(order.inputs[i].token)));
                if (token == address(0)) {
                    if (msgValue < order.inputs[i].amount) revert InsufficientNativeToken();
                    msgValue -= order.inputs[i].amount;
                } else {
                    IERC20(token).safeTransferFrom(msg.sender, address(this), order.inputs[i].amount);
                }

                _orders[commitment][token] += reducedInputs[i].amount;

                unchecked {
                    ++i;
                }
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
                IUniswapV2Router02(uniswapV2).swapETHForExactTokens{value: msgValue}(
                    order.fees, path, address(this), block.timestamp
                );
            } else {
                IERC20(feeToken).safeTransferFrom(msg.sender, address(this), order.fees);
            }

            _orders[commitment][TRANSACTION_FEES] = order.fees;
        }

        emit OrderPlaced({
            user: order.user,
            source: order.source,
            destination: order.destination,
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
    function fillOrder(Order calldata order, FillOptions calldata options) public payable {
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
            bytes32 solver;
            bytes32 storedSessionKey;
            bytes32 sessionSlot = bytes32(uint256(commitment) + 1);
            assembly {
                solver := tload(commitment)
                storedSessionKey := tload(sessionSlot)
            }

            if (address(uint160(uint256(solver))) != msg.sender) revert Unauthorized();
            if (address(uint160(uint256(storedSessionKey))) != order.session) revert Unauthorized();
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
    function cancelOrder(Order calldata order, CancelOptions calldata options) public payable {
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
