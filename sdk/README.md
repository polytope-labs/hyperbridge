# Hyperbridge SDK

![CI](https://github.com/polytope-labs/hyperbridge-sdk/actions/workflows/test-sdk.yml/badge.svg)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Hyperbridge SDK is a comprehensive solution for cross-chain message indexing and retrieval. This monorepo contains two main packages:

- **@hyperbridge/indexer**: A SubQuery-based indexer for tracking cross-chain messages
- **@hyperbridge/sdk**: A JavaScript/TypeScript SDK for interacting with the indexed data

## Packages

| Package                                    | Description                                                        |
| ------------------------------------------ | ------------------------------------------------------------------ |
| [@hyperbridge/indexer](./packages/indexer) | The indexer service that processes and stores cross-chain messages |
| [@hyperbridge/sdk](./packages/sdk)         | SDK for developers to query and monitor cross-chain messages       |

## Getting Started

### Prerequisites

- Node.js 22+
- pnpm 7+

### Installation

```bash
# Clone the repository
git clone https://github.com/polytope-labs/hyperbridge-sdk.git
cd hyperbridge-sdk

# Install dependencies
pnpm install

# Build all packages
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

Commit your changes along with the changeset:

git add .
git commit -m "feat: your feature description"
git push
```

## License
This project is licensed under the MIT License - see the [LICENSE](/LICENSE) file for details.

## Acknowledgments
- [SubQuery](https://subquery.network) - The indexing framework
- [Polkadot](https://polkadot.com) - The interoperability protocol
