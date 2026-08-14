# Simplex

Automated intent solver for the Hyperbridge IntentGateway. This image runs the `simplex` binary;
the same solver is also an embeddable Node library, published as
[`@hyperbridge/simplex`](https://www.npmjs.com/package/@hyperbridge/simplex).

**Documentation:**
[running the binary](https://docs.hyperbridge.network/developers/evm/intent-gateway/simplex) ·
[embedding as a library](https://docs.hyperbridge.network/developers/sdk/simplex) ·
[API reference](https://docs.hyperbridge.network/developers/sdk/api/simplex)

Multi-arch manifest (`linux/amd64` + `linux/arm64`): Linux, macOS (Intel and Apple Silicon) and
Windows hosts each pull a natively-built image. The same commands run in bash and PowerShell.

## Quickstart

```bash
docker volume create simplex-data
docker run -d --name simplex --restart unless-stopped \
    -p 127.0.0.1:8686:8686 \
    -v simplex-data:/data \
    polytopelabs/simplex:latest
```

With no config in the volume this serves the setup wizard at `http://localhost:8686`, writes
`filler-config.toml` into `/data` and starts the solver in the same process; later restarts find
that config and run directly.

Two alternatives to the browser wizard:

```bash
# Mount a config you already have
docker run -d --name simplex --restart unless-stopped \
    -p 127.0.0.1:8686:8686 \
    -v /path/to/filler-config.toml:/data/filler-config.toml:ro \
    -v simplex-data:/data \
    polytopelabs/simplex:latest

# Or run the terminal wizard once, then start normally
docker run --rm -it -v simplex-data:/data polytopelabs/simplex:latest init -o /data/filler-config.toml
```

## Ports and volumes

`8686` is the web UI and setup wizard. The container's default command binds it to `0.0.0.0` —
a container-local loopback bind is unreachable from the host, and Docker Desktop on macOS and
Windows has no `--network host` to fall back on. The container's network namespace is the boundary,
so the port stays private until published. **Keep the `127.0.0.1:` prefix when publishing**: the UI
is unauthenticated, and the wizard collects private keys. If you override the command, carry
`--ui 0.0.0.0:8686` over with it.

`/data` holds `filler-config.toml`, the bids database and runtime state. Mount a volume there —
bid records are how locked deposits are found again for retraction, so they must survive container
replacement.

## Health

The image ships a healthcheck against the UI server's `/health`. `docker ps` shows
`healthy`/`unhealthy` accordingly; running with `--no-ui` disables the endpoint the healthcheck
probes.

## Compose

A ready-made service definition lives in the repository:
[`sdk/packages/simplex/scripts/docker-compose.yml`](https://github.com/polytope-labs/hyperbridge/blob/main/sdk/packages/simplex/scripts/docker-compose.yml)

## Tags

- `latest` — most recent release
- `vX.Y.Z` — one tag per release, matching the npm package version
