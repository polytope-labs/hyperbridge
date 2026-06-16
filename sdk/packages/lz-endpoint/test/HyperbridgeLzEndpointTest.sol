// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

import "forge-std/Test.sol";

import {HyperbridgeLzEndpoint} from "../contracts/HyperbridgeLzEndpoint.sol";

import {OFT} from "@layerzerolabs/oft-evm/contracts/OFT.sol";
import {SendParam} from "@layerzerolabs/oft-evm/contracts/interfaces/IOFT.sol";
import {
    MessagingParams,
    MessagingReceipt,
    MessagingFee,
    Origin
} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/ILayerZeroEndpointV2.sol";

import {PostRequest} from "@hyperbridge/core/libraries/Message.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";
import {IDispatcher} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

/// @dev Minimal EvmHost interface
interface IEvmHost {
    function feeToken() external view returns (address);
    function admin() external view returns (address);
    function setFrozenState(uint8 newState) external;
}

/// @dev Concrete OFT for testing
contract TestOFT is OFT {
    constructor(
        address _endpoint,
        address _delegate
    ) OFT("Test OFT", "tOFT", _endpoint, _delegate) Ownable(_delegate) {
        _mint(_delegate, 1_000_000 ether);
    }
}

contract HyperbridgeLzEndpointTest is Test {
    // Deployed Hyperbridge EvmHost on Ethereum mainnet
    address constant MAINNET_HOST = 0x792A6236AF69787C40cF76b69B4c8c7B28c4cA20;

    HyperbridgeLzEndpoint internal srcEndpoint;
    HyperbridgeLzEndpoint internal dstEndpoint;
    TestOFT internal srcOft;
    TestOFT internal dstOft;

    uint32 internal constant SRC_EID = 30101; // Ethereum
    uint32 internal constant DST_EID = 30110; // Arbitrum

    bytes internal srcStateMachine;
    bytes internal dstStateMachine;

    address internal alice;
    address internal bob = address(0xB0B);
    address internal feeToken;

    uint256 internal mainnetFork;

    function setUp() public {
        string memory rpcUrl = vm.envString("MAINNET_FORK_URL");
        mainnetFork = vm.createFork(rpcUrl);
        vm.selectFork(mainnetFork);

        srcStateMachine = StateMachine.evm(1);       // Ethereum
        dstStateMachine = StateMachine.evm(42161);    // Arbitrum

        // Read feeToken from the live host and unfreeze it
        IEvmHost hostContract = IEvmHost(MAINNET_HOST);
        feeToken = hostContract.feeToken();

        // Unfreeze the host (it may be frozen on mainnet)
        address admin = hostContract.admin();
        vm.prank(admin);
        hostContract.setFrozenState(0); // FrozenStatus.None

        // Use a mainnet whale as alice (has ETH + DAI)
        alice = address(0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045);

        // Deploy source endpoint using the real mainnet host
        srcEndpoint = new HyperbridgeLzEndpoint(address(this));
        srcEndpoint.setHost(MAINNET_HOST, SRC_EID);
        srcEndpoint.setEidMapping(DST_EID, dstStateMachine);
        srcEndpoint.setEidMapping(SRC_EID, srcStateMachine);

        // Deploy destination endpoint (also on same fork for testing)
        dstEndpoint = new HyperbridgeLzEndpoint(address(this));
        dstEndpoint.setHost(MAINNET_HOST, DST_EID);
        dstEndpoint.setEidMapping(SRC_EID, srcStateMachine);
        dstEndpoint.setEidMapping(DST_EID, dstStateMachine);

        // Deploy OFTs pointing to our adapters
        srcOft = new TestOFT(address(srcEndpoint), address(this));
        dstOft = new TestOFT(address(dstEndpoint), address(this));

        // Configure peers
        srcOft.setPeer(DST_EID, bytes32(uint256(uint160(address(dstOft)))));
        dstOft.setPeer(SRC_EID, bytes32(uint256(uint160(address(srcOft)))));

        // Fund alice with OFT tokens and ETH
        require(srcOft.transfer(alice, 10_000 ether), "transfer failed");
        vm.deal(alice, 100 ether);

        // Alice approves feeToken to the endpoint (for lzToken payment path)
        vm.startPrank(alice);
        IERC20(feeToken).approve(address(srcEndpoint), type(uint256).max);
        vm.stopPrank();
    }

    // ==================== Send Tests ====================

    function testSendBurnsAndDispatches() public {
        uint256 balanceBefore = srcOft.balanceOf(alice);
        uint256 sendAmount = 100 ether;

        SendParam memory sendParam = SendParam({
            dstEid: DST_EID,
            to: bytes32(uint256(uint160(bob))),
            amountLD: sendAmount,
            minAmountLD: sendAmount,
            extraOptions: "",
            composeMsg: "",
            oftCmd: ""
        });

        // Quote with native payment
        MessagingFee memory fee = srcOft.quoteSend(sendParam, false);

        vm.prank(alice);
        srcOft.send{value: fee.nativeFee}(sendParam, fee, alice);

        // OFT burned tokens from alice
        assertEq(srcOft.balanceOf(alice), balanceBefore - sendAmount);
    }

    function testSendWithFeeToken() public {
        uint256 balanceBefore = srcOft.balanceOf(alice);
        uint256 sendAmount = 100 ether;

        SendParam memory sendParam = SendParam({
            dstEid: DST_EID,
            to: bytes32(uint256(uint160(bob))),
            amountLD: sendAmount,
            minAmountLD: sendAmount,
            extraOptions: "",
            composeMsg: "",
            oftCmd: ""
        });

        // Quote with feeToken (lzToken) payment
        MessagingFee memory fee = srcOft.quoteSend(sendParam, true);
        assertTrue(fee.lzTokenFee > 0, "lzTokenFee should be non-zero");
        assertEq(fee.nativeFee, 0, "nativeFee should be zero for lzToken payment");

        // Give alice some feeToken (DAI on mainnet)
        deal(feeToken, alice, fee.lzTokenFee * 2);

        vm.startPrank(alice);
        IERC20(feeToken).approve(address(srcEndpoint), type(uint256).max);
        IERC20(feeToken).approve(address(srcOft), type(uint256).max);
        srcOft.send(sendParam, fee, alice);
        vm.stopPrank();

        // OFT burned tokens
        assertEq(srcOft.balanceOf(alice), balanceBefore - sendAmount);

        // Fee token was spent
        assertTrue(IERC20(feeToken).balanceOf(alice) < fee.lzTokenFee * 2, "feeToken should have been spent");
    }

    // ==================== Receive Tests ====================

    function testReceiveMintsTokens() public {
        uint256 sendAmount = 100 ether;

        // Build the ISMP body as srcEndpoint would encode it
        uint64 nonce = 1;
        bytes32 sender = bytes32(uint256(uint160(address(srcOft))));
        bytes32 receiver = bytes32(uint256(uint160(address(dstOft))));
        bytes32 guid = keccak256(abi.encodePacked(nonce, SRC_EID, sender, DST_EID, receiver));

        // OFT message: recipient (bytes32) + amountSD (uint64)
        // shared decimals = 6, local decimals = 18, conversion = 1e12
        // forge-lint: disable-next-line(unsafe-typecast)
        uint64 amountSD = uint64(sendAmount / 1e12);
        bytes memory oftMessage = abi.encodePacked(bytes32(uint256(uint160(bob))), amountSD);

        bytes memory body = abi.encode(guid, SRC_EID, sender, nonce, receiver, oftMessage);

        PostRequest memory request = PostRequest({
            source: srcStateMachine,
            dest: dstStateMachine,
            nonce: 0,
            from: abi.encodePacked(address(dstEndpoint)),
            to: abi.encodePacked(address(dstEndpoint)),
            timeoutTimestamp: 0,
            body: body
        });

        uint256 bobBefore = dstOft.balanceOf(bob);

        // Simulate ISMP host calling onAccept
        vm.prank(MAINNET_HOST);
        dstEndpoint.onAccept(IncomingPostRequest({request: request, relayer: address(0)}));

        assertEq(dstOft.balanceOf(bob), bobBefore + sendAmount);
    }

    // ==================== Nonce Tests ====================

    function testNonceTracking() public {
        uint256 sendAmount = 10 ether;

        SendParam memory sendParam = SendParam({
            dstEid: DST_EID,
            to: bytes32(uint256(uint160(bob))),
            amountLD: sendAmount,
            minAmountLD: sendAmount,
            extraOptions: "",
            composeMsg: "",
            oftCmd: ""
        });

        vm.startPrank(alice);
        MessagingFee memory fee1 = srcOft.quoteSend(sendParam, false);
        (MessagingReceipt memory r1,) = srcOft.send{value: fee1.nativeFee}(sendParam, fee1, alice);
        MessagingFee memory fee2 = srcOft.quoteSend(sendParam, false);
        (MessagingReceipt memory r2,) = srcOft.send{value: fee2.nativeFee}(sendParam, fee2, alice);
        vm.stopPrank();

        assertEq(r1.nonce, 1);
        assertEq(r2.nonce, 2);
        assertTrue(r1.guid != r2.guid);
    }

    // ==================== Reject Tests ====================

    function testRejectUnknownEid() public {
        SendParam memory sendParam = SendParam({
            dstEid: 99999,
            to: bytes32(uint256(uint160(bob))),
            amountLD: 1 ether,
            minAmountLD: 1 ether,
            extraOptions: "",
            composeMsg: "",
            oftCmd: ""
        });

        vm.prank(alice);
        vm.expectRevert();
        srcOft.send(sendParam, MessagingFee(0, 0), alice);
    }

    function testRejectInvalidNonce() public {
        bytes32 sender = bytes32(uint256(uint160(address(srcOft))));
        bytes32 receiver = bytes32(uint256(uint160(address(dstOft))));
        uint64 badNonce = 2; // skipped nonce 1

        bytes memory body = abi.encode(
            keccak256(abi.encodePacked(badNonce, SRC_EID, sender, DST_EID, receiver)),
            SRC_EID, sender, badNonce, receiver, ""
        );

        PostRequest memory request = PostRequest({
            source: srcStateMachine,
            dest: dstStateMachine,
            nonce: 0,
            from: abi.encodePacked(address(dstEndpoint)),
            to: abi.encodePacked(address(dstEndpoint)),
            timeoutTimestamp: 0,
            body: body
        });

        vm.prank(MAINNET_HOST);
        vm.expectRevert();
        dstEndpoint.onAccept(IncomingPostRequest({request: request, relayer: address(0)}));
    }

    function testRejectUnknownSource() public {
        bytes memory body = abi.encode(bytes32(0), SRC_EID, bytes32(0), uint64(1), bytes32(0), "");

        PostRequest memory request = PostRequest({
            source: srcStateMachine,
            dest: dstStateMachine,
            nonce: 0,
            from: abi.encodePacked(address(0xDEAD)), // wrong source
            to: abi.encodePacked(address(dstEndpoint)),
            timeoutTimestamp: 0,
            body: body
        });

        vm.prank(MAINNET_HOST);
        vm.expectRevert(HyperbridgeLzEndpoint.UnknownSource.selector);
        dstEndpoint.onAccept(IncomingPostRequest({request: request, relayer: address(0)}));
    }

    // ==================== Pause Tests ====================

    function testPauseSend() public {
        srcEndpoint.pause();

        SendParam memory sendParam = SendParam({
            dstEid: DST_EID,
            to: bytes32(uint256(uint160(bob))),
            amountLD: 1 ether,
            minAmountLD: 1 ether,
            extraOptions: "",
            composeMsg: "",
            oftCmd: ""
        });

        vm.prank(alice);
        vm.expectRevert();
        srcOft.send(sendParam, MessagingFee(0, 0), alice);
    }

    function testPauseReceive() public {
        dstEndpoint.pause();

        bytes memory body = abi.encode(bytes32(0), SRC_EID, bytes32(0), uint64(1), bytes32(0), "");
        PostRequest memory request = PostRequest({
            source: srcStateMachine,
            dest: dstStateMachine,
            nonce: 0,
            from: abi.encodePacked(address(dstEndpoint)),
            to: abi.encodePacked(address(dstEndpoint)),
            timeoutTimestamp: 0,
            body: body
        });

        vm.prank(MAINNET_HOST);
        vm.expectRevert();
        dstEndpoint.onAccept(IncomingPostRequest({request: request, relayer: address(0)}));
    }

    // ==================== Config Tests ====================

    function testEid() public view {
        assertEq(srcEndpoint.eid(), SRC_EID);
        assertEq(dstEndpoint.eid(), DST_EID);
    }

    function testIsSupportedEid() public view {
        assertTrue(srcEndpoint.isSupportedEid(DST_EID));
        assertFalse(srcEndpoint.isSupportedEid(99999));
    }

    function testLzTokenReturnsFeeToken() public view {
        assertEq(srcEndpoint.lzToken(), feeToken);
    }

    // ==================== E2E: Send → Receive ====================

    function testEndToEndSendAndReceive() public {
        uint256 sendAmount = 500 ether;
        uint256 aliceBefore = srcOft.balanceOf(alice);
        uint256 bobBefore = dstOft.balanceOf(bob);

        // Step 1: Alice sends via srcOft
        SendParam memory sendParam = SendParam({
            dstEid: DST_EID,
            to: bytes32(uint256(uint160(bob))),
            amountLD: sendAmount,
            minAmountLD: sendAmount,
            extraOptions: "",
            composeMsg: "",
            oftCmd: ""
        });

        MessagingFee memory fee = srcOft.quoteSend(sendParam, false);

        vm.prank(alice);
        srcOft.send{value: fee.nativeFee}(sendParam, fee, alice);

        assertEq(srcOft.balanceOf(alice), aliceBefore - sendAmount);

        // Step 2: Construct the ISMP body matching what srcEndpoint dispatched
        // Re-derive the message that was dispatched
        bytes32 sender = bytes32(uint256(uint160(address(srcOft))));
        bytes32 receiver = bytes32(uint256(uint160(address(dstOft))));
        uint64 nonce = 1;
        bytes32 guid = keccak256(abi.encodePacked(nonce, SRC_EID, sender, DST_EID, receiver));

        // forge-lint: disable-next-line(unsafe-typecast)
        uint64 amountSD = uint64(sendAmount / 1e12);
        bytes memory oftMessage = abi.encodePacked(bytes32(uint256(uint160(bob))), amountSD);

        bytes memory body = abi.encode(guid, SRC_EID, sender, nonce, receiver, oftMessage);

        PostRequest memory request = PostRequest({
            source: srcStateMachine,
            dest: dstStateMachine,
            nonce: 0,
            from: abi.encodePacked(address(dstEndpoint)),
            to: abi.encodePacked(address(dstEndpoint)),
            timeoutTimestamp: 0,
            body: body
        });

        // Step 3: Deliver on destination via ISMP host
        vm.prank(MAINNET_HOST);
        dstEndpoint.onAccept(IncomingPostRequest({request: request, relayer: address(0)}));

        assertEq(dstOft.balanceOf(bob), bobBefore + sendAmount);
    }
}
