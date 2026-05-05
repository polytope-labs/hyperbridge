# @hyperbridge/core

![Unit Tests](https://github.com/polytope-labs/hyperbridge-sdk/actions/workflows/test-core.yml/badge.svg)
[![NPM](https://img.shields.io/npm/v/@hyperbridge/core?label=%40hyperbridge%2Fcore)](https://www.npmjs.com/package/@hyperbridge/core)

Hyperbridge SDK for EVM environments. Contains libraries, interfaces, and application contracts for building cross-chain applications on Hyperbridge. These contracts can be used for sending data (Post requests), pulling data (Get requests), and working with cross-chain token transfers.

### Interfaces

 - [`IHost`](contracts/interfaces/IHost.sol) - The protocol host interface
 - [`IDispatcher`](contracts/interfaces/IDispatcher.sol) - The protocol dispatcher interface
 - [`IHandler`](contracts/interfaces/IHandler.sol) - The protocol message handler interface
 - [`IConsensus`](contracts/interfaces/IConsensus.sol) - The consensus interface
 - [`IApp`](contracts/interfaces/IApp.sol) - The protocol application interface
 - [`ICallDispatcher`](contracts/interfaces/ICallDispatcher.sol) - Interface for executing arbitrary calls on the destination chain
 - [`IWETH`](contracts/interfaces/IWETH.sol) - Minimal WETH interface for native token wrapping

### Libraries

 - [`Message`](contracts/libraries/Message.sol) - The protocol message types and data structures
 - [`StateMachine`](contracts/libraries/StateMachine.sol) - State machine identifier utilities

### Apps

 - [`HyperApp`](contracts/apps/HyperApp.sol) - Abstract base contract that implements `IApp` for building cross-chain applications with built-in fee estimation and host authorization
 - [`HyperFungibleToken`](contracts/apps/HyperFungibleToken.sol) - Cross-chain fungible token that is itself a bridge application. Burns tokens on the source chain and mints on the destination chain. Each deployment communicates with peer instances across networks via Hyperbridge — no governance overhead or shared custody pool.
 - [`WrappedHyperFungibleToken`](contracts/apps/WrappedHyperFungibleToken.sol) - Cross-chain wrapper for existing ERC20 tokens. Locks the underlying token on the source chain and unlocks on the destination. Supports native ETH wrapping via WETH. Can be paired with a companion `HyperFungibleToken` on another network to isolate TVL per instance.
 - [`IntentGatewayV2`](contracts/apps/IntentGatewayV2.sol) - Cross-chain intent-based order gateway. Users place orders specifying desired outputs, solvers fill them on the destination chain and prove fulfillment via Hyperbridge

## HyperApp

Build any cross-chain application by extending `HyperApp`. Override `onAccept` to handle incoming messages.

```solidity
import {HyperApp} from "@hyperbridge/core/apps/HyperApp.sol";
import {DispatchPost, IDispatcher} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";
import {PostRequest} from "@hyperbridge/core/libraries/Message.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";

contract MyApp is HyperApp {
    address internal _host;

    function host() public view override returns (address) {
        return _host;
    }

    /// Send a cross-chain message
    function ping(bytes calldata dest, bytes calldata message) external payable {
        DispatchPost memory request = DispatchPost({
            dest: dest,
            to: abi.encodePacked(address(this)),
            body: message,
            timeout: 3600,
            fee: 0,
            payer: msg.sender
        });

        IDispatcher(_host).dispatch{value: msg.value}(request);
    }

    /// Receive a cross-chain message
    function onAccept(IncomingPostRequest calldata incoming) external override onlyHost {
        // Handle incoming.request.body
    }

    /// Handle timeout (message not delivered before expiry)
    function onPostRequestTimeout(PostRequest memory request) external override onlyHost {
        // Refund or cleanup
    }
}
```

## HyperFungibleToken

A token that IS its own bridge. No TokenGateway, no governance, no shared custody.

```solidity
// Deploy
HyperFungibleToken token = new HyperFungibleToken("My Token", "MTK");

// Configure
token.configure(HyperFungibleToken.ConfigOptions({
    host: ismpHostAddress,
    dispatcher: callDispatcherAddress
}));

// Register peer on Arbitrum
token.addChain(StateMachine.evm(42161), arbitrumTokenAddress);

// Send cross-chain
token.send(HyperFungibleToken.SendParams({
    dest: StateMachine.evm(42161),
    to: abi.encodePacked(recipient),
    amount: 1000e18,
    timeout: 3600,
    relayerFee: 0,
    data: ""
}));
```

## WrappedHyperFungibleToken

Wraps an existing ERC20 for cross-chain transfers. Supports native ETH via WETH.

```solidity
// Deploy
WrappedHyperFungibleToken wrapper = new WrappedHyperFungibleToken();

// Configure with WETH as underlying
wrapper.configure(WrappedHyperFungibleToken.WrappedConfigOptions({
    host: ismpHostAddress,
    dispatcher: callDispatcherAddress,
    underlying: wethAddress,
    isWeth: true
}));

// Register peer chain
wrapper.addChain(StateMachine.evm(42161), arbitrumWrapperAddress);

// Send native ETH cross-chain (wraps to WETH, locks, sends)
wrapper.send{value: 1 ether}(HyperFungibleToken.SendParams({
    dest: StateMachine.evm(42161),
    to: abi.encodePacked(recipient),
    amount: 1 ether,
    timeout: 3600,
    relayerFee: 0,
    data: ""
}));
```

## IntentGatewayV2

The IntentGateway enables cross-chain intent-based orders. Users place orders specifying desired outputs, and market makers (solvers) fill them on the destination chain.

```solidity
import {Order, TokenInfo, PaymentInfo, FillOptions, DispatchInfo} from "@hyperbridge/core/apps/IntentGatewayV2.sol";

// Define inputs: 1000 USDC escrowed on source chain
TokenInfo[] memory inputs = new TokenInfo[](1);
inputs[0] = TokenInfo({
    token: bytes32(uint256(uint160(USDC_ADDRESS))),
    amount: 1000e6
});

// Define outputs: 1000 DAI to receive on destination chain
TokenInfo[] memory outputs = new TokenInfo[](1);
outputs[0] = TokenInfo({
    token: bytes32(uint256(uint160(DAI_ADDRESS))),
    amount: 1000e18
});

// Place an order: swap 1000 USDC on Ethereum for 1000 DAI on Arbitrum
Order memory order = Order({
    user: bytes32(uint256(uint160(msg.sender))),
    source: StateMachine.evm(1),
    destination: StateMachine.evm(42161),
    deadline: block.number + 100,
    nonce: 0,
    fees: 0,
    session: address(0),
    predispatch: DispatchInfo({assets: new TokenInfo[](0), call: new bytes(0)}),
    inputs: inputs,
    output: PaymentInfo({
        beneficiary: bytes32(uint256(uint160(recipient))),
        assets: outputs,
        call: new bytes(0)
    })
});

intentGateway.placeOrder(order, bytes32(0));
```

Solvers fill orders on the destination chain and prove fulfillment via Hyperbridge to claim escrowed tokens on the source.

## License

This library is licensed under the Apache 2.0 License, Copyright (c) 2025 Polytope Labs.
