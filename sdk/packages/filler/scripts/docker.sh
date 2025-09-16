#!/bin/bash
set -e

# Simple Docker script for Hyperbridge Filler
# Handles the essential Docker operations: build, run, and docker-compose

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

# Show help
show_help() {
    echo "Usage: $0 [command]"
    echo
    echo "Commands:"
    echo "  build       Build the Docker image"
    echo "  run         Run the filler in a Docker container"
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
        docker build -t polytopelabs/hyperbridge-filler:latest -f "$DOCKERFILE" "$(dirname "$(dirname "$ROOT_DIR")")"
        echo -e "${GREEN}✓ Docker image built successfully!${NC}"
        ;;

    run)
        echo -e "${YELLOW}Running Docker container...${NC}"

        # Check if config file exists
        if [ ! -f "$CONFIG_FILE" ]; then
            echo -e "${RED}Error: Config file not found at $CONFIG_FILE${NC}"
            exit 1
        fi

        # Check if image exists
        if ! docker image inspect polytopelabs/hyperbridge-filler:latest &> /dev/null; then
            echo -e "${YELLOW}Image not found, building first...${NC}"
            docker build -t polytopelabs/hyperbridge-filler:latest -f "$DOCKERFILE" "$(dirname "$(dirname "$ROOT_DIR")")"
        fi

        # Remove existing container if it exists
        if docker ps -a | grep -q hyperbridge-filler; then
            echo -e "${YELLOW}Removing existing container...${NC}"
            docker rm -f hyperbridge-filler
        fi

        # Run the container
        docker run -d \
            --name hyperbridge-filler \
            --restart unless-stopped \
            -v "$CONFIG_FILE:/app/packages/filler/config/config.toml:ro" \
            -e NODE_ENV=production \
            --log-driver json-file \
            --log-opt max-size=10m \
            --log-opt max-file=3 \
            polytopelabs/hyperbridge-filler:latest

        echo -e "${GREEN}✓ Container started!${NC}"
        echo "  View logs:   docker logs -f hyperbridge-filler"
        echo "  Stop:        docker stop hyperbridge-filler"
        ;;

    up)
        echo -e "${YELLOW}Starting with Docker Compose...${NC}"

        # Check if config file exists
        if [ ! -f "$CONFIG_FILE" ]; then
            echo -e "${RED}Error: Config file not found at $CONFIG_FILE${NC}"
            exit 1
        fi

        # Ensure config file exists before starting
        if [ ! -f "$CONFIG_FILE" ]; then
            echo -e "${RED}Error: Config file not found at $CONFIG_FILE${NC}"
            exit 1
        fi

        # Start with Docker Compose
        CONFIG_PATH="$CONFIG_FILE" docker-compose -f "$COMPOSE_FILE" up -d
        echo -e "${GREEN}✓ Services started!${NC}"
        echo "  View logs:   $0 logs"
        echo "  Stop:        $0 down"
        ;;

    down)
        echo -e "${YELLOW}Stopping Docker Compose services...${NC}"
        docker-compose -f "$COMPOSE_FILE" down
        echo -e "${GREEN}✓ Services stopped${NC}"
        ;;

    logs)
        echo -e "${YELLOW}Showing logs (Ctrl+C to exit)...${NC}"
        docker-compose -f "$COMPOSE_FILE" logs -f
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
