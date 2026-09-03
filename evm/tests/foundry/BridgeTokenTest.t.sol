// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Test.sol";
import {BaseTest} from "./BaseTest.sol";

import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";
import {PostRequest, Message} from "@hyperbridge/core/libraries/Message.sol";
import {IncomingPostRequest, PostRequestTimeout} from "@hyperbridge/core/interfaces/IApp.sol";
import {HyperFungibleToken} from "@hyperbridge/core/apps/HyperFungibleToken.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

import {BridgeToken} from "../../src/apps/BridgeToken.sol";
import {CallDispatcher} from "../../src/utils/CallDispatcher.sol";

/// The behaviour inherited from `HyperFungibleToken` is covered by `HyperFungibleTokenTest`.
/// What is specific to `BridgeToken` is the metadata and the nexus peer baked into the
/// bytecode, and the relayer gate that fails closed, so these tests deploy it exactly as the
/// script does (relayer set, then configured) and check that it can already talk to nexus,
/// through that relayer, and nothing else.
contract BridgeTokenTest is BaseTest {
    using Message for PostRequest;

    BridgeToken internal bridge;
    CallDispatcher internal callDispatcher;
    /// The only account allowed to deliver to the token, as the deploy script sets it
    address internal relayer;

    /// nexus, the chain BRIDGE is native to
    bytes internal nexus;
    /// the 8 byte `pall_hft` PalletId of `pallet-hyper-fungible-token`
    bytes internal palletId;

    address internal constant RECIPIENT = address(0xCAFE);
    uint256 internal constant MINT_AMOUNT = 100 ether;

    /// A 32 byte substrate account, the shape of a beneficiary on nexus
    bytes internal constant NEXUS_ACCOUNT =
        hex"0000000000000000000000000000000000000000000000000000000000000b0b";

    function setUp() public override {
        super.setUp();

        nexus = StateMachine.polkadot(3367);
        palletId = abi.encodePacked(bytes8("pall_hft"));

        callDispatcher = new CallDispatcher();
        relayer = makeAddr("relayer");
        bridge = new BridgeToken(address(this));
        bridge.setRelayer(relayer);
        bridge.configure(
            HyperFungibleToken.ConfigOptions({host: address(host), dispatcher: address(callDispatcher)})
        );

        feeToken.mint(address(this), 10_000 ether);
        feeToken.superApprove(address(this), address(bridge));
        feeToken.superApprove(address(bridge), address(host));
    }

    // ---------- metadata ----------

    function testMetadataIsFixedInTheBytecode() public view {
        assertEq(bridge.name(), "Hyperbridge");
        assertEq(bridge.symbol(), "BRIDGE");
        // 18 here while BRIDGE is 12 decimals on nexus, so the pallet scales by 10^6 in
        // both directions and `register_token` must declare 18 for this contract.
        assertEq(bridge.decimals(), 18);
    }

    /// The supply is minted only against balances escrowed by the pallet, so a fresh
    /// deployment holds nothing.
    function testStartsWithNoSupply() public view {
        assertEq(bridge.totalSupply(), 0);
    }

    // ---------- constructor wiring ----------

    function testNexusIsRegisteredAtConstruction() public view {
        assertEq(bridge.nexus(), bytes("POLKADOT-3367"));
        assertEq(bridge.supportedChain(nexus), palletId);
        assertEq(bridge.NEXUS_MODULE_ID(), bytes8("pall_hft"));
    }

    // ---------- receive ----------

    /// The pallet escrows on nexus and this contract mints here, with no `addChain` call
    /// in between.
    function testOnAcceptFromNexusMints() public {
        vm.prank(address(host));
        bridge.onAccept(
            IncomingPostRequest({request: _fromNexus(palletId, RECIPIENT, MINT_AMOUNT), relayer: relayer})
        );

        assertEq(bridge.balanceOf(RECIPIENT), MINT_AMOUNT);
        assertEq(bridge.totalSupply(), MINT_AMOUNT);
    }

    /// Another pallet on nexus is not the peer, even though the chain is registered.
    function testOnAcceptRejectsAnotherPalletOnNexus() public {
        PostRequest memory request =
            _fromNexus(abi.encodePacked(bytes8("pall_tkg")), RECIPIENT, MINT_AMOUNT);

        vm.prank(address(host));
        vm.expectRevert(HyperFungibleToken.UnauthorizedSource.selector);
        bridge.onAccept(IncomingPostRequest({request: request, relayer: relayer}));
    }

    /// Nexus is the only chain the deployment knows until its owner registers more.
    function testOnAcceptRejectsAnotherParachain() public {
        PostRequest memory request = _fromNexus(palletId, RECIPIENT, MINT_AMOUNT);
        request.source = StateMachine.polkadot(2000);

        vm.prank(address(host));
        vm.expectRevert(HyperFungibleToken.UnsupportedChain.selector);
        bridge.onAccept(IncomingPostRequest({request: request, relayer: relayer}));
    }

    // ---------- send ----------

    /// Sending back to nexus burns here, which is what releases the escrow there.
    function testSendToNexusBurns() public {
        vm.prank(address(host));
        bridge.onAccept(
            IncomingPostRequest({
                request: _fromNexus(palletId, address(this), MINT_AMOUNT),
                relayer: relayer
            })
        );

        bridge.send(
            HyperFungibleToken.SendParams({
                dest: nexus,
                to: NEXUS_ACCOUNT,
                amount: MINT_AMOUNT,
                timeout: 0,
                relayerFee: 0,
                data: ""
            })
        );

        assertEq(bridge.balanceOf(address(this)), 0);
        assertEq(bridge.totalSupply(), 0);
    }

    function testSendRejectsUnregisteredChain() public {
        vm.prank(address(host));
        bridge.onAccept(
            IncomingPostRequest({
                request: _fromNexus(palletId, address(this), MINT_AMOUNT),
                relayer: relayer
            })
        );

        vm.expectRevert(HyperFungibleToken.UnsupportedChain.selector);
        bridge.send(
            HyperFungibleToken.SendParams({
                dest: StateMachine.evm(42161),
                to: abi.encodePacked(RECIPIENT),
                amount: MINT_AMOUNT,
                timeout: 0,
                relayerFee: 0,
                data: ""
            })
        );
    }

    // ---------- relayer ----------

    function testRelayerIsSetBeforeConfigure() public view {
        assertEq(bridge.relayer(), relayer);
    }

    /// Unlike the base token, no relayer means nobody may deliver, so a deployment that skipped
    /// `setRelayer` cannot mint.
    function testFreshTokenRejectsEveryRelayerUntilSet() public {
        BridgeToken fresh = new BridgeToken(address(this));
        fresh.configure(
            HyperFungibleToken.ConfigOptions({host: address(host), dispatcher: address(callDispatcher)})
        );
        assertEq(fresh.relayer(), address(0));
        PostRequest memory request = _fromNexus(palletId, RECIPIENT, MINT_AMOUNT);
        request.to = abi.encodePacked(address(fresh));

        vm.prank(address(host));
        vm.expectRevert(HyperFungibleToken.UnauthorizedRelayer.selector);
        fresh.onAccept(IncomingPostRequest({request: request, relayer: relayer}));
        vm.prank(address(host));
        vm.expectRevert(HyperFungibleToken.UnauthorizedRelayer.selector);
        fresh.onAccept(IncomingPostRequest({request: request, relayer: address(0xD00D)}));
        assertEq(fresh.totalSupply(), 0);

        fresh.setRelayer(relayer);
        vm.prank(address(host));
        fresh.onAccept(IncomingPostRequest({request: request, relayer: relayer}));
        assertEq(fresh.balanceOf(RECIPIENT), MINT_AMOUNT);
    }

    function testOnAcceptRejectsUnlistedRelayer() public {
        PostRequest memory request = _fromNexus(palletId, RECIPIENT, MINT_AMOUNT);

        vm.prank(address(host));
        vm.expectRevert(HyperFungibleToken.UnauthorizedRelayer.selector);
        bridge.onAccept(IncomingPostRequest({request: request, relayer: address(0xD00D)}));
        assertEq(bridge.totalSupply(), 0, "nothing minted");

        vm.prank(address(host));
        bridge.onAccept(IncomingPostRequest({request: request, relayer: relayer}));
        assertEq(bridge.balanceOf(RECIPIENT), MINT_AMOUNT, "same message mints once the relayer delivers it");
    }

    /// A timeout refund is a mint too, so a forged timeout proof is refused the same way.
    function testTimeoutRefundRejectsUnlistedRelayer() public {
        vm.prank(address(host));
        bridge.onAccept(
            IncomingPostRequest({request: _fromNexus(palletId, address(this), MINT_AMOUNT), relayer: relayer})
        );
        bridge.send(
            HyperFungibleToken.SendParams({
                dest: nexus,
                to: NEXUS_ACCOUNT,
                amount: MINT_AMOUNT,
                timeout: 3600,
                relayerFee: 0,
                data: ""
            })
        );
        assertEq(bridge.totalSupply(), 0);

        PostRequest memory timedOut = PostRequest({
            source: StateMachine.evm(block.chainid),
            dest: nexus,
            nonce: 0,
            from: abi.encodePacked(address(bridge)),
            to: palletId,
            timeoutTimestamp: 3600,
            body: abi.encode(
                HyperFungibleToken.Message({
                    from: abi.encodePacked(address(this)),
                    to: NEXUS_ACCOUNT,
                    amount: MINT_AMOUNT,
                    data: ""
                })
            )
        });

        vm.prank(address(host));
        vm.expectRevert(HyperFungibleToken.UnauthorizedRelayer.selector);
        bridge.onPostRequestTimeout(PostRequestTimeout(timedOut, address(0xD00D)));
        assertEq(bridge.totalSupply(), 0, "no refund minted");

        vm.prank(address(host));
        bridge.onPostRequestTimeout(PostRequestTimeout(timedOut, relayer));
        assertEq(bridge.balanceOf(address(this)), MINT_AMOUNT, "authorised relayer refunds");
    }

    function testSetRelayerOnlyOwner() public {
        address outsider = address(0xD00D);
        vm.prank(outsider);
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, outsider));
        bridge.setRelayer(outsider);

        // The host delivers messages but does not administer the token.
        vm.prank(address(host));
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, address(host)));
        bridge.setRelayer(address(host));

        assertEq(bridge.relayer(), relayer);
    }

    function testSetRelayerRotates() public {
        address next = makeAddr("nextRelayer");
        vm.expectEmit(true, true, true, true, address(bridge));
        emit HyperFungibleToken.RelayerUpdated(relayer, next);
        bridge.setRelayer(next);
        assertEq(bridge.relayer(), next);

        PostRequest memory request = _fromNexus(palletId, RECIPIENT, MINT_AMOUNT);
        vm.prank(address(host));
        vm.expectRevert(HyperFungibleToken.UnauthorizedRelayer.selector);
        bridge.onAccept(IncomingPostRequest({request: request, relayer: relayer}));

        vm.prank(address(host));
        bridge.onAccept(IncomingPostRequest({request: request, relayer: next}));
        assertEq(bridge.balanceOf(RECIPIENT), MINT_AMOUNT);
    }

    /// Zero does not reopen the token the way it does in the base contract.
    function testSetRelayerToZeroFailsClosed() public {
        bridge.setRelayer(address(0));
        PostRequest memory request = _fromNexus(palletId, RECIPIENT, MINT_AMOUNT);

        vm.prank(address(host));
        vm.expectRevert(HyperFungibleToken.UnauthorizedRelayer.selector);
        bridge.onAccept(IncomingPostRequest({request: request, relayer: relayer}));
        vm.prank(address(host));
        vm.expectRevert(HyperFungibleToken.UnauthorizedRelayer.selector);
        bridge.onAccept(IncomingPostRequest({request: request, relayer: address(0xD00D)}));
        assertEq(bridge.totalSupply(), 0);
    }

    /// Through the real host: a refused delivery leaves no receipt, so the relayer can deliver
    /// the same message afterwards.
    function testRejectedDeliveryStaysRetryableThroughHost() public {
        PostRequest memory request = _fromNexus(palletId, RECIPIENT, MINT_AMOUNT);
        bytes32 commitment = request.hash();

        vm.prank(address(handler));
        host.dispatchIncoming(request, address(0xD00D));
        assertEq(host.requestReceipts(commitment), address(0), "refused delivery leaves no receipt");
        assertEq(bridge.totalSupply(), 0, "nothing minted");

        vm.prank(address(handler));
        host.dispatchIncoming(request, relayer);
        assertEq(host.requestReceipts(commitment), relayer, "delivery recorded");
        assertEq(bridge.balanceOf(RECIPIENT), MINT_AMOUNT, "minted");
    }

    /// Builds an incoming transfer from nexus, `from` being the module id it was sent by.
    function _fromNexus(bytes memory from, address recipient, uint256 amount)
        internal
        view
        returns (PostRequest memory)
    {
        return PostRequest({
            source: nexus,
            dest: StateMachine.evm(block.chainid),
            nonce: 0,
            from: from,
            to: abi.encodePacked(address(bridge)),
            timeoutTimestamp: 0,
            body: abi.encode(
                HyperFungibleToken.Message({
                    from: NEXUS_ACCOUNT,
                    to: abi.encodePacked(recipient),
                    amount: amount,
                    data: ""
                })
            )
        });
    }
}
