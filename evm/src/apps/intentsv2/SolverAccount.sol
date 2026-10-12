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
pragma solidity ^0.8.17;

import {Account} from "@openzeppelin/contracts/account/Account.sol";
import {ERC4337Utils} from "@openzeppelin/contracts/account/utils/draft-ERC4337Utils.sol";
import {ERC7821} from "@openzeppelin/contracts/account/extensions/draft-ERC7821.sol";
import {IEntryPoint, PackedUserOperation} from "@openzeppelin/contracts/interfaces/draft-IERC4337.sol";
import {Execution} from "@openzeppelin/contracts/interfaces/draft-IERC7579.sol";
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {IERC1271} from "@openzeppelin/contracts/interfaces/IERC1271.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

import {SelectOptions, IIntentGatewayV2} from "@hyperbridge/core/apps/IntentGatewayV2.sol";

/**
 * @title SolverAccount
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @dev ERC-4337 and ERC-7821 account that a solver's EOA delegates to through EIP-7702, so its bids
 * can be selected and filled on the IntentGateway. `address(this)` is the solver's EOA, which signs
 * every operation.
 */
contract SolverAccount is Account, ERC7821, IERC1271 {
    using SafeERC20 for IERC20;

    /**
     * @dev What each limit order has paid out so far, by order id.
     *
     * @custom:storage-location erc7201:hyperbridge.storage.SolverAccount.Budgets
     */
    struct Budgets {
        mapping(bytes32 => uint256) spent;
    }

    /**
     * @dev The payout would take `orderId` to `total`, past its `cap`.
     */
    error LimitOrderExceeded(bytes32 orderId, uint256 total, uint256 cap);

    /**
     * @dev `keccak256(abi.encode(uint256(keccak256("hyperbridge.storage.SolverAccount.Budgets")) - 1))`
     * with its last byte cleared. Namespaced because the storage is the EOA's own, shared with
     * whatever else it delegates to.
     */
    bytes32 private constant BUDGETS_STORAGE_SLOT = 0xef37eedb8cd243d7bb1074a6cb5a4fad8c39bd328408761135c4a5a7d5c29900;

    /**
     * @dev ERC-4337 EntryPoint v0.9, the only one this account accepts.
     */
    IEntryPoint private constant ENTRYPOINT_V09 = IEntryPoint(0x433709009B8330FDa32311DF1C2AFA402eD8D009);

    /**
     * @dev A plain ECDSA signature: r, s, v.
     */
    uint256 private constant ECDSA_SIGNATURE_LENGTH = 65;

    /**
     * @dev `abi.encodePacked(commitment, solverSignature, sessionSignature)`: 32 + 65 + 65 bytes.
     */
    uint256 private constant INTENT_SELECT_SIGNATURE_LENGTH = 162;

    /**
     * @dev `IIntentGatewayV2.fillOrder`, refused on the 65-byte path.
     */
    bytes4 private constant FILL_ORDER_SELECTOR = IIntentGatewayV2.fillOrder.selector;

    /**
     * @dev `fillOrder` of earlier gateway releases, with and without `validUntil`. Bids signed
     * against them are still public, so they are refused on the 65-byte path too.
     */
    bytes4 private constant HISTORICAL_FILL_ORDER_SELECTOR = 0xa5470064;
    bytes4 private constant HISTORICAL_FILL_ORDER_NO_EXPIRY_SELECTOR = 0x5cfb1ea5;

    /**
     * @dev ERC-7821 `execute`, the batch a bid's calldata is wrapped in.
     */
    bytes4 private constant EXECUTE_SELECTOR = ERC7821.execute.selector;

    /**
     * @dev The gateway this account selects and fills on.
     */
    address private immutable _intentGateway;

    /**
     * @param gateway The IntentGateway instance this account selects and fills on.
     */
    constructor(address gateway) {
        _intentGateway = gateway;
    }

    /**
     * @dev EntryPoint v0.9 in place of OpenZeppelin's v0.8. It gates `validateUserOp`, `getNonce`
     * and the EntryPoint's right to execute batches.
     */
    function entryPoint() public pure override returns (IEntryPoint) {
        return ENTRYPOINT_V09;
    }

    /**
     * @dev A 65-byte signature is plain ECDSA over `userOpHash`. It is refused for ops that call
     * `fillOrder`: bids are public, so a bid's solver signature could be replayed here to burn its
     * nonce and the solver's gas.
     *
     * A 162-byte signature is `abi.encodePacked(commitment, solverSignature, sessionSignature)`.
     * The session key signs the commitment and `userOpHash`, so its selection holds for this op
     * alone. The gateway's `select` recovers that key, and the nonce key must derive from the
     * commitment, that key and the op's calldata, so none of them can be swapped after signing.
     *
     * The calldata is what makes each bid's key its own. A solver bidding several prices on one
     * order signs one op per price, and with a key shared across them the EntryPoint would run
     * them only in sequence order: a bid that was never selected would block every one behind it.
     * Keyed by their calldata, each bid is sequence 0 of its own key and can execute on its own,
     * in any order. Whether an order can be filled again is the gateway's to decide, not the
     * key's: `_filled[commitment]` closes it on a full fill, and a partial fill leaves it open.
     */
    function validateUserOp(PackedUserOperation calldata op, bytes32 userOpHash, uint256 missingAccountFunds)
        public
        override
        onlyEntryPoint
        returns (uint256)
    {
        if (op.signature.length == ECDSA_SIGNATURE_LENGTH) {
            if (_containsFillOrder(op.callData)) return ERC4337Utils.SIG_VALIDATION_FAILED;
            return super.validateUserOp(op, userOpHash, missingAccountFunds);
        }

        if (op.signature.length < INTENT_SELECT_SIGNATURE_LENGTH) return ERC4337Utils.SIG_VALIDATION_FAILED;

        bytes32 commitment = bytes32(op.signature[0:32]);
        bytes calldata solverSignature = op.signature[32:97];
        bytes calldata sessionSignature = op.signature[97:162];

        // Recovers the session key and stages the selection that `fillOrder` checks while this op
        // executes. A bad session signature fails validation instead of reverting it, as ERC-4337
        // requires.
        SelectOptions memory selectOptions =
            SelectOptions({commitment: commitment, userOpHash: userOpHash, signature: sessionSignature});
        address sessionKey;
        try IIntentGatewayV2(_intentGateway).select(selectOptions) returns (address recovered) {
            sessionKey = recovered;
        } catch {
            return ERC4337Utils.SIG_VALIDATION_FAILED;
        }

        uint192 nonceKey = uint192(uint256(keccak256(abi.encodePacked(commitment, sessionKey, keccak256(op.callData)))));
        if (uint192(op.nonce >> 64) != nonceKey) return ERC4337Utils.SIG_VALIDATION_FAILED;
        if (!_rawSignatureValidation(userOpHash, solverSignature)) return ERC4337Utils.SIG_VALIDATION_FAILED;

        _payPrefund(missingAccountFunds);

        return ERC4337Utils.SIG_VALIDATION_SUCCESS;
    }

    /**
     * @dev Whether `callData` is an ERC-7821 batch that calls the gateway's `fillOrder`. Only that
     * shape matters: the solver's signature covers the calldata, so a replayed bid can't be
     * reshaped to hide the call.
     */
    function _containsFillOrder(bytes calldata callData) private view returns (bool) {
        if (callData.length < 4 || bytes4(callData[0:4]) != EXECUTE_SELECTOR) return false;

        (, bytes memory executionData) = abi.decode(callData[4:], (bytes32, bytes));
        Execution[] memory calls = abi.decode(executionData, (Execution[]));

        for (uint256 i = 0; i < calls.length; i++) {
            if (calls[i].target == _intentGateway && _isFillOrder(bytes4(calls[i].callData))) return true;
        }
        return false;
    }

    /**
     * @dev Whether `selector` is a `fillOrder` of this or an earlier gateway release.
     */
    function _isFillOrder(bytes4 selector) private pure returns (bool) {
        return selector == FILL_ORDER_SELECTOR || selector == HISTORICAL_FILL_ORDER_SELECTOR
            || selector == HISTORICAL_FILL_ORDER_NO_EXPIRY_SELECTOR;
    }

    /**
     * @dev Accepts only ECDSA signatures by the solver's EOA.
     */
    function _rawSignatureValidation(bytes32 hash, bytes calldata signature) internal view override returns (bool) {
        return ECDSA.recover(hash, signature) == address(this);
    }

    /**
     * @dev ERC-1271, for checks like USDC's permit. The delegated EOA has code, so OpenZeppelin's
     * `SignatureChecker` asks this instead of using `ecrecover`.
     */
    function isValidSignature(bytes32 hash, bytes calldata signature) external view override returns (bytes4) {
        return _rawSignatureValidation(hash, signature) ? bytes4(0x1626ba7e) : bytes4(0xffffffff);
    }

    /**
     * @dev Adds what a fill paid out in `token` to the tally of `orderId`, and reverts the batch if
     * that takes it past `cap`. Called by the account on itself, last in a fill's batch.
     *
     * The payout is read off the gateway's allowance: what the batch approved, less what is left,
     * less the dispatch fee the gateway drew from the same allowance. The allowance is then cleared.
     * @param orderId The limit order the fill counts against.
     * @param cap The most the order may pay out in total.
     * @param token The token the fill paid out.
     * @param approved The allowance the batch gave the gateway for `token`.
     * @param fee The dispatch fee the gateway pulls from this token's allowance, zero when the fee
     * token is another one or the dispatch is paid in the native token.
     */
    function debitOrder(bytes32 orderId, uint256 cap, address token, uint256 approved, uint256 fee) external {
        if (msg.sender != address(this)) revert AccountUnauthorized(msg.sender);

        uint256 used = approved - IERC20(token).allowance(address(this), _intentGateway) - fee;
        Budgets storage budgets = _budgets();
        uint256 total = budgets.spent[orderId] + used;
        if (total > cap) revert LimitOrderExceeded(orderId, total, cap);
        budgets.spent[orderId] = total;

        IERC20(token).forceApprove(_intentGateway, 0);
    }

    /**
     * @dev What the limit order `orderId` has paid out so far.
     */
    function spent(bytes32 orderId) external view returns (uint256) {
        return _budgets().spent[orderId];
    }

    /**
     * @dev The limit orders' tallies, at their namespaced slot.
     */
    function _budgets() private pure returns (Budgets storage budgets) {
        bytes32 slot = BUDGETS_STORAGE_SLOT;
        assembly {
            budgets.slot := slot
        }
    }

    /**
     * @dev Also lets the EntryPoint execute batches.
     */
    function _erc7821AuthorizedExecutor(address caller, bytes32 mode, bytes calldata executionData)
        internal
        view
        virtual
        override
        returns (bool)
    {
        return caller == address(entryPoint()) || super._erc7821AuthorizedExecutor(caller, mode, executionData);
    }
}
