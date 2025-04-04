# @hyperbridge/sdk

A JavaScript/TypeScript SDK for interacting with the Hyperbridge indexer and monitoring cross-chain messages.

## Installation

```bash
npm install hyperbridge-sdk
# or
yarn add hyperbridge-sdk
# or
pnpm add hyperbridge-sdk
```

## Usage

### Initialize Client

```ts
import { IndexerClient } from "hyperbridge-sdk"

const queryClient = createQueryClient({
	url: "http://localhost:3000", // URL of the Hyperbridge indexer API
})

const indexer = new IndexerClient({
	queryClient: queryClient,
	pollInterval: 1_000, // Every second
	source: {
		consensusStateId: "BSC0",
		rpcUrl: "https://data-seed-prebsc-1-s1.binance.org:8545",
		stateMachineId: "EVM-97",
		host: "0x...", // Host contract address
	},
	dest: {
		consensusStateId: "GNO0",
		rpcUrl: "https://rpc.chiadochain.net",
		stateMachineId: "EVM-10200",
		host: "0x...", // Host contract address
	},
	hyperbridge: {
		consensusStateId: "PAS0",
		stateMachineId: "KUSAMA-4009",
		wsUrl: "wss://gargantua.polytope.technology",
	},
})
```

### Monitor Post Request Status

```ts
import { postRequestCommitment } from "hyperbridge-sdk"

// Get status stream for a commitment
const commitment = postRequestCommitment(request)
for await (const status of indexer.postRequestStatusStream(commitment)) {
	switch (status.status) {
		case RequestStatus.SOURCE_FINALIZED:
			console.log("Request finalized on source chain")
			break
		case RequestStatus.HYPERBRIDGE_DELIVERED:
			console.log("Request delivered to Hyperbridge")
			break
		// other statuses
	}
}
```

### Monitor Timeout Status

```ts
// Get timeout status stream
for await (const timeout of indexer.postRequestTimeoutStream(commitment)) {
	switch (timeout.status) {
		case TimeoutStatus.PENDING_TIMEOUT:
			console.log("Request pending timeout")
			break
		case TimeoutStatus.HYPERBRIDGE_TIMED_OUT:
			console.log("Request timed out on Hyperbridge")
			break
		// other timeout statuses
	}
}
```

### Query Request Status

```ts
// Get current status
const request = await indexer.queryRequestWithStatus(commitment)
console.log(request?.statuses)
```

Alternatively. You can use the `queryRequest` utility

```ts
import { createQueryClient, queryRequest } from "hyperbridge-sdk"

const queryClient = createQueryClient({
	url: "http://localhost:3000", // URL of the Hyperbridge indexer API
})

const commitmentHash = "0x...."

// Get request statuses
const request = await queryRequest({ commitmentHash, queryClient })
console.log(request.statuses) // read transaction statuses
```

### Chain Utilities

```ts
import { EvmChain, SubstrateChain } from "hyperbridge-sdk"

// Interact with EVM chains
const evmChain = new EvmChain({
	url: "https://rpc.chiadochain.net",
	chainId: 10200,
	host: "0x58A41B89F4871725E5D898d98eF4BF917601c5eB",
})

// Interact with Substrate chains
const hyperbridge = new SubstrateChain({
	ws: "wss://gargantua.dev.polytope.technology",
	hasher: "Keccak",
})

const proof = await hyperbridge.queryStateProof(blockNumber, keys)
```

## API Reference

### Classes

- IndexerClient - Main client for interacting with the indexer
- EvmChain - Utilities for EVM chain interaction
- SubstrateChain - Utilities for Substrate chain interaction

### Types

- RequestStatus - Enum of possible request statuses
- TimeoutStatus - Enum of possible timeout statuses
- HexString - Type for hex-encoded strings

### Examples

See the tests [directory](/packages/sdk/src/tests/postRequest.test.ts) for complete examples.
