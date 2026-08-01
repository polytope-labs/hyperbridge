# syntax=docker/dockerfile:1
#
# Multi-arch image for @hyperbridge/simplex — published for linux/amd64 and linux/arm64 so
# it runs natively on Intel/AMD Linux and Windows (WSL2) hosts as well as Apple Silicon
# Macs and Windows-on-ARM, with no QEMU emulation.
#
# Nothing below is arch-specific: node:24 and the Debian build deps are multi-arch, and
# better-sqlite3 resolves a per-arch prebuild (falling back to a source build with the
# toolchain installed in the builder stage). Build both arches with:
#   docker buildx build --platform linux/amd64,linux/arm64 -f packages/simplex/scripts/Dockerfile sdk

# ---------------------------------------------------------------------------
# Builder — installs the toolchain, compiles protos, bundles the CLI and web UI.
# ---------------------------------------------------------------------------
FROM node:24 AS builder

# protoc is needed by packages/simplex/scripts/build.sh; python3/make/g++ are the fallback
# path for native modules (better-sqlite3) on an arch with no matching prebuild.
RUN apt-get update && apt-get install -y python3 make g++ git curl protobuf-compiler libprotobuf-dev --no-install-recommends && rm -rf /var/lib/apt/lists/*

# Install pnpm
RUN npm install -g pnpm

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
FROM node:24-slim AS runtime

# Create non-root user
RUN groupadd -g 1001 simplex && \
    useradd -u 1001 -g simplex -s /bin/sh -m simplex

WORKDIR /app

# The whole tree, pnpm's symlinked store included — COPY preserves the relative symlinks
# under node_modules/.pnpm, and the native builds match this arch because each platform is
# built on a runner of its own architecture.
COPY --from=builder --chown=simplex:simplex /app /app

# Default persistent data location (bids database, runtime state). Mount a volume here to
# survive container replacement.
RUN mkdir -p /data && chown simplex:simplex /data

# Node-only healthcheck: the slim base has neither curl nor ps. Probes the metrics server,
# which the default CMD below turns on.
RUN printf '%s\n' \
    'const res = await fetch("http://127.0.0.1:9090/health").catch(() => undefined)' \
    'process.exit(res?.ok ? 0 : 1)' > /app/healthcheck.mjs

USER simplex

# Set environment variables
ENV NODE_ENV=production

EXPOSE 8686 9090

# Set up healthcheck
HEALTHCHECK --interval=30s --timeout=10s --start-period=15s --retries=3 CMD ["node", "/app/healthcheck.mjs"]

# Run out of /data, not the install tree: config discovery and `init -o` default to the
# working directory, so a volume mounted at /data is what makes filler-config.toml survive
# `docker rm`.
WORKDIR /data

# ENTRYPOINT fixes the binary; CMD provides the default arguments.
#
# --ui/-p bind every container interface rather than the CLI's 127.0.0.1 default: a
# container-local loopback bind is unreachable from the host, and Docker Desktop on
# macOS/Windows runs the daemon inside a VM with no --network host to fall back on, so
# published ports are the only route in. The ports stay private until published with -p.
# Carry both flags over if you override this command.
#
# Generate the config with the terminal wizard — the browser wizard handles private keys and
# refuses any non-loopback bind, so it cannot be published out of a container:
#   docker run --rm -it -v simplex-data:/data <image> init -o /data/filler-config.toml
ENTRYPOINT ["node", "/app/packages/simplex/dist/bin/simplex.js"]
CMD ["run", "--data-dir", "/data", "--ui", "0.0.0.0:8686", "-p", "0.0.0.0:9090"]
