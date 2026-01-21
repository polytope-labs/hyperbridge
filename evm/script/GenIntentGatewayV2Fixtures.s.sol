// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "forge-std/console.sol";
import {
    IntentGatewayV2,
    Order,
    Params,
    TokenInfo,
    PaymentInfo,
    DispatchInfo,
    FillOptions,
    SelectOptions
} from "../src/apps/IntentGatewayV2.sol";

/**
 * @title GenIntentGatewayV2Fixtures
 * @notice Generates JSON test vectors for TypeScript SDK tests
 * @dev Run with: forge script script/GenIntentGatewayV2Fixtures.s.sol --tc GenIntentGatewayV2Fixtures -vvv
 * @dev Export JSON: forge script script/GenIntentGatewayV2Fixtures.s.sol --tc GenIntentGatewayV2Fixtures --offline -vvv 2>&1 | awk '/^== Logs ==$/,0 {if (!/^== Logs ==$/) print}' > src/fixtures/intent-gateway-v2.json
 */
contract GenIntentGatewayV2Fixtures is Script {
    // Known test private keys (DO NOT use in production)
    uint256 constant SESSION_KEY_PRIVATE_KEY = 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80;
    uint256 constant SOLVER_PRIVATE_KEY = 0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d;
    
    // Test addresses derived from above keys
    address sessionKeyAddress;
    address solverAddress;
    
    // Mock addresses for testing
    address constant MOCK_USER = 0x70997970C51812dc3A010C7d01b50e0d17dc79C8;
    address constant MOCK_USDC = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48;
    address constant MOCK_DAI = 0x6B175474E89094C44Da98b954EedeAC495271d0F;
    address constant MOCK_ENTRY_POINT = 0x0000000071727De22E5E9d8BAf0edAc6f37da032;
    
    IntentGatewayV2 intentGateway;
    
    function setUp() public {
        sessionKeyAddress = vm.addr(SESSION_KEY_PRIVATE_KEY);
        solverAddress = vm.addr(SOLVER_PRIVATE_KEY);
    }

    function run() public {
        console.log("{");
        
        // Generate all test vectors as a single JSON object
        console.log("  \"orderCommitmentVectors\": ");
        generateOrderCommitmentVectors();
        console.log(",");
        
        console.log("  \"eip712SignatureVectors\": ");
        generateEIP712SignatureVectors();
        console.log(",");
        
        console.log("  \"userOpHashVectors\": ");
        generateUserOpHashVectors();
        console.log(",");
        
        console.log("  \"gasPackingVectors\": ");
        generateGasPackingVectors();
        console.log(",");
        
        console.log("  \"bidSignatureVectors\": ");
        generateBidSignatureVectors();
        
        console.log("}");
    }
    
    // =========================================================================
    // Order Commitment Vectors
    // =========================================================================
    
    function generateOrderCommitmentVectors() internal {
        console.log("  [");
        
        // Vector 1: Simple order with single input/output
        {
            TokenInfo[] memory inputs = new TokenInfo[](1);
            inputs[0] = TokenInfo({
                token: bytes32(uint256(uint160(MOCK_USDC))),
                amount: 1000 * 1e6
            });
            
            TokenInfo[] memory outputAssets = new TokenInfo[](1);
            outputAssets[0] = TokenInfo({
                token: bytes32(uint256(uint160(MOCK_DAI))),
                amount: 999 * 1e18
            });
            
            Order memory order = Order({
                user: bytes32(uint256(uint160(MOCK_USER))),
                source: hex"45564d2d31",  // "EVM-1"
                destination: hex"45564d2d3133373030303030",  // "EVM-1370000"
                deadline: 1000000,
                nonce: 1,
                fees: 1000000000000000000,  // 1e18
                session: sessionKeyAddress,
                predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
                inputs: inputs,
                output: PaymentInfo({
                    beneficiary: bytes32(uint256(uint160(MOCK_USER))),
                    assets: outputAssets,
                    call: ""
                })
            });
            
            bytes32 commitment = keccak256(abi.encode(order));
            
            console.log("  {");
            console.log("    \"name\": \"simple_single_token\",");
            console.log("    \"order\": {");
            console.log("      \"user\": \"%s\",", vm.toString(order.user));
            console.log("      \"source\": \"%s\",", vm.toString(order.source));
            console.log("      \"destination\": \"%s\",", vm.toString(order.destination));
            console.log("      \"deadline\": %d,", order.deadline);
            console.log("      \"nonce\": %d,", order.nonce);
            console.log("      \"fees\": \"%d\",", order.fees);
            console.log("      \"session\": \"%s\",", vm.toString(order.session));
            console.log("      \"predispatch\": { \"assets\": [], \"call\": \"0x\" },");
            console.log("      \"inputs\": [{ \"token\": \"%s\", \"amount\": \"%d\" }],", vm.toString(inputs[0].token), inputs[0].amount);
            console.log("      \"output\": {");
            console.log("        \"beneficiary\": \"%s\",", vm.toString(order.output.beneficiary));
            console.log("        \"assets\": [{ \"token\": \"%s\", \"amount\": \"%d\" }],", vm.toString(outputAssets[0].token), outputAssets[0].amount);
            console.log("        \"call\": \"0x\"");
            console.log("      }");
            console.log("    },");
            console.log("    \"commitment\": \"%s\",", vm.toString(commitment));
            console.log("    \"encodedOrder\": \"%s\"", vm.toString(abi.encode(order)));
            console.log("  },");
        }
        
        // Vector 2: Order with multiple inputs/outputs
        {
            TokenInfo[] memory inputs = new TokenInfo[](2);
            inputs[0] = TokenInfo({
                token: bytes32(uint256(uint160(MOCK_USDC))),
                amount: 500 * 1e6
            });
            inputs[1] = TokenInfo({
                token: bytes32(uint256(uint160(MOCK_DAI))),
                amount: 500 * 1e18
            });
            
            TokenInfo[] memory outputAssets = new TokenInfo[](2);
            outputAssets[0] = TokenInfo({
                token: bytes32(uint256(uint160(MOCK_USDC))),
                amount: 495 * 1e6
            });
            outputAssets[1] = TokenInfo({
                token: bytes32(uint256(uint160(MOCK_DAI))),
                amount: 495 * 1e18
            });
            
            Order memory order = Order({
                user: bytes32(uint256(uint160(MOCK_USER))),
                source: hex"45564d2d31",
                destination: hex"45564d2d3833343533",  // "EVM-8453" (Base)
                deadline: 2000000,
                nonce: 42,
                fees: 2500000000000000000,  // 2.5e18
                session: sessionKeyAddress,
                predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
                inputs: inputs,
                output: PaymentInfo({
                    beneficiary: bytes32(uint256(uint160(MOCK_USER))),
                    assets: outputAssets,
                    call: ""
                })
            });
            
            bytes32 commitment = keccak256(abi.encode(order));
            
            console.log("  {");
            console.log("    \"name\": \"multi_token\",");
            console.log("    \"order\": {");
            console.log("      \"user\": \"%s\",", vm.toString(order.user));
            console.log("      \"source\": \"%s\",", vm.toString(order.source));
            console.log("      \"destination\": \"%s\",", vm.toString(order.destination));
            console.log("      \"deadline\": %d,", order.deadline);
            console.log("      \"nonce\": %d,", order.nonce);
            console.log("      \"fees\": \"%d\",", order.fees);
            console.log("      \"session\": \"%s\",", vm.toString(order.session));
            console.log("      \"predispatch\": { \"assets\": [], \"call\": \"0x\" },");
            console.log("      \"inputs\": [");
            console.log("        { \"token\": \"%s\", \"amount\": \"%d\" },", vm.toString(inputs[0].token), inputs[0].amount);
            console.log("        { \"token\": \"%s\", \"amount\": \"%d\" }", vm.toString(inputs[1].token), inputs[1].amount);
            console.log("      ],");
            console.log("      \"output\": {");
            console.log("        \"beneficiary\": \"%s\",", vm.toString(order.output.beneficiary));
            console.log("        \"assets\": [");
            console.log("          { \"token\": \"%s\", \"amount\": \"%d\" },", vm.toString(outputAssets[0].token), outputAssets[0].amount);
            console.log("          { \"token\": \"%s\", \"amount\": \"%d\" }", vm.toString(outputAssets[1].token), outputAssets[1].amount);
            console.log("        ],");
            console.log("        \"call\": \"0x\"");
            console.log("      }");
            console.log("    },");
            console.log("    \"commitment\": \"%s\",", vm.toString(commitment));
            console.log("    \"encodedOrder\": \"%s\"", vm.toString(abi.encode(order)));
            console.log("  },");
        }
        
        // Vector 3: Order with native token (address(0))
        {
            TokenInfo[] memory inputs = new TokenInfo[](1);
            inputs[0] = TokenInfo({
                token: bytes32(0),  // Native token
                amount: 1 ether
            });
            
            TokenInfo[] memory outputAssets = new TokenInfo[](1);
            outputAssets[0] = TokenInfo({
                token: bytes32(0),  // Native token
                amount: 0.99 ether
            });
            
            Order memory order = Order({
                user: bytes32(uint256(uint160(MOCK_USER))),
                source: hex"45564d2d31",
                destination: hex"45564d2d31",
                deadline: 999999,
                nonce: 0,
                fees: 0,
                session: sessionKeyAddress,
                predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
                inputs: inputs,
                output: PaymentInfo({
                    beneficiary: bytes32(uint256(uint160(MOCK_USER))),
                    assets: outputAssets,
                    call: ""
                })
            });
            
            bytes32 commitment = keccak256(abi.encode(order));
            
            console.log("  {");
            console.log("    \"name\": \"native_token\",");
            console.log("    \"order\": {");
            console.log("      \"user\": \"%s\",", vm.toString(order.user));
            console.log("      \"source\": \"%s\",", vm.toString(order.source));
            console.log("      \"destination\": \"%s\",", vm.toString(order.destination));
            console.log("      \"deadline\": %d,", order.deadline);
            console.log("      \"nonce\": %d,", order.nonce);
            console.log("      \"fees\": \"%d\",", order.fees);
            console.log("      \"session\": \"%s\",", vm.toString(order.session));
            console.log("      \"predispatch\": { \"assets\": [], \"call\": \"0x\" },");
            console.log("      \"inputs\": [{ \"token\": \"%s\", \"amount\": \"%d\" }],", vm.toString(inputs[0].token), inputs[0].amount);
            console.log("      \"output\": {");
            console.log("        \"beneficiary\": \"%s\",", vm.toString(order.output.beneficiary));
            console.log("        \"assets\": [{ \"token\": \"%s\", \"amount\": \"%d\" }],", vm.toString(outputAssets[0].token), outputAssets[0].amount);
            console.log("        \"call\": \"0x\"");
            console.log("      }");
            console.log("    },");
            console.log("    \"commitment\": \"%s\",", vm.toString(commitment));
            console.log("    \"encodedOrder\": \"%s\"", vm.toString(abi.encode(order)));
            console.log("  }");
        }
        
        console.log("  ]");
    }
    
    // =========================================================================
    // EIP-712 Signature Vectors (SelectSolver)
    // =========================================================================
    
    function generateEIP712SignatureVectors() internal {
        console.log("  [");
        
        // We need to compute EIP-712 components manually since we don't have a deployed contract
        bytes32 SELECT_SOLVER_TYPEHASH = keccak256("SelectSolver(bytes32 commitment,address solver)");
        
        // EIP-712 Domain for testing (matching what a real contract would have)
        // Domain: { name: "IntentGatewayV2", version: "1", chainId, verifyingContract }
        bytes32 DOMAIN_TYPEHASH = keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)");
        
        address mockGateway = address(0x1234567890123456789012345678901234567890);
        uint256 chainId = 1;  // Mainnet
        
        bytes32 domainSeparator = keccak256(abi.encode(
            DOMAIN_TYPEHASH,
            keccak256("IntentGatewayV2"),
            keccak256("1"),
            chainId,
            mockGateway
        ));
        
        // Vector 1: Basic signature
        {
            bytes32 commitment = bytes32(uint256(0x1234));
            
            bytes32 structHash = keccak256(abi.encode(
                SELECT_SOLVER_TYPEHASH,
                commitment,
                solverAddress
            ));
            
            bytes32 digest = keccak256(abi.encodePacked("\x19\x01", domainSeparator, structHash));
            
            (uint8 v, bytes32 r, bytes32 s) = vm.sign(SESSION_KEY_PRIVATE_KEY, digest);
            bytes memory signature = abi.encodePacked(r, s, v);
            
            console.log("  {");
            console.log("    \"name\": \"basic_select_solver\",");
            console.log("    \"chainId\": %d,", chainId);
            console.log("    \"verifyingContract\": \"%s\",", vm.toString(mockGateway));
            console.log("    \"commitment\": \"%s\",", vm.toString(commitment));
            console.log("    \"solver\": \"%s\",", vm.toString(solverAddress));
            console.log("    \"sessionKeyAddress\": \"%s\",", vm.toString(sessionKeyAddress));
            console.log("    \"sessionKeyPrivateKey\": \"%s\",", vm.toString(bytes32(SESSION_KEY_PRIVATE_KEY)));
            console.log("    \"SELECT_SOLVER_TYPEHASH\": \"%s\",", vm.toString(SELECT_SOLVER_TYPEHASH));
            console.log("    \"domainSeparator\": \"%s\",", vm.toString(domainSeparator));
            console.log("    \"structHash\": \"%s\",", vm.toString(structHash));
            console.log("    \"digest\": \"%s\",", vm.toString(digest));
            console.log("    \"signature\": \"%s\"", vm.toString(signature));
            console.log("  },");
        }
        
        // Vector 2: Different chain ID
        {
            uint256 chainId2 = 8453;  // Base
            bytes32 domainSeparator2 = keccak256(abi.encode(
                DOMAIN_TYPEHASH,
                keccak256("IntentGatewayV2"),
                keccak256("1"),
                chainId2,
                mockGateway
            ));
            
            bytes32 commitment = bytes32(uint256(0xabcdef));
            
            bytes32 structHash = keccak256(abi.encode(
                SELECT_SOLVER_TYPEHASH,
                commitment,
                solverAddress
            ));
            
            bytes32 digest = keccak256(abi.encodePacked("\x19\x01", domainSeparator2, structHash));
            
            (uint8 v, bytes32 r, bytes32 s) = vm.sign(SESSION_KEY_PRIVATE_KEY, digest);
            bytes memory signature = abi.encodePacked(r, s, v);
            
            console.log("  {");
            console.log("    \"name\": \"different_chain_id\",");
            console.log("    \"chainId\": %d,", chainId2);
            console.log("    \"verifyingContract\": \"%s\",", vm.toString(mockGateway));
            console.log("    \"commitment\": \"%s\",", vm.toString(commitment));
            console.log("    \"solver\": \"%s\",", vm.toString(solverAddress));
            console.log("    \"sessionKeyAddress\": \"%s\",", vm.toString(sessionKeyAddress));
            console.log("    \"sessionKeyPrivateKey\": \"%s\",", vm.toString(bytes32(SESSION_KEY_PRIVATE_KEY)));
            console.log("    \"SELECT_SOLVER_TYPEHASH\": \"%s\",", vm.toString(SELECT_SOLVER_TYPEHASH));
            console.log("    \"domainSeparator\": \"%s\",", vm.toString(domainSeparator2));
            console.log("    \"structHash\": \"%s\",", vm.toString(structHash));
            console.log("    \"digest\": \"%s\",", vm.toString(digest));
            console.log("    \"signature\": \"%s\"", vm.toString(signature));
            console.log("  }");
        }
        
        console.log("  ]");
    }
    
    // =========================================================================
    // UserOp Hash Vectors (ERC-4337 v0.7)
    // =========================================================================
    
    function generateUserOpHashVectors() internal {
        console.log("  [");
        
        // Vector 1: Basic UserOp
        {
            address sender = solverAddress;
            uint256 nonce = 0;
            bytes memory initCode = "";
            bytes memory callData = hex"deadbeef";
            
            // Pack gas limits: verificationGasLimit (16 bytes) | callGasLimit (16 bytes)
            uint128 verificationGasLimit = 100000;
            uint128 callGasLimit = 500000;
            bytes32 accountGasLimits = bytes32(uint256(verificationGasLimit) << 128 | uint256(callGasLimit));
            
            uint256 preVerificationGas = 21000;
            
            // Pack gas fees: maxPriorityFeePerGas (16 bytes) | maxFeePerGas (16 bytes)
            uint128 maxPriorityFeePerGas = 1000000000;  // 1 gwei
            uint128 maxFeePerGas = 50000000000;  // 50 gwei
            bytes32 gasFees = bytes32(uint256(maxPriorityFeePerGas) << 128 | uint256(maxFeePerGas));
            
            bytes memory paymasterAndData = "";
            
            // Compute hash per ERC-4337 v0.7
            bytes32 userOpHashInner = keccak256(abi.encode(
                sender,
                nonce,
                keccak256(initCode),
                keccak256(callData),
                accountGasLimits,
                preVerificationGas,
                gasFees,
                keccak256(paymasterAndData)
            ));
            
            uint256 chainId = 1;
            bytes32 userOpHash = keccak256(abi.encode(userOpHashInner, MOCK_ENTRY_POINT, chainId));
            
            console.log("  {");
            console.log("    \"name\": \"basic_userop\",");
            console.log("    \"userOp\": {");
            console.log("      \"sender\": \"%s\",", vm.toString(sender));
            console.log("      \"nonce\": \"%d\",", nonce);
            console.log("      \"initCode\": \"0x\",");
            console.log("      \"callData\": \"%s\",", vm.toString(callData));
            console.log("      \"accountGasLimits\": \"%s\",", vm.toString(accountGasLimits));
            console.log("      \"preVerificationGas\": \"%d\",", preVerificationGas);
            console.log("      \"gasFees\": \"%s\",", vm.toString(gasFees));
            console.log("      \"paymasterAndData\": \"0x\"");
            console.log("    },");
            console.log("    \"entryPoint\": \"%s\",", vm.toString(MOCK_ENTRY_POINT));
            console.log("    \"chainId\": %d,", chainId);
            console.log("    \"userOpHashInner\": \"%s\",", vm.toString(userOpHashInner));
            console.log("    \"userOpHash\": \"%s\"", vm.toString(userOpHash));
            console.log("  },");
        }
        
        // Vector 2: UserOp with initCode and paymasterAndData
        {
            address sender = solverAddress;
            uint256 nonce = 42;
            bytes memory initCode = hex"aabbccdd";
            bytes memory callData = hex"1234567890abcdef";
            
            uint128 verificationGasLimit = 200000;
            uint128 callGasLimit = 1000000;
            bytes32 accountGasLimits = bytes32(uint256(verificationGasLimit) << 128 | uint256(callGasLimit));
            
            uint256 preVerificationGas = 50000;
            
            uint128 maxPriorityFeePerGas = 2000000000;  // 2 gwei
            uint128 maxFeePerGas = 100000000000;  // 100 gwei
            bytes32 gasFees = bytes32(uint256(maxPriorityFeePerGas) << 128 | uint256(maxFeePerGas));
            
            bytes memory paymasterAndData = hex"feedfacecafebeef";
            
            bytes32 userOpHashInner = keccak256(abi.encode(
                sender,
                nonce,
                keccak256(initCode),
                keccak256(callData),
                accountGasLimits,
                preVerificationGas,
                gasFees,
                keccak256(paymasterAndData)
            ));
            
            uint256 chainId = 8453;  // Base
            bytes32 userOpHash = keccak256(abi.encode(userOpHashInner, MOCK_ENTRY_POINT, chainId));
            
            console.log("  {");
            console.log("    \"name\": \"userop_with_initcode\",");
            console.log("    \"userOp\": {");
            console.log("      \"sender\": \"%s\",", vm.toString(sender));
            console.log("      \"nonce\": \"%d\",", nonce);
            console.log("      \"initCode\": \"%s\",", vm.toString(initCode));
            console.log("      \"callData\": \"%s\",", vm.toString(callData));
            console.log("      \"accountGasLimits\": \"%s\",", vm.toString(accountGasLimits));
            console.log("      \"preVerificationGas\": \"%d\",", preVerificationGas);
            console.log("      \"gasFees\": \"%s\",", vm.toString(gasFees));
            console.log("      \"paymasterAndData\": \"%s\"", vm.toString(paymasterAndData));
            console.log("    },");
            console.log("    \"entryPoint\": \"%s\",", vm.toString(MOCK_ENTRY_POINT));
            console.log("    \"chainId\": %d,", chainId);
            console.log("    \"userOpHashInner\": \"%s\",", vm.toString(userOpHashInner));
            console.log("    \"userOpHash\": \"%s\"", vm.toString(userOpHash));
            console.log("  }");
        }
        
        console.log("  ]");
    }
    
    // =========================================================================
    // Gas Packing Vectors
    // =========================================================================
    
    function generateGasPackingVectors() internal {
        console.log("  [");
        
        // Vector 1: Small values
        {
            uint128 verificationGasLimit = 100000;
            uint128 callGasLimit = 500000;
            // SDK packs as: callGasLimit (16 bytes) | verificationGasLimit (16 bytes)
            // But ERC-4337 v0.7 packs as: verificationGasLimit (16 bytes) | callGasLimit (16 bytes)
            // Let's output both for clarity
            bytes32 packedERC4337 = bytes32(uint256(verificationGasLimit) << 128 | uint256(callGasLimit));
            bytes32 packedSDK = bytes32(uint256(callGasLimit) << 128 | uint256(verificationGasLimit));
            
            uint128 maxPriorityFeePerGas = 1000000000;
            uint128 maxFeePerGas = 50000000000;
            bytes32 gasFeesERC4337 = bytes32(uint256(maxPriorityFeePerGas) << 128 | uint256(maxFeePerGas));
            bytes32 gasFeesSDK = bytes32(uint256(maxPriorityFeePerGas) << 128 | uint256(maxFeePerGas));
            
            console.log("  {");
            console.log("    \"name\": \"small_values\",");
            console.log("    \"callGasLimit\": \"%d\",", callGasLimit);
            console.log("    \"verificationGasLimit\": \"%d\",", verificationGasLimit);
            console.log("    \"accountGasLimits_erc4337\": \"%s\",", vm.toString(packedERC4337));
            console.log("    \"accountGasLimits_sdk\": \"%s\",", vm.toString(packedSDK));
            console.log("    \"maxPriorityFeePerGas\": \"%d\",", maxPriorityFeePerGas);
            console.log("    \"maxFeePerGas\": \"%d\",", maxFeePerGas);
            console.log("    \"gasFees\": \"%s\"", vm.toString(gasFeesERC4337));
            console.log("  },");
        }
        
        // Vector 2: Large values
        {
            uint128 verificationGasLimit = type(uint128).max / 2;
            uint128 callGasLimit = type(uint128).max / 3;
            bytes32 packedERC4337 = bytes32(uint256(verificationGasLimit) << 128 | uint256(callGasLimit));
            bytes32 packedSDK = bytes32(uint256(callGasLimit) << 128 | uint256(verificationGasLimit));
            
            uint128 maxPriorityFeePerGas = type(uint128).max / 4;
            uint128 maxFeePerGas = type(uint128).max / 2;
            bytes32 gasFees = bytes32(uint256(maxPriorityFeePerGas) << 128 | uint256(maxFeePerGas));
            
            console.log("  {");
            console.log("    \"name\": \"large_values\",");
            console.log("    \"callGasLimit\": \"%d\",", callGasLimit);
            console.log("    \"verificationGasLimit\": \"%d\",", verificationGasLimit);
            console.log("    \"accountGasLimits_erc4337\": \"%s\",", vm.toString(packedERC4337));
            console.log("    \"accountGasLimits_sdk\": \"%s\",", vm.toString(packedSDK));
            console.log("    \"maxPriorityFeePerGas\": \"%d\",", maxPriorityFeePerGas);
            console.log("    \"maxFeePerGas\": \"%d\",", maxFeePerGas);
            console.log("    \"gasFees\": \"%s\"", vm.toString(gasFees));
            console.log("  }");
        }
        
        console.log("  ]");
    }
    
    // =========================================================================
    // Bid Signature Vectors (messageHash for solver signature)
    // =========================================================================
    
    function generateBidSignatureVectors() internal {
        console.log("  [");
        
        // The bid signature signs: keccak256(abi.encodePacked(userOpHash, commitment, sessionKey))
        // where sessionKey is an address (20 bytes), not padded
        
        // Vector 1: Basic bid signature
        {
            bytes32 userOpHash = bytes32(uint256(0x1111));
            bytes32 commitment = bytes32(uint256(0x2222));
            address sessionKey = sessionKeyAddress;
            
            // This is what the SDK computes: concat([userOpHash, commitment, sessionKey])
            // In Solidity: abi.encodePacked(userOpHash, commitment, sessionKey) 
            // = 32 bytes + 32 bytes + 20 bytes = 84 bytes
            bytes32 messageHash = keccak256(abi.encodePacked(userOpHash, commitment, sessionKey));
            
            // Sign with solver private key
            (uint8 v, bytes32 r, bytes32 s) = vm.sign(SOLVER_PRIVATE_KEY, 
                keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", messageHash))
            );
            bytes memory signature = abi.encodePacked(r, s, v);
            
            console.log("  {");
            console.log("    \"name\": \"basic_bid_signature\",");
            console.log("    \"userOpHash\": \"%s\",", vm.toString(userOpHash));
            console.log("    \"commitment\": \"%s\",", vm.toString(commitment));
            console.log("    \"sessionKey\": \"%s\",", vm.toString(sessionKey));
            console.log("    \"messageHash\": \"%s\",", vm.toString(messageHash));
            console.log("    \"solverAddress\": \"%s\",", vm.toString(solverAddress));
            console.log("    \"solverPrivateKey\": \"%s\",", vm.toString(bytes32(SOLVER_PRIVATE_KEY)));
            console.log("    \"signature\": \"%s\"", vm.toString(signature));
            console.log("  },");
        }
        
        // Vector 2: Real-looking values
        {
            bytes32 userOpHash = 0x5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a;
            bytes32 commitment = 0xabababababababababababababababababababababababababababababababab;
            address sessionKey = 0x1234567890AbcdEF1234567890aBcdef12345678;
            
            bytes32 messageHash = keccak256(abi.encodePacked(userOpHash, commitment, sessionKey));
            
            (uint8 v, bytes32 r, bytes32 s) = vm.sign(SOLVER_PRIVATE_KEY, 
                keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", messageHash))
            );
            bytes memory signature = abi.encodePacked(r, s, v);
            
            console.log("  {");
            console.log("    \"name\": \"realistic_bid_signature\",");
            console.log("    \"userOpHash\": \"%s\",", vm.toString(userOpHash));
            console.log("    \"commitment\": \"%s\",", vm.toString(commitment));
            console.log("    \"sessionKey\": \"%s\",", vm.toString(sessionKey));
            console.log("    \"messageHash\": \"%s\",", vm.toString(messageHash));
            console.log("    \"solverAddress\": \"%s\",", vm.toString(solverAddress));
            console.log("    \"solverPrivateKey\": \"%s\",", vm.toString(bytes32(SOLVER_PRIVATE_KEY)));
            console.log("    \"signature\": \"%s\"", vm.toString(signature));
            console.log("  }");
        }
        
        console.log("  ]");
    }
}