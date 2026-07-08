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
    Deployment
} from "@hyperbridge/core/apps/IntentGatewayV2.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {RLPReader} from "@polytope-labs/solidity-merkle-trees/src/trie/ethereum/RLPReader.sol";
import {ERC1967Utils} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Utils.sol";


/**
 * @title ExtrinsicIntents
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @dev Cross-chain intent logic & HyperApp callback handlers (onAccept, onGetResponse).
 */
abstract contract ExtrinsicIntents is IntentsBase, HyperApp {
    using SafeERC20 for IERC20;
    using RLPReader for bytes;
    using RLPReader for RLPReader.RLPItem;

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
     * @dev Fills a cross-chain order on the destination chain, supporting both partial and full
     * fills. The solver provides output tokens directly to the beneficiary, and a Hyperbridge post
     * request is dispatched back to the source chain to release the escrowed input tokens.
     *
     * Partial-fill tracking mirrors the same-chain path: cumulative progress per output token is
     * recorded in `_partialFills`, and the escrow released for each fill is computed via
     * `_cumulativeReleased` over `order.inputs[i].amount`. Because the escrow itself lives on the
     * source chain, the proportional slice is carried in the dispatched message rather than
     * released locally. The monotonic release function guarantees that, across any number of
     * partial fills, the redeemed slices sum to exactly the escrowed amount.
     *
     * - Partial fill: clears `_filled` (so the next solver can continue) and dispatches a
     *   `RedeemEscrowPartial` message (non-finalizing on the source). Emits `PartialFill`.
     * - Full fill: keeps `_filled` set, executes any attached calldata, and dispatches a
     *   `RedeemEscrow` message (finalizing, forwarding accumulated fees). Emits `OrderFilled`.
     *
     * Surplus handling (only when a solver overpays on a fresh, unfilled output):
     * - If the order has attached calldata, all surplus goes to the protocol.
     * - Otherwise, surplus is split between beneficiary and protocol per `surplusShareBps`.
     *
     * Orders carrying output calldata cannot be partially filled — the attached call only runs on
     * a full fill, so an incomplete fill reverts with PartialFillNotAllowed.
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
        bool isFullyFilled = true;

        TokenInfo[] memory escrowReleases = new TokenInfo[](outputsLen);
        TokenInfo[] memory outputFills = new TokenInfo[](outputsLen);

        for (uint256 i; i < outputsLen; i++) {
            bytes32 outputToken = order.output.assets[i].token;
            if (options.outputs[i].token != outputToken) revert InvalidInput();

            address token = address(uint160(uint256(outputToken)));
            uint256 totalRequired = order.output.assets[i].amount;
            uint256 solverAmount = options.outputs[i].amount;

            uint256 alreadyFilled = _partialFills[commitment][outputToken];
            uint256 remaining = totalRequired - alreadyFilled;
            if (remaining == 0 || solverAmount == 0) {
                if (solverAmount == 0 && remaining > 0) isFullyFilled = false;
                // Record the real tokens (with zero amounts) so emitted events carry token identity.
                escrowReleases[i] = TokenInfo({token: order.inputs[i].token, amount: 0});
                outputFills[i] = TokenInfo({token: outputToken, amount: 0});
                continue;
            }
            uint256 fillAmount;

            uint256 beneficiaryShare = 0;
            uint256 protocolShare = 0;
            if (alreadyFilled == 0 && solverAmount > totalRequired) {
                fillAmount = totalRequired;
                uint256 dust = solverAmount - totalRequired;
                if (order.output.call.length > 0) {
                    protocolShare = dust;
                } else {
                    protocolShare = (dust * _params.surplusShareBps) / 10_000;
                    beneficiaryShare = dust - protocolShare;
                }
            } else {
                fillAmount = solverAmount > remaining ? remaining : solverAmount;
            }

            uint256 amountFilled = alreadyFilled + fillAmount;
            _partialFills[commitment][outputToken] = amountFilled;
            uint256 beneficiaryTotal = fillAmount + beneficiaryShare;

            if (token == address(0)) {
                if (msgValue < beneficiaryTotal + protocolShare) revert InsufficientNativeToken();
                msgValue -= (beneficiaryTotal + protocolShare);
                (bool sent,) = beneficiary.call{value: beneficiaryTotal}("");
                if (!sent) revert InsufficientNativeToken();
            } else {
                IERC20(token).safeTransferFrom(msg.sender, beneficiary, beneficiaryTotal);
                if (protocolShare > 0) {
                    IERC20(token).safeTransferFrom(msg.sender, address(this), protocolShare);
                }
            }

            if (totalRequired > amountFilled) isFullyFilled = false;
            if (protocolShare > 0) emit DustCollected(token, protocolShare);

            // Escrow lives on the source chain; carry this fill's proportional slice in the message.
            uint256 escrowTotal = order.inputs[i].amount;
            uint256 releaseNow = _cumulativeReleased(escrowTotal, amountFilled, totalRequired)
                - _cumulativeReleased(escrowTotal, alreadyFilled, totalRequired);
            escrowReleases[i] = TokenInfo({token: order.inputs[i].token, amount: releaseNow});
            outputFills[i] = TokenInfo({token: outputToken, amount: fillAmount});
        }

        // Orders with output calldata can't be partially filled; the call only runs on a full fill.
        if (order.output.call.length > 0 && !isFullyFilled) revert PartialFillNotAllowed();

        if (isFullyFilled) {
            _execute(order, outputsLen);
        } else {
            // Clear the optimistic claim so the next solver can fill the remainder.
            delete _filled[commitment];
        }

        address hostAddr = host();
        bytes memory body = bytes.concat(
            bytes1(uint8(isFullyFilled ? RequestKind.RedeemEscrow : RequestKind.RedeemEscrowPartial)),
            abi.encode(
                WithdrawalRequest({
                    commitment: commitment, tokens: escrowReleases, beneficiary: bytes32(uint256(uint160(msg.sender)))
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
            msgValue -= options.nativeDispatchFee;
        } else {
            dispatchWithFeeToken(request);
        }

        // Refund any unspent native tokens to the solver.
        if (msgValue > 0) {
            (bool sent,) = msg.sender.call{value: msgValue}("");
            if (!sent) revert InsufficientNativeToken();
        }

        if (isFullyFilled) {
            emit OrderFilled({commitment: commitment, filler: msg.sender, outputs: outputFills, inputs: escrowReleases});
        } else {
            emit PartialFill({commitment: commitment, filler: msg.sender, outputs: outputFills, inputs: escrowReleases});
        }
    }

    /**
     * @dev Initiates cancellation of a cross-chain order from the source chain.
     *
     * Only the order creator may cancel, and only after the order deadline has passed
     * (verified by `options.height > order.deadline`). The deadline gate is what makes a
     * proof at `options.height` a *final* snapshot of fill progress: once `block.number`
     * passes the deadline no further fills can occur on the destination, so the proven
     * `_partialFills` values can no longer change.
     *
     * Dispatches a Hyperbridge GET request reading the destination's
     * `_partialFills[commitment][token]` slot for each output token. The response is handled by
     * `onGetResponse`, which refunds the proven-unredeemed fraction of each escrowed input — never
     * the raw remaining escrow, so that any `RedeemEscrow` messages still in flight for fills that
     * happened before the deadline remain covered.
     *
     * `placeOrder` guarantees `order.inputs.length == order.output.assets.length`, so each input is
     * paired with the output at the same index.
     *
     * @param order The order to cancel.
     * @param options Cancel options including the proof height and relayer fee.
     * @param commitment The keccak256 hash of the ABI-encoded order.
     */
    function _cancelFromSource(Order calldata order, CancelOptions calldata options, bytes32 commitment) internal {
        if (order.user != bytes32(uint256(uint160(msg.sender)))) revert Unauthorized();

        if (options.height <= order.deadline) revert NotExpired();

        uint256 inputsLen = order.inputs.length;
        address destGateway = _instance(order.destination);

        bytes[] memory keys = new bytes[](inputsLen);
        uint256[] memory totalRequired = new uint256[](inputsLen);
        for (uint256 i; i < inputsLen;) {
            keys[i] = bytes.concat(
                abi.encodePacked(destGateway),
                _calculatePartialFillSlotHash(commitment, order.output.assets[i].token)
            );
            totalRequired[i] = order.output.assets[i].amount;
            unchecked {
                ++i;
            }
        }
        bytes memory context = abi.encode(commitment, order.user, order.inputs, totalRequired);

        DispatchGet memory request = DispatchGet({
            dest: order.destination,
            keys: keys,
            timeout: 0,
            height: options.height,
            fee: options.relayerFee,
            context: context,
            payer: msg.sender
        });

        address hostAddr = host();
        if (msg.value > 0) {
            IDispatcher(hostAddr).dispatch{value: msg.value}(request);
        } else {
            dispatchWithFeeToken(request);
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
     * RefundEscrow message via Hyperbridge to the source chain. Because this runs on the
     * destination, `_partialFills` is read directly: only the unredeemed fraction of each escrowed
     * input is refunded, leaving the portion already (or about to be) redeemed by partial-fill
     * solvers untouched. Setting `_filled` and snapshotting `_partialFills` happen in the same
     * transaction, so the snapshot is final without needing a deadline gate.
     *
     * @param order The order to cancel.
     * @param options Cancel options including the relayer fee.
     * @param commitment The keccak256 hash of the ABI-encoded order.
     */
    function _cancelFromDest(Order calldata order, CancelOptions calldata options, bytes32 commitment) internal {
        if (order.deadline >= block.number) {
            if (order.user != bytes32(uint256(uint160(msg.sender)))) revert Unauthorized();
        }

        // Freeze the order, then snapshot fill progress in the same tx and refund the unredeemed rest.
        _filled[commitment] = address(uint160(uint256(order.user)));

        uint256 inputsLen = order.inputs.length;
        TokenInfo[] memory refunds = new TokenInfo[](inputsLen);
        for (uint256 i; i < inputsLen;) {
            uint256 escrowTotal = order.inputs[i].amount;
            uint256 filled = _partialFills[commitment][order.output.assets[i].token];
            uint256 refund = escrowTotal - _cumulativeReleased(escrowTotal, filled, order.output.assets[i].amount);
            refunds[i] = TokenInfo({token: order.inputs[i].token, amount: refund});
            unchecked {
                ++i;
            }
        }

        bytes memory body = bytes.concat(
            bytes1(uint8(RequestKind.RefundEscrow)),
            abi.encode(WithdrawalRequest({commitment: commitment, tokens: refunds, beneficiary: order.user}))
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
            dispatchWithFeeToken(request);
        }
    }

    /**
     * @dev Handles incoming cross-chain post requests dispatched via Hyperbridge.
     * The first byte of the request body encodes the `RequestKind`, which determines
     * the action to take:
     *
     * - RedeemEscrow: Releases escrowed tokens to the solver who completed (fully filled) the
     *   order on the destination chain, finalizing it and forwarding accumulated fees.
     *   Authenticated against the registered gateway instance.
     * - RedeemEscrowPartial: Releases a proportional slice of escrowed tokens to a solver who
     *   partially filled the order, without finalizing it (so further redeems and the user's
     *   cancel refund remain possible). Authenticated against the registered gateway instance.
     * - RefundEscrow: Refunds escrowed tokens to the original user after a successful
     *   cancellation from the destination chain. Authenticated against the registered gateway.
     * - NewDeployment: Registers a new gateway instance for a state machine. Only
     *   Hyperbridge itself may dispatch this request.
     * - UpdateParams: Updates the gateway's configuration parameters and per-destination
     *   protocol fees. Only Hyperbridge may dispatch this request.
     * - SweepDust: Transfers accumulated protocol dust to a specified beneficiary.
     *   Only Hyperbridge may dispatch this request.
     * - UpgradeContract: Points the ERC-1967 proxy at a new implementation, optionally
     *   running migration calldata atomically. Only Hyperbridge may dispatch this request.
     *
     * @param incoming The incoming post request from Hyperbridge.
     */
    function onAccept(IncomingPostRequest calldata incoming) external override onlyHost {
        RequestKind kind = RequestKind(uint8(incoming.request.body[0]));
        if (
            kind == RequestKind.RedeemEscrow ||
            kind == RequestKind.RefundEscrow ||
            kind == RequestKind.RedeemEscrowPartial
        ) {
            _authenticate(incoming.request);
            WithdrawalRequest memory body = abi.decode(incoming.request.body[1:], (WithdrawalRequest));
            // A partial redeem must not finalize: escrow stays open for further redeems / a cancel
            // refund, and the fee pot is left for the completing redeem. _withdraw emits EscrowReleased
            // regardless of finalize, so the partial release is still observable on the source chain.
            bool finalize = kind != RequestKind.RedeemEscrowPartial;
            return _withdraw(body, kind == RequestKind.RefundEscrow, finalize);
        }

        // only hyperbridge is permitted to perform these actions
        if (keccak256(incoming.request.source) != keccak256(IDispatcher(host()).hyperbridge())) revert Unauthorized();
        if (kind == RequestKind.NewDeployment) {
            _addDeployment(abi.decode(incoming.request.body[1:], (Deployment)));
        } else if (kind == RequestKind.UpdateParams) {
            _updateParams(abi.decode(incoming.request.body[1:], (ParamsUpdate)));
        } else if (kind == RequestKind.SweepDust) {
            _sweepDust(abi.decode(incoming.request.body[1:], (SweepDust)));
        } else if (kind == RequestKind.UpgradeContract) {
            (address newImpl, bytes memory initData) = abi.decode(incoming.request.body[1:], (address, bytes));
            ERC1967Utils.upgradeToAndCall(newImpl, initData);
        }
    }

    /**
     * @dev Handles the response to a Hyperbridge GET request dispatched during
     * `_cancelFromSource`. The response carries the destination's `_partialFills[commitment][token]`
     * value for each output token; for each escrowed input this refunds the proven-unredeemed
     * fraction (`escrowTotal - _cumulativeReleased(escrowTotal, filled, totalRequired)`) to the
     * user, leaving exactly enough escrow to cover redeems still in flight. The order is marked
     * filled for idempotency, and the user's prepaid fees are returned only if the order did not
     * fully fill on the destination. Reverts with `Filled` on a duplicate cancel response.
     *
     * @param incoming The incoming GET response from Hyperbridge containing the storage proofs.
     */
    function onGetResponse(IncomingGetResponse calldata incoming) external override onlyHost {
        (bytes32 commitment, bytes32 beneficiary, TokenInfo[] memory inputs, uint256[] memory totalRequired) =
            abi.decode(incoming.response.request.context, (bytes32, bytes32, TokenInfo[], uint256[]));

        // Idempotency: block duplicate/concurrent cancel responses before releasing any funds.
        if (_filled[commitment] != address(0)) revert Filled();
        _filled[commitment] = address(uint160(uint256(beneficiary)));

        uint256 len = inputs.length;
        TokenInfo[] memory refunds = new TokenInfo[](len);
        bool fullyFilled = true;
        for (uint256 i; i < len;) {
            // Values come back sorted by key, not in request order, so match by key. request.keys[i]
            // is the slot for input i's output, and the request is verified against its committed hash.
            bytes calldata raw = _proofValueForKey(incoming, incoming.response.request.keys[i]);
            uint256 filled = raw.length == 0 ? 0 : raw.toRlpItem().toUint();

            // Refund only the unredeemed fraction; the complement is what pre-deadline fills will redeem.
            uint256 escrowTotal = inputs[i].amount;
            uint256 refund = escrowTotal - _cumulativeReleased(escrowTotal, filled, totalRequired[i]);
            if (filled < totalRequired[i]) fullyFilled = false;
            refunds[i] = TokenInfo({token: inputs[i].token, amount: refund});
            unchecked {
                ++i;
            }
        }

        // `_filled` is already set above for idempotency. Finalize — which flushes the prepaid fee
        // pot to the user — only when the order did not fully fill; a fully-filled order's fees belong
        // to the completing solver. _withdraw emits EscrowRefunded for the refunded tokens.
        _withdraw(
            WithdrawalRequest({
                commitment: commitment, 
                tokens: refunds, 
                beneficiary: beneficiary
            }),
            true,
            !fullyFilled
        );
    }

    /**
     * @dev Returns the proof value whose storage key matches `key`. GET responses return values
     * sorted by key (the responder iterates a BTreeMap), so positional indexing would mispair
     * values with inputs for multi-token orders. Absent slots are still returned (with an empty
     * value), so a matching key is always expected; reverts if none is found.
     * @param incoming The incoming GET response.
     * @param key The expected storage key (one of the request's keys).
     * @return The raw (RLP-encoded) proof value bytes for that key.
     */
    function _proofValueForKey(IncomingGetResponse calldata incoming, bytes calldata key)
        internal
        pure
        returns (bytes calldata)
    {
        bytes32 want = keccak256(key);
        uint256 n = incoming.response.values.length;
        for (uint256 j; j < n;) {
            if (keccak256(incoming.response.values[j].key) == want) return incoming.response.values[j].value;
            unchecked {
                ++j;
            }
        }
        revert InvalidInput();
    }
}
