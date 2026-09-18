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
pragma solidity ^0.8.24;
import {IntentQuoteTestUtils} from "./IntentQuoteTestUtils.sol";

import "forge-std/Test.sol";
import {intentGatewayUpgradeInitialization} from "../../script/IntentGatewayScript.sol";
import {MainnetForkBaseTest} from "./MainnetForkBaseTest.sol";
import {deployIntentGatewayImpl, deployIntentModules} from "./IntentGatewayDeploy.sol";
import {
    IntentGatewayV2,
    Order,
    Params,
    InitParams,
    TokenInfo,
    PaymentInfo,
    DispatchInfo,
    FillOptions,
    CancelOptions
} from "../../src/apps/IntentGatewayV2.sol";
import {HyperApp} from "@hyperbridge/core/apps/HyperApp.sol";
import {IntentsBase} from "../../src/apps/intentsv2/IntentsBase.sol";
import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import {ExtrinsicIntents} from "../../src/apps/intentsv2/ExtrinsicIntents.sol";
import {IntrinsicModule} from "../../src/apps/intentsv2/IntrinsicModule.sol";
import {ExtrinsicModule} from "../../src/apps/intentsv2/ExtrinsicModule.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {Initializable} from "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";
import {PostRequest} from "@hyperbridge/core/interfaces/IDispatcher.sol";

/// @dev A replacement same-chain module whose fills fail with a marker error.
contract SwappedIntrinsicModule is IntentsBase {
    error ModuleSwapped();

    constructor() EIP712("IntentGateway", "2") {}

    function fillOrder(Order calldata, FillOptions calldata, bytes32) external payable {
        revert ModuleSwapped();
    }
}

/// @dev A replacement cross-chain module whose fills fail with a marker error.
contract SwappedExtrinsicModule is IntentsBase {
    error ExtrinsicSwapped();

    constructor() EIP712("IntentGateway", "2") {}

    function fillOrder(Order calldata, FillOptions calldata, bytes32) external payable {
        revert ExtrinsicSwapped();
    }
}

/// @notice The delegatecall split: shared storage layout, modules closed to direct calls, code
/// checks at construction, verbatim revert bubbling, and module upgrades. Fill and cancel behaviour
/// is covered by the pre-existing suites.
contract IntentGatewayModulesTest is MainnetForkBaseTest {
    IntentGatewayV2 internal gateway;
    address internal user;
    address internal solver;

    function setUp() public override {
        super.setUp();
        user = makeCleanAddr("user");
        solver = makeCleanAddr("solver");

        IntentGatewayV2 implementation = deployIntentGatewayImpl();
        gateway = IntentGatewayV2(payable(address(new ERC1967Proxy(address(implementation), ""))));
        bytes[] memory peers = new bytes[](1);
        peers[0] = bytes("DEST_CHAIN");
        // Relayer gate left open so governance requests in these tests need no relayer.
        gateway.initialize(
            InitParams({params: _params(), peerChains: peers, relayer: address(0), owner: address(this)})
        );

        deal(address(usdc), user, 10_000 * 1e6);
        deal(address(dai), solver, 10_000 * 1e18);
    }

    function _params() internal view returns (Params memory) {
        return Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10_000,
            protocolFeeBps: 0,
            priceOracle: address(0)
        });
    }

    /*//////////////////////////////////////////////////////////////
                              STORAGE LAYOUT
    //////////////////////////////////////////////////////////////*/

    /// @dev One `storageLayout.storage` entry of a forge artifact, fields in `vm.parseJson` order.
    struct StorageEntry {
        uint256 astId;
        string contract_;
        string label;
        uint256 offset;
        string slot;
        string type_;
    }

    function _storageLayout(string memory artifact) internal view returns (StorageEntry[] memory) {
        string memory json = vm.readFile(artifact);
        require(vm.keyExistsJson(json, ".storageLayout"), "build with extra_output = [\"storageLayout\"]");
        return abi.decode(vm.parseJson(json, ".storageLayout.storage"), (StorageEntry[]));
    }

    /// @dev Drops the AST id from type ids like `t_struct(Params)4459_storage` before comparing.
    function _canonicalType(string memory typeId) internal pure returns (string memory) {
        bytes memory b = bytes(typeId);
        bytes memory out = new bytes(b.length);
        uint256 n;
        bool afterParen;
        for (uint256 i; i < b.length; i++) {
            bytes1 c = b[i];
            if (afterParen && c >= 0x30 && c <= 0x39) continue;
            afterParen = c == 0x29;
            out[n++] = c;
        }
        assembly ("memory-safe") {
            mstore(out, n)
        }
        return string(out);
    }

    /// The implementation and both modules compile to the same storage layout, slot for slot.
    function testStorageLayoutIsSharedByImplementationAndModules() public view {
        StorageEntry[] memory impl = _storageLayout("out/IntentGatewayV2.sol/IntentGatewayV2.json");
        assertGt(impl.length, 0, "implementation layout is non-empty");
        assertEq(impl[2].label, "_filled", "_filled is third after the EIP-712 fallbacks");
        assertEq(impl[2].slot, "2", "_filled at slot 2, the cancel proof key depends on it");

        string[2] memory modules =
            ["out/IntrinsicModule.sol/IntrinsicModule.json", "out/ExtrinsicModule.sol/ExtrinsicModule.json"];
        for (uint256 m; m < modules.length; m++) {
            StorageEntry[] memory layout = _storageLayout(modules[m]);
            assertEq(layout.length, impl.length, string.concat(modules[m], ": variable count"));
            for (uint256 i; i < impl.length; i++) {
                string memory where = string.concat(modules[m], ": ", impl[i].label);
                assertEq(layout[i].label, impl[i].label, where);
                assertEq(layout[i].slot, impl[i].slot, string.concat(where, " slot"));
                assertEq(layout[i].offset, impl[i].offset, string.concat(where, " offset"));
                assertEq(_canonicalType(layout[i].type_), _canonicalType(impl[i].type_), string.concat(where, " type"));
            }
        }
    }

    /// Existing proof keys and every earlier field remain in place. The unused `_paused` byte is gone, so
    /// `_relayer` sits alone at slot 13 offset 0; `migrate` moves it there on live proxies.
    function testFeeAccountingAppendsAfterEveryExistingStorageField() public view {
        StorageEntry[] memory layout = _storageLayout("out/IntentGatewayV2.sol/IntentGatewayV2.json");
        string[11] memory labels = [
            "_nameFallback",
            "_versionFallback",
            "_filled",
            "_nonce",
            "_params",
            "_orders",
            "_instances",
            "_partialFills",
            "_destinationProtocolFees",
            "_relayer",
            "_protocolFees"
        ];
        string[11] memory slots = ["0", "1", "2", "3", "4", "9", "10", "11", "12", "13", "14"];
        assertEq(layout.length, labels.length);
        for (uint256 i; i < labels.length; i++) {
            assertEq(layout[i].label, labels[i]);
            assertEq(layout[i].slot, slots[i], labels[i]);
            assertEq(layout[i].offset, 0, labels[i]);
        }
    }

    function testImplementationAndModulesFitEIP170() public view {
        assertLe(gateway.intrinsicModule().code.length, 24_576, "intrinsic module");
        assertLe(gateway.extrinsicModule().code.length, 24_576, "extrinsic module");
        assertLe(_implementationOf(address(gateway)).code.length, 24_576, "implementation");
    }

    /*//////////////////////////////////////////////////////////////
                             DIRECT CALLS
    //////////////////////////////////////////////////////////////*/

    /// Every module function refuses a call at the module's own address.
    function testModulesRefuseDirectCalls() public {
        IntrinsicModule intrinsic = IntrinsicModule(gateway.intrinsicModule());
        ExtrinsicModule extrinsic = ExtrinsicModule(gateway.extrinsicModule());
        Order memory order = _sameChainOrder(1e6, 1e18);
        FillOptions memory fill = FillOptions({
            relayerFee: 0,
            nativeDispatchFee: 0,
            validUntil: 0,
            outputs: order.output.assets,
            inputs: IntentQuoteTestUtils.inputs(order, order.output.assets)
        });
        CancelOptions memory cancel = CancelOptions({relayerFee: 0, height: 0});
        bytes32 commitment = keccak256(abi.encode(order));

        vm.expectRevert(IntentsBase.Unauthorized.selector);
        intrinsic.fillOrder(order, fill, commitment);
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        intrinsic.cancelSameChain(order, commitment);

        vm.expectRevert(IntentsBase.Unauthorized.selector);
        extrinsic.fillOrder(order, fill, commitment);
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        extrinsic.cancelFromSource(order, cancel, commitment);
        vm.expectRevert(IntentsBase.Unauthorized.selector);
        extrinsic.cancelFromDest(order, cancel, commitment);
        // The host callbacks keep their `onlyHost` guard, which a direct call also fails.
        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        extrinsic.onAccept(
            IncomingPostRequest({
                relayer: address(this),
                request: PostRequest({
                    source: "", dest: "", nonce: 0, from: "", to: "", body: hex"00", timeoutTimestamp: 0
                })
            })
        );
    }

    /*//////////////////////////////////////////////////////////////
                              CONSTRUCTION
    //////////////////////////////////////////////////////////////*/

    function testConstructorRecordsModules() public {
        (address intrinsic, address extrinsic) = deployIntentModules();
        IntentGatewayV2 implementation = new IntentGatewayV2(intrinsic, extrinsic);
        assertEq(implementation.intrinsicModule(), intrinsic);
        assertEq(implementation.extrinsicModule(), extrinsic);
    }

    /// A module address without code is refused at construction.
    function testConstructorRejectsModuleWithoutCode() public {
        (address intrinsic, address extrinsic) = deployIntentModules();
        address noCode = makeCleanAddr("noCode");

        vm.expectRevert(IntentsBase.InvalidInput.selector);
        new IntentGatewayV2(noCode, extrinsic);
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        new IntentGatewayV2(intrinsic, noCode);
    }

    /*//////////////////////////////////////////////////////////////
                            REVERT BUBBLING
    //////////////////////////////////////////////////////////////*/

    /// A custom error raised inside a module reaches the caller as exactly its four bytes.
    function testModuleCustomErrorBubblesVerbatim() public {
        Order memory order = _placeSameChainOrder(1000 * 1e6, 900 * 1e18);
        vm.prank(solver);
        vm.expectRevert(abi.encodeWithSelector(IntentsBase.Unauthorized.selector));
        gateway.cancelOrder(order, CancelOptions({relayerFee: 0, height: 0}));
    }

    /// A revert with a payload, here mainnet DAI's string reason, bubbles intact.
    function testModuleRevertWithDataBubblesVerbatim() public {
        Order memory order = _placeSameChainOrder(1000 * 1e6, 900 * 1e18);
        // No DAI approval from the solver.
        vm.prank(solver);
        vm.expectRevert(bytes("Dai/insufficient-allowance"));
        gateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: order.output.assets,
                inputs: IntentQuoteTestUtils.inputs(order, order.output.assets)
            })
        );
    }

    /*//////////////////////////////////////////////////////////////
                             MODULE UPGRADE
    //////////////////////////////////////////////////////////////*/

    /// A module upgrade is an implementation upgrade with new immutables, delivered through `Execute`.
    function testModuleUpgradeIsAnImplementationUpgrade() public {
        Order memory order = _placeSameChainOrder(1000 * 1e6, 900 * 1e18);

        address swapped = address(new SwappedIntrinsicModule());
        IntentGatewayV2 newImpl = new IntentGatewayV2(swapped, gateway.extrinsicModule());
        _upgradeThroughExecute(address(newImpl), "");

        assertEq(gateway.intrinsicModule(), swapped, "new implementation carries the new module");
        assertEq(gateway._nonce(), 1, "state survives");
        assertEq(gateway._orders(keccak256(abi.encode(order)), 0), 1000 * 1e6, "escrow survives");

        vm.startPrank(solver);
        dai.approve(address(gateway), 900 * 1e18);
        vm.expectRevert(SwappedIntrinsicModule.ModuleSwapped.selector);
        gateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: order.output.assets,
                inputs: IntentQuoteTestUtils.inputs(order, order.output.assets)
            })
        );
        vm.stopPrank();
    }

    /// The extrinsic module is swapped the same way. A cross-chain fill on the destination side
    /// then runs the new module's code.
    function testExtrinsicModuleUpgradeIsAnImplementationUpgrade() public {
        address swapped = address(new SwappedExtrinsicModule());
        IntentGatewayV2 newImpl = new IntentGatewayV2(gateway.intrinsicModule(), swapped);
        _upgradeThroughExecute(address(newImpl), "");
        assertEq(gateway.extrinsicModule(), swapped, "new implementation carries the new module");

        Order memory order = _sameChainOrder(1000 * 1e6, 900 * 1e18);
        order.source = bytes("SOURCE_CHAIN"); // destination stays this chain: a cross-chain fill
        vm.prank(solver);
        vm.expectRevert(SwappedExtrinsicModule.ExtrinsicSwapped.selector);
        gateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: order.output.assets,
                inputs: IntentQuoteTestUtils.inputs(order, order.output.assets)
            })
        );
    }

    /// @dev OpenZeppelin's `Initializable` namespaced slot; `_initialized` is its low 8 bytes.
    bytes32 internal constant INITIALIZABLE_SLOT = 0xf0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00;

    /// The release's own upgrade: a proxy at version 2, as every live one is, moved to this
    /// implementation with `migrate(owner)` as the init data lands at 4 with its state intact and
    /// its owner set. Pinned here because the live-fork test's precondition expires once mainnet
    /// is upgraded.
    function testUpgradeFromVersionTwoWithMigrate() public {
        Order memory order = _placeSameChainOrder(1000 * 1e6, 900 * 1e18);
        vm.store(address(gateway), INITIALIZABLE_SLOT, bytes32(uint256(2)));
        assertEq(gateway.version(), 2, "a proxy on the previous implementation");

        IntentGatewayV2 newImpl = deployIntentGatewayImpl();
        PostRequest memory upgrade =
            _upgradeRequest(address(newImpl), abi.encodeCall(IntentGatewayV2.migrate, (address(this))));
        vm.expectEmit(true, true, true, true, address(gateway));
        emit Initializable.Initialized(3);
        vm.prank(address(host));
        gateway.onAccept(IncomingPostRequest({relayer: address(this), request: upgrade}));

        assertEq(gateway.version(), 3, "migrated");
        assertEq(gateway.owner(), address(this), "owner set by the migration");
        assertEq(gateway._orders(keccak256(abi.encode(order)), 0), 1000 * 1e6, "escrow survives");
        assertEq(gateway.instance(bytes("DEST_CHAIN")), address(gateway), "peers survive");

        // `migrate` is one-shot: a second upgrade carrying it is refused.
        PostRequest memory again = _upgradeRequest(
            address(deployIntentGatewayImpl()), abi.encodeCall(IntentGatewayV2.migrate, (address(this)))
        );
        vm.prank(address(host));
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        gateway.onAccept(IncomingPostRequest({relayer: address(this), request: again}));
    }

    function testUpgradeHelperUsesEmptyInitializationForVersionFour() public view {
        assertEq(intentGatewayUpgradeInitialization(gateway, address(this)), bytes(""));
    }

    function testUpgradeHelperMigratesVersionTwo() public {
        vm.store(address(gateway), INITIALIZABLE_SLOT, bytes32(uint256(2)));
        assertEq(
            intentGatewayUpgradeInitialization(gateway, address(this)),
            abi.encodeCall(IntentGatewayV2.migrate, (address(this)))
        );
    }

    function testUpgradeHelperRequiresAnOwnerToMigrate() public {
        vm.store(address(gateway), INITIALIZABLE_SLOT, bytes32(uint256(2)));
        vm.expectRevert("GATEWAY_OWNER is unset");
        this.upgradeInitialization(address(gateway), address(0));
    }

    function testUpgradeHelperRejectsUnsupportedVersions() public {
        uint256[3] memory unsupported = [uint256(0), uint256(1), uint256(5)];
        for (uint256 i; i < unsupported.length; i++) {
            vm.store(address(gateway), INITIALIZABLE_SLOT, bytes32(unsupported[i]));
            vm.expectRevert("Unsupported IntentGateway version");
            this.upgradeInitialization(address(gateway), address(this));
        }
    }

    function upgradeInitialization(address target, address owner) external view returns (bytes memory) {
        return intentGatewayUpgradeInitialization(IntentGatewayV2(payable(target)), owner);
    }

    /// Upgrading is still possible after the split, and repeatedly. The second upgrade is the one
    /// that proves the point: it runs through the new path, where `Execute` delegatecalls the
    /// extrinsic module and the module's `upgradeToAndCall` writes the proxy's ERC-1967 slot.
    function testUpgradesChainAfterTheSplit() public {
        Order memory order = _placeSameChainOrder(1000 * 1e6, 900 * 1e18);
        bytes32 commitment = keccak256(abi.encode(order));

        for (uint256 i; i < 3; i++) {
            IntentGatewayV2 next = deployIntentGatewayImpl();
            _upgradeThroughExecute(address(next), "");

            assertEq(_implementationOf(address(gateway)), address(next), "implementation installed");
            assertEq(gateway.intrinsicModule(), next.intrinsicModule(), "modules follow the implementation");
            assertEq(gateway.extrinsicModule(), next.extrinsicModule());
            assertEq(gateway.version(), 3, "no migration ran");
            assertEq(gateway._nonce(), 1, "_nonce preserved");
            assertEq(gateway._orders(commitment, 0), 1000 * 1e6, "escrow preserved");
            assertEq(gateway.instance(bytes("DEST_CHAIN")), address(gateway), "peers preserved");
        }

        // The order placed before the first upgrade is still fillable through the modules the
        // last upgrade installed.
        vm.startPrank(solver);
        dai.approve(address(gateway), 900 * 1e18);
        gateway.fillOrder(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: order.output.assets,
                inputs: IntentQuoteTestUtils.inputs(order, order.output.assets)
            })
        );
        vm.stopPrank();
        assertEq(gateway._filled(commitment), solver, "filled after three upgrades");
    }

    /// An order placed through the previous intrinsic module remains publicly cancellable after
    /// an empty-data implementation upgrade installs a new intrinsic module.
    function testExpiredSameChainOrderCanBePubliclyCancelledAfterModuleUpgrade() public {
        uint256 amount = 1000 * 1e6;
        uint256 userBefore = usdc.balanceOf(user);
        Order memory order = _placeSameChainOrder(amount, 900 * 1e18);
        bytes32 commitment = keccak256(abi.encode(order));
        address previousIntrinsic = gateway.intrinsicModule();

        IntentGatewayV2 newImpl = deployIntentGatewayImpl();
        _upgradeThroughExecute(address(newImpl), "");
        assertNotEq(gateway.intrinsicModule(), previousIntrinsic, "intrinsic module switched");
        assertEq(gateway._orders(commitment, 0), amount, "pre-upgrade escrow preserved");

        vm.roll(order.deadline + 1);
        uint256 keeperBefore = usdc.balanceOf(solver);
        vm.prank(solver);
        gateway.cancelOrder(order, CancelOptions({relayerFee: 0, height: 0}));

        assertEq(usdc.balanceOf(user), userBefore, "original user refunded");
        assertEq(usdc.balanceOf(solver), keeperBefore, "keeper receives no escrow");
        assertEq(gateway._orders(commitment, 0), 0, "escrow cleared");
        assertEq(gateway._filled(commitment), user, "refund finalizes for original user");
    }

    /*//////////////////////////////////////////////////////////////
                                HELPERS
    //////////////////////////////////////////////////////////////*/

    /// @dev The ERC-1967 implementation slot: keccak256("eip1967.proxy.implementation") - 1.
    function _implementationOf(address proxy) internal view returns (address) {
        return
            address(
                uint160(uint256(vm.load(proxy, 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc)))
            );
    }

    /// @dev Governance upgrade as the pallet sends it: an `Execute` request from Hyperbridge
    /// carrying `upgradeToAndCall(newImpl, initData)`.
    function _upgradeRequest(address newImpl, bytes memory initData) internal view returns (PostRequest memory) {
        return PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(gateway)),
            to: abi.encodePacked(address(gateway)),
            body: bytes.concat(
                bytes1(uint8(IntentsBase.RequestKind.Execute)),
                abi.encodeCall(ExtrinsicIntents.upgradeToAndCall, (newImpl, initData))
            ),
            timeoutTimestamp: 0
        });
    }

    /// @dev Builds and delivers the upgrade through the host. Build the request yourself when a
    /// `vm.expectRevert` or `vm.expectEmit` must target the delivery.
    function _upgradeThroughExecute(address newImpl, bytes memory initData) internal {
        PostRequest memory request = _upgradeRequest(newImpl, initData);
        vm.prank(address(host));
        gateway.onAccept(IncomingPostRequest({relayer: address(this), request: request}));
    }

    function _sameChainOrder(uint256 inputAmount, uint256 outputAmount) internal view returns (Order memory) {
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});
        return Order({
            user: bytes32(0),
            source: "",
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputs, call: ""})
        });
    }

    function _placeSameChainOrder(uint256 inputAmount, uint256 outputAmount) internal returns (Order memory order) {
        order = _sameChainOrder(inputAmount, outputAmount);
        vm.startPrank(user);
        usdc.approve(address(gateway), inputAmount);
        gateway.placeOrder(order, bytes32(0));
        vm.stopPrank();
        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;
    }
}
