# Hyperbridge Simplex

Automated intent filler for the Hyperbridge IntentGatewayV2.

Full documentation: [docs.hyperbridge.network/developers/intent-gateway/simplex](https://docs.hyperbridge.network/developers/intent-gateway/simplex)

## Quick start

```bash
simplex
```

With no config present, `simplex` opens a local browser wizard that walks through the
minimum setup (chains, RPCs, bundlers, signer, Hyperbridge account, strategies),
validates every endpoint live, writes a commented `filler-config.toml` (mode 600) and
starts the filler in the same process. `simplex init` is the equivalent terminal wizard.

With a config present (`./filler-config.toml`, `$SIMPLEX_HOME/config.toml`, or `-c <path>`),
`simplex` runs the filler directly.

## Web UI

The filler serves a local web UI at `127.0.0.1:8686` by default:

- setup wizard (when no config exists)
- status, pause/resume (persists across restarts), balances per chain
- inflight FX price curve updates without a restart

Flags:

```bash
simplex run -c filler-config.toml            # UI on 127.0.0.1:8686
simplex run -c filler-config.toml --ui 9000  # custom port
simplex run -c filler-config.toml --no-ui    # headless
```

`--admin-port` is replaced by `--ui`; the curve-update API is unchanged:

```bash
curl http://127.0.0.1:8686/api/strategies
curl -X PUT http://127.0.0.1:8686/api/strategies/0/curves \
    -H "Content-Type: application/json" -H "X-Simplex-UI: 1" \
    -d '{"askPriceCurve": [{"amount": "0", "price": "1550"}]}'
```

Curve changes apply immediately but are lost on restart; the TOML config is re-read on
boot. Venue-priced strategies and disabled sides (one-sided LP) are not editable. The
server is unauthenticated — mutating requests need the `X-Simplex-UI: 1` header (CSRF
hygiene), and the setup wizard only ever binds loopback. Only bind another interface
(e.g. `--ui 0.0.0.0:8686` in docker) on a trusted network.

## Development

```bash
pnpm install
pnpm build            # tsup bundle + vite web UI into dist/ui
pnpm test
pnpm cli run -c filler-config.toml
pnpm ui:dev           # web UI dev server with /api proxied to a running filler
```
