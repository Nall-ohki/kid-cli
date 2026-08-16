#!/bin/bash

# Kid-CLI Docker Health & Autostart Utility
# Ensures the Docker container runtime (Colima or Docker Desktop) is active.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

# If Docker is already responsive, succeed immediately
if command -v docker &>/dev/null && docker info &>/dev/null; then
    exit 0
fi

# Delegate to the top-level start_docker.sh script
if [ -f "$ROOT_DIR/scripts/start_docker.sh" ]; then
    "$ROOT_DIR/scripts/start_docker.sh"
else
    echo "⚠️  Docker is not running. Please start Docker or run './scripts/start_docker.sh'." >&2
    exit 1
fi
