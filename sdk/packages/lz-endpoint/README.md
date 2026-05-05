# @hyperbridge/lz-endpoint

![Tests](https://github.com/polytope-labs/hyperbridge/actions/workflows/test-lz-endpoint.yml/badge.svg)
[![NPM](https://img.shields.io/npm/v/@hyperbridge/lz-endpoint?label=%40hyperbridge%2Flz-endpoint)](https://www.npmjs.com/package/@hyperbridge/lz-endpoint)

A LayerZero V2 endpoint adapter that routes messages through Hyperbridge's ISMP protocol. Allows existing OApp and OFT deployments to use Hyperbridge for cross-chain transport without any code changes.

## Overview

`HyperbridgeLzEndpoint` implements the full `ILayerZeroEndpointV2` interface — including send, receive, compose, nonce tracking, fee quoting, and send context — but uses ISMP under the hood instead of LayerZero's DVN/executor infrastructure. Any OApp or OFT can point to this contract as its endpoint and get Hyperbridge-secured cross-chain messaging.

This contract is intended to be deployed **per OApp, per chain**. Each deployment gets its own `HyperbridgeLzEndpoint` instance configured with the appropriate host, eid, and chain mappings.

## Compatibility

Compatible with all OApps built on LayerZero V2, including:

- **OFTs** (burn/mint cross-chain tokens)
- **OFT Adapters** (lock/unlock wrappers)
- **OApps with compose** (`SEND_AND_CALL` pattern)
- **Custom OApps** (any `ILayerZeroReceiver` implementation)

### Compose behavior

Compose follows the same two-step model as the real LZ endpoint:

1. During `lzReceive`, the OApp calls `endpoint.sendCompose()` which queues the compose message hash
2. In a separate transaction, anyone (typically a relayer) calls `endpoint.lzCompose()` to verify the hash and execute the compose call on the target `ILayerZeroComposer`

This means tokens land safely even if the compose call hasn't been executed yet, and compose can be retried if it reverts.

## How it works

### Outgoing (OApp calls `send()`)

1. OApp calls `endpoint.send(MessagingParams, refundAddress)`
2. The adapter translates `MessagingParams` into an ISMP `DispatchPost`
3. The LZ message payload is wrapped in the ISMP body along with nonce, guid, and sender/receiver info
4. The ISMP host dispatches the message to the destination chain via Hyperbridge

### Incoming (Hyperbridge delivers via `onAccept()`)

1. ISMP host calls `onAccept()` with the incoming cross-chain message
2. The adapter decodes the LZ message components from the ISMP body
3. Validates the inbound nonce for ordered delivery
4. Calls `lzReceive()` on the destination OApp with the original LZ `Origin`, guid, and message
5. If the OApp triggers a compose via `sendCompose()`, the compose message is queued for separate execution via `lzCompose()`

### Fee payment

Fees can be paid in two ways:

- **Native tokens** (`payInLzToken = false`) — the OApp sends ETH via `msg.value`. The ISMP host swaps it to the fee token via Uniswap.
- **Fee token** (`payInLzToken = true`) — the OApp transfers the host's fee token directly. `endpoint.lzToken()` returns the host's fee token address. No swap needed, cheaper.

The `quote()` function returns the appropriate fee based on the `payInLzToken` flag.

### Relayer fees

A per-destination relayer fee incentivizes Hyperbridge relayers to deliver messages. Defaults to $0.30 (computed from the fee token's decimals). The adapter owner can customize fees:

- `setRelayerFee(uint32 dstEid, uint256 fee)` — set a per-destination fee
- `setDefaultRelayerFee(uint256 fee)` — change the default
- `relayerFee(uint32 dstEid)` — returns the effective fee (per-destination if set, otherwise default)

## Deployment

```solidity
// Deploy the adapter (no constructor args for CREATE2 compatibility)
HyperbridgeLzEndpoint endpoint = new HyperbridgeLzEndpoint();

// Configure host and local eid
endpoint.setHost(ismpHostAddress, localEid);

// Register chain mappings (LZ eid <-> ISMP state machine ID)
endpoint.setEidMapping(30101, StateMachine.evm(1));       // Ethereum
endpoint.setEidMapping(30110, StateMachine.evm(42161));    // Arbitrum
endpoint.setEidMapping(30111, StateMachine.evm(10));       // Optimism

// Deploy a new OApp pointing to this adapter
MyOFT oft = new MyOFT(address(endpoint), delegate);
```

## Migrating an existing proxy OApp

If you have an existing OApp deployed behind a proxy (UUPS or Transparent), you can migrate to Hyperbridge without redeploying or migrating state.

The `endpoint` variable in LayerZero's `OAppCore` is `immutable` — it lives in the implementation contract's bytecode, not in proxy storage. This means all proxy storage (token balances, peers, ownership, nonces) is preserved across the upgrade. Only the endpoint address changes.

### Steps

1. **Deploy the adapter** on each chain where your OApp exists:

```solidity
HyperbridgeLzEndpoint adapter = new HyperbridgeLzEndpoint();
adapter.setHost(ismpHostAddress, localEid);
adapter.setEidMapping(30101, StateMachine.evm(1));
// ... add all chains your OApp supports
```

2. **Deploy a new implementation** that hardcodes the adapter as the endpoint. Your existing OApp contract stays the same — just change the endpoint address passed to the constructor:

```solidity
// Before (LayerZero endpoint)
contract MyOFT is OFT {
    constructor() OFT("My Token", "MTK", 0x1a44076050125825900e736c501f859c50fE728c, owner) {}
}

// After (Hyperbridge adapter)
contract MyOFTV2 is OFT {
    constructor() OFT("My Token", "MTK", address(hyperbridgeAdapter), owner) {}
}
```

3. **Upgrade the proxy** on each chain:

```solidity
proxy.upgradeTo(address(myOFTV2Implementation));
```

All token balances, peer configurations, and ownership are preserved in proxy storage. The OApp now routes messages through Hyperbridge instead of LayerZero.

### What's preserved

- Token balances (ERC20 storage)
- Peer mappings (`peers[eid] => bytes32`)
- Ownership
- All other proxy storage

### What changes

- The `endpoint` immutable now points to `HyperbridgeLzEndpoint`
- Cross-chain messages are secured by Hyperbridge's ISMP consensus proofs instead of LayerZero's DVN network

## EID Mapping

LayerZero uses its own endpoint IDs (eids) which differ from chain IDs. The mapping between eids and ISMP state machine identifiers is configured by the contract owner via `setEidMapping(uint32 eid, bytes stateMachineId)`.

Query mappings via:
- `eidMapping(uint32 eid)` — returns the ISMP state machine ID for a given eid
- `eidFor(bytes stateMachineId)` — returns the eid for a given ISMP state machine ID
- `isSupportedEid(uint32 eid)` — checks if an eid has been configured
