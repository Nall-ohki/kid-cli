#!/bin/bash
set -e

# Get the directory where this script sits
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# The project root is one level up from scripts/
KID_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Building kid-env:latest ==="
# We allow the user to override via KID_DIR env var if they move things
DOCKER_BUILDKIT=1 docker build -t kid-env:latest "$KID_DIR"

echo "=== Launching Interactive Kid Environment ==="
# We use --rm to automatically clean up the container when exited
# We use --init and -it to properly handle process reaping and PTYs for tmux
docker run --rm -it --init -u kid --name kid_manual_test kid-env:latest tmux new-session
