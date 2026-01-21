# Hyperbridge Intent FillerV2

A high-performance intent filler for the Hyperbridge IntentGatewayV2 protocol. This package provides both a library interface and a CLI binary for running an intent filler that monitors and fills cross-chain orders.

## Installation

### Binary

```bash
npm install -g @hyperbridge/filler-v2
# or
pnpm add -g @hyperbridge/filler-v2
```

### Library

```bash
npm install @hyperbridge/filler-v2
# or
pnpm add @hyperbridge/filler-v2
```

## Quick Start

### 1. Create Configuration

Copy the example configuration file and customize it:

```bash
cp filler-config-example.toml filler-config.toml
```

### 2. Edit Configuration

Update `filler-config.toml` with:

- Your EVM private key
- RPC URLs for each chain you want to support
- Confirmation policies for each chain
- (Optional) Solver selection mode settings for Hyperbridge integration

### 3. Run the FillerV2

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

The filler uses a TOML configuration file. See `filler-config-example.toml` for a complete example.

### Basic Configuration

```toml
[filler]
privateKey = "0xYourPrivateKey"
maxConcurrentOrders = 5

# Logging configuration
[filler.logging]
level = "debug"  # Options: trace, debug, info, warn, error

# Pending queue configuration
[filler.pendingQueue]
maxRechecks = 10
recheckDelayMs = 30000

# Strategy configuration
[[strategies]]
type = "basic"

# Chain configurations (only chainId and rpcUrl required - other data from SDK)
[[chains]]
chainId = 1  # Ethereum Mainnet
rpcUrl = "https://your-eth-rpc-url"

[[chains]]
chainId = 56  # BSC Mainnet
rpcUrl = "https://your-bsc-rpc-url"

[[chains]]
chainId = 42161  # Arbitrum Mainnet
rpcUrl = "https://your-arbitrum-rpc-url"

# Confirmation policies per chain
[confirmationPolicies."1"]  # Ethereum Mainnet
minAmount = "5"       # 5 USD
maxAmount = "5000"    # 5000 USD
minConfirmations = 3
maxConfirmations = 12

[confirmationPolicies."56"]  # BSC Mainnet
minAmount = "1"       # 1 USD
maxAmount = "5000"    # 5000 USD
minConfirmations = 3
maxConfirmations = 15
```

### Watch-Only Mode

Monitor orders without executing fills. Useful for testing or observing market activity.

```toml
# Option 1: Global watch-only (all chains)
[filler]
watchOnly = true

# Option 2: Per-chain watch-only
[filler.watchOnly]
"1" = true    # Ethereum Mainnet - watch only
"56" = false  # BSC Mainnet - normal execution
```

### Solver Selection Mode

For participating in Hyperbridge's solver selection mechanism:

```toml
[filler]
privateKey = "0xYourEVMPrivateKey"

# Substrate private key for signing Hyperbridge extrinsics
# Can be a hex seed (without 0x prefix) or mnemonic phrase
# Note: Requires BRIDGE tokens for transaction fees
substratePrivateKey = "your-substrate-seed-or-mnemonic"

# Hyperbridge WebSocket URL
hyperbridgeWsUrl = "wss://hyperbridge-rpc-url"

# ERC-4337 EntryPoint contract address
entryPointAddress = "0x..."

# SolverAccount.sol contract address for EIP-7702 delegation
solverAccountContractAddress = "0x..."

# Directory for persistent bid storage (enables fund recovery)
dataDir = "/path/to/data"
```

## CLI Commands

```bash
# Run the filler with configuration
filler run -c <config-file>
```

## Strategies

### Basic Filler

- Direct token transfers between chains
- No swapping capability
- Lower gas costs
- Recommended for standard cross-chain fills

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

## Data Storage

The filler stores bid transaction hashes for fund recovery purposes. By default, data is stored in `.filler-data` in the current working directory. You can customize this with the `dataDir` configuration option.

## Security

⚠️ **Never commit private keys to version control!**

- Use environment variables or secure key management in production
- Run fillers in isolated environments
- Monitor for unusual activity
- Keep your Substrate account funded with BRIDGE tokens for solver selection mode

## License

Part of the Hyperbridge SDK.
