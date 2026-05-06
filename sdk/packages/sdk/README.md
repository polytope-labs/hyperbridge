# @hyperbridge/sdk

A JavaScript/TypeScript SDK for interacting with the Hyperbridge indexer and monitoring cross-chain messages.

## Installation

```bash
npm install @hyperbridge/sdk
# or
yarn add @hyperbridge/sdk
# or
pnpm add @hyperbridge/sdk
```

## Usage

### Initialize Client

```ts
import { IsmpClient, createQueryClient, EvmChain, SubstrateChain } from "@hyperbridge/sdk"

const queryClient = createQueryClient({
	url: "http://localhost:3000", // URL of the Hyperbridge indexer API
})

// Create chain instances directly
const sourceChain = new EvmChain({
	chainId: 97,
	rpcUrl: "https://data-seed-prebsc-1-s1.binance.org:8545",
	host: "0x...", // Host contract address
	consensusStateId: "BSC0"
})

const destChain = new EvmChain({
	chainId: 10200,
	rpcUrl: "https://rpc.chiadochain.net",
	host: "0x...", // Host contract address
	consensusStateId: "GNO0"
})

const hyperbridgeChain = new SubstrateChain({
	stateMachineId: "KUSAMA-4009",
	wsUrl: "wss://gargantua.polytope.technology",
	hasher: "Keccak",
	consensusStateId: "PAS0"
})

// Connect to Substrate chain
await hyperbridgeChain.connect()

// Create the IsmpClient
const indexer = new IsmpClient({
	queryClient: queryClient,
	pollInterval: 1_000, // Every second
	source: sourceChain,
	dest: destChain,
	hyperbridge: hyperbridgeChain
})
```


### Monitor Post Request Status

```ts
import { postRequestCommitment } from "@hyperbridge/sdk"

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

Alternatively. You can use the `queryPostRequest` utility

```ts
import { createQueryClient, queryPostRequest } from "@hyperbridge/sdk"

const queryClient = createQueryClient({
	url: "http://localhost:3000", // URL of the Hyperbridge indexer API
})

const commitmentHash = "0x...."

// Get request statuses
const request = await queryPostRequest({ commitmentHash, queryClient })
console.log(request.statuses) // read transaction statuses
```

### Chain Utilities

```ts
import { EvmChain, SubstrateChain } from "@hyperbridge/sdk"

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

### HyperFungibleToken - Cross-Chain Token Transfers

The `HyperFungibleToken` class provides a generator-based flow for bridging tokens cross-chain via Hyperbridge. It supports both `HyperFungibleToken` (burn/mint) and `WrappedHyperFungibleToken` (lock/unlock) contracts, with automatic type detection via ERC165.

#### Setup

```ts
import {
	HyperFungibleToken,
	IsmpClient,
	createQueryClient,
	EvmChain,
	SubstrateChain
} from "@hyperbridge/sdk"
import { parseEther } from "viem"

const source = EvmChain.fromParams({
	chainId: 97, // BSC Testnet
	rpcUrl: "https://data-seed-prebsc-1-s1.binance.org:8545",
	host: "0x...", // IsmpHost contract address
	consensusStateId: "BSC0",
})

const dest = EvmChain.fromParams({
	chainId: 80002, // Polygon Amoy
	rpcUrl: "https://rpc-amoy.polygon.technology",
	host: "0x...", // IsmpHost contract address
	consensusStateId: "POL0",
})

const hft = new HyperFungibleToken({ source, dest })
```

#### Detect Token Type

```ts
/* Returns true for WrappedHyperFungibleToken contracts (ERC165) */
const isWrapped = await hft.isWrapped(tokenAddress)
```

#### Quote Fees

```ts
const fee = await hft.quote({
	token: "0x...",       // HFT or WrappedHFT contract address
	from: senderAddress,
	to: recipientAddress, // 20-byte EVM or 32-byte substrate address
	amount: parseEther("100"),
	dest: "EVM-80002",   // Destination state machine ID
})

console.log(fee.totalNativeCost)       // msg.value needed (native token)
console.log(fee.totalFeeTokenCost)     // equivalent in host fee token
console.log(fee.relayerFeeInFeeToken)  // relayer fee component
```

Fee estimation works by:
1. Estimating gas cost for message delivery on the destination chain
2. Converting dest gas cost to dest fee token via Uniswap
3. Scaling decimals between source and dest fee tokens
4. Calling the on-chain `quote()` / `quoteNative()` methods

#### Bridge Tokens

The `bridge()` method returns an async generator that yields steps for the caller to execute:

```ts
const gen = hft.bridge({
	token: "0x...",
	from: account.address,
	to: recipientAddress,
	amount: parseEther("1"),
	dest: "EVM-80002",
	timeout: 3600n,           // optional, default 3600s
	payInFeeToken: false,     // optional, default false (pay in native)
	relayerFee: undefined,    // optional, override relayer fee (0n for self-relay)
})

let result = await gen.next()

while (!result.done) {
	const step = result.value

	if (step.type === "approve") {
		/* ERC20 approval needed (WrappedHFT or feeToken) */
		const hash = await walletClient.sendTransaction({
			to: step.tx.to,
			data: step.tx.data,
		})
		await publicClient.waitForTransactionReceipt({ hash })
		result = await gen.next()
		continue
	}

	if (step.type === "send") {
		/* The cross-chain send transaction */
		const hash = await walletClient.sendTransaction({
			to: step.tx.to,
			data: step.tx.data,
			value: step.tx.value,
		})
		result = await gen.next(hash) // resume with tx hash
		continue
	}

	if (step.type === "submitted") {
		console.log("Commitment:", step.commitment)
		result = await gen.next()
		continue
	}

	if (step.type === "status") {
		console.log("Status:", step.status)
		/* Statuses: SOURCE_FINALIZED → HYPERBRIDGE_DELIVERED →
		   HYPERBRIDGE_FINALIZED → DESTINATION */
		if (step.status === "DESTINATION") break
		result = await gen.next()
		continue
	}

	result = await gen.next()
}
```

#### Generator Steps

| Step | Description |
|------|-------------|
| `approve` | ERC20 approval tx. Yielded for WrappedHFT (underlying token) or when `payInFeeToken` is true (fee token). Only if current allowance is insufficient. |
| `send` | The cross-chain send tx. Resume the generator with the submitted tx hash. |
| `submitted` | Emitted after the send tx is mined. Contains the ISMP `commitment` hash. |
| `status` | ISMP request lifecycle updates. Only yielded if `ismpClient` was provided. |

#### Tracking with IsmpClient

To receive status updates after submission, provide an `IsmpClient`:

```ts
const hyperbridge = await SubstrateChain.connect({
	wsUrl: "wss://gargantua.rpc.polytope.technology",
	consensusStateId: "PAS0",
	hasher: "Keccak",
	stateMachineId: "KUSAMA-4009",
})

const queryClient = createQueryClient({
	url: "https://gargantua.indexer.polytope.technology",
})

const ismpClient = new IsmpClient({
	queryClient,
	source,
	dest,
	hyperbridge,
	pollInterval: 5_000,
})

const hft = new HyperFungibleToken({ source, dest, ismpClient })
```

Without `ismpClient`, the generator terminates after the `submitted` step.

#### Self-Relay

Set `relayerFee: 0n` and handle the `HYPERBRIDGE_FINALIZED` status to submit the proof calldata yourself:

```ts
const gen = hft.bridge({
	token: "0x...",
	from: account.address,
	to: account.address,
	amount: parseEther("1"),
	dest: "EVM-80002",
	relayerFee: 0n,
	payInFeeToken: true,
})

/* ... handle approve/send/submitted steps ... */

if (step.type === "status" && step.status === "HYPERBRIDGE_FINALIZED") {
	const { calldata } = step.metadata
	/* Submit calldata to the dest chain's handler contract */
	const hostParams = await destPublicClient.readContract({
		address: destHostAddress,
		abi: evmHostABI,
		functionName: "hostParams",
	})
	await destWalletClient.sendTransaction({
		to: hostParams.handler,
		data: calldata,
	})
}
```

**HyperFungibleToken Methods:**

- `isWrapped(tokenAddress)` - Detect whether a token is a WrappedHyperFungibleToken via ERC165
- `quote(params)` - Quote the cross-chain fee. Returns `{ totalNativeCost, totalFeeTokenCost, relayerFeeInFeeToken }`
- `bridge(params)` - Async generator that yields `approve`, `send`, `submitted`, and `status` steps

## Vite Integration

If you're using Vite in your project, Hyperbridge SDK includes a plugin to handle WebAssembly dependencies correctly.

### Using the Vite Plugin

```ts
// vite.config.ts
import { defineConfig } from "vite"
import hyperbridge from "@hyperbridge/sdk/plugins/vite"

export default defineConfig({
	plugins: [
		// ... other plugins
		// Add the Hyperbridge WASM plugin
		hyperbridge({
		  logLevel: "trace"
		}),
	],
})
```

The plugin automatically copies the necessary WebAssembly files to the correct location in Vite's dependency cache during development. This ensures that any WASM dependencies required by Hyperbridge SDK are properly loaded when using Vite's dev server.

## API Reference

### Classes

- **IsmpClient** - Main client for tracking ISMP requests via the indexer
- **HyperFungibleToken** - Generator-based cross-chain token bridging with fee quoting and ISMP status tracking
- **IntentGateway** - Cross-chain intent order placement and status tracking (pair with `withQueryClient(queryClient)` for indexer-backed order status)
- **EvmChain** - Utilities for EVM chain interaction
- **SubstrateChain** - Utilities for Substrate chain interaction

### Types

- RequestStatus - Enum of possible request statuses
- TimeoutStatus - Enum of possible timeout statuses
- BridgeParams - Parameters for `HyperFungibleToken.bridge()` and `quote()`
- BridgeStep - Union type yielded by the bridge generator
- QuoteResult - Fee quote returned by `HyperFungibleToken.quote()`
- HexString - Type for hex-encoded strings

### Examples

See the [HyperFungibleToken tests](/packages/sdk/src/tests/hyperFungibleToken.test.ts) and [request tracking tests](/packages/sdk/src/tests/sequential/requests.test.ts) for complete examples.
