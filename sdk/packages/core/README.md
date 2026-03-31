# @hyperbridge/core

![Unit Tests](https://github.com/polytope-labs/hyperbridge-sdk/actions/workflows/test-core.yml/badge.svg)
[![NPM](https://img.shields.io/npm/v/@hyperbridge/core?label=%40hyperbridge%2Fcore)](https://www.npmjs.com/package/@hyperbridge/core)

Hyperbridge SDK for EVM environments. This contains libraries & interfaces for working with hyperbridge contracts. These contracts can be used for sending data (Post requests), pulling data (Get requests), and working with token transfers (TokenGateway & IntentGateway)

### Interfaces

 - [`IHost`](contracts/interfaces/IHost.sol) - The protocol host interface
 - [`IDispatcher`](contracts/interfaces/IDispatcher.sol) - The protocol dispatcher interface
 - [`IHandler`](contracts/interfaces/IHandler.sol) - The protocol message handler interface
 - [`IConsensus`](contracts/interfaces/IConsensus.sol) - The consensus interface
 - [`IApp`](contracts/interfaces/IApp.sol) - The protocol application interface

### Libraries

 - [`Message`](contracts/libraries/Message.sol) - The protocol message types and data structures
 - [`StateMachine`](contracts/libraries/StateMachine.sol) - State machine identifier utilities

### Apps

 - [`HyperApp`](contracts/apps/HyperApp.sol) - Abstract base contract that implements `IApp` for building cross-chain applications with built-in fee estimation and host authorization
 - [`HyperFungibleToken`](contracts/apps/HyperFungibleToken.sol) - Abstract ERC20 token implementation with gateway-restricted minting and burning capabilities for cross-chain bridging
 - [`ITokenGateway`](contracts/apps/TokenGateway.sol) - Interface for the TokenGateway contract that enables transfers of hyper-fungible tokens using Hyperbridge. Supports both ERC20 token custody and ERC6160 token burn-and-mint mechanisms
 - [`IIntentGateway`](contracts/apps/IntentGateway.sol) - Interface for the IntentGateway contract that facilitates cross-chain intent-based orders. Allows users to place orders that can be filled by market makers across different chains

## License

This library is licensed under the Apache 2.0 License, Copyright (c) 2025 Polytope Labs.