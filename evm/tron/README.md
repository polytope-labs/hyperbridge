# Hyperbridge TRON Deployment

This directory contains the [TronBox](https://developers.tron.network/docs/tronbox) project for deploying Hyperbridge ISMP contracts to the **TRON** network.

## Contracts Deployed

| Contract | Description | Nile Testnet Address |
|---|---|---|
| **BeefyV1FiatShamir** | Fiat-Shamir sampled BEEFY consensus client | [`TH8oxJmHaLSXVHxvM7s4bEEED1yHPG9PYy`](https://nile.tronscan.org/#/contract/TH8oxJmHaLSXVHxvM7s4bEEED1yHPG9PYy) |
| **ConsensusRouter** | Consensus proof router (only FiatShamir active on TRON) | [`TPXBv4ALJKKRPSVDKygPja4EoE25e96yew`](https://nile.tronscan.org/#/contract/TPXBv4ALJKKRPSVDKygPja4EoE25e96yew) |
| **HandlerV1** | ISMP message handler — verifies cryptographic proofs for cross-chain messages | [`TVu5adsYw2CRHrgXwKKdfPR1qYdtA4hFRG`](https://nile.tronscan.org/#/contract/TVu5adsYw2CRHrgXwKKdfPR1qYdtA4hFRG) |
| **HostManager** | Cross-chain governance module for updating host params and withdrawing revenue | [`TUyim7389oUZVyZBz1cpFJ63BBWwBEeFKe`](https://nile.tronscan.org/#/contract/TUyim7389oUZVyZBz1cpFJ63BBWwBEeFKe) |
| **TronHost** | The `IsmpHost` and `IsmpDispatcher` implementation for TRON (`chainId = 728126428`) | [`TNduR7v184pMWv2oTamRxxzsmz7oHrKqJc`](https://nile.tronscan.org/#/contract/TNduR7v184pMWv2oTamRxxzsmz7oHrKqJc) |
| **CallDispatcher** | Utility for dispatching untrusted external calls | [`TA9XyBPuXL9ecXcLpcFV1g778fzstke2Eh`](https://nile.tronscan.org/#/contract/TA9XyBPuXL9ecXcLpcFV1g778fzstke2Eh) |
| **IntentGatewayV2** | Intent-based cross-chain order creation and fulfillment | [`TT4CjjHw7QgLbE9wKtYEopid1YqePkbAfb`](https://nile.tronscan.org/#/contract/TT4CjjHw7QgLbE9wKtYEopid1YqePkbAfb) |

## Prerequisites

- **Node.js** ≥ 18
- **TronBox** (installed as a local dependency)
- A funded TRON account (TRX for energy/bandwidth)

## Quick Start

### 1. Install dependencies

```sh
npm install
```

> This will install `tronbox`, `tronweb`, and all Solidity dependencies that mirror the parent Foundry project.

### 2. Configure environment

```sh
cp .env.example .env
```

Edit `.env` and fill in at minimum:

| Variable | Description |
|---|---|
| `PRIVATE_KEY` | Hex-encoded private key of the deployer account (no `0x` prefix) |
| `ADMIN` | Admin address that will govern the deployment |
| `FEE_TOKEN` | TRC-20 fee token address (e.g. USDT on TRON) |
| `PARA_ID` | Hyperbridge parachain ID |
| `CONSENSUS_STATE` | Hex-encoded initial BEEFY consensus state |
| `NETWORK` | `mainnet` or `testnet` — controls the hyperbridge state machine encoding |

### 3. Compile contracts

```sh
npx tronbox compile
```

Compiled artifacts are written to `build/contracts/`.

### 4. Deploy

**Shasta testnet:**

```sh
npx tronbox migrate --network shasta
```

**Nile testnet:**

```sh
npx tronbox migrate --network nile
```

**Mainnet:**

```sh
npx tronbox migrate --network mainnet
```

### 5. Interactive console

```sh
npx tronbox console --network shasta
```

Inside the console you can interact with deployed contracts:

```js
const host = await TronHost.deployed();
const chainId = await host.chainId();
console.log("Chain ID:", chainId.toString());
```

### 6. Flatten contracts for verification

To flatten contracts for TronScan verification:

```sh
# Flatten all contracts
npm run flatten

# Flatten a specific contract
npm run flatten TronHost
```

This creates individual flattened files in `flattened/` directory:
- `TronHost_flat.sol`
- `HandlerV1_flat.sol`
- `HostManager_flat.sol`
- `BeefyV1FiatShamir_flat.sol`
- `ConsensusRouter_flat.sol`
- `CallDispatcher_flat.sol`
- `IntentGatewayV2_flat.sol`

**To verify a contract on TronScan:**

1. Navigate to your deployed contract:
   - Nile: https://nile.tronscan.org/#/contract/`<address>`/code
   - Shasta: https://shasta.tronscan.org/#/contract/`<address>`/code
   - Mainnet: https://tronscan.org/#/contract/`<address>`/code

2. Click "Verify Contract"

3. Fill in the verification form:
   - **Contract Name**: Match the deployed contract (e.g., `TronHost`)
   - **Compiler Version**: `0.8.24`
   - **Optimization**: `Enabled`
   - **Optimization Runs**: `200`
   - **EVM Version**: `paris`

4. Upload the corresponding flattened file (e.g., `flattened/TronHost_flat.sol`)

5. Submit for verification

## Deployment Order & Wiring

The migration script (`migrations/2_deploy_ismp.js`) handles the full deployment lifecycle:

```
BeefyV1FiatShamir
       │
       ▼
ConsensusRouter(address(0), address(0), beefyV1FiatShamir)
       │
       ▼
HandlerV1
       │
       ▼
HostManager(admin, address(0))
       │
       ▼
TronHost(hostParams)
       │
       ├──► HostManager.setIsmpHost(tronHost)
       │
       ├──► TronHost.setConsensusState(...)   [if CONSENSUS_STATE is set]
       │
       ▼
CallDispatcher
       │
       ▼
IntentGatewayV2(admin)
       │
       └──► IntentGatewayV2.setParams(host, dispatcher, ...)
```

The `ConsensusRouter` is deployed with `address(0)` for both `sp1Beefy` and `beefyV1` since only the **BeefyV1FiatShamir** consensus client is used on TRON. Attempting to submit a proof with type `Naive` (0x00) or `ZK` (0x01) will revert.

## Project Structure

```
tron/
├── contracts/
│   ├── Migrations.sol              # TronBox migration tracker
│   └── deploy/
│       └── TronContracts.sol       # Import hub — pulls all contracts from ../src/
├── migrations/
│   ├── 1_initial_migration.js      # Deploys Migrations.sol
│   └── 2_deploy_ismp.js           # Deploys & wires all Hyperbridge contracts
├── build/                          # Compiled artifacts (gitignored)
├── .env.example                    # Environment variable template
├── package.json
├── tronbox.js                      # TronBox configuration
└── README.md
```

## Compiler Configuration

| Setting | Value | Rationale |
|---|---|---|
| Solidity version | `0.8.24` | Latest version supported by TronBox's solc |
| EVM version | `paris` | TRON's TVM is compatible up to the Paris hard fork |
| Optimizer | Enabled, 200 runs | Balance between deployment cost and runtime gas |
| Remappings | Configured in `tronbox.js` | Points to the parent project's `node_modules/` and `lib/` |

## Network Configuration

| Network | Full Node | Network ID |
|---|---|---|
| `development` | `http://127.0.0.1:9090` | `*` |
| `shasta` | `https://api.shasta.trongrid.io` | `2` |
| `nile` | `https://nile.trongrid.io` | `3` |
| `mainnet` | `https://api.trongrid.io` | `1` |

## Notes

- **Energy & Bandwidth**: Deploying these contracts requires significant energy. Make sure your account has enough TRX staked for energy, or enough TRX to burn. The `fee_limit` in `tronbox.js` is set to 15,000 TRX by default.
- **Address Format**: TronBox automatically handles conversion between hex addresses (used in Solidity) and TRON's base58 addresses (starting with `T`). Environment variables can use either format.
- **Fee Token**: On TRON mainnet, this is typically USDT (`TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t`). Set `FEE_TOKEN_DECIMALS=6` accordingly.
- **SunSwap V2**: If you need token swaps for fee conversion, set `UNISWAP_V2` to the SunSwap V2 Router address.