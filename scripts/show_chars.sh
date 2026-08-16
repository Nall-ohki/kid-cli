#!/bin/bash
set -e

# Get the directory where this script sits
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# The project root is one level up from scripts/
KID_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Ensure Docker is running (macOS health check and auto-start)
"$SCRIPT_DIR/dev/ensure_docker.sh" || true
if ! docker info &> /dev/null; then
    echo "❌ Error: Docker is not running." >&2
    echo "💡 Run './scripts/start_docker.sh' to start Docker, then try again." >&2
    exit 1
fi

echo "=== Ensuring kid-env:latest is up to date ==="
DOCKER_BUILDKIT=1 docker build -t kid-env:latest "$KID_DIR"

echo "=== Launching Character Viewer in Container ==="
docker run --rm -it --init -u kid kid-env:latest /kid/bin/kid characters
