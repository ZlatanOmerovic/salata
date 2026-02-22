#!/usr/bin/env bash
# Start the Salata interactive playground.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
COMPOSE_FILE="$SCRIPT_DIR/docker-compose.playground.yml"
IMAGE_NAME="salata-playground"

# Check prerequisites.
if ! command -v docker &>/dev/null; then
    echo "Error: docker is not installed or not in PATH."
    echo "Install Docker: https://docs.docker.com/get-docker/"
    exit 1
fi

if ! docker compose version &>/dev/null; then
    echo "Error: 'docker compose' is not available."
    echo "Install Docker Compose v2: https://docs.docker.com/compose/install/"
    exit 1
fi

# Build the image if it doesn't exist yet.
if ! docker image inspect "$IMAGE_NAME" &>/dev/null; then
    echo "Building Salata Playground image (this only happens once)..."
    echo "This takes a few minutes — installing runtimes and compiling salata."
    echo ""
    docker compose -f "$COMPOSE_FILE" build
    echo ""
    echo "Build complete!"
    echo ""
fi

echo "Starting Salata Playground..."
echo ""

docker compose -f "$COMPOSE_FILE" \
    run --rm --service-ports playground bash --login

echo ""
echo "Salata Playground exited."
