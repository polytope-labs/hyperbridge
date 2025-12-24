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

import {PackedUserOperation} from "@openzeppelin/contracts/interfaces/draft-IERC4337.sol";
import {Account} from "@openzeppelin/contracts/account/Account.sol";
import {ERC4337Utils} from "@openzeppelin/contracts/account/utils/draft-ERC4337Utils.sol";
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";

import {SelectOptions, IIntentGatewayV2, Order, FillOptions, TokenInfo} from "./interfaces/IntentGatewayV2.sol";

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
contract IntentSolverAccount is Account {
    /**
     * @notice Standard length of an ECDSA signature (r: 32 bytes, s: 32 bytes, v: 1 byte)
     */
    uint256 private constant ECDSA_SIGNATURE_LENGTH = 65;

    /**
     * @notice Expected calldata length for select function (4 byte selector + 256 bytes data)
     */
    uint256 private constant SELECT_CALLDATA_LENGTH = 260;

    /**
     * @notice Cached select function selector
     */
    bytes4 private constant SELECT_SELECTOR = IIntentGatewayV2.select.selector;

    /**
     * @notice Cached fillOrder function selector
     */
    bytes4 private constant FILL_ORDER_SELECTOR = IIntentGatewayV2.fillOrder.selector;

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
     * @notice Cached SELECT_SOLVER_TYPEHASH from IntentGateway
     */
    bytes32 private immutable SELECT_SOLVER_TYPEHASH;

    /**
     * @notice Constructor for IntentSolverAccount
     * @param intentGatewayV2 The IntentGatewayV2 contract address
     * @dev The owner EOA will sign all operations on behalf of this solver account.
     *      The solver is identified by the deployed contract address (address(this)).
     *      Caches domain separator and type hash to save gas on validation.
     */
    constructor(address intentGatewayV2) {
        INTENT_GATEWAY_V2 = intentGatewayV2;
        DOMAIN_SEPARATOR = IIntentGatewayV2(intentGatewayV2).DOMAIN_SEPARATOR();
        SELECT_SOLVER_TYPEHASH = IIntentGatewayV2(intentGatewayV2).SELECT_SOLVER_TYPEHASH();
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
        (address sessionKey, bytes calldata solverSignature) = abi.decode(op.signature, (address, bytes));

        // Ensure calldata has exact length for select function
        if (op.callData.length != SELECT_CALLDATA_LENGTH) return ERC4337Utils.SIG_VALIDATION_FAILED;

        // Decode SelectOptions from calldata (skip 4-byte selector)
        SelectOptions memory options = abi.decode(op.callData[4:], (SelectOptions));
        if (options.solver != address(this)) return ERC4337Utils.SIG_VALIDATION_FAILED;

        // Use cached domain separator and type hash
        bytes32 structHash = keccak256(abi.encode(SELECT_SOLVER_TYPEHASH, options.commitment, sessionKey));
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
     * @notice Fallback function to forward select and fillOrder calls to IntentGateway
     * @dev Only allows calls to select(SelectOptions) and fillOrder(Order, FillOptions)
     *      All other calls will revert. This enables the solver account to interact
     *      with the IntentGateway through the ERC4337 EntryPoint.
     *      For fillOrder, calculates required native token amount from FillOptions.
     */
    fallback() external payable {
        bytes4 selector = msg.sig;
        uint256 value = 0;

        // Only allow select and fillOrder functions
        if (selector == SELECT_SELECTOR) {} else if (selector == FILL_ORDER_SELECTOR) {
            // Decode fillOrder parameters to calculate native token amount
            // fillOrder(Order calldata order, FillOptions calldata options)
            (, FillOptions memory options) = abi.decode(msg.data[4:], (Order, FillOptions));

            // Add native dispatch fee
            value += options.nativeDispatchFee;

            // Sum up all native token amounts (token == bytes32(0))
            for (uint256 i = 0; i < options.outputs.length; i++) {
                if (options.outputs[i].token == bytes32(0)) {
                    value += options.outputs[i].amount;
                }
            }
        } else {
            revert("Unsupported function");
        }

        // Forward the call to IntentGateway with calculated value
        (bool success, bytes memory returnData) = INTENT_GATEWAY_V2.call{value: value}(msg.data);

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
