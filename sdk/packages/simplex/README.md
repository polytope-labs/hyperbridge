# @hyperbridge/simplex

Automated intent solver for the Hyperbridge IntentGateway. Run it as a standalone binary, or embed
it in your own Node application.

Full documentation:
[docs.hyperbridge.network/developers/sdk/simplex](https://docs.hyperbridge.network/developers/sdk/simplex/)

## As a library

```bash
npm install @hyperbridge/simplex
```

```ts
import { Simplex, privateKeySigner } from "@hyperbridge/simplex"
import { SqliteDataStore } from "@hyperbridge/simplex/sqlite"

const simplex = await Simplex.start({
    config,
    signer: privateKeySigner(process.env.SOLVER_KEY as `0x${string}`),
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
your stdout.

Signing is an interface, not a setting. `Signer` is an identity and three operations — sign this
typed data, this EIP-7702 authorization, this transaction — with no viem types on it, so satisfying
it never means matching this package's viem version. `privateKeySigner`,
`turnkeySigner` and `mpcVaultSigner` ship with the package, `viemSigner` adapts any viem account (a
`toAccount` wrapper around an HSM or a remote signing service included), and your own implementation
is a first-class citizen. Persistence is pluggable the same way: the default
store is in-memory, `SqliteDataStore` is durable, and `SimplexDataStore` is a small async interface
you can implement over Postgres, Redis or anything else. A solver that submits bids should use a
durable store, since bid records are how locked deposits are found again for retraction.

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

## Remote access from a phone

The UI is loopback-only, and most operator machines sit behind NAT. Remote access keeps an
outbound SSH tunnel from simplex to a relay ([polytope-labs/simplex-tunnel](https://github.com/polytope-labs/simplex-tunnel),
hosted at `simplex.tunnel.polytope.technology`) that leases this simplex a stable public port.
A phone's SSH client connects to that port with a local port forward, and the browser opens
`http://localhost:8686`.

The phone's SSH session terminates in an SSH server embedded in simplex, not in sshd and not in
the relay, so the relay only ever carries ciphertext. That server accepts public-key auth against
the devices paired in the UI and `direct-tcpip` channels to the UI bind, and nothing else: no
shell, exec, PTY or other destinations.

It is off by default. Turn it on and pair devices under **Operations > Remote access** in the UI;
the choice is written to `[simplex.tunnel]` in the config. To pair, create a key in the phone's SSH
app and paste its public key into the panel, so the private key never leaves the phone; for apps
that cannot make their own, simplex can generate a pair instead and shows the private key exactly
once, as text and as a QR code. Either way the panel shows the host, port, username, host-key
fingerprint and local forward to enter in the SSH app (Blink and Termius on iOS, ConnectBot and
JuiceSSH on Android). A paired key opens the whole dashboard, including the
Send and treasury tools, so keep it on the device and revoke it from the same panel if the
device is lost. Keys live under `<data-dir>/tunnel/` in plain OpenSSH formats.

The hosted relay's host key is pinned in the binary, so first contact is verified. A self-hosted
relay is pinned on first contact unless `relayHostKey` in `[simplex.tunnel]` names its fingerprint.

The tunnel is best-effort: it retries with backoff and never affects filling. It only runs in
operator mode, never while the setup wizard holds secrets.

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
