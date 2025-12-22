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
import {ERC4337Utils} from "@openzeppelin/contracts/account/utils/draft-ERC4337Utils.sol";
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";

import {SelectOptions} from "./interfaces/IntentGatewayV2.sol";

/**
 * @title IntentSolverAccount
 * @notice ERC-4337 compliant smart contract account for solvers
 * @dev This contract extends OpenZeppelin's Account implementation and integrates with the Intent Gateway
 *      to enable solver delegation primarily for solver selection. Solvers can delegate to this smart
 *      contract account using EIP-7702.
 *
 *      The contract supports two validation modes:
 *      1. Standard ERC-4337 validation: Owner signs the userOpHash (65-byte ECDSA signature)
 *      2. Intent solver selection: Owner signs EIP-712 structured data (commitment, sessionKey) to
 *         authorize the solver account to be selected for filling orders
 *
 *      Workflow for intent solver selection:
 *      - User places an order with a session key
 *      - Owner signs SelectSolver(commitment, sessionKey) using EIP-712
 *      - UserOp contains: calldata = IntentGatewayV2.select(...)
 *      - validateUserOp verifies the owner signature and calls IntentGateway.select
 *      - The solver (this contract's address) is registered and can fill the order
 *
 * @author Polytope Labs
 */
contract IntentSolverAccount is Account, IAccountExecute {
    /**
     * @notice Standard length of an ECDSA signature (r: 32 bytes, s: 32 bytes, v: 1 byte)
     */
    uint256 private constant ECDSA_SIGNATURE_LENGTH = 65;

    /**
     * @notice Address of the Intent Gateway V2 contract that authorizes voucher-based transactions
     * @dev This is set during deployment via constructor
     */
    address private immutable INTENT_GATEWAY_V2;

    /**
     * @notice EIP-712 Domain separator type hash
     */
    bytes32 private constant DOMAIN_TYPEHASH =
        keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)");

    /**
     * @notice EIP-712 type hash for SelectSolver message
     */
    bytes32 private constant SELECT_SOLVER_TYPEHASH = keccak256("SelectSolver(bytes32 commitment,address solver)");

    /**
     * @notice Constructor for IntentSolverAccount
     * @param intentGatewayV2 The IntentGatewayV2 contract address
     * @dev The owner EOA will sign all operations on behalf of this solver account.
     *      The solver is identified by the deployed contract address (address(this)).
     */
    constructor(address intentGatewayV2) {
        INTENT_GATEWAY_V2 = intentGatewayV2;
    }

    /**
     * @notice Validates a user operation before execution
     * @dev Implements ERC-4337 validation logic with two modes:
     *
     *      Mode 1 - Standard validation (65-byte signature):
     *      - Delegates to parent Account implementation
     *      - Owner signs the userOpHash
     *
     *      Mode 2 - Intent solver selection (longer signature):
     *      - Signature format: abi.encode(sessionKey, ownerSignature)
     *      - CallData format: IntentGatewayV2.select(SelectOptions)
     *      - Validates that:
     *        1. Solver in calldata matches address(this)
     *        2. Owner signed EIP-712 message: SelectSolver(commitment, sessionKey)
     *        3. IntentGateway.select call succeeds
     *      - On success, registers this solver account for the order
     *
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

        // Decode signature: (sessionKey, solverSignature)
        (address sessionKey, bytes memory solverSignature) = abi.decode(op.signature, (address, bytes));

        // Decode calldata 
        // Ensure calldata has minimum length as IntentGatewayV2.select(SelectOptions)
        if (op.callData.length != 260) return ERC4337Utils.SIG_VALIDATION_FAILED;

        (bytes32 commitment, address solver, bytes memory sessionKeySignature) =
            abi.decode(op.callData[4:], (bytes32, address, bytes));
            
        // Decode SelectOptions from calldata (skip 4-byte selector)
        SelectOptions memory options = abi.decode(op.callData[4:], (SelectOptions));

        if (options.solver != address(this)) return ERC4337Utils.SIG_VALIDATION_FAILED;

        // Compute IntentGatewayV2 domain separator
        bytes32 domainSeparator = keccak256(
            abi.encode(
                DOMAIN_TYPEHASH,
                keccak256(bytes("IntentGateway")),
                keccak256(bytes("2")),
                block.chainid,
                INTENT_GATEWAY_V2
            )
        );

        // Verify that the solver (address(this)) has signed (commitment, sessionKey) using EIP-712
        bytes32 structHash = keccak256(abi.encode(
            SELECT_SOLVER_TYPEHASH,
            options.commitment,
            sessionKey
        ));
        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", domainSeparator, structHash));
        address recoveredSolver = ECDSA.recover(digest, solverSignature);

        if (recoveredSolver != address(this)) return ERC4337Utils.SIG_VALIDATION_FAILED;

        // Call IntentGateway.select with the calldata
        (bool success,) = INTENT_GATEWAY_V2.call(op.callData);
        if (!success) return ERC4337Utils.SIG_VALIDATION_FAILED;

        // Pay for gas if needed
        _payPrefund(missingAccountFunds);

        return ERC4337Utils.SIG_VALIDATION_SUCCESS;
    }

    /**
     * @notice Executes the user operation after successful validation
     * @dev Called by the EntryPoint after validateUserOp succeeds. For intent solver selection,
     *      the actual execution (IntentGateway.select call) happens in validateUserOp, so this
     *      is a no-op. For standard operations, execution would happen here.
     * @param userOp The packed user operation to execute
     * @param userOpHash The hash of the user operation for verification
     */
    function executeUserOp(PackedUserOperation calldata userOp, bytes32 userOpHash) external onlyEntryPoint {
        // NO-OP: For intent solver selection, we already executed the select call in validateUserOp
    }

    /**
     * @notice Validates a raw signature against a hash
     * @dev Internal function used by the Account base contract for signature validation.
     *      Recovers the signer from the ECDSA signature and verifies it matches the owner EOA.
     *      This is used for standard ERC-4337 operations where the owner signs the userOpHash.
     * @param hash The hash that was signed (typically userOpHash)
     * @param signature The ECDSA signature to validate (65 bytes: r, s, v)
     * @return bool True if the recovered signer matches this contract's address, false otherwise
     */
    function _rawSignatureValidation(bytes32 hash, bytes calldata signature) internal view override returns (bool) {
        return ECDSA.recover(hash, signature) == address(this);
    }
}
