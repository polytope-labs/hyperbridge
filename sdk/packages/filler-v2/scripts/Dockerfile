# Dockerfile for Hyperbridge Intent FillerV2
FROM node:24-alpine

# Install dependencies required for builds
RUN apk add --no-cache python3 make g++ git curl

# Create non-root user
RUN addgroup -g 1001 -S filler && \
    adduser -S filler -u 1001

# Set working directory
WORKDIR /app

# Create config directories (support both /app/config and /app/packages/filler-v2/config)
RUN mkdir -p /app/config /app/packages/filler-v2/config

# Install pnpm
RUN npm install -g pnpm@9.15.0

# Copy the entire workspace
COPY . .

# Install dependencies for the monorepo
RUN pnpm install --frozen-lockfile

# Build the SDK first (dependency of filler-v2)
RUN cd packages/sdk && pnpm build

# Build the filler-v2
RUN cd packages/filler-v2 && pnpm build

# Configure permissions
RUN chown -R filler:filler /app

# Switch to non-root user
USER filler

# Set environment variables
ENV NODE_ENV=production

# Create a healthcheck script
RUN echo '#!/bin/sh' > /app/healthcheck.sh && \
    echo 'ps -ef | grep "node" | grep -v grep > /dev/null || exit 1' >> /app/healthcheck.sh && \
    chmod +x /app/healthcheck.sh

# Set up healthcheck
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 CMD [ "/app/healthcheck.sh" ]

# Working directory for running the application
# Set working directory to filler-v2 package
WORKDIR /app/packages/filler-v2

# Create symbolic link between config locations for compatibility
RUN ln -sf /app/config/config.toml /app/packages/filler-v2/config/config.toml

# Command to run the application
CMD ["node", "./dist/bin/filler.js", "run", "-c", "/app/config/config.toml"]
