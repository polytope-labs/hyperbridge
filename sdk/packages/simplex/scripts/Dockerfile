# Dockerfile for simplex
FROM node:22

# Install dependencies required for builds
RUN apt-get update && apt-get install -y python3 make g++ git curl --no-install-recommends && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -g 1001 simplex && \
    useradd -u 1001 -g simplex -s /bin/sh -m simplex

# Install pnpm
RUN npm install -g pnpm

# Set working directory
WORKDIR /app

# Copy workspace manifests and lockfile first for better layer caching
COPY --chown=simplex:simplex pnpm-workspace.yaml package.json .npmrc tsconfig.base.json pnpm-lock.yaml ./
COPY --chown=simplex:simplex packages/sdk/package.json ./packages/sdk/
COPY --chown=simplex:simplex packages/simplex/package.json ./packages/simplex/

# Install dependencies for sdk and simplex only
RUN pnpm install --frozen-lockfile --filter @hyperbridge/sdk --filter @hyperbridge/simplex

# Copy source files
COPY --chown=simplex:simplex packages/sdk/ ./packages/sdk/
COPY --chown=simplex:simplex packages/simplex/ ./packages/simplex/

# Build the SDK first (dependency of simplex)
RUN cd packages/sdk && pnpm build

# Build simplex
RUN cd packages/simplex && pnpm build

# Create a healthcheck script that hits the metrics /health endpoint
RUN echo '#!/bin/sh' > /app/healthcheck.sh && \
    echo 'node -e "fetch(\"http://localhost:9090/health\").then(r=>{if(!r.ok)throw 1})" 2>/dev/null || ps -ef | grep "node" | grep -v grep > /dev/null || exit 1' >> /app/healthcheck.sh && \
    chmod +x /app/healthcheck.sh

USER simplex

# Set environment variables
ENV NODE_ENV=production

# Set up healthcheck
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 CMD [ "/app/healthcheck.sh" ]

# Set working directory to simplex package for the entrypoint
WORKDIR /app/packages/simplex

# ENTRYPOINT fixes the binary; CMD provides the default subcommand.
# Example: docker run <image> run -c /path/to/config.toml --data-dir /data --watch-only
ENTRYPOINT ["node", "./dist/bin/simplex.js"]
CMD ["run"]
