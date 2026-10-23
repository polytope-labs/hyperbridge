# @hyperbridge/simplex

[![npm](https://img.shields.io/npm/v/@hyperbridge/simplex.svg)](https://www.npmjs.com/package/@hyperbridge/simplex)
[![node](https://img.shields.io/node/v/@hyperbridge/simplex.svg)](https://nodejs.org)
[![license](https://img.shields.io/npm/l/@hyperbridge/simplex.svg)](https://github.com/polytope-labs/hyperbridge/blob/main/sdk/packages/simplex/LICENSE)

An automated solver for [Hyperbridge](https://hyperbridge.network) intents.

Simplex watches the IntentGateway on the chains you configure. It prices each swap against the
limit orders you post to the HyperFX orderbook, then bids for and fills the swaps you have priced.

Embed it in your own Node service, or run it as a ready-made app:

- **Library**: call `Simplex.start()` in your process, with your own signer and storage.
- **CLI**: `simplex` sets itself up in the browser and serves a local dashboard.
- **Desktop app**: installers for macOS, Windows and Linux.
- **Docker**: the [`polytopelabs/simplex`](https://hub.docker.com/r/polytopelabs/simplex) image.

## Install

```bash
npm install @hyperbridge/simplex
```

Requires Node.js 22.23.4+, 24.22.1+ or 26.12.1+.

## Quick start

```ts
import { Simplex, privateKeySigner, type SimplexConfig } from "@hyperbridge/simplex"
import { SqliteDataStore } from "@hyperbridge/simplex/sqlite"

const config: SimplexConfig = {
    simplex: {
        maxConcurrentOrders: 5,
        substratePrivateKey: process.env.SUBSTRATE_KEY!,
        hyperbridgeWsUrl: "wss://nexus.rpc.polytope.technology",
    },
    chains: [{ rpcUrls: ["https://base.example"] }, { rpcUrls: ["https://ethereum.example"] }],
    orderbook: { url: "https://orderbook.hyperfx.finance/mainnet/graphql" },
}

const simplex = await Simplex.start({
    config,
    signer: privateKeySigner(process.env.SOLVER_KEY as `0x${string}`),
    data: new SqliteDataStore("./simplex-data"),
    logger: process.stdout,
})

// Pay out 13,900,000 cNGN on Base for 10,000 USDC, to swaps from Ethereum or Base.
await simplex.limitOrders.create({
    fillChain: "EVM-8453",
    tokenIn: "USDC",
    amountIn: "10000",
    tokenOut: "CNGN",
    amountOut: "13900000",
    acceptedSources: ["EVM-1", "EVM-8453"],
})

simplex.on("order:filled", ({ orderId, profitUsd }) => {
    console.log(`filled ${orderId} for $${profitUsd}`)
})

// When you are done:
await simplex.stop()
```

`Simplex.start` resolves once the solver is running. The config holds no prices. Simplex fills only
against the limit orders you create, so a fresh solver quotes nothing until you post one.

## Features

- **Priced by your limit orders.** No price feeds. Each order says what you take in, what you pay
  out, and on which chain.
- **Bring your own signer.** `Signer` is a small interface with no viem types. Simplex ships
  `privateKeySigner`, `secretPhraseSigner`, `turnkeySigner` and `mpcVaultSigner`, and `viemSigner`
  wraps any viem account.
- **Bring your own storage.** The default store is in memory. `SqliteDataStore` is durable, and
  `SimplexDataStore` is a small interface you can implement over any database. Use a durable store
  when bidding, because bid records are how locked deposits are found again.
- **Live control.** Limit orders, chains, vaults, assets and rebalancing all change at runtime
  through `simplex.limitOrders`, `simplex.chains` and the other controllers. Nothing needs a restart.
- **Typed events.** Subscribe to `order:detected`, `order:filled`, `limit-order:filled` and more.
- **Quiet by default.** Simplex logs nothing until you pass `logger`, so it never writes to your
  stdout uninvited.

## Command line

```bash
npm install -g @hyperbridge/simplex
simplex
```

Install the scoped package. The bare `simplex` package on npm is unrelated.

With no config, `simplex` opens a setup wizard in your browser. The wizard writes
`filler-config.toml` and starts the solver. With a config, `simplex` starts the solver and serves its
dashboard at `http://127.0.0.1:8686`.

```bash
simplex run -c filler-config.toml           # run with an existing config
simplex run -c filler-config.toml --no-ui   # run without the dashboard
simplex init                                # set up in the terminal instead
```

The same binary runs in Docker as
[`polytopelabs/simplex`](https://hub.docker.com/r/polytopelabs/simplex).

## Desktop app

Simplex Desktop bundles this package with its own Node runtime. It runs the setup wizard on first
launch and keeps the solver running from the system tray.
[Download it for macOS, Windows or Linux](https://docs.hyperbridge.network/developers/evm/simplex/installation/).

## Security

- `filler-config.toml` holds private keys in plain text. It is written with mode `0600`, but keep it
  out of source control and shared backups.
- The dashboard has no login. It listens only on `127.0.0.1` by default. Do not expose it on a
  network you do not trust. To reach it from a phone, use
  [remote access](https://docs.hyperbridge.network/developers/evm/simplex/api/remote-access/).

## Documentation

- [Library guide](https://docs.hyperbridge.network/developers/sdk/simplex/)
- [API reference](https://docs.hyperbridge.network/developers/sdk/api/simplex/)
- [Configuration](https://docs.hyperbridge.network/developers/evm/simplex/configuration/)
- [Limit orders](https://docs.hyperbridge.network/developers/evm/simplex/limit-orders/)
- [Dashboard](https://docs.hyperbridge.network/developers/evm/simplex/dashboard/) and
  [HTTP API](https://docs.hyperbridge.network/developers/evm/simplex/api/overview/)
- [Troubleshooting](https://docs.hyperbridge.network/developers/evm/simplex/troubleshooting/)

## Development

This package lives in the [Hyperbridge monorepo](https://github.com/polytope-labs/hyperbridge) at
`sdk/packages/simplex`.

```bash
pnpm install
pnpm build   # library, CLI and dashboard
pnpm test
```

## License

[Apache-2.0](https://github.com/polytope-labs/hyperbridge/blob/main/sdk/packages/simplex/LICENSE)
