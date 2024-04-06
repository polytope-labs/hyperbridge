// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";
import "forge-std/console.sol";

import {MainnetForkBaseTest} from "./MainnetForkBaseTest.sol";
import {GetResponseMessage, GetTimeoutMessage, GetRequest, PostRequest, Message} from "ismp/Message.sol";
import {TeleportParams, Body, BODY_BYTES_SIZE} from "../src/modules/TokenGateway.sol";
import {StateMachine} from "ismp/StateMachine.sol";

contract TeleportSwapTest is MainnetForkBaseTest {
    // Maximum slippage of 0.5%
    uint256 maxSlippagePercentage = 50; // 0.5 * 100

    function testCanTeleportAssetsUsingUsdcForFee() public {
        address mainnetUsdcHolder = address(0xf584F8728B874a6a5c7A8d4d387C9aae9172D621);
        
        // relayer fee + per-byte fee
        uint256 messagingFee = (9 * 1e17) + (BODY_BYTES_SIZE * host.perByteFee());

        address[] memory path = new address[](2);
        path[0] = address(usdc);
        path[1] = address(feeToken);

        uint256 _fromTokenAmountIn = _uniswapV2Router.getAmountsIn(messagingFee, path)[0];

        // Handling Slippage Implementation
        uint _slippageAmount = (_fromTokenAmountIn * maxSlippagePercentage) / 10000; // Adjusted for percentage times 100
        uint _amountInMax = _fromTokenAmountIn + _slippageAmount;

        // mainnet forking - impersonation
        vm.startPrank(mainnetUsdcHolder);

        dai.approve(address(gateway), 10000 * 1e18);
        dai.approve(address(host), messagingFee);
        usdc.approve(address(gateway), 10000 * 1e18);

        gateway.teleport(
            TeleportParams({
                feeToken: address(usdc),
                amount: 1000 * 1e18, // $1000
                redeem: false,
                dest: StateMachine.bsc(),
                fee: 9 * 1e17, // $0.9
                timeout: 0,
                to: address(this),
                assetId: keccak256("USD.h"),
                data: new bytes(0),
                amountInMax: _amountInMax
            })
        );

        assert(feeToken.balanceOf(address(this)) == 0);
        assert(feeToken.balanceOf(address(host)) == messagingFee);
    }
}