// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Test.sol";

import {MainnetForkBaseTest} from "./MainnetForkBaseTest.sol";
import {IntentGateway, RequestBody, Order, Params, PaymentInfo, TokenInfo, FillOptions, CancelOptions, NewDeployment} from "../src/modules/IntentGateway.sol";

import {IncomingPostRequest, IncomingGetResponse, BaseIsmpModule} from "@polytope-labs/ismp-solidity-v1/IIsmpModule.sol";
import {PostRequest, GetResponse, GetRequest} from "@polytope-labs/ismp-solidity-v1/Message.sol";
import {StorageValue} from "@polytope-labs/solidity-merkle-trees/src/Types.sol";

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

contract IntentGatewayTest is MainnetForkBaseTest {
    using SafeERC20 for IERC20;
    IntentGateway public intentGateway;

    address public filler = address(0x27865);

    receive() external payable {}

    function setUp() public override {
        super.setUp();
        intentGateway = new IntentGateway(address(this));
        Params memory params = Params({host: address(host), dispatcher: address(dispatcher)});
        intentGateway.setParams(params);
        vm.stopPrank();

        // Set initial balances and approvals
        vm.deal(address(this), 100 ether);
        vm.deal(filler, 100 ether);
    }

    // Helper to create an Order struct
    function createTestOrder(bytes memory callData) internal view returns (Order memory) {
        PaymentInfo[] memory outputs = new PaymentInfo[](1);
        outputs[0] = PaymentInfo({
            token: bytes32(uint256(uint160(address(0)))), // Native token
            amount: 1 ether,
            beneficiary: bytes32(uint256(uint160(address(this))))
        });

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(0)))), // Native token
            amount: 1 ether
        });

        return
            Order({
                user: bytes32(uint256(uint160(address(this)))),
                sourceChain: host.host(),
                nonce: 0,
                destChain: bytes("EVM-1"),
                deadline: block.number + 100,
                fees: 0,
                outputs: outputs,
                inputs: inputs,
                callData: callData
            });
    }

    /**
     * @notice Tests the order placement functionality of the IntentGateway
     * @dev This test function verifies the correct behavior of placing orders through the gateway
     */
    function testPlaceOrder() public {
        Order memory order = createTestOrder(bytes(""));

        // Place the order
        intentGateway.placeOrder{value: 1 ether}(order, bytes32(""));

        vm.expectRevert(IntentGateway.InsufficientNativeToken.selector);
        intentGateway.placeOrder{value: 0.9 ether}(order, bytes32(""));

        // Check the balances
        assertEq(address(this).balance, 99 ether);
        assertEq(address(intentGateway).balance, 1 ether);
    }

    // write a test for the `fillOrder` function
    function testFillOrder() public {
        Order memory order = createTestOrder(bytes(""));

        // Place the order
        intentGateway.placeOrder{value: 1 ether}(order, bytes32(""));

        assertEq(filler.balance, 100 ether);
        assertEq(address(intentGateway).balance, 1 ether);

        Order memory order1 = abi.decode(abi.encode(order), (Order));
        order1.destChain = bytes("EVM-2");
        vm.expectRevert(IntentGateway.WrongChain.selector);
        intentGateway.fillOrder{value: 2 ether}(order1, FillOptions({relayerFee: 0}));

        uint256 initial = block.number;
        vm.roll(order.deadline + 1);
        assertEq(block.number, order.deadline + 1);
        vm.expectRevert(IntentGateway.Expired.selector);
        intentGateway.fillOrder{value: 2 ether}(order, FillOptions({relayerFee: 0}));
        vm.roll(initial);

        vm.expectRevert(IntentGateway.InsufficientNativeToken.selector);
        intentGateway.fillOrder{value: 0.9 ether}(order, FillOptions({relayerFee: 0}));

        // Fill the order
        vm.prank(filler);
        intentGateway.fillOrder{value: 2 ether}(order, FillOptions({relayerFee: 0}));

        vm.expectRevert(IntentGateway.Filled.selector);
        intentGateway.cancelOrder{value: 1 ether}(order, CancelOptions({relayerFee: 0, height: order.deadline + 1}));

        // Construct storage value for filled request
        bytes memory context = abi.encode(
            RequestBody({
                commitment: keccak256(abi.encode(order)),
                tokens: order.inputs,
                beneficiary: bytes32(uint256(uint160(address(this))))
            })
        );
        bytes memory hostId = host.host();
        StorageValue[] memory values = new StorageValue[](1);
        values[0].value = bytes("0xdeadbeef");
        vm.startPrank(address(host));
        vm.expectRevert(IntentGateway.Filled.selector);
        intentGateway.onGetResponse(
            IncomingGetResponse({
                relayer: address(0),
                response: GetResponse({
                    values: values,
                    request: GetRequest({
                        source: hostId,
                        dest: hostId,
                        nonce: 0,
                        from: address(intentGateway),
                        timeoutTimestamp: 0,
                        context: context,
                        keys: new bytes[](0),
                        height: uint64(block.number + 1)
                    })
                })
            })
        );

        // Check the balances
        assertEq(address(this).balance, 100 ether);
        assertEq(filler.balance, 98 ether);
    }

    function testRedeemEscrow() public {
        Order memory order = createTestOrder(bytes(""));

        // Place the order
        intentGateway.placeOrder{value: 1 ether}(order, bytes32(""));

        assertEq(filler.balance, 100 ether);
        assertEq(address(intentGateway).balance, 1 ether);

        // Fill the order
        vm.prank(filler);
        intentGateway.fillOrder{value: 2 ether}(order, FillOptions({relayerFee: 0}));

        // Check the balances
        assertEq(address(this).balance, 100 ether);
        assertEq(filler.balance, 98 ether);

        // Redeem the escrow
        bytes memory data = abi.encode(
            RequestBody({
                commitment: keccak256(abi.encode(order)),
                tokens: order.inputs,
                beneficiary: bytes32(uint256(uint160(filler)))
            })
        );
        bytes memory hostId = host.host();
        PostRequest memory request = PostRequest({
            source: hostId,
            dest: hostId,
            nonce: 0,
            from: abi.encodePacked(bytes32(uint256(uint160(address(0x1256))))),
            to: abi.encodePacked(bytes32(uint256(uint160(address(intentGateway))))),
            body: bytes.concat(bytes1(uint8(IntentGateway.RequestKind.RedeemEscrow)), data),
            timeoutTimestamp: 0
        });

        vm.expectRevert(BaseIsmpModule.UnauthorizedCall.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: request}));

        request.from = abi.encodePacked(bytes32(uint256(uint160(address(intentGateway)))));
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: request}));

        // requests are invalidated as soon as they are executed
        // but just checking that we delete the order from storage
        vm.prank(address(host));
        vm.expectRevert(IntentGateway.UnknownOrder.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: request}));

        // Check the balances
        assertEq(address(intentGateway).balance, 0 ether);
        assertEq(filler.balance, 99 ether);
    }

    function testCancelOrder() public {
        // Place the order
        Order memory order = createTestOrder(bytes(""));
        order.deadline = block.number - 100;
        intentGateway.placeOrder{value: 1 ether}(order, bytes32(""));

        assertEq(address(this).balance, 99 ether);
        assertEq(address(intentGateway).balance, 1 ether);

        vm.prank(filler);
        vm.expectRevert(IntentGateway.Unauthorized.selector);
        intentGateway.cancelOrder{value: 1 ether}(order, CancelOptions({relayerFee: 0, height: block.number}));

        vm.expectRevert(IntentGateway.NotExpired.selector);
        intentGateway.cancelOrder{value: 1 ether}(order, CancelOptions({relayerFee: 0, height: order.deadline - 1}));

        // Cancel the order
        intentGateway.cancelOrder{value: 1 ether}(order, CancelOptions({relayerFee: 0, height: block.number}));

        // Respond with storage proof
        bytes memory context = abi.encode(
            RequestBody({
                commitment: keccak256(abi.encode(order)),
                tokens: order.inputs,
                beneficiary: bytes32(uint256(uint160(address(this))))
            })
        );
        bytes memory hostId = host.host();
        StorageValue[] memory values = new StorageValue[](1);
        GetResponse memory response = GetResponse({
            values: values,
            request: GetRequest({
                source: hostId,
                dest: hostId,
                nonce: 0,
                from: address(intentGateway),
                timeoutTimestamp: 0,
                context: context,
                keys: new bytes[](0),
                height: uint64(block.number + 1)
            })
        });
        vm.prank(address(host));
        intentGateway.onGetResponse(IncomingGetResponse({relayer: address(0), response: response}));

        // requests are invalidated as soon as they are executed
        // but just checking that we delete the order from storage
        vm.prank(address(host));
        vm.expectRevert(IntentGateway.UnknownOrder.selector);
        intentGateway.onGetResponse(IncomingGetResponse({relayer: address(0), response: response}));

        // Check the balances
        assertEq(address(this).balance, 99 ether);
        assertEq(address(intentGateway).balance, 0 ether);
    }

    function testCrossChainGovernance() public {
        // Add new contract deployments
        NewDeployment memory deployment = NewDeployment({
            stateMachineId: bytes("EVM-5"),
            gateway: bytes32(uint256(uint160(address(0xdeadbeef))))
        });
        bytes memory data = abi.encode(deployment);
        bytes memory hostId = host.host();
        PostRequest memory newDeploymentRequest = PostRequest({
            source: hostId,
            dest: hostId,
            nonce: 0,
            from: abi.encodePacked(bytes32(uint256(uint160(address(intentGateway))))),
            to: abi.encodePacked(bytes32(uint256(uint160(address(intentGateway))))),
            body: bytes.concat(bytes1(uint8(IntentGateway.RequestKind.NewDeployment)), data),
            timeoutTimestamp: 0
        });

        // source should be hyperbridge
        vm.prank(address(host));
        vm.expectRevert(IntentGateway.Unauthorized.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: newDeploymentRequest}));

        // ensure that intent gateway rejects the settlement request
        Order memory order = createTestOrder(bytes(""));
        intentGateway.placeOrder{value: 1 ether}(order, bytes32(""));
        PostRequest memory redeemEscrowRequest = PostRequest({
            source: deployment.stateMachineId,
            dest: hostId,
            nonce: 0,
            from: abi.encodePacked(deployment.gateway),
            to: abi.encodePacked(bytes32(uint256(uint160(address(intentGateway))))),
            body: bytes.concat(
                bytes1(uint8(IntentGateway.RequestKind.RedeemEscrow)),
                abi.encode(
                    RequestBody({
                        commitment: keccak256(abi.encode(order)),
                        tokens: order.inputs,
                        beneficiary: bytes32(uint256(uint160(filler)))
                    })
                )
            ),
            timeoutTimestamp: 0
        });
        vm.prank(address(host));
        vm.expectRevert(IntentGateway.Unauthorized.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: redeemEscrowRequest}));

        // Execute cross-chain governance request
        newDeploymentRequest.source = host.hyperbridge();
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: newDeploymentRequest}));

        // Now it recognizes the deployment
        vm.deal(address(intentGateway), 1 ether);
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: redeemEscrowRequest}));

        // Check the settlement was successful
        assertEq(address(intentGateway).balance, 0 ether);
        assertEq(filler.balance, 101 ether);

        // Update parameters
        Params memory params = intentGateway.params();
        params.dispatcher = address(0xdeadbeef);
        PostRequest memory updateParamsRequest = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(bytes32(uint256(uint160(address(intentGateway))))),
            to: abi.encodePacked(bytes32(uint256(uint160(address(intentGateway))))),
            body: bytes.concat(bytes1(uint8(IntentGateway.RequestKind.UpdateParams)), abi.encode(params)),
            timeoutTimestamp: 0
        });
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: updateParamsRequest}));
        assertEq(intentGateway.params().dispatcher, params.dispatcher);
    }
}
