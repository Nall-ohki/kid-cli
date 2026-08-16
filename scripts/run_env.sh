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

# Parse arguments
BUILD_ONLY=false
if [[ "$1" == "--build-only" ]]; then
    BUILD_ONLY=true
fi

echo "=== Building kid-env:latest ==="
DOCKER_BUILDKIT=1 docker build -t kid-env:latest "$KID_DIR"

if [ "$BUILD_ONLY" = true ]; then
    echo "=== Build Complete (Build Only) ==="
    exit 0
fi

echo "=== Launching Interactive Kid Environment ==="
# Pre-emptively remove any existing container with the same name to avoid conflicts
docker rm -f kid_manual_test 2>/dev/null || true

# We use --rm to automatically clean up the container when exited
# We use --init and -it to properly handle process reaping and PTYs for tmux
docker run --rm -it --init -u kid --name kid_manual_test kid-env:latest tmux new-session
