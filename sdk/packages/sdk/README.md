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
import { IndexerClient, createQueryClient, EvmChain, SubstrateChain } from "@hyperbridge/sdk"

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

// Create the IndexerClient
const indexer = new IndexerClient({
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

### TokenGateway - Cross-Chain Token Transfers

The TokenGateway class provides methods for estimating fees and managing cross-chain token teleports via Hyperbridge. Supports both EVM and Substrate chains as destination.

```ts
import { TokenGateway, EvmChain, SubstrateChain } from "@hyperbridge/sdk"
import { keccak256, toHex, pad, parseEther } from "viem"

// Create chain instances
const sourceChain = new EvmChain({
	chainId: 97, // BSC Testnet
	rpcUrl: "https://data-seed-prebsc-1-s1.binance.org:8545",
	host: "0x...", // IsmpHost contract address
	consensusStateId: "BSC0"
})

const destChain = new EvmChain({
	chainId: 10200, // Gnosis Chiado
	rpcUrl: "https://rpc.chiadochain.net",
	host: "0x...", // IsmpHost contract address
	consensusStateId: "GNO0"
})

// Initialize TokenGateway (destination can be EvmChain or SubstrateChain)
const tokenGateway = new TokenGateway({
	source: sourceChain,
	dest: destChain // EvmChain or SubstrateChain
})

// Estimate fees for a teleport
const assetId = keccak256(toHex("USDC")) // Asset identifier
const recipientAddress = pad("0xRecipientAddress", { size: 32 })

const teleportParams = {
	amount: parseEther("100"), // Amount to teleport
	assetId: assetId,
	redeem: true, // Redeem as ERC20 on destination
	to: recipientAddress,
	dest: "EVM-10200", // Destination chain
	timeout: 3600n, // Timeout in seconds
	data: "0x" // Optional call data
}

// Get native cost estimate (protocol + relayer fees)
// For EVM destination chains, the relayer fee is automatically estimated by:
// 1. Creating a dummy post request with 191 bytes of random data
// 2. Estimating gas for delivery on the destination chain
// 3. Converting gas cost to native tokens and adding 1% buffer
// 4. Converting relayer fee to source fee token using getAmountsOut
// For Substrate destination chains, relayer fee is set to zero
// Returns: totalNativeCost (protocol fee with 1% buffer) and relayerFeeInSourceFeeToken
const { totalNativeCost, relayerFeeInSourceFeeToken } = await tokenGateway.quoteNative(teleportParams)
console.log(`Total native cost: ${totalNativeCost} wei`)
console.log(`Relayer fee in fee token: ${relayerFeeInSourceFeeToken}`)

// Example with Substrate destination
const substrateDestChain = new SubstrateChain({
	stateMachineId: "KUSAMA-4009",
	wsUrl: "wss://gargantua.polytope.technology",
	hasher: "Keccak",
	consensusStateId: "PAS0"
})

const tokenGatewayToSubstrate = new TokenGateway({
	source: sourceChain,
	dest: substrateDestChain // SubstrateChain destination
})

// For Substrate destinations, relayer fee will be 0
const { totalNativeCost: substrateCost, relayerFeeInSourceFeeToken: substrateRelayerFee } = 
	await tokenGatewayToSubstrate.quoteNative({
		amount: parseEther("100"),
		assetId: assetId,
		redeem: true,
		to: recipientAddress,
		dest: "KUSAMA-4009",
		timeout: 3600n
	})
console.log(`Substrate destination - Native cost: ${substrateCost} wei`)
console.log(`Substrate destination - Relayer fee: ${substrateRelayerFee}`) // Will be 0

// Get token addresses
const erc20Address = await tokenGateway.getErc20Address(assetId)
const erc6160Address = await tokenGateway.getErc6160Address(assetId)

// Get gateway parameters
const params = await tokenGateway.getParams()
console.log(`Host: ${params.host}, Dispatcher: ${params.dispatcher}`)
```

**TokenGateway Methods:**

- `quoteNative(params)` - Estimate native token cost for a teleport operation. For EVM destination chains, the relayer fee is automatically estimated by generating a dummy post request with 191 bytes of random data, estimating gas on the destination chain, converting to native tokens, and adding a 1% buffer to the relayer fee. The relayer fee is then converted to source chain fee token using Uniswap V2's `getAmountsOut`. For Substrate destinations, relayer fee is set to zero. Returns an object with `totalNativeCost` (relayer fee + protocol fee, both with 1% buffers) and `relayerFeeInSourceFeeToken` (relayer fee converted to source chain fee token).
- `getErc20Address(assetId)` - Get the ERC20 contract address for an asset
- `getErc6160Address(assetId)` - Get the ERC6160 (hyper-fungible) contract address for an asset
- `getInstanceAddress(destination)` - Get the TokenGateway address on the destination chain
- `getParams()` - Get the TokenGateway contract parameters (host and dispatcher addresses)

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

- **IndexerClient** - Main client for interacting with the indexer
- **EvmChain** - Utilities for EVM chain interaction
- **SubstrateChain** - Utilities for Substrate chain interaction
- **TokenGateway** - Utilities for cross-chain token transfers and fee estimation

### Types

- RequestStatus - Enum of possible request statuses
- TimeoutStatus - Enum of possible timeout statuses
- HexString - Type for hex-encoded strings

### Examples

See the tests [directory](/packages/sdk/src/tests/postRequest.test.ts) for complete examples.
