#!/bin/bash
set -e

# Simple Docker script for Hyperbridge Simplex
# Handles the essential Docker operations: build, run, and docker compose
#
# Linux and macOS convenience wrapper. On Windows, run the same operations with
# `docker compose -f scripts/docker-compose.yml up -d` from PowerShell — nothing in the
# compose file needs a POSIX shell.

# Color codes for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Get the script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CONFIG_FILE="$ROOT_DIR/config.toml"
DOCKERFILE="$SCRIPT_DIR/Dockerfile"
COMPOSE_FILE="$SCRIPT_DIR/docker-compose.yml"
IMAGE="polytopelabs/simplex:latest"
UI_PORT="${SIMPLEX_UI_PORT:-8686}"

# Compose v2 is a docker subcommand; keep working where only the v1 binary exists.
compose() {
    if docker compose version &> /dev/null; then
        docker compose "$@"
    else
        docker-compose "$@"
    fi
}

# Show help
show_help() {
    echo "Usage: $0 [command]"
    echo
    echo "Commands:"
    echo "  build       Build the Docker image"
    echo "  init        Run the terminal setup wizard, writing into the simplex-data volume"
    echo "  run         Run Simplex in a Docker container"
    echo "  up          Start using Docker Compose"
    echo "  down        Stop and remove Docker Compose containers"
    echo "  logs        View logs from Docker Compose containers"
    echo "  help        Show this help message"
}

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo -e "${RED}Error: Docker is not installed${NC}"
    exit 1
fi

# No arguments provided
if [ $# -eq 0 ]; then
    show_help
    exit 1
fi

# Parse command
COMMAND="$1"
shift

case "$COMMAND" in
    build)
        echo -e "${YELLOW}Building Docker image...${NC}"
        echo -e "$(dirname "$(dirname "$ROOT_DIR")")"
        # Single-arch, for this machine. The published image is a linux/amd64 +
        # linux/arm64 manifest built by .github/workflows/publish-simplex.yml; reproduce it
        # locally with: docker buildx build --platform linux/amd64,linux/arm64 ...
        docker build -t "$IMAGE" -f "$DOCKERFILE" "$(dirname "$(dirname "$ROOT_DIR")")"
        echo -e "${GREEN}✓ Docker image built successfully!${NC}"
        ;;

    init)
        # The terminal wizard. `$0 up` with an empty volume serves the browser one instead.
        echo -e "${YELLOW}Starting the setup wizard...${NC}"
        docker volume create simplex-data > /dev/null
        docker run --rm -it -v simplex-data:/data "$IMAGE" init -o /data/filler-config.toml
        echo -e "${GREEN}✓ Config written to /data/filler-config.toml in the simplex-data volume${NC}"
        echo "  Start it:    $0 up"
        ;;

    run)
        echo -e "${YELLOW}Running Docker container...${NC}"

        # Check if config file exists
        if [ ! -f "$CONFIG_FILE" ]; then
            echo -e "${RED}Error: Config file not found at $CONFIG_FILE${NC}"
            exit 1
        fi

        # Check if image exists
        if ! docker image inspect "$IMAGE" &> /dev/null; then
            echo -e "${YELLOW}Image not found, building first...${NC}"
            docker build -t "$IMAGE" -f "$DOCKERFILE" "$(dirname "$(dirname "$ROOT_DIR")")"
        fi

        # Remove existing container if it exists
        if docker ps -a | grep -q simplex; then
            echo -e "${YELLOW}Removing existing container...${NC}"
            docker rm -f simplex
        fi

        # Ports are published on loopback rather than shared with the host network stack:
        # the UI is unauthenticated, and Docker Desktop (macOS/Windows) has no --network
        # host to fall back on anyway.
        docker run -d \
            --name simplex \
            --restart unless-stopped \
            -p "127.0.0.1:$UI_PORT:8686" \
            -v simplex-data:/data \
            -v "$CONFIG_FILE:/data/filler-config.toml:ro" \
            -e NODE_ENV=production \
            --log-driver json-file \
            --log-opt max-size=10m \
            --log-opt max-file=3 \
            "$IMAGE" \
            run -c /data/filler-config.toml --data-dir /data --ui 0.0.0.0:8686

        echo -e "${GREEN}✓ Container started!${NC}"
        echo "  Web UI:      http://localhost:$UI_PORT"
            echo "  View logs:   docker logs -f simplex"
        echo "  Stop:        docker stop simplex"
        ;;

    up)
        echo -e "${YELLOW}Starting with Docker Compose...${NC}"

        # With no /data/filler-config.toml in the volume, the service comes up in the setup
        # wizard on the published UI port.
        compose -f "$COMPOSE_FILE" up -d
        echo -e "${GREEN}✓ Services started!${NC}"
        echo "  Web UI:      http://localhost:$UI_PORT"
        echo "  View logs:   $0 logs"
        echo "  Stop:        $0 down"
        ;;

    down)
        echo -e "${YELLOW}Stopping Docker Compose services...${NC}"
        compose -f "$COMPOSE_FILE" down
        echo -e "${GREEN}✓ Services stopped${NC}"
        ;;

    logs)
        echo -e "${YELLOW}Showing logs (Ctrl+C to exit)...${NC}"
        compose -f "$COMPOSE_FILE" logs -f
        ;;

    help)
        show_help
        ;;

    *)
        echo -e "${RED}Error: Unknown command $COMMAND${NC}"
        show_help
        exit 1
        ;;
esac
