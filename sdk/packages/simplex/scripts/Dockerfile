# syntax=docker/dockerfile:1
#
# Multi-arch image for @hyperbridge/simplex — published for linux/amd64 and linux/arm64 so
# it runs natively on Intel/AMD Linux and Windows (WSL2) hosts as well as Apple Silicon
# Macs and Windows-on-ARM, with no QEMU emulation.
#
# Nothing below is arch-specific: the node images are multi-arch and the tree compiles no
# native addon, so there is no per-arch artifact to resolve. Build both arches with:
#   docker buildx build --platform linux/amd64,linux/arm64 -f packages/simplex/scripts/Dockerfile sdk
#
# The base tags are pinned to an exact patch on purpose, and both stages move together. SQLite
# is now the runtime's own (`node:sqlite`) rather than a compiled module, which makes the base
# image the thing that supplies the database engine: these tags must stay at or above the
# `engines.node` floor in package.json, and a bump is a change to what the store runs on.

# ---------------------------------------------------------------------------
# Builder — installs the toolchain, compiles protos, bundles the CLI and web UI.
# ---------------------------------------------------------------------------
FROM node:24.19.0 AS builder

# protoc is needed by packages/simplex/scripts/build.sh. There is deliberately no python3,
# make or g++ here: nothing in the tree compiles any more, so node-gyp has nothing to fall
# back to and the toolchain is dead weight in the builder layer.
RUN apt-get update && apt-get install -y git curl protobuf-compiler libprotobuf-dev --no-install-recommends && rm -rf /var/lib/apt/lists/*

# Install pnpm, pinned to the major the workspace's packageManager field names: pnpm 12
# refuses to resolve pnpm@11.0.0 as a trust downgrade, so an unpinned install broke the image.
RUN npm install -g pnpm@11

# Set working directory
WORKDIR /app

# Copy workspace manifests and lockfile first for better layer caching
COPY pnpm-workspace.yaml package.json .npmrc tsconfig.base.json pnpm-lock.yaml ./
COPY packages/sdk/package.json ./packages/sdk/
COPY packages/simplex/package.json ./packages/simplex/

# Install dependencies for sdk and simplex only
RUN pnpm install --frozen-lockfile --filter @hyperbridge/sdk --filter @hyperbridge/simplex

# Copy source files
COPY packages/sdk/ ./packages/sdk/
COPY packages/simplex/ ./packages/simplex/

# Build the SDK first (dependency of simplex)
RUN cd packages/sdk && pnpm build

# Build simplex (protoc codegen + tsup bundle + vite web UI)
RUN cd packages/simplex && pnpm build

# ---------------------------------------------------------------------------
# Runtime — slim base, no compiler toolchain. Keeps the pull small on Docker Desktop
# hosts, where every layer lands inside the Linux VM's disk image.
# ---------------------------------------------------------------------------
FROM node:24.19.0-slim AS runtime

# Create non-root user
RUN groupadd -g 1001 simplex && \
    useradd -u 1001 -g simplex -s /bin/sh -m simplex

WORKDIR /app

# The whole tree, pnpm's symlinked store included — COPY preserves the relative symlinks
# under node_modules/.pnpm.
COPY --from=builder --chown=simplex:simplex /app /app

# Default persistent data location (bids database, runtime state). Mount a volume here to
# survive container replacement.
RUN mkdir -p /data && chown simplex:simplex /data

# Node-only healthcheck: the slim base has neither curl nor ps. Probes the UI server,
# which the default CMD below turns on.
RUN printf '%s\n' \
    'const res = await fetch("http://127.0.0.1:8686/health").catch(() => undefined)' \
    'process.exit(res?.ok ? 0 : 1)' > /app/healthcheck.mjs

USER simplex

# Set environment variables
ENV NODE_ENV=production

EXPOSE 8686

# Set up healthcheck
HEALTHCHECK --interval=30s --timeout=10s --start-period=15s --retries=3 CMD ["node", "/app/healthcheck.mjs"]

# Run out of /data, not the install tree: config discovery and `init -o` default to the
# working directory, so a volume mounted at /data is what makes filler-config.toml survive
# `docker rm`.
WORKDIR /data

# ENTRYPOINT fixes the binary; CMD provides the default arguments.
#
# --ui binds every container interface rather than the CLI's 127.0.0.1 default: a
# container-local loopback bind is unreachable from the host, and Docker Desktop on
# macOS/Windows runs the daemon inside a VM with no --network host to fall back on, so
# published ports are the only route in. The port stays private until published with `docker run -p`.
# Carry the --ui flag over if you override this command.
#
# With no /data/filler-config.toml this serves the browser setup wizard on 8686 and starts
# the filler once it is saved; with one, it runs the filler directly. `init` is the same
# flow in the terminal, for headless hosts:
#   docker run --rm -it -v simplex-data:/data <image> init -o /data/filler-config.toml
ENTRYPOINT ["node", "--enable-source-maps", "--disable-warning=ExperimentalWarning", "/app/packages/simplex/dist/bin/simplex.js"]
CMD ["run", "--data-dir", "/data", "--ui", "0.0.0.0:8686"]
