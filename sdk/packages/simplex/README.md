# @hyperbridge/simplex

Automated intent filler for the Hyperbridge IntentGateway. Run it as a standalone binary, or embed
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

`Simplex.start` takes a plain config object — no TOML file required — and returns once the filler is
running. It logs nothing until you point `logger` at a sink, so importing the package never writes to
your stdout. Persistence is pluggable: the default store is in-memory, `SqliteDataStore` is durable, and
`SimplexDataStore` is a small async interface you can implement over Postgres, Redis or anything
else. A filler that submits bids should use a durable store, since bid records are how locked
deposits are found again for retraction.

See [Running as a library](https://docs.hyperbridge.network/developers/sdk/simplex).

## As a binary

```bash
simplex
```

With no config present, `simplex` opens a local browser wizard that walks through the minimum setup
(chains, RPCs, bundlers, signer, Hyperbridge account, strategies), validates every endpoint live,
writes a commented `filler-config.toml` (mode 600) and starts the filler in the same process.
`simplex init` is the equivalent terminal wizard.

With a config present (`./filler-config.toml`, `$SIMPLEX_HOME/config.toml`, or `-c <path>`),
`simplex` runs the filler directly.

### Docker

`polytopelabs/simplex` is published as a multi-arch manifest (`linux/amd64` + `linux/arm64`), so
Linux, macOS (Intel and Apple Silicon) and Windows hosts each pull a natively-built image. The same
commands run in bash and in PowerShell.

```bash
docker volume create simplex-data
docker run -d --name simplex --restart unless-stopped \
    -p 127.0.0.1:8686:8686 \
    -p 127.0.0.1:9090:9090 \
    -v simplex-data:/data \
    polytopelabs/simplex:latest
```

With no config in the volume this serves the setup wizard at `http://localhost:8686`, writes
`filler-config.toml` into `/data` and starts the filler in the same process; later restarts find
that config and run directly. Two alternatives to the browser wizard: mount a config you already
have (`-v /path/to/filler-config.toml:/data/filler-config.toml:ro`), or run the terminal wizard with
`docker run --rm -it -v simplex-data:/data polytopelabs/simplex:latest init -o /data/filler-config.toml`.

The container's default command binds the UI and metrics servers to `0.0.0.0` — a container-local
loopback bind is unreachable from the host, and Docker Desktop on macOS and Windows has no
`--network host` to fall back on. The container's network namespace is the boundary there, so the
ports stay private until published: keep the `127.0.0.1:` prefix above, particularly while the
wizard is up, since it collects private keys. If you override the command, carry
`--ui 0.0.0.0:8686 -p 0.0.0.0:9090` over with it.

`scripts/docker-compose.yml` brings the same up alongside Prometheus and Grafana.

### Web UI

The filler serves a local web UI at `127.0.0.1:8686` by default:

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
pnpm ui:dev           # web UI dev server with /api proxied to a running filler
```

The build emits two shapes from one source tree: the library entry points
(`dist/index.js`, `dist/sqlite.js`) leave dependencies external so a consumer resolves one copy of
`viem` and `decimal.js`, while the CLI (`dist/bin/simplex.js`) bundles everything into a single file
so a global install or the docker image runs without a dependency tree.

## License

Apache-2.0
