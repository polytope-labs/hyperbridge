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

import {IntentsBase} from "./IntentsBase.sol";
import {HyperApp} from "@hyperbridge/core/apps/HyperApp.sol";
import {IncomingPostRequest, IncomingGetResponse} from "@hyperbridge/core/interfaces/IApp.sol";
import {DispatchPost, DispatchGet, PostRequest, IDispatcher} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {
    TokenInfo,
    Order,
    Params,
    ParamsUpdate,
    SweepDust,
    WithdrawalRequest,
    FillOptions,
    CancelOptions,
    NewDeployment
} from "@hyperbridge/core/apps/IntentGatewayV2.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {SafeCast} from "@openzeppelin/contracts/utils/math/SafeCast.sol";

/**
 * @title ExtrinsicIntents
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @dev Cross-chain intent logic & HyperApp callback handlers (onAccept, onGetResponse).
 */
abstract contract ExtrinsicIntents is IntentsBase, HyperApp {
    using SafeERC20 for IERC20;

    /**
     * @dev Returns the Hyperbridge host contract address. Overrides both IntentsBase and
     * HyperApp to resolve the diamond inheritance conflict — both parent contracts
     * declare a virtual `host()` function.
     * @return The host contract address from stored params.
     */
    function host() public view virtual override(IntentsBase, HyperApp) returns (address) {
        return _params.host;
    }

    /**
     * @dev Authenticates an incoming cross-chain post request by verifying that the
     * sender module matches the registered gateway instance for the source chain.
     * Reverts with InvalidInput if the sender address is malformed, or Unauthorized
     * if the sender is not the expected gateway.
     * @param request The incoming post request to authenticate.
     */
    function _authenticate(PostRequest calldata request) internal view {
        if (request.from.length != 20) revert InvalidInput();
        address module = address(bytes20(request.from));
        if (_instance(request.source) != module) revert Unauthorized();
    }

    /**
     * @dev Fills a cross-chain order on the destination chain. The solver provides output
     * tokens directly to the beneficiary, and a Hyperbridge post request is dispatched
     * back to the source chain to release the escrowed input tokens to the solver.
     *
     * Unlike same-chain fills, cross-chain fills are all-or-nothing — partial fills
     * are not supported. The solver must provide at least the full required amount
     * for every output asset.
     *
     * Surplus handling (when solver overpays):
     * - If the order has attached calldata, all surplus goes to the protocol.
     * - Otherwise, surplus is split between beneficiary and protocol per `surplusShareBps`.
     *
     * After transferring tokens and executing any attached calldata, dispatches a
     * RedeemEscrow message to the source chain gateway via Hyperbridge.
     *
     * @param order The cross-chain order to fill.
     * @param options Fill options including output amounts, relayer fee, and native dispatch fee.
     * @param commitment The keccak256 hash of the ABI-encoded order.
     */
    function _fillCrossChain(Order calldata order, FillOptions calldata options, bytes32 commitment) internal {
        uint256 outputsLen = order.output.assets.length;

        _filled[commitment] = msg.sender;

        uint256 msgValue = msg.value;
        address beneficiary = address(uint160(uint256(order.output.beneficiary)));

        for (uint256 i; i < outputsLen; i++) {
            bytes32 outputToken = order.output.assets[i].token;
            if (options.outputs[i].token != outputToken) revert InvalidInput();

            address token = address(uint160(uint256(outputToken)));
            uint256 totalRequired = order.output.assets[i].amount;
            uint256 solverAmount = options.outputs[i].amount;

            if (solverAmount < totalRequired) revert InvalidInput();

            uint256 dust = solverAmount - totalRequired;
            uint256 beneficiaryShare = 0;
            uint256 protocolShare = 0;

            if (dust > 0) {
                if (order.output.call.length > 0) {
                    protocolShare = dust;
                } else {
                    protocolShare = (dust * _params.surplusShareBps) / 10_000;
                    beneficiaryShare = dust - protocolShare;
                }
            }

            if (token == address(0)) {
                if (msgValue < solverAmount) revert InsufficientNativeToken();
                uint256 beneficiaryTotal = totalRequired + beneficiaryShare;
                (bool sent,) = beneficiary.call{value: beneficiaryTotal}("");
                if (!sent) revert InsufficientNativeToken();
                msgValue -= (beneficiaryTotal + protocolShare);
            } else {
                IERC20(token).safeTransferFrom(msg.sender, beneficiary, totalRequired + beneficiaryShare);
                if (protocolShare > 0) {
                    IERC20(token).safeTransferFrom(msg.sender, address(this), protocolShare);
                }
            }
            if (protocolShare > 0) emit DustCollected(token, protocolShare);
        }

        _execute(order, outputsLen);

        address hostAddr = host();
        bytes memory body = bytes.concat(
            bytes1(uint8(RequestKind.RedeemEscrow)),
            abi.encode(
                WithdrawalRequest({
                    commitment: commitment, tokens: order.inputs, beneficiary: bytes32(uint256(uint160(msg.sender)))
                })
            )
        );
        DispatchPost memory request = DispatchPost({
            dest: order.source,
            to: abi.encodePacked(_instance(order.source)),
            body: body,
            timeout: 0,
            fee: options.relayerFee,
            payer: msg.sender
        });

        if (options.nativeDispatchFee > 0 && msgValue >= options.nativeDispatchFee) {
            IDispatcher(hostAddr).dispatch{value: options.nativeDispatchFee}(request);
        } else {
            dispatchWithFeeToken(request, msg.sender);
        }

        emit OrderFilled({commitment: commitment, filler: msg.sender});
    }

    /**
     * @dev Initiates cancellation of a cross-chain order from the source chain.
     *
     * Only the order creator may cancel, and only after the order deadline has passed
     * (verified by `options.height > order.deadline`). Dispatches a Hyperbridge GET
     * request to the destination chain to verify that the `_filled` storage slot for
     * this commitment is empty (i.e., the order was never filled on the destination).
     *
     * The GET response is handled by `onGetResponse`, which refunds the escrow if
     * the slot is indeed empty.
     *
     * @param order The order to cancel.
     * @param options Cancel options including the proof height and relayer fee.
     * @param commitment The keccak256 hash of the ABI-encoded order.
     */
    function _cancelFromSource(Order calldata order, CancelOptions calldata options, bytes32 commitment) internal {
        if (order.user != bytes32(uint256(uint160(msg.sender)))) revert Unauthorized();

        if (options.height <= order.deadline) revert NotExpired();

        uint256 inputsLen = order.inputs.length;
        for (uint256 i; i < inputsLen;) {
            if (_orders[commitment][address(uint160(uint256(order.inputs[i].token)))] == 0) revert UnknownOrder();

            unchecked {
                ++i;
            }
        }

        bytes memory context =
            abi.encode(WithdrawalRequest({commitment: commitment, tokens: order.inputs, beneficiary: order.user}));

        bytes[] memory keys = new bytes[](1);
        keys[0] = bytes.concat(abi.encodePacked(_instance(order.destination)), _calculateCommitmentSlotHash(commitment));
        DispatchGet memory request = DispatchGet({
            dest: order.destination,
            keys: keys,
            timeout: 0,
            height: SafeCast.toUint64(options.height),
            fee: options.relayerFee,
            context: context
        });

        address hostAddr = host();
        if (msg.value > 0) {
            IDispatcher(hostAddr).dispatch{value: msg.value}(request);
        } else {
            dispatchWithFeeToken(request, msg.sender);
        }
    }

    /**
     * @dev Initiates cancellation of a cross-chain order from the destination chain.
     *
     * If the order deadline has not yet passed, only the order creator may cancel.
     * After the deadline, anyone may trigger the cancellation (e.g., a relayer acting
     * on behalf of the user).
     *
     * Marks the order as filled (to prevent future fill attempts) and dispatches a
     * RefundEscrow message via Hyperbridge to the source chain to release the escrowed
     * tokens back to the original user.
     *
     * @param order The order to cancel.
     * @param options Cancel options including the relayer fee.
     * @param commitment The keccak256 hash of the ABI-encoded order.
     */
    function _cancelFromDest(Order calldata order, CancelOptions calldata options, bytes32 commitment) internal {
        if (order.deadline >= block.number) {
            if (order.user != bytes32(uint256(uint160(msg.sender)))) revert Unauthorized();
        }

        _filled[commitment] = address(uint160(uint256(order.user)));

        bytes memory body = bytes.concat(
            bytes1(uint8(RequestKind.RefundEscrow)),
            abi.encode(WithdrawalRequest({commitment: commitment, tokens: order.inputs, beneficiary: order.user}))
        );

        DispatchPost memory request = DispatchPost({
            dest: order.source,
            to: abi.encodePacked(_instance(order.source)),
            body: body,
            timeout: 0,
            fee: options.relayerFee,
            payer: msg.sender
        });

        address hostAddr = host();
        if (msg.value > 0) {
            IDispatcher(hostAddr).dispatch{value: msg.value}(request);
        } else {
            dispatchWithFeeToken(request, msg.sender);
        }
    }

    /**
     * @dev Handles incoming cross-chain post requests dispatched via Hyperbridge.
     * The first byte of the request body encodes the `RequestKind`, which determines
     * the action to take:
     *
     * - RedeemEscrow: Releases escrowed tokens to the solver who filled the order
     *   on the destination chain. Authenticated against the registered gateway instance.
     * - RefundEscrow: Refunds escrowed tokens to the original user after a successful
     *   cancellation from the destination chain. Authenticated against the registered gateway.
     * - NewDeployment: Registers a new gateway instance for a state machine. Only
     *   Hyperbridge itself may dispatch this request.
     * - UpdateParams: Updates the gateway's configuration parameters and per-destination
     *   protocol fees. Only Hyperbridge may dispatch this request.
     * - SweepDust: Transfers accumulated protocol dust to a specified beneficiary.
     *   Only Hyperbridge may dispatch this request.
     *
     * @param incoming The incoming post request from Hyperbridge.
     */
    function onAccept(IncomingPostRequest calldata incoming) external override onlyHost {
        RequestKind kind = RequestKind(uint8(incoming.request.body[0]));
        if (kind == RequestKind.RedeemEscrow || kind == RequestKind.RefundEscrow) {
            _authenticate(incoming.request);
            WithdrawalRequest memory body = abi.decode(incoming.request.body[1:], (WithdrawalRequest));
            return _withdraw(body, kind == RequestKind.RefundEscrow, true);
        }

        // only hyperbridge is permitted to perform these actions
        if (keccak256(incoming.request.source) != keccak256(IDispatcher(host()).hyperbridge())) revert Unauthorized();
        if (kind == RequestKind.NewDeployment) {
            _addDeployment(abi.decode(incoming.request.body[1:], (NewDeployment)));
        } else if (kind == RequestKind.UpdateParams) {
            _updateParams(abi.decode(incoming.request.body[1:], (ParamsUpdate)));
        } else if (kind == RequestKind.SweepDust) {
            _sweepDust(abi.decode(incoming.request.body[1:], (SweepDust)));
        }
    }

    /**
     * @dev Handles the response to a Hyperbridge GET request dispatched during
     * `_cancelFromSource`. Verifies that the `_filled` storage slot on the destination
     * chain is empty (meaning the order was never filled), then refunds the escrowed
     * tokens to the original user. Reverts with `Filled` if the slot is non-empty.
     *
     * @param incoming The incoming GET response from Hyperbridge containing the storage proof.
     */
    function onGetResponse(IncomingGetResponse calldata incoming) external override onlyHost {
        if (incoming.response.values[0].value.length != 0) revert Filled();

        WithdrawalRequest memory body = abi.decode(incoming.response.request.context, (WithdrawalRequest));
        _withdraw(body, true, true);
    }
}
