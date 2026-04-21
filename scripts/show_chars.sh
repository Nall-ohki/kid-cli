#!/bin/bash
set -e

# Get the directory where this script sits
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# The project root is one level up from scripts/
KID_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Ensuring kid-env:latest is up to date ==="
DOCKER_BUILDKIT=1 docker build -t kid-env:latest "$KID_DIR"

echo "=== Launching Character Viewer in Container ==="
docker run --rm -it --init -u kid kid-env:latest /kid/bin/kid characters
