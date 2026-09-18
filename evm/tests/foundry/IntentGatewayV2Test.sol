// Copyright (C) Polytope Labs Ltd.
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
import {IntentQuoteTestUtils} from "./IntentQuoteTestUtils.sol";

import "forge-std/Test.sol";
import {MainnetForkBaseTest} from "./MainnetForkBaseTest.sol";
import {
    IntentGatewayV2,
    Order,
    Params,
    InitParams,
    ParamsUpdate,
    DestinationFee,
    TokenInfo,
    SweepDust,
    PaymentInfo,
    DispatchInfo,
    FillOptions,
    CancelOptions,
    Deployment,
    WithdrawalRequest,
    SelectOptions
} from "../../src/apps/IntentGatewayV2.sol";
import {deployIntentGatewayImpl, deployIntentModules} from "./IntentGatewayDeploy.sol";
import {IntentsBase} from "../../src/apps/intentsv2/IntentsBase.sol";
import {ExtrinsicIntents} from "../../src/apps/intentsv2/ExtrinsicIntents.sol";
import {HyperApp} from "@hyperbridge/core/apps/HyperApp.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {ERC1967Utils} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Utils.sol";
import {Initializable} from "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {Ownable2StepUpgradeable} from "@openzeppelin/contracts-upgradeable/access/Ownable2StepUpgradeable.sol";
import {PausableUpgradeable} from "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import {ICallDispatcher, Call} from "@hyperbridge/core/interfaces/ICallDispatcher.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {IUniswapV2Router02} from "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";
import {ISwapRouter} from "@uniswap/v3-periphery/contracts/interfaces/ISwapRouter.sol";
import {IQuoter} from "@uniswap/v3-periphery/contracts/interfaces/IQuoter.sol";
import {IncomingPostRequest, IncomingGetResponse} from "@hyperbridge/core/interfaces/IApp.sol";
import {PostRequest, IDispatcher} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {GetRequest, GetResponse, Message} from "@hyperbridge/core/libraries/Message.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";
import {StorageValue} from "@polytope-labs/solidity-merkle-trees/src/trie/Node.sol";

/// @dev `initialize` as the live mainnet implementation (version 2) declares it, before the owner.
interface ILiveGatewayInitialize {
    function initialize(Params memory p, bytes[] memory peerChains, address relayer_) external;
}

contract IntentGatewayV2Test is MainnetForkBaseTest {
    using Message for PostRequest;

    IntentGatewayV2 public intentGateway;

    // Mainnet addresses
    address public constant WETH = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
    address public constant UNISWAP_V3_ROUTER = 0xE592427A0AEce92De3Edee1F18E0157C05861564;
    address public constant UNISWAP_V3_QUOTER = 0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6;

    // Test users
    address public user;
    address public filler;
    // The only account whose deliveries the gateway accepts (see `_deployGatewayProxy`).
    address public relayer;

    // Protocol fee in BPS (30 BPS = 0.3%)
    uint256 public constant PROTOCOL_FEE_BPS = 30;

    // EIP-1967 implementation slot: keccak256("eip1967.proxy.implementation") - 1.
    bytes32 internal constant ERC1967_IMPL_SLOT = 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc;

    function setUp() public override {
        super.setUp();

        // Setup test accounts
        user = makeCleanAddr("user");
        filler = makeCleanAddr("filler");
        relayer = makeCleanAddr("relayer");

        // Deploy IntentGatewayV2
        intentGateway = _deployGatewayProxy();

        // Set params
        Params memory intentParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000, // 100% to protocol, 0% to beneficiary (default)
            protocolFeeBps: 0,
            priceOracle: address(0)
        });
        // Register the peer chains exercised by the cross-chain tests, all bound to this gateway's
        // own address — `_instance` reverts with UnknownInstance for unregistered chains.
        bytes[] memory peers = new bytes[](3);
        peers[0] = host.host();
        peers[1] = bytes("SOURCE_CHAIN");
        peers[2] = bytes("DEST_CHAIN");
        // Armed from init data: only `relayer` may deliver from here on, and the proxy is at 2.
        intentGateway.initialize(
            InitParams({params: intentParams, peerChains: peers, relayer: relayer, owner: address(this)})
        );

        // Fund test accounts
        _fundTestAccounts();
    }

    /// @dev Proxy with empty init data so each test calls `initialize` with its own params and
    /// peers. Its relayer gate stays open unless a test arms it through the host. Production
    /// initializes atomically (see `testAtomicInitialization`).
    function _deployGatewayProxy() internal returns (IntentGatewayV2) {
        IntentGatewayV2 implementation = deployIntentGatewayImpl();
        ERC1967Proxy proxy = new ERC1967Proxy(address(implementation), "");
        return IntentGatewayV2(payable(address(proxy)));
    }

    function _fundTestAccounts() internal {
        // Fund user with ETH and tokens using deal
        vm.deal(user, 10 ether);
        deal(address(usdc), user, 10000 * 1e6); // 10,000 USDC
        deal(address(dai), user, 10000 * 1e18); // 10,000 DAI

        // Fund filler with ETH and tokens
        vm.deal(filler, 100 ether);
        deal(address(usdc), filler, 10000 * 1e6);
        deal(address(dai), filler, 10000 * 1e18);
    }

    /// @dev Helper function to create EIP-712 signature for solver selection
    function _createSelectSolverSignature(bytes32 commitment, address solver, uint256 privateKey, address gateway)
        internal
        view
        returns (bytes memory)
    {
        // Compute the EIP-712 digest using public constants
        IntentGatewayV2 gatewayContract = IntentGatewayV2(payable(gateway));
        bytes32 structHash = keccak256(abi.encode(gatewayContract.SELECT_SOLVER_TYPEHASH(), commitment, solver));

        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", gatewayContract.DOMAIN_SEPARATOR(), structHash));

        // Sign the digest
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(privateKey, digest);

        // Return the signature in the expected format
        return abi.encodePacked(r, s, v);
    }

    function testDustCollectionFromPredispatchSwapWithUniswapV2() public {
        // Test scenario: User wants to swap 1 ETH for DAI using UniswapV2, then escrow the DAI
        uint256 ethAmount = 1 ether;

        // Prepare predispatch call to swap ETH -> DAI via UniswapV2
        address[] memory path = new address[](2);
        path[0] = WETH;
        path[1] = address(dai);

        // Get quote for expected output
        uint256[] memory amounts = _uniswapV2Router.getAmountsOut(ethAmount, path);
        uint256 expectedDaiAmount = amounts[1];
        uint256 minDaiAmount = (expectedDaiAmount * 95) / 100; // 5% slippage tolerance

        bytes memory swapCalldata = abi.encodeWithSelector(
            _uniswapV2Router.swapExactETHForTokens.selector,
            minDaiAmount,
            path,
            address(dispatcher),
            block.timestamp + 3600
        );

        Call[] memory calls = new Call[](1);
        calls[0] = Call({to: address(_uniswapV2Router), value: ethAmount, data: swapCalldata});

        // Setup predispatch info
        TokenInfo[] memory predispatchAssets = new TokenInfo[](1);
        predispatchAssets[0] = TokenInfo({
            token: bytes32(0), // Native token (ETH)
            amount: ethAmount
        });

        DispatchInfo memory predispatch = DispatchInfo({assets: predispatchAssets, call: abi.encode(calls)});

        // Setup order inputs (what will be escrowed)
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: minDaiAmount});

        // Setup order output assets (what filler will provide on destination chain)
        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: 2000 * 1e6 // 2000 USDC
        });

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        // Create order
        Order memory order = Order({
            user: bytes32(0), // Will be set by contract
            source: "", // Will be set by contract
            destination: abi.encodePacked("DEST_CHAIN"),
            deadline: 0,
            nonce: 0, // Will be set by contract
            fees: 0,
            session: address(0),
            predispatch: predispatch,
            inputs: inputs,
            output: output
        });

        // Place order
        vm.startPrank(user);

        // Record events
        vm.recordLogs();

        uint256 daiBalanceBefore = dai.balanceOf(address(intentGateway));

        intentGateway.placeOrder{value: ethAmount}(order, bytes32(0));

        uint256 daiBalanceAfter = dai.balanceOf(address(intentGateway));

        vm.stopPrank();

        // Verify DAI was received
        assertGe(daiBalanceAfter - daiBalanceBefore, minDaiAmount, "Minimum DAI not escrowed");

        // Check for DustCollected event
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool dustCollectedFound = false;

        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                dustCollectedFound = true;
                break;
            }
        }

        assertTrue(dustCollectedFound, "DustCollected event should be emitted");
    }

    /// @dev Mirror of the OrderPlaced event parameters, used to abi.decode raw log data.
    struct OrderPlacedEventData {
        bytes32 user;
        string source;
        string destination;
        uint256 deadline;
        uint256 nonce;
        uint256 fees;
        address session;
        bytes32 beneficiary;
        TokenInfo[] predispatch;
        TokenInfo[] inputs;
        TokenInfo[] outputs;
        bytes predispatchCall;
        bytes outputCall;
        bytes32 graffiti;
    }

    function testOrderPlacedEventCarriesCallPayloadsAndGraffiti() public {
        // Predispatch: swap 1 ETH -> DAI via UniswapV2 so predispatchCall is non-empty.
        uint256 ethAmount = 1 ether;
        address[] memory path = new address[](2);
        path[0] = WETH;
        path[1] = address(dai);
        uint256[] memory amounts = _uniswapV2Router.getAmountsOut(ethAmount, path);
        uint256 minDaiAmount = (amounts[1] * 95) / 100;

        Call[] memory calls = new Call[](1);
        calls[0] = Call({
            to: address(_uniswapV2Router),
            value: ethAmount,
            data: abi.encodeWithSelector(
                _uniswapV2Router.swapExactETHForTokens.selector,
                minDaiAmount,
                path,
                address(dispatcher),
                block.timestamp + 3600
            )
        });

        TokenInfo[] memory predispatchAssets = new TokenInfo[](1);
        predispatchAssets[0] = TokenInfo({token: bytes32(0), amount: ethAmount});

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: minDaiAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 2000 * 1e6});

        Call[] memory outputCalls = new Call[](1);
        outputCalls[0] =
            Call({to: address(usdc), value: 0, data: abi.encodeWithSelector(IERC20.transfer.selector, user, 1)});

        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: abi.encodePacked("DEST_CHAIN"),
            deadline: block.timestamp + 1 hours,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: predispatchAssets, call: abi.encode(calls)}),
            inputs: inputs,
            output: PaymentInfo({
                beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: abi.encode(outputCalls)
            })
        });

        bytes32 graffiti = keccak256("solver-frontend");

        vm.recordLogs();
        vm.prank(user);
        intentGateway.placeOrder{value: ethAmount}(order, graffiti);

        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool found = false;
        OrderPlacedEventData memory ev;
        for (uint256 i = 0; i < entries.length; i++) {
            if (
                entries[i].topics[0]
                    == keccak256(
                        "OrderPlaced(bytes32,string,string,uint256,uint256,uint256,address,bytes32,(bytes32,uint256)[],(bytes32,uint256)[],(bytes32,uint256)[],bytes,bytes,bytes32)"
                    )
            ) {
                // Event data is the bare parameter tuple; prepend an offset word so it
                // decodes as the equivalent tuple-wrapped struct encoding.
                ev = abi.decode(bytes.concat(abi.encode(uint256(0x20)), entries[i].data), (OrderPlacedEventData));
                found = true;
                break;
            }
        }
        assertTrue(found, "OrderPlaced event should be emitted");

        assertEq(ev.predispatchCall, order.predispatch.call, "predispatch calldata should be emitted");
        assertEq(ev.outputCall, order.output.call, "output calldata should be emitted");
        assertEq(ev.graffiti, graffiti, "graffiti should be emitted");

        // The event alone must be sufficient to reconstruct the committed order.
        Order memory fromEvent = Order({
            user: ev.user,
            source: bytes(ev.source),
            destination: bytes(ev.destination),
            deadline: ev.deadline,
            nonce: ev.nonce,
            fees: ev.fees,
            session: ev.session,
            predispatch: DispatchInfo({assets: ev.predispatch, call: ev.predispatchCall}),
            inputs: ev.inputs,
            output: PaymentInfo({beneficiary: ev.beneficiary, assets: ev.outputs, call: ev.outputCall})
        });
        bytes32 commitment = keccak256(abi.encode(fromEvent));
        assertEq(intentGateway._orders(commitment, 0), minDaiAmount, "commitment from event must match escrow");
    }

    function testDustCollectionFromPredispatchSwapWithUniswapV3() public {
        // Test scenario: User wants to swap 1 ETH for USDC using UniswapV3
        uint256 ethAmount = 1 ether;

        // Get quote for expected output and calculate minimum with slippage
        uint256 minUsdcAmount =
            (IQuoter(UNISWAP_V3_QUOTER)
                        .quoteExactInputSingle(
                            WETH,
                            address(usdc),
                            3000, // 0.3% fee tier
                            ethAmount,
                            0
                        )
                    * 95) / 100; // 5% slippage tolerance

        // Prepare predispatch call to swap ETH -> USDC via UniswapV3
        bytes memory swapCalldata = abi.encodeWithSelector(
            ISwapRouter.exactInputSingle.selector,
            ISwapRouter.ExactInputSingleParams({
                tokenIn: WETH,
                tokenOut: address(usdc),
                fee: 3000, // 0.3% fee tier
                recipient: address(dispatcher),
                deadline: block.timestamp + 3600,
                amountIn: ethAmount,
                amountOutMinimum: minUsdcAmount,
                sqrtPriceLimitX96: 0
            })
        );

        // Setup calls: wrap ETH, approve WETH, and swap
        Call[] memory calls = new Call[](3);
        calls[0] = Call({to: WETH, value: ethAmount, data: abi.encodeWithSignature("deposit()")});
        calls[1] = Call({
            to: WETH, value: 0, data: abi.encodeWithSelector(IERC20.approve.selector, UNISWAP_V3_ROUTER, ethAmount)
        });
        calls[2] = Call({to: UNISWAP_V3_ROUTER, value: 0, data: swapCalldata});

        // Setup predispatch info
        TokenInfo[] memory predispatchAssets = new TokenInfo[](1);
        predispatchAssets[0] = TokenInfo({
            token: bytes32(0), // Native token (ETH)
            amount: ethAmount
        });

        DispatchInfo memory predispatch = DispatchInfo({assets: predispatchAssets, call: abi.encode(calls)});

        // Setup order inputs
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: minUsdcAmount});

        // Setup order outputs
        // Setup order output assets (what filler will provide on destination chain)
        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: 2000 * 1e6 // 2000 USDC
        });

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        // Create order
        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: abi.encodePacked("DEST_CHAIN"),
            deadline: 0,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: predispatch,
            inputs: inputs,
            output: output
        });

        // Place order
        vm.startPrank(user);
        vm.recordLogs();

        uint256 usdcBalanceBefore = usdc.balanceOf(address(intentGateway));

        intentGateway.placeOrder{value: ethAmount}(order, bytes32(0));

        uint256 usdcBalanceAfter = usdc.balanceOf(address(intentGateway));

        vm.stopPrank();

        // Verify USDC was received
        assertGe(usdcBalanceAfter - usdcBalanceBefore, minUsdcAmount, "Minimum USDC not escrowed");

        // Check for DustCollected event
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool dustCollectedFound = false;

        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                dustCollectedFound = true;
                break;
            }
        }

        assertTrue(dustCollectedFound, "DustCollected event should be emitted");
    }

    function testDustCollectionFromSolverSingleToken() public {
        // Test that dust is correctly collected when solver provides extra tokens
        uint256 inputAmount = 1000 * 1e6; // 1000 USDC
        uint256 outputAmount = 1000 * 1e18; // 1000 DAI
        uint256 dust = 3 * 1e18; // 3 DAI extra as dust

        // Setup order
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        DispatchInfo memory predispatch = DispatchInfo({assets: new TokenInfo[](0), call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: predispatch,
            inputs: inputs,
            output: output
        });

        // User places order
        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Record gateway DAI balance before fill
        uint256 gatewayDaiBalanceBefore = dai.balanceOf(address(intentGateway));
        uint256 userDaiBalanceBefore = dai.balanceOf(user);

        // Filler fills order with extra tokens (dust)
        vm.startPrank(filler);
        dai.approve(address(intentGateway), outputAmount + dust);
        // Approve fee token for dispatch costs
        dai.approve(address(intentGateway), type(uint256).max);

        vm.recordLogs();

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount + dust});

        FillOptions memory fillOptions = FillOptions({
            relayerFee: 0,
            nativeDispatchFee: 0,
            validUntil: 0,
            outputs: solverOutputs,
            inputs: IntentQuoteTestUtils.inputs(order, solverOutputs)
        });
        intentGateway.fillOrder(order, fillOptions);

        vm.stopPrank();

        // Verify user received exact requested amount
        assertEq(
            dai.balanceOf(user) - userDaiBalanceBefore, outputAmount, "User should receive exactly the requested amount"
        );

        // Verify gateway collected the exact dust amount
        assertEq(
            dai.balanceOf(address(intentGateway)) - gatewayDaiBalanceBefore,
            dust,
            "Gateway should hold exactly the dust amount"
        );

        // Check DustCollected event was emitted with correct values
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool eventFound = false;
        uint256 dustAmountFromEvent = 0;

        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                eventFound = true;
                // Decode event to verify values
                (address token, uint256 amount) = abi.decode(entries[i].data, (address, uint256));
                assertEq(token, address(dai), "Token should be DAI");
                dustAmountFromEvent = amount;
                break;
            }
        }

        assertTrue(eventFound, "DustCollected event should be emitted");
        assertEq(dustAmountFromEvent, dust, "Event dust amount should match expected");
    }

    function testDustCollectionFromSolverNativeToken() public {
        // Test that dust is correctly collected when solver provides extra native tokens
        uint256 inputAmount = 1000 * 1e6; // 1000 USDC
        uint256 outputAmount = 1 ether; // 1 ETH
        uint256 dust = 0.1 ether; // 0.1 ETH extra as dust

        // Setup order
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        // Setup order output assets
        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({
            token: bytes32(0), // Native token
            amount: outputAmount
        });

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        DispatchInfo memory predispatch = DispatchInfo({assets: new TokenInfo[](0), call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: predispatch,
            inputs: inputs,
            output: output
        });

        // User places order
        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Record gateway ETH balance before fill
        uint256 gatewayEthBalanceBefore = address(intentGateway).balance;
        uint256 userEthBalanceBefore = user.balance;

        // Filler fills order with extra native tokens (dust)
        vm.startPrank(filler);
        // Approve fee token for dispatch costs
        dai.approve(address(intentGateway), type(uint256).max);

        vm.recordLogs();

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(0), amount: outputAmount + dust});

        FillOptions memory fillOptions = FillOptions({
            relayerFee: 0,
            nativeDispatchFee: 0,
            validUntil: 0,
            outputs: solverOutputs,
            inputs: IntentQuoteTestUtils.inputs(order, solverOutputs)
        });
        intentGateway.fillOrder{value: outputAmount + dust}(order, fillOptions);

        vm.stopPrank();

        // Verify user received exact requested amount
        assertEq(
            user.balance - userEthBalanceBefore, outputAmount, "User should receive exactly the requested ETH amount"
        );

        // Verify gateway collected the exact dust amount
        assertEq(
            address(intentGateway).balance - gatewayEthBalanceBefore,
            dust,
            "Gateway should hold exactly the ETH dust amount"
        );

        // Check DustCollected event was emitted with correct values
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool eventFound = false;
        uint256 dustAmountFromEvent = 0;

        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                // Decode event to verify values
                (address token, uint256 amount) = abi.decode(entries[i].data, (address, uint256));
                if (token == address(0)) {
                    eventFound = true;
                    dustAmountFromEvent = amount;
                    break;
                }
            }
        }

        assertTrue(eventFound, "DustCollected event should be emitted for native token");
        assertEq(dustAmountFromEvent, dust, "Event dust amount should match expected");
    }

    function testNoDustCollectionWhenExactAmount() public {
        // Test that no dust is collected when solver provides exact amount
        IntentGatewayV2 zeroFeeGateway = _deployGatewayProxy();

        Params memory zeroFeeParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 5000,
            protocolFeeBps: 0,
            priceOracle: address(0)
        });
        zeroFeeGateway.initialize(
            InitParams({params: zeroFeeParams, peerChains: new bytes[](0), relayer: address(0), owner: address(this)})
        );

        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        DispatchInfo memory predispatch = DispatchInfo({assets: new TokenInfo[](0), call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: predispatch,
            inputs: inputs,
            output: output
        });

        // User places order
        vm.startPrank(user);
        usdc.approve(address(zeroFeeGateway), inputAmount);
        zeroFeeGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Filler fills order
        vm.startPrank(filler);
        dai.approve(address(zeroFeeGateway), 1000 * 1e18);
        // Approve fee token for dispatch costs
        dai.approve(address(zeroFeeGateway), type(uint256).max);

        vm.recordLogs();

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        FillOptions memory fillOptions = FillOptions({
            relayerFee: 0,
            nativeDispatchFee: 0,
            validUntil: 0,
            outputs: solverOutputs,
            inputs: IntentQuoteTestUtils.inputs(order, solverOutputs)
        });
        zeroFeeGateway.fillOrder(order, fillOptions);

        vm.stopPrank();

        // No DustCollected event should be emitted when solver provides exact amounts
        Vm.Log[] memory entries = vm.getRecordedLogs();
        for (uint256 i = 0; i < entries.length; i++) {
            assertTrue(
                entries[i].topics[0] != keccak256("DustCollected(address,uint256)"),
                "DustCollected event should not be emitted when no dust"
            );
        }
    }

    function testPredispatchFailsWithInsufficientBalance() public {
        // Test that predispatch reverts if swap doesn't produce enough tokens
        uint256 ethAmount = 0.01 ether; // Very small amount

        address[] memory path = new address[](2);
        path[0] = WETH;
        path[1] = address(dai);

        // Get quote for expected output
        uint256[] memory amounts = _uniswapV2Router.getAmountsOut(ethAmount, path);
        uint256 expectedDaiFromSwap = amounts[1];
        uint256 unrealisticDaiAmount = expectedDaiFromSwap * 10; // Request 10x more than possible
        uint256 minDaiAmount = (expectedDaiFromSwap * 95) / 100; // 5% slippage tolerance

        bytes memory swapCalldata = abi.encodeWithSelector(
            _uniswapV2Router.swapExactETHForTokens.selector,
            minDaiAmount,
            path,
            address(dispatcher),
            block.timestamp + 3600
        );

        Call[] memory calls = new Call[](1);
        calls[0] = Call({to: address(_uniswapV2Router), value: ethAmount, data: swapCalldata});

        TokenInfo[] memory predispatchAssets = new TokenInfo[](1);
        predispatchAssets[0] = TokenInfo({token: bytes32(0), amount: ethAmount});

        DispatchInfo memory predispatch = DispatchInfo({assets: predispatchAssets, call: abi.encode(calls)});

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: unrealisticDaiAmount});

        // Setup order output assets
        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: 2000 * 1e6 // 2000 USDC
        });

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: abi.encodePacked("DEST_CHAIN"),
            deadline: 0,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: predispatch,
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        intentGateway.placeOrder{value: ethAmount}(order, bytes32(0));
        vm.stopPrank();
    }

    function testSweepDustERC20() public {
        // Simulate accumulated dust in the gateway
        uint256 feeAmount = 1000 * 1e6;

        // Transfer tokens to gateway instead of using deal
        vm.prank(user);
        usdc.transfer(address(intentGateway), feeAmount);

        // Setup fee collection request
        address treasury = user; // Use existing user address
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: feeAmount // Sweep exact amount
        });

        SweepDust memory sweepDustReq = SweepDust({beneficiary: treasury, outputs: outputs});

        // Create sweep dust request from hyperbridge
        bytes memory data = abi.encode(sweepDustReq);
        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: bytes.concat(bytes1(uint8(IntentsBase.RequestKind.SweepDust)), data),
            timeoutTimestamp: 0
        });

        vm.recordLogs();

        uint256 treasuryBalanceBefore = usdc.balanceOf(treasury);
        uint256 gatewayBalanceBefore = usdc.balanceOf(address(intentGateway));

        // Verify gateway has the funds before collection
        assertEq(gatewayBalanceBefore, feeAmount, "Gateway should have funds before collection");

        // Execute dust sweep
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        // Verify dust was transferred
        assertEq(usdc.balanceOf(treasury) - treasuryBalanceBefore, feeAmount, "Treasury should receive protocol fees");
        assertEq(usdc.balanceOf(address(intentGateway)), 0, "Gateway should have no USDC left");

        // Verify DustSwept event was emitted
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool eventFound = false;
        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustSwept(address,uint256,address)")) {
                eventFound = true;
                break;
            }
        }
        assertTrue(eventFound, "DustSwept event should be emitted");
    }

    function testSweepDustNative() public {
        // Simulate accumulated ETH dust
        uint256 feeAmount = 1 ether;
        vm.deal(address(intentGateway), feeAmount);

        address treasury = user; // Use existing user address that can receive ETH
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(0), amount: feeAmount});

        SweepDust memory sweepDustReq = SweepDust({beneficiary: treasury, outputs: outputs});

        bytes memory data = abi.encode(sweepDustReq);
        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: bytes.concat(bytes1(uint8(IntentsBase.RequestKind.SweepDust)), data),
            timeoutTimestamp: 0
        });

        uint256 treasuryBalanceBefore = treasury.balance;

        // Verify gateway has the funds before sweep
        assertEq(address(intentGateway).balance, feeAmount, "Gateway should have ETH before sweep");

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        assertEq(treasury.balance - treasuryBalanceBefore, feeAmount, "Treasury should receive ETH dust");
        assertEq(address(intentGateway).balance, 0, "Gateway should have no ETH left");
    }

    function testSweepMultipleTokenDust() public {
        // Fund gateway with multiple tokens
        uint256 usdcAmount = 500 * 1e6;
        uint256 daiAmount = 1000 * 1e18;
        uint256 ethAmount = 0.5 ether;

        // Transfer tokens to gateway
        vm.startPrank(user);
        usdc.transfer(address(intentGateway), usdcAmount);
        dai.transfer(address(intentGateway), daiAmount);
        vm.stopPrank();
        vm.deal(address(intentGateway), ethAmount);

        address treasury = user; // Use existing user address that can receive ETH
        TokenInfo[] memory outputs = new TokenInfo[](3);
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: usdcAmount});
        outputs[1] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: daiAmount});
        outputs[2] = TokenInfo({token: bytes32(0), amount: ethAmount});

        SweepDust memory sweepDustReq = SweepDust({beneficiary: treasury, outputs: outputs});

        bytes memory data = abi.encode(sweepDustReq);
        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: bytes.concat(bytes1(uint8(IntentsBase.RequestKind.SweepDust)), data),
            timeoutTimestamp: 0
        });

        uint256 usdcBalanceBefore = usdc.balanceOf(treasury);
        uint256 daiBalanceBefore = dai.balanceOf(treasury);
        uint256 ethBalanceBefore = treasury.balance;

        // Verify gateway has all funds before collection
        assertEq(usdc.balanceOf(address(intentGateway)), usdcAmount, "Gateway should have USDC");
        assertEq(dai.balanceOf(address(intentGateway)), daiAmount, "Gateway should have DAI");
        assertEq(address(intentGateway).balance, ethAmount, "Gateway should have ETH");

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        // Verify all dust was swept
        assertEq(usdc.balanceOf(treasury) - usdcBalanceBefore, usdcAmount, "Treasury should receive USDC");
        assertEq(dai.balanceOf(treasury) - daiBalanceBefore, daiAmount, "Treasury should receive DAI");
        assertEq(treasury.balance - ethBalanceBefore, ethAmount, "Treasury should receive ETH");

        // Gateway should be empty
        assertEq(usdc.balanceOf(address(intentGateway)), 0, "Gateway USDC should be 0");
        assertEq(dai.balanceOf(address(intentGateway)), 0, "Gateway DAI should be 0");
        assertEq(address(intentGateway).balance, 0, "Gateway ETH should be 0");
    }

    function testSurplusSplitBetweenBeneficiaryAndProtocol() public {
        // Test 50/50 split: solver provides 2100 DAI, user gets 2050, protocol gets 50
        IntentGatewayV2 customGateway = _deployGatewayProxy();
        Params memory customParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 5000, // 50% to protocol, 50% to beneficiary
            protocolFeeBps: 0,
            priceOracle: address(0)
        });
        customGateway.initialize(
            InitParams({params: customParams, peerChains: new bytes[](0), relayer: address(0), owner: address(this)})
        );

        uint256 solverOutputAmount = 2100 * 1e18;

        // Setup order
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 1000 * 1e6});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: abi.encodePacked(host.host()),
            destination: abi.encodePacked(host.host()),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        // User places order
        vm.startPrank(user);
        usdc.approve(address(customGateway), 1000 * 1e6);
        customGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Filler fills order with surplus
        vm.startPrank(filler);
        dai.approve(address(customGateway), 2200 * 1e18); // Approve surplus + fees

        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: solverOutputAmount});

        uint256 userDaiBalanceBefore = dai.balanceOf(user);

        vm.recordLogs();
        customGateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: outputs,
                inputs: IntentQuoteTestUtils.inputs(order, outputs)
            })
        );
        vm.stopPrank();

        // Verify beneficiary received 2050 DAI (2000 + 50% of 100 surplus)
        assertEq(dai.balanceOf(user) - userDaiBalanceBefore, 2050 * 1e18, "Beneficiary gets 50% surplus");

        // Verify protocol received 50 DAI (50% of 100 surplus)
        assertEq(dai.balanceOf(address(customGateway)), 50 * 1e18, "Protocol gets 50% surplus");

        // Verify DustCollected event
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool found = false;
        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                found = true;
                (address token, uint256 amount) = abi.decode(entries[i].data, (address, uint256));
                assertEq(token, address(dai), "Token should be DAI");
                assertEq(amount, 50 * 1e18, "Amount should be 50 DAI");
                break;
            }
        }
        assertTrue(found, "DustCollected event should be emitted");
    }

    function testSurplusSplitWith100PercentToBeneficiary() public {
        // Test with 100% surplus going to beneficiary (0% to protocol)
        IntentGatewayV2 customGateway = _deployGatewayProxy();
        Params memory customParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 0, // 0% to protocol, 100% to beneficiary
            protocolFeeBps: 0,
            priceOracle: address(0)
        });
        customGateway.initialize(
            InitParams({params: customParams, peerChains: new bytes[](0), relayer: address(0), owner: address(this)})
        );

        uint256 solverOutputAmount = 2100 * 1e18; // 100 DAI surplus

        // Setup order
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 1000 * 1e6});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: abi.encodePacked(host.host()),
            destination: abi.encodePacked(host.host()),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        // User places order
        vm.startPrank(user);
        usdc.approve(address(customGateway), 1000 * 1e6);
        customGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Filler fills order with surplus
        vm.startPrank(filler);
        dai.approve(address(customGateway), 2200 * 1e18); // Approve surplus + fees

        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: solverOutputAmount});

        uint256 userDaiBalanceBefore = dai.balanceOf(user);

        customGateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: outputs,
                inputs: IntentQuoteTestUtils.inputs(order, outputs)
            })
        );
        vm.stopPrank();

        // Verify beneficiary received requested amount + all surplus (2000 + 100 = 2100 DAI)
        assertEq(dai.balanceOf(user) - userDaiBalanceBefore, 2100 * 1e18, "Beneficiary should receive 100% of surplus");

        // Verify protocol received nothing
        assertEq(dai.balanceOf(address(customGateway)), 0, "Protocol should receive 0%");
    }

    function testSurplusSplitWith0PercentToBeneficiary() public {
        // Test 0/100 split: solver provides 2100 DAI, user gets 2000, protocol gets 100
        IntentGatewayV2 customGateway = _deployGatewayProxy();
        Params memory customParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000, // 100% to protocol, 0% to beneficiary
            protocolFeeBps: 0,
            priceOracle: address(0)
        });
        customGateway.initialize(
            InitParams({params: customParams, peerChains: new bytes[](0), relayer: address(0), owner: address(this)})
        );

        uint256 solverOutputAmount = 2100 * 1e18; // 100 DAI surplus

        // Setup order
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 1000 * 1e6});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: abi.encodePacked(host.host()),
            destination: abi.encodePacked(host.host()),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        // User places order
        vm.startPrank(user);
        usdc.approve(address(customGateway), 1000 * 1e6);
        customGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Filler fills order with surplus
        vm.startPrank(filler);
        dai.approve(address(customGateway), 2200 * 1e18); // Approve surplus + fees

        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: solverOutputAmount});

        uint256 userDaiBalanceBefore = dai.balanceOf(user);

        vm.recordLogs();
        customGateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: outputs,
                inputs: IntentQuoteTestUtils.inputs(order, outputs)
            })
        );
        vm.stopPrank();

        // Verify beneficiary received only requested amount (2000 DAI, no surplus)
        assertEq(dai.balanceOf(user) - userDaiBalanceBefore, 2000 * 1e18, "Beneficiary should receive only requested");

        // Verify protocol received all surplus (100 DAI)
        assertEq(dai.balanceOf(address(customGateway)), 100 * 1e18, "Protocol should receive 100% of surplus");

        // Verify DustCollected event was emitted
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool found = false;
        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                found = true;
                (address token, uint256 amount) = abi.decode(entries[i].data, (address, uint256));
                assertEq(token, address(dai), "Token should be DAI");
                assertEq(amount, 100 * 1e18, "Amount should be 100 DAI");
                break;
            }
        }
        assertTrue(found, "DustCollected event should be emitted");
    }

    function testSurplusWithCalldataGoesToProtocol() public {
        // Test that when calldata is present, ALL surplus goes to protocol
        // Compare: without calldata and 50% split, protocol gets 50 DAI
        //          with calldata and 50% split, protocol gets 100 DAI (all surplus)
        IntentGatewayV2 customGateway = _deployGatewayProxy();
        customGateway.initialize(
            InitParams({
                params: Params({
                    host: address(host),
                    dispatcher: address(dispatcher),
                    solverSelection: false,
                    surplusShareBps: 5000,
                    protocolFeeBps: 0,
                    priceOracle: address(0)
                }),
                peerChains: new bytes[](0),
                relayer: address(0),
                owner: address(this)
            })
        );

        // Setup order WITH calldata
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 1000 * 1e6});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});

        // Create postdispatch calls with a simple token approval (non-reverting)
        Call[] memory postdispatchCalls = new Call[](1);
        postdispatchCalls[0] = Call({
            to: address(dai),
            value: 0,
            data: abi.encodeWithSelector(IERC20.approve.selector, address(intentGateway), 1000 * 1e18)
        });

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: abi.encodePacked(host.host()),
            destination: abi.encodePacked(host.host()),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: PaymentInfo({
                beneficiary: bytes32(uint256(uint160(user))),
                assets: outputAssets,
                call: abi.encode(postdispatchCalls) // Valid non-reverting calldata
            })
        });

        // Place and fill order
        vm.prank(user);
        usdc.approve(address(customGateway), 1000 * 1e6);
        vm.prank(user);
        customGateway.placeOrder(order, bytes32(0));

        vm.prank(filler);
        dai.approve(address(customGateway), 2200 * 1e18);

        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2100 * 1e18});

        uint256 userBalanceBefore = dai.balanceOf(user);
        uint256 gatewayBalanceBefore = dai.balanceOf(address(customGateway));

        vm.recordLogs();
        vm.prank(filler);
        customGateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: outputs,
                inputs: IntentQuoteTestUtils.inputs(order, outputs)
            })
        );

        // Verify beneficiary got ONLY requested amount (2000 DAI, no surplus)
        assertEq(
            dai.balanceOf(user) - userBalanceBefore, 2000 * 1e18, "Beneficiary should get 0% surplus with calldata"
        );

        // Verify protocol got ALL surplus (100 DAI, not 50 which would be with 50% split)
        assertEq(
            dai.balanceOf(address(customGateway)) - gatewayBalanceBefore,
            100 * 1e18,
            "Protocol should get 100% surplus with calldata"
        );

        // Verify DustCollected event shows full 100 DAI
        Vm.Log[] memory entries = vm.getRecordedLogs();
        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                (, uint256 amount) = abi.decode(entries[i].data, (address, uint256));
                assertEq(amount, 100 * 1e18, "All surplus should go to protocol with calldata");
                return; // Test passed
            }
        }
        fail("DustCollected event not found");
    }

    function testSweepDustUnauthorized() public {
        // Fund gateway
        vm.prank(user);
        usdc.transfer(address(intentGateway), 1000 * 1e6);

        address treasury = user; // Use existing user address
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 1000 * 1e6});

        SweepDust memory sweepDustReq = SweepDust({beneficiary: treasury, outputs: outputs});

        bytes memory data = abi.encode(sweepDustReq);

        // Request NOT from hyperbridge
        PostRequest memory request = PostRequest({
            source: bytes("UNAUTHORIZED_CHAIN"),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(0x1234)),
            to: abi.encodePacked(address(intentGateway)),
            body: bytes.concat(bytes1(uint8(IntentsBase.RequestKind.SweepDust)), data),
            timeoutTimestamp: 0
        });

        vm.prank(address(host));
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));
    }

    function testFillOrderWithNoPostdispatch() public {
        // Test that fillOrder works correctly when there's no postdispatch calldata
        uint256 inputAmount = 1000 * 1e6; // 1000 USDC
        uint256 outputAmount = 1000 * 1e18; // 1000 DAI

        // Setup order with no postdispatch
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        DispatchInfo memory predispatch = DispatchInfo({assets: new TokenInfo[](0), call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: predispatch,
            inputs: inputs,
            output: output
        });

        // User places order
        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Filler fills order with no postdispatch in dispatcher
        vm.startPrank(filler);
        dai.approve(address(intentGateway), type(uint256).max);

        uint256 userDaiBalanceBefore = dai.balanceOf(user);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        FillOptions memory fillOptions = FillOptions({
            relayerFee: 0,
            nativeDispatchFee: 0,
            validUntil: 0,
            outputs: solverOutputs,
            inputs: IntentQuoteTestUtils.inputs(order, solverOutputs)
        });
        intentGateway.fillOrder(order, fillOptions);

        vm.stopPrank();

        // Verify user received DAI
        assertEq(dai.balanceOf(user) - userDaiBalanceBefore, outputAmount, "User should receive output amount");
    }

    function testPostdispatchTokenSweep() public {
        // Test realistic postdispatch: exact output swap on Uniswap V2 where refunded input tokens are swept
        // Scenario: User wants 1000 DAI on destination, solver sends USDC to dispatcher,
        // dispatcher swaps exact output for DAI, refunded USDC is swept back to gateway

        uint256 inputAmount = 1000 * 1e6; // 1000 USDC escrow
        uint256 daiOutputAmount = 1000 * 1e18; // Exact 1000 DAI output wanted

        // Setup order inputs
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        // Create postdispatch calls that:
        // 1. Approve Uniswap router to spend USDC
        // 2. Execute exact output swap (swapTokensForExactTokens) - USDC -> DAI
        // 3. Transfer DAI to user
        Call[] memory postdispatchCalls = new Call[](3);

        // Get quote for how much USDC needed for 1000 DAI (will be less than what solver sends)
        address[] memory path = new address[](2);
        path[0] = address(usdc);
        path[1] = address(dai);
        address uniswapRouter = 0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D;
        uint256[] memory amounts = IUniswapV2Router02(uniswapRouter).getAmountsIn(daiOutputAmount, path);
        uint256 usdcNeeded = amounts[0];

        // Call 1: Approve Uniswap router
        postdispatchCalls[0] = Call({
            to: address(usdc),
            value: 0,
            data: abi.encodeWithSelector(IERC20.approve.selector, uniswapRouter, type(uint256).max)
        });

        // Call 2: Exact output swap - swap USDC for exactly 1000 DAI
        postdispatchCalls[1] = Call({
            to: uniswapRouter,
            value: 0,
            data: abi.encodeWithSelector(
                bytes4(keccak256("swapTokensForExactTokens(uint256,uint256,address[],address,uint256)")),
                daiOutputAmount, // exact amount out
                type(uint256).max, // max amount in
                path,
                address(dispatcher), // tokens come back to dispatcher
                block.timestamp
            )
        });

        // Call 3: Transfer DAI to user
        postdispatchCalls[2] = Call({
            to: address(dai), value: 0, data: abi.encodeWithSelector(IERC20.transfer.selector, user, daiOutputAmount)
        });

        // Setup order output - beneficiary is dispatcher, it will receive USDC from solver
        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: usdcNeeded + 100 * 1e6}); // Solver sends more than needed

        PaymentInfo memory output = PaymentInfo({
            beneficiary: bytes32(uint256(uint160(address(dispatcher)))), // Dispatcher receives USDC
            assets: outputAssets,
            call: abi.encode(postdispatchCalls)
        });

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        // Record gateway balance before order placement
        uint256 gatewayUsdcBefore = usdc.balanceOf(address(intentGateway));

        // User places order
        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Record user DAI balance before fill
        uint256 userDaiBalanceBefore = dai.balanceOf(user);

        // Filler fills order - sends USDC to dispatcher
        vm.startPrank(filler);
        uint256 solverUsdcAmount = usdcNeeded + 100 * 1e6; // Solver sends extra USDC
        usdc.approve(address(intentGateway), solverUsdcAmount);
        dai.approve(address(intentGateway), type(uint256).max);

        vm.recordLogs();

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: solverUsdcAmount});

        intentGateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: solverOutputs,
                inputs: IntentQuoteTestUtils.inputs(order, solverOutputs)
            })
        );

        vm.stopPrank();

        // Verify user received exact DAI output
        assertEq(dai.balanceOf(user) - userDaiBalanceBefore, daiOutputAmount, "User should receive exact DAI output");

        // Verify dispatcher has 0 USDC balance (refunded tokens were swept)
        assertEq(usdc.balanceOf(address(dispatcher)), 0, "Dispatcher should have 0 USDC after sweep");

        // Verify IntentGateway received the refunded USDC (difference between what solver sent and what swap used)
        // Note: Gateway balance change = +escrow (from user) - escrow (to solver) + swept dust
        // Net change should be approximately the swept dust amount
        uint256 gatewayUsdcAfter = usdc.balanceOf(address(intentGateway));
        uint256 refundedUsdc = solverUsdcAmount - usdcNeeded;
        uint256 netChange = gatewayUsdcAfter - gatewayUsdcBefore;
        assertGt(netChange, 0, "IntentGateway should receive refunded USDC");

        // The net change should be approximately the refunded amount (allowing for small swap variance)
        // This accounts for: user escrow in (+1000), solver redemption out (-1000), dust swept in (+refunded)
        assertApproxEqAbs(
            netChange,
            refundedUsdc,
            5 * 1e6, // 5 USDC tolerance for swap price variance
            "Gateway net balance change should be approximately the refunded USDC"
        );

        // Check DustCollected event was emitted for swept USDC
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool dustEventFound = false;
        uint256 dustAmountFromEvent = 0;

        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                (address token, uint256 amount) = abi.decode(entries[i].data, (address, uint256));
                if (token == address(usdc) && amount > 0) {
                    dustEventFound = true;
                    dustAmountFromEvent = amount;
                    break;
                }
            }
        }

        assertTrue(dustEventFound, "DustCollected event should be emitted for swept refunded USDC");
        assertGt(dustAmountFromEvent, 0, "Dust amount should be greater than 0");
    }

    // ============================================
    // Solver Selection Tests
    // ============================================

    function testSelect() public {
        // Test solver selection with valid session signature
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: vm.addr(1), // Session key
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        bytes32 commitment = keccak256(abi.encode(order));

        // Create EIP-712 signature from session key
        bytes memory sessionSignature = _createSelectSolverSignature(
            commitment,
            filler,
            1, // Session key private key
            address(intentGateway)
        );

        // Solver selects themselves
        vm.prank(filler);
        intentGateway.select(SelectOptions({commitment: commitment, solver: filler, signature: sessionSignature}));
    }

    function testFillOrderWithSolverSelection() public {
        // Enable solver selection
        Params memory newParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: true,
            surplusShareBps: 10000,
            protocolFeeBps: 0,
            priceOracle: address(0)
        });

        IntentGatewayV2 gatewayWithSelection = _deployGatewayProxy();
        gatewayWithSelection.initialize(
            InitParams({params: newParams, peerChains: new bytes[](0), relayer: address(0), owner: address(this)})
        );

        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: vm.addr(1), // Session key
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(gatewayWithSelection), inputAmount);
        gatewayWithSelection.placeOrder(order, bytes32(0));
        vm.stopPrank();

        bytes32 commitment = keccak256(abi.encode(order));

        // Create EIP-712 signature from session key
        bytes memory sessionSignature = _createSelectSolverSignature(
            commitment,
            filler,
            1, // Session key private key
            address(gatewayWithSelection)
        );

        // Solver selects themselves
        vm.startPrank(filler);
        gatewayWithSelection.select(
            SelectOptions({commitment: commitment, solver: filler, signature: sessionSignature})
        );

        // Filler fills order
        dai.approve(address(gatewayWithSelection), type(uint256).max);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        gatewayWithSelection.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: solverOutputs,
                inputs: IntentQuoteTestUtils.inputs(order, solverOutputs)
            })
        );
        vm.stopPrank();
    }

    function testFillOrderWithWrongSolver() public {
        // Enable solver selection
        Params memory newParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: true,
            surplusShareBps: 10000,
            protocolFeeBps: 0,
            priceOracle: address(0)
        });

        IntentGatewayV2 gatewayWithSelection = _deployGatewayProxy();
        gatewayWithSelection.initialize(
            InitParams({params: newParams, peerChains: new bytes[](0), relayer: address(0), owner: address(this)})
        );

        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: vm.addr(1),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(gatewayWithSelection), inputAmount);
        gatewayWithSelection.placeOrder(order, bytes32(0));
        vm.stopPrank();

        bytes32 commitment = keccak256(abi.encode(order));

        // Create EIP-712 signature from session key for filler
        bytes memory sessionSignature = _createSelectSolverSignature(
            commitment,
            filler,
            1, // Session key private key
            address(gatewayWithSelection)
        );

        // Solver selects filler
        vm.prank(filler);
        gatewayWithSelection.select(
            SelectOptions({commitment: commitment, solver: filler, signature: sessionSignature})
        );

        // Different address tries to fill - should revert
        address wrongSolver = address(0x9999);
        deal(address(dai), wrongSolver, 10000 * 1e18);

        vm.startPrank(wrongSolver);
        dai.approve(address(gatewayWithSelection), 1000 * 1e18);
        dai.approve(address(gatewayWithSelection), type(uint256).max);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        vm.expectRevert(IntentsBase.Unauthorized.selector);
        gatewayWithSelection.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: solverOutputs,
                inputs: IntentQuoteTestUtils.inputs(order, solverOutputs)
            })
        );
        vm.stopPrank();
    }

    // ============================================
    // fillOrder Edge Case Tests
    // ============================================

    function testFillOrderExpired() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 10,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Roll past deadline
        vm.roll(block.number + 11);

        vm.startPrank(filler);
        dai.approve(address(intentGateway), 1000 * 1e18);
        dai.approve(address(intentGateway), type(uint256).max);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        vm.expectRevert(IntentsBase.Expired.selector);
        intentGateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: solverOutputs,
                inputs: IntentQuoteTestUtils.inputs(order, solverOutputs)
            })
        );
        vm.stopPrank();
    }

    /// @dev On Arbitrum the gateway must read the L2 block number from the ArbSys precompile;
    /// the `block.number` opcode there returns the (much smaller) L1 block number, which would
    /// keep expired orders fillable forever.
    function testFillOrderExpiredOnArbitrumUsesArbSys() public {
        uint256 inputAmount = 1000 * 1e6;
        // A deadline denominated in Arbitrum L2 blocks, far above both the current
        // fork's block.number and any L1 block number.
        uint256 l2Deadline = 400_000_000;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: l2Deadline,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Pretend we're on Arbitrum One: ArbSys reports an L2 block number past the deadline
        // while block.number (~L1 height on Arbitrum) remains far below it.
        vm.chainId(42161);
        vm.etch(address(100), hex"fe"); // Arbitrum precompiles expose 0xfe as their code
        vm.mockCall(address(100), abi.encodeWithSignature("arbBlockNumber()"), abi.encode(l2Deadline + 1));
        assertLt(block.number, l2Deadline);

        vm.startPrank(filler);
        dai.approve(address(intentGateway), type(uint256).max);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        vm.expectRevert(IntentsBase.Expired.selector);
        intentGateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: solverOutputs,
                inputs: IntentQuoteTestUtils.inputs(order, solverOutputs)
            })
        );
        vm.stopPrank();
    }

    /// @dev Counterpart to the expiry test: an L2-denominated deadline still in the future
    /// (per ArbSys) is fillable on Arbitrum.
    function testFillOrderNotExpiredOnArbitrumUsesArbSys() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 l2Deadline = 400_000_000;

        // Pretend we're on Arbitrum One for the entire order lifecycle — the host derives
        // its state machine id from block.chainid, so it must be consistent between
        // placeOrder and fillOrder.
        vm.chainId(42161);
        vm.etch(address(100), hex"fe");
        vm.mockCall(address(100), abi.encodeWithSignature("arbBlockNumber()"), abi.encode(l2Deadline));

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: l2Deadline,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        vm.startPrank(filler);
        dai.approve(address(intentGateway), type(uint256).max);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        intentGateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: solverOutputs,
                inputs: IntentQuoteTestUtils.inputs(order, solverOutputs)
            })
        );
        vm.stopPrank();

        assertEq(intentGateway._filled(keccak256(abi.encode(order))), filler);
    }

    function testFillOrderAlreadyFilled() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        vm.startPrank(filler);
        dai.approve(address(intentGateway), 2000 * 1e18);
        dai.approve(address(intentGateway), type(uint256).max);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        // Fill once
        intentGateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: solverOutputs,
                inputs: IntentQuoteTestUtils.inputs(order, solverOutputs)
            })
        );

        // Try to fill again - should revert
        vm.expectRevert(IntentsBase.Filled.selector);
        intentGateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: solverOutputs,
                inputs: IntentQuoteTestUtils.inputs(order, solverOutputs)
            })
        );
        vm.stopPrank();
    }

    function testFillOrderWrongChain() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: bytes("DIFFERENT_CHAIN"),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        vm.startPrank(filler);
        dai.approve(address(intentGateway), 1000 * 1e18);
        dai.approve(address(intentGateway), type(uint256).max);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        vm.expectRevert(IntentsBase.WrongChain.selector);
        intentGateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: solverOutputs,
                inputs: IntentQuoteTestUtils.inputs(order, solverOutputs)
            })
        );
        vm.stopPrank();
    }

    function testFillOrderPartialAmount_IsValidPartialFill() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 1000 * 1e18;
        uint256 partialAmount = 500 * 1e18;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        uint256 userDaiBefore = dai.balanceOf(user);
        uint256 fillerUsdcBefore = usdc.balanceOf(filler);

        vm.startPrank(filler);
        dai.approve(address(intentGateway), partialAmount);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: partialAmount});

        intentGateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: solverOutputs,
                inputs: IntentQuoteTestUtils.inputs(order, solverOutputs)
            })
        );
        vm.stopPrank();

        // User receives partial output
        assertEq(dai.balanceOf(user), userDaiBefore + partialAmount, "User should receive partial DAI");

        // Filler receives proportional input: 1000 * 500 / 1000 = 500 USDC
        uint256 expectedInputRelease = (inputAmount * partialAmount) / outputAmount;
        assertEq(
            usdc.balanceOf(filler), fillerUsdcBefore + expectedInputRelease, "Filler should receive proportional USDC"
        );
    }

    function testFillOrderInsufficientNativeToken() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(0), amount: 1 ether});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        vm.startPrank(filler);
        dai.approve(address(intentGateway), type(uint256).max);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(0), amount: 1 ether});

        vm.expectRevert(IntentsBase.InsufficientNativeToken.selector);
        intentGateway.fillOrder{value: 0.5 ether}(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: solverOutputs,
                inputs: IntentQuoteTestUtils.inputs(order, solverOutputs)
            })
        );
        vm.stopPrank();
    }

    // ============================================
    // Order Cancellation Tests
    // ============================================

    function testCancelOrder() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Roll past deadline
        vm.roll(block.number + 101);

        CancelOptions memory cancelOptions = CancelOptions({relayerFee: 0, height: uint64(order.deadline + 1)});

        vm.startPrank(user);
        dai.approve(address(intentGateway), type(uint256).max);

        vm.expectEmit(true, false, false, true, address(intentGateway));
        emit IntentsBase.OrderCancelled(keccak256(abi.encode(order)), user);

        intentGateway.cancelOrder(order, cancelOptions);
        vm.stopPrank();
    }

    function testCancelOrderUnauthorized() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Same-chain cancellation remains owner-only through the exact deadline block.
        vm.roll(order.deadline);

        CancelOptions memory cancelOptions = CancelOptions({relayerFee: 0, height: uint64(order.deadline)});

        // Different user tries to cancel
        vm.startPrank(filler);
        dai.approve(address(intentGateway), type(uint256).max);
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        intentGateway.cancelOrder(order, cancelOptions);
        vm.stopPrank();
    }

    function testCancelOrderNotExpired() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: bytes("DEST_CHAIN"), // Different chain for cross-chain test
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Try to cancel before deadline
        CancelOptions memory cancelOptions = CancelOptions({relayerFee: 0, height: uint64(block.number + 50)});

        vm.startPrank(user);
        dai.approve(address(intentGateway), type(uint256).max);
        vm.expectRevert(IntentsBase.NotExpired.selector);
        intentGateway.cancelOrder(order, cancelOptions);
        vm.stopPrank();
    }

    // ============================================
    // placeOrder Edge Case Tests
    // ============================================

    function testPlaceOrderWithFees() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 feeAmount = 100 * 1e18;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: feeAmount,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        dai.approve(address(intentGateway), feeAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();
    }

    function testPlaceOrderInvalidInput() public {
        TokenInfo[] memory inputs = new TokenInfo[](0); // Empty inputs

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();
    }

    // ============================================
    // onAccept Variant Tests
    // ============================================

    function testOnAcceptRedeemEscrow() public {
        // This is tested indirectly by fillOrder tests
        // The fillOrder dispatches RedeemEscrow message which is handled by onAccept
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        bytes32 commitment = keccak256(abi.encode(order));

        // Simulate RedeemEscrow request from IntentGateway on another chain
        bytes memory body = bytes.concat(
            bytes1(uint8(IntentsBase.RequestKind.RedeemEscrow)),
            abi.encode(
                WithdrawalRequest({
                    commitment: commitment, tokens: inputs, beneficiary: bytes32(uint256(uint160(filler)))
                })
            )
        );

        PostRequest memory request = PostRequest({
            source: host.host(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: body,
            timeoutTimestamp: 0
        });

        uint256 fillerBalanceBefore = usdc.balanceOf(filler);

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        assertEq(usdc.balanceOf(filler) - fillerBalanceBefore, inputAmount, "Filler should receive escrowed tokens");
    }

    function testOnAcceptRefundEscrow() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        bytes32 commitment = keccak256(abi.encode(order));

        // Simulate RedeemEscrow request
        bytes memory body = bytes.concat(
            bytes1(uint8(IntentsBase.RequestKind.RedeemEscrow)),
            abi.encode(
                WithdrawalRequest({
                    commitment: commitment, tokens: inputs, beneficiary: bytes32(uint256(uint160(user)))
                })
            )
        );

        PostRequest memory request = PostRequest({
            source: host.host(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: body,
            timeoutTimestamp: 0
        });

        uint256 userBalanceBefore = usdc.balanceOf(user);

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        assertEq(usdc.balanceOf(user) - userBalanceBefore, inputAmount, "User should receive refunded tokens");
    }

    function testCancelOrderFromDestinationChain() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        // Create cross-chain order: source is SOURCE_CHAIN, destination is current chain
        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: bytes("SOURCE_CHAIN"),
            destination: host.host(), // Current chain is destination
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        bytes32 commitment = keccak256(abi.encode(order));

        // User cancels from destination chain - no escrow here, it's on source
        CancelOptions memory cancelOptions = CancelOptions({relayerFee: 0, height: 0});

        vm.startPrank(user);
        dai.approve(address(intentGateway), type(uint256).max);

        vm.expectEmit(true, false, false, true, address(intentGateway));
        emit IntentsBase.OrderCancelled(commitment, user);

        intentGateway.cancelOrder(order, cancelOptions);
        vm.stopPrank();

        // Verify order is marked as cancelled
        address filledBy = intentGateway._filled(commitment);
        assertEq(filledBy, user, "Order should be marked as cancelled by user");
    }

    function testCancelOrderFromDestinationChainDispatchesRefundRequest() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: bytes("SOURCE_CHAIN"),
            destination: host.host(),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        CancelOptions memory cancelOptions = CancelOptions({relayerFee: 1 ether, height: 0});

        // Expect the dispatch call to host
        vm.expectCall(address(host), abi.encodeWithSignature("dispatch((bytes,bytes,bytes,uint64,uint256,address))"));

        vm.startPrank(user);
        dai.approve(address(intentGateway), type(uint256).max);
        intentGateway.cancelOrder{value: 0.1 ether}(order, cancelOptions);
        vm.stopPrank();
    }

    function testFillOrderAfterDestinationChainCancellationFails() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: bytes("SOURCE_CHAIN"),
            destination: host.host(),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        // User cancels from destination
        CancelOptions memory cancelOptions = CancelOptions({relayerFee: 0, height: 0});
        vm.startPrank(user);
        dai.approve(address(intentGateway), type(uint256).max);
        intentGateway.cancelOrder(order, cancelOptions);
        vm.stopPrank();

        // Now solver tries to fill the order
        FillOptions memory fillOptions = FillOptions({
            outputs: outputAssets,
            relayerFee: 0,
            nativeDispatchFee: 0,
            validUntil: 0,
            inputs: IntentQuoteTestUtils.inputs(order, outputAssets)
        });

        vm.startPrank(filler);
        dai.approve(address(intentGateway), 1000 * 1e18);
        vm.expectRevert(IntentsBase.Filled.selector);
        intentGateway.fillOrder(order, fillOptions);
        vm.stopPrank();
    }

    function testCancelOrderFromSourceChainDispatchesGetRequest() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        // Cross-chain order placed here; this chain is the source.
        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: bytes("DEST_CHAIN"),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        vm.roll(block.number + 101);

        bytes32 commitment = keccak256(abi.encode(order));
        CancelOptions memory cancelOptions = CancelOptions({relayerFee: 1 ether, height: uint64(order.deadline + 1)});

        // The GET dispatch is the only other trace this route leaves, and it carries no
        // reference to the order — `OrderCancelled` is what keys the cancel to a commitment.
        vm.expectCall(
            address(host), abi.encodeWithSignature("dispatch((bytes,uint64,bytes[],uint64,uint256,bytes,address))")
        );
        vm.expectEmit(true, false, false, true, address(intentGateway));
        emit IntentsBase.OrderCancelled(commitment, user);

        vm.prank(user);
        intentGateway.cancelOrder{value: 0.1 ether}(order, cancelOptions);

        // Escrow is untouched until the GET response comes back through `onGetResponse`.
        assertEq(intentGateway._orders(commitment, 0), inputAmount, "Escrow should still be held");
    }

    function testCancelOrderFromWrongChainFails() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        // Order where current chain is neither source nor destination
        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: bytes("SOURCE_CHAIN"),
            destination: bytes("DEST_CHAIN"),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        CancelOptions memory cancelOptions = CancelOptions({relayerFee: 0, height: 0});

        vm.startPrank(user);
        vm.expectRevert(IntentsBase.WrongChain.selector);
        intentGateway.cancelOrder(order, cancelOptions);
        vm.stopPrank();
    }

    function testCancelSameChainOrderBelongingToAnotherChainFails() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        // Same-chain order (source == destination), but for a chain that is not this one. The
        // same-chain route is selected on `source == destination` alone, so without this check a
        // foreign order would reach `_cancelSameChain` and be refunded against escrow held here.
        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: bytes("SOURCE_CHAIN"),
            destination: bytes("SOURCE_CHAIN"),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        CancelOptions memory cancelOptions = CancelOptions({relayerFee: 0, height: 0});

        vm.prank(user);
        vm.expectRevert(IntentsBase.WrongChain.selector);
        intentGateway.cancelOrder(order, cancelOptions);
    }

    function testRefundEscrowOnSourceChainAfterDestinationCancellation() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: bytes("DEST_CHAIN"),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        // Place order on source chain
        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        bytes32 commitment = keccak256(abi.encode(order));

        // Simulate RefundEscrow request from destination chain
        bytes memory body = bytes.concat(
            bytes1(uint8(IntentsBase.RequestKind.RefundEscrow)),
            abi.encode(
                WithdrawalRequest({
                    commitment: commitment, tokens: inputs, beneficiary: bytes32(uint256(uint160(user)))
                })
            )
        );

        PostRequest memory request = PostRequest({
            source: bytes("DEST_CHAIN"),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: body,
            timeoutTimestamp: 0
        });

        uint256 userBalanceBefore = usdc.balanceOf(user);

        vm.expectEmit(true, false, false, false);
        emit IntentsBase.EscrowRefunded(commitment, inputs);

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        assertEq(usdc.balanceOf(user) - userBalanceBefore, inputAmount, "User should receive refunded tokens");
        assertEq(intentGateway._filled(commitment), user, "Order should be marked as refunded with user as beneficiary");
    }

    function testCancelOrderFromDestinationChainUnauthorizedBeforeExpiry() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        // Create cross-chain order: source is SOURCE_CHAIN, destination is current chain
        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: bytes("SOURCE_CHAIN"),
            destination: host.host(), // Current chain is destination
            deadline: block.number + 100, // Not expired
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        CancelOptions memory cancelOptions = CancelOptions({relayerFee: 0, height: 0});

        // Non-owner (filler) tries to cancel before expiry - should fail
        vm.startPrank(filler);
        dai.approve(address(intentGateway), type(uint256).max);
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        intentGateway.cancelOrder(order, cancelOptions);
        vm.stopPrank();
    }

    function testCancelOrderFromDestinationChainByAnyoneAfterExpiry() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        // Create cross-chain order: source is SOURCE_CHAIN, destination is current chain
        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: bytes("SOURCE_CHAIN"),
            destination: host.host(), // Current chain is destination
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        // Roll past deadline
        vm.roll(block.number + 101);

        bytes32 commitment = keccak256(abi.encode(order));
        CancelOptions memory cancelOptions = CancelOptions({relayerFee: 0, height: 0});

        // Anyone (filler) can cancel after expiry
        vm.startPrank(filler);
        dai.approve(address(intentGateway), type(uint256).max);

        // `canceller` is msg.sender, which on this route need not be the order's creator.
        vm.expectEmit(true, false, false, true, address(intentGateway));
        emit IntentsBase.OrderCancelled(commitment, filler);

        intentGateway.cancelOrder(order, cancelOptions);
        vm.stopPrank();

        // Verify order is marked as cancelled
        address filledBy = intentGateway._filled(commitment);
        assertEq(filledBy, user, "Order should be marked as cancelled with user as beneficiary");
    }

    function testOnAcceptNewDeployment() public {
        bytes memory stateMachineId = bytes("NEW_CHAIN");
        address gateway = address(0x1234);

        Deployment memory deployment = Deployment({chain: stateMachineId, gateway: gateway});

        bytes memory body = bytes.concat(bytes1(uint8(IntentsBase.RequestKind.NewDeployment)), abi.encode(deployment));

        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: body,
            timeoutTimestamp: 0
        });

        vm.recordLogs();

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        // Check DeploymentAdded event
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool eventFound = false;

        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DeploymentAdded(string,address)")) {
                eventFound = true;
                break;
            }
        }

        assertTrue(eventFound, "DeploymentAdded event should be emitted");

        // Verify instance was stored
        assertEq(intentGateway.instance(stateMachineId), gateway, "Gateway instance should be stored");
    }

    function testOnAcceptUpdateParams() public {
        Params memory newParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: true,
            surplusShareBps: 10000,
            protocolFeeBps: 0,
            priceOracle: address(0)
        });

        DestinationFee[] memory emptyFees = new DestinationFee[](0);
        ParamsUpdate memory update = ParamsUpdate({params: newParams, destinationFees: emptyFees});

        bytes memory body = bytes.concat(bytes1(uint8(IntentsBase.RequestKind.UpdateParams)), abi.encode(update));

        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: body,
            timeoutTimestamp: 0
        });

        vm.recordLogs();

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        // Check ParamsUpdated event
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool eventFound = false;

        for (uint256 i = 0; i < entries.length; i++) {
            if (
                entries[i].topics[0]
                    == keccak256(
                        "ParamsUpdated((address,address,bool,uint256,uint256,address),(address,address,bool,uint256,uint256,address))"
                    )
            ) {
                eventFound = true;
                break;
            }
        }

        assertTrue(eventFound, "ParamsUpdated event should be emitted");

        // Verify params were updated
        Params memory updatedParams = intentGateway.params();
        assertEq(updatedParams.host, newParams.host, "Host should be updated");
        assertEq(updatedParams.dispatcher, newParams.dispatcher, "Dispatcher should be updated");
        assertEq(updatedParams.solverSelection, newParams.solverSelection, "SolverSelection should be updated");
    }

    function testOnAcceptUpdateParamsWithDestinationFees() public {
        Params memory newParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000,
            protocolFeeBps: 100,
            priceOracle: address(0)
        });

        // Create destination fees
        DestinationFee[] memory destinationFees = new DestinationFee[](2);
        bytes memory arbitrumStateMachineId = bytes("ARBITRUM");
        bytes memory optimismStateMachineId = bytes("OPTIMISM");

        destinationFees[0] = DestinationFee({destinationFeeBps: 50, chain: arbitrumStateMachineId});

        destinationFees[1] = DestinationFee({destinationFeeBps: 150, chain: optimismStateMachineId});

        ParamsUpdate memory update = ParamsUpdate({params: newParams, destinationFees: destinationFees});

        bytes memory body = bytes.concat(bytes1(uint8(IntentsBase.RequestKind.UpdateParams)), abi.encode(update));

        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: body,
            timeoutTimestamp: 0
        });

        vm.recordLogs();

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        // Check events
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool paramsUpdatedFound = false;
        uint256 destinationFeeEventsFound = 0;

        for (uint256 i = 0; i < entries.length; i++) {
            if (
                entries[i].topics[0]
                    == keccak256(
                        "ParamsUpdated((address,address,bool,uint256,uint256,address),(address,address,bool,uint256,uint256,address))"
                    )
            ) {
                paramsUpdatedFound = true;
            }

            if (entries[i].topics[0] == keccak256("DestinationProtocolFeeUpdated(string,uint256)")) {
                destinationFeeEventsFound++;
            }
        }

        assertTrue(paramsUpdatedFound, "ParamsUpdated event should be emitted");
        assertEq(destinationFeeEventsFound, 2, "Should emit 2 DestinationProtocolFeeUpdated events");

        // Verify params were updated
        Params memory updatedParams = intentGateway.params();
        assertEq(updatedParams.host, newParams.host, "Host should be updated");
        assertEq(updatedParams.protocolFeeBps, newParams.protocolFeeBps, "ProtocolFeeBps should be updated");
    }

    function testOnAcceptUpdateParamsWithEmptyDestinationFees() public {
        Params memory newParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000,
            protocolFeeBps: 200,
            priceOracle: address(0)
        });

        // Empty destination fees array
        DestinationFee[] memory emptyFees = new DestinationFee[](0);

        ParamsUpdate memory update = ParamsUpdate({params: newParams, destinationFees: emptyFees});

        bytes memory body = bytes.concat(bytes1(uint8(IntentsBase.RequestKind.UpdateParams)), abi.encode(update));

        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: body,
            timeoutTimestamp: 0
        });

        vm.recordLogs();

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        // Check events
        Vm.Log[] memory entries = vm.getRecordedLogs();
        uint256 destinationFeeEventsFound = 0;

        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DestinationProtocolFeeUpdated(string,uint256)")) {
                destinationFeeEventsFound++;
            }
        }

        assertEq(destinationFeeEventsFound, 0, "Should not emit DestinationProtocolFeeUpdated events for empty array");

        // Verify params were updated
        Params memory updatedParams = intentGateway.params();
        assertEq(updatedParams.protocolFeeBps, 200, "ProtocolFeeBps should be updated");
    }

    function testDestinationProtocolFeeUpdatedEventData() public {
        Params memory newParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000,
            protocolFeeBps: 100,
            priceOracle: address(0)
        });

        bytes memory arbitrumStateMachineId = bytes("ARBITRUM");
        uint256 feeBps = 75;

        DestinationFee[] memory destinationFees = new DestinationFee[](1);
        destinationFees[0] = DestinationFee({destinationFeeBps: feeBps, chain: arbitrumStateMachineId});

        ParamsUpdate memory update = ParamsUpdate({params: newParams, destinationFees: destinationFees});

        bytes memory body = bytes.concat(bytes1(uint8(IntentsBase.RequestKind.UpdateParams)), abi.encode(update));

        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: body,
            timeoutTimestamp: 0
        });

        vm.expectEmit(true, false, false, true);
        emit IntentsBase.DestinationProtocolFeeUpdated(string(arbitrumStateMachineId), feeBps);

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));
    }

    function testPlaceOrderWithDestinationSpecificFee() public {
        // Setup: Set default protocol fee to 1% and destination-specific fee to 0.5%
        IntentGatewayV2 customGateway = _deployGatewayProxy();
        Params memory customParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000,
            protocolFeeBps: 100, // 1% default
            priceOracle: address(0)
        });
        customGateway.initialize(
            InitParams({params: customParams, peerChains: new bytes[](0), relayer: address(0), owner: address(this)})
        );

        // Set destination-specific fee via governance
        bytes memory destinationChain = bytes("ARBITRUM");

        DestinationFee[] memory destinationFees = new DestinationFee[](1);
        destinationFees[0] = DestinationFee({
            destinationFeeBps: 50, // 0.5% for this destination
            chain: destinationChain
        });

        ParamsUpdate memory update = ParamsUpdate({params: customParams, destinationFees: destinationFees});

        bytes memory body = bytes.concat(bytes1(uint8(IntentsBase.RequestKind.UpdateParams)), abi.encode(update));

        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(customGateway)),
            to: abi.encodePacked(address(customGateway)),
            body: body,
            timeoutTimestamp: 0
        });

        vm.prank(address(host));
        customGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        // Place an order to the destination with specific fee
        uint256 inputAmount = 1000 * 1e6; // 1000 USDC
        uint256 expectedDestinationFee = (inputAmount * 50) / 10000; // 5 USDC (0.5%)

        deal(address(usdc), user, inputAmount);

        Order memory order = Order({
            user: bytes32(0),
            source: bytes(""),
            destination: destinationChain, // Use the destination with specific fee
            deadline: block.timestamp + 1 hours,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: new TokenInfo[](1),
            output: PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: new TokenInfo[](1), call: ""})
        });

        order.inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});
        order.output.assets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});

        vm.startPrank(user);
        usdc.approve(address(customGateway), inputAmount);

        vm.recordLogs();
        customGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        _assertPendingPlacementFee(customGateway, order, vm.getRecordedLogs(), expectedDestinationFee);
        assertEq(usdc.balanceOf(address(customGateway)), inputAmount, "Gateway should have full input amount");
    }

    function testPlaceOrderDestinationFeeWithFallback() public {
        // Test that when destination fee is not set (or is 0), it falls back to default protocol fee
        IntentGatewayV2 customGateway = _deployGatewayProxy();
        Params memory customParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000,
            protocolFeeBps: 100, // 1% default
            priceOracle: address(0)
        });
        customGateway.initialize(
            InitParams({params: customParams, peerChains: new bytes[](0), relayer: address(0), owner: address(this)})
        );

        // Place order to destination without specific fee set
        uint256 inputAmount = 1000 * 1e6; // 1000 USDC
        uint256 expectedDefaultFee = (inputAmount * 100) / 10000; // 10 USDC (1% default)

        deal(address(usdc), user, inputAmount);

        bytes memory unknownDestination = bytes("UNKNOWN_CHAIN");

        Order memory order = Order({
            user: bytes32(0),
            source: bytes(""),
            destination: unknownDestination,
            deadline: block.timestamp + 1 hours,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: new TokenInfo[](1),
            output: PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: new TokenInfo[](1), call: ""})
        });

        order.inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});
        order.output.assets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});

        vm.startPrank(user);
        usdc.approve(address(customGateway), inputAmount);

        vm.recordLogs();
        customGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        _assertPendingPlacementFee(customGateway, order, vm.getRecordedLogs(), expectedDefaultFee);
    }

    // ============================================
    // Helper Function Tests
    // ============================================

    function testInstance() public {
        bytes memory stateMachineId = bytes("TEST_CHAIN");

        // An unregistered chain reverts with UnknownInstance.
        vm.expectRevert(IntentsBase.UnknownInstance.selector);
        intentGateway.instance(stateMachineId);

        // Register an explicit override deployment
        address gateway = address(0xABCD);
        Deployment memory deployment = Deployment({chain: stateMachineId, gateway: gateway});

        bytes memory body = bytes.concat(bytes1(uint8(IntentsBase.RequestKind.NewDeployment)), abi.encode(deployment));

        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: body,
            timeoutTimestamp: 0
        });

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        // Now should return the stored gateway
        address instance = intentGateway.instance(stateMachineId);
        assertEq(instance, gateway, "Should return stored gateway address");
    }

    function testCalculateCommitmentSlotHash() public view {
        bytes32 commitment = keccak256("test_commitment");
        bytes memory slotHash = intentGateway.calculateCommitmentSlotHash(commitment);

        assertGt(slotHash.length, 0, "Should return non-empty slot hash");
    }

    function testParams() public view {
        Params memory currentParams = intentGateway.params();

        assertEq(currentParams.host, address(host), "Host should match");
        assertEq(currentParams.dispatcher, address(dispatcher), "Dispatcher should match");
        assertEq(currentParams.solverSelection, false, "SolverSelection should be false");
    }

    function testHost() public view {
        address hostAddr = intentGateway.host();
        assertEq(hostAddr, address(host), "Host address should match");
    }

    function testReceive() public {
        uint256 amount = 1 ether;
        uint256 balanceBefore = address(intentGateway).balance;

        vm.deal(user, 10 ether);
        vm.prank(user);
        (bool sent,) = address(intentGateway).call{value: amount}("");

        assertTrue(sent, "ETH transfer should succeed");
        assertEq(address(intentGateway).balance, balanceBefore + amount, "Contract should receive ETH");
    }

    function testOnGetResponse() public {
        // Test successful cancellation via GET response
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        bytes32 commitment = keccak256(abi.encode(order));

        // Partial-fill-aware cancel context: per-leg _partialFills proof. An empty proof value
        // decodes to filled=0, so the full escrow is refundable.
        uint256[] memory totalRequired = new uint256[](1);
        totalRequired[0] = outputAssets[0].amount;
        bytes memory context = abi.encode(commitment, bytes32(uint256(uint160(user))), inputs, totalRequired);

        bytes[] memory keys = new bytes[](1);
        keys[0] = abi.encodePacked(_partialFillSlot(commitment, 0));
        StorageValue[] memory values = new StorageValue[](1);
        values[0] = StorageValue({key: keys[0], value: new bytes(0)}); // Empty value = not filled

        GetRequest memory getRequest = GetRequest({
            source: host.host(),
            dest: order.destination,
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            keys: keys,
            height: 0,
            timeoutTimestamp: 0,
            context: context
        });

        GetResponse memory getResponse = GetResponse({request: getRequest, values: values});

        IncomingGetResponse memory incoming = IncomingGetResponse({response: getResponse, relayer: relayer});

        uint256 userBalanceBefore = usdc.balanceOf(user);

        vm.prank(address(host));
        intentGateway.onGetResponse(incoming);

        assertEq(usdc.balanceOf(user) - userBalanceBefore, inputAmount, "User should receive refunded tokens");
    }

    // ============================================
    // Protocol Fee Tests
    // ============================================

    function _assertPendingPlacementFee(
        IntentGatewayV2 gateway,
        Order memory originalOrder,
        Vm.Log[] memory entries,
        uint256 expectedFee
    ) internal view {
        for (uint256 i; i < entries.length; i++) {
            if (entries[i].emitter == address(gateway)) {
                assertTrue(
                    entries[i].topics[0] != keccak256("DustCollected(address,uint256)"), "held fee is not revenue"
                );
            }
        }
        // Copy before normalizing so callers can still inspect their original gross-input order.
        Order memory placed = abi.decode(abi.encode(originalOrder), (Order));
        placed.user = bytes32(uint256(uint160(user)));
        placed.source = host.host();
        placed.nonce = gateway._nonce() - 1;
        placed.inputs[0].amount -= expectedFee;
        (uint256 fee, uint256 committed) = gateway._protocolFees(keccak256(abi.encode(placed)), 0);
        assertEq(fee, expectedFee, "exact placement fee held for this order");
        assertEq(committed, placed.inputs[0].amount, "original post-fee input");
    }

    function testProtocolFeeWith1Percent() public {
        // Test with 1% protocol fee (100 basis points)
        IntentGatewayV2 customGateway = _deployGatewayProxy();
        Params memory customParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000,
            protocolFeeBps: 100, // 1%
            priceOracle: address(0)
        });
        bytes[] memory peers = new bytes[](1);
        peers[0] = host.host();
        customGateway.initialize(
            InitParams({params: customParams, peerChains: peers, relayer: address(0), owner: address(this)})
        );

        uint256 inputAmount = 1000 * 1e6; // 1000 USDC
        uint256 expectedProtocolFee = (inputAmount * 100) / 10000; // 10 USDC
        uint256 expectedAmountAfterFee = inputAmount - expectedProtocolFee; // 990 USDC

        deal(address(usdc), user, inputAmount);

        Order memory order = Order({
            user: bytes32(0),
            source: bytes(""),
            destination: host.host(),
            deadline: block.timestamp + 1 hours,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: new TokenInfo[](1),
            output: PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: new TokenInfo[](1), call: ""})
        });

        order.inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});
        order.output.assets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});

        vm.startPrank(user);
        usdc.approve(address(customGateway), inputAmount);

        vm.recordLogs();
        customGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        _assertPendingPlacementFee(customGateway, order, vm.getRecordedLogs(), expectedProtocolFee);
        assertEq(usdc.balanceOf(address(customGateway)), inputAmount, "Gateway holds principal and pending fee");

        // Verify commitment is calculated with REDUCED amounts
        // Need to reconstruct the order exactly as the contract sees it after filling in fields
        Order memory orderWithReducedAmount = order;
        orderWithReducedAmount.user = bytes32(uint256(uint160(user)));
        orderWithReducedAmount.source = host.host();
        orderWithReducedAmount.nonce = 0; // First order
        orderWithReducedAmount.inputs[0].amount = expectedAmountAfterFee;
        bytes32 expectedCommitment = keccak256(abi.encode(orderWithReducedAmount));

        // Calculate storage slot for _orders[commitment][leg 0]
        // _orders is at storage slot 9 (see forge inspect storage-layout)
        // For nested mappings: keccak256(abi.encode(innerKey, keccak256(abi.encode(outerKey, baseSlot))))
        bytes32 commitmentSlot = keccak256(abi.encode(expectedCommitment, uint256(9)));
        bytes32 escrowSlot = keccak256(abi.encode(uint256(0), commitmentSlot));

        // Verify escrow storage contains REDUCED amount (not full amount)
        uint256 escrowedAmount = uint256(vm.load(address(customGateway), escrowSlot));
        assertEq(escrowedAmount, expectedAmountAfterFee, "Escrowed amount should be reduced (990 USDC)");

        // Test redemption with reduced amount works correctly
        TokenInfo[] memory redeemInputs = new TokenInfo[](1);
        redeemInputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: expectedAmountAfterFee});

        bytes memory body = bytes.concat(
            bytes1(uint8(IntentsBase.RequestKind.RedeemEscrow)),
            abi.encode(
                WithdrawalRequest({
                    commitment: expectedCommitment, tokens: redeemInputs, beneficiary: bytes32(uint256(uint160(filler)))
                })
            )
        );

        PostRequest memory request = PostRequest({
            source: host.host(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(customGateway)),
            to: abi.encodePacked(address(customGateway)),
            body: body,
            timeoutTimestamp: 0
        });

        uint256 fillerBalanceBefore = usdc.balanceOf(filler);
        vm.prank(address(host));
        customGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        // Filler receives the REDUCED amount (after protocol fees)
        assertEq(
            usdc.balanceOf(filler) - fillerBalanceBefore,
            expectedAmountAfterFee,
            "Filler should receive reduced amount (990 USDC)"
        );
    }

    function testProtocolFeeWith10Percent() public {
        // Test with 10% protocol fee (1000 basis points)
        IntentGatewayV2 customGateway = _deployGatewayProxy();
        Params memory customParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000,
            protocolFeeBps: 1000, // 10%
            priceOracle: address(0)
        });
        customGateway.initialize(
            InitParams({params: customParams, peerChains: new bytes[](0), relayer: address(0), owner: address(this)})
        );

        uint256 inputAmount = 1000 * 1e6; // 1000 USDC
        uint256 expectedProtocolFee = (inputAmount * 1000) / 10000; // 100 USDC
        uint256 expectedAmountAfterFee = inputAmount - expectedProtocolFee; // 900 USDC

        deal(address(usdc), user, inputAmount);

        Order memory order = Order({
            user: bytes32(0),
            source: bytes(""),
            destination: host.host(),
            deadline: block.timestamp + 1 hours,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: new TokenInfo[](1),
            output: PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: new TokenInfo[](1), call: ""})
        });

        order.inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});
        order.output.assets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});

        vm.startPrank(user);
        usdc.approve(address(customGateway), inputAmount);

        vm.recordLogs();
        customGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        _assertPendingPlacementFee(customGateway, order, vm.getRecordedLogs(), expectedProtocolFee);
        assertEq(usdc.balanceOf(address(customGateway)), inputAmount, "Gateway holds principal and pending fee");

        // Verify commitment is calculated with REDUCED amounts
        // Need to reconstruct the order exactly as the contract sees it after filling in fields
        Order memory orderWithReducedAmount = order;
        orderWithReducedAmount.user = bytes32(uint256(uint160(user)));
        orderWithReducedAmount.source = host.host();
        orderWithReducedAmount.nonce = 0; // First order
        orderWithReducedAmount.inputs[0].amount = expectedAmountAfterFee;
        bytes32 expectedCommitment = keccak256(abi.encode(orderWithReducedAmount));

        // Calculate storage slot for _orders[commitment][leg 0]
        bytes32 commitmentSlot = keccak256(abi.encode(expectedCommitment, uint256(9)));
        bytes32 escrowSlot = keccak256(abi.encode(uint256(0), commitmentSlot));

        // Verify escrow storage contains REDUCED amount
        uint256 escrowedAmount = uint256(vm.load(address(customGateway), escrowSlot));
        assertEq(escrowedAmount, expectedAmountAfterFee, "Escrowed amount should be reduced (900 USDC)");
    }

    function testProtocolFeeWithZeroPercent() public {
        // Test with 0% protocol fee - should not emit DustCollected
        IntentGatewayV2 customGateway = _deployGatewayProxy();
        Params memory customParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000,
            protocolFeeBps: 0, // 0%
            priceOracle: address(0)
        });
        customGateway.initialize(
            InitParams({params: customParams, peerChains: new bytes[](0), relayer: address(0), owner: address(this)})
        );

        uint256 inputAmount = 1000 * 1e6; // 1000 USDC

        deal(address(usdc), user, inputAmount);

        Order memory order = Order({
            user: bytes32(0),
            source: bytes(""),
            destination: host.host(),
            deadline: block.timestamp + 1 hours,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: new TokenInfo[](1),
            output: PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: new TokenInfo[](1), call: ""})
        });

        order.inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});
        order.output.assets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});

        vm.startPrank(user);
        usdc.approve(address(customGateway), inputAmount);

        vm.recordLogs();
        customGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Check that DustCollected event was NOT emitted
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool dustCollectedFound = false;

        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                dustCollectedFound = true;
            }
        }

        assertFalse(dustCollectedFound, "DustCollected event should NOT be emitted when protocolFeeBps is 0");

        // Verify the gateway received the full amount
        assertEq(usdc.balanceOf(address(customGateway)), inputAmount, "Gateway should have full input amount");
    }

    function testProtocolFeeOrderPlacedEventHasReducedAmounts() public {
        // Test that OrderPlaced event contains reduced amounts after protocol fee
        IntentGatewayV2 customGateway = _deployGatewayProxy();
        Params memory customParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000,
            protocolFeeBps: 500, // 5%
            priceOracle: address(0)
        });
        customGateway.initialize(
            InitParams({params: customParams, peerChains: new bytes[](0), relayer: address(0), owner: address(this)})
        );

        uint256 inputAmount = 1000 * 1e6; // 1000 USDC
        uint256 expectedProtocolFee = (inputAmount * 500) / 10000; // 50 USDC

        deal(address(usdc), user, inputAmount);

        Order memory order = Order({
            user: bytes32(0),
            source: bytes(""),
            destination: host.host(),
            deadline: block.timestamp + 1 hours,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: new TokenInfo[](1),
            output: PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: new TokenInfo[](1), call: ""})
        });

        order.inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});
        order.output.assets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});

        vm.startPrank(user);
        usdc.approve(address(customGateway), inputAmount);

        vm.recordLogs();
        customGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        Vm.Log[] memory entries = vm.getRecordedLogs();
        _assertPendingPlacementFee(customGateway, order, entries, expectedProtocolFee);
        for (uint256 i; i < entries.length; i++) {
            if (
                entries[i].emitter != address(customGateway)
                    || entries[i].topics[0]
                        != keccak256(
                            "OrderPlaced(bytes32,string,string,uint256,uint256,uint256,address,bytes32,(bytes32,uint256)[],(bytes32,uint256)[],(bytes32,uint256)[],bytes,bytes,bytes32)"
                        )
            ) continue;
            (,,,,,,,,, TokenInfo[] memory placedInputs,,,,) = abi.decode(
                entries[i].data,
                (
                    bytes32,
                    string,
                    string,
                    uint256,
                    uint256,
                    uint256,
                    address,
                    bytes32,
                    TokenInfo[],
                    TokenInfo[],
                    TokenInfo[],
                    bytes,
                    bytes,
                    bytes32
                )
            );
            assertEq(placedInputs[0].amount, inputAmount - expectedProtocolFee, "event contains net principal");
            return;
        }
        fail("OrderPlaced missing");
    }

    /*//////////////////////////////////////////////////////////////
                NATIVE TOKEN OVERPAYMENT REFUND TESTS
    //////////////////////////////////////////////////////////////*/

    /// @notice placeOrder with fee swap refunds unused ETH after swapETHForExactTokens.
    function testPlaceOrder_FeeSwap_RefundsExcessNativeToken() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 feeAmount = 1 * 1e18; // 1 DAI worth of fees

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: feeAmount,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        uint256 userEthBefore = user.balance;

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        // Send 5 ETH for a fee swap that should cost much less
        intentGateway.placeOrder{value: 5 ether}(order, bytes32(0));
        vm.stopPrank();

        // User should get back most of the 5 ETH — the swap only needed a tiny fraction
        uint256 ethSpent = userEthBefore - user.balance;
        assertTrue(ethSpent < 1 ether, "User should have been refunded most of the 5 ETH");
        assertTrue(ethSpent > 0, "User should have spent some ETH on the fee swap");
    }

    /// @notice Cross-chain fillOrder refunds solver's excess native ETH.
    function testFillCrossChain_RefundsSolverExcessNativeToken() public {
        uint256 outputAmount = 1 ether;
        uint256 overpayment = 0.5 ether;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 1000 * 1e6});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(0), amount: outputAmount}); // native ETH output

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        // Cross-chain order: source is remote, destination is current chain
        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: bytes("SOURCE_CHAIN"),
            destination: host.host(),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(0), amount: outputAmount});

        uint256 fillerEthBefore = filler.balance;

        vm.startPrank(filler);
        // Approve fee token for cross-chain dispatch
        dai.approve(address(intentGateway), type(uint256).max);
        intentGateway.fillOrder{value: outputAmount + overpayment}(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: solverOutputs,
                inputs: IntentQuoteTestUtils.inputs(order, solverOutputs)
            })
        );
        vm.stopPrank();

        // Solver should only have spent outputAmount
        assertEq(filler.balance, fillerEthBefore - outputAmount, "Solver overpayment should be refunded");
    }

    /*//////////////////////////////////////////////////////////////
                    PARAMS VALIDATION TESTS
    //////////////////////////////////////////////////////////////*/

    /// @notice setParams rejects zero host address.
    function testRevert_SetParams_ZeroHost() public {
        IntentGatewayV2 gw = _deployGatewayProxy();
        Params memory p = Params({
            host: address(0),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 5000,
            protocolFeeBps: 0,
            priceOracle: address(0)
        });
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        gw.initialize(InitParams({params: p, peerChains: new bytes[](0), relayer: address(0), owner: address(this)}));
    }

    /// @notice setParams rejects EOA dispatcher (no code).
    function testRevert_SetParams_EOADispatcher() public {
        IntentGatewayV2 gw = _deployGatewayProxy();
        Params memory p = Params({
            host: address(host),
            dispatcher: address(0xdead),
            solverSelection: false,
            surplusShareBps: 5000,
            protocolFeeBps: 0,
            priceOracle: address(0)
        });
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        gw.initialize(InitParams({params: p, peerChains: new bytes[](0), relayer: address(0), owner: address(this)}));
    }

    /// @notice setParams rejects surplusShareBps > 10000.
    function testRevert_SetParams_SurplusShareBpsTooHigh() public {
        IntentGatewayV2 gw = _deployGatewayProxy();
        Params memory p = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10001,
            protocolFeeBps: 0,
            priceOracle: address(0)
        });
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        gw.initialize(InitParams({params: p, peerChains: new bytes[](0), relayer: address(0), owner: address(this)}));
    }

    /// @notice setParams rejects protocolFeeBps >= 10000.
    function testRevert_SetParams_ProtocolFeeBpsTooHigh() public {
        IntentGatewayV2 gw = _deployGatewayProxy();
        Params memory p = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 5000,
            protocolFeeBps: 10000,
            priceOracle: address(0)
        });
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        gw.initialize(InitParams({params: p, peerChains: new bytes[](0), relayer: address(0), owner: address(this)}));
    }

    /// @notice setParams rejects non-contract priceOracle.
    function testRevert_SetParams_EOAPriceOracle() public {
        IntentGatewayV2 gw = _deployGatewayProxy();
        Params memory p = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 5000,
            protocolFeeBps: 0,
            priceOracle: address(0xbeef)
        });
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        gw.initialize(InitParams({params: p, peerChains: new bytes[](0), relayer: address(0), owner: address(this)}));
    }

    /// @notice updateParams via governance rejects destinationFeeBps >= 10000.
    function testRevert_UpdateParams_DestinationFeeBpsTooHigh() public {
        DestinationFee[] memory fees = new DestinationFee[](1);
        fees[0] = DestinationFee({destinationFeeBps: 10000, chain: bytes("ARBITRUM")});

        ParamsUpdate memory update = ParamsUpdate({
            params: Params({
                host: address(host),
                dispatcher: address(dispatcher),
                solverSelection: false,
                surplusShareBps: 5000,
                protocolFeeBps: 0,
                priceOracle: address(0)
            }),
            destinationFees: fees
        });

        bytes memory body = bytes.concat(bytes1(uint8(IntentsBase.RequestKind.UpdateParams)), abi.encode(update));

        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: body,
            timeoutTimestamp: 0
        });

        vm.prank(address(host));
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));
    }

    // ============================================================
    // UpgradeContract (cross-chain governance upgrade) Tests
    // ============================================================

    /// @dev Builds a same-chain order with the given input/output amounts and nonce.
    /// `user`, `source`, and `nonce` mirror what `placeOrder` will stamp, so the local
    /// `keccak256(abi.encode(order))` equals the on-chain commitment (protocol fee is 0).
    function _sameChainOrder(uint256 inputAmount, uint256 outputAmount, uint256 nonce)
        internal
        view
        returns (Order memory)
    {
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});
        DispatchInfo memory predispatch = DispatchInfo({assets: new TokenInfo[](0), call: ""});

        return Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: nonce,
            fees: 0,
            session: address(0),
            predispatch: predispatch,
            inputs: inputs,
            output: output
        });
    }

    /// @dev Seeds proxy state: order A is placed and fully filled (sets `_filled`); order B
    /// is placed only (leaves `_orders` escrowed). Returns the two commitments plus the
    /// escrowed token/amount so callers can assert these survive an implementation swap.
    function _seedUpgradeState()
        internal
        returns (bytes32 filledCommitment, bytes32 escrowedCommitment, uint256 escrowedAmount)
    {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 1000 * 1e18;
        escrowedAmount = inputAmount; // protocolFeeBps is 0 in setUp, so escrow == input.

        // Order A: place + full fill -> _filled[commitment] = filler.
        Order memory orderA = _sameChainOrder(inputAmount, outputAmount, 0);
        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(orderA, bytes32(0));
        vm.stopPrank();
        filledCommitment = keccak256(abi.encode(orderA));

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});
        vm.startPrank(filler);
        dai.approve(address(intentGateway), outputAmount);
        intentGateway.fillOrder(
            orderA,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: solverOutputs,
                inputs: IntentQuoteTestUtils.inputs(orderA, solverOutputs)
            })
        );
        vm.stopPrank();

        // Order B: place only -> _orders[commitment][0] = inputAmount.
        Order memory orderB = _sameChainOrder(inputAmount, outputAmount, 1);
        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(orderB, bytes32(0));
        vm.stopPrank();
        escrowedCommitment = keccak256(abi.encode(orderB));
    }

    /// @dev Builds an Execute onAccept request from `source` carrying `data` for the current
    /// implementation.
    function _executeRequest(bytes memory source, bytes memory data) internal view returns (PostRequest memory) {
        return PostRequest({
            source: source,
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: bytes.concat(bytes1(uint8(IntentsBase.RequestKind.Execute)), data),
            timeoutTimestamp: 0
        });
    }

    /// @dev A rotation is an Execute request calling `setRelayer(next)`.
    function _rotateRequest(address next) internal view returns (PostRequest memory) {
        return _executeRequest(host.hyperbridge(), abi.encodeCall(ExtrinsicIntents.setRelayer, (next)));
    }

    /// @dev An upgrade is an Execute request calling `upgradeToAndCall(newImpl, initData)`.
    function _upgradeRequest(bytes memory source, address newImpl, bytes memory initData)
        internal
        view
        returns (PostRequest memory)
    {
        return _executeRequest(source, abi.encodeCall(ExtrinsicIntents.upgradeToAndCall, (newImpl, initData)));
    }

    /// Governance rotates the relayer with a plain call: no implementation change, version unchanged.
    function testExecuteRotatesRelayerWithoutUpgrade() public {
        address implBefore = _implementationOf(address(intentGateway));
        address next = makeCleanAddr("nextRelayer");
        PostRequest memory request =
            _executeRequest(host.hyperbridge(), abi.encodeCall(ExtrinsicIntents.setRelayer, (next)));

        vm.expectEmit(true, true, true, true, address(intentGateway));
        emit IntentsBase.RelayerUpdated(relayer, next);
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        assertEq(intentGateway.relayer(), next);
        assertEq(intentGateway.version(), 3, "no migration ran");
        assertEq(_implementationOf(address(intentGateway)), implBefore, "implementation unchanged");
    }

    function testExecuteRejectsNonHyperbridgeSource() public {
        PostRequest memory request =
            _executeRequest(bytes("SOURCE_CHAIN"), abi.encodeCall(ExtrinsicIntents.setRelayer, (user)));
        vm.prank(address(host));
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));
        assertEq(intentGateway.relayer(), relayer, "relayer unchanged");
    }

    /// A revert inside the call surfaces unchanged, so the host records the message undelivered.
    /// Here `migrate` on a proxy already at `VERSION`, three delegatecalls deep.
    function testExecuteBubblesReverts() public {
        PostRequest memory request = _upgradeRequest(
            host.hyperbridge(), address(_upgradedImpl()), abi.encodeCall(IntentGatewayV2.migrate, (address(this)))
        );
        vm.prank(address(host));
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));
    }

    /// The `(address, bytes)` body of the previous implementation's `UpgradeContract` shares the
    /// discriminator; here it selects no function and reverts rather than doing anything.
    function testLegacyUpgradeBodyIsRefused() public {
        address implBefore = _implementationOf(address(intentGateway));
        IntentGatewayV2Upgraded newImpl = _upgradedImpl();
        PostRequest memory request = _executeRequest(host.hyperbridge(), abi.encode(address(newImpl), bytes("")));
        vm.prank(address(host));
        vm.expectRevert();
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));
        assertEq(_implementationOf(address(intentGateway)), implBefore, "implementation unchanged");
    }

    /// `upgradeToAndCall` lives on the extrinsic module, reached only by `Execute`.
    function testUpgradeToAndCallOnlyThroughExecute() public {
        IntentGatewayV2Upgraded newImpl = _upgradedImpl();
        address implBefore = _implementationOf(address(intentGateway));
        (bool ok,) =
            address(intentGateway).call(abi.encodeCall(ExtrinsicIntents.upgradeToAndCall, (address(newImpl), "")));
        assertFalse(ok, "not on the gateway");

        ExtrinsicIntents module = ExtrinsicIntents(intentGateway.extrinsicModule());
        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        module.upgradeToAndCall(address(newImpl), "");
        vm.prank(address(host));
        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        module.upgradeToAndCall(address(newImpl), "");
        assertEq(_implementationOf(address(intentGateway)), implBefore, "implementation unchanged");
    }

    /// @dev A new implementation with extra logic, reusing the live gateway's modules.
    function _upgradedImpl() internal returns (IntentGatewayV2Upgraded) {
        return new IntentGatewayV2Upgraded(intentGateway.intrinsicModule(), intentGateway.extrinsicModule());
    }

    function _implementationOf(address proxy) internal view returns (address) {
        return address(uint160(uint256(vm.load(proxy, ERC1967_IMPL_SLOT))));
    }

    function testOnAcceptUpgradeContractPreservesState() public {
        (bytes32 filledCommitment, bytes32 escrowedCommitment, uint256 escrowedAmount) = _seedUpgradeState();

        uint256 nonceBefore = intentGateway._nonce();
        assertEq(nonceBefore, 2, "precondition: two orders placed");
        assertEq(intentGateway._filled(filledCommitment), filler, "precondition: order A filled");
        assertEq(intentGateway._orders(escrowedCommitment, 0), escrowedAmount, "precondition: order B escrowed");

        IntentGatewayV2Upgraded newImpl = _upgradedImpl();
        PostRequest memory request = _upgradeRequest(host.hyperbridge(), address(newImpl), "");

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        // The proxy now points at the new implementation and its new logic is reachable.
        assertEq(_implementationOf(address(intentGateway)), address(newImpl), "implementation slot updated");
        assertEq(IntentGatewayV2Upgraded(payable(address(intentGateway))).upgradedMarker(), 42, "new logic is active");

        // All escrow-critical state survives the implementation swap.
        assertEq(intentGateway._nonce(), nonceBefore, "_nonce preserved");
        assertEq(intentGateway._filled(filledCommitment), filler, "_filled preserved");
        assertEq(intentGateway._orders(escrowedCommitment, 0), escrowedAmount, "_orders preserved");
    }

    /// @dev Production deploy path: the proxy initializes atomically via its init data, so the
    /// `initialize` call arrives through the proxy constructor. Must succeed.
    function testAtomicInitialization() public {
        IntentGatewayV2 implementation = deployIntentGatewayImpl();
        Params memory intentParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000,
            protocolFeeBps: 0,
            priceOracle: address(0)
        });
        bytes[] memory peers = new bytes[](1);
        peers[0] = bytes("SOURCE_CHAIN");

        bytes memory initData = abi.encodeCall(
            IntentGatewayV2.initialize,
            (InitParams({params: intentParams, peerChains: peers, relayer: relayer, owner: address(this)}))
        );
        ERC1967Proxy proxy = new ERC1967Proxy(address(implementation), initData);
        IntentGatewayV2 gateway = IntentGatewayV2(payable(address(proxy)));

        assertEq(gateway.params().host, address(host), "params set via atomic init");
        assertEq(gateway.instance(bytes("SOURCE_CHAIN")), address(gateway), "peer bound to address(this)");
        assertEq(gateway.relayer(), relayer, "relayer armed from the init data");
        assertEq(gateway.version(), 3, "at VERSION from the init data");

        vm.expectRevert();
        gateway.initialize(
            InitParams({params: intentParams, peerChains: peers, relayer: address(0), owner: address(this)})
        );
    }

    function testFilledMappingStaysAtSlotTwo() public {
        (bytes32 filledCommitment,,) = _seedUpgradeState();

        // _filled is `mapping(bytes32 => address)` declared at storage slot 2. The cross-chain
        // cancel proof (FILLED_SLOT_BIG_ENDIAN_BYTES) depends on this exact slot.
        bytes32 slot = keccak256(abi.encode(filledCommitment, uint256(2)));
        address filledFromSlot = address(uint160(uint256(vm.load(address(intentGateway), slot))));

        assertEq(filledFromSlot, filler, "_filled must occupy storage slot 2");
        assertEq(filledFromSlot, intentGateway._filled(filledCommitment), "slot-2 read matches getter");
    }

    function testOnAcceptUpgradeContractRejectsNonHyperbridgeSource() public {
        address implBefore = _implementationOf(address(intentGateway));
        IntentGatewayV2Upgraded newImpl = _upgradedImpl();

        // A registered peer gateway (not the Hyperbridge coprocessor) must not be able to upgrade.
        PostRequest memory request = _upgradeRequest(bytes("SOURCE_CHAIN"), address(newImpl), "");

        vm.prank(address(host));
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        assertEq(_implementationOf(address(intentGateway)), implBefore, "implementation must be unchanged");
    }

    function testOnAcceptUpgradeContractRejectsNoCodeImpl() public {
        address implBefore = _implementationOf(address(intentGateway));
        address noCode = makeCleanAddr("noCodeImpl"); // EOA, no contract code.

        PostRequest memory request = _upgradeRequest(host.hyperbridge(), noCode, "");

        vm.prank(address(host));
        vm.expectRevert(abi.encodeWithSelector(ERC1967Utils.ERC1967InvalidImplementation.selector, noCode));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        assertEq(_implementationOf(address(intentGateway)), implBefore, "implementation must be unchanged");
    }

    function testRawImplementationCannotBeInitialized() public {
        // The raw implementation behind the proxy is locked by `_disableInitializers()`.
        address impl = _implementationOf(address(intentGateway));
        Params memory p = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000,
            protocolFeeBps: 0,
            priceOracle: address(0)
        });
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        IntentGatewayV2(payable(impl))
            .initialize(InitParams({params: p, peerChains: new bytes[](0), relayer: address(0), owner: address(this)}));
    }

    function testProxyCannotBeReinitialized() public {
        // The proxy was already initialized in setUp; a second initialize must revert.
        Params memory p = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000,
            protocolFeeBps: 0,
            priceOracle: address(0)
        });
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        intentGateway.initialize(
            InitParams({params: p, peerChains: new bytes[](0), relayer: address(0), owner: address(this)})
        );
    }

    // ============================================================
    // Relayer allowlist Tests
    // ============================================================

    /// @dev Places a same-chain order so its input sits in escrow, and returns the RedeemEscrow
    /// request a peer gateway would send to release that escrow to `filler`.
    function _escrowedRedeemRequest()
        internal
        returns (PostRequest memory request, bytes32 commitment, uint256 amount)
    {
        amount = 1000 * 1e6;
        Order memory order = _sameChainOrder(amount, 1000 * 1e18, 0);
        vm.startPrank(user);
        usdc.approve(address(intentGateway), amount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();
        commitment = keccak256(abi.encode(order));

        bytes memory body = bytes.concat(
            bytes1(uint8(IntentsBase.RequestKind.RedeemEscrow)),
            abi.encode(
                WithdrawalRequest({
                    commitment: commitment, tokens: order.inputs, beneficiary: bytes32(uint256(uint160(filler)))
                })
            )
        );
        request = PostRequest({
            source: host.host(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: body,
            timeoutTimestamp: 0
        });
    }

    /// @dev Places a same-chain order and returns the GET response a source-chain cancel receives
    /// when the destination reports the order unfilled.
    function _cancelResponse() internal returns (GetResponse memory response, bytes32 commitment, uint256 amount) {
        amount = 1000 * 1e6;
        Order memory order = _sameChainOrder(amount, 1000 * 1e18, 0);
        vm.startPrank(user);
        usdc.approve(address(intentGateway), amount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();
        commitment = keccak256(abi.encode(order));

        uint256[] memory totalRequired = new uint256[](1);
        totalRequired[0] = order.output.assets[0].amount;
        bytes[] memory keys = new bytes[](1);
        keys[0] = abi.encodePacked(_partialFillSlot(commitment, 0));

        // Nothing filled on the destination: an empty proof value, so the whole escrow refunds.
        StorageValue[] memory values = new StorageValue[](1);
        values[0] = StorageValue({key: keys[0], value: new bytes(0)});
        GetRequest memory getRequest = GetRequest({
            source: host.host(),
            dest: order.destination,
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            keys: keys,
            height: 0,
            timeoutTimestamp: 0,
            context: abi.encode(commitment, order.user, order.inputs, totalRequired)
        });
        response = GetResponse({request: getRequest, values: values});
    }

    function _newDeploymentRequest(bytes memory chain, address gateway) internal view returns (PostRequest memory) {
        return PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: bytes.concat(
                bytes1(uint8(IntentsBase.RequestKind.NewDeployment)),
                abi.encode(Deployment({chain: chain, gateway: gateway}))
            ),
            timeoutTimestamp: 0
        });
    }

    /// @dev `_relayer` sits alone at slot 13 offset 0.
    function _relayerSlot(address r) internal pure returns (bytes32) {
        return bytes32(uint256(uint160(r)));
    }

    /// @dev Slot 13 as earlier implementations left it: an unset `bool _paused` at offset 0 and
    /// `_relayer` packed behind it at offset 1.
    function _legacyRelayerSlot(address r) internal pure returns (bytes32) {
        return bytes32(uint256(uint160(r)) << 8);
    }

    /// 0 on a bare proxy, 2 after `initialize`; the raw implementation is locked at the maximum.
    function testVersionTracksInitialization() public {
        IntentGatewayV2 bare = _deployGatewayProxy();
        assertEq(bare.version(), 0, "bare proxy");
        assertEq(intentGateway.version(), 3, "initialized");
        address impl = _implementationOf(address(intentGateway));
        assertEq(IntentGatewayV2(payable(impl)).version(), type(uint64).max, "raw implementation is locked");
    }

    function testRelayerSitsAtSlotThirteen() public view {
        assertEq(intentGateway.relayer(), relayer, "getter");
        assertEq(
            vm.load(address(intentGateway), bytes32(uint256(13))),
            _relayerSlot(relayer),
            "_relayer must sit alone at slot 13 offset 0"
        );
        assertEq(vm.load(address(intentGateway), bytes32(uint256(14))), bytes32(0), "slot 14 unused");
    }

    /// `setRelayer` lives on the extrinsic module, reached only by `Execute`: the gateway has no
    /// such function, and the module refuses everyone, the host included, when called directly.
    function testSetRelayerOnlyThroughExecute() public {
        (bool ok,) = address(intentGateway).call(abi.encodeCall(ExtrinsicIntents.setRelayer, (user)));
        assertFalse(ok, "not on the gateway");

        ExtrinsicIntents module = ExtrinsicIntents(intentGateway.extrinsicModule());
        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        module.setRelayer(user);
        vm.prank(user);
        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        module.setRelayer(user);
        vm.prank(address(host));
        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        module.setRelayer(user);

        assertEq(intentGateway.relayer(), relayer, "relayer unchanged");
    }

    function testSetRelayerRotates() public {
        address next = makeCleanAddr("nextRelayer");
        (PostRequest memory request,, uint256 amount) = _escrowedRedeemRequest();

        PostRequest memory rotate = _rotateRequest(next);
        vm.expectEmit(true, true, true, true, address(intentGateway));
        emit IntentsBase.RelayerUpdated(relayer, next);
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: rotate}));
        assertEq(intentGateway.relayer(), next);
        assertEq(intentGateway.version(), 3, "a rotation is not a migration");

        // The previous relayer is locked out immediately.
        vm.prank(address(host));
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        uint256 before = usdc.balanceOf(filler);
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: next, request: request}));
        assertEq(usdc.balanceOf(filler) - before, amount, "new relayer releases escrow");
    }

    /// `initialize` already took this proxy to `VERSION`, so `migrate` is refused.
    function testMigrateRunsOnce() public {
        vm.prank(address(host));
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        intentGateway.migrate(address(this));
        assertEq(intentGateway.version(), 3, "version unchanged");
    }

    /// A proxy an upgrade left at an earlier version cannot be re-initialized by anyone; only the
    /// host-only `migrate` takes it to `VERSION`.
    function testInitializeRefusedOnLegacyProxy() public {
        IntentGatewayV2 gateway = _legacyGateway();
        Params memory p = _openParams();

        vm.expectRevert(Initializable.InvalidInitialization.selector);
        gateway.initialize(InitParams({params: p, peerChains: new bytes[](0), relayer: user, owner: address(this)}));
        vm.prank(user);
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        gateway.initialize(InitParams({params: p, peerChains: new bytes[](0), relayer: user, owner: address(this)}));
        assertEq(gateway.version(), 2, "still at version 2");

        vm.prank(address(host));
        gateway.migrate(address(this));
        assertEq(gateway.version(), 3);
    }


    function testMigrateRejectsEveryoneButHost() public {
        IntentGatewayV2 gateway = _legacyGateway();

        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        gateway.migrate(address(this));

        vm.prank(user);
        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        gateway.migrate(address(this));

        assertEq(gateway.version(), 2, "still at version 2");
    }

    /// `migrate` moves the relayer from slot 13 offset 1 to offset 0, dropping the removed `_paused`
    /// byte ahead of it, and bumps the version. The relayer keeps its value, so the gate is unchanged.
    function testMigrateMovesTheRelayerToOffsetZero() public {
        IntentGatewayV2 gateway = _legacyGateway();
        // As an earlier implementation left it, with the old `_paused` byte set to show it is dropped.
        vm.store(address(gateway), bytes32(uint256(13)), bytes32(uint256(_legacyRelayerSlot(relayer)) | 1));

        vm.expectEmit(true, true, true, true, address(gateway));
        emit Initializable.Initialized(3);
        vm.prank(address(host));
        gateway.migrate(address(this));

        assertEq(gateway.relayer(), relayer, "relayer moved");
        assertEq(vm.load(address(gateway), bytes32(uint256(13))), _relayerSlot(relayer), "old _paused byte dropped");
        assertFalse(gateway.paused(), "the old byte does not pause");
        assertEq(gateway.version(), 3);
    }

    /// `initialize` arms the gate from the init data and lands at `VERSION`.
    function testInitializeArmsTheGate() public {
        IntentGatewayV2 gateway = _deployGatewayProxy();
        Params memory p = _openParams();
        vm.expectEmit(true, true, true, true, address(gateway));
        emit IntentsBase.RelayerUpdated(address(0), relayer);
        vm.expectEmit(true, true, true, true, address(gateway));
        emit Initializable.Initialized(3);
        gateway.initialize(InitParams({params: p, peerChains: new bytes[](0), relayer: relayer, owner: address(this)}));
        assertEq(gateway.relayer(), relayer);
        assertEq(gateway.version(), 3);
    }

    /// @dev OpenZeppelin's `Initializable` namespaced slot; `_initialized` is its low 8 bytes.
    bytes32 internal constant INITIALIZABLE_SLOT = 0xf0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00;

    /// @dev A proxy as an implementation from before this one left it: open gate, version 2.
    function _legacyGateway() internal returns (IntentGatewayV2 gateway) {
        gateway = _freshInitializedGateway();
        vm.store(address(gateway), INITIALIZABLE_SLOT, bytes32(uint256(2)));
        assertEq(gateway.version(), 2, "legacy proxy");
    }

    function _openParams() internal view returns (Params memory) {
        return Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000,
            protocolFeeBps: 0,
            priceOracle: address(0)
        });
    }

    /// @dev Through `initialize` with no relayer: open gate, at `VERSION`, no peers.
    function _freshInitializedGateway() internal returns (IntentGatewayV2 gateway) {
        gateway = _deployGatewayProxy();
        Params memory p = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000,
            protocolFeeBps: 0,
            priceOracle: address(0)
        });
        gateway.initialize(
            InitParams({params: p, peerChains: new bytes[](0), relayer: address(0), owner: address(this)})
        );
    }

    function testSetRelayerToZeroReopensTheGate() public {
        (PostRequest memory request,, uint256 amount) = _escrowedRedeemRequest();
        PostRequest memory reopen = _rotateRequest(address(0));
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: reopen}));
        assertEq(intentGateway.relayer(), address(0));
        assertEq(intentGateway.version(), 3, "reopening the gate is not a migration either");

        // With no relayer set the gate is open, so a delivery from anyone lands.
        uint256 before = usdc.balanceOf(filler);
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: filler, request: request}));
        assertEq(usdc.balanceOf(filler) - before, amount, "open gate releases escrow");
    }

    function testOnAcceptRejectsUnlistedRelayer() public {
        (PostRequest memory request, bytes32 commitment, uint256 amount) = _escrowedRedeemRequest();

        vm.prank(address(host));
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: filler, request: request}));
        assertEq(intentGateway._orders(commitment, 0), amount, "escrow untouched");
        assertEq(intentGateway._filled(commitment), address(0), "order not finalised");

        // The very same message goes through once the authorised relayer submits it.
        uint256 before = usdc.balanceOf(filler);
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));
        assertEq(usdc.balanceOf(filler) - before, amount, "authorised relayer releases escrow");
        assertEq(intentGateway._filled(commitment), filler, "order finalised");
    }

    function testOnAcceptGovernanceRejectsUnlistedRelayer() public {
        // UpgradeContract: a forged upgrade cannot land unless the relayer submits it.
        address implBefore = _implementationOf(address(intentGateway));
        IntentGatewayV2Upgraded newImpl = _upgradedImpl();
        PostRequest memory upgrade = _upgradeRequest(host.hyperbridge(), address(newImpl), "");

        vm.prank(address(host));
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: filler, request: upgrade}));
        assertEq(_implementationOf(address(intentGateway)), implBefore, "implementation unchanged");

        // NewDeployment: the gate runs before the body is decoded, so every kind is covered.
        PostRequest memory deployment = _newDeploymentRequest(bytes("NEW_CHAIN"), address(0xBEEF));
        vm.prank(address(host));
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: filler, request: deployment}));
        vm.expectRevert(IntentsBase.UnknownInstance.selector);
        intentGateway.instance(bytes("NEW_CHAIN"));

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: deployment}));
        assertEq(intentGateway.instance(bytes("NEW_CHAIN")), address(0xBEEF), "relayer-submitted governance applies");
    }

    function testOnGetResponseRejectsUnlistedRelayer() public {
        (GetResponse memory response, bytes32 commitment, uint256 amount) = _cancelResponse();

        vm.prank(address(host));
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        intentGateway.onGetResponse(IncomingGetResponse({response: response, relayer: user}));
        assertEq(intentGateway._orders(commitment, 0), amount, "escrow untouched");

        uint256 before = usdc.balanceOf(user);
        vm.prank(address(host));
        intentGateway.onGetResponse(IncomingGetResponse({response: response, relayer: relayer}));
        assertEq(usdc.balanceOf(user) - before, amount, "authorised relayer refunds escrow");
    }

    /// A fresh proxy has no relayer and accepts every delivery: `initialize` does not touch the
    /// gate, and its only setter is host-only, so the governance upgrade that arms it has to get
    /// through first. Once armed, only that relayer is accepted.
    function testFreshProxyIsOpenUntilGovernanceArmsIt() public {
        IntentGatewayV2 gateway = _freshInitializedGateway();
        assertEq(gateway.relayer(), address(0), "no relayer after initialize");

        PostRequest memory deployment = _newDeploymentRequest(bytes("NEW_CHAIN"), address(0xBEEF));
        deployment.from = abi.encodePacked(address(gateway));
        deployment.to = abi.encodePacked(address(gateway));

        // Open: an arbitrary relayer's delivery is applied.
        vm.prank(address(host));
        gateway.onAccept(IncomingPostRequest({relayer: filler, request: deployment}));
        assertEq(gateway.instance(bytes("NEW_CHAIN")), address(0xBEEF), "open gate applies governance");

        // Armed by a rotation: only `relayer` from now on, version unchanged.
        PostRequest memory arm = _rotateRequest(relayer);
        vm.prank(address(host));
        gateway.onAccept(IncomingPostRequest({relayer: filler, request: arm}));
        assertEq(gateway.version(), 3, "a rotation leaves the version alone");
        PostRequest memory another = _newDeploymentRequest(bytes("OTHER_CHAIN"), address(0xCAFE));
        another.from = abi.encodePacked(address(gateway));
        another.to = abi.encodePacked(address(gateway));

        vm.prank(address(host));
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        gateway.onAccept(IncomingPostRequest({relayer: filler, request: another}));
        vm.expectRevert(IntentsBase.UnknownInstance.selector);
        gateway.instance(bytes("OTHER_CHAIN"));

        vm.prank(address(host));
        gateway.onAccept(IncomingPostRequest({relayer: relayer, request: another}));
        assertEq(gateway.instance(bytes("OTHER_CHAIN")), address(0xCAFE));
    }

    /// A rotation cannot ride in upgrade init data any more: that data runs against the new
    /// implementation, and `setRelayer` lives on the extrinsic module. It is its own `Execute`,
    /// delivered by the relayer authorised at the time, after which only the new one is accepted.
    function testUpgradeThenRotateAreTwoExecutes() public {
        (bytes32 filledCommitment, bytes32 escrowedCommitment, uint256 escrowedAmount) = _seedUpgradeState();
        address next = makeCleanAddr("nextRelayer");
        IntentGatewayV2Upgraded newImpl = _upgradedImpl();
        PostRequest memory upgrade = _upgradeRequest(host.hyperbridge(), address(newImpl), "");
        PostRequest memory rotate = _rotateRequest(next);
        rotate.nonce = 1;

        // Both must arrive through the relayer authorised at the time.
        vm.prank(address(host));
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: next, request: upgrade}));
        vm.prank(address(host));
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: next, request: rotate}));

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: upgrade}));
        assertEq(_implementationOf(address(intentGateway)), address(newImpl), "implementation slot updated");
        assertEq(intentGateway.relayer(), relayer, "upgrade leaves the relayer alone");

        vm.expectEmit(true, true, true, true, address(intentGateway));
        emit IntentsBase.RelayerUpdated(relayer, next);
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: rotate}));
        assertEq(intentGateway.relayer(), next, "rotated");
        assertEq(intentGateway.version(), 3, "neither is a migration");
        assertEq(intentGateway._nonce(), 2, "_nonce preserved");
        assertEq(intentGateway._filled(filledCommitment), filler, "_filled preserved");
        assertEq(intentGateway._orders(escrowedCommitment, 0), escrowedAmount, "_orders preserved");

        // From here on only the new relayer is accepted.
        PostRequest memory deployment = _newDeploymentRequest(bytes("NEW_CHAIN"), address(0xBEEF));
        vm.prank(address(host));
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: deployment}));
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: next, request: deployment}));
        assertEq(intentGateway.instance(bytes("NEW_CHAIN")), address(0xBEEF));
    }

    function testUpgradeWithoutInitDataKeepsExistingRelayer() public {
        IntentGatewayV2Upgraded newImpl = _upgradedImpl();
        PostRequest memory request = _upgradeRequest(host.hyperbridge(), address(newImpl), "");
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));

        assertEq(_implementationOf(address(intentGateway)), address(newImpl));
        assertEq(intentGateway.relayer(), relayer, "relayer survives an implementation swap");
        assertEq(intentGateway.version(), 3, "no migration ran, so the version is unchanged");
    }

    /// @dev Through the real host: a delivery the gateway refuses is recorded as undelivered, so
    /// the authorised relayer can submit the same message afterwards.
    function testRejectedDeliveryStaysRetryableThroughHost() public {
        (PostRequest memory request, bytes32 commitment, uint256 amount) = _escrowedRedeemRequest();
        bytes32 requestCommitment = request.hash();

        vm.prank(address(handler));
        host.dispatchIncoming(request, filler);
        assertEq(host.requestReceipts(requestCommitment), address(0), "refused delivery leaves no receipt");
        assertEq(intentGateway._orders(commitment, 0), amount, "escrow untouched");

        uint256 before = usdc.balanceOf(filler);
        vm.prank(address(handler));
        host.dispatchIncoming(request, relayer);
        assertEq(host.requestReceipts(requestCommitment), relayer, "delivery recorded");
        assertEq(usdc.balanceOf(filler) - before, amount, "escrow released");
    }

    // ============================================================
    // Live mainnet proxy upgrade
    // ============================================================

    /// @dev The IntentGatewayV2 proxy deployed on Ethereum mainnet (identical address on every chain).
    address internal constant LIVE_GATEWAY = 0xAe041F7B0CB581876832830baeB6a2Aa2a3C9716;

    function _livePeers() internal pure returns (bytes[] memory peers) {
        uint256[9] memory ids = [uint256(1), 10, 42161, 8453, 56, 100, 137, 1868, 420420419];
        peers = new bytes[](ids.length);
        for (uint256 i; i < ids.length; i++) {
            peers[i] = StateMachine.evm(ids[i]);
        }
    }

    /// @dev The proxy that is actually live on mainnet, on the fork: armed and migrated, closed to
    /// re-initialisation, and governed only by its own relayer, including the next upgrade, which
    /// must keep every readable piece of state, and a rotation.
    function testLiveProxyIsArmedAndGovernedOnlyByItsRelayer() public {
        IntentGatewayV2 live = IntentGatewayV2(payable(LIVE_GATEWAY));
        assertGt(LIVE_GATEWAY.code.length, 0, "live gateway present on the fork");
        address liveRelayer = live.relayer();
        assertTrue(liveRelayer != address(0), "live proxy is armed");
        assertEq(live.version(), 2, "live proxy has been migrated");
        assertEq(
            vm.load(LIVE_GATEWAY, bytes32(uint256(13))),
            _legacyRelayerSlot(liveRelayer),
            "relayer packed behind an unset _paused in slot 13"
        );

        address implBefore = _implementationOf(LIVE_GATEWAY);
        uint256 nonce = live._nonce();
        Params memory p = live.params();
        bytes32 domain = live.DOMAIN_SEPARATOR();
        address liveHost = live.host();
        bytes[] memory peers = _livePeers();
        address[] memory instances = new address[](peers.length);
        uint256[] memory fees = new uint256[](peers.length);
        for (uint256 i; i < peers.length; i++) {
            instances[i] = live.instance(peers[i]);
            fees[i] = live._destinationProtocolFees(keccak256(peers[i]));
        }

        // Nobody can initialise it again, through the `initialize` its current implementation has.
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        ILiveGatewayInitialize(LIVE_GATEWAY).initialize(p, peers, filler);

        // The next upgrade is an Execute request carrying `migrate(owner)`. Anyone but the relayer is
        // refused before the body is read...
        IntentGatewayV2 newImpl = deployIntentGatewayImpl();
        PostRequest memory upgrade = PostRequest({
            source: IDispatcher(liveHost).hyperbridge(),
            dest: IDispatcher(liveHost).host(),
            nonce: 0,
            from: abi.encodePacked(LIVE_GATEWAY),
            to: abi.encodePacked(LIVE_GATEWAY),
            body: bytes.concat(
                bytes1(uint8(IntentsBase.RequestKind.Execute)),
                abi.encodeCall(
                    ExtrinsicIntents.upgradeToAndCall,
                    (address(newImpl), abi.encodeCall(IntentGatewayV2.migrate, (address(this))))
                )
            ),
            timeoutTimestamp: 0
        });
        vm.prank(liveHost);
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        live.onAccept(IncomingPostRequest({relayer: filler, request: upgrade}));
        assertEq(_implementationOf(LIVE_GATEWAY), implBefore, "refused upgrade leaves the implementation alone");

        // ...and the relayer's delivery installs it with every readable piece of state intact.
        vm.prank(liveHost);
        live.onAccept(IncomingPostRequest({relayer: liveRelayer, request: upgrade}));
        assertEq(_implementationOf(LIVE_GATEWAY), address(newImpl), "implementation slot updated");
        assertTrue(implBefore != address(newImpl), "implementation actually changed");
        assertEq(live.relayer(), liveRelayer, "relayer survives the upgrade");
        assertEq(vm.load(LIVE_GATEWAY, bytes32(uint256(13))), _relayerSlot(liveRelayer), "relayer moved to offset 0");
        assertEq(live.version(), 3, "migrated by the upgrade calldata");
        assertEq(live.owner(), address(this), "owner set by the upgrade calldata");
        assertFalse(live.paused(), "placement stays open");
        vm.prank(liveHost);
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        live.migrate(address(this));
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        live.initialize(InitParams({params: p, peerChains: peers, relayer: filler, owner: address(this)}));
        assertEq(live._nonce(), nonce, "_nonce preserved");
        Params memory q = live.params();
        assertEq(q.host, p.host, "params.host preserved");
        assertEq(q.dispatcher, p.dispatcher, "params.dispatcher preserved");
        assertEq(q.solverSelection, p.solverSelection, "params.solverSelection preserved");
        assertEq(q.surplusShareBps, p.surplusShareBps, "params.surplusShareBps preserved");
        assertEq(q.protocolFeeBps, p.protocolFeeBps, "params.protocolFeeBps preserved");
        assertEq(q.priceOracle, p.priceOracle, "params.priceOracle preserved");
        assertEq(live.DOMAIN_SEPARATOR(), domain, "EIP-712 domain preserved");
        assertEq(live.host(), liveHost, "host preserved");
        for (uint256 i; i < peers.length; i++) {
            assertEq(live.instance(peers[i]), instances[i], "peer instance preserved");
            assertEq(live._destinationProtocolFees(keccak256(peers[i])), fees[i], "destination fee preserved");
        }

        // A rotation is an Execute request from the current relayer; afterwards that relayer is out.
        address next = makeCleanAddr("liveNextRelayer");
        PostRequest memory rotate = upgrade;
        rotate.nonce = 1;
        rotate.body = bytes.concat(
            bytes1(uint8(IntentsBase.RequestKind.Execute)), abi.encodeCall(ExtrinsicIntents.setRelayer, (next))
        );
        vm.prank(liveHost);
        live.onAccept(IncomingPostRequest({relayer: liveRelayer, request: rotate}));
        assertEq(live.relayer(), next, "rotated through Execute");
        assertEq(live.version(), 3, "a rotation leaves the version alone");
        vm.prank(liveHost);
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        live.onAccept(IncomingPostRequest({relayer: liveRelayer, request: rotate}));
    }

    // ============================================
    // Cross-chain partial fill tests
    // ============================================

    /// @dev Minimal big-endian RLP encoding of a uint, matching what an Ethereum storage proof
    /// returns for a slot value (`RLP(slotValueTrimmed)`). Returns empty bytes for 0 (absent slot).
    function _rlpEncodeUint(uint256 x) internal pure returns (bytes memory) {
        if (x == 0) return bytes("");
        bytes32 be = bytes32(x);
        uint256 firstNonZero = 0;
        while (firstNonZero < 32 && be[firstNonZero] == 0) firstNonZero++;
        uint256 len = 32 - firstNonZero;
        bytes memory trimmed = new bytes(len);
        for (uint256 i; i < len; i++) {
            trimmed[i] = be[firstNonZero + i];
        }
        if (len == 1 && uint8(trimmed[0]) < 0x80) return trimmed;
        return abi.encodePacked(bytes1(uint8(0x80 + len)), trimmed);
    }

    /// @dev Recomputes the `_partialFills[commitment][index]` storage slot independently of the
    /// contract, to guard against storage-layout drift (slot 11).
    function _partialFillSlot(bytes32 commitment, uint256 index) internal pure returns (bytes32) {
        bytes32 inner = keccak256(abi.encodePacked(commitment, bytes32(uint256(11))));
        return keccak256(abi.encodePacked(index, inner));
    }

    /// @dev Builds a single-input/single-output cross-chain order (USDC -> DAI) with the given
    /// source/destination and amounts. `user`/`nonce` are left as the literals the caller expects.
    function _xchainOrder(bytes memory source, bytes memory destination, uint256 inputAmount, uint256 outputAmount)
        internal
        view
        returns (Order memory order)
    {
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});
        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});
        order = Order({
            user: bytes32(uint256(uint160(user))),
            source: source,
            destination: destination,
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""})
        });
    }

    /// @dev A withdrawal message (`RedeemEscrow`, `RedeemEscrowPartial` or `RefundEscrow`) as the destination
    /// gateway dispatches it to this source-chain gateway.
    function _withdrawalPost(
        IntentsBase.RequestKind kind,
        bytes32 commitment,
        TokenInfo[] memory tokens,
        address beneficiary
    ) internal view returns (PostRequest memory) {
        bytes memory body = bytes.concat(
            bytes1(uint8(kind)),
            abi.encode(
                WithdrawalRequest({
                    commitment: commitment, tokens: tokens, beneficiary: bytes32(uint256(uint160(beneficiary)))
                })
            )
        );
        return PostRequest({
            source: bytes("DEST_CHAIN"),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: body,
            timeoutTimestamp: 0
        });
    }

    /// @dev Replays a RedeemEscrow / RedeemEscrowPartial message arriving on the source chain.
    function _replayRedeem(IntentsBase.RequestKind kind, bytes32 commitment, TokenInfo[] memory tokens, address solver)
        internal
    {
        PostRequest memory request = _withdrawalPost(kind, commitment, tokens, solver);
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));
    }

    /// @dev The source-side cancel's GET response, proving `provenFilled` of the single output filled.
    function _cancelProof(bytes32 commitment, uint256 inputAmount, uint256 totalOutput, uint256 provenFilled)
        internal
        view
        returns (IncomingGetResponse memory)
    {
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});
        uint256[] memory totalRequired = new uint256[](1);
        totalRequired[0] = totalOutput;
        bytes memory context = abi.encode(commitment, bytes32(uint256(uint160(user))), inputs, totalRequired);

        // request.keys[i] is the _partialFills slot key for output i; the value carries the same key.
        bytes[] memory keys = new bytes[](1);
        keys[0] = abi.encodePacked(_partialFillSlot(commitment, 0));

        StorageValue[] memory values = new StorageValue[](1);
        values[0] = StorageValue({key: keys[0], value: _rlpEncodeUint(provenFilled)});

        GetRequest memory getRequest = GetRequest({
            source: host.host(),
            dest: bytes("DEST_CHAIN"),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            keys: keys,
            height: 0,
            timeoutTimestamp: 0,
            context: context
        });
        return IncomingGetResponse({response: GetResponse({request: getRequest, values: values}), relayer: relayer});
    }

    /// @dev Drives onGetResponse for a single-token partial-fill-aware cancel with the given proven
    /// fill amount on the destination.
    function _replayCancel(bytes32 commitment, uint256 inputAmount, uint256 totalOutput, uint256 provenFilled)
        internal
    {
        IncomingGetResponse memory incoming = _cancelProof(commitment, inputAmount, totalOutput, provenFilled);
        vm.prank(address(host));
        intentGateway.onGetResponse(incoming);
    }

    /// @dev Destination-side: a partial fill pays the beneficiary pro-rata, records cumulative
    /// progress, clears `_filled`, and dispatches a proportional escrow release (asserted via the
    /// PartialFill event's `inputs`). A second solver then completes the order.
    function testRate_CancelAndTwoRedemptionsConserveEscrowInAllDeliveryOrders() public {
        bytes memory source = host.host();
        Order memory order = _xchainOrder(source, bytes("DEST_CHAIN"), 1000, 1000);
        vm.startPrank(user);
        usdc.approve(address(intentGateway), 1000);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();
        bytes32 commitment = keccak256(abi.encode(order));
        // Exercise destination settlement against the same layout, then deliver its
        // earned slices and proof to the source in every order. Transport is mocked.
        vm.mockCall(address(host), abi.encodeWithSignature("host()"), abi.encode(bytes("DEST_CHAIN")));
        TokenInfo[] memory takes = new TokenInfo[](1);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        takes[0] = TokenInfo(order.inputs[0].token, 200);
        outputs[0] = TokenInfo(order.output.assets[0].token, 220);
        vm.startPrank(filler);
        dai.approve(address(intentGateway), 550);
        intentGateway.fillOrder(order, FillOptions(0, 0, 0, outputs, takes));
        takes[0].amount = 300;
        outputs[0].amount = 330;
        intentGateway.fillOrder(order, FillOptions(0, 0, 0, outputs, takes));
        vm.stopPrank();
        vm.clearMockedCalls();
        assertEq(intentGateway._partialFills(commitment, 0), 500);
        uint8[3][6] memory permutations = [
            [uint8(0), 1, 2], [uint8(0), 2, 1], [uint8(1), 0, 2], [uint8(1), 2, 0], [uint8(2), 0, 1], [uint8(2), 1, 0]
        ];
        for (uint256 i; i < 6; ++i) {
            uint256 snapshot = vm.snapshotState();
            uint256 beforeUser = usdc.balanceOf(user);
            uint256 beforeSolver = usdc.balanceOf(filler);
            for (uint256 j; j < 3; ++j) {
                uint8 action = permutations[i][j];
                if (action == 2) {
                    _replayCancel(commitment, 1000, 1000, 500);
                } else {
                    takes[0].amount = action == 0 ? 200 : 300;
                    _replayRedeem(IntentsBase.RequestKind.RedeemEscrowPartial, commitment, takes, filler);
                }
            }
            assertEq(usdc.balanceOf(user) - beforeUser, 500);
            assertEq(usdc.balanceOf(filler) - beforeSolver, 500);
            assertEq(intentGateway._orders(commitment, 0), 0);
            assertTrue(vm.revertToStateAndDelete(snapshot));
        }
    }

    function testRate_CrossChainSurplusCapsAndCreditsOnlyOrderOutput() public {
        Order memory order = _xchainOrder(bytes("SOURCE_CHAIN"), host.host(), 1000, 1000);
        bytes32 commitment = keccak256(abi.encode(order));
        TokenInfo[] memory takes = new TokenInfo[](1);
        takes[0] = TokenInfo(order.inputs[0].token, 800);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo(order.output.assets[0].token, 880);
        TokenInfo[] memory credited = new TokenInfo[](1);
        credited[0] = TokenInfo(order.output.assets[0].token, 800);
        vm.startPrank(filler);
        dai.approve(address(intentGateway), type(uint256).max);
        uint256 before = dai.balanceOf(filler);
        uint256 userBefore = dai.balanceOf(user);
        vm.expectEmit(true, false, false, true);
        emit IntentsBase.PartialFill(commitment, filler, credited, takes);
        intentGateway.fillOrder(order, FillOptions(0, 0, 0, outputs, takes));
        assertEq(intentGateway._partialFills(commitment, 0), 800);
        assertEq(dai.balanceOf(user) - userBefore, 800);
        takes[0].amount = 500;
        outputs[0].amount = 550;
        credited[0].amount = 200;
        TokenInfo[] memory released = new TokenInfo[](1);
        released[0] = TokenInfo(order.inputs[0].token, 200);
        vm.expectEmit(true, false, false, true);
        emit IntentsBase.OrderFilled(commitment, filler, credited, released);
        intentGateway.fillOrder(order, FillOptions(0, 0, 0, outputs, takes));
        vm.stopPrank();
        assertEq(before - dai.balanceOf(filler), 1100);
        assertEq(dai.balanceOf(user) - userBefore, 1000);
        assertEq(intentGateway._partialFills(commitment, 0), 1000);
    }

    function testCrossChainPartialFill_ReleasesProportionalEscrowAndCompletes() public {
        uint256 inputAmount = 1000 * 1e6; // 1000 USDC escrowed on the source chain
        uint256 outputAmount = 1000 * 1e18; // 1000 DAI requested on this (destination) chain
        Order memory order = _xchainOrder(bytes("SOURCE_CHAIN"), host.host(), inputAmount, outputAmount);
        bytes32 commitment = keccak256(abi.encode(order));
        bytes32 daiToken = bytes32(uint256(uint160(address(dai))));

        // Solver A fills 40%.
        TokenInfo[] memory outA = new TokenInfo[](1);
        outA[0] = TokenInfo({token: daiToken, amount: 400 * 1e18});
        TokenInfo[] memory expOutA = new TokenInfo[](1);
        expOutA[0] = TokenInfo({token: daiToken, amount: 400 * 1e18});
        TokenInfo[] memory expInA = new TokenInfo[](1);
        expInA[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 400 * 1e6});

        uint256 userDaiBefore = dai.balanceOf(user);
        vm.startPrank(filler);
        dai.approve(address(intentGateway), type(uint256).max);
        vm.expectEmit(true, false, false, true);
        emit IntentsBase.PartialFill(commitment, filler, expOutA, expInA);
        intentGateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: outA,
                inputs: IntentQuoteTestUtils.inputs(order, outA)
            })
        );
        vm.stopPrank();

        assertEq(dai.balanceOf(user) - userDaiBefore, 400 * 1e18, "beneficiary gets 40% output");
        assertEq(intentGateway._partialFills(commitment, 0), 400 * 1e18, "cumulative fill recorded");
        assertEq(intentGateway._filled(commitment), address(0), "filled cleared so next solver can continue");

        // Solver B completes the remaining 60%.
        address solverB = makeAddr("solverB");
        deal(address(dai), solverB, 10000 * 1e18);
        TokenInfo[] memory outB = new TokenInfo[](1);
        outB[0] = TokenInfo({token: daiToken, amount: 600 * 1e18});
        TokenInfo[] memory expOutB = new TokenInfo[](1);
        expOutB[0] = TokenInfo({token: daiToken, amount: 600 * 1e18});
        TokenInfo[] memory expInB = new TokenInfo[](1);
        expInB[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 600 * 1e6});

        vm.startPrank(solverB);
        dai.approve(address(intentGateway), type(uint256).max);
        vm.expectEmit(true, false, false, true);
        emit IntentsBase.OrderFilled(commitment, solverB, expOutB, expInB);
        intentGateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: outB,
                inputs: IntentQuoteTestUtils.inputs(order, outB)
            })
        );
        vm.stopPrank();

        assertEq(dai.balanceOf(user) - userDaiBefore, 1000 * 1e18, "beneficiary fully paid");
        assertEq(intentGateway._partialFills(commitment, 0), 1000 * 1e18, "order fully filled");
        assertEq(intentGateway._filled(commitment), solverB, "completing solver recorded");
    }

    /// @dev Source-side: a RedeemEscrowPartial releases only its slice without finalizing (no
    /// `_filled`, fees retained); the completing RedeemEscrow finalizes and forwards the fee pot.
    function testCrossChainPartialRedeem_DoesNotFinalizeUntilComplete() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 feeAmount = 5 * 1e18; // fees are paid in the fee token (DAI)
        Order memory order = _xchainOrder(host.host(), bytes("DEST_CHAIN"), inputAmount, 1000 * 1e18);
        order.fees = feeAmount;

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        dai.approve(address(intentGateway), feeAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        bytes32 commitment = keccak256(abi.encode(order));
        address solver = makeAddr("xchainSolver");

        // Partial redeem of 40% of the escrow. The source emits EscrowReleased for the slice even
        // though it does not finalize, so partial releases are observable.
        TokenInfo[] memory slice = new TokenInfo[](1);
        slice[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 400 * 1e6});
        vm.expectEmit(true, false, false, true);
        emit IntentsBase.EscrowReleased(commitment, solver, slice);
        _replayRedeem(IntentsBase.RequestKind.RedeemEscrowPartial, commitment, slice, solver);

        assertEq(usdc.balanceOf(solver), 400 * 1e6, "solver received partial slice");
        assertEq(intentGateway._orders(commitment, 0), 600 * 1e6, "escrow reduced, not drained");
        assertEq(intentGateway._filled(commitment), address(0), "partial redeem does not finalize");
        assertEq(dai.balanceOf(solver), 0, "fees not forwarded on partial redeem");

        // Completing redeem of the remaining 60% finalizes and forwards the fee pot.
        TokenInfo[] memory rest = new TokenInfo[](1);
        rest[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 600 * 1e6});
        _replayRedeem(IntentsBase.RequestKind.RedeemEscrow, commitment, rest, solver);

        assertEq(usdc.balanceOf(solver), 1000 * 1e6, "solver received full escrow");
        assertEq(intentGateway._orders(commitment, 0), 0, "escrow drained");
        assertEq(intentGateway._filled(commitment), solver, "completing redeem finalizes");
        assertEq(dai.balanceOf(solver), feeAmount, "completing solver takes the fee pot");
    }

    /// @dev Cancel after a partial fill refunds only the proven-unredeemed fraction, never the raw
    /// remaining escrow, so an already-redeemed slice plus the refund sum to the full escrow.
    function testCrossChainCancel_RefundsUnfilledFractionOnly() public {
        uint256 inputAmount = 1000 * 1e6;
        Order memory order = _xchainOrder(host.host(), bytes("DEST_CHAIN"), inputAmount, 1000 * 1e18);

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        bytes32 commitment = keccak256(abi.encode(order));
        address solver = makeAddr("xchainSolver");

        // A 40% partial fill was already redeemed on the source chain.
        TokenInfo[] memory slice = new TokenInfo[](1);
        slice[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 400 * 1e6});
        _replayRedeem(IntentsBase.RequestKind.RedeemEscrowPartial, commitment, slice, solver);

        // User cancels with a proof showing 40% of the output filled on the destination.
        uint256 userUsdcBefore = usdc.balanceOf(user);
        _replayCancel(commitment, inputAmount, 1000 * 1e18, 400 * 1e18);

        assertEq(usdc.balanceOf(user) - userUsdcBefore, 600 * 1e6, "user refunded only the unfilled 60%");
        assertEq(intentGateway._orders(commitment, 0), 0, "escrow fully accounted for");
        assertEq(intentGateway._filled(commitment), user, "cancel finalizes for idempotency");
        assertEq(usdc.balanceOf(solver), 400 * 1e6, "solver keeps its redeemed slice");
    }

    /// @dev Cancel can reach onGetResponse even for an order fully filled on the destination, when
    /// the completing RedeemEscrow is still in flight to the source (source _filled is still 0). The
    /// proof then shows full fill: refund is 0 and the fee pot must be withheld for the completing solver.
    function testCrossChainCancel_FullyFilledRaceRefundsNothingAndWithholdsFees() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 feeAmount = 5 * 1e18;
        Order memory order = _xchainOrder(host.host(), bytes("DEST_CHAIN"), inputAmount, 1000 * 1e18);
        order.fees = feeAmount;

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        dai.approve(address(intentGateway), feeAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        bytes32 commitment = keccak256(abi.encode(order));

        // Fully filled on the destination, completing RedeemEscrow still in flight. User cancels;
        // the proof shows 100% filled.
        uint256 userUsdcBefore = usdc.balanceOf(user);
        uint256 userDaiBefore = dai.balanceOf(user);
        _replayCancel(commitment, inputAmount, 1000 * 1e18, 1000 * 1e18);

        assertEq(usdc.balanceOf(user) - userUsdcBefore, 0, "nothing refunded for a fully-filled order");
        assertEq(dai.balanceOf(user) - userDaiBefore, 0, "fees withheld for the completing solver");
        assertEq(intentGateway._orders(commitment, 0), inputAmount, "escrow intact for in-flight redeem");
        uint256 txFeeKey = uint160(uint256(keccak256("txFees")));
        assertEq(intentGateway._orders(commitment, txFeeKey), feeAmount, "fee pot intact");
        assertEq(intentGateway._filled(commitment), user, "cancel still records idempotency");

        // The in-flight completing redeem lands: the solver receives the full escrow and the fee pot.
        address solver = makeAddr("xchainSolver");
        TokenInfo[] memory full = new TokenInfo[](1);
        full[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});
        _replayRedeem(IntentsBase.RequestKind.RedeemEscrow, commitment, full, solver);

        assertEq(usdc.balanceOf(solver), inputAmount, "completing solver redeems full escrow");
        assertEq(dai.balanceOf(solver), feeAmount, "completing solver receives the fee pot");
    }

    /// @dev A RedeemEscrowPartial that arrives AFTER the user has cancelled still succeeds: the
    /// cancel refunded only the unfilled fraction, leaving exactly enough escrow for the in-flight slice.
    function testCrossChainCancel_InFlightRedeemAfterCancelStaysConsistent() public {
        uint256 inputAmount = 1000 * 1e6;
        Order memory order = _xchainOrder(host.host(), bytes("DEST_CHAIN"), inputAmount, 1000 * 1e18);

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        bytes32 commitment = keccak256(abi.encode(order));
        address solver = makeAddr("xchainSolver");

        // User cancels first; proof shows 40% filled before the deadline, redeem still in flight.
        uint256 userUsdcBefore = usdc.balanceOf(user);
        _replayCancel(commitment, inputAmount, 1000 * 1e18, 400 * 1e18);
        assertEq(usdc.balanceOf(user) - userUsdcBefore, 600 * 1e6, "user refunded unfilled 60%");
        assertEq(intentGateway._orders(commitment, 0), 400 * 1e6, "escrow reserved for in-flight redeem");

        // The in-flight 40% redeem now lands and is fully covered.
        TokenInfo[] memory slice = new TokenInfo[](1);
        slice[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 400 * 1e6});
        _replayRedeem(IntentsBase.RequestKind.RedeemEscrowPartial, commitment, slice, solver);

        assertEq(usdc.balanceOf(solver), 400 * 1e6, "in-flight redeem paid in full");
        assertEq(intentGateway._orders(commitment, 0), 0, "escrow fully settled");
    }

    /// @dev A second cancel response for the same commitment is rejected (idempotency).
    function testCrossChainCancel_DoubleCancelBlocked() public {
        uint256 inputAmount = 1000 * 1e6;
        Order memory order = _xchainOrder(host.host(), bytes("DEST_CHAIN"), inputAmount, 1000 * 1e18);

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        bytes32 commitment = keccak256(abi.encode(order));
        _replayCancel(commitment, inputAmount, 1000 * 1e18, 0);
        assertEq(intentGateway._filled(commitment), user, "first cancel finalized");

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});
        uint256[] memory totalRequired = new uint256[](1);
        totalRequired[0] = 1000 * 1e18;
        bytes memory context = abi.encode(commitment, bytes32(uint256(uint160(user))), inputs, totalRequired);
        bytes[] memory keys = new bytes[](1);
        keys[0] = abi.encodePacked(_partialFillSlot(commitment, 0));
        StorageValue[] memory values = new StorageValue[](1);
        values[0] = StorageValue({key: keys[0], value: _rlpEncodeUint(0)});
        GetRequest memory getRequest = GetRequest({
            source: host.host(),
            dest: bytes("DEST_CHAIN"),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            keys: keys,
            height: 0,
            timeoutTimestamp: 0,
            context: context
        });
        IncomingGetResponse memory incoming =
            IncomingGetResponse({response: GetResponse({request: getRequest, values: values}), relayer: relayer});
        vm.prank(address(host));
        vm.expectRevert(IntentsBase.Filled.selector);
        intentGateway.onGetResponse(incoming);
    }

    /// @dev Places a 1000 USDC -> 1000 DAI cross-chain order. `half` is the 500 USDC slice that, at a 50% fill,
    /// both a cancel refunds and the in-flight redeem claims.
    function _placeCrossChainOrder() internal returns (bytes32 commitment, TokenInfo[] memory half) {
        uint256 inputAmount = 1000 * 1e6;
        Order memory order = _xchainOrder(host.host(), bytes("DEST_CHAIN"), inputAmount, 1000 * 1e18);

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        commitment = keccak256(abi.encode(order));
        half = new TokenInfo[](1);
        half[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 500 * 1e6});
    }

    /// Cancelling from both chains sends two refunds for the same unfilled slice, since each chain only sees its
    /// own `_filled`. Here the GET cancel lands first: it refunds the unfilled half and reserves the other half for the solver's
    /// in-flight redeem. The destination's RefundEscrow for the same half must then be rejected, or it drains
    /// the reserve and the solver, who already delivered half the output, is never paid.
    function testCrossChainCancel_RefundEscrowAfterGetCancelRejected() public {
        (bytes32 commitment, TokenInfo[] memory half) = _placeCrossChainOrder();
        address solver = makeAddr("xchainSolver");

        uint256 userUsdcBefore = usdc.balanceOf(user);
        _replayCancel(commitment, 1000 * 1e6, 1000 * 1e18, 500 * 1e18);
        assertEq(usdc.balanceOf(user) - userUsdcBefore, 500 * 1e6, "user refunded the unfilled half");
        assertEq(intentGateway._orders(commitment, 0), 500 * 1e6, "half reserved for the redeem");

        PostRequest memory refund = _withdrawalPost(IntentsBase.RequestKind.RefundEscrow, commitment, half, user);
        vm.prank(address(host));
        vm.expectRevert(IntentsBase.Filled.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: refund}));

        assertEq(usdc.balanceOf(user) - userUsdcBefore, 500 * 1e6, "no second refund");
        assertEq(intentGateway._orders(commitment, 0), 500 * 1e6, "reserve intact");

        // The guard is scoped to RefundEscrow: the delayed redeem still consumes the reserve.
        _replayRedeem(IntentsBase.RequestKind.RedeemEscrowPartial, commitment, half, solver);
        assertEq(usdc.balanceOf(solver), 500 * 1e6, "solver paid for its half");
        assertEq(intentGateway._orders(commitment, 0), 0, "escrow fully settled");
    }

    /// The reverse delivery order: RefundEscrow lands first and finalizes, so the GET cancel is rejected. A
    /// delayed redeem after a RefundEscrow cancel still consumes the reserve it left.
    function testCrossChainCancel_GetCancelAfterRefundEscrowRejected() public {
        (bytes32 commitment, TokenInfo[] memory half) = _placeCrossChainOrder();
        address solver = makeAddr("xchainSolver");

        uint256 userUsdcBefore = usdc.balanceOf(user);
        PostRequest memory refund = _withdrawalPost(IntentsBase.RequestKind.RefundEscrow, commitment, half, user);
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: refund}));
        assertEq(usdc.balanceOf(user) - userUsdcBefore, 500 * 1e6, "user refunded the unfilled half");
        assertEq(intentGateway._filled(commitment), user, "refund finalized the order");

        IncomingGetResponse memory cancel = _cancelProof(commitment, 1000 * 1e6, 1000 * 1e18, 500 * 1e18);
        vm.prank(address(host));
        vm.expectRevert(IntentsBase.Filled.selector);
        intentGateway.onGetResponse(cancel);

        assertEq(usdc.balanceOf(user) - userUsdcBefore, 500 * 1e6, "no second refund");
        assertEq(intentGateway._orders(commitment, 0), 500 * 1e6, "reserve intact");

        _replayRedeem(IntentsBase.RequestKind.RedeemEscrowPartial, commitment, half, solver);
        assertEq(usdc.balanceOf(solver), 500 * 1e6, "solver paid for its half");
        assertEq(intentGateway._orders(commitment, 0), 0, "escrow fully settled");
    }

    /// @dev Cross-chain orders carrying output calldata cannot be partially filled.
    function testCrossChainPartialFill_CalldataRevertsPartialFillNotAllowed() public {
        Order memory order = _xchainOrder(bytes("SOURCE_CHAIN"), host.host(), 1000 * 1e6, 1000 * 1e18);
        // Attach a (harmless) output call so the order requires single-fill completion.
        Call[] memory calls = new Call[](1);
        calls[0] = Call({to: address(dai), value: 0, data: abi.encodeWithSelector(IERC20.balanceOf.selector, user)});
        order.output.call = abi.encode(calls);
        bytes32 daiToken = bytes32(uint256(uint160(address(dai))));

        TokenInfo[] memory partialOut = new TokenInfo[](1);
        partialOut[0] = TokenInfo({token: daiToken, amount: 400 * 1e18});

        vm.startPrank(filler);
        dai.approve(address(intentGateway), type(uint256).max);
        TokenInfo[] memory takes = new TokenInfo[](1);
        takes[0] = TokenInfo(order.inputs[0].token, 400 * 1e6);
        vm.expectRevert(IntentsBase.PartialFillNotAllowed.selector);
        intentGateway.fillOrder(
            order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, validUntil: 0, outputs: partialOut, inputs: takes})
        );
        vm.stopPrank();
        bytes32 commitment = keccak256(abi.encode(order));
        assertEq(intentGateway._partialFills(commitment, 0), 0);
        assertEq(intentGateway._filled(commitment), address(0));
    }

    /// @dev External helper so the test can use calldata slicing to strip the RequestKind prefix.
    function decodeWithdrawalBody(bytes calldata body) external pure returns (uint8 kind, WithdrawalRequest memory wr) {
        kind = uint8(body[0]);
        wr = abi.decode(body[1:], (WithdrawalRequest));
    }

    /// @dev Destination-side cancel after a partial fill must refund only the unredeemed fraction
    /// of escrow (read from local `_partialFills`), not the full `order.inputs`.
    function testCrossChainCancelFromDest_RefundsUnredeemedFractionOnly() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 1000 * 1e18;
        Order memory order = _xchainOrder(bytes("SOURCE_CHAIN"), host.host(), inputAmount, outputAmount);
        bytes32 commitment = keccak256(abi.encode(order));
        bytes32 daiToken = bytes32(uint256(uint160(address(dai))));

        // Solver fills 40% on this (destination) chain.
        TokenInfo[] memory outA = new TokenInfo[](1);
        outA[0] = TokenInfo({token: daiToken, amount: 400 * 1e18});
        vm.startPrank(filler);
        dai.approve(address(intentGateway), type(uint256).max);
        intentGateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: outA,
                inputs: IntentQuoteTestUtils.inputs(order, outA)
            })
        );
        vm.stopPrank();
        assertEq(intentGateway._partialFills(commitment, 0), 400 * 1e18, "40% recorded");

        // User cancels from the destination; capture the dispatched RefundEscrow.
        vm.recordLogs();
        vm.prank(user);
        intentGateway.cancelOrder(order, CancelOptions({relayerFee: 0, height: 0}));

        Vm.Log[] memory logs = vm.getRecordedLogs();
        bytes32 postTopic = keccak256("PostRequestEvent(string,string,address,bytes,uint256,uint256,bytes,uint256)");
        bytes memory body;
        for (uint256 i = 0; i < logs.length; i++) {
            if (logs[i].topics[0] == postTopic) {
                (,,,,, bytes memory b,) =
                    abi.decode(logs[i].data, (string, string, bytes, uint256, uint256, bytes, uint256));
                body = b;
            }
        }
        assertGt(body.length, 0, "RefundEscrow dispatched");

        (uint8 kind, WithdrawalRequest memory wr) = this.decodeWithdrawalBody(body);
        assertEq(kind, uint8(IntentsBase.RequestKind.RefundEscrow), "refund escrow kind");
        assertEq(wr.tokens.length, 1, "one input");
        // 40% of output filled -> 40% of USDC escrow redeemed -> refund the unredeemed 60% = 600 USDC.
        assertEq(wr.tokens[0].amount, 600 * 1e6, "refunds only the unredeemed 60%");
        assertEq(wr.beneficiary, bytes32(uint256(uint160(user))), "refund to user");
        assertEq(intentGateway._filled(commitment), user, "order frozen on destination");
    }

    /// @dev placeOrder rejects a zero-amount output, which would otherwise strand its paired escrow.
    function testPlaceOrder_RevertsOnZeroAmountOutput() public {
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 1000 * 1e6});
        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 0});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: bytes("DEST_CHAIN"),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""})
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), type(uint256).max);
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();
    }

    /// @dev Guards the `_partialFills` storage slot (11) used to build cross-chain cancel proofs.
    function testPartialFillsStorageSlotIsEleven() public {
        Order memory order = _xchainOrder(bytes("SOURCE_CHAIN"), host.host(), 1000 * 1e6, 1000 * 1e18);
        bytes32 commitment = keccak256(abi.encode(order));
        bytes32 daiToken = bytes32(uint256(uint160(address(dai))));

        TokenInfo[] memory outA = new TokenInfo[](1);
        outA[0] = TokenInfo({token: daiToken, amount: 250 * 1e18});
        vm.startPrank(filler);
        dai.approve(address(intentGateway), type(uint256).max);
        intentGateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: outA,
                inputs: IntentQuoteTestUtils.inputs(order, outA)
            })
        );
        vm.stopPrank();

        uint256 viaGetter = intentGateway._partialFills(commitment, 0);
        bytes32 raw = vm.load(address(intentGateway), _partialFillSlot(commitment, 0));
        assertEq(viaGetter, 250 * 1e18, "partial fill recorded via getter");
        assertEq(uint256(raw), viaGetter, "slot-11 derivation matches public getter");
    }

    /// @notice Inputs and outputs pair 1:1 into legs: both arrays non-empty, equal length, every output
    /// amount non-zero. Legs may repeat tokens, and each leg is escrowed under its own index.
    function testPlaceOrder_LegShape() public {
        bytes32 usdcToken = bytes32(uint256(uint160(address(usdc))));
        bytes32 daiToken = bytes32(uint256(uint160(address(dai))));
        TokenInfo[] memory none = new TokenInfo[](0);
        TokenInfo[] memory oneIn = new TokenInfo[](1);
        oneIn[0] = TokenInfo({token: usdcToken, amount: 1000 * 1e6});
        TokenInfo[] memory twoIn = new TokenInfo[](2);
        twoIn[0] = TokenInfo({token: usdcToken, amount: 1000 * 1e6});
        twoIn[1] = TokenInfo({token: usdcToken, amount: 500 * 1e6});
        TokenInfo[] memory oneOut = new TokenInfo[](1);
        oneOut[0] = TokenInfo({token: daiToken, amount: 1000 * 1e18});
        TokenInfo[] memory twoOut = new TokenInfo[](2);
        twoOut[0] = TokenInfo({token: daiToken, amount: 1000 * 1e18});
        twoOut[1] = TokenInfo({token: daiToken, amount: 490 * 1e18});
        TokenInfo[] memory zeroSecondOut = new TokenInfo[](2);
        zeroSecondOut[0] = twoOut[0];
        zeroSecondOut[1] = TokenInfo({token: daiToken, amount: 0});

        vm.startPrank(user);
        usdc.approve(address(intentGateway), type(uint256).max);
        dai.approve(address(intentGateway), type(uint256).max);

        vm.expectRevert(IntentsBase.InvalidInput.selector);
        intentGateway.placeOrder(_orderWithLegs(none, none), bytes32(0));
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        intentGateway.placeOrder(_orderWithLegs(none, oneOut), bytes32(0));
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        intentGateway.placeOrder(_orderWithLegs(twoIn, oneOut), bytes32(0));
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        intentGateway.placeOrder(_orderWithLegs(oneIn, twoOut), bytes32(0));
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        intentGateway.placeOrder(_orderWithLegs(twoIn, zeroSecondOut), bytes32(0));

        // One pair at two prices: both legs repeat USDC -> DAI.
        Order memory order = _orderWithLegs(twoIn, twoOut);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        order.source = host.host();
        bytes32 commitment = keccak256(abi.encode(order));
        assertEq(intentGateway._orders(commitment, 0), 1000 * 1e6, "leg 0 escrowed on its own");
        assertEq(intentGateway._orders(commitment, 1), 500 * 1e6, "leg 1 escrowed on its own");
        assertEq(usdc.balanceOf(address(intentGateway)), 1500 * 1e6, "both legs escrowed");
    }

    /// @dev A cross-chain order with the given legs. Builds without external calls, so it can sit between
    /// `vm.expectRevert` and the call it targets; `placeOrder` stamps `source` itself.
    function _orderWithLegs(TokenInfo[] memory inputs, TokenInfo[] memory outputs)
        internal
        view
        returns (Order memory)
    {
        return Order({
            user: bytes32(uint256(uint160(user))),
            source: "",
            destination: bytes("DEST_CHAIN"),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputs, call: ""})
        });
    }

    // ============================================
    // Owner and placement pause
    // ============================================

    /// @dev OpenZeppelin `OwnableUpgradeable`'s ERC-7201 slot, derived independently of the library.
    function _ownershipSlot() internal pure returns (bytes32) {
        return keccak256(abi.encode(uint256(keccak256("openzeppelin.storage.Ownable")) - 1)) & ~bytes32(uint256(0xff));
    }

    function _pausableSlot() internal pure returns (bytes32) {
        return keccak256(abi.encode(uint256(keccak256("openzeppelin.storage.Pausable")) - 1)) & ~bytes32(uint256(0xff));
    }

    function _notOwner(address account) internal pure returns (bytes memory) {
        return abi.encodeWithSelector(OwnableUpgradeable.OwnableUnauthorizedAccount.selector, account);
    }

    function testInitializeSetsTheOwnerAtItsNamespacedSlot() public view {
        assertEq(intentGateway.owner(), address(this), "owner from the init data");
        assertEq(
            address(uint160(uint256(vm.load(address(intentGateway), _ownershipSlot())))),
            address(this),
            "owner at the namespaced slot"
        );
        assertEq(intentGateway.pendingOwner(), address(0), "nothing pending");
        assertFalse(intentGateway.paused(), "placement open");
    }

    function testInitializeRejectsAZeroOwner() public {
        IntentGatewayV2 gateway = _deployGatewayProxy();
        vm.expectRevert(abi.encodeWithSelector(OwnableUpgradeable.OwnableInvalidOwner.selector, address(0)));
        gateway.initialize(
            InitParams({params: _openParams(), peerChains: new bytes[](0), relayer: address(0), owner: address(0)})
        );
    }

    /// `migrate` sets the owner of a proxy coming from an earlier implementation.
    function testMigrateSetsTheOwner() public {
        IntentGatewayV2 gateway = _legacyGateway();
        address next = makeCleanAddr("migratedOwner");

        vm.prank(address(host));
        vm.expectRevert(abi.encodeWithSelector(OwnableUpgradeable.OwnableInvalidOwner.selector, address(0)));
        gateway.migrate(address(0));

        vm.expectEmit(true, true, true, true, address(gateway));
        emit OwnableUpgradeable.OwnershipTransferred(address(this), next);
        vm.prank(address(host));
        gateway.migrate(next);

        assertEq(gateway.owner(), next, "owner from the migration");
        assertEq(gateway.version(), 3);
    }

    function testOwnershipTransferIsTwoStep() public {
        address next = makeCleanAddr("nextOwner");

        vm.prank(user);
        vm.expectRevert(_notOwner(user));
        intentGateway.transferOwnership(user);

        vm.expectEmit(true, true, true, true, address(intentGateway));
        emit Ownable2StepUpgradeable.OwnershipTransferStarted(address(this), next);
        intentGateway.transferOwnership(next);
        assertEq(intentGateway.owner(), address(this), "unchanged until accepted");
        assertEq(intentGateway.pendingOwner(), next, "proposal pending");

        vm.prank(user);
        vm.expectRevert(_notOwner(user));
        intentGateway.acceptOwnership();

        vm.expectEmit(true, true, true, true, address(intentGateway));
        emit OwnableUpgradeable.OwnershipTransferred(address(this), next);
        vm.prank(next);
        intentGateway.acceptOwnership();
        assertEq(intentGateway.owner(), next, "accepted");
        assertEq(intentGateway.pendingOwner(), address(0), "proposal cleared");

        // The previous owner keeps no power.
        vm.expectRevert(_notOwner(address(this)));
        intentGateway.pause();
    }

    function testOwnershipProposalCanBeWithdrawn() public {
        address next = makeCleanAddr("nextOwner");
        intentGateway.transferOwnership(next);
        intentGateway.transferOwnership(address(0));

        vm.prank(next);
        vm.expectRevert(_notOwner(next));
        intentGateway.acceptOwnership();
        assertEq(intentGateway.owner(), address(this), "owner unchanged");
    }

    /// The host counts as the owner, so governance can pause, resume, and recover from a renounced owner.
    function testHostCountsAsOwner() public {
        vm.prank(address(host));
        intentGateway.pause();
        assertTrue(intentGateway.paused(), "host paused");
        vm.prank(address(host));
        intentGateway.unpause();
        assertFalse(intentGateway.paused(), "host resumed");

        intentGateway.renounceOwnership();
        assertEq(intentGateway.owner(), address(0), "renounced");

        address next = makeCleanAddr("recoveredOwner");
        vm.prank(address(host));
        intentGateway.transferOwnership(next);
        vm.prank(next);
        intentGateway.acceptOwnership();
        assertEq(intentGateway.owner(), next, "recovered through the host");
    }

    /// Governance replaces the owner without the owner key: an `Execute` carrying
    /// `upgradeToAndCall(currentImplementation, transferOwnership(next))`, the host still the caller.
    function testGovernanceReplacesTheOwnerThroughExecute() public {
        address next = makeCleanAddr("governanceOwner");
        address impl = _implementationOf(address(intentGateway));
        PostRequest memory request = _upgradeRequest(
            host.hyperbridge(), impl, abi.encodeCall(Ownable2StepUpgradeable.transferOwnership, (next))
        );

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: request}));
        assertEq(intentGateway.pendingOwner(), next, "proposed by the host");
        assertEq(_implementationOf(address(intentGateway)), impl, "implementation unchanged");

        vm.prank(next);
        intentGateway.acceptOwnership();
        assertEq(intentGateway.owner(), next, "replaced");
    }

    /// Pausing stops `placeOrder`, `fillOrder`, escrow deliveries and cancel proofs. Governance
    /// deliveries still land and `cancelOrder` stays open; what was refused goes through once resumed.
    function testPauseStopsPlacementFillsAndEscrowDeliveries() public {
        Order memory toRedeem = _sameChainOrder(1000 * 1e6, 1000 * 1e18, 0);
        Order memory toFill = _sameChainOrder(1000 * 1e6, 1000 * 1e18, 1);
        Order memory toCancel = _sameChainOrder(1000 * 1e6, 1000 * 1e18, 2);
        Order memory later = _sameChainOrder(1000 * 1e6, 1000 * 1e18, 3);
        vm.startPrank(user);
        usdc.approve(address(intentGateway), type(uint256).max);
        intentGateway.placeOrder(toRedeem, bytes32(0));
        intentGateway.placeOrder(toFill, bytes32(0));
        intentGateway.placeOrder(toCancel, bytes32(0));
        vm.stopPrank();
        vm.prank(filler);
        dai.approve(address(intentGateway), type(uint256).max);

        PostRequest memory redeem = PostRequest({
            source: host.host(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: bytes.concat(
                bytes1(uint8(IntentsBase.RequestKind.RedeemEscrow)),
                abi.encode(
                    WithdrawalRequest({
                        commitment: keccak256(abi.encode(toRedeem)),
                        tokens: toRedeem.inputs,
                        beneficiary: bytes32(uint256(uint160(filler)))
                    })
                )
            ),
            timeoutTimestamp: 0
        });
        FillOptions memory fill = FillOptions({
            relayerFee: 0,
            nativeDispatchFee: 0,
            validUntil: 0,
            outputs: toFill.output.assets,
            inputs: IntentQuoteTestUtils.inputs(toFill, toFill.output.assets)
        });
        address next = makeCleanAddr("pausedRotation");
        PostRequest memory rotate = _rotateRequest(next);

        vm.prank(user);
        vm.expectRevert(_notOwner(user));
        intentGateway.pause();

        vm.expectEmit(true, true, true, true, address(intentGateway));
        emit PausableUpgradeable.Paused(address(this));
        intentGateway.pause();
        assertTrue(intentGateway.paused(), "paused");
        assertEq(uint256(vm.load(address(intentGateway), _pausableSlot())), 1, "flag at the namespaced slot");
        assertEq(
            vm.load(address(intentGateway), bytes32(uint256(13))), _relayerSlot(relayer), "slot 13 untouched by pausing"
        );
        vm.expectRevert(PausableUpgradeable.EnforcedPause.selector);
        intentGateway.pause();

        vm.prank(user);
        vm.expectRevert(PausableUpgradeable.EnforcedPause.selector);
        intentGateway.placeOrder(later, bytes32(0));

        vm.prank(filler);
        vm.expectRevert(PausableUpgradeable.EnforcedPause.selector);
        intentGateway.fillOrder(toFill, fill);

        vm.prank(address(host));
        vm.expectRevert(PausableUpgradeable.EnforcedPause.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: redeem}));

        IncomingGetResponse memory response; // refused before it is read
        vm.prank(address(host));
        vm.expectRevert(PausableUpgradeable.EnforcedPause.selector);
        intentGateway.onGetResponse(response);

        // Governance still lands while paused.
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: relayer, request: rotate}));
        assertEq(intentGateway.relayer(), next, "governance delivery accepted while paused");

        // Users can still start a refund.
        uint256 before = usdc.balanceOf(user);
        vm.prank(user);
        intentGateway.cancelOrder(toCancel, CancelOptions({relayerFee: 0, height: 0}));
        assertEq(usdc.balanceOf(user) - before, 1000 * 1e6, "cancellations continue while paused");

        vm.prank(user);
        vm.expectRevert(_notOwner(user));
        intentGateway.unpause();

        vm.expectEmit(true, true, true, true, address(intentGateway));
        emit PausableUpgradeable.Unpaused(address(this));
        intentGateway.unpause();
        assertFalse(intentGateway.paused(), "resumed");
        vm.expectRevert(PausableUpgradeable.ExpectedPause.selector);
        intentGateway.unpause();

        // The refused delivery and fill go through once resumed.
        before = usdc.balanceOf(filler);
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: next, request: redeem}));
        assertEq(usdc.balanceOf(filler) - before, 1000 * 1e6, "redeem delivered after resuming");

        vm.prank(filler);
        intentGateway.fillOrder(toFill, fill);
        assertEq(intentGateway._filled(keccak256(abi.encode(toFill))), filler, "fill after resuming");

        vm.prank(user);
        intentGateway.placeOrder(later, bytes32(0));
    }
}

contract IntentGatewayV2Upgraded is IntentGatewayV2 {
    constructor(address intrinsic, address extrinsic) IntentGatewayV2(intrinsic, extrinsic) {}

    function upgradedMarker() external pure returns (uint256) {
        return 42;
    }
}
