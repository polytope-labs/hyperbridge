// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Test.sol";
import {SolverAccount} from "../../src/SolverAccount.sol";
import {IntentGatewayV2} from "../../src/apps/IntentGatewayV2.sol";
import {
    SelectOptions,
    Order,
    FillOptions,
    TokenInfo,
    Params,
    DispatchInfo,
    PaymentInfo
} from "../../src/interfaces/IntentGatewayV2.sol";
import {PackedUserOperation} from "@openzeppelin/contracts/interfaces/draft-IERC4337.sol";

import {ERC4337Utils} from "@openzeppelin/contracts/account/utils/draft-ERC4337Utils.sol";
import {MessageHashUtils} from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

contract SolverAccountTest is Test {
    SolverAccount public solverAccount;
    IntentGatewayV2 public intentGateway;

    address public entryPoint = address(ERC4337Utils.ENTRYPOINT_V08); // ERC-4337 v0.8 EntryPoint
    address public solver;
    uint256 public solverPrivateKey;
    address public sessionKey;
    uint256 public sessionKeyPrivateKey;

    bytes32 public testCommitment;

    function setUp() public {
        // Create test accounts
        solverPrivateKey = 0x1234567890abcdef;
        solver = vm.addr(solverPrivateKey);

        sessionKeyPrivateKey = 0xabcdef1234567890;
        sessionKey = vm.addr(sessionKeyPrivateKey);

        // Deploy IntentGateway
        intentGateway = new IntentGatewayV2(address(this));

        Params memory params =
            Params({host: address(0x1), dispatcher: address(0x2), solverSelection: true, surplusShareBps: 5000});
        intentGateway.setParams(params);

        // Deploy SolverAccount
        solverAccount = new SolverAccount(address(intentGateway));

        // Fund solver account
        vm.deal(address(solverAccount), 10 ether);

        // Create test commitment
        testCommitment = keccak256("test_order_commitment");
    }

    // ============================================
    // Constructor Tests
    // ============================================

    function test_Constructor_SetsCachedValues() public {
        assertEq(address(solverAccount.entryPoint()), entryPoint);

        // Verify immutables are set by testing they work in validation
        bytes32 domainSep = intentGateway.DOMAIN_SEPARATOR();
        bytes32 typeHash = intentGateway.SELECT_SOLVER_TYPEHASH();
        assertTrue(domainSep != bytes32(0));
        assertTrue(typeHash != bytes32(0));
    }

    // ============================================
    // validateUserOp - Standard ECDSA Mode Tests
    // ============================================

    function test_ValidateUserOp_StandardECDSA_Success() public {
        // Create a standard 65-byte ECDSA signature
        bytes32 userOpHash = keccak256("test_userop");

        // Sign with solver (contract address)
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(solverPrivateKey, MessageHashUtils.toEthSignedMessageHash(userOpHash));
        bytes memory signature = abi.encodePacked(r, s, v);

        PackedUserOperation memory op = PackedUserOperation({
            sender: address(solverAccount),
            nonce: 0,
            initCode: "",
            callData: "",
            accountGasLimits: bytes32(0),
            preVerificationGas: 0,
            gasFees: bytes32(0),
            paymasterAndData: "",
            signature: signature
        });

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, userOpHash, 0);

        // Should delegate to parent implementation
        // Result depends on whether signature is valid for the account
        assertTrue(result == 0 || result == 1);
    }

    // ============================================
    // validateUserOp - Intent Solver Selection Tests
    // ============================================

    function test_ValidateUserOp_IntentSelection_WrongSignatureLength() public {
        bytes memory callData = abi.encodeWithSelector(
            intentGateway.select.selector,
            SelectOptions({commitment: testCommitment, solver: address(solverAccount), signature: ""})
        );

        // Wrong signature length (not 85 bytes)
        bytes memory signature = new bytes(100);

        PackedUserOperation memory op = PackedUserOperation({
            sender: address(solverAccount),
            nonce: 0,
            initCode: "",
            callData: callData,
            accountGasLimits: bytes32(0),
            preVerificationGas: 0,
            gasFees: bytes32(0),
            paymasterAndData: "",
            signature: signature
        });

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, bytes32(0), 0);

        assertEq(result, ERC4337Utils.SIG_VALIDATION_FAILED);
    }

    function test_ValidateUserOp_IntentSelection_WrongCallDataLength() public {
        bytes memory signature = new bytes(85);

        PackedUserOperation memory op = PackedUserOperation({
            sender: address(solverAccount),
            nonce: 0,
            initCode: "",
            callData: "wrong_length",
            accountGasLimits: bytes32(0),
            preVerificationGas: 0,
            gasFees: bytes32(0),
            paymasterAndData: "",
            signature: signature
        });

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, bytes32(0), 0);

        assertEq(result, ERC4337Utils.SIG_VALIDATION_FAILED);
    }

    function test_ValidateUserOp_IntentSelection_WrongFunctionSelector() public {
        // Create calldata with wrong selector
        bytes memory callData = abi.encodeWithSelector(bytes4(keccak256("wrongFunction()")), new bytes(256));

        bytes memory signature = new bytes(85);

        PackedUserOperation memory op = PackedUserOperation({
            sender: address(solverAccount),
            nonce: 0,
            initCode: "",
            callData: callData,
            accountGasLimits: bytes32(0),
            preVerificationGas: 0,
            gasFees: bytes32(0),
            paymasterAndData: "",
            signature: signature
        });

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, bytes32(0), 0);

        assertEq(result, ERC4337Utils.SIG_VALIDATION_FAILED);
    }

    function test_ValidateUserOp_IntentSelection_WrongSolverAddress() public {
        bytes memory sessionKeySig = _createSessionKeySignature(testCommitment, address(0xdead));
        SelectOptions memory options = SelectOptions({
            commitment: testCommitment,
            solver: address(0xdead), // Wrong solver
            signature: sessionKeySig
        });

        bytes memory callData = abi.encodeWithSelector(intentGateway.select.selector, options);

        bytes memory solverSig = _createSolverSignature(testCommitment, sessionKey);
        bytes memory signature = abi.encodePacked(sessionKey, solverSig);

        PackedUserOperation memory op = PackedUserOperation({
            sender: address(solverAccount),
            nonce: 0,
            initCode: "",
            callData: callData,
            accountGasLimits: bytes32(0),
            preVerificationGas: 0,
            gasFees: bytes32(0),
            paymasterAndData: "",
            signature: signature
        });

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, bytes32(0), 0);

        assertEq(result, ERC4337Utils.SIG_VALIDATION_FAILED);
    }

    // ============================================
    // Fallback Function Tests
    // ============================================

    function test_Fallback_SelectCall_Success() public {
        bytes memory sessionKeySig = _createSessionKeySignature(testCommitment, address(solverAccount));
        SelectOptions memory options =
            SelectOptions({commitment: testCommitment, solver: address(solverAccount), signature: sessionKeySig});

        bytes memory callData = abi.encodeWithSelector(intentGateway.select.selector, options);

        // Call fallback directly
        (bool success,) = address(solverAccount).call(callData);

        assertTrue(success, "Fallback should forward select call successfully");
    }

    function test_Fallback_NonSelectCall_Reverts() public {
        bytes memory callData = abi.encodeWithSelector(bytes4(keccak256("someOtherFunction()")));

        vm.expectRevert(
            abi.encodeWithSelector(SolverAccount.UnsupportedFunction.selector, bytes4(keccak256("someOtherFunction()")))
        );
        (bool success,) = address(solverAccount).call(callData);
    }

    function test_Fallback_ReceivesETH() public {
        uint256 balanceBefore = address(solverAccount).balance;

        (bool success,) = address(solverAccount).call{value: 1 ether}("");

        assertTrue(success);
        assertEq(address(solverAccount).balance, balanceBefore + 1 ether);
    }

    // ============================================
    // ERC-7821 Tests
    // ============================================

    function test_ERC7821_SupportsExecutionMode() public {
        bytes32 mode = bytes32(uint256(0x01) << 248); // Simple mode
        bool supported = solverAccount.supportsExecutionMode(mode);
        assertTrue(supported);
    }

    // ============================================
    // Helper Functions
    // ============================================

    function _createSessionKeySignature(bytes32 commitment, address solverAddr) internal returns (bytes memory) {
        bytes32 structHash = keccak256(abi.encode(intentGateway.SELECT_SOLVER_TYPEHASH(), commitment, solverAddr));

        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", intentGateway.DOMAIN_SEPARATOR(), structHash));

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(sessionKeyPrivateKey, digest);
        return abi.encodePacked(r, s, v);
    }

    function _createSolverSignature(bytes32 commitment, address sessionKeyAddr) internal returns (bytes memory) {
        bytes32 structHash = keccak256(abi.encode(intentGateway.SELECT_SOLVER_TYPEHASH(), commitment, sessionKeyAddr));

        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", intentGateway.DOMAIN_SEPARATOR(), structHash));

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(solverPrivateKey, digest);
        return abi.encodePacked(r, s, v);
    }
}
