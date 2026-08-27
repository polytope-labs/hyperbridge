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
import {Execution} from "@openzeppelin/contracts/interfaces/draft-IERC7579.sol";
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {IERC1271} from "@openzeppelin/contracts/interfaces/IERC1271.sol";

import {SelectOptions, IIntentGatewayV2} from "@hyperbridge/core/apps/IntentGatewayV2.sol";

/**
 * @title SolverAccount
 * @notice ERC-4337 and ERC-7821 compliant smart contract account for solvers
 * @dev This contract extends OpenZeppelin's Account and ERC7821 implementations and integrates with the IntentGateway
 *      to enable solver delegation primarily for solver selection. Solvers can delegate to this smart
 *      contract account using EIP-7702.
 * @author Polytope Labs
 */
contract SolverAccount is Account, ERC7821, IERC1271 {
    /**
     * @notice Standard length of an ECDSA signature (r: 32 bytes, s: 32 bytes, v: 1 byte)
     */
    uint256 private constant ECDSA_SIGNATURE_LENGTH = 65;

    /**
     * @notice Expected signature length for intent solver selection
     * @dev abi.encodePacked(commitment, validUntil, solverSignature, sessionSignature)
     *      = 32 + 6 + 65 + 65 = 168 bytes
     */
    uint256 private constant INTENT_SELECT_SIGNATURE_LENGTH = 168;

    /**
     * @notice EIP-712 type hash for the expiry the solver signs alongside the operation
     * @dev The bid's `validUntil` cannot live in an unsigned part of the operation: the
     *      EntryPoint's `userOpHash` does not cover `op.signature`, so an expiry carried
     *      there and nowhere else could simply be rewritten by whoever replays the bid.
     *      Binding it into the digest the solver signs is what makes it tamper-evident.
     */
    bytes32 private constant BID_VALIDITY_TYPEHASH = keccak256("BidValidity(bytes32 userOpHash,uint48 validUntil)");

    /**
     * @notice EIP-712 domain type hash
     */
    bytes32 private constant EIP712_DOMAIN_TYPEHASH =
        keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)");

    /**
     * @notice Hashed EIP-712 domain name for the bid-validity digest
     */
    bytes32 private constant BID_DOMAIN_NAME = keccak256("SolverAccount");

    /**
     * @notice Hashed EIP-712 domain version for the bid-validity digest
     */
    bytes32 private constant BID_DOMAIN_VERSION = keccak256("1");

    /**
     * @notice Cached select function selector
     */
    bytes4 private constant SELECT_SELECTOR = IIntentGatewayV2.select.selector;

    /**
     * @notice Cached fillOrder function selector
     */
    bytes4 private constant FILL_ORDER_SELECTOR = IIntentGatewayV2.fillOrder.selector;

    /**
     * @notice Cached ERC-7821 execute function selector
     */
    bytes4 private constant EXECUTE_SELECTOR = ERC7821.execute.selector;

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
     * @dev Two modes, discriminated by signature length:
     *
     * 1. Standard ECDSA (65 bytes): validated by the Account base contract against
     *    the plain userOpHash. Refused if the calldata contains a fillOrder call to
     *    the IntentGateway: bids are public and embed a valid 65-byte solver signature
     *    over the userOpHash, so anyone could strip the commitment and session
     *    signature from a bid and submit the op on this path. Without a select()
     *    staged during validation the fill reverts, but the bid's nonce would be
     *    consumed and the solver griefed of the gas fees.
     * 2. Intent solver selection (168 bytes): abi.encodePacked(commitment, validUntil,
     *    solverSignature, sessionSignature). The solver signs an EIP-712 BidValidity
     *    digest over (userOpHash, validUntil) rather than the plain userOpHash, and the
     *    userOp's nonce key must equal the lower 192 bits of
     *    keccak256(abi.encodePacked(commitment, sessionKey)) — binding the operation
     *    to the order and the session key it was bid against, so neither can be
     *    swapped after signing.
     *
     *    `validUntil` is returned to the EntryPoint as a validity range, so a bid stops
     *    being executable once it lapses. Without it a signed bid was valid forever: the
     *    order's `deadline` is chosen by the placer with no upper bound, and neither
     *    retracting the bid on Hyperbridge nor letting it go unselected invalidates
     *    anything on this chain — leaving the placer holding a free, unexpiring option
     *    to make the solver fill at a stale price.
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
            if (_containsFillOrder(op.callData)) return ERC4337Utils.SIG_VALIDATION_FAILED;
            return super.validateUserOp(op, userOpHash, missingAccountFunds);
        }

        // Expected format: abi.encodePacked(commitment, validUntil, solverSignature, sessionSignature)
        // commitment: 32 bytes, validUntil: 6 bytes, solverSignature: 65 bytes, sessionSignature: 65 bytes.
        // Exact-length (not >=): the layout is fixed, so trailing bytes can only be malleability.
        if (op.signature.length != INTENT_SELECT_SIGNATURE_LENGTH) return ERC4337Utils.SIG_VALIDATION_FAILED;

        bytes32 commitment = bytes32(op.signature[0:32]);
        uint48 validUntil = uint48(bytes6(op.signature[32:38]));
        bytes calldata solverSignature = op.signature[38:103];
        bytes calldata sessionSignature = op.signature[103:168];

        // A zero validUntil is not "no opinion", it is unbounded: ERC4337Utils.parseValidationData
        // expands 0 to BLOCK_RANGE_MASK. Refusing it here is what stops the old never-expiring
        // behaviour from being reachable simply by signing an empty expiry.
        if (validUntil == 0) return ERC4337Utils.SIG_VALIDATION_FAILED;

        // Call IntentGatewayV2.select to recover the sessionKey. This also stages the
        // transient-storage selection that fillOrder enforces at execution.
        SelectOptions memory selectOptions =
            SelectOptions({commitment: commitment, solver: address(this), signature: sessionSignature});
        bytes memory selectCalldata = abi.encodeWithSelector(SELECT_SELECTOR, selectOptions);
        (bool success, bytes memory returnData) = INTENT_GATEWAY_V2.call(selectCalldata);

        if (!success || returnData.length < 32) return ERC4337Utils.SIG_VALIDATION_FAILED;

        address sessionKey = abi.decode(returnData, (address));
        uint192 userOpNonce = uint192(uint256(keccak256(abi.encodePacked(commitment, sessionKey))));
        if (uint192(op.nonce >> 64) != userOpNonce) return ERC4337Utils.SIG_VALIDATION_FAILED;
        if (!_rawSignatureValidation(_bidValidityDigest(userOpHash, validUntil), solverSignature)) {
            return ERC4337Utils.SIG_VALIDATION_FAILED;
        }

        // Pay for gas if needed
        _payPrefund(missingAccountFunds);

        // validAfter = 0 keeps the BLOCK_RANGE_FLAG clear, so this packs as a TIMESTAMP range —
        // the only range the canonical EntryPoint interprets.
        return ERC4337Utils.packValidationData(address(0), 0, validUntil);
    }

    /**
     * @notice EIP-712 digest binding an operation hash to the expiry the solver agreed to
     * @dev The domain's `verifyingContract` is `address(this)`, which under EIP-7702 is the
     *      solver's own EOA — so a bid signed for one solver account cannot be replayed against
     *      another. The digest is recomputed here rather than trusting a value passed in calldata
     *      because only the solver's signature over it makes the expiry unforgeable.
     *
     *      Note there is deliberately no upper bound on `validUntil` in this contract: ERC-7562
     *      forbids the TIMESTAMP opcode during validation, so the account cannot compare the
     *      expiry against the current time. Capping the tenor is the signer's job — see
     *      `maxBidTenorSec` on the filler side.
     * @param userOpHash The EntryPoint-supplied hash of the operation
     * @param validUntil Unix timestamp after which the bid is no longer valid
     * @return The EIP-712 digest the solver is expected to have signed
     */
    function _bidValidityDigest(bytes32 userOpHash, uint48 validUntil) internal view returns (bytes32) {
        bytes32 domainSeparator = keccak256(
            abi.encode(EIP712_DOMAIN_TYPEHASH, BID_DOMAIN_NAME, BID_DOMAIN_VERSION, block.chainid, address(this))
        );
        bytes32 structHash = keccak256(abi.encode(BID_VALIDITY_TYPEHASH, userOpHash, validUntil));
        return keccak256(abi.encodePacked(hex"1901", domainSeparator, structHash));
    }

    /**
     * @notice Scans userOp calldata for a call to IntentGatewayV2.fillOrder
     * @dev The calldata is covered by the solver's signature over the userOpHash, so a
     *      replayed bid cannot be reshaped to hide the call — the scan only needs to
     *      recognize the bid's ERC-7821 execute(mode, executionData) batch. abi.decode
     *      reverts on malformed calldata, rejecting the op during validation just as
     *      execution would.
     * @param callData The userOp calldata to scan
     * @return bool True if the calldata contains a fillOrder call to the IntentGateway
     */
    function _containsFillOrder(bytes calldata callData) private view returns (bool) {
        if (callData.length < 4 || bytes4(callData[0:4]) != EXECUTE_SELECTOR) return false;

        (, bytes memory executionData) = abi.decode(callData[4:], (bytes32, bytes));
        Execution[] memory calls = abi.decode(executionData, (Execution[]));

        for (uint256 i = 0; i < calls.length; i++) {
            bool hasFillOrder = calls[i].target == INTENT_GATEWAY_V2 && bytes4(calls[i].callData) == FILL_ORDER_SELECTOR;
            if (hasFillOrder) return true;
        }
        return false;
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
     * @notice ERC-1271 signature validation for EIP-7702 delegated accounts.
     * @dev Required so that protocols using OpenZeppelin's SignatureChecker (e.g. USDC's
     *      EIP-2612 permit) can verify signatures from this account. Under EIP-7702 the
     *      account has code, so SignatureChecker takes the ERC-1271 path instead of
     *      ecrecover. Delegates to {_rawSignatureValidation} which performs ECDSA recovery
     *      and checks that the recovered address equals address(this) (the delegating EOA).
     */
    function isValidSignature(bytes32 hash, bytes calldata signature) external view override returns (bytes4) {
        return _rawSignatureValidation(hash, signature) ? bytes4(0x1626ba7e) : bytes4(0xffffffff);
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
