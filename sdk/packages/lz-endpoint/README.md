# @hyperbridge/oft-adapter

A LayerZero V2 endpoint adapter that routes messages through Hyperbridge's ISMP protocol. Allows existing OFT deployments to use Hyperbridge for cross-chain transport without any code changes.

## Overview

`HyperbridgeLzEndpoint` implements the full `ILayerZeroEndpointV2` interface but uses ISMP under the hood instead of LayerZero's DVN/executor infrastructure. An OFT simply sets this contract as its endpoint and gets Hyperbridge-secured cross-chain messaging.

This contract is intended to be deployed **per OFT, per chain**. Each OFT deployment gets its own `HyperbridgeLzEndpoint` instance configured with the appropriate host, eid, and chain mappings.

## How it works

### Outgoing (OFT calls `send()`)

1. OFT calls `endpoint.send(MessagingParams, refundAddress)`
2. The adapter translates `MessagingParams` into an ISMP `DispatchPost`
3. The LZ message payload is wrapped in the ISMP body along with nonce, guid, and sender/receiver info
4. The ISMP host dispatches the message to the destination chain via Hyperbridge

### Incoming (Hyperbridge delivers via `onAccept()`)

1. ISMP host calls `onAccept()` with the incoming cross-chain message
2. The adapter decodes the LZ message components from the ISMP body
3. Validates the inbound nonce for ordered delivery
4. Calls `lzReceive()` on the destination OApp with the original LZ `Origin`, guid, and message

### Fee payment

Fees are paid in the same way as any Hyperbridge application — either native tokens (via `msg.value`) or the host's fee token. The `quote()` function returns the estimated native fee via `HyperApp.quoteNative()`.

## Deployment

```solidity
// Deploy the adapter for an OFT on this chain (no constructor args for CREATE2 compatibility)
HyperbridgeLzEndpoint endpoint = new HyperbridgeLzEndpoint();

// Configure host and local eid
endpoint.setHost(ismpHostAddress, localEid);

// Register chain mappings (LZ eid <-> ISMP state machine ID)
endpoint.setEidMapping(30101, StateMachine.evm(1));       // Ethereum
endpoint.setEidMapping(30110, StateMachine.evm(42161));    // Arbitrum
endpoint.setEidMapping(30111, StateMachine.evm(10));       // Optimism

// Deploy a new OFT pointing to this adapter
MyOFT oft = new MyOFT(address(endpoint), delegate);
```

## Migrating an existing proxy OFT

If you have an existing OFT deployed behind a proxy (UUPS or Transparent), you can migrate to Hyperbridge without redeploying or migrating state.

The `endpoint` variable in LayerZero's `OAppCore` is `immutable` — it lives in the implementation contract's bytecode, not in proxy storage. This means all proxy storage (token balances, peers, ownership, nonces) is preserved across the upgrade. Only the endpoint address changes.

### Steps

1. **Deploy the adapter** on each chain where your OFT exists:

```solidity
HyperbridgeLzEndpoint adapter = new HyperbridgeLzEndpoint();
adapter.setHost(ismpHostAddress, localEid);
adapter.setEidMapping(30101, StateMachine.evm(1));
// ... add all chains your OFT supports
```

2. **Deploy a new implementation** that hardcodes the adapter as the endpoint. Your existing OFT contract stays the same — just change the endpoint address passed to the constructor:

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

That's it. All token balances, peer configurations, and ownership are preserved in proxy storage. The OFT now routes messages through Hyperbridge instead of LayerZero.

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
