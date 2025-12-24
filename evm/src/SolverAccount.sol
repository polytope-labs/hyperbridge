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

import {SelectOptions, IIntentGatewayV2, Order, FillOptions, TokenInfo} from "./interfaces/IntentGatewayV2.sol";

/**
 * @title SolverAccount
 * @notice ERC-4337 and ERC-7821 compliant smart contract account for solvers
 * @dev This contract extends OpenZeppelin's Account and ERC7821 implementations and integrates with the Intent Gateway
 *      to enable solver delegation primarily for solver selection. Solvers can delegate to this smart
 *      contract account using EIP-7702.
 *
 *      The contract supports two validation modes:
 *      1. Standard ERC-4337 validation: Owner signs the userOpHash (65-byte ECDSA signature)
 *      2. Intent solver selection: Owner signs EIP-712 structured data (commitment, sessionKey) to
 *         authorize the solver account to be selected for filling orders
 *
 *      Workflow for intent solver selection via ERC-4337:
 *      - User places an order with a session key
 *      - Solver EOA (via EIP-7702) signs SelectSolver(commitment, sessionKey) using EIP-712
 *      - UserOp is submitted with:
 *        - calldata: IntentGatewayV2.select(SelectOptions)
 *        - signature: abi.encodePacked(sessionKey, solverSignature)
 *      - validateUserOp verifies the signature and calldata (validation phase)
 *      - Bundler executes op.callData by calling this contract (execution phase)
 *      - Fallback function forwards the select() call to IntentGateway
 *      - The solver (this contract's address) is registered and can fill the order
 *
 * @author Polytope Labs
 */
contract SolverAccount is Account, ERC7821 {
    /**
     * @notice Thrown when an unsupported function is called via fallback
     */
    error UnsupportedFunction(bytes4 selector);

    /**
     * @notice Standard length of an ECDSA signature (r: 32 bytes, s: 32 bytes, v: 1 byte)
     */
    uint256 private constant ECDSA_SIGNATURE_LENGTH = 65;

    /**
     * @notice Expected calldata length for select function (4 byte selector + 256 bytes data)
     */
    uint256 private constant SELECT_CALLDATA_LENGTH = 260;

    /**
     * @notice Expected signature length for intent solver selection
     * @dev abi.encodePacked(address, bytes65) = 20 + 65 = 85 bytes
     */
    uint256 private constant INTENT_SELECT_SIGNATURE_LENGTH = 85;

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
     * @notice Cached domain separator from IntentGateway
     */
    bytes32 private immutable DOMAIN_SEPARATOR;

    /**
     * @notice Local SELECT_SOLVER_TYPEHASH with nonce for replay protection
     * @dev SelectSolver(bytes32 commitment,address solver,uint256 nonce)
     */
    bytes32 private constant SELECT_SOLVER_WITH_NONCE_TYPEHASH =
        keccak256("SelectSolver(bytes32 commitment,address solver,uint256 nonce)");

    /**
     * @notice Constructor for SolverAccount
     * @param intentGatewayV2 The IntentGatewayV2 contract address
     * @dev The owner EOA will sign all operations on behalf of this solver account.
     *      The solver is identified by the deployed contract address (address(this)).
     *      Caches domain separator to save gas on validation.
     */
    constructor(address intentGatewayV2) {
        INTENT_GATEWAY_V2 = intentGatewayV2;
        DOMAIN_SEPARATOR = IIntentGatewayV2(intentGatewayV2).DOMAIN_SEPARATOR();
    }

    /**
     * @notice Validates a user operation before execution
     * @dev Implements ERC-4337 validation logic with two modes:
     *
     *      Mode 1 - Standard validation (65-byte signature):
     *      - Delegates to parent Account implementation
     *      - Owner signs the userOpHash
     *
     *      Mode 2 - Intent solver selection (85-byte signature):
     *      - Signature format: abi.encodePacked(sessionKey, ownerSignature)
     *      - CallData format: IntentGatewayV2.select(SelectOptions)
     *      - Validates that:
     *        1. Calldata length and function selector are correct for select()
     *        2. Solver in calldata matches address(this)
     *        3. Nonce from EntryPoint (using first 192 bits of commitment as key)
     *        4. Owner signed EIP-712 message: SelectSolver(commitment, sessionKey, nonce)
     *      - NOTE: This function only validates. The bundler will execute the callData
     *        (IntentGateway.select) via this contract's fallback function after validation.
     *      - Replay protection: Signature includes nonce from EntryPoint
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

        if (
            op.callData.length != SELECT_CALLDATA_LENGTH || bytes4(op.callData[0:4]) != SELECT_SELECTOR
                || op.signature.length != INTENT_SELECT_SIGNATURE_LENGTH
        ) {
            return ERC4337Utils.SIG_VALIDATION_FAILED;
        }

        // Decode SelectOptions from calldata (skip 4-byte selector)
        SelectOptions memory options = abi.decode(op.callData[4:], (SelectOptions));
        if (options.solver != address(this)) return ERC4337Utils.SIG_VALIDATION_FAILED;

        address sessionKey = address(bytes20(op.signature[0:20]));
        bytes calldata solverSignature = op.signature[20:];

        // Get nonce from EntryPoint using first 192 bits of commitment as key
        uint192 nonceKey = uint192(uint256(options.commitment) >> 64);
        uint256 nonce = entryPoint().getNonce(address(this), nonceKey);

        // Validate solver signature with nonce for replay protection
        bytes32 structHash =
            keccak256(abi.encode(SELECT_SOLVER_WITH_NONCE_TYPEHASH, options.commitment, sessionKey, nonce));
        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", DOMAIN_SEPARATOR, structHash));
        if (!_rawSignatureValidation(digest, solverSignature)) return ERC4337Utils.SIG_VALIDATION_FAILED;

        // Pay for gas if needed
        _payPrefund(missingAccountFunds);

        return ERC4337Utils.SIG_VALIDATION_SUCCESS;
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

    /**
     * @notice Fallback function to forward select calls to IntentGateway
     * @dev Only allows calls to select(SelectOptions). All other calls will revert.
     *
     *      This function is called in two scenarios:
     *      1. ERC-4337 execution: After validateUserOp succeeds, the bundler executes
     *         the UserOp.callData by calling this contract, triggering the fallback.
     *      2. Direct calls: Anyone can call this function directly with select() calldata.
     *
     *      The function forwards the call to IntentGateway to register the solver.
     */
    fallback() external payable {
        // Only allow select function
        if (msg.sig != SELECT_SELECTOR) revert UnsupportedFunction(msg.sig);

        // Forward the call to IntentGateway with calculated value
        (bool success, bytes memory returnData) = INTENT_GATEWAY_V2.call(msg.data);

        if (!success) {
            // Bubble up the revert reason
            assembly {
                revert(add(returnData, 32), mload(returnData))
            }
        }

        // Return the result
        assembly {
            return(add(returnData, 32), mload(returnData))
        }
    }
}
