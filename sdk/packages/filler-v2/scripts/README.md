# Scripts Directory

This directory contains utility scripts for the Hyperbridge FillerV2.

## Docker Script

A single, simplified script to handle all Docker operations:

- **docker.sh**: All-in-one Docker operations for building, running, and managing containers
- **Dockerfile**: Used to build the filler-v2 Docker image
- **docker-compose.yml**: Configuration for running the filler-v2 using Docker Compose

Note: The `.dockerignore` file is located in the parent directory as it needs to be in the root of the Docker build context.

### Usage

```bash
./docker.sh [command]

# Commands:
#   build       Build the Docker image
#   run         Run the filler in a Docker container
#   up          Start using Docker Compose
#   down        Stop and remove Docker Compose containers
#   logs        View logs from Docker Compose containers
#   help        Show this help message
```

## Other Scripts

- **make-executable.sh**: Utility to make scripts executable

## Configuration

The Docker script is configured to use the config file from the parent directory (`../config.toml`).

## Development Notes

If you modify any of these scripts, make sure they remain executable:

```bash
chmod +x scripts/*.sh
```
