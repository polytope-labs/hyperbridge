// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";

import {BaseTest} from "./BaseTest.sol";
import {GetResponseMessage, GetTimeoutMessage, GetRequest, PostRequest, Message} from "ismp/Message.sol";
import {TeleportParams, Body, BODY_BYTES_SIZE, Asset, BodyWithCall} from "../src/modules/TokenGateway.sol";
import {StateMachine} from "ismp/StateMachine.sol";

contract TokenGatewayTest is BaseTest {
    function testCanTeleportAssets() public {
        // relayer fee + per-byte fee
        uint256 messagingFee = (9 * 1e17) + (BODY_BYTES_SIZE * host.perByteFee());
        feeToken.mint(address(this), 1_000 * 1e18 + messagingFee, "");

        assert(feeToken.balanceOf(address(this)) == 1_000 * 1e18 + messagingFee);
        assert(feeToken.balanceOf(address(host)) == 0);

        gateway.teleport(
            TeleportParams({
                feeToken: address(feeToken),
                amount: 1000 * 1e18, // $1000
                redeem: false,
                dest: StateMachine.bsc(),
                fee: 9 * 1e17, // $0.9
                timeout: 0,
                to: address(this),
                assetId: keccak256("USD.h"),
                data: new bytes(0)
            })
        );

        assert(feeToken.balanceOf(address(this)) == 0);

        assert(feeToken.balanceOf(address(host)) == messagingFee);
    }

    function testCanTeleportAssetsWithCall() public {
        // relayer fee + per-byte fee
        uint256 messagingFee = (9 * 1e17) + (321 * host.perByteFee());
        feeToken.mint(address(this), 1_000 * 1e18 + messagingFee, "");

        assert(feeToken.balanceOf(address(this)) == 1_000 * 1e18 + messagingFee);
        assert(feeToken.balanceOf(address(host)) == 0);

        bytes memory stakeCalldata = abi.encodeWithSignature("recordStake(address)", address(miniStaking));

        gateway.teleport(
            TeleportParams({
                feeToken: address(feeToken),
                amount: 1000 * 1e18, // $1000
                redeem: false,
                dest: StateMachine.bsc(),
                fee: 9 * 1e17, // $0.9
                timeout: 0,
                to: address(miniStaking),
                assetId: keccak256("USD.h"),
                data: stakeCalldata
            })
        );

        assert(feeToken.balanceOf(address(this)) == 0);
        assert(feeToken.balanceOf(address(host)) == messagingFee);
    }

    function testCannotTeleportAssetsWithInsufficientBalance() public {
        assert(feeToken.balanceOf(address(this)) == 0);

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
                assetId: keccak256("USD.h"),
                data: new bytes(0)
            })
        );
    }

    function testCanReceiveAssets() public {
        assert(feeToken.balanceOf(address(this)) == 0);

        Body memory body = Body({
            assetId: keccak256("USD.h"),
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
                nonce: 0,
                source: new bytes(0),
                timeoutTimestamp: 0
            })
        );

        assert(feeToken.balanceOf(address(this)) == 1_000 * 1e18);
    }

    function testCanReceiveAssetsWithCall() public {
        assert(feeToken.balanceOf(address(this)) == 0);

        bytes memory stakeCalldata = abi.encodeWithSignature("recordStake(address)", address(this));

        BodyWithCall memory body = BodyWithCall({
            assetId: keccak256("USD.h"),
            to: address(miniStaking),
            redeem: false,
            amount: 1_000 * 1e18,
            from: address(this),
            data: stakeCalldata
        });

        vm.prank(address(host));
        gateway.onAccept(
            PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(gateway)),
                dest: new bytes(0),
                body: bytes.concat(hex"00", abi.encode(body)),
                nonce: 0,
                source: new bytes(0),
                timeoutTimestamp: 0
            })
        );

        assert(miniStaking.balanceOf(address(this)) == 1_000 * 1e18);
    }

    function testCanTimeoutRequest() public {
        assert(feeToken.balanceOf(address(this)) == 0);

        Body memory body = Body({
            assetId: keccak256("USD.h"),
            to: address(this),
            redeem: false,
            amount: 1_000 * 1e18,
            from: address(this)
        });
        vm.prank(address(host));
        gateway.onPostRequestTimeout(
            PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(gateway)),
                dest: new bytes(0),
                body: bytes.concat(hex"00", abi.encode(body)),
                nonce: 0,
                source: new bytes(0),
                timeoutTimestamp: 0
            })
        );

        assert(feeToken.balanceOf(address(this)) == 1_000 * 1e18);
    }

    function getMappingValue(address target, uint256 mapSlot, bytes32 key) public view returns (bytes32) {
        bytes32 slotValue = vm.load(target, keccak256(abi.encode(key, mapSlot)));
        return slotValue;
    }

    function testAddAssetOnAccept() public {
        Asset memory asset =
            Asset({erc20: address(mockUSDC), erc6160: address(feeToken), identifier: keccak256("USD.h")});

        Asset[] memory assets = new Asset[](1);
        assets[0] = asset;

        bytes memory hyperbridge = host.hyperbridge();

        vm.prank(address(host));

        gateway.onAccept(
            PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(gateway)),
                dest: new bytes(0),
                body: bytes.concat(hex"0100", abi.encode(assets)),
                nonce: 0,
                source: hyperbridge,
                timeoutTimestamp: 0
            })
        );

        bytes32 key = keccak256("USD.h");
        bytes32 Erc6160Asset = getMappingValue(address(gateway), 7, key);

        bytes32 Erc20Asset = getMappingValue(address(gateway), 8, key);

        address erc6160Asset = address(uint160(uint256(Erc6160Asset)));
        address erc20Asset = address(uint160(uint256(Erc20Asset)));

        assert(erc6160Asset == address(feeToken));
        assert(erc20Asset == address(mockUSDC));
    }

    function testToRevertOnAddAssetOnAcceptForUnauthorizedRequest() public {
        Asset memory asset =
            Asset({erc20: address(mockUSDC), erc6160: address(feeToken), identifier: keccak256("USD.h")});

        Asset[] memory assets = new Asset[](1);
        assets[0] = asset;

        vm.prank(address(host));

        vm.expectRevert(bytes("Unauthorized request"));

        gateway.onAccept(
            PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(gateway)),
                dest: new bytes(0),
                body: bytes.concat(hex"0100", abi.encode(assets)),
                nonce: 0,
                source: new bytes(0),
                timeoutTimestamp: 0
            })
        );
    }

    function testRemoveAssetOnAccept() public {
        Asset memory asset = Asset({erc20: address(0), erc6160: address(0), identifier: keccak256("USD.h")});

        Asset[] memory assets = new Asset[](1);
        assets[0] = asset;

        bytes memory hyperbridge = host.hyperbridge();

        vm.prank(address(host));

        gateway.onAccept(
            PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(gateway)),
                dest: new bytes(0),
                body: bytes.concat(hex"0100", abi.encode(assets)),
                nonce: 0,
                source: hyperbridge,
                timeoutTimestamp: 0
            })
        );

        bytes32 key = keccak256("USD.h");
        bytes32 Erc6160Asset = getMappingValue(address(gateway), 6, key);

        bytes32 Erc20Asset = getMappingValue(address(gateway), 7, key);

        address erc6160Asset = address(uint160(uint256(Erc6160Asset)));
        address erc20Asset = address(uint160(uint256(Erc20Asset)));

        assert(erc6160Asset == address(0));
        assert(erc20Asset == address(0));
    }

    function testChangeRelayerFeeOnAccept() public {
        bytes memory hyperbridge = host.hyperbridge();

        vm.prank(address(host));

        gateway.onAccept(
            PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(gateway)),
                dest: new bytes(0),
                body: bytes.concat(hex"0101", abi.encode(400)),
                nonce: 0,
                source: hyperbridge,
                timeoutTimestamp: 0
            })
        );

        uint256 slot = 3;
        bytes32 slotValue = vm.load(address(gateway), bytes32(slot));

        uint256 relayerFeePercentage = uint256(slotValue);

        assert(relayerFeePercentage == 400);
    }

    function test_ChangeProtocolFeeOnAccept() public {
        bytes memory hyperbridge = host.hyperbridge();

        vm.prank(address(host));

        gateway.onAccept(
            PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(gateway)),
                dest: new bytes(0),
                body: bytes.concat(hex"0102", abi.encode(500)),
                nonce: 0,
                source: hyperbridge,
                timeoutTimestamp: 0
            })
        );

        uint256 slot = 4;
        bytes32 slotValue = vm.load(address(gateway), bytes32(slot));

        console.logBytes32(slotValue);

        uint256 protocolFeePercentage = uint256(slotValue);

        assert(protocolFeePercentage == 500);
    }

    function testOnlyHostCanCallOnAccept() public {
        Body memory body = Body({
            assetId: keccak256("USD.h"),
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
                nonce: 0,
                source: new bytes(0),
                timeoutTimestamp: 0
            })
        );
    }

    function testWillRejectRequestFromUnkownApplication() public {
        Body memory body = Body({
            assetId: keccak256("USD.h"),
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
                nonce: 0,
                source: new bytes(0),
                timeoutTimestamp: 0
            })
        );
    }
}
