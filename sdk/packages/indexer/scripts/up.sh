
#!/bin/bash

# Default directory is current directory if none provided
SEARCH_DIR="${1:-.}"

# Find all potential docker-compose files
echo "Searching for Docker Compose files in $SEARCH_DIR..."
files=$(find "$SEARCH_DIR" -type f -name "*.yml" -o -name "*.yaml")

if [ -z "$files" ]; then
    echo "No Docker Compose files found!"
    exit 1
fi

# Process each file
for file in $files; do
    dir=$(dirname "$file")
    filename=$(basename "$file")

    echo "----------------------------------------"
    echo "Starting Docker Compose from: $file"

    docker compose -f "$dir/$filename" --env-file ../../.env.$ENV up -d --force-recreate

    echo "Successfully started: $dir/$filename"
done

echo "----------------------------------------"
echo "All Docker Compose files have been started!"
