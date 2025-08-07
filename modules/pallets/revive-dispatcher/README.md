# pallet-revive-ismp-dispatcher

ISMP dispatcher precompile implementation for pallet-revive.

## Overview

The `pallet-revive-ismp-dispatcher` provides a precompile interface for PolkaVM contracts to dispatch ISMP (Interoperable State Machine Protocol) messages. This pallet focuses exclusively on outgoing message dispatch functionality, enabling contracts to:

- Dispatch POST requests to other chains
- Dispatch GET requests for cross-chain state queries
- Dispatch responses to incoming requests
- Fund existing requests and responses with additional fees
- Query configuration parameters like fee tokens and per-byte fees

## Interface

The dispatcher exposes the following core functionality:

### Configuration Queries
- `host()` - Returns the host state machine identifier
- `hyperbridge()` - Returns the connected hyperbridge instance identifier
- `nonce()` - Returns the next available nonce for requests
- `feeToken()` - Returns the configured fee token address
- `perByteFee(bytes dest)` - Returns the per-byte fee for the destination chain

### Message Dispatch
- `dispatch(DispatchPost)` - Dispatches a POST request
- `dispatch(DispatchGet)` - Dispatches a GET request
- `dispatch(DispatchPostResponse)` - Dispatches a response to a POST request

### Fee Management
- `fundRequest(bytes32 commitment, uint256 amount)` - Adds additional fees to a pending request
- `fundResponse(bytes32 commitment, uint256 amount)` - Adds additional fees to a pending response

## Usage

This pallet is designed to be used as a precompile in pallet-revive based runtimes. Contracts can interact with it at the fixed address `0xD27` (3367 in decimal).

## License

This project is licensed under the Apache License 2.0. See the LICENSE file for details.