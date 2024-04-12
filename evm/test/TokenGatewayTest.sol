// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";

import {BaseTest} from "./BaseTest.sol";
import {GetResponseMessage, GetTimeoutMessage, GetRequest, PostRequest, Message} from "ismp/Message.sol";
import {
    TeleportParams,
    Body,
    BODY_BYTES_SIZE,
    Asset,
    BodyWithCall,
    AssetFees,
    TokenGatewayParamsExt
} from "../src/modules/TokenGateway.sol";
import {StateMachine} from "ismp/StateMachine.sol";

contract TokenGatewayTest is BaseTest {
    function testCanTeleportAssets() public {
        // relayer fee + per-byte fee
        uint256 messagingFee = (9 * 1e17) + (BODY_BYTES_SIZE * host.perByteFee());
        uint256 totalFee = 1_000 * 1e18 + messagingFee;
        feeToken.mint(address(this), totalFee, "");

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
                to: addressToBytes32(address(this)),
                assetId: keccak256("USD.h"),
                data: new bytes(0),
                amountInMax: 0
            })
        );

        assert(feeToken.balanceOf(address(this)) == 0);
        assert(feeToken.balanceOf(address(host)) == messagingFee);
    }

    function testCanTeleportAssetsWithCall() public {
        // relayer fee + per-byte fee
        uint256 messagingFee = (9 * 1e17) + (321 * host.perByteFee());
        uint256 totalFee = 1_000 * 1e18 + messagingFee;
        feeToken.mint(address(this), totalFee, "");

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
                to: addressToBytes32(address(miniStaking)),
                assetId: keccak256("USD.h"),
                data: stakeCalldata,
                amountInMax: 0
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
                to: addressToBytes32(address(this)),
                assetId: keccak256("USD.h"),
                data: new bytes(0),
                amountInMax: 0
            })
        );
    }

    function testCanReceiveAssets() public {
        assert(feeToken.balanceOf(address(this)) == 0);

        Body memory body = Body({
            assetId: keccak256("USD.h"),
            to: addressToBytes32(address(this)),
            redeem: false,
            amount: 1_000 * 1e18,
            from: addressToBytes32(address(this))
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
            to: addressToBytes32(address(miniStaking)),
            redeem: false,
            amount: 1_000 * 1e18,
            from: addressToBytes32(address(this)),
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
            to: addressToBytes32(address(this)),
            redeem: false,
            amount: 1_000 * 1e18,
            from: addressToBytes32(address(this))
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
        Asset memory asset = Asset({
            erc20: address(mockUSDC),
            erc6160: address(feeToken),
            identifier: keccak256("USD.h"),
            fees: AssetFees({
                protocolFeePercentage: 100, // 0.1
                relayerFeePercentage: 300 // 0.3
            })
        });

        Asset[] memory assets = new Asset[](1);
        assets[0] = asset;

        bytes memory hyperbridge = StateMachine.kusama(2000);
        TokenGatewayParamsExt memory params = TokenGatewayParamsExt({params: gateway.params(), assets: assets});

        vm.prank(address(host));
        gateway.onAccept(
            PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(gateway)),
                dest: new bytes(0),
                body: bytes.concat(hex"01", abi.encode(params)),
                nonce: 0,
                source: hyperbridge,
                timeoutTimestamp: 0
            })
        );

        console.log("Finished onAccept");

        bytes32 key = keccak256("USD.h");
        address erc6160Asset = gateway.erc6160(key);
        address erc20Asset = gateway.erc20(key);

        assert(erc6160Asset == address(feeToken));
        assert(erc20Asset == address(mockUSDC));
    }

    function testToRevertOnAddAssetOnAcceptForUnauthorizedRequest() public {
        Asset memory asset = Asset({
            erc20: address(mockUSDC),
            erc6160: address(feeToken),
            identifier: keccak256("USD.h"),
            fees: AssetFees({
                protocolFeePercentage: 100, // 0.1
                relayerFeePercentage: 300 // 0.3
            })
        });

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
        Asset memory asset = Asset({
            erc20: address(0),
            erc6160: address(0),
            identifier: keccak256("USD.h"),
            fees: AssetFees({
                protocolFeePercentage: 100, // 0.1
                relayerFeePercentage: 300 // 0.3
            })
        });

        Asset[] memory assets = new Asset[](1);
        assets[0] = asset;

        bytes memory hyperbridge = StateMachine.kusama(2000);
        TokenGatewayParamsExt memory params = TokenGatewayParamsExt({params: gateway.params(), assets: assets});

        vm.prank(address(host));
        gateway.onAccept(
            PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(gateway)),
                dest: new bytes(0),
                body: bytes.concat(hex"01", abi.encode(params)),
                nonce: 0,
                source: hyperbridge,
                timeoutTimestamp: 0
            })
        );

        bytes32 key = keccak256("USD.h");

        address erc6160Asset = gateway.erc6160(key);
        address erc20Asset = gateway.erc20(key);

        assert(erc6160Asset == address(0));
        assert(erc20Asset == address(0));
    }

    function testChangeRelayerFeeOnAccept() public {
        Asset memory asset = Asset({
            erc20: address(0),
            erc6160: address(0),
            identifier: keccak256("USD.h"),
            fees: AssetFees({
                protocolFeePercentage: 100, // 0.1
                relayerFeePercentage: 400 // 0.4
            })
        });

        Asset[] memory assets = new Asset[](1);
        assets[0] = asset;

        bytes memory hyperbridge = StateMachine.kusama(2000);
        TokenGatewayParamsExt memory params = TokenGatewayParamsExt({params: gateway.params(), assets: assets});

        vm.prank(address(host));

        gateway.onAccept(
            PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(gateway)),
                dest: new bytes(0),
                body: bytes.concat(hex"01", abi.encode(params)),
                nonce: 0,
                source: hyperbridge,
                timeoutTimestamp: 0
            })
        );

        assert(gateway.fees(keccak256("USD.h")).relayerFeePercentage == 400);
    }

    function test_ChangeProtocolFeeOnAccept() public {
        Asset memory asset = Asset({
            erc20: address(0),
            erc6160: address(0),
            identifier: keccak256("USD.h"),
            fees: AssetFees({
                protocolFeePercentage: 500, // 0.1
                relayerFeePercentage: 300 // 0.4
            })
        });

        Asset[] memory assets = new Asset[](1);
        assets[0] = asset;

        bytes memory hyperbridge = StateMachine.kusama(2000);
        TokenGatewayParamsExt memory params = TokenGatewayParamsExt({params: gateway.params(), assets: assets});

        vm.prank(address(host));

        gateway.onAccept(
            PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(gateway)),
                dest: new bytes(0),
                body: bytes.concat(hex"01", abi.encode(params)),
                nonce: 0,
                source: hyperbridge,
                timeoutTimestamp: 0
            })
        );

        assert(gateway.fees(keccak256("USD.h")).protocolFeePercentage == 500);
    }

    function testOnlyHostCanCallOnAccept() public {
        Body memory body = Body({
            assetId: keccak256("USD.h"),
            to: addressToBytes32(address(this)),
            redeem: false,
            amount: 1_000 * 1e18,
            from: addressToBytes32(address(this))
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
            to: addressToBytes32(address(this)),
            redeem: false,
            amount: 1_000 * 1e18,
            from: addressToBytes32(address(this))
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

    function testRelayerRedeemLiquidity() public {
        Asset memory asset = Asset({
            erc20: address(mockUSDC),
            erc6160: address(feeToken),
            identifier: keccak256("USD.h"),
            fees: AssetFees({
                protocolFeePercentage: 100, // 0.1
                relayerFeePercentage: 300 // 0.3
            })
        });

        Asset[] memory assets = new Asset[](1);
        assets[0] = asset;

        bytes memory hyperbridge = StateMachine.kusama(2000);
        TokenGatewayParamsExt memory params = TokenGatewayParamsExt({params: gateway.params(), assets: assets});

        feeToken.mint(address(this), 1_000 * 1e18, "");
        mockUSDC.mint(address(this), 1_000_000 * 1e18);

        vm.prank(address(host));

        // Adding Erc20 token to the existing `USD.h` asset using governance action
        gateway.onAccept(
            PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(gateway)),
                dest: new bytes(0),
                body: bytes.concat(hex"01", abi.encode(params)),
                nonce: 0,
                source: hyperbridge,
                timeoutTimestamp: 0
            })
        );

        // Send in ERC20assets to gateway contract, this is mimicking a user who locked there asset on this chain,
        // now the relayer is bringing the ERC6160 asset obatined from the other chain for providing this liquidity.
        mockUSDC.mint(address(gateway), 1_000 * 1e18);

        // Relayer USDC receaiving address
        address relayer_vault = address(1);

        Body memory redeemBody = Body({
            assetId: keccak256("USD.h"),
            to: addressToBytes32(relayer_vault),
            redeem: true,
            amount: 1_000 * 1e18,
            from: addressToBytes32(address(this))
        });

        vm.prank(address(host));
        gateway.onAccept(
            PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(gateway)),
                dest: new bytes(0),
                body: bytes.concat(hex"00", abi.encode(redeemBody)),
                nonce: 0,
                source: new bytes(0),
                timeoutTimestamp: 0
            })
        );

        uint256 protocolFee = 1_000 * 1e18 / 1000; // 0.1% of the total amount
        assert(mockUSDC.balanceOf(address(gateway)) == protocolFee); // this should be the protocol fee
        assert(mockUSDC.balanceOf(address(relayer_vault)) == 1_000 * 1e18 - protocolFee); // this should be the remaining amount
    }

    function testHandleIncomingAssetWithSwap() public {
        // Adding new Asset to the gateway
        Asset memory asset = Asset({
            erc20: address(hyperInu),
            erc6160: address(hyperInu_h),
            identifier: keccak256("HyperInu.h"),
            fees: AssetFees({
                protocolFeePercentage: 100, // 0.1
                relayerFeePercentage: 300 // 0.3
            })
        });

        Asset[] memory assets = new Asset[](1);
        assets[0] = asset;

        bytes memory hyperbridge = StateMachine.kusama(2000);
        TokenGatewayParamsExt memory params = TokenGatewayParamsExt({params: gateway.params(), assets: assets});

        // relayer fee + per-byte fee
        uint256 messagingFee = (9 * 1e17) + (BODY_BYTES_SIZE * host.perByteFee());
        feeToken.mint(address(this), 1_000 * 1e18 + messagingFee, "");

        vm.prank(address(host));
        gateway.onAccept(
            PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(gateway)),
                dest: new bytes(0),
                body: bytes.concat(hex"01", abi.encode(params)),
                nonce: 0,
                source: hyperbridge,
                timeoutTimestamp: 0
            })
        );

        address user_vault = address(1);
        address relayer_address = address(tx.origin);

        hyperInu.mint(relayer_address, 1_000 * 1e18);
        hyperInu.superApprove(relayer_address, address(gateway));

        Body memory body = Body({
            assetId: keccak256("HyperInu.h"),
            to: addressToBytes32(user_vault),
            redeem: false,
            amount: 1_000 * 1e18,
            from: addressToBytes32(address(this))
        });

        uint256 relayerBalanceBefore = hyperInu_h.balanceOf(relayer_address);

        // hitting the gateway with the incoming asset
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

        uint256 relayerBalanceAfter = hyperInu_h.balanceOf(relayer_address);

        uint256 liquidityFee = 3000000000000000000; // 0.3% of the total amount (997000000000000000000)

        assert(hyperInu.balanceOf(user_vault) == 1_000 * 1e18 - liquidityFee); // user should have the ERC20 token - fee
        assert((relayerBalanceAfter - relayerBalanceBefore) == 1_000 * 1e18); // relayer should have the ERC6160 token
    }
}

function addressToBytes32(address _address) pure returns (bytes32) {
    return bytes32(uint256(uint160(_address)));
}

function bytes32ToAddress(bytes32 _bytes) pure returns (address) {
    return address(uint160(uint256(_bytes)));
}
