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
import {PackedUserOperation} from "@openzeppelin/contracts/interfaces/draft-IERC4337.sol";
import {Execution} from "@openzeppelin/contracts/interfaces/draft-IERC7579.sol";
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {IERC1271} from "@openzeppelin/contracts/interfaces/IERC1271.sol";

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
     * @dev A 65-byte signature is plain ECDSA over `userOpHash`. It is refused for ops that call
     * `fillOrder`: bids are public, so a bid's solver signature could be replayed here to burn its
     * nonce and the solver's gas.
     *
     * A 162-byte signature is `abi.encodePacked(commitment, solverSignature, sessionSignature)`.
     * The gateway's `select` recovers the session key, and the nonce key must derive from the
     * commitment, that key and the op's calldata, so none of them can be swapped after signing.
     *
     * The calldata is what makes each bid's key its own. A solver bidding several prices on one
     * order signs one op per price, and with a key shared across them the EntryPoint would run
     * them only in sequence order: a bid that was never selected would block every one behind it.
     * Keyed by their calldata, each bid is sequence 0 of its own key and can execute on its own,
     * in any order, while the same calldata still executes once.
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

        // Recovers the session key and stages the selection that `fillOrder` checks. A bad session
        // signature fails validation instead of reverting it, as ERC-4337 requires.
        SelectOptions memory selectOptions =
            SelectOptions({commitment: commitment, solver: address(this), signature: sessionSignature});
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
        return selector == FILL_ORDER_SELECTOR 
            || selector == HISTORICAL_FILL_ORDER_SELECTOR
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
