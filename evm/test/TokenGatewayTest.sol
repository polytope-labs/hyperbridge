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

import "forge-std/Test.sol";

import {BaseTest} from "./BaseTest.sol";
import {IncomingPostRequest} from "@polytope-labs/ismp-solidity/IIsmpModule.sol";
import "@polytope-labs/ismp-solidity/Message.sol";
import {StateMachine} from "@polytope-labs/ismp-solidity/StateMachine.sol";
import {NotRoleAdmin} from "@polytope-labs/erc6160/tokens/ERC6160Ext20.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {ERC6160Ext20} from "@polytope-labs/erc6160/tokens/ERC6160Ext20.sol";
import "../src/modules/TokenGateway.sol";
import "../src/modules/CallDispatcher.sol";

contract TokenGatewayTest is BaseTest {
    using Message for PostRequest;

    function testCanTeleportAssets() public {
        // relayer fee + per-byte fee
        uint256 messagingFee = (9 * 1e17) + (BODY_BYTES_SIZE * host.perByteFee(StateMachine.evm(97)));
        uint256 totalFee = 1_000 * 1e18 + messagingFee;
        feeToken.mint(address(this), totalFee);

        assert(feeToken.balanceOf(address(this)) == 1_000 * 1e18 + messagingFee);
        assert(feeToken.balanceOf(address(host)) == 0);

        gateway.teleport(
            TeleportParams({
                amount: 1000 * 1e18, // $1000
                redeem: false,
                dest: StateMachine.evm(97),
                relayerFee: 9 * 1e17, // $0.9
                timeout: 0,
                to: addressToBytes32(address(this)),
                assetId: keccak256("USD.h"),
                data: new bytes(0),
                nativeCost: 0
            })
        );

        assert(feeToken.balanceOf(address(this)) == 0);
        assert(feeToken.balanceOf(address(host)) == messagingFee);
    }

    function testCanTeleportAssetsWithCall() public {
        // relayer fee + per-byte fee
        uint256 messagingFee = (9 * 1e17) + (321 * host.perByteFee(StateMachine.evm(97)));
        uint256 totalFee = 1_000 * 1e18 + messagingFee;
        feeToken.mint(address(this), totalFee);

        assert(feeToken.balanceOf(address(this)) == 1_000 * 1e18 + messagingFee);
        assert(feeToken.balanceOf(address(host)) == 0);

        bytes memory stakeCalldata = abi.encodeWithSignature("recordStake(address)", address(miniStaking));

        gateway.teleport(
            TeleportParams({
                amount: 1000 * 1e18, // $1000
                redeem: false,
                dest: StateMachine.evm(97),
                relayerFee: 9 * 1e17, // $0.9
                timeout: 0,
                to: addressToBytes32(address(miniStaking)),
                assetId: keccak256("USD.h"),
                data: stakeCalldata,
                nativeCost: 0
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
                amount: 1000 * 1e18, // $1000
                redeem: false,
                dest: StateMachine.evm(97),
                relayerFee: 9 * 1e17, // $0.9
                timeout: 0,
                to: addressToBytes32(address(this)),
                assetId: keccak256("USD.h"),
                data: new bytes(0),
                nativeCost: 0
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
            IncomingPostRequest({
                request: PostRequest({
                    to: abi.encodePacked(address(0)),
                    from: abi.encodePacked(address(gateway)),
                    dest: new bytes(0),
                    body: bytes.concat(hex"00", abi.encode(body)),
                    nonce: 0,
                    source: new bytes(0),
                    timeoutTimestamp: 0
                }),
                relayer: address(0)
            })
        );

        assert(feeToken.balanceOf(address(this)) == 1_000 * 1e18);
    }

    function testCanReceiveAssetsWithCall() public {
        assert(feeToken.balanceOf(address(this)) == 0);

        address calldataTarget = address(miniStaking);
        bytes memory stakeCalldata = abi.encodeWithSignature("recordStake(address)", address(this));

        Call[] memory calls = new Call[](1);
        calls[0] = Call({to: calldataTarget, data: stakeCalldata, value: 0});

        BodyWithCall memory body = BodyWithCall({
            assetId: keccak256("USD.h"),
            to: addressToBytes32(address(miniStaking)),
            redeem: false,
            amount: 1_000 * 1e18,
            from: addressToBytes32(address(this)),
            data: abi.encode(calls)
        });

        vm.prank(address(host));
        gateway.onAccept(
            IncomingPostRequest({
                request: PostRequest({
                    to: abi.encodePacked(address(0)),
                    from: abi.encodePacked(address(gateway)),
                    dest: new bytes(0),
                    body: bytes.concat(hex"00", abi.encode(body)),
                    nonce: 0,
                    source: new bytes(0),
                    timeoutTimestamp: 0
                }),
                relayer: address(0)
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

        bytes memory stakeCalldata = abi.encodeWithSignature("recordStake(address)", address(this));
        BodyWithCall memory bodyWithCall = BodyWithCall({
            assetId: keccak256("USD.h"),
            to: addressToBytes32(address(miniStaking)),
            redeem: false,
            amount: 1_000 * 1e18,
            from: addressToBytes32(address(this)),
            data: stakeCalldata
        });
        vm.prank(address(host));
        gateway.onPostRequestTimeout(
            PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(gateway)),
                dest: new bytes(0),
                body: bytes.concat(hex"00", abi.encode(bodyWithCall)),
                nonce: 0,
                source: new bytes(0),
                timeoutTimestamp: 0
            })
        );
    }

    function testAddAssetOnAccept() public {
        AssetMetadata memory asset1 = AssetMetadata({
            erc20: address(mockUSDC),
            erc6160: address(feeToken),
            name: "Hyperbridge USD",
            symbol: "USD",
            beneficiary: address(0),
            initialSupply: 0
        });

        bytes memory hyperbridge = StateMachine.kusama(2000);

        vm.prank(address(host));
        gateway.onAccept(
            IncomingPostRequest({
                request: PostRequest({
                    to: abi.encodePacked(address(0)),
                    from: abi.encodePacked(address(gateway)),
                    dest: new bytes(0),
                    body: bytes.concat(hex"02", abi.encode(asset1)),
                    nonce: 0,
                    source: hyperbridge,
                    timeoutTimestamp: 0
                }),
                relayer: address(0)
            })
        );

        bytes32 key = keccak256("USD");
        address erc6160Asset = gateway.erc6160(key);
        address erc20Asset = gateway.erc20(key);

        assert(erc6160Asset == address(feeToken));
        assert(erc20Asset == address(mockUSDC));
        assert(keccak256(bytes(ERC6160Ext20(erc6160Asset).symbol())) == keccak256(bytes(string("USD.h")))); // should add suffix

        AssetMetadata memory asset2 = AssetMetadata({
            erc20: address(0),
            erc6160: address(0),
            name: "Hyperbridge USD",
            symbol: "USDH",
            beneficiary: address(0),
            initialSupply: 0
        });

        vm.prank(address(host));
        gateway.onAccept(
            IncomingPostRequest({
                request: PostRequest({
                    to: abi.encodePacked(address(0)),
                    from: abi.encodePacked(address(gateway)),
                    dest: new bytes(0),
                    body: bytes.concat(hex"02", abi.encode(asset2)),
                    nonce: 0,
                    source: hyperbridge,
                    timeoutTimestamp: 0
                }),
                relayer: address(0)
            })
        );

        address usdh = gateway.erc6160(keccak256("USDH"));
        assert(keccak256(bytes(ERC6160Ext20(usdh).symbol())) == keccak256(bytes(string("USDH")))); // no suffix
    }

    function testToRevertOnAddAssetOnAcceptForUnauthorizedRequest() public {
        AssetMetadata memory asset = AssetMetadata({
            erc20: address(mockUSDC),
            erc6160: address(feeToken),
            name: "Hyperbridge USD",
            symbol: "USD.h",
            beneficiary: address(0),
            initialSupply: 0
        });

        vm.prank(address(host));

        vm.expectRevert(TokenGateway.UnauthorizedAction.selector);

        gateway.onAccept(
            IncomingPostRequest({
                request: PostRequest({
                    to: abi.encodePacked(address(0)),
                    from: abi.encodePacked(address(gateway)),
                    dest: new bytes(0),
                    body: bytes.concat(hex"02", abi.encode(asset)),
                    nonce: 0,
                    source: new bytes(0),
                    timeoutTimestamp: 0
                }),
                relayer: address(0)
            })
        );
    }

    function testRemoveAssetOnAccept() public {
        bytes32[] memory assets = new bytes32[](1);
        assets[0] = keccak256(bytes("USD.h"));

        bytes memory hyperbridge = StateMachine.kusama(2000);

        vm.prank(address(host));
        gateway.onAccept(
            IncomingPostRequest({
                request: PostRequest({
                    to: abi.encodePacked(address(0)),
                    from: abi.encodePacked(address(gateway)),
                    dest: new bytes(0),
                    body: bytes.concat(hex"03", abi.encode(DeregsiterAsset({assetIds: assets}))),
                    nonce: 0,
                    source: hyperbridge,
                    timeoutTimestamp: 0
                }),
                relayer: address(0)
            })
        );

        bytes32 key = keccak256("USD.h");

        address erc6160Asset = gateway.erc6160(key);
        address erc20Asset = gateway.erc20(key);

        assert(erc6160Asset == address(0));
        assert(erc20Asset == address(0));
    }

    function testOnlyHostCanCallOnAccept() public {
        Body memory body = Body({
            assetId: keccak256("USD.h"),
            to: addressToBytes32(address(this)),
            redeem: false,
            amount: 1_000 * 1e18,
            from: addressToBytes32(address(this))
        });
        vm.expectRevert(TokenGateway.UnauthorizedAction.selector);
        gateway.onAccept(
            IncomingPostRequest({
                request: PostRequest({
                    to: abi.encodePacked(address(0)),
                    from: abi.encodePacked(address(gateway)),
                    dest: new bytes(0),
                    body: bytes.concat(hex"00", abi.encode(body)),
                    nonce: 0,
                    source: new bytes(0),
                    timeoutTimestamp: 0
                }),
                relayer: address(0)
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
        vm.expectRevert(TokenGateway.UnauthorizedAction.selector);
        gateway.onAccept(
            IncomingPostRequest({
                request: PostRequest({
                    to: abi.encodePacked(address(0)),
                    // not from gateway
                    from: abi.encodePacked(address(this)),
                    dest: new bytes(0),
                    body: bytes.concat(hex"00", abi.encode(body)),
                    nonce: 0,
                    source: new bytes(0),
                    timeoutTimestamp: 0
                }),
                relayer: address(0)
            })
        );
    }

    function testCanModifyProtocolParams() public {
        TokenGatewayParams memory params = gateway.params();

        params.dispatcher = msg.sender;

        vm.prank(address(host));

        gateway.onAccept(
            IncomingPostRequest({
                request: PostRequest({
                    to: abi.encodePacked(address(0)),
                    from: abi.encodePacked(address(gateway)),
                    dest: new bytes(0),
                    body: bytes.concat(hex"01", abi.encode(params)),
                    nonce: 0,
                    source: StateMachine.kusama(2000),
                    timeoutTimestamp: 0
                }),
                relayer: address(0)
            })
        );

        assert(gateway.params().dispatcher == msg.sender);
    }

    function testCanChangeAssetOwner() public {
        // set gateway as the admin
        feeToken.changeAdmin(address(gateway));

        ChangeAssetAdmin memory changeAsset = ChangeAssetAdmin({
            assetId: keccak256(bytes(feeToken.symbol())),
            newAdmin: address(this)
        });

        vm.prank(address(host));
        gateway.onAccept(
            IncomingPostRequest({
                request: PostRequest({
                    to: abi.encodePacked(address(0)),
                    from: abi.encodePacked(address(gateway)),
                    dest: new bytes(0),
                    body: bytes.concat(hex"04", abi.encode(changeAsset)),
                    nonce: 0,
                    source: StateMachine.kusama(2000),
                    timeoutTimestamp: 0
                }),
                relayer: address(0)
            })
        );

        // we're the new owner, so we can change the owner as well
        feeToken.changeAdmin(msg.sender);
        vm.expectRevert(NotRoleAdmin.selector);
        feeToken.changeAdmin(msg.sender);
    }

    function testCanAddNewContractInstance() public {
        // set gateway as the admin
        feeToken.changeAdmin(address(gateway));

        bytes memory chain = bytes("MNTL");
        ContractInstance memory instance = ContractInstance({chain: chain, moduleId: address(this)});

        bytes memory hyperbridge = host.hyperbridge();
        vm.prank(address(host));
        gateway.onAccept(
            IncomingPostRequest({
                request: PostRequest({
                    to: abi.encodePacked(address(0)),
                    from: abi.encodePacked(address(gateway)),
                    dest: new bytes(0),
                    body: bytes.concat(hex"05", abi.encode(instance)),
                    nonce: 0,
                    source: hyperbridge,
                    timeoutTimestamp: 0
                }),
                relayer: address(0)
            })
        );

        // can now receive assets from new instance

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
            IncomingPostRequest({
                request: PostRequest({
                    to: abi.encodePacked(address(0)),
                    from: abi.encodePacked(address(this)),
                    dest: new bytes(0),
                    body: bytes.concat(hex"00", abi.encode(body)),
                    nonce: 0,
                    source: chain,
                    timeoutTimestamp: 0
                }),
                relayer: address(0)
            })
        );

        assert(feeToken.balanceOf(address(this)) == 1_000 * 1e18);
    }
}

function addressToBytes32(address _address) pure returns (bytes32) {
    return bytes32(uint256(uint160(_address)));
}

function bytes32ToAddress(bytes32 _bytes) pure returns (address) {
    return address(uint160(uint256(_bytes)));
}
