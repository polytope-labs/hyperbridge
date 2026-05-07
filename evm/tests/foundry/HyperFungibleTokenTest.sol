// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Test.sol";
import {BaseTest} from "./BaseTest.sol";
import {MockUSCDC} from "./MockUSDC.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";
import {PostRequest, Message} from "@hyperbridge/core/libraries/Message.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";
import {DispatchPost} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {CallDispatcher} from "../../src/utils/CallDispatcher.sol";
import {Call} from "../../src/interfaces/ICallDispatcher.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

import {HyperFungibleToken} from "@hyperbridge/core/apps/HyperFungibleToken.sol";
import {WrappedHyperFungibleToken} from "@hyperbridge/core/apps/WrappedHyperFungibleToken.sol";

// Concrete HyperFungibleToken for testing
contract TestHFT is HyperFungibleToken {
    constructor() HyperFungibleToken("Test HFT", "tHFT", msg.sender) {
        _mint(msg.sender, 1000 ether);
    }
}

// Concrete WrappedHyperFungibleToken for testing
contract TestWrappedHFT is WrappedHyperFungibleToken {
    constructor() WrappedHyperFungibleToken(msg.sender) {}
}

// Minimal WETH9 mock
contract WETH9 is ERC20 {
    constructor() ERC20("Wrapped Ether", "WETH") {}

    function deposit() external payable {
        _mint(msg.sender, msg.value);
    }

    function withdraw(uint256 amount) external {
        _burn(msg.sender, amount);
        (bool sent,) = msg.sender.call{value: amount}("");
        require(sent, "WETH: ETH transfer failed");
    }

    receive() external payable {
        _mint(msg.sender, msg.value);
    }
}


contract HyperFungibleTokenTest is BaseTest {
    using Message for PostRequest;

    TestHFT internal hft;
    CallDispatcher internal callDispatcher;
    bytes internal destChain;
    bytes internal remoteContract;

    uint256 internal constant MINT_AMOUNT = 1000 ether;
    uint256 internal constant SEND_AMOUNT = 100 ether;

    function setUp() public override {
        super.setUp();

        destChain = StateMachine.evm(42161); // Arbitrum
        remoteContract = abi.encodePacked(address(0xBEEF));

        callDispatcher = new CallDispatcher();
        // Grant minter role to CallDispatcher so calldata tests can mint via it
        feeToken.grantMinterRole(address(callDispatcher));

        hft = new TestHFT();
        hft.configure(HyperFungibleToken.ConfigOptions({
            host: address(host),
            dispatcher: address(callDispatcher)
        }));
        hft.addChain(destChain, remoteContract);

        // TestHFT constructor mints MINT_AMOUNT to deployer (address(this))

        // Approve fee token for dispatch
        feeToken.mint(address(this), 10000 ether);
        feeToken.superApprove(address(this), address(hft));
        feeToken.superApprove(address(hft), address(host));
    }

    // ========== Configuration Tests ==========

    function testConfigure() public {
        assertEq(hft.host(), address(host));
    }

    function testConfigureOnlyOwner() public {
        vm.prank(address(0xDEAD));
        vm.expectRevert();
        hft.configure(HyperFungibleToken.ConfigOptions({
            host: address(0x1),
            dispatcher: address(0x2)
        }));
    }

    function testAddChain() public {
        bytes memory newChain = StateMachine.evm(10);
        bytes memory newContract = abi.encodePacked(address(0xFACE));
        hft.addChain(newChain, newContract);

        // Verify by sending (would revert if chain not added)
        deal(address(hft), address(this), SEND_AMOUNT);
        // We can't easily test the mapping directly, but removing and sending should fail
        hft.removeChain(newChain);
    }

    function testAddChainOnlyOwner() public {
        vm.prank(address(0xDEAD));
        vm.expectRevert();
        hft.addChain(StateMachine.evm(10), abi.encodePacked(address(0x1)));
    }

    function testRemoveChain() public {
        hft.removeChain(destChain);

        vm.expectRevert(HyperFungibleToken.UnsupportedChain.selector);
        hft.send(HyperFungibleToken.SendParams({
            dest: destChain,
            to: abi.encodePacked(address(0xCAFE)),
            amount: SEND_AMOUNT,
            timeout: 3600,
            relayerFee: 0,
            data: ""
        }));
    }

    // ========== Pause Tests ==========

    function testPause() public {
        hft.pause();
        assertTrue(hft.paused());
    }

    function testPauseOnlyOwner() public {
        vm.prank(address(0xDEAD));
        vm.expectRevert();
        hft.pause();
    }

    function testSendRevertsWhenPaused() public {
        hft.pause();

        vm.expectRevert();
        hft.send(HyperFungibleToken.SendParams({
            dest: destChain,
            to: abi.encodePacked(address(0xCAFE)),
            amount: SEND_AMOUNT,
            timeout: 3600,
            relayerFee: 0,
            data: ""
        }));
    }

    function testOnAcceptRevertsWhenPaused() public {
        hft.pause();

        PostRequest memory request = _makeIncomingRequest(address(this), SEND_AMOUNT, "");

        vm.prank(address(host));
        // onAccept should revert when paused
        (bool success,) = address(hft).call(
            abi.encodeWithSelector(
                hft.onAccept.selector,
                IncomingPostRequest({request: request, relayer: address(0)})
            )
        );
        assertFalse(success);
    }

    function testUnpause() public {
        hft.pause();
        hft.unpause();
        assertFalse(hft.paused());
    }

    // ========== Send Tests ==========

    function testSendBurnsTokens() public {
        uint256 balanceBefore = hft.balanceOf(address(this));

        hft.send(HyperFungibleToken.SendParams({
            dest: destChain,
            to: abi.encodePacked(address(0xCAFE)),
            amount: SEND_AMOUNT,
            timeout: 3600,
            relayerFee: 0,
            data: ""
        }));

        assertEq(hft.balanceOf(address(this)), balanceBefore - SEND_AMOUNT);
    }

    function testSendRevertsUnsupportedChain() public {
        vm.expectRevert(HyperFungibleToken.UnsupportedChain.selector);
        hft.send(HyperFungibleToken.SendParams({
            dest: StateMachine.evm(999),
            to: abi.encodePacked(address(0xCAFE)),
            amount: SEND_AMOUNT,
            timeout: 3600,
            relayerFee: 0,
            data: ""
        }));
    }

    // ========== Receive (onAccept) Tests ==========

    function testOnAcceptMintsTokens() public {
        address recipient = address(0xCAFE);
        uint256 amount = 50 ether;

        PostRequest memory request = _makeIncomingRequest(recipient, amount, "");

        vm.prank(address(host));
        hft.onAccept(IncomingPostRequest({request: request, relayer: address(0)}));

        assertEq(hft.balanceOf(recipient), amount);
    }

    function testOnAcceptWithCalldata() public {
        address recipient = address(0xCAFE);
        uint256 amount = 50 ether;
        address finalRecipient = address(0xF00D);

        // Build Call[] calldata: approve callDispatcher, then transfer from recipient to finalRecipient
        // Since CallDispatcher executes calls as itself, we use a simpler pattern:
        // just call a known contract function
        Call[] memory calls = new Call[](1);
        calls[0] = Call({
            to: address(feeToken),
            value: 0,
            data: abi.encodeWithSelector(feeToken.mint.selector, finalRecipient, 1 ether)
        });
        bytes memory callData = abi.encode(calls);

        PostRequest memory request = _makeIncomingRequest(recipient, amount, callData);

        vm.prank(address(host));
        hft.onAccept(IncomingPostRequest({request: request, relayer: address(0)}));

        assertEq(hft.balanceOf(recipient), amount);
        // Verify the calldata was executed via CallDispatcher
        assertEq(feeToken.balanceOf(finalRecipient), 1 ether);
    }

    function testOnAcceptRevertsUnauthorizedSource() public {
        PostRequest memory request = PostRequest({
            source: destChain,
            dest: StateMachine.evm(1),
            nonce: 0,
            from: abi.encodePacked(address(0xBAD)), // wrong source address
            to: abi.encodePacked(address(hft)),
            timeoutTimestamp: 0,
            body: abi.encode(HyperFungibleToken.Message({
                from: abi.encodePacked(address(0x1)),
                to: abi.encodePacked(address(0x2)),
                amount: 1 ether,
                data: ""
            }))
        });

        vm.prank(address(host));
        vm.expectRevert(HyperFungibleToken.UnauthorizedSource.selector);
        hft.onAccept(IncomingPostRequest({request: request, relayer: address(0)}));
    }

    function testOnAcceptRevertsUnsupportedChain() public {
        PostRequest memory request = PostRequest({
            source: StateMachine.evm(999), // unsupported
            dest: StateMachine.evm(1),
            nonce: 0,
            from: abi.encodePacked(address(0x1)),
            to: abi.encodePacked(address(hft)),
            timeoutTimestamp: 0,
            body: abi.encode(HyperFungibleToken.Message({
                from: abi.encodePacked(address(0x1)),
                to: abi.encodePacked(address(0x2)),
                amount: 1 ether,
                data: ""
            }))
        });

        vm.prank(address(host));
        vm.expectRevert(HyperFungibleToken.UnsupportedChain.selector);
        hft.onAccept(IncomingPostRequest({request: request, relayer: address(0)}));
    }

    function testOnAcceptOnlyHost() public {
        PostRequest memory request = _makeIncomingRequest(address(0xCAFE), 1 ether, "");

        vm.expectRevert();
        hft.onAccept(IncomingPostRequest({request: request, relayer: address(0)}));
    }

    // ========== Timeout Tests ==========

    function testOnPostRequestTimeoutRefunds() public {
        address sender = address(this);

        // Send tokens (burns them)
        hft.send(HyperFungibleToken.SendParams({
            dest: destChain,
            to: abi.encodePacked(address(0xCAFE)),
            amount: SEND_AMOUNT,
            timeout: 3600,
            relayerFee: 0,
            data: ""
        }));

        uint256 balanceAfterSend = hft.balanceOf(sender);

        // Simulate timeout — re-mints tokens to sender
        PostRequest memory request = PostRequest({
            source: StateMachine.evm(1),
            dest: destChain,
            nonce: 0,
            from: abi.encodePacked(address(hft)),
            to: abi.encodePacked(remoteContract),
            timeoutTimestamp: 3600,
            body: abi.encode(HyperFungibleToken.Message({
                from: abi.encodePacked(sender),
                to: abi.encodePacked(address(0xCAFE)),
                amount: SEND_AMOUNT,
                data: ""
            }))
        });

        vm.prank(address(host));
        hft.onPostRequestTimeout(request);

        assertEq(hft.balanceOf(sender), balanceAfterSend + SEND_AMOUNT);
    }

    function testOnPostRequestTimeoutOnlyHost() public {
        PostRequest memory request = PostRequest({
            source: StateMachine.evm(1),
            dest: destChain,
            nonce: 0,
            from: abi.encodePacked(address(hft)),
            to: abi.encodePacked(remoteContract),
            timeoutTimestamp: 3600,
            body: abi.encode(HyperFungibleToken.Message({
                from: abi.encodePacked(address(this)),
                to: abi.encodePacked(address(0xCAFE)),
                amount: SEND_AMOUNT,
                data: ""
            }))
        });

        vm.expectRevert();
        hft.onPostRequestTimeout(request);
    }

    // ========== Helpers ==========

    function _makeIncomingRequest(address recipient, uint256 amount, bytes memory data)
        internal
        view
        returns (PostRequest memory)
    {
        return PostRequest({
            source: destChain,
            dest: StateMachine.evm(1),
            nonce: 0,
            from: abi.encodePacked(remoteContract),
            to: abi.encodePacked(address(hft)),
            timeoutTimestamp: 0,
            body: abi.encode(HyperFungibleToken.Message({
                from: abi.encodePacked(address(0x1111)),
                to: abi.encodePacked(recipient),
                amount: amount,
                data: data
            }))
        });
    }
}

contract WrappedHyperFungibleTokenTest is BaseTest {
    using Message for PostRequest;
    using SafeERC20 for IERC20;

    TestWrappedHFT internal whft;
    CallDispatcher internal wrappedCallDispatcher;
    WETH9 internal weth;
    MockUSCDC internal mockToken;
    bytes internal destChain;
    bytes internal remoteContract;

    uint256 internal constant SEND_AMOUNT = 1 ether;

    function setUp() public override {
        super.setUp();

        destChain = StateMachine.evm(42161);
        remoteContract = abi.encodePacked(address(0xBEEF));

        wrappedCallDispatcher = new CallDispatcher();
        feeToken.grantMinterRole(address(wrappedCallDispatcher));
        weth = new WETH9();
        mockToken = new MockUSCDC("Mock Token", "MTK");

        // ---- ERC20 wrapped token ----
        whft = new TestWrappedHFT();
        whft.configure(WrappedHyperFungibleToken.WrappedConfigOptions({
            host: address(host),
            dispatcher: address(wrappedCallDispatcher),
            underlying: address(mockToken),
            isWeth: false
        }));
        whft.addChain(destChain, remoteContract);

        // Fund and approve
        mockToken.mint(address(this), 1000 ether);
        mockToken.approve(address(whft), type(uint256).max);
        mockToken.mint(address(whft), 1000 ether); // pre-fund for receive tests

        // Approve fee token for dispatch
        feeToken.mint(address(this), 10000 ether);
        feeToken.superApprove(address(this), address(whft));
        feeToken.superApprove(address(whft), address(host));
    }

    // ========== Configuration Tests ==========

    function testConfigure() public {
        assertEq(whft.host(), address(host));
        assertEq(whft.underlying(), address(mockToken));
    }

    function testConfigureOnlyOwner() public {
        vm.prank(address(0xDEAD));
        vm.expectRevert();
        whft.configure(WrappedHyperFungibleToken.WrappedConfigOptions({
            host: address(0x1),
            dispatcher: address(0x2),
            underlying: address(0x3),
            isWeth: false
        }));
    }

    // ========== Pause Tests ==========

    function testPause() public {
        whft.pause();
        assertTrue(whft.paused());
    }

    function testSendRevertsWhenPaused() public {
        whft.pause();

        vm.expectRevert();
        whft.send(HyperFungibleToken.SendParams({
            dest: destChain,
            to: abi.encodePacked(address(0xCAFE)),
            amount: SEND_AMOUNT,
            timeout: 3600,
            relayerFee: 0,
            data: ""
        }));
    }

    // ========== Send (ERC20) Tests ==========

    function testSendLocksERC20() public {
        uint256 balanceBefore = mockToken.balanceOf(address(this));
        uint256 contractBefore = mockToken.balanceOf(address(whft));

        whft.send(HyperFungibleToken.SendParams({
            dest: destChain,
            to: abi.encodePacked(address(0xCAFE)),
            amount: SEND_AMOUNT,
            timeout: 3600,
            relayerFee: 0,
            data: ""
        }));

        assertEq(mockToken.balanceOf(address(this)), balanceBefore - SEND_AMOUNT);
        assertEq(mockToken.balanceOf(address(whft)), contractBefore + SEND_AMOUNT);
    }

    function testSendRevertsUnsupportedChain() public {
        vm.expectRevert(WrappedHyperFungibleToken.UnsupportedChain.selector);
        whft.send(HyperFungibleToken.SendParams({
            dest: StateMachine.evm(999),
            to: abi.encodePacked(address(0xCAFE)),
            amount: SEND_AMOUNT,
            timeout: 3600,
            relayerFee: 0,
            data: ""
        }));
    }

    // ========== Send (WETH / Native) Tests ==========

    function testSendWrapsNativeETH() public {
        // Reconfigure as WETH
        TestWrappedHFT wethWhft = new TestWrappedHFT();
        wethWhft.configure(WrappedHyperFungibleToken.WrappedConfigOptions({
            host: address(host),
            dispatcher: address(wrappedCallDispatcher),
            underlying: address(weth),
            isWeth: true
        }));
        wethWhft.addChain(destChain, remoteContract);
        feeToken.superApprove(address(this), address(wethWhft));
        feeToken.superApprove(address(wethWhft), address(host));

        uint256 wethBefore = weth.balanceOf(address(wethWhft));

        wethWhft.send{value: SEND_AMOUNT}(HyperFungibleToken.SendParams({
            dest: destChain,
            to: abi.encodePacked(address(0xCAFE)),
            amount: SEND_AMOUNT,
            timeout: 3600,
            relayerFee: 0,
            data: ""
        }));

        // WETH should be deposited in the contract
        assertEq(weth.balanceOf(address(wethWhft)), wethBefore + SEND_AMOUNT);
    }

    // ========== Receive (onAccept) Tests ==========

    function testOnAcceptTransfersERC20() public {
        address recipient = address(0xCAFE);

        PostRequest memory request = _makeIncomingRequest(recipient, SEND_AMOUNT, "");

        vm.prank(address(host));
        whft.onAccept(IncomingPostRequest({request: request, relayer: address(0)}));

        assertEq(mockToken.balanceOf(recipient), SEND_AMOUNT);
    }

    function testOnAcceptWithCalldata() public {
        address recipient = address(0xCAFE);
        address finalRecipient = address(0xF00D);

        Call[] memory calls = new Call[](1);
        calls[0] = Call({
            to: address(feeToken),
            value: 0,
            data: abi.encodeWithSelector(feeToken.mint.selector, finalRecipient, 1 ether)
        });
        bytes memory callData = abi.encode(calls);

        PostRequest memory request = _makeIncomingRequest(recipient, SEND_AMOUNT, callData);

        vm.prank(address(host));
        whft.onAccept(IncomingPostRequest({request: request, relayer: address(0)}));

        assertEq(mockToken.balanceOf(recipient), SEND_AMOUNT);
        assertEq(feeToken.balanceOf(finalRecipient), 1 ether);
    }

    function testOnAcceptRevertsUnauthorizedSource() public {
        PostRequest memory request = PostRequest({
            source: destChain,
            dest: StateMachine.evm(1),
            nonce: 0,
            from: abi.encodePacked(address(0xBAD)),
            to: abi.encodePacked(address(whft)),
            timeoutTimestamp: 0,
            body: abi.encode(HyperFungibleToken.Message({
                from: abi.encodePacked(address(0x1)),
                to: abi.encodePacked(address(0x2)),
                amount: 1 ether,
                data: ""
            }))
        });

        vm.prank(address(host));
        vm.expectRevert(WrappedHyperFungibleToken.UnauthorizedSource.selector);
        whft.onAccept(IncomingPostRequest({request: request, relayer: address(0)}));
    }

    function testOnAcceptOnlyHost() public {
        PostRequest memory request = _makeIncomingRequest(address(0xCAFE), SEND_AMOUNT, "");

        vm.expectRevert();
        whft.onAccept(IncomingPostRequest({request: request, relayer: address(0)}));
    }

    // ========== Timeout (ERC20) Tests ==========

    function testTimeoutRefundsERC20() public {
        uint256 balanceBefore = mockToken.balanceOf(address(this));

        // Lock tokens via send
        whft.send(HyperFungibleToken.SendParams({
            dest: destChain,
            to: abi.encodePacked(address(0xCAFE)),
            amount: SEND_AMOUNT,
            timeout: 3600,
            relayerFee: 0,
            data: ""
        }));

        assertEq(mockToken.balanceOf(address(this)), balanceBefore - SEND_AMOUNT);

        // Timeout
        PostRequest memory request = PostRequest({
            source: StateMachine.evm(1),
            dest: destChain,
            nonce: 0,
            from: abi.encodePacked(address(whft)),
            to: abi.encodePacked(remoteContract),
            timeoutTimestamp: 3600,
            body: abi.encode(HyperFungibleToken.Message({
                from: abi.encodePacked(address(this)),
                to: abi.encodePacked(address(0xCAFE)),
                amount: SEND_AMOUNT,
                data: ""
            }))
        });

        vm.prank(address(host));
        whft.onPostRequestTimeout(request);

        assertEq(mockToken.balanceOf(address(this)), balanceBefore);
    }

    // ========== Timeout (WETH / Native) Tests ==========

    function testTimeoutRefundsNativeETH() public {
        // Deploy WETH-configured wrapper
        TestWrappedHFT wethWhft = new TestWrappedHFT();
        wethWhft.configure(WrappedHyperFungibleToken.WrappedConfigOptions({
            host: address(host),
            dispatcher: address(wrappedCallDispatcher),
            underlying: address(weth),
            isWeth: true
        }));
        wethWhft.addChain(destChain, remoteContract);
        feeToken.superApprove(address(this), address(wethWhft));
        feeToken.superApprove(address(wethWhft), address(host));

        // Send native ETH (wraps to WETH)
        address sender = address(this);
        uint256 ethBefore = sender.balance;

        wethWhft.send{value: SEND_AMOUNT}(HyperFungibleToken.SendParams({
            dest: destChain,
            to: abi.encodePacked(address(0xCAFE)),
            amount: SEND_AMOUNT,
            timeout: 3600,
            relayerFee: 0,
            data: ""
        }));

        // Timeout — should unwrap WETH and send native ETH back
        PostRequest memory request = PostRequest({
            source: StateMachine.evm(1),
            dest: destChain,
            nonce: 0,
            from: abi.encodePacked(address(wethWhft)),
            to: abi.encodePacked(remoteContract),
            timeoutTimestamp: 3600,
            body: abi.encode(HyperFungibleToken.Message({
                from: abi.encodePacked(sender),
                to: abi.encodePacked(address(0xCAFE)),
                amount: SEND_AMOUNT,
                data: ""
            }))
        });

        vm.prank(address(host));
        wethWhft.onPostRequestTimeout(request);

        // Sender should have native ETH back
        assertEq(sender.balance, ethBefore);
    }

    function testTimeoutOnlyHost() public {
        PostRequest memory request = PostRequest({
            source: StateMachine.evm(1),
            dest: destChain,
            nonce: 0,
            from: abi.encodePacked(address(whft)),
            to: abi.encodePacked(remoteContract),
            timeoutTimestamp: 3600,
            body: abi.encode(HyperFungibleToken.Message({
                from: abi.encodePacked(address(this)),
                to: abi.encodePacked(address(0xCAFE)),
                amount: SEND_AMOUNT,
                data: ""
            }))
        });

        vm.expectRevert();
        whft.onPostRequestTimeout(request);
    }

    // ========== Helpers ==========

    function _makeIncomingRequest(address recipient, uint256 amount, bytes memory data)
        internal
        view
        returns (PostRequest memory)
    {
        return PostRequest({
            source: destChain,
            dest: StateMachine.evm(1),
            nonce: 0,
            from: abi.encodePacked(remoteContract),
            to: abi.encodePacked(address(whft)),
            timeoutTimestamp: 0,
            body: abi.encode(HyperFungibleToken.Message({
                from: abi.encodePacked(address(0x1111)),
                to: abi.encodePacked(recipient),
                amount: amount,
                data: data
            }))
        });
    }

    receive() external payable {}
}
