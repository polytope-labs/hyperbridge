#!/bin/bash

# Check if directory argument is provided
if [ $# -lt 1 ]; then
    echo "Error: You must provide a directory to search for Docker Compose files."
    echo "Usage: $0 <directory> [optional_specific_compose_file]"
    exit 1
fi

SEARCH_DIR="$1"

# Validate that the search directory exists
if [ ! -d "$SEARCH_DIR" ]; then
    echo "Error: Directory '$SEARCH_DIR' does not exist."
    exit 1
fi


# Function to shut down a specific docker-compose file
shutdown_compose() {
    local file="$1"

    if [ ! -f "$file" ]; then
        echo "Error: File '$file' does not exist."
        return 1
    fi

    dir=$(dirname "$file")
    filename=$(basename "$file")

    echo "Shutting down services from: $dir/$filename"

    docker compose -f "$dir/$filename" --env-file ../../.env.$ENV rm -fsv

    echo "Successfully shut down: $dir/$filename"
}

# Function to shut down all docker-compose files in a directory
shutdown_all_compose() {
    local dir="$1"

    echo "Searching for Docker Compose files in $dir..."
    files=$(find "$dir" -type f -name "*.yml" -o -name "*.yaml")

    if [ -z "$files" ]; then
        echo "No Docker Compose files found in $dir!"
        return 1
    fi

    for file in $files; do
        shutdown_compose "$file"
        echo "----------------------------------------"
    done

    echo "All Docker Compose services have been shut down!"
}

# Main script execution
if [ $# -eq 1 ]; then
    # Only directory provided, shut down all docker-compose files in that directory
    shutdown_all_compose "$SEARCH_DIR"
elif [ $# -eq 2 ]; then
    # Both directory and specific file provided
    SPECIFIC_FILE="$2"

    # Check if the specific file is an absolute path
    if [[ "$SPECIFIC_FILE" = /* ]]; then
        # It's an absolute path, use it directly
        shutdown_compose "$SPECIFIC_FILE"
    else
        # It's a relative path, combine with the search directory
        shutdown_compose "$SEARCH_DIR/$SPECIFIC_FILE"
    fi
else
    echo "Error: Too many arguments provided."
    echo "Usage: $0 <directory> [optional_specific_compose_file]"
    exit 1
fi
