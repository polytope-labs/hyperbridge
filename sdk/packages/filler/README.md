# Hyperbridge Intent Filler

A high-performance intent filler for the Hyperbridge IntentGateway protocol. This package provides both a library interface and a CLI binary for running an intent filler that monitors and fills cross-chain orders.

## Installation

### Binary

```bash
npm install -g @hyperbridge/filler
# or
pnpm add -g @hyperbridge/filler
```

### Library

```bash
npm install @hyperbridge/filler
# or
pnpm add @hyperbridge/filler
```

## Quick Start

### 1. Generate Configuration

```bash
filler init -o filler-config.toml
```

### 2. Edit Configuration

Update `filler-config.toml` with:

- Your private key
- Chain configurations (chainId, rpcUrl, intentGatewayAddress)
- Confirmation policies for each chain

### 3. Run the Filler

```bash
filler run -c filler-config.toml
```

## Docker Usage

We provide a simple script for Docker operations:

```bash
# Build Docker image
./scripts/docker.sh build

# Run as container
./scripts/docker.sh run

# Use Docker Compose
./scripts/docker.sh up
./scripts/docker.sh down
./scripts/docker.sh logs
```

## Configuration

The filler uses a TOML configuration file:

```toml
[filler]
privateKey = "0xYourPrivateKey"
maxConcurrentOrders = 5

[filler.pendingQueue]
maxRechecks = 10
recheckDelayMs = 30000

[[strategies]]
type = "basic"
privateKey = "0xYourPrivateKey"

# Chain configurations
[[chains]]
chainId = 97
rpcUrl = "https://bsc-testnet.public.blastapi.io"
intentGatewayAddress = "0xFC91c1932F70D36E35Ae7F622cE6C8B86CCeE8e9"

[[chains]]
chainId = 10200
rpcUrl = "https://rpc.chiadochain.net"
intentGatewayAddress = "0xFC91c1932F70D36E35Ae7F622cE6C8B86CCeE8e9"

[confirmationPolicies."97"]
minAmount = "1000000000000000000"
maxAmount = "1000000000000000000000"
minConfirmations = 1
maxConfirmations = 5

[confirmationPolicies."10200"]
minAmount = "1000000000000000000"
maxAmount = "1000000000000000000000"
minConfirmations = 1
maxConfirmations = 5
```

## CLI Commands

- `filler init` - Generate a sample configuration file
- `filler validate -c <config>` - Validate a configuration file
- `filler run -c <config>` - Run the filler with the specified configuration

## Library Usage

```typescript
import { IntentFiller, BasicFiller } from "@hyperbridge/filler"

// Configure chains
const chainConfigs = [
	{
		chainId: 97,
		rpcUrl: "https://bsc-testnet.public.blastapi.io",
		intentGatewayAddress: "0xFC91c1932F70D36E35Ae7F622cE6C8B86CCeE8e9",
	},
	{
		chainId: 10200,
		rpcUrl: "https://rpc.chiadochain.net",
		intentGatewayAddress: "0xFC91c1932F70D36E35Ae7F622cE6C8B86CCeE8e9",
	},
]

// Configure filler
const fillerConfig = {
	confirmationPolicy: {
		getConfirmationBlocks: (chainId, amount) => 1,
	},
	maxConcurrentOrders: 5,
	pendingQueueConfig: {
		maxRechecks: 10,
		recheckDelayMs: 30000,
	},
}

// Initialize strategies
const strategies = [new BasicFiller("0xYourPrivateKey")]

// Create and start filler
const intentFiller = new IntentFiller(chainConfigs, strategies, fillerConfig)
intentFiller.start()

// Listen to events
const monitor = intentFiller.monitor
monitor.on("newOrder", (data) => console.log("New order:", data.order))
monitor.on("orderFilled", (data) => console.log("Order filled:", data.orderId))

// Stop when done
intentFiller.stop()
```

## Strategies

### Basic Filler

- Direct token transfers between chains
- No swapping capability
- Lower gas costs

### Stable Swap Filler

- Supports token swapping via Uniswap V2
- Can capture arbitrage opportunities
- Higher gas costs

## Development

```bash
# Install dependencies
pnpm install

# Build
pnpm build

# Run tests
pnpm test

# Run CLI in development
pnpm cli run -c filler-config.toml
```

## Security

⚠️ **Never commit private keys to version control!**

- Use environment variables or secure key management in production
- Run fillers in isolated environments
- Monitor for unusual activity

## License

Part of the Hyperbridge SDK.
