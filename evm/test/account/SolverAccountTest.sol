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

    // Matches SolverAccount's SELECT_SOLVER_WITH_NONCE_TYPEHASH
    bytes32 public constant SELECT_SOLVER_WITH_NONCE_TYPEHASH =
        keccak256("SelectSolver(bytes32 commitment,address solver,uint256 nonce)");

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

        // Deploy SolverAccount at a temporary address to get bytecode
        SolverAccount tempAccount = new SolverAccount(address(intentGateway));

        // Simulate EIP-7702: Etch the SolverAccount bytecode at the solver EOA address
        vm.etch(solver, address(tempAccount).code);
        solverAccount = SolverAccount(payable(solver));

        // Fund solver account
        vm.deal(address(solverAccount), 10 ether);

        // Create test commitment
        testCommitment = keccak256("test_order_commitment");
    }

    // ============================================
    // Constructor Tests
    // ============================================

    function test_Constructor_SetsCachedValues() public view {
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

        // Sign with solver private key (in EIP-7702, the EOA IS the contract)
        // Sign the userOpHash directly without Ethereum signed message prefix
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(solverPrivateKey, userOpHash);
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

        // With EIP-7702 simulation, the signature should be valid
        assertEq(result, ERC4337Utils.SIG_VALIDATION_SUCCESS);
    }

    function test_ValidateUserOp_StandardECDSA_InvalidSigner_Fails() public {
        // Create a standard 65-byte ECDSA signature
        bytes32 userOpHash = keccak256("test_userop");

        // Sign with WRONG private key (not the solver)
        uint256 wrongPrivateKey = 0x9999999999999999;
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(wrongPrivateKey, userOpHash);
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

        // Should fail because signer doesn't match solver account (address(this))
        assertEq(result, ERC4337Utils.SIG_VALIDATION_FAILED);
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

        (bool success,) = address(solverAccount).call(callData);
        assertFalse(success, "Call should revert for unsupported function");
    }

    function test_Fallback_ReceivesETH() public {
        uint256 balanceBefore = address(solverAccount).balance;

        (bool success,) = address(solverAccount).call{value: 1 ether}("");

        assertTrue(success);
        assertEq(address(solverAccount).balance, balanceBefore + 1 ether);
    }

    // ============================================
    // Nonce and Replay Protection Tests
    // ============================================

    function test_ValidateUserOp_IntentSelection_WithCorrectNonce_Success() public {
        bytes memory sessionKeySig = _createSessionKeySignature(testCommitment, address(solverAccount));
        SelectOptions memory options =
            SelectOptions({commitment: testCommitment, solver: address(solverAccount), signature: sessionKeySig});

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

        assertEq(result, ERC4337Utils.SIG_VALIDATION_SUCCESS);
    }

    function test_ValidateUserOp_IntentSelection_WithWrongNonce_Fails() public {
        bytes memory sessionKeySig = _createSessionKeySignature(testCommitment, address(solverAccount));
        SelectOptions memory options =
            SelectOptions({commitment: testCommitment, solver: address(solverAccount), signature: sessionKeySig});

        bytes memory callData = abi.encodeWithSelector(intentGateway.select.selector, options);

        // Create signature with nonce = 0
        uint192 nonceKey = uint192(uint256(testCommitment) >> 64);
        uint256 wrongNonce = 5; // Use wrong nonce in signature

        bytes32 structHash =
            keccak256(abi.encode(SELECT_SOLVER_WITH_NONCE_TYPEHASH, testCommitment, sessionKey, wrongNonce));
        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", intentGateway.DOMAIN_SEPARATOR(), structHash));
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(solverPrivateKey, digest);
        bytes memory solverSig = abi.encodePacked(r, s, v);

        // Mock EntryPoint to return nonce = 0 (different from signature)
        vm.mockCall(
            entryPoint,
            abi.encodeWithSignature("getNonce(address,uint192)", address(solverAccount), nonceKey),
            abi.encode(0)
        );

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

    function test_ValidateUserOp_IntentSelection_SequentialNonces_Success() public {
        bytes memory sessionKeySig = _createSessionKeySignature(testCommitment, address(solverAccount));
        SelectOptions memory options =
            SelectOptions({commitment: testCommitment, solver: address(solverAccount), signature: sessionKeySig});

        bytes memory callData = abi.encodeWithSelector(intentGateway.select.selector, options);

        uint192 nonceKey = uint192(uint256(testCommitment) >> 64);

        // Test with nonce = 0
        uint256 nonce0 = 0;
        vm.mockCall(
            entryPoint,
            abi.encodeWithSignature("getNonce(address,uint192)", address(solverAccount), nonceKey),
            abi.encode(nonce0)
        );

        bytes32 structHash0 =
            keccak256(abi.encode(SELECT_SOLVER_WITH_NONCE_TYPEHASH, testCommitment, sessionKey, nonce0));
        bytes32 digest0 = keccak256(abi.encodePacked("\x19\x01", intentGateway.DOMAIN_SEPARATOR(), structHash0));
        (uint8 v0, bytes32 r0, bytes32 s0) = vm.sign(solverPrivateKey, digest0);
        bytes memory solverSig0 = abi.encodePacked(r0, s0, v0);
        bytes memory signature0 = abi.encodePacked(sessionKey, solverSig0);

        PackedUserOperation memory op0 = PackedUserOperation({
            sender: address(solverAccount),
            nonce: 0,
            initCode: "",
            callData: callData,
            accountGasLimits: bytes32(0),
            preVerificationGas: 0,
            gasFees: bytes32(0),
            paymasterAndData: "",
            signature: signature0
        });

        vm.prank(entryPoint);
        uint256 result0 = solverAccount.validateUserOp(op0, bytes32(0), 0);
        assertEq(result0, ERC4337Utils.SIG_VALIDATION_SUCCESS);

        // Test with nonce = 1 (simulating next operation)
        uint256 nonce1 = 1;
        vm.mockCall(
            entryPoint,
            abi.encodeWithSignature("getNonce(address,uint192)", address(solverAccount), nonceKey),
            abi.encode(nonce1)
        );

        bytes32 structHash1 =
            keccak256(abi.encode(SELECT_SOLVER_WITH_NONCE_TYPEHASH, testCommitment, sessionKey, nonce1));
        bytes32 digest1 = keccak256(abi.encodePacked("\x19\x01", intentGateway.DOMAIN_SEPARATOR(), structHash1));
        (uint8 v1, bytes32 r1, bytes32 s1) = vm.sign(solverPrivateKey, digest1);
        bytes memory solverSig1 = abi.encodePacked(r1, s1, v1);
        bytes memory signature1 = abi.encodePacked(sessionKey, solverSig1);

        PackedUserOperation memory op1 = PackedUserOperation({
            sender: address(solverAccount),
            nonce: 1,
            initCode: "",
            callData: callData,
            accountGasLimits: bytes32(0),
            preVerificationGas: 0,
            gasFees: bytes32(0),
            paymasterAndData: "",
            signature: signature1
        });

        vm.prank(entryPoint);
        uint256 result1 = solverAccount.validateUserOp(op1, bytes32(0), 0);
        assertEq(result1, ERC4337Utils.SIG_VALIDATION_SUCCESS);

        // Test that old signature (nonce=0) fails with current nonce (nonce=1)
        vm.mockCall(
            entryPoint,
            abi.encodeWithSignature("getNonce(address,uint192)", address(solverAccount), nonceKey),
            abi.encode(nonce1)
        );

        PackedUserOperation memory opReplay = PackedUserOperation({
            sender: address(solverAccount),
            nonce: 1,
            initCode: "",
            callData: callData,
            accountGasLimits: bytes32(0),
            preVerificationGas: 0,
            gasFees: bytes32(0),
            paymasterAndData: "",
            signature: signature0 // Using old signature with nonce=0
        });

        vm.prank(entryPoint);
        uint256 resultReplay = solverAccount.validateUserOp(opReplay, bytes32(0), 0);
        assertEq(resultReplay, ERC4337Utils.SIG_VALIDATION_FAILED);
    }

    function test_ValidateUserOp_IntentSelection_DifferentCommitments_DifferentNonceKeys() public {
        // Create two different commitments
        bytes32 commitment1 = keccak256("test_order_1");
        bytes32 commitment2 = keccak256("test_order_2");

        // Calculate nonce keys for both commitments
        uint192 nonceKey1 = uint192(uint256(commitment1) >> 64);
        uint192 nonceKey2 = uint192(uint256(commitment2) >> 64);

        // Verify they're different (should be different unless extremely unlikely collision)
        assertTrue(nonceKey1 != nonceKey2, "Nonce keys should be different for different commitments");

        // Both can use nonce = 0 because they have different nonce keys
        uint256 nonce = 0;

        // Setup for commitment1
        bytes memory sessionKeySig1 = _createSessionKeySignature(commitment1, address(solverAccount));
        SelectOptions memory options1 =
            SelectOptions({commitment: commitment1, solver: address(solverAccount), signature: sessionKeySig1});
        bytes memory callData1 = abi.encodeWithSelector(intentGateway.select.selector, options1);

        vm.mockCall(
            entryPoint,
            abi.encodeWithSignature("getNonce(address,uint192)", address(solverAccount), nonceKey1),
            abi.encode(nonce)
        );

        bytes32 structHash1 = keccak256(abi.encode(SELECT_SOLVER_WITH_NONCE_TYPEHASH, commitment1, sessionKey, nonce));
        bytes32 digest1 = keccak256(abi.encodePacked("\x19\x01", intentGateway.DOMAIN_SEPARATOR(), structHash1));
        (uint8 v1, bytes32 r1, bytes32 s1) = vm.sign(solverPrivateKey, digest1);
        bytes memory signature1 = abi.encodePacked(sessionKey, abi.encodePacked(r1, s1, v1));

        PackedUserOperation memory op1 = PackedUserOperation({
            sender: address(solverAccount),
            nonce: 0,
            initCode: "",
            callData: callData1,
            accountGasLimits: bytes32(0),
            preVerificationGas: 0,
            gasFees: bytes32(0),
            paymasterAndData: "",
            signature: signature1
        });

        vm.prank(entryPoint);
        uint256 result1 = solverAccount.validateUserOp(op1, bytes32(0), 0);
        assertEq(result1, ERC4337Utils.SIG_VALIDATION_SUCCESS, "First commitment should succeed");

        // Setup for commitment2 (also using nonce = 0, but different nonce key)
        bytes memory sessionKeySig2 = _createSessionKeySignature(commitment2, address(solverAccount));
        SelectOptions memory options2 =
            SelectOptions({commitment: commitment2, solver: address(solverAccount), signature: sessionKeySig2});
        bytes memory callData2 = abi.encodeWithSelector(intentGateway.select.selector, options2);

        vm.mockCall(
            entryPoint,
            abi.encodeWithSignature("getNonce(address,uint192)", address(solverAccount), nonceKey2),
            abi.encode(nonce)
        );

        bytes32 structHash2 = keccak256(abi.encode(SELECT_SOLVER_WITH_NONCE_TYPEHASH, commitment2, sessionKey, nonce));
        bytes32 digest2 = keccak256(abi.encodePacked("\x19\x01", intentGateway.DOMAIN_SEPARATOR(), structHash2));
        (uint8 v2, bytes32 r2, bytes32 s2) = vm.sign(solverPrivateKey, digest2);
        bytes memory signature2 = abi.encodePacked(sessionKey, abi.encodePacked(r2, s2, v2));

        PackedUserOperation memory op2 = PackedUserOperation({
            sender: address(solverAccount),
            nonce: 0,
            initCode: "",
            callData: callData2,
            accountGasLimits: bytes32(0),
            preVerificationGas: 0,
            gasFees: bytes32(0),
            paymasterAndData: "",
            signature: signature2
        });

        vm.prank(entryPoint);
        uint256 result2 = solverAccount.validateUserOp(op2, bytes32(0), 0);
        assertEq(result2, ERC4337Utils.SIG_VALIDATION_SUCCESS, "Second commitment should also succeed with nonce=0");
    }

    // ============================================
    // ERC-7821 Tests
    // ============================================

    function test_ERC7821_SupportsExecutionMode() public view {
        bytes32 mode = bytes32(uint256(0x01) << 248); // Simple mode
        bool supported = solverAccount.supportsExecutionMode(mode);
        assertTrue(supported);
    }

    // ============================================
    // Helper Functions
    // ============================================

    function _createSessionKeySignature(bytes32 commitment, address solverAddr) internal view returns (bytes memory) {
        bytes32 structHash = keccak256(abi.encode(intentGateway.SELECT_SOLVER_TYPEHASH(), commitment, solverAddr));

        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", intentGateway.DOMAIN_SEPARATOR(), structHash));

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(sessionKeyPrivateKey, digest);
        return abi.encodePacked(r, s, v);
    }

    function _createSolverSignature(bytes32 commitment, address sessionKeyAddr) internal returns (bytes memory) {
        // Calculate nonce key (first 192 bits of commitment)
        uint192 nonceKey = uint192(uint256(commitment) >> 64);

        // Mock EntryPoint's getNonce call
        // Note: In real tests with a deployed EntryPoint, this would return the actual nonce
        // For this test, we assume nonce = 0 for the first operation
        uint256 nonce = 0;
        vm.mockCall(
            entryPoint,
            abi.encodeWithSignature("getNonce(address,uint192)", address(solverAccount), nonceKey),
            abi.encode(nonce)
        );

        // Create signature with nonce for replay protection
        bytes32 structHash = keccak256(abi.encode(SELECT_SOLVER_WITH_NONCE_TYPEHASH, commitment, sessionKeyAddr, nonce));

        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", intentGateway.DOMAIN_SEPARATOR(), structHash));

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(solverPrivateKey, digest);
        return abi.encodePacked(r, s, v);
    }
}
