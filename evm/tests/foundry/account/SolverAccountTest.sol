// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Test.sol";
import {SolverAccount} from "../../../src/apps/intentsv2/SolverAccount.sol";
import {IntentGatewayV2} from "../../../src/apps/IntentGatewayV2.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {
    SelectOptions,
    Order,
    FillOptions,
    TokenInfo,
    Params,
    DispatchInfo,
    PaymentInfo,
    Deployment
} from "@hyperbridge/core/apps/IntentGatewayV2.sol";
import {PackedUserOperation} from "@openzeppelin/contracts/interfaces/draft-IERC4337.sol";
import {Execution} from "@openzeppelin/contracts/interfaces/draft-IERC7579.sol";

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

    function _deployGatewayProxy() internal returns (IntentGatewayV2) {
        IntentGatewayV2 implementation = new IntentGatewayV2(address(this));
        ERC1967Proxy proxy = new ERC1967Proxy(address(implementation), "");
        return IntentGatewayV2(payable(address(proxy)));
    }

    function setUp() public {
        // Create test accounts
        solverPrivateKey = 0x1234567890abcdef;
        solver = vm.addr(solverPrivateKey);

        sessionKeyPrivateKey = 0xabcdef1234567890;
        sessionKey = vm.addr(sessionKeyPrivateKey);

        // Deploy IntentGateway
        intentGateway = _deployGatewayProxy();

        Params memory params = Params({
            host: address(new MockContract()),
            dispatcher: address(new MockContract()),
            solverSelection: true,
            surplusShareBps: 5000,
            protocolFeeBps: 0,
            priceOracle: address(0)
        });
        intentGateway.initialize(params, new bytes[](0));

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

    /// @dev The fast path must refuse fillOrder calldata: bids are public and embed a
    ///      valid 65-byte solver signature over the userOpHash, so anyone could strip
    ///      the commitment and session signature from a bid and submit the op with it.
    ///      The fill would revert (no selection staged during validation), but the
    ///      bid's nonce would be consumed and the solver griefed of the gas fees.
    function test_ValidateUserOp_StandardECDSA_FillOrderCalldata_Fails() public {
        bytes32 userOpHash = keccak256("test_userop");

        Execution[] memory calls = new Execution[](1);
        calls[0] = Execution({
            target: address(intentGateway),
            value: 0,
            callData: abi.encodeWithSelector(intentGateway.fillOrder.selector)
        });

        PackedUserOperation memory op = _standardOp(_executeCalldata(calls), _signUserOpHash(userOpHash));

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, userOpHash, 0);

        assertEq(result, ERC4337Utils.SIG_VALIDATION_FAILED);
    }

    function test_ValidateUserOp_StandardECDSA_NonFillOrderBatch_Success() public {
        bytes32 userOpHash = keccak256("test_userop");

        // approve + a non-fillOrder gateway call — neither disables the fast path
        Execution[] memory calls = new Execution[](2);
        calls[0] = Execution({
            target: address(0xBEEF),
            value: 0,
            callData: abi.encodeWithSignature("approve(address,uint256)", address(intentGateway), 1 ether)
        });
        calls[1] = Execution({
            target: address(intentGateway),
            value: 0,
            callData: abi.encodeWithSelector(intentGateway.cancelOrder.selector)
        });

        PackedUserOperation memory op = _standardOp(_executeCalldata(calls), _signUserOpHash(userOpHash));

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, userOpHash, 0);

        assertEq(result, ERC4337Utils.SIG_VALIDATION_SUCCESS);
    }

    /// @dev The fillOrder selector on a target other than the IntentGateway is harmless.
    function test_ValidateUserOp_StandardECDSA_FillOrderSelectorWrongTarget_Success() public {
        bytes32 userOpHash = keccak256("test_userop");

        Execution[] memory calls = new Execution[](1);
        calls[0] = Execution({
            target: address(0xBEEF),
            value: 0,
            callData: abi.encodeWithSelector(intentGateway.fillOrder.selector)
        });

        PackedUserOperation memory op = _standardOp(_executeCalldata(calls), _signUserOpHash(userOpHash));

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, userOpHash, 0);

        assertEq(result, ERC4337Utils.SIG_VALIDATION_SUCCESS);
    }

    // ============================================
    // validateUserOp - Intent Solver Selection Tests (New Logic)
    // ============================================

    function test_ValidateUserOp_IntentSelection_PlainUserOpHash_Success() public {
        bytes32 userOpHash = keccak256("test_userop");

        // Create session signature (EIP-712 signature by session key)
        bytes memory sessionSignature = _createSessionKeySignature(testCommitment, address(solverAccount));

        // Solver signs the plain userOpHash — the order/session binding is carried
        // by the nonce key.
        bytes memory solverSignature = _signUserOpHash(userOpHash);

        bytes memory signature = abi.encodePacked(testCommitment, solverSignature, sessionSignature);

        PackedUserOperation memory op = PackedUserOperation({
            sender: address(solverAccount),
            nonce: _bidNonce(testCommitment, sessionKey),
            initCode: "",
            callData: "",
            accountGasLimits: bytes32(0),
            preVerificationGas: 0,
            gasFees: bytes32(0),
            paymasterAndData: "",
            signature: signature
        });

        SelectOptions memory expectedOptions =
            SelectOptions({commitment: testCommitment, solver: address(solverAccount), signature: sessionSignature});
        bytes memory selectCalldata = abi.encodeWithSelector(intentGateway.select.selector, expectedOptions);
        vm.mockCall(address(intentGateway), selectCalldata, abi.encode(sessionKey));

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, userOpHash, 0);

        assertEq(result, ERC4337Utils.SIG_VALIDATION_SUCCESS);
    }

    /// @dev fillOrder calldata is the normal payload for a selected bid — it must only
    ///      be refused on the fast path, not here.
    function test_ValidateUserOp_IntentSelection_FillOrderCalldata_Success() public {
        bytes32 userOpHash = keccak256("test_userop");

        bytes memory sessionSignature = _createSessionKeySignature(testCommitment, address(solverAccount));
        bytes memory solverSignature = _signUserOpHash(userOpHash);
        bytes memory signature = abi.encodePacked(testCommitment, solverSignature, sessionSignature);

        Execution[] memory calls = new Execution[](1);
        calls[0] = Execution({
            target: address(intentGateway),
            value: 0,
            callData: abi.encodeWithSelector(intentGateway.fillOrder.selector)
        });

        PackedUserOperation memory op = PackedUserOperation({
            sender: address(solverAccount),
            nonce: _bidNonce(testCommitment, sessionKey),
            initCode: "",
            callData: _executeCalldata(calls),
            accountGasLimits: bytes32(0),
            preVerificationGas: 0,
            gasFees: bytes32(0),
            paymasterAndData: "",
            signature: signature
        });

        SelectOptions memory expectedOptions =
            SelectOptions({commitment: testCommitment, solver: address(solverAccount), signature: sessionSignature});
        bytes memory selectCalldata = abi.encodeWithSelector(intentGateway.select.selector, expectedOptions);
        vm.mockCall(address(intentGateway), selectCalldata, abi.encode(sessionKey));

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, userOpHash, 0);

        assertEq(result, ERC4337Utils.SIG_VALIDATION_SUCCESS);
    }

    function test_ValidateUserOp_IntentSelection_PlainUserOpHash_WrongNonceKey_Fails() public {
        bytes32 userOpHash = keccak256("test_userop");

        bytes memory sessionSignature = _createSessionKeySignature(testCommitment, address(solverAccount));

        // Valid solver signature over the plain userOpHash...
        bytes memory solverSignature = _signUserOpHash(userOpHash);

        bytes memory signature = abi.encodePacked(testCommitment, solverSignature, sessionSignature);

        // ...but the userOp's nonce key is not keccak256(commitment ‖ sessionKey):
        // the signed userOp is not bound to the order/session being selected, so
        // validation must fail.
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

        SelectOptions memory expectedOptions =
            SelectOptions({commitment: testCommitment, solver: address(solverAccount), signature: sessionSignature});
        bytes memory selectCalldata = abi.encodeWithSelector(intentGateway.select.selector, expectedOptions);
        vm.mockCall(address(intentGateway), selectCalldata, abi.encode(sessionKey));

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, userOpHash, 0);

        assertEq(result, ERC4337Utils.SIG_VALIDATION_FAILED);
    }

    /// @dev The pre-upgrade composite format (EIP-191 over (userOpHash, commitment,
    ///      sessionKey)) is no longer accepted — older solvers remain delegated to
    ///      the previous SolverAccount deployment instead.
    function test_ValidateUserOp_IntentSelection_LegacyFormat_Fails() public {
        bytes32 userOpHash = keccak256("test_userop");

        // Create session signature (EIP-712 signature by session key)
        bytes memory sessionSignature = _createSessionKeySignature(testCommitment, address(solverAccount));

        // Legacy composite solver signature, with an otherwise-correct nonce binding
        bytes memory solverSignature = _createSolverSignature(userOpHash, testCommitment, sessionKey);

        // Create combined signature: commitment + solverSignature + sessionSignature
        bytes memory signature = abi.encodePacked(testCommitment, solverSignature, sessionSignature);

        PackedUserOperation memory op = PackedUserOperation({
            sender: address(solverAccount),
            nonce: _bidNonce(testCommitment, sessionKey),
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

        assertEq(result, ERC4337Utils.SIG_VALIDATION_FAILED);
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

        // Create INVALID solver signature (wrong signer over the plain userOpHash)
        uint256 wrongPrivateKey = 0x9999999999999999;
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(wrongPrivateKey, userOpHash);
        bytes memory invalidSolverSignature = abi.encodePacked(r, s, v);

        // Create combined signature
        bytes memory signature = abi.encodePacked(testCommitment, invalidSolverSignature, sessionSignature);

        PackedUserOperation memory op = PackedUserOperation({
            sender: address(solverAccount),
            nonce: _bidNonce(testCommitment, sessionKey),
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

        // Valid solver signature over the plain userOpHash
        bytes memory solverSignature = _signUserOpHash(userOpHash);

        // But include WRONG commitment in the signature bytes
        bytes memory signature = abi.encodePacked(wrongCommitment, solverSignature, sessionSignature);

        PackedUserOperation memory op = PackedUserOperation({
            sender: address(solverAccount),
            nonce: _bidNonce(testCommitment, sessionKey),
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
        bytes memory solverSignature = _signUserOpHash(userOpHash);

        // Create combined signature
        bytes memory signature = abi.encodePacked(testCommitment, solverSignature, sessionSignature);

        PackedUserOperation memory op = PackedUserOperation({
            sender: address(solverAccount),
            nonce: _bidNonce(testCommitment, sessionKey),
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
        bytes memory solverSignature1 = _signUserOpHash(userOpHash1);
        bytes memory signature1 = abi.encodePacked(commitment1, solverSignature1, sessionSignature1);

        PackedUserOperation memory op1 = PackedUserOperation({
            sender: address(solverAccount),
            nonce: _bidNonce(commitment1, sessionKey),
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
        bytes memory solverSignature2 = _signUserOpHash(userOpHash2);
        bytes memory signature2 = abi.encodePacked(commitment2, solverSignature2, sessionSignature2);

        PackedUserOperation memory op2 = PackedUserOperation({
            sender: address(solverAccount),
            nonce: _bidNonce(commitment2, sessionKey),
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

    /// @dev Anti-griefing: anyone can produce a valid `SelectSolver` signature for
    ///      (commitment, solver) with their own key. If that swapped session
    ///      signature were accepted, validation would pass and execution would
    ///      revert at fillOrder's session check — consuming the bid's nonce and
    ///      charging the solver. The nonce key binds the session key the solver
    ///      actually bid against, so the swap must fail during validation.
    function test_ValidateUserOp_IntentSelection_DifferentSessionKeys() public {
        bytes32 userOpHash = keccak256("test_userop");

        // Create second session key
        uint256 sessionKey2PrivateKey = 0xfedcba0987654321;
        address sessionKey2 = vm.addr(sessionKey2PrivateKey);

        // Create session signature with first session key
        bytes memory sessionSignature = _createSessionKeySignature(testCommitment, address(solverAccount));

        // Valid solver signature over the plain userOpHash, with the nonce bound to
        // the session key the solver bid against
        bytes memory solverSignature = _signUserOpHash(userOpHash);

        // Create combined signature
        bytes memory signature = abi.encodePacked(testCommitment, solverSignature, sessionSignature);

        PackedUserOperation memory op = PackedUserOperation({
            sender: address(solverAccount),
            nonce: _bidNonce(testCommitment, sessionKey),
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

    /// @dev Pinned against the same vector asserted in the SDK's
    ///      packedUserOpTypedData.test.ts — guards TS/Solidity drift in the bid
    ///      nonce-key derivation.
    function test_BidNonceKey_MatchesSdkVector() public pure {
        bytes32 commitment = keccak256("test_order_commitment");
        address sessionKeyAddr = address(0x00000000000000000000000000000000000000AA);
        uint192 key = uint192(uint256(keccak256(abi.encodePacked(commitment, sessionKeyAddr))));
        assertEq(uint256(key), 0x31c77a0860bd1b3f77fde0d2d875914d69220cf6b18ad191);
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

    /// @notice ERC-7821 execute(mode, executionData) calldata for a batch of calls
    function _executeCalldata(Execution[] memory calls) internal view returns (bytes memory) {
        bytes32 mode = bytes32(uint256(0x01) << 248); // CALLTYPE_BATCH, EXECTYPE_DEFAULT
        return abi.encodeWithSelector(solverAccount.execute.selector, mode, abi.encode(calls));
    }

    /// @notice A standard-mode (65-byte ECDSA) userOp with the given calldata
    function _standardOp(bytes memory callData, bytes memory signature)
        internal
        view
        returns (PackedUserOperation memory)
    {
        return PackedUserOperation({
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
    }

    /// @notice Creates an EIP-712 signature by session key for IntentGateway.select
    function _createSessionKeySignature(bytes32 commitment, address solverAddr) internal view returns (bytes memory) {
        bytes32 structHash = keccak256(abi.encode(intentGateway.SELECT_SOLVER_TYPEHASH(), commitment, solverAddr));
        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", intentGateway.DOMAIN_SEPARATOR(), structHash));
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(sessionKeyPrivateKey, digest);
        return abi.encodePacked(r, s, v);
    }

    /// @notice Creates the pre-upgrade composite solver signature (EIP-191 over
    ///         (userOpHash, commitment, sessionKey)) — no longer accepted; kept to
    ///         assert its rejection.
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

    /// @notice Solver signature over the plain v0.8 userOpHash (the EIP-712 digest of
    ///         the PackedUserOperation).
    function _signUserOpHash(bytes32 userOpHash) internal view returns (bytes memory) {
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(solverPrivateKey, userOpHash);
        return abi.encodePacked(r, s, v);
    }

    /// @notice The 4337 nonce binding a bid to its order and session key:
    ///         key = lower 192 bits of keccak256(commitment ‖ sessionKey), sequence 0.
    function _bidNonce(bytes32 commitment, address sessionKeyAddr) internal pure returns (uint256) {
        return uint256(uint192(uint256(keccak256(abi.encodePacked(commitment, sessionKeyAddr))))) << 64;
    }
}

contract MockContract {
    fallback() external payable {}
}
