// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";

import {BaseTest} from "./BaseTest.sol";
import {GetResponseMessage, GetTimeoutMessage, GetRequest, PostRequest, Message} from "ismp/Message.sol";
import {TeleportParams, Body} from "../src/modules/TokenGateway.sol";
import {StateMachine} from "ismp/StateMachine.sol";

contract TokenGatewayTest is BaseTest {
    function testCanTeleportAssets() public {
        feeToken.mint(address(this), 1_000 * 1e18, "");

        assert(feeToken.balanceOf(address(this)) == 1_000 * 1e18);

        gateway.teleport(
            TeleportParams({
                feeToken: address(feeToken),
                amount: 1000 * 1e18, // $1000
                redeem: false,
                dest: StateMachine.bsc(),
                fee: 9 * 1e17, // $0.9
                timeout: 0,
                to: address(this),
                tokenId: keccak256("USD.h")
            })
        );

        assert(feeToken.balanceOf(address(this)) == 0);
    }

    function testCannotTeleportAssetsWithInsufficientBalance() public {
        vm.expectRevert(bytes("ERC20: burn amount exceeds balance"));
        gateway.teleport(
            TeleportParams({
                feeToken: address(feeToken),
                amount: 1000 * 1e18, // $1000
                redeem: false,
                dest: StateMachine.bsc(),
                fee: 9 * 1e17, // $0.9
                timeout: 0,
                to: address(this),
                tokenId: keccak256("USD.h")
            })
        );

        assert(feeToken.balanceOf(address(this)) == 0);
    }

    function testCanReceiveAssets() public {
        assert(feeToken.balanceOf(address(this)) == 0);

        Body memory body = Body({
            tokenId: keccak256("USD.h"),
            to: address(this),
            redeem: false,
            amount: 1_000 * 1e18,
            from: address(this)
        });
        vm.prank(address(host));
        gateway.onAccept(
            PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(gateway)),
                dest: new bytes(0),
                body: bytes.concat(hex"00", abi.encode(body)),
                gaslimit: uint64(0),
                nonce: 0,
                source: new bytes(0),
                timeoutTimestamp: 0
            })
        );

        assert(feeToken.balanceOf(address(this)) == 1_000 * 1e18);
    }

    function testOnlyHostCanCallOnAccept() public {
        Body memory body = Body({
            tokenId: keccak256("USD.h"),
            to: address(this),
            redeem: false,
            amount: 1_000 * 1e18,
            from: address(this)
        });
        vm.expectRevert(bytes("TokenGateway: Unauthorized action"));
        gateway.onAccept(
            PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(gateway)),
                dest: new bytes(0),
                body: bytes.concat(hex"00", abi.encode(body)),
                gaslimit: uint64(0),
                nonce: 0,
                source: new bytes(0),
                timeoutTimestamp: 0
            })
        );
    }

    function testWillRejectRequestFromUnkownApplication() public {
        Body memory body = Body({
            tokenId: keccak256("USD.h"),
            to: address(this),
            redeem: false,
            amount: 1_000 * 1e18,
            from: address(this)
        });
        vm.startPrank(address(host));
        vm.expectRevert(bytes("Unauthorized request"));
        gateway.onAccept(
            PostRequest({
                to: abi.encodePacked(address(0)),
                // not from gateway
                from: abi.encodePacked(address(this)),
                dest: new bytes(0),
                body: bytes.concat(hex"00", abi.encode(body)),
                gaslimit: uint64(0),
                nonce: 0,
                source: new bytes(0),
                timeoutTimestamp: 0
            })
        );
    }
}
