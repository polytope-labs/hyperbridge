# Hyperbridge Indexers

Multichain GraphQL API using [SubQuery](https://subquery.network) that provides access to Hyperbridge specific onchain data including but not limited to the following:

- Hyperbridge operational metrics e.g number of post requests handled, total amount of fees payed to relayers etc.
- Relayer specific information e.g total amount of fees earned, networks supported by a relayer etc

## GraphQL API

The Hyperbridge GraphQL indexer API can be found at [Hyperbridge Indexer API](https://explorer.subquery.network/subquery/polytope-labs/hyperbridge-indexers?stage=true), alongside detailed documentation

# Hyperbridge Indexer
A multi-chain indexer for the Hyperbridge Protocol that tracks cross-chain messages, assets, and protocol metrics across multiple networks.

# Overview
The Hyperbridge Indexer uses SubQuery to index and track:

- Cross-chain message delivery
- Asset transfers and teleports
- Protocol performance metrics
- Relayer activities
- Chain state updates

# Supported Networks
- Ethereum Sepolia
- Base Sepolia
- Optimism Sepolia
- Arbitrum Sepolia
- BSC Chapel
- Hyperbridge Gargantua (and other Substrate based chains)

# Architecture
The indexer runs multiple SubQuery nodes, each dedicated to a specific chain:

```mermaid
graph TD
    A[PostgreSQL Database] --> B[GraphQL API]
    C[Ethereum Node] --> D[SubQuery Node - Ethereum]
    E[Base Node] --> F[SubQuery Node - Base]
    G[Optimism Node] --> H[SubQuery Node - Optimism]
    I[Arbitrum Node] --> J[SubQuery Node - Arbitrum]
    K[BSC Node] --> L[SubQuery Node - BSC]
    M[Substrate Node] --> N[SubQuery Node - Gargantua]
    D --> A
    F --> A
    H --> A
    J --> A
    L --> A
```

# Key Features
- Multi-chain event tracking
- Asset transfer monitoring
- Protocol metrics collection
- Relayer performance tracking
- Cross-chain message indexing
- State machine updates

# Getting Started
## Prerequisites
- Docker and Docker Compose
- Node.js 16+
- NPM

## Installation
- Clone the repository:
```bash
git clone https://github.com/polytope-labs/hyperbridge-indexer.git
cd hyperbridge-indexer
```
- Install dependencies:
```bash
npm install
```
- Start the indexer:
```bash
npm run dev
```
This launches:
- PostgreSQL database
- SubQuery nodes for each chain
- GraphQL endpoint ([http://localhost:3000/graphql](http://localhost:3000/graphql))

# Contributing
- Fork the repository
- Create feature branch
- Commit changes
- Open pull request