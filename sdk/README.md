# Hyperbridge SDK

![CI](https://github.com/polytope-labs/hyperbridge-sdk/actions/workflows/test-sdk.yml/badge.svg)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Hyperbridge SDK is a monorepo containing packages for building on Hyperbridge — a cross-chain interoperability protocol. It includes a Solidity SDK, a JavaScript/TypeScript SDK, a cross-chain message indexer, and the Intent Gateway filler.

## Packages

| Package | Description |
| --- | --- |
| [@hyperbridge/core](./packages/core) | Solidity SDK for dispatching and receiving cross-chain messages |
| [@hyperbridge/sdk](./packages/sdk) | JavaScript/TypeScript SDK for querying and monitoring cross-chain messages |
| [@hyperbridge/subql-indexer](./packages/indexer) | SubQuery-based indexer for tracking cross-chain messages |
| [@hyperbridge/simplex](./packages/simplex) | Simplex — automated market maker for cross-chain intents |

## Getting Started

### Prerequisites

- Node.js 22+
- pnpm 7+

### Installation

```bash
git clone https://github.com/polytope-labs/hyperbridge-sdk.git
cd hyperbridge-sdk
pnpm install
pnpm build
```

### Development

```bash
# Run tests
pnpm test

# Lint code
pnpm lint

# Format code
pnpm format
```

## Contributing

Create a changeset when making changes:

```bash
pnpm changeset
```

Commit your changes along with the changeset:

```bash
git add .
git commit -m "feat: your feature description"
git push
```

## License

This project is licensed under the MIT License - see the [LICENSE](/LICENSE) file for details.
