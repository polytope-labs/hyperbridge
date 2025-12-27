/// Copyright (C) Polytope Labs Ltd.
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
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";

import {SelectOptions, IIntentGatewayV2} from "@hyperbridge/core/apps/IntentGatewayV2.sol";

/**
 * @title SolverAccount
 * @notice ERC-4337 and ERC-7821 compliant smart contract account for solvers
 * @dev This contract extends OpenZeppelin's Account and ERC7821 implementations and integrates with the IntentGateway
 *      to enable solver delegation primarily for solver selection. Solvers can delegate to this smart
 *      contract account using EIP-7702.
 * @author Polytope Labs
 */
contract SolverAccount is Account, ERC7821 {
    /**
     * @notice Standard length of an ECDSA signature (r: 32 bytes, s: 32 bytes, v: 1 byte)
     */
    uint256 private constant ECDSA_SIGNATURE_LENGTH = 65;

    /**
     * @notice Expected signature length for intent solver selection
     * @dev abi.encodePacked(commitment, solverSignature, sessionSignature) = 32 + 65 + 65 = 162 bytes
     */
    uint256 private constant INTENT_SELECT_SIGNATURE_LENGTH = 162;

    /**
     * @notice Cached select function selector
     */
    bytes4 private constant SELECT_SELECTOR = IIntentGatewayV2.select.selector;

    /**
     * @notice Address of the Intent Gateway V2 contract that authorizes voucher-based transactions
     * @dev This is set during deployment via constructor
     */
    address private immutable INTENT_GATEWAY_V2;

    /**
     * @notice Constructor for SolverAccount
     * @param intentGatewayV2 The IntentGatewayV2 contract address
     * @dev The solver EOA (via EIP-7702) will sign all operations on behalf of this solver account.
     *      The solver is identified by the deployed contract address (address(this)).
     */
    constructor(address intentGatewayV2) {
        INTENT_GATEWAY_V2 = intentGatewayV2;
    }

    /**
     * @notice Validates a user operation before execution
     * @dev Implements ERC-4337 validation logic with two modes:
     *
     * @param op The packed user operation containing calldata, signature, and other fields
     * @param userOpHash The hash of the user operation (with EntryPoint and chain ID)
     * @param missingAccountFunds The amount of funds missing in the account to pay for gas
     * @return validationData A packed value indicating validation result and time range
     *         - SIG_VALIDATION_SUCCESS indicates successful validation
     *         - SIG_VALIDATION_FAILED indicates signature validation failure
     */
    function validateUserOp(PackedUserOperation calldata op, bytes32 userOpHash, uint256 missingAccountFunds)
        public
        override
        onlyEntryPoint
        returns (uint256)
    {
        if (op.signature.length == ECDSA_SIGNATURE_LENGTH) {
            return super.validateUserOp(op, userOpHash, missingAccountFunds);
        }

        // Expected format: abi.encodePacked(commitment, solverSignature, sessionSignature)
        // commitment: 32 bytes, solverSignature: 65 bytes, sessionSignature: 65 bytes
        if (op.signature.length < INTENT_SELECT_SIGNATURE_LENGTH) return ERC4337Utils.SIG_VALIDATION_FAILED;

        bytes32 commitment = bytes32(op.signature[0:32]);
        bytes calldata solverSignature = op.signature[32:97];
        bytes calldata sessionSignature = op.signature[97:162];

        // Call IntentGatewayV2.select to recover the sessionKey
        SelectOptions memory selectOptions =
            SelectOptions({commitment: commitment, solver: address(this), signature: sessionSignature});
        bytes memory selectCalldata = abi.encodeWithSelector(SELECT_SELECTOR, selectOptions);
        (bool success, bytes memory returnData) = INTENT_GATEWAY_V2.call(selectCalldata);

        if (!success || returnData.length < 32) return ERC4337Utils.SIG_VALIDATION_FAILED;

        address sessionKey = abi.decode(returnData, (address));

        // Recover the solver's account from the solver signature over (userOpHash, commitment, sessionKey)
        bytes32 messageHash = keccak256(abi.encodePacked(userOpHash, commitment, sessionKey));
        bytes32 ethSignedMessageHash = keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", messageHash));

        if (!_rawSignatureValidation(ethSignedMessageHash, solverSignature)) return ERC4337Utils.SIG_VALIDATION_FAILED;

        // Pay for gas if needed
        _payPrefund(missingAccountFunds);

        return ERC4337Utils.SIG_VALIDATION_SUCCESS;
    }

    /**
     * @notice Validates a raw signature against a hash
     * @dev Internal function used by the Account base contract for signature validation.
     *      Recovers the signer from the ECDSA signature and verifies it matches address(this).
     *      In EIP-7702 delegation, the EOA's address becomes this contract's address.
     *      Used for both standard ERC-4337 operations and intent solver selection validation.
     * @param hash The hash that was signed (typically userOpHash or Ethereum signed message hash)
     * @param signature The ECDSA signature to validate (65 bytes: r, s, v)
     * @return bool True if the recovered signer matches this contract's address, false otherwise
     */
    function _rawSignatureValidation(bytes32 hash, bytes calldata signature) internal view override returns (bool) {
        return ECDSA.recover(hash, signature) == address(this);
    }

    /**
     * @notice Validates an ERC-7821 authorized executor
     * @param caller The address of the caller
     * @param mode The mode of the call
     * @param executionData The data of the call
     * @return bool True if the caller is authorized, false otherwise
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
