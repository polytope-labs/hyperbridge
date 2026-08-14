# @hyperbridge/simplex

Automated intent solver for the Hyperbridge IntentGateway. Run it as a standalone binary, or embed
it in your own Node application.

Full documentation:
[docs.hyperbridge.network/developers/evm/intent-gateway/simplex](https://docs.hyperbridge.network/developers/evm/intent-gateway/simplex)

## As a library

```bash
npm install @hyperbridge/simplex
```

```ts
import { Simplex } from "@hyperbridge/simplex"
import { SqliteDataStore } from "@hyperbridge/simplex/sqlite"

const simplex = await Simplex.start({
    config,
    data: new SqliteDataStore("./simplex-data"),
})

simplex.on("order:filled", ({ orderId, profitUsd }) => {
    console.log(`filled ${orderId} for $${profitUsd}`)
})

// Every runtime control the dashboard offers is a method — nothing needs a restart.
await simplex.pairs.setCurve(0, "ask", [{ amount: "0", price: "1550" }])
await simplex.chains.setRpcUrls(8453, ["https://base-new.example"])

await simplex.stop()
```

`Simplex.start` takes a plain config object — no TOML file required — and returns once the solver is
running. It logs nothing until you point `logger` at a sink, so importing the package never writes to
your stdout. Persistence is pluggable: the default store is in-memory, `SqliteDataStore` is durable, and
`SimplexDataStore` is a small async interface you can implement over Postgres, Redis or anything
else. A solver that submits bids should use a durable store, since bid records are how locked
deposits are found again for retraction.

See [Running as a library](https://docs.hyperbridge.network/developers/sdk/simplex).

## As a binary

```bash
npm install -g @hyperbridge/simplex
simplex
```

The scope matters: the bare `simplex` name on npm is an unrelated package. The installed command is
still `simplex`. Prefer a container? The same binary ships as
[`polytopelabs/simplex`](https://hub.docker.com/r/polytopelabs/simplex) — quickstart on the Docker
Hub page.

With no config present, `simplex` opens a local browser wizard that walks through the minimum setup
(chains, RPCs, bundlers, signer, Hyperbridge account, strategies), validates every endpoint live,
writes a commented `filler-config.toml` (mode 600) and starts the solver in the same process.
`simplex init` is the equivalent terminal wizard.

With a config present (`./filler-config.toml`, `$SIMPLEX_HOME/config.toml`, or `-c <path>`),
`simplex` runs the solver directly.

### Web UI

The solver serves a local web UI at `127.0.0.1:8686` by default:

- setup wizard (when no config exists) — private key, MPCVault or Turnkey signer, static curves or Uniswap V4 pool pricing
- status, pause/resume (persists across restarts), graceful stop, balances per chain
- live activity feed (orders detected/filled/skipped, bids, rebalances) streamed over SSE
- operations: manual vault sweep/redeem, runtime allowlist editing, log level switch, rebalancing trigger view, masked config view
- inflight FX price curve updates without a restart, persisted back to the config file
- overfill-protection self-halts surfaced with an operator reset

Flags:

```bash
simplex run -c filler-config.toml            # UI on 127.0.0.1:8686
simplex run -c filler-config.toml --ui 9000  # custom port
simplex run -c filler-config.toml --no-ui    # headless
```

The curve-update API:

```bash
curl http://127.0.0.1:8686/api/strategies
curl -X PUT http://127.0.0.1:8686/api/strategies/0/curves \
    -H "Content-Type: application/json" -H "X-Simplex-UI: 1" \
    -d '{"askPriceCurve": [{"amount": "0", "price": "1550"}]}'
```

Curve changes apply immediately and are written back to the config file (regenerated with standard
comments) so restarts keep them. Venue-priced strategies and disabled sides (one-sided LP) are not
editable. The server is unauthenticated — mutating requests need the `X-Simplex-UI: 1` header (CSRF
hygiene), and both the wizard and the operator UI bind loopback unless told otherwise. Only bind
another interface (e.g. `--ui 0.0.0.0:8686`, which the docker image does inside its own network
namespace) on a trusted network.

## Development

```bash
pnpm install
pnpm build            # library + CLI bundles, then the vite web UI into dist/ui
pnpm test
pnpm cli run -c filler-config.toml
pnpm ui:dev           # web UI dev server with /api proxied to a running solver
```

The build emits two shapes from one source tree: the library entry points
(`dist/index.js`, `dist/sqlite.js`) leave dependencies external so a consumer resolves one copy of
`viem` and `decimal.js`, while the CLI (`dist/bin/simplex.js`) bundles everything into a single file
so a global install or the docker image runs without a dependency tree.

## License

Apache-2.0
