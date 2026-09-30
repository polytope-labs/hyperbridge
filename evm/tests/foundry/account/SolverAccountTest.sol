// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Test.sol";
import {SolverAccount} from "../../../src/apps/intentsv2/SolverAccount.sol";
import {IntentGatewayV2} from "../../../src/apps/IntentGatewayV2.sol";
import {IntentsBase} from "../../../src/apps/intentsv2/IntentsBase.sol";
import {deployIntentGatewayImpl, deployIntentModules} from "../IntentGatewayDeploy.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {IntentQuoteTestUtils} from "../IntentQuoteTestUtils.sol";
import {ERC20Token} from "../mocks/ERC20Token.sol";
import {
    SelectOptions,
    Params,
    InitParams,
    DispatchInfo,
    PaymentInfo,
    Deployment,
    Order,
    TokenInfo,
    FillOptions
} from "@hyperbridge/core/apps/IntentGatewayV2.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";
import {PostRequest, DispatchPost} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {Account as AccountBase} from "@openzeppelin/contracts/account/Account.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
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

    // An account delegated to code whose gateway is a mock that only pulls tokens.
    SolverAccount public budgetAccount;
    MockGateway public budgetGateway;
    ERC20Token public token;
    address public beneficiary = address(0xB0B);
    bytes32 public orderId = keccak256("limit_order");

    bytes32 internal constant BATCH_MODE = bytes32(uint256(0x01) << 248);
    bytes internal constant CHAIN = bytes("EVM-31337");
    bytes internal constant PEER_CHAIN = bytes("EVM-1");
    bytes internal constant HYPERBRIDGE = bytes("POLKADOT-3367");

    function _deployGatewayProxy() internal returns (IntentGatewayV2) {
        IntentGatewayV2 implementation = deployIntentGatewayImpl();
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
        intentGateway.initialize(
            InitParams({params: params, peerChains: new bytes[](0), relayer: address(0), owner: address(this)})
        );

        // Deploy SolverAccount at a temporary address to get bytecode
        SolverAccount tempAccount = new SolverAccount(address(intentGateway));

        // Simulate EIP-7702: Etch the SolverAccount bytecode at the solver EOA address
        vm.etch(solver, address(tempAccount).code);
        solverAccount = SolverAccount(payable(solver));

        // Fund solver account
        vm.deal(address(solverAccount), 10 ether);

        // Create test commitment
        testCommitment = keccak256("test_order_commitment");

        budgetGateway = new MockGateway();
        address budgetSolver = makeAddr("budgetSolver");
        vm.etch(budgetSolver, address(new SolverAccount(address(budgetGateway))).code);
        budgetAccount = SolverAccount(payable(budgetSolver));

        token = new ERC20Token("Output", "OUT", 18);
        token.mint(budgetSolver, 1_000_000e18);
    }

    // ============================================
    // Constructor Tests
    // ============================================

    function test_ReleaseVersionProtectsCurrentAndHistoricalSelectors() public view {
        assertEq(intentGateway.version(), 3);
        assertNotEq(intentGateway.fillOrder.selector, bytes4(0xa5470064));
        assertNotEq(intentGateway.fillOrder.selector, bytes4(0x5cfb1ea5));
    }

    function testVersionTwoMigrationShiftsRelayerOnce() public {
        bytes32 initSlot = 0xf0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00;
        vm.store(address(intentGateway), initSlot, bytes32(uint256(2)));
        address relayer = address(0x123456);
        vm.store(address(intentGateway), bytes32(uint256(13)), bytes32(uint256(uint160(relayer)) << 8));
        vm.prank(intentGateway.host());
        intentGateway.migrate(address(0xabc));
        assertEq(intentGateway.version(), 3);
        assertEq(intentGateway.owner(), address(0xabc));
        assertEq(intentGateway.relayer(), relayer);
    }

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
            target: address(intentGateway), value: 0, callData: abi.encodeWithSelector(intentGateway.fillOrder.selector)
        });

        PackedUserOperation memory op = _standardOp(_executeCalldata(calls), _signUserOpHash(userOpHash));

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, userOpHash, 0);

        assertEq(result, ERC4337Utils.SIG_VALIDATION_FAILED);
    }

    function test_ValidateUserOp_StandardECDSA_HistoricalFillCalldata_Fails() public {
        _assertHistoricalFillRejectsStandardSignature(0xa5470064);
        _assertHistoricalFillRejectsStandardSignature(0x5cfb1ea5);
    }

    function _assertHistoricalFillRejectsStandardSignature(bytes4 selector) internal {
        bytes32 userOpHash = keccak256("test_userop");

        Execution[] memory calls = new Execution[](1);
        calls[0] = Execution({target: address(intentGateway), value: 0, callData: abi.encodeWithSelector(selector)});

        PackedUserOperation memory op = _standardOp(_executeCalldata(calls), _signUserOpHash(userOpHash));

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, userOpHash, 0);

        assertEq(result, ERC4337Utils.SIG_VALIDATION_FAILED);
    }

    function test_ValidateUserOp_StandardECDSA_AppendedFillCalldata_Fails() public {
        bytes32 userOpHash = keccak256("test_userop");

        Execution[] memory calls = new Execution[](1);
        calls[0] = Execution({
            target: address(intentGateway),
            value: 0,
            callData: abi.encodeWithSignature(
                "fillOrder((bytes32,bytes,bytes,uint256,uint256,uint256,address,((bytes32,uint256)[],bytes),(bytes32,uint256)[],(bytes32,(bytes32,uint256)[],bytes)),(uint256,uint256,uint256,(bytes32,uint256)[],(bytes32,uint256)[]))"
            )
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
            target: address(0xBEEF), value: 0, callData: abi.encodeWithSelector(intentGateway.fillOrder.selector)
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
            nonce: _bidNonce(testCommitment, sessionKey, ""),
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
            target: address(intentGateway), value: 0, callData: abi.encodeWithSelector(intentGateway.fillOrder.selector)
        });

        PackedUserOperation memory op = PackedUserOperation({
            sender: address(solverAccount),
            nonce: _bidNonce(testCommitment, sessionKey, _executeCalldata(calls)),
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

    function test_ValidateUserOp_IntentSelection_HistoricalFillCalldata_Success() public {
        _assertHistoricalFillAcceptsIntentSignature(0xa5470064);
        _assertHistoricalFillAcceptsIntentSignature(0x5cfb1ea5);
    }

    function _assertHistoricalFillAcceptsIntentSignature(bytes4 selector) internal {
        bytes32 userOpHash = keccak256("test_userop");

        bytes memory sessionSignature = _createSessionKeySignature(testCommitment, address(solverAccount));
        bytes memory solverSignature = _signUserOpHash(userOpHash);
        bytes memory signature = abi.encodePacked(testCommitment, solverSignature, sessionSignature);

        Execution[] memory calls = new Execution[](1);
        calls[0] = Execution({target: address(intentGateway), value: 0, callData: abi.encodeWithSelector(selector)});

        PackedUserOperation memory op = PackedUserOperation({
            sender: address(solverAccount),
            nonce: _bidNonce(testCommitment, sessionKey, _executeCalldata(calls)),
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

        // ...but the userOp's nonce key is not keccak256(commitment ‖ sessionKey ‖ keccak256(callData)):
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

    /// @dev A solver bidding two prices on one order signs two ops. Each is keyed by its own
    ///      calldata, so each is sequence 0 of its own key and validates on its own, in either
    ///      order: neither waits on the other.
    function test_ValidateUserOp_IntentSelection_TwoBidsOnOneOrder_EachOnItsOwnKey() public {
        bytes memory sessionSignature = _createSessionKeySignature(testCommitment, address(solverAccount));
        SelectOptions memory expectedOptions =
            SelectOptions({commitment: testCommitment, solver: address(solverAccount), signature: sessionSignature});
        vm.mockCall(
            address(intentGateway),
            abi.encodeWithSelector(intentGateway.select.selector, expectedOptions),
            abi.encode(sessionKey)
        );

        bytes memory first = _fillCalldata(hex"01");
        bytes memory second = _fillCalldata(hex"02");
        uint256 firstNonce = _bidNonce(testCommitment, sessionKey, first);
        uint256 secondNonce = _bidNonce(testCommitment, sessionKey, second);

        assertTrue(firstNonce != secondNonce, "each bid has its own key");
        assertEq(firstNonce & type(uint64).max, 0, "and signs its first sequence");
        assertEq(secondNonce & type(uint64).max, 0);

        // The second bid validates first: nothing orders them.
        assertEq(_validateBid(second, secondNonce, sessionSignature), ERC4337Utils.SIG_VALIDATION_SUCCESS);
        assertEq(_validateBid(first, firstNonce, sessionSignature), ERC4337Utils.SIG_VALIDATION_SUCCESS);
    }

    /// @dev The key commits to the calldata, so an op carrying calldata other than the one its
    ///      key was derived from is refused.
    function test_ValidateUserOp_IntentSelection_NonceKeyFromOtherCalldata_Fails() public {
        bytes memory sessionSignature = _createSessionKeySignature(testCommitment, address(solverAccount));
        SelectOptions memory expectedOptions =
            SelectOptions({commitment: testCommitment, solver: address(solverAccount), signature: sessionSignature});
        vm.mockCall(
            address(intentGateway),
            abi.encodeWithSelector(intentGateway.select.selector, expectedOptions),
            abi.encode(sessionKey)
        );

        uint256 keyedOnFirst = _bidNonce(testCommitment, sessionKey, _fillCalldata(hex"01"));

        assertEq(
            _validateBid(_fillCalldata(hex"02"), keyedOnFirst, sessionSignature), ERC4337Utils.SIG_VALIDATION_FAILED
        );
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
            nonce: _bidNonce(testCommitment, sessionKey, ""),
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
            nonce: _bidNonce(testCommitment, sessionKey, ""),
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
            nonce: _bidNonce(testCommitment, sessionKey, ""),
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

    /// @dev A gateway returning something decodable but not the session key fails validation: the
    /// nonce key cannot match a key the bid was never signed against.
    function test_ValidateUserOp_IntentSelection_GatewayReturnsWrongSessionKey_Fails() public {
        bytes32 userOpHash = keccak256("test_userop");

        bytes memory sessionSignature = _createSessionKeySignature(testCommitment, address(solverAccount));
        bytes memory solverSignature = _signUserOpHash(userOpHash);

        bytes memory signature = abi.encodePacked(testCommitment, solverSignature, sessionSignature);

        PackedUserOperation memory op = PackedUserOperation({
            sender: address(solverAccount),
            nonce: _bidNonce(testCommitment, sessionKey, ""),
            initCode: "",
            callData: "",
            accountGasLimits: bytes32(0),
            preVerificationGas: 0,
            gasFees: bytes32(0),
            paymasterAndData: "",
            signature: signature
        });

        _mockSelect(sessionSignature, abi.encode(address(0xdead)));

        vm.prank(entryPoint);
        uint256 result = solverAccount.validateUserOp(op, userOpHash, 0);

        assertEq(result, ERC4337Utils.SIG_VALIDATION_FAILED);
    }

    /// @dev Return data too short to decode as an address is not caught by `try`: validation reverts
    /// rather than failing softly. Unreachable against the real gateway, whose `select` returns an
    /// address, so this pins the behaviour if that address is ever pointed at something else.
    function test_ValidateUserOp_IntentSelection_GatewayReturnsTruncatedData_Reverts() public {
        bytes32 userOpHash = keccak256("test_userop");

        bytes memory sessionSignature = _createSessionKeySignature(testCommitment, address(solverAccount));
        bytes memory solverSignature = _signUserOpHash(userOpHash);

        bytes memory signature = abi.encodePacked(testCommitment, solverSignature, sessionSignature);

        PackedUserOperation memory op = PackedUserOperation({
            sender: address(solverAccount),
            nonce: _bidNonce(testCommitment, sessionKey, ""),
            initCode: "",
            callData: "",
            accountGasLimits: bytes32(0),
            preVerificationGas: 0,
            gasFees: bytes32(0),
            paymasterAndData: "",
            signature: signature
        });

        _mockSelect(sessionSignature, hex"1234");

        vm.prank(entryPoint);
        vm.expectRevert();
        solverAccount.validateUserOp(op, userOpHash, 0);
    }

    /// @dev Mocks the gateway's `select` for this commitment and session signature.
    function _mockSelect(bytes memory sessionSignature, bytes memory returnData) internal {
        SelectOptions memory expectedOptions =
            SelectOptions({commitment: testCommitment, solver: address(solverAccount), signature: sessionSignature});
        vm.mockCall(
            address(intentGateway), abi.encodeWithSelector(intentGateway.select.selector, expectedOptions), returnData
        );
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
            nonce: _bidNonce(commitment1, sessionKey, ""),
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
            nonce: _bidNonce(commitment2, sessionKey, ""),
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
            nonce: _bidNonce(testCommitment, sessionKey, ""),
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
        bytes memory callData = hex"deadbeef";
        uint192 key = uint192(uint256(keccak256(abi.encodePacked(commitment, sessionKeyAddr, keccak256(callData)))));
        assertEq(uint256(key), 0xbc2d417670f508428573098858469957bf350bc5ba31cc25);
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
    // debitOrder Tests
    // ============================================

    function test_DebitOrder_NotSelf_Reverts() public {
        address[3] memory callers = [entryPoint, address(budgetGateway), address(0xBAD)];

        for (uint256 i = 0; i < callers.length; i++) {
            vm.expectRevert(abi.encodeWithSelector(AccountBase.AccountUnauthorized.selector, callers[i]));
            vm.prank(callers[i]);
            budgetAccount.debitOrder(orderId, 100e18, address(token), 0, 0);
        }

        assertEq(budgetAccount.spent(orderId), 0);
    }

    function test_DebitOrder_RecordsPayoutAndClearsAllowance() public {
        _executeBatch(_budgetBatch(orderId, address(token), 45e18, 40e18, 0, 100e18));

        assertEq(budgetAccount.spent(orderId), 40e18);
        assertEq(token.balanceOf(beneficiary), 40e18);
        assertEq(token.allowance(address(budgetAccount), address(budgetGateway)), 0);
    }

    function test_DebitOrder_AccumulatesPerOrder() public {
        bytes32 otherOrder = keccak256("other_limit_order");

        _executeBatch(_budgetBatch(orderId, address(token), 40e18, 40e18, 0, 100e18));
        _executeBatch(_budgetBatch(otherOrder, address(token), 7e18, 7e18, 0, 100e18));
        _executeBatch(_budgetBatch(orderId, address(token), 25e18, 25e18, 0, 100e18));

        assertEq(budgetAccount.spent(orderId), 65e18);
        assertEq(budgetAccount.spent(otherOrder), 7e18);
    }

    function test_DebitOrder_PastCap_RevertsAndUndoesPayout() public {
        _executeBatch(_budgetBatch(orderId, address(token), 60e18, 60e18, 0, 100e18));
        uint256 accountBalance = token.balanceOf(address(budgetAccount));

        Execution[] memory calls = _budgetBatch(orderId, address(token), 41e18, 41e18, 0, 100e18);
        vm.expectRevert(abi.encodeWithSelector(SolverAccount.LimitOrderExceeded.selector, orderId, 101e18, 100e18));
        _executeBatch(calls);

        assertEq(budgetAccount.spent(orderId), 60e18);
        assertEq(token.balanceOf(beneficiary), 60e18);
        assertEq(token.balanceOf(address(budgetAccount)), accountBalance);
        assertEq(token.allowance(address(budgetAccount), address(budgetGateway)), 0);
    }

    function test_DebitOrder_ReachingCap_Succeeds() public {
        _executeBatch(_budgetBatch(orderId, address(token), 60e18, 60e18, 0, 100e18));
        _executeBatch(_budgetBatch(orderId, address(token), 40e18, 40e18, 0, 100e18));

        assertEq(budgetAccount.spent(orderId), 100e18);
        assertEq(token.balanceOf(beneficiary), 100e18);
    }

    /// @dev The dispatch fee leaves through the same allowance as the payout but is not part of it.
    function test_DebitOrder_FeeIsNotCounted() public {
        uint256 payout = 40e18;
        uint256 extra = 5e18;
        uint256 fee = 2e18;

        _executeBatch(_budgetBatch(orderId, address(token), payout + extra + fee, payout, fee, payout));

        assertEq(budgetAccount.spent(orderId), payout);
        assertEq(token.balanceOf(beneficiary), payout);
        assertEq(token.balanceOf(address(budgetGateway)), fee);
        assertEq(token.allowance(address(budgetAccount), address(budgetGateway)), 0);
    }

    function test_DebitOrder_PartialPull_CountsWhatWasPulled() public {
        _executeBatch(_budgetBatch(orderId, address(token), 40e18, 15e18, 0, 100e18));

        assertEq(budgetAccount.spent(orderId), 15e18);
        assertEq(token.balanceOf(beneficiary), 15e18);
        assertEq(token.allowance(address(budgetAccount), address(budgetGateway)), 0);
    }

    function test_DebitOrder_TokenWithoutApproveReturnValue() public {
        NoReturnToken usdt = new NoReturnToken();
        usdt.mint(address(budgetAccount), 1_000e6);

        _executeBatch(_budgetBatch(orderId, address(usdt), 400e6, 400e6, 0, 1_000e6));
        _executeBatch(_budgetBatch(orderId, address(usdt), 400e6, 150e6, 0, 1_000e6));

        assertEq(budgetAccount.spent(orderId), 550e6);
        assertEq(usdt.balanceOf(beneficiary), 550e6);
        assertEq(usdt.allowance(address(budgetAccount), address(budgetGateway)), 0);
    }

    /// @dev An `approved` below what the gateway was really given underflows, and the batch reverts.
    function test_DebitOrder_UnderstatedApproval_Panics() public {
        Execution[] memory calls = _budgetBatch(orderId, address(token), 40e18, 10e18, 0, 100e18);
        calls[3].callData = abi.encodeCall(SolverAccount.debitOrder, (orderId, 100e18, address(token), 29e18, 0));

        vm.expectRevert(stdError.arithmeticError);
        _executeBatch(calls);

        assertEq(budgetAccount.spent(orderId), 0);
        assertEq(token.balanceOf(beneficiary), 0);
    }

    /// @dev A `fee` above what left the allowance underflows too.
    function test_DebitOrder_OverstatedFee_Panics() public {
        Execution[] memory calls = _budgetBatch(orderId, address(token), 40e18, 1e18, 0, 100e18);
        calls[3].callData = abi.encodeCall(SolverAccount.debitOrder, (orderId, 100e18, address(token), 40e18, 2e18));

        vm.expectRevert(stdError.arithmeticError);
        _executeBatch(calls);

        assertEq(budgetAccount.spent(orderId), 0);
        assertEq(token.balanceOf(beneficiary), 0);
    }

    function test_DebitOrder_TallyLivesAtItsNamespacedSlot() public {
        bytes32 namespace = keccak256(abi.encode(uint256(keccak256("hyperbridge.storage.SolverAccount.Budgets")) - 1))
            & ~bytes32(uint256(0xff));
        assertEq(namespace, 0xef37eedb8cd243d7bb1074a6cb5a4fad8c39bd328408761135c4a5a7d5c29900);

        _executeBatch(_budgetBatch(orderId, address(token), 40e18, 40e18, 0, 100e18));

        bytes32 tallySlot = keccak256(abi.encode(orderId, namespace));
        assertEq(uint256(vm.load(address(budgetAccount), tallySlot)), 40e18);
    }

    /// @dev A selected bid against the gateway itself: validated, filled and debited in one op.
    function test_DebitOrder_GatewayFill() public {
        uint256 inputAmount = 1000e18;
        uint256 outputAmount = 900e18;
        uint256 extra = 5e18;
        ERC20Token inputToken = new ERC20Token("Input", "IN", 18);
        token.mint(address(solverAccount), outputAmount + extra);

        Order memory order = _placeOrder(inputToken, inputAmount, outputAmount);
        FillOptions memory options = _fillOptions(order, outputAmount, 0);

        _runBid(order, _gatewayBatch(order, options, outputAmount + extra, outputAmount));

        assertEq(solverAccount.spent(orderId), outputAmount);
        assertEq(token.balanceOf(beneficiary), outputAmount);
        assertEq(token.balanceOf(address(solverAccount)), extra);
        assertEq(inputToken.balanceOf(address(solverAccount)), inputAmount);
        assertEq(token.allowance(address(solverAccount), address(intentGateway)), 0);
    }

    /// @dev A cross-chain fill that pays its dispatch fee in the output token, by a solver offering
    ///      above the order's rate. The tally takes the payout and the protocol's share of the
    ///      surplus, and leaves out the fee and the part of the approval the gateway never pulled.
    function test_DebitOrder_GatewayCrossChainFill_FeeInOutputToken() public {
        uint256 required = 900e18;
        uint256 offered = 1000e18;
        uint256 relayerFee = 3e18;
        uint256 extra = 5e18;
        uint256 approved = offered + extra + relayerFee;
        token.mint(address(solverAccount), approved);

        address host = _useHost(address(token));
        _addPeer(PEER_CHAIN, address(0xCAFE));

        Order memory order = _order(address(0x1111), 1000e18, required);
        order.user = bytes32(uint256(uint160(makeAddr("user"))));
        order.source = PEER_CHAIN;
        FillOptions memory options = _fillOptions(order, offered, relayerFee);

        _runBid(order, _gatewayBatch(order, options, approved, offered));

        // Half of the surplus goes to the beneficiary and half to the protocol.
        uint256 surplus = offered - required;
        assertEq(token.balanceOf(beneficiary), required + surplus / 2);
        assertEq(token.balanceOf(address(intentGateway)), surplus / 2);
        assertEq(token.balanceOf(host), relayerFee);

        assertEq(solverAccount.spent(orderId), offered);
        assertEq(token.balanceOf(address(solverAccount)), extra);
        assertEq(token.allowance(address(solverAccount), address(intentGateway)), 0);
    }

    // ============================================
    // Helper Functions
    // ============================================

    function _approval(address asset, address spender, uint256 amount) internal pure returns (Execution memory) {
        return Execution({target: asset, value: 0, callData: abi.encodeCall(IERC20.approve, (spender, amount))});
    }

    /// @dev A fill's batch against the mock gateway: the approvals, the gateway pulling `payout` and
    ///      `fee`, then the debit.
    function _budgetBatch(bytes32 id, address asset, uint256 approved, uint256 payout, uint256 fee, uint256 cap)
        internal
        view
        returns (Execution[] memory calls)
    {
        calls = new Execution[](4);
        calls[0] = _approval(asset, address(budgetGateway), 0);
        calls[1] = _approval(asset, address(budgetGateway), approved);
        calls[2] = Execution({
            target: address(budgetGateway),
            value: 0,
            callData: abi.encodeCall(MockGateway.fill, (asset, beneficiary, payout, fee))
        });
        calls[3] = Execution({
            target: address(budgetAccount),
            value: 0,
            callData: abi.encodeCall(SolverAccount.debitOrder, (id, cap, asset, approved, fee))
        });
    }

    function _executeBatch(Execution[] memory calls) internal {
        vm.prank(entryPoint);
        budgetAccount.execute(BATCH_MODE, abi.encode(calls));
    }

    /// @dev Puts a host that answers the gateway at the gateway's host address.
    function _useHost(address feeToken) internal returns (address host) {
        host = intentGateway.host();
        vm.etch(host, address(new MockHost()).code);
        MockHost(host).configure(CHAIN, HYPERBRIDGE, feeToken);
    }

    /// @dev Registers `gateway` as the gateway's peer on `chain`, as governance does.
    function _addPeer(bytes memory chain, address gateway) internal {
        PostRequest memory request = PostRequest({
            source: HYPERBRIDGE,
            dest: CHAIN,
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            timeoutTimestamp: 0,
            body: bytes.concat(
                bytes1(uint8(IntentsBase.RequestKind.NewDeployment)),
                abi.encode(Deployment({chain: chain, gateway: gateway}))
            )
        });

        address host = intentGateway.host();
        vm.prank(host);
        intentGateway.onAccept(IncomingPostRequest({request: request, relayer: address(0)}));
    }

    /// @dev An order for this chain selling `inputToken` for `token`, selected by `sessionKey`.
    function _order(address inputToken, uint256 inputAmount, uint256 outputAmount)
        internal
        view
        returns (Order memory)
    {
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(inputToken))), amount: inputAmount});
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(token)))), amount: outputAmount});

        return Order({
            user: bytes32(0),
            source: "",
            destination: CHAIN,
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: sessionKey,
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: PaymentInfo({beneficiary: bytes32(uint256(uint160(beneficiary))), assets: outputs, call: ""})
        });
    }

    /// @dev Places `_order` on the gateway as a same-chain order.
    function _placeOrder(ERC20Token inputToken, uint256 inputAmount, uint256 outputAmount)
        internal
        returns (Order memory order)
    {
        _useHost(address(0));
        address user = makeAddr("user");
        inputToken.mint(user, inputAmount);
        order = _order(address(inputToken), inputAmount, outputAmount);

        vm.startPrank(user);
        inputToken.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        order.user = bytes32(uint256(uint160(user)));
        order.source = CHAIN;
    }

    /// @dev A quote for all of `order`'s input, offering `offered` of the output token.
    function _fillOptions(Order memory order, uint256 offered, uint256 relayerFee)
        internal
        pure
        returns (FillOptions memory)
    {
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: order.output.assets[0].token, amount: offered});

        return FillOptions({
            relayerFee: relayerFee,
            nativeDispatchFee: 0,
            validUntil: 0,
            outputs: outputs,
            inputs: IntentQuoteTestUtils.inputs(order, outputs)
        });
    }

    /// @dev A fill's batch against the gateway: the approvals, `fillOrder`, then the debit.
    function _gatewayBatch(Order memory order, FillOptions memory options, uint256 approved, uint256 cap)
        internal
        view
        returns (Execution[] memory calls)
    {
        calls = new Execution[](4);
        calls[0] = _approval(address(token), address(intentGateway), 0);
        calls[1] = _approval(address(token), address(intentGateway), approved);
        calls[2] = Execution({
            target: address(intentGateway),
            value: 0,
            callData: abi.encodeCall(intentGateway.fillOrder, (order, options))
        });
        calls[3] = Execution({
            target: address(solverAccount),
            value: 0,
            callData: abi.encodeCall(
                SolverAccount.debitOrder, (orderId, cap, address(token), approved, options.relayerFee)
            )
        });
    }

    /// @dev Runs a selected bid on `order` as the EntryPoint does: validation, then the batch.
    function _runBid(Order memory order, Execution[] memory calls) internal {
        bytes32 commitment = keccak256(abi.encode(order));
        bytes memory callData = _executeCalldata(calls);
        bytes32 userOpHash = keccak256(callData);
        bytes memory sessionSignature = _createSessionKeySignature(commitment, address(solverAccount));

        PackedUserOperation memory op = PackedUserOperation({
            sender: address(solverAccount),
            nonce: _bidNonce(commitment, sessionKey, callData),
            initCode: "",
            callData: callData,
            accountGasLimits: bytes32(0),
            preVerificationGas: 0,
            gasFees: bytes32(0),
            paymasterAndData: "",
            signature: abi.encodePacked(commitment, _signUserOpHash(userOpHash), sessionSignature)
        });

        vm.prank(entryPoint);
        assertEq(solverAccount.validateUserOp(op, userOpHash, 0), ERC4337Utils.SIG_VALIDATION_SUCCESS);

        vm.prank(entryPoint);
        (bool ok, bytes memory returned) = address(solverAccount).call(callData);
        if (!ok) {
            assembly {
                revert(add(returned, 0x20), mload(returned))
            }
        }
    }

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
    /// @dev A bid's batch calling fillOrder on the gateway; `tag` stands in for the quote, which is
    ///      what differs between two bids on one order.
    function _fillCalldata(bytes memory tag) internal view returns (bytes memory) {
        Execution[] memory calls = new Execution[](1);
        calls[0] = Execution({
            target: address(intentGateway), value: 0, callData: abi.encodePacked(intentGateway.fillOrder.selector, tag)
        });
        return _executeCalldata(calls);
    }

    /// @dev Validates a selected bid carrying `callData` and `nonce`, signed by the solver.
    function _validateBid(bytes memory callData, uint256 nonce, bytes memory sessionSignature)
        internal
        returns (uint256)
    {
        bytes32 userOpHash = keccak256(abi.encode(callData, nonce));
        bytes memory signature = abi.encodePacked(testCommitment, _signUserOpHash(userOpHash), sessionSignature);
        PackedUserOperation memory op = PackedUserOperation({
            sender: address(solverAccount),
            nonce: nonce,
            initCode: "",
            callData: callData,
            accountGasLimits: bytes32(0),
            preVerificationGas: 0,
            gasFees: bytes32(0),
            paymasterAndData: "",
            signature: signature
        });
        vm.prank(entryPoint);
        return solverAccount.validateUserOp(op, userOpHash, 0);
    }

    function _bidNonce(bytes32 commitment, address sessionKeyAddr, bytes memory callData)
        internal
        pure
        returns (uint256)
    {
        return uint256(uint192(uint256(keccak256(abi.encodePacked(commitment, sessionKeyAddr, keccak256(callData))))))
            << 64;
    }
}

contract MockContract {
    fallback() external payable {}
}

/// @dev Answers what the gateway asks its host, and takes the dispatch fee from it as the host does.
contract MockHost {
    bytes public host;
    bytes public hyperbridge;
    address public feeToken;

    function configure(bytes memory host_, bytes memory hyperbridge_, address feeToken_) external {
        host = host_;
        hyperbridge = hyperbridge_;
        feeToken = feeToken_;
    }

    function dispatch(DispatchPost memory request) external payable returns (bytes32) {
        IERC20(feeToken).transferFrom(msg.sender, address(this), request.fee);
        return keccak256(abi.encode(request));
    }
}

/// @dev Pulls a payout and a fee from its caller, as the gateway does from a filler.
contract MockGateway {
    using SafeERC20 for IERC20;

    function fill(address token, address beneficiary, uint256 payout, uint256 fee) external {
        IERC20(token).safeTransferFrom(msg.sender, beneficiary, payout);
        if (fee > 0) IERC20(token).safeTransferFrom(msg.sender, address(this), fee);
    }
}

/// @dev Mainnet USDT's shape: nothing is returned, and a non-zero allowance must be cleared before
///      it is set again.
contract NoReturnToken {
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    function mint(address to, uint256 amount) external {
        balanceOf[to] += amount;
    }

    function approve(address spender, uint256 amount) external {
        require(amount == 0 || allowance[msg.sender][spender] == 0, "allowance not cleared");
        allowance[msg.sender][spender] = amount;
    }

    function transfer(address to, uint256 amount) external {
        balanceOf[msg.sender] -= amount;
        balanceOf[to] += amount;
    }

    function transferFrom(address from, address to, uint256 amount) external {
        allowance[from][msg.sender] -= amount;
        balanceOf[from] -= amount;
        balanceOf[to] += amount;
    }
}
