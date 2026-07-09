# Hyperbridge Simplex

Automated intent filler for the Hyperbridge IntentGatewayV2.

Full documentation: [docs.hyperbridge.network/developers/intent-gateway/simplex](https://docs.hyperbridge.network/developers/intent-gateway/simplex)

## Development

```bash
pnpm install
pnpm build
pnpm test
pnpm cli run -c filler-config.toml
```

## Admin server

Pass `--admin-port <[host:]port>` to `simplex run` to serve a local UI and JSON API for updating FX price curves in memory, without restarting the filler:

```bash
simplex run -c filler-config.toml --admin-port 8686
```

Open `http://127.0.0.1:8686/` to edit the bid/ask curves, or use the API directly:

```bash
curl http://127.0.0.1:8686/api/strategies
curl -X PUT http://127.0.0.1:8686/api/strategies/0/curves \
    -H "Content-Type: application/json" \
    -d '{"askPriceCurve": [{"amount": "0", "price": "1550"}]}'
```

Changes apply immediately but are lost on restart; the TOML config is re-read on boot. Venue-priced strategies and disabled sides (one-sided LP) are not editable. The server is unauthenticated — the host defaults to `127.0.0.1`; only bind another interface (e.g. `--admin-port 0.0.0.0:8686` in docker) on a trusted network.
