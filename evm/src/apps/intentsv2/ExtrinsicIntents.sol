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
    ParamsUpdate,
    SweepDust,
    WithdrawalRequest,
    FillOptions,
    CancelOptions,
    Deployment
} from "@hyperbridge/core/apps/IntentGatewayV2.sol";
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
    using RLPReader for bytes;
    using RLPReader for RLPReader.RLPItem;

    /**
     * @dev The Hyperbridge host.
     */
    function host() public view virtual override(IntentsBase, HyperApp) returns (address) {
        return _params.host;
    }

    /**
     * @dev Reverts unless the request comes from the gateway registered for its source chain.
     */
    function _authenticate(PostRequest calldata request) internal view {
        if (request.from.length != 20) revert InvalidInput();
        address module = address(bytes20(request.from));
        if (_instance(request.source) != module) revert Unauthorized();
    }

    /**
     * @dev Once a relayer is set, rejects deliveries from anyone else.
     */
    modifier onlyRelayer(address relayer) {
        address authorised = _relayer;
        if (authorised != address(0) && relayer != authorised) revert Unauthorized();
        _;
    }

    /**
     * @dev Rotates the relayer; zero accepts any. Host-only, so reached through `Execute`.
     */
    function setRelayer(address relayer) external onlyHost {
        _setRelayer(relayer);
    }

    /**
     * @dev Upgrades the proxy and runs `data` on the new implementation. Host-only, so reached
     * through `Execute`.
     */
    function upgradeToAndCall(address newImplementation, bytes calldata data) external onlyHost {
        ERC1967Utils.upgradeToAndCall(newImplementation, data);
    }

    /**
     * @dev `kind` followed by the ABI-encoded `WithdrawalRequest`.
     */
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

    /**
     * @dev Posts `body` to the source chain's gateway, paying `nativeFee` natively when non-zero
     * and in the fee token otherwise.
     */
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
     * @dev Pays each leg here and asks the source chain for the escrow it earns: `RedeemEscrow` for
     * a completing fill, `RedeemEscrowPartial` otherwise. The result is net of any native dispatch
     * fee.
     */
    function _fillOrder(Order calldata order, FillOptions calldata options, bytes32 commitment)
        internal
        returns (FillResult memory result)
    {
        result = _fillLegs(order, options, commitment);

        uint256 nativeFee = options.nativeDispatchFee;
        if (nativeFee > result.nativeRemaining) nativeFee = 0;
        result.nativeRemaining -= nativeFee;

        RequestKind requestKind = result.fullyFilled ? RequestKind.RedeemEscrow : RequestKind.RedeemEscrowPartial;
        bytes memory body = _body(requestKind, commitment, result.releasedInputs, bytes32(uint256(uint160(msg.sender))));
        _post(order, body, options.relayerFee, nativeFee);
    }

    /**
     * @dev Proves each leg's fill progress on the destination with a GET request that
     * `onGetResponse` settles. Creator only, and only after the deadline, so the proof is final.
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

        if (msg.value > 0) {
            IDispatcher(host()).dispatch{value: msg.value}(request);
        } else {
            dispatchWithFeeToken(request);
        }
    }

    /**
     * @dev Closes the order here and posts `RefundEscrow` for each leg's unfilled rest. Creator
     * only until the deadline.
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
     * @dev Handles escrow redemptions and refunds from peer gateways, and governance requests from
     * Hyperbridge.
     */
    function onAccept(IncomingPostRequest calldata incoming) external override onlyHost onlyRelayer(incoming.relayer) {
        RequestKind kind = RequestKind(uint8(incoming.request.body[0]));
        if (
            kind == RequestKind.RedeemEscrow || kind == RequestKind.RefundEscrow
                || kind == RequestKind.RedeemEscrowPartial
        ) {
            _authenticate(incoming.request);
            WithdrawalRequest memory body = abi.decode(incoming.request.body[1:], (WithdrawalRequest));
            // Refuse destination-chain cancel if source-chain already cancelled
            if (kind == RequestKind.RefundEscrow && _filled[body.commitment] != address(0)) revert Filled();
            // A partial redeem doesn't finalize, leaving the escrow and fees for later redeems or a cancel.
            return _withdraw(
                Withdrawal({
                    commitment: body.commitment,
                    beneficiary: body.beneficiary,
                    tokens: body.tokens,
                    isRefund: kind == RequestKind.RefundEscrow,
                    finalize: kind != RequestKind.RedeemEscrowPartial
                })
            );
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
     * @dev Settles a source-chain cancel: refunds each leg's proven unfilled rest, and the fees
     * unless the order completed.
     */
    function onGetResponse(IncomingGetResponse calldata incoming)
        external
        override
        onlyHost
        onlyRelayer(incoming.relayer)
    {
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

        _withdraw(
            Withdrawal({
                commitment: commitment,
                beneficiary: beneficiary,
                tokens: refunds,
                isRefund: true,
                finalize: !fullyFilled
            })
        );
    }

    /**
     * @dev Indexes the response's values by key in transient storage, so each leg finds its value
     * in one lookup.
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
     * @dev The proven value for `key`. Responses sort values by key, not request order, so legs are
     * matched by key.
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

    /**
     * @dev Clears the index from transient storage, so a later response
     * in the same transaction cannot read it.
     */
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
