// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.13;

import {Test, console2} from "forge-std/Test.sol";
import {TokenGateway, SendParams} from "../src/modules/TokenGateway.sol";
import {MultiChainNativeERC20} from "multi-chain-tokens/tokens/ERC20.sol";
import {ERC20Token} from "./mocks/ERC20Token.sol";
import "openzeppelin/utils/math/Math.sol";
import "ismp/StateMachine.sol";
import {MockHost} from "./mocks/MockHost.sol";
import {MockAutoRelayer} from "./mocks/MockAutoRelayer.sol";

contract TokenGatewayTest is Test {
    bytes chain_a = StateMachine.ethereum();
    bytes chain_b = StateMachine.optimism();

    MockHost chain_a_host;
    MockHost chain_b_host;

    MockAutoRelayer relayer;

    TokenGateway chain_a_gateway;
    TokenGateway chain_b_gateway;

    ERC20Token chain_a_dai;
    ERC20Token chain_b_dai;
    ERC20Token chain_a_usdc;
    ERC20Token chain_b_usdc;
    MultiChainNativeERC20 chain_a_wrapped_usdc;
    MultiChainNativeERC20 chain_b_wrapped_usdc;

    address owner = vm.addr(uint256(keccak256("owner")));
    address user1 = vm.addr(uint256(keccak256("user1")));
    address user2 = vm.addr(uint256(keccak256("user2")));

    function label() private {
        vm.label(user1, "user1");
        vm.label(user2, "user2");
        vm.label(address(chain_a_host), "chain_a_host");
        vm.label(address(chain_b_host), "chain_b_host");
        vm.label(address(relayer), "relayer");
        vm.label(address(chain_a_gateway), "chain_a_gateway");
        vm.label(address(chain_b_gateway), "chain_b_gateway");
        vm.label(address(chain_a_dai), "chain_a_dai");
        vm.label(address(chain_b_dai), "chain_b_dai");
        vm.label(address(chain_a_usdc), "chain_a_usdc");
        vm.label(address(chain_b_usdc), "chain_b_usdc");
        vm.label(address(chain_a_wrapped_usdc), "chain_a_wrapped_usdc");
        vm.label(address(chain_b_wrapped_usdc), "chain_b_wrapped_usdc");
    }

    function setUp() external {
        vm.startPrank(owner);

        chain_a_dai = new ERC20Token("DAI-A", "DAI", 18);
        chain_b_dai = new ERC20Token("DAI-B", "DAI", 18);
        chain_a_usdc = new ERC20Token("USDC-A", "USDC", 18);
        chain_b_usdc = new ERC20Token("USDC-B", "USDC", 18);

        chain_a_gateway = new TokenGateway(owner);
        chain_b_gateway = new TokenGateway(owner);

        chain_a_wrapped_usdc = new MultiChainNativeERC20(address(chain_a_gateway), "USDC-A", "USDC");
        chain_b_wrapped_usdc = new MultiChainNativeERC20(address(chain_b_gateway), "USDC-B", "USDC");

        relayer = new MockAutoRelayer();

        chain_a_host = new MockHost(chain_a, address(chain_a_dai), address(relayer));
        chain_b_host = new MockHost(chain_b, address(chain_b_dai), address(relayer));

        label();

        relayer.set(address(chain_a_host), address(chain_b_host));

        chain_a_dai.mint(address(relayer), 1_000_000e18);
        chain_a_usdc.mint(address(relayer), 1_000_000e18);
        chain_b_dai.mint(address(relayer), 1_000_000e18);
        chain_b_usdc.mint(address(relayer), 1_000_000e18);

        chain_a_dai.mint(user1, 1_000_000e18);
        chain_a_usdc.mint(user1, 1_000_000e18);
        chain_b_dai.mint(user1, 1_000_000e18);
        chain_b_usdc.mint(user1, 1_000_000e18);

        chain_a_dai.mint(user2, 1_000_000e18);
        chain_a_usdc.mint(user2, 1_000_000e18);
        chain_b_dai.mint(user2, 1_000_000e18);
        chain_b_usdc.mint(user2, 1_000_000e18);

        chain_a_gateway.setIsmpHost(address(chain_a_host));
        chain_a_gateway.setTokenIdentifierDetails(
            keccak256("USDC-A"), address(chain_a_usdc), address(chain_a_wrapped_usdc)
        );
        chain_a_gateway.setForeignTokenIdToLocalTokenId(keccak256("USDC-B"), keccak256("USDC-A"));
        chain_a_gateway.setChainsGateway(chain_b, address(chain_b_gateway));
        chain_b_gateway.setIsmpHost(address(chain_b_host));
        chain_b_gateway.setTokenIdentifierDetails(
            keccak256("USDC-B"), address(chain_b_usdc), address(chain_b_wrapped_usdc)
        );
        chain_b_gateway.setForeignTokenIdToLocalTokenId(keccak256("USDC-A"), keccak256("USDC-B"));
        chain_b_gateway.setChainsGateway(chain_a, address(chain_a_gateway));

        vm.startPrank(user1);
        chain_a_dai.approve(address(chain_a_host), type(uint256).max);
        chain_b_dai.approve(address(chain_b_host), type(uint256).max);
        chain_a_usdc.approve(address(chain_a_gateway), type(uint256).max);
        chain_b_usdc.approve(address(chain_b_gateway), type(uint256).max);
        chain_a_wrapped_usdc.approve(address(chain_a_gateway), type(uint256).max);
        chain_b_wrapped_usdc.approve(address(chain_b_gateway), type(uint256).max);

        vm.startPrank(user2);
        chain_a_dai.approve(address(chain_a_host), type(uint256).max);
        chain_b_dai.approve(address(chain_b_host), type(uint256).max);
        chain_a_usdc.approve(address(chain_a_gateway), type(uint256).max);
        chain_b_usdc.approve(address(chain_b_gateway), type(uint256).max);
        chain_a_wrapped_usdc.approve(address(chain_a_gateway), type(uint256).max);
        chain_b_wrapped_usdc.approve(address(chain_b_gateway), type(uint256).max);

        vm.startPrank(address(relayer));
        chain_a_dai.approve(address(chain_a_host), type(uint256).max);
        chain_b_dai.approve(address(chain_b_host), type(uint256).max);
        chain_a_usdc.approve(address(chain_a_gateway), type(uint256).max);
        chain_b_usdc.approve(address(chain_b_gateway), type(uint256).max);
        chain_a_wrapped_usdc.approve(address(chain_a_gateway), type(uint256).max);
        chain_b_wrapped_usdc.approve(address(chain_b_gateway), type(uint256).max);
    }

    // This is a fuzz test for user sending tx from a chain to another
    function test_sendFromChainAToB(uint256 amount, address to, uint256 fee) external {
        amount = bound(amount, 0, 1_000_000e18);
        fee = bound(fee, 0, 1_000_000e18);
        if (to == address(0)) to = mutateAddress(to);

        vm.startPrank(user1, user1);

        SendParams memory sendParams = SendParams({
            amount: amount,
            fee: fee,
            gaslimit: type(uint64).max,
            tokenId: keccak256("USDC-A"),
            to: to,
            dest: chain_b,
            timeout: 1000,
            redeem: false
        });

        uint256 hostDaiAPreBalance = chain_a_dai.balanceOf(address(chain_a_host));
        uint256 user1DaiAPreBalance = chain_a_dai.balanceOf(user1);
        uint256 user1UsdcAPreBalance = chain_a_usdc.balanceOf(user1);
        uint256 gatewayUsdcAPreBalance = chain_a_usdc.balanceOf(address(chain_a_gateway));
        uint256 relayerWrappedUsdcBPreBalance = chain_b_wrapped_usdc.balanceOf(address(relayer));
        uint256 toUsdcBPreBalance = chain_b_usdc.balanceOf(to);
        uint256 relayerUsdcBPreBalance = chain_b_usdc.balanceOf(address(relayer));

        (, address msgSender, address txOrigin) = vm.readCallers();
        chain_a_gateway.send(sendParams);
        vm.startPrank(msgSender, txOrigin);

        assertEq(chain_a_dai.balanceOf(address(chain_a_host)), hostDaiAPreBalance + fee);
        assertEq(chain_a_dai.balanceOf(address(user1)), user1DaiAPreBalance - fee);
        assertEq(chain_a_usdc.balanceOf(user1), user1UsdcAPreBalance - amount);
        assertEq(chain_a_usdc.balanceOf(address(chain_a_gateway)), gatewayUsdcAPreBalance + amount);
        assertEq(chain_b_wrapped_usdc.balanceOf(address(relayer)), relayerWrappedUsdcBPreBalance + amount);
        assertEq(chain_b_usdc.balanceOf(to), toUsdcBPreBalance + amount);
        assertEq(chain_b_usdc.balanceOf(address(relayer)), relayerUsdcBPreBalance - amount);
    }

    // This is a fuzz test for user sending tx from a chain to another
    function test_relayerRedeemLiquidity(uint256 amountToRedeem, address to, uint256 redeemFee) external {
        amountToRedeem = bound(amountToRedeem, 0, 1_000_000e18);
        redeemFee = bound(redeemFee, 0, 1_000_000e18);
        if (to == address(0)) to = mutateAddress(to);

        // make a bridge tx so that relayer has some wrapper tokens
        vm.startPrank(user1, user1);
        SendParams memory sendParams = SendParams({
            amount: 1_000_000e18,
            fee: 100e18,
            gaslimit: type(uint64).max,
            tokenId: keccak256("USDC-B"),
            to: user1,
            dest: chain_a,
            timeout: 1000,
            redeem: false
        });
        (, address msgSender, address txOrigin) = vm.readCallers();
        chain_b_gateway.send(sendParams);
        vm.startPrank(msgSender, txOrigin);

        uint256 hostDaiAPreBalance = chain_a_dai.balanceOf(address(chain_a_host));
        uint256 relayerDaiAPreBalance = chain_a_dai.balanceOf(address(relayer));
        uint256 relayerWrappedUsdcAPreBalance = chain_a_wrapped_usdc.balanceOf(address(relayer));
        uint256 gatewayUsdcBPreBalance = chain_b_usdc.balanceOf(address(chain_b_gateway));
        uint256 toUsdcBPreBalance = chain_b_usdc.balanceOf(to);

        // redeem by relayer
        vm.startPrank(address(relayer), address(relayer));
        sendParams = SendParams({
            amount: amountToRedeem,
            fee: redeemFee,
            gaslimit: type(uint64).max,
            tokenId: keccak256("USDC-A"),
            to: to,
            dest: chain_b,
            timeout: 1000,
            redeem: true
        });
        (, msgSender, txOrigin) = vm.readCallers();
        chain_a_gateway.send(sendParams);

        assertEq(chain_a_dai.balanceOf(address(chain_a_host)), hostDaiAPreBalance + redeemFee);
        assertEq(chain_a_dai.balanceOf(address(relayer)), relayerDaiAPreBalance - redeemFee);
        assertEq(chain_a_wrapped_usdc.balanceOf(address(relayer)), relayerWrappedUsdcAPreBalance - amountToRedeem);
        assertEq(chain_b_usdc.balanceOf(address(chain_b_gateway)), gatewayUsdcBPreBalance - amountToRedeem);
        assertEq(chain_b_usdc.balanceOf(to), toUsdcBPreBalance + amountToRedeem);
    }

    function mutateAddress(address addr) internal pure returns (address) {
        unchecked {
            return address(uint160(uint256(keccak256(abi.encode(addr)))));
        }
    }
}
