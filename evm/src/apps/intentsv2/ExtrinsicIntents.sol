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
import {ERC1967Utils} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Utils.sol";
import {Address} from "@openzeppelin/contracts/utils/Address.sol";
import {RLPReader} from "@polytope-labs/solidity-merkle-trees/src/trie/ethereum/RLPReader.sol";

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
     * @dev Once a relayer is set, rejects deliveries from anyone else before the body is read. The
     * host records the revert as undelivered, so the authorised relayer can resubmit. While unset,
     * every delivery passes: a proxy stays open until governance arms it through `setRelayer`.
     * @param relayer The account that submitted the message to the handler.
     */
    function _checkRelayer(address relayer) internal view {
        address authorised = _relayer;
        if (authorised != address(0) && relayer != authorised) revert Unauthorized();
    }

    /**
     * @dev Rotates the authorised relayer. Host-only, so reachable only through an `Execute`
     * request, which delegatecalls it with the host still `msg.sender`. Leaves `version()` alone.
     * @param relayer The account whose deliveries are accepted from now on. Zero reopens the gate.
     */
    function setRelayer(address relayer) external onlyHost {
        _setRelayer(relayer);
    }

    /**
     * @dev Points the proxy at `newImplementation` and delegatecalls `data` on it in the same
     * transaction, e.g. `migrate()`. Host-only, so reachable only through `Execute`.
     * @param newImplementation The implementation to install; must have code.
     * @param data Migration calldata run against the new implementation, or empty.
     */
    function upgradeToAndCall(address newImplementation, bytes calldata data) external onlyHost {
        ERC1967Utils.upgradeToAndCall(newImplementation, data);
    }

    /// @dev `kind` followed by the ABI-encoded `WithdrawalRequest`.
    function _body(RequestKind kind, bytes32 commitment, TokenInfo[] memory tokens, bytes32 beneficiary)
        internal
        pure
        returns (bytes memory)
    {
        return bytes.concat(
            bytes1(uint8(kind)),
            abi.encode(WithdrawalRequest({commitment: commitment, tokens: tokens, beneficiary: beneficiary}))
        );
    }

    /// @dev Posts `body` to the gateway on the order's source chain, paying `nativeFee` in native
    /// tokens when non-zero and in the fee token otherwise.
    function _post(Order calldata order, bytes memory body, uint256 relayerFee, uint256 nativeFee) internal {
        DispatchPost memory request = DispatchPost({
            dest: order.source,
            to: abi.encodePacked(_instance(order.source)),
            body: body,
            timeout: 0,
            fee: relayerFee,
            payer: msg.sender
        });
        if (nativeFee > 0) {
            IDispatcher(host()).dispatch{value: nativeFee}(request);
        } else {
            dispatchWithFeeToken(request);
        }
    }

    /**
     * @dev Delivers output on this chain and requests the matching per-leg escrow from
     * the source chain. Partial fills send RedeemEscrowPartial and reopen the order;
     * a completing fill executes beneficiary calldata and sends RedeemEscrow.
     * @param order The cross-chain order being filled.
     * @param options Output payments, optional input quotes and dispatch fees.
     * @param commitment The keccak256 hash of the ABI-encoded order.
     */
    function _fillCrossChain(Order calldata order, FillOptions calldata options, bytes32 commitment) internal {
        _filled[commitment] = msg.sender;
        FillResult memory result = _fillLegs(order, options, commitment, false);
        if (result.fullyFilled) {
            _execute(order, order.output.assets.length);
        } else {
            delete _filled[commitment];
        }

        uint256 nativeFee = options.nativeDispatchFee;
        if (nativeFee > result.nativeRemaining) nativeFee = 0;
        result.nativeRemaining -= nativeFee;

        RequestKind requestKind = result.fullyFilled ? RequestKind.RedeemEscrow : RequestKind.RedeemEscrowPartial;
        bytes memory body = _body(requestKind, commitment, result.releasedInputs, bytes32(uint256(uint160(msg.sender))));
        _post(order, body, options.relayerFee, nativeFee);

        if (result.nativeRemaining > 0) _sendValue(msg.sender, result.nativeRemaining);
        if (result.fullyFilled) {
            emit OrderFilled(commitment, msg.sender, result.creditedOutputs, result.releasedInputs);
        } else {
            emit PartialFill(commitment, msg.sender, result.creditedOutputs, result.releasedInputs);
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
     * `_partialFills[commitment][i]` slot for each leg `i`. The response is handled by
     * `onGetResponse`, which refunds the proven-unredeemed fraction of each escrowed input — never
     * the raw remaining escrow, so that any `RedeemEscrow` messages still in flight for fills that
     * happened before the deadline remain covered.
     *
     * `placeOrder` guarantees `order.inputs.length == order.output.assets.length`, so each input is
     * paired with the output at the same index. The proof keys are per leg, so they stay distinct
     * even when legs repeat an output token.
     *
     * `cancelOrder` has already emitted `OrderCancelled`; the matching `EscrowRefunded` follows
     * on this chain once the GET response returns through Hyperbridge.
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
            keys[i] = bytes.concat(abi.encodePacked(destGateway), _calculatePartialFillSlotHash(commitment, i));
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
     * `cancelOrder` has already emitted `OrderCancelled` on this chain — the only trace of the
     * cancellation a solver watching this chain gets, since the host's `PostRequestEvent` carries
     * no reference to the order. The matching `EscrowRefunded` follows on the source chain once
     * Hyperbridge delivers the refund message.
     *
     * @param order The order to cancel.
     * @param options Cancel options including the relayer fee.
     * @param commitment The keccak256 hash of the ABI-encoded order.
     */
    function _cancelFromDest(Order calldata order, CancelOptions calldata options, bytes32 commitment) internal {
        if (order.deadline >= _blockNumber()) {
            if (order.user != bytes32(uint256(uint160(msg.sender)))) revert Unauthorized();
        }

        // Freeze the order, then snapshot fill progress in the same tx and refund the unredeemed rest.
        _filled[commitment] = address(uint160(uint256(order.user)));

        uint256 inputsLen = order.inputs.length;
        TokenInfo[] memory refunds = new TokenInfo[](inputsLen);
        for (uint256 i; i < inputsLen;) {
            uint256 escrowTotal = order.inputs[i].amount;
            uint256 filled = _partialFills[commitment][i];
            uint256 refund = escrowTotal - _cumulativeReleased(escrowTotal, filled, order.output.assets[i].amount);
            refunds[i] = TokenInfo({token: order.inputs[i].token, amount: refund});
            unchecked {
                ++i;
            }
        }

        _post(order, _body(RequestKind.RefundEscrow, commitment, refunds, order.user), options.relayerFee, msg.value);
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
     *   Rejected with `Filled` once the order is finalized, e.g. by a GET cancel from this chain.
     * - NewDeployment: Registers a new gateway instance for a state machine. Only
     *   Hyperbridge itself may dispatch this request.
     * - UpdateParams: Updates the gateway's configuration parameters and per-destination
     *   protocol fees. Only Hyperbridge may dispatch this request.
     * - SweepDust: Transfers accumulated protocol dust to a specified beneficiary.
     *   Only Hyperbridge may dispatch this request.
     * - Execute: Delegatecalls this module with the rest of the body, the host still
     *   `msg.sender`, so the host-only functions (`upgradeToAndCall`, `setRelayer`) are
     *   reachable. Reverts bubble up unchanged. Only Hyperbridge may dispatch this request.
     *
     * @param incoming The incoming post request from Hyperbridge.
     */
    function onAccept(IncomingPostRequest calldata incoming) external override onlyHost {
        _checkRelayer(incoming.relayer);
        RequestKind kind = RequestKind(uint8(incoming.request.body[0]));
        if (
            kind == RequestKind.RedeemEscrow || kind == RequestKind.RefundEscrow
                || kind == RequestKind.RedeemEscrowPartial
        ) {
            _authenticate(incoming.request);
            WithdrawalRequest memory body = abi.decode(incoming.request.body[1:], (WithdrawalRequest));
            // An order can be cancelled from both chains, and each only sees its own `_filled`. Once one cancel
            // has finalized it here, a RefundEscrow from the other would refund the same unfilled slice again,
            // out of the escrow reserved for redeems still in flight. Redeems stay allowed: they consume it.
            if (kind == RequestKind.RefundEscrow && _filled[body.commitment] != address(0)) revert Filled();
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
        } else if (kind == RequestKind.Execute) {
            Address.functionDelegateCall(__self, incoming.request.body[1:]);
        }
    }

    /**
     * @dev Handles the response to a Hyperbridge GET request dispatched during
     * `_cancelFromSource`. The response carries the destination's `_partialFills[commitment][i]`
     * value for each leg `i`; for each escrowed input this refunds the proven-unredeemed
     * fraction (`escrowTotal - _cumulativeReleased(escrowTotal, filled, totalRequired)`) to the
     * user, leaving exactly enough escrow to cover redeems still in flight. The order is marked
     * filled for idempotency, and the user's prepaid fees are returned only if the order did not
     * fully fill on the destination. Reverts with `Filled` on a duplicate cancel response.
     *
     * @param incoming The incoming GET response from Hyperbridge containing the storage proofs.
     */
    function onGetResponse(IncomingGetResponse calldata incoming) external override onlyHost {
        _checkRelayer(incoming.relayer);
        (bytes32 commitment, bytes32 beneficiary, TokenInfo[] memory inputs, uint256[] memory totalRequired) =
            abi.decode(incoming.response.request.context, (bytes32, bytes32, TokenInfo[], uint256[]));

        // Idempotency: block duplicate/concurrent cancel responses before releasing any funds.
        if (_filled[commitment] != address(0)) revert Filled();
        _filled[commitment] = address(uint160(uint256(beneficiary)));

        uint256 len = inputs.length;
        TokenInfo[] memory refunds = new TokenInfo[](len);
        bool fullyFilled = true;
        bytes32[] memory proofSlots = _indexProofValues(incoming);
        for (uint256 i; i < len;) {
            // Values come back sorted by key, not in request order, so match by key. request.keys[i]
            // is leg i's slot, and the request is verified against its committed hash.
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
        _clearProofValueIndex(proofSlots);

        // `_filled` is already set above for idempotency. Finalize — which flushes the prepaid fee
        // pot to the user — only when the order did not fully fill; a fully-filled order's fees belong
        // to the completing solver. _withdraw emits EscrowRefunded for the refunded tokens.
        _withdraw(
            WithdrawalRequest({commitment: commitment, tokens: refunds, beneficiary: beneficiary}), true, !fullyFilled
        );
    }

    /**
     * @dev Indexes the response's proven values in transient storage: under the hash of each value's
     * key, its position plus one. Every leg then finds its value with one lookup instead of rescanning
     * the values, so a cancel proof for N legs hashes about 2N keys rather than N(N+1)/2. The first
     * value for a key wins. A slot is the keccak256 of a storage key, a different preimage from the
     * reentrancy guard's and solver selection's slots. `onGetResponse` clears them before any transfer.
     * @param incoming The incoming GET response.
     * @return slots The transient slots written, for `_clearProofValueIndex`.
     */
    function _indexProofValues(IncomingGetResponse calldata incoming) internal returns (bytes32[] memory slots) {
        uint256 n = incoming.response.values.length;
        slots = new bytes32[](n);
        for (uint256 j; j < n;) {
            bytes32 slot = keccak256(incoming.response.values[j].key);
            slots[j] = slot;
            assembly ("memory-safe") {
                if iszero(tload(slot)) { tstore(slot, add(j, 1)) }
            }
            unchecked {
                ++j;
            }
        }
    }

    /**
     * @dev Returns the proof value whose storage key matches `key`, through the index
     * `_indexProofValues` built. GET responses return values sorted by key (the responder iterates a
     * BTreeMap), so positional indexing would mispair values with legs on multi-leg orders. Every leg
     * has its own key, so no two legs share a value. Absent slots are still returned (with an empty
     * value), so a matching key is always expected; reverts if none is found.
     * @param incoming The incoming GET response.
     * @param key The expected storage key (one of the request's keys).
     * @return The raw (RLP-encoded) proof value bytes for that key.
     */
    function _proofValueForKey(IncomingGetResponse calldata incoming, bytes calldata key)
        internal
        view
        returns (bytes calldata)
    {
        bytes32 slot = keccak256(key);
        uint256 position;
        assembly ("memory-safe") {
            position := tload(slot)
        }
        if (position == 0) revert InvalidInput();
        return incoming.response.values[position - 1].value;
    }

    /// @dev Clears the transient index `_indexProofValues` wrote, so a later call in the same
    /// transaction cannot resolve a key against another response's values.
    function _clearProofValueIndex(bytes32[] memory slots) internal {
        uint256 n = slots.length;
        for (uint256 j; j < n;) {
            bytes32 slot = slots[j];
            assembly ("memory-safe") {
                tstore(slot, 0)
            }
            unchecked {
                ++j;
            }
        }
    }
}
