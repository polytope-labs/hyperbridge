# Scripts Directory

This directory contains utility scripts for Hyperbridge Simplex.

## Docker

- **Dockerfile**: Builds the simplex image. Multi-stage (node:24 builder → node:24-slim
  runtime) and arch-neutral, so the same file produces the published `linux/amd64` +
  `linux/arm64` manifest. The build context is the `sdk` workspace root, not this directory.
- **docker-compose.yml**: Simplex as a service. Works unchanged on Linux, macOS and
  Windows — `docker compose -f scripts/docker-compose.yml up -d`.
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

The container publishes nothing by itself. `8686` is the web UI (and setup wizard); it binds
`0.0.0.0` inside the container so `-p` can reach it, since Docker Desktop on macOS and
Windows has no host networking. Publish it to `127.0.0.1` — it is unauthenticated. `/data` holds `filler-config.toml`, the bids database
and runtime state; mount a volume there to keep it across container replacement.

With no config in `/data` the container serves the setup wizard on 8686. For a headless host,
the terminal wizard writes the same file:

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
