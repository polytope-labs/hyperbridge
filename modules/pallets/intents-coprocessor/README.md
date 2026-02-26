# Pallet Intents Coprocessor

A Substrate pallet that provides the Hyperbridge component for the Intent Gateway protocol, enabling bid management and cross-chain governance for intent-based transactions.

## Overview

The Pallet Intents serves as the Hyperbridge counterpart to the [IntentGatewayV2.sol](https://github.com/polytope-labs/hyperbridge/blob/main/evm/node_modules/@hyperbridge/core/contracts/apps/IntentGatewayV2.sol) contract, providing three core functionalities:

### 1. Bid Management

Allows fillers (solvers) to place bids for user orders by submitting signed user operations as opaque bytes to the pallet.

- **Storage Fees**: The pallet charges a configurable storage fee for storing bids on-chain
- **Bid Retraction**: Fillers can retract their bids and receive a refund of their storage deposit
- **Decentralized Bid Storage**: Bids are stored transparently on-chain for order matching

### 2. Cross-Chain Governance

Provides cross-chain governance utilities that allow Hyperbridge to manage deployed Intent Gateway instances across multiple chains.

- **Gateway Registry**: Maintains addresses of deployed Intent Gateway contracts and their corresponding chains
- **Parameter Management**: Stores and updates protocol parameters for each Intent Gateway instance
- **Cross-Chain Updates**: Dispatches cross-chain messages to update Intent Gateway configurations

### 3. Oracle Management

Manages the VWAP Oracle token decimals configuration through cross-chain governance.

- **Token Decimals Updates**: Updates token decimal configurations in the [VWAPOracle.sol](https://github.com/polytope-labs/hyperbridge/blob/main/evm/src/utils/VWAPOracle.sol) contract
- **Multi-Chain Support**: Supports decimal updates for tokens across multiple chains

## Architecture

The pallet integrates with:
- **ISMP (Interoperable State Machine Protocol)**: For cross-chain message passing
- **Pallet Hyperbridge**: For core Hyperbridge functionality
- **Intent Gateway Contracts**: EVM-based smart contracts for intent fulfillment

## License

This pallet is licensed under Apache 2.0.