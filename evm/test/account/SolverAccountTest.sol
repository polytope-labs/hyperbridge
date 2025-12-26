// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Test.sol";
import {SolverAccount} from "../../src/utils/SolverAccount.sol";
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
            Params({host: address(0x1), dispatcher: address(0x2), solverSelection: true, surplusShareBps: 5000, protocolFeeBps: 0});
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
    // validateUserOp - Intent Solver Selection Tests (New Logic)
    // ============================================

    function test_ValidateUserOp_IntentSelection_Success() public {
        bytes32 userOpHash = keccak256("test_userop");

        // Create session signature (EIP-712 signature by session key)
        bytes memory sessionSignature = _createSessionKeySignature(testCommitment, address(solverAccount));

        // Create solver signature (Ethereum signed message over userOpHash, commitment, sessionKey)
        bytes memory solverSignature = _createSolverSignature(userOpHash, testCommitment, sessionKey);

        // Create combined signature: commitment + solverSignature + sessionSignature
        bytes memory signature = abi.encodePacked(testCommitment, solverSignature, sessionSignature);

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

        // Mock the IntentGateway.select call to return the sessionKey
        SelectOptions memory expectedOptions =
            SelectOptions({commitment: testCommitment, solver: address(solverAccount), signature: sessionSignature});
        bytes memory selectCalldata = abi.encodeWithSelector(intentGateway.select.selector, expectedOptions);

        vm.mockCall(address(intentGateway), selectCalldata, abi.encode(sessionKey));

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, userOpHash, 0);

        assertEq(result, ERC4337Utils.SIG_VALIDATION_SUCCESS);
    }

    function test_ValidateUserOp_IntentSelection_WrongSignatureLength_TooShort() public {
        bytes32 userOpHash = keccak256("test_userop");

        // Create signature that's too short (less than 162 bytes)
        bytes memory signature = new bytes(161);

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

        assertEq(result, ERC4337Utils.SIG_VALIDATION_FAILED);
    }

    function test_ValidateUserOp_IntentSelection_InvalidSessionSignature() public {
        bytes32 userOpHash = keccak256("test_userop");

        // Create invalid session signature (wrong signer)
        uint256 wrongPrivateKey = 0x9999999999999999;
        bytes32 structHash =
            keccak256(abi.encode(intentGateway.SELECT_SOLVER_TYPEHASH(), testCommitment, address(solverAccount)));
        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", intentGateway.DOMAIN_SEPARATOR(), structHash));
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(wrongPrivateKey, digest);
        bytes memory invalidSessionSignature = abi.encodePacked(r, s, v);

        // Create valid solver signature
        bytes memory solverSignature = _createSolverSignature(userOpHash, testCommitment, sessionKey);

        // Create combined signature
        bytes memory signature = abi.encodePacked(testCommitment, solverSignature, invalidSessionSignature);

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

        // Mock IntentGateway.select to fail (return empty or revert)
        SelectOptions memory expectedOptions = SelectOptions({
            commitment: testCommitment, solver: address(solverAccount), signature: invalidSessionSignature
        });
        bytes memory selectCalldata = abi.encodeWithSelector(intentGateway.select.selector, expectedOptions);

        vm.mockCallRevert(address(intentGateway), selectCalldata, "Invalid session signature");

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, userOpHash, 0);

        assertEq(result, ERC4337Utils.SIG_VALIDATION_FAILED);
    }

    function test_ValidateUserOp_IntentSelection_InvalidSolverSignature() public {
        bytes32 userOpHash = keccak256("test_userop");

        // Create valid session signature
        bytes memory sessionSignature = _createSessionKeySignature(testCommitment, address(solverAccount));

        // Create INVALID solver signature (wrong signer)
        uint256 wrongPrivateKey = 0x9999999999999999;
        bytes32 messageHash = keccak256(abi.encodePacked(userOpHash, testCommitment, sessionKey));
        bytes32 ethSignedMessageHash = keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", messageHash));
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(wrongPrivateKey, ethSignedMessageHash);
        bytes memory invalidSolverSignature = abi.encodePacked(r, s, v);

        // Create combined signature
        bytes memory signature = abi.encodePacked(testCommitment, invalidSolverSignature, sessionSignature);

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

        // Mock the IntentGateway.select call to return the sessionKey
        SelectOptions memory expectedOptions =
            SelectOptions({commitment: testCommitment, solver: address(solverAccount), signature: sessionSignature});
        bytes memory selectCalldata = abi.encodeWithSelector(intentGateway.select.selector, expectedOptions);

        vm.mockCall(address(intentGateway), selectCalldata, abi.encode(sessionKey));

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, userOpHash, 0);

        // Should fail because solver signature is invalid
        assertEq(result, ERC4337Utils.SIG_VALIDATION_FAILED);
    }

    function test_ValidateUserOp_IntentSelection_WrongCommitment() public {
        bytes32 userOpHash = keccak256("test_userop");
        bytes32 wrongCommitment = keccak256("wrong_commitment");

        // Create session signature for correct commitment
        bytes memory sessionSignature = _createSessionKeySignature(testCommitment, address(solverAccount));

        // Create solver signature for correct commitment
        bytes memory solverSignature = _createSolverSignature(userOpHash, testCommitment, sessionKey);

        // But include WRONG commitment in the signature bytes
        bytes memory signature = abi.encodePacked(wrongCommitment, solverSignature, sessionSignature);

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

        // Mock the IntentGateway.select call - it will be called with wrongCommitment
        SelectOptions memory expectedOptions =
            SelectOptions({commitment: wrongCommitment, solver: address(solverAccount), signature: sessionSignature});
        bytes memory selectCalldata = abi.encodeWithSelector(intentGateway.select.selector, expectedOptions);

        // This should fail because session signature was for testCommitment, not wrongCommitment
        vm.mockCallRevert(address(intentGateway), selectCalldata, "Invalid commitment");

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, userOpHash, 0);

        assertEq(result, ERC4337Utils.SIG_VALIDATION_FAILED);
    }

    function test_ValidateUserOp_IntentSelection_IntentGatewayReturnsInvalidData() public {
        bytes32 userOpHash = keccak256("test_userop");

        // Create valid signatures
        bytes memory sessionSignature = _createSessionKeySignature(testCommitment, address(solverAccount));
        bytes memory solverSignature = _createSolverSignature(userOpHash, testCommitment, sessionKey);

        // Create combined signature
        bytes memory signature = abi.encodePacked(testCommitment, solverSignature, sessionSignature);

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

        // Mock IntentGateway.select to return invalid data (less than 32 bytes)
        SelectOptions memory expectedOptions =
            SelectOptions({commitment: testCommitment, solver: address(solverAccount), signature: sessionSignature});
        bytes memory selectCalldata = abi.encodeWithSelector(intentGateway.select.selector, expectedOptions);

        vm.mockCall(address(intentGateway), selectCalldata, abi.encode(bytes("")));

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, userOpHash, 0);

        assertEq(result, ERC4337Utils.SIG_VALIDATION_FAILED);
    }

    function test_ValidateUserOp_IntentSelection_MultipleCommitments() public {
        bytes32 userOpHash1 = keccak256("test_userop_1");
        bytes32 commitment1 = keccak256("commitment_1");

        bytes32 userOpHash2 = keccak256("test_userop_2");
        bytes32 commitment2 = keccak256("commitment_2");

        // First operation with commitment1
        bytes memory sessionSignature1 = _createSessionKeySignature(commitment1, address(solverAccount));
        bytes memory solverSignature1 = _createSolverSignature(userOpHash1, commitment1, sessionKey);
        bytes memory signature1 = abi.encodePacked(commitment1, solverSignature1, sessionSignature1);

        PackedUserOperation memory op1 = PackedUserOperation({
            sender: address(solverAccount),
            nonce: 0,
            initCode: "",
            callData: "",
            accountGasLimits: bytes32(0),
            preVerificationGas: 0,
            gasFees: bytes32(0),
            paymasterAndData: "",
            signature: signature1
        });

        SelectOptions memory expectedOptions1 =
            SelectOptions({commitment: commitment1, solver: address(solverAccount), signature: sessionSignature1});
        bytes memory selectCalldata1 = abi.encodeWithSelector(intentGateway.select.selector, expectedOptions1);
        vm.mockCall(address(intentGateway), selectCalldata1, abi.encode(sessionKey));

        vm.prank(entryPoint);
        uint256 result1 = solverAccount.validateUserOp(op1, userOpHash1, 0);
        assertEq(result1, ERC4337Utils.SIG_VALIDATION_SUCCESS);

        // Second operation with commitment2
        bytes memory sessionSignature2 = _createSessionKeySignature(commitment2, address(solverAccount));
        bytes memory solverSignature2 = _createSolverSignature(userOpHash2, commitment2, sessionKey);
        bytes memory signature2 = abi.encodePacked(commitment2, solverSignature2, sessionSignature2);

        PackedUserOperation memory op2 = PackedUserOperation({
            sender: address(solverAccount),
            nonce: 1,
            initCode: "",
            callData: "",
            accountGasLimits: bytes32(0),
            preVerificationGas: 0,
            gasFees: bytes32(0),
            paymasterAndData: "",
            signature: signature2
        });

        SelectOptions memory expectedOptions2 =
            SelectOptions({commitment: commitment2, solver: address(solverAccount), signature: sessionSignature2});
        bytes memory selectCalldata2 = abi.encodeWithSelector(intentGateway.select.selector, expectedOptions2);
        vm.mockCall(address(intentGateway), selectCalldata2, abi.encode(sessionKey));

        vm.prank(entryPoint);
        uint256 result2 = solverAccount.validateUserOp(op2, userOpHash2, 0);
        assertEq(result2, ERC4337Utils.SIG_VALIDATION_SUCCESS);
    }

    function test_ValidateUserOp_IntentSelection_DifferentSessionKeys() public {
        bytes32 userOpHash = keccak256("test_userop");

        // Create second session key
        uint256 sessionKey2PrivateKey = 0xfedcba0987654321;
        address sessionKey2 = vm.addr(sessionKey2PrivateKey);

        // Create session signature with first session key
        bytes memory sessionSignature = _createSessionKeySignature(testCommitment, address(solverAccount));

        // Create solver signature expecting first session key to be returned
        bytes memory solverSignature = _createSolverSignature(userOpHash, testCommitment, sessionKey);

        // Create combined signature
        bytes memory signature = abi.encodePacked(testCommitment, solverSignature, sessionSignature);

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

        // Mock IntentGateway to return DIFFERENT session key
        SelectOptions memory expectedOptions =
            SelectOptions({commitment: testCommitment, solver: address(solverAccount), signature: sessionSignature});
        bytes memory selectCalldata = abi.encodeWithSelector(intentGateway.select.selector, expectedOptions);
        vm.mockCall(address(intentGateway), selectCalldata, abi.encode(sessionKey2));

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, userOpHash, 0);

        // Should fail because solver signature was for sessionKey but IntentGateway returned sessionKey2
        assertEq(result, ERC4337Utils.SIG_VALIDATION_FAILED);
    }

    // ============================================
    // Fallback Function Tests
    // ============================================

    function test_Fallback_ReceivesETH() public {
        uint256 balanceBefore = address(solverAccount).balance;

        (bool success,) = address(solverAccount).call{value: 1 ether}("");

        assertTrue(success);
        assertEq(address(solverAccount).balance, balanceBefore + 1 ether);
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

    /// @notice Creates an EIP-712 signature by session key for IntentGateway.select
    function _createSessionKeySignature(bytes32 commitment, address solverAddr) internal view returns (bytes memory) {
        bytes32 structHash = keccak256(abi.encode(intentGateway.SELECT_SOLVER_TYPEHASH(), commitment, solverAddr));
        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", intentGateway.DOMAIN_SEPARATOR(), structHash));
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(sessionKeyPrivateKey, digest);
        return abi.encodePacked(r, s, v);
    }

    /// @notice Creates an Ethereum signed message signature by solver over (userOpHash, commitment, sessionKey)
    function _createSolverSignature(bytes32 userOpHash, bytes32 commitment, address sessionKeyAddr)
        internal
        view
        returns (bytes memory)
    {
        bytes32 messageHash = keccak256(abi.encodePacked(userOpHash, commitment, sessionKeyAddr));
        bytes32 ethSignedMessageHash = keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", messageHash));
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(solverPrivateKey, ethSignedMessageHash);
        return abi.encodePacked(r, s, v);
    }
}
