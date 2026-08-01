# Scripts Directory

This directory contains utility scripts for Hyperbridge Simplex.

## Docker

- **Dockerfile**: Builds the simplex image. Multi-stage (node:24 builder → node:24-slim
  runtime) and arch-neutral, so the same file produces the published `linux/amd64` +
  `linux/arm64` manifest. The build context is the `sdk` workspace root, not this directory.
- **docker-compose.yml**: Simplex plus Prometheus and Grafana. Works unchanged on Linux,
  macOS and Windows — `docker compose -f scripts/docker-compose.yml up -d`.
- **docker.sh**: bash convenience wrapper around the two (Linux/macOS; on Windows call
  `docker compose` directly).

Note: The `.dockerignore` that applies is `sdk/.dockerignore`, since the build context is
the `sdk` workspace root.

### Usage

```bash
./docker.sh [command]

# Commands:
#   build       Build the Docker image (single-arch, for this machine)
#   run         Run Simplex in a Docker container
#   up          Start using Docker Compose
#   down        Stop and remove Docker Compose containers
#   logs        View logs from Docker Compose containers
#   help        Show this help message
```

To reproduce the published multi-arch image locally (needs QEMU for the foreign arch):

```bash
docker buildx build --platform linux/amd64,linux/arm64 \
    -f packages/simplex/scripts/Dockerfile sdk
```

### Ports and volumes

The container publishes nothing by itself. `8686` is the operator web UI and `9090` the
Prometheus metrics endpoint; both bind `0.0.0.0` inside the container so `-p` can reach them,
since Docker Desktop on macOS and Windows has no host networking. `/data` holds
`filler-config.toml`, the bids database and runtime state; mount a volume there to keep it
across container replacement.

Set up with the terminal wizard — the browser wizard refuses any non-loopback bind (it
handles private keys), so it cannot be published out of a container:

```bash
docker run --rm -it -v simplex-data:/data polytopelabs/simplex:latest init -o /data/filler-config.toml
```

## Build scripts

- **build.sh**: protoc codegen + tsup bundle + vite web UI (`pnpm build`).
- **generate-proto.sh**: the codegen half on its own (`pnpm codegen`).

## Development Notes

If you modify any of these scripts, make sure they remain executable:

```bash
chmod +x scripts/*.sh
```
