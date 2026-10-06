// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import {Test} from "forge-std/Test.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {ERC1967Utils} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Utils.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";
import {PostRequest} from "@hyperbridge/core/libraries/Message.sol";

import {SimplexPaymaster, AggregatorV3Interface} from "../../src/utils/SimplexPaymaster.sol";
import {FixedPriceAggregator} from "../../script/testnet/FixedPriceAggregator.sol";

contract KusamaHost {
    function hyperbridge() external pure returns (bytes memory) {
        return bytes("KUSAMA-4009");
    }
}

/// @notice Pins the testnet `upgrade_paymaster` init_data that sets the bundler allowlist on a
///         runtime without `set_paymaster_bundlers`. The outer `UpgradeContract` points the proxy
///         at its current implementation and delegatecalls this init_data, a nested `onAccept`
///         carrying `SetBundlers`, with the host still `msg.sender`.
contract SimplexPaymasterBundlerRolloutTest is Test {
    address constant RELAYER = 0xc8809DD0b00370be097382d741A43347Ad582757;
    address constant RUNDLER_SIGNER = 0x2E0E23636E27826193EA4A585a05CD1808d3c96c;
    address constant RUNDLER_SIMULATION_ORIGIN = 0x0643866dA50efE0b055Cd15aF95191968c8411b5;

    // `node e2e/entrypoint-v09-sudo.mjs init-data` in sdk/packages/simplex prints both.
    bytes constant ALLOW_INIT_DATA =
        hex"0fee32ce00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000040000000000000000000000000c8809dd0b00370be097382d741a43347ad58275700000000000000000000000000000000000000000000000000000000000000e00000000000000000000000000000000000000000000000000000000000000120000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001400000000000000000000000000000000000000000000000000000000000000180000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001a0000000000000000000000000000000000000000000000000000000000000000b4b5553414d412d343030390000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000e70616c6c65742d696e74656e7473000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a1080000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000020000000000000000000000002e0e23636e27826193ea4a585a05cd1808d3c96c0000000000000000000000000643866da50efe0b055cd15af95191968c8411b500000000000000000000000000000000000000000000000000000000000000";
    bytes constant CLEAR_INIT_DATA =
        hex"0fee32ce00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000040000000000000000000000000c8809dd0b00370be097382d741a43347ad58275700000000000000000000000000000000000000000000000000000000000000e00000000000000000000000000000000000000000000000000000000000000120000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001400000000000000000000000000000000000000000000000000000000000000180000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001a0000000000000000000000000000000000000000000000000000000000000000b4b5553414d412d343030390000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000e70616c6c65742d696e74656e7473000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a1080000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000002e0e23636e27826193ea4a585a05cd1808d3c96c0000000000000000000000000643866da50efe0b055cd15af95191968c8411b500000000000000000000000000000000000000000000000000000000000000";

    KusamaHost host;
    SimplexPaymaster paymaster;
    address implementation;

    function setUp() public {
        host = new KusamaHost();
        implementation = address(new SimplexPaymaster());
        SimplexPaymaster.Params memory params = SimplexPaymaster.Params({
            nativeOracle: AggregatorV3Interface(address(new FixedPriceAggregator(600e8, 8))),
            markupBps: 200,
            treasury: address(this),
            maxOracleAge: 90_000,
            swapSlippageBps: 200
        });
        bytes memory init = abi.encodeCall(
            SimplexPaymaster.initialize,
            (address(host), params, new address[](0), new AggregatorV3Interface[](0), RELAYER)
        );
        paymaster = SimplexPaymaster(payable(address(new ERC1967Proxy(implementation, init))));
    }

    function testInitDataIsTheNestedSetBundlersRequest() public pure {
        assertEq(ALLOW_INIT_DATA, _initData(_defaultBundlers(), true, RELAYER));
        assertEq(CLEAR_INIT_DATA, _initData(_defaultBundlers(), false, RELAYER));
    }

    function testUpgradeToCurrentImplementationSetsThenClearsBundlers() public {
        _upgradePaymaster(ALLOW_INIT_DATA);
        assertEq(paymaster.getBundlers(), _defaultBundlers());
        assertEq(_implementation(), implementation);
        assertEq(paymaster.relayer(), RELAYER);

        _upgradePaymaster(CLEAR_INIT_DATA);
        assertEq(paymaster.getBundlers().length, 0);
        assertEq(_implementation(), implementation);
    }

    function testNestedRequestFromAnotherRelayerReverts() public {
        vm.expectRevert(SimplexPaymaster.UnauthorizedRelayer.selector);
        _upgradePaymaster(_initData(_defaultBundlers(), true, makeAddr("other")));
    }

    /// @dev What the relayer delivers for `upgrade_paymaster(Evm(_), implementation, initData)`.
    function _upgradePaymaster(bytes memory initData) internal {
        PostRequest memory request = PostRequest({
            source: bytes("KUSAMA-4009"),
            dest: bytes("EVM-97"),
            nonce: 0,
            from: bytes("pallet-intents"),
            to: abi.encodePacked(address(paymaster)),
            timeoutTimestamp: 0,
            body: bytes.concat(
                bytes1(uint8(SimplexPaymaster.RequestKind.UpgradeContract)), abi.encode(implementation, initData)
            )
        });
        vm.prank(address(host));
        paymaster.onAccept(IncomingPostRequest({request: request, relayer: RELAYER}));
    }

    function _initData(address[] memory bundlers, bool allowed, address relayer) internal pure returns (bytes memory) {
        PostRequest memory request = PostRequest({
            source: bytes("KUSAMA-4009"),
            dest: "",
            nonce: 0,
            from: bytes("pallet-intents"),
            to: "",
            timeoutTimestamp: 0,
            body: bytes.concat(bytes1(uint8(SimplexPaymaster.RequestKind.SetBundlers)), abi.encode(bundlers, allowed))
        });
        return abi.encodeCall(SimplexPaymaster.onAccept, (IncomingPostRequest({request: request, relayer: relayer})));
    }

    function _defaultBundlers() internal pure returns (address[] memory bundlers) {
        bundlers = new address[](2);
        bundlers[0] = RUNDLER_SIGNER;
        bundlers[1] = RUNDLER_SIMULATION_ORIGIN;
    }

    function _implementation() internal view returns (address) {
        return address(uint160(uint256(vm.load(address(paymaster), ERC1967Utils.IMPLEMENTATION_SLOT))));
    }
}
