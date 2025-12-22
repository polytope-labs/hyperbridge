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

import {PackedUserOperation, IAccountExecute} from "@openzeppelin/contracts/interfaces/draft-IERC4337.sol";
import {Account} from "@openzeppelin/contracts/account/Account.sol";
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";

/**
 * @title IntentSolverAccount
 * @notice ERC-4337 compliant smart contract account for executing cross-chain intents via Hyperbridge
 * @dev This contract extends OpenZeppelin's Account implementation and integrates with the Intent Gateway
 *      to enable voucher-based transaction execution. It supports both standard ECDSA signatures and
 *      voucher-based authorization from the Intent Gateway for cross-chain operations.
 * @author Polytope Labs
 */
contract IntentSolverAccount is Account, IAccountExecute {
    /**
     * @notice Standard length of an ECDSA signature (r: 32 bytes, s: 32 bytes, v: 1 byte)
     */
    uint256 private constant ECDSA_SIGNATURE_LENGTH = 65;

    /**
     * @notice Address of the Intent Gateway V2 contract that authorizes voucher-based transactions
     * @dev This should be set to the deployed Intent Gateway address
     */
    address private constant INTENT_GATEWAY_V2 = address(0);

    /**
     * @notice Validates a user operation before execution
     * @dev Implements ERC-4337 validation logic. For standard ECDSA signatures (65 bytes),
     *      delegates to the parent Account implementation. For longer signatures, validates
     *      voucher-based authorization from the Intent Gateway for cross-chain operations.
     * @param op The packed user operation containing calldata, signature, and other fields
     * @param userOpHash The hash of the user operation (with EntryPoint and chain ID)
     * @param missingAccountFunds The amount of funds missing in the account to pay for gas
     * @return validationData A packed value indicating validation result and time range
     *         - 0 indicates successful validation
     *         - 1 indicates signature validation failure
     *         - Other values can encode time ranges for signature validity
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

        /* TODO: Verify that the Intent Gateway has authorized this voucher for cross-chain execution
         * This should validate the voucher signature and ensure the relayer is authorized
         */
        return 0;
    }

    /**
     * @notice Executes the user operation after successful validation
     * @dev Called by the EntryPoint after validateUserOp succeeds. This is where the actual
     *      transaction logic is executed, including cross-chain intent fulfillment.
     * @param userOp The packed user operation to execute
     * @param userOpHash The hash of the user operation for verification
     */
    function executeUserOp(PackedUserOperation calldata userOp, bytes32 userOpHash) external onlyEntryPoint {
        /* TODO: Execute the selected transaction based on the user operation calldata
         * This should handle both standard transactions and cross-chain intent execution
         */
    }

    /**
     * @notice Validates a raw signature against a hash
     * @dev Internal function used by the Account base contract for signature validation.
     *      Recovers the signer from the ECDSA signature and verifies it matches this contract's address.
     * @param hash The hash that was signed
     * @param signature The ECDSA signature to validate (65 bytes: r, s, v)
     * @return bool True if the recovered signer matches this contract's address, false otherwise
     */
    function _rawSignatureValidation(bytes32 hash, bytes calldata signature) internal view override returns (bool) {
        return ECDSA.recover(hash, signature) == address(this);
    }
}
