#!/bin/bash

# Kid-CLI Docker Health & Autostart Utility (macOS)
# Checks if the Docker daemon is responsive. If not and running on macOS,
# attempts to start Colima or Docker Desktop.

# Check if docker command is available
if ! command -v docker &> /dev/null; then
    echo "⚠️  docker command not found. Please install Docker." >&2
    exit 1
fi

# Check if Docker daemon is responsive
if ! docker info &> /dev/null; then
    if [[ "$(uname -s)" == "Darwin" ]]; then
        echo "🐳 Docker daemon is not running." >&2
        
        # 1. Try Colima if installed
        if command -v colima &> /dev/null; then
            echo "Attempting to start Colima..." >&2
            # Use colima context
            docker context use colima &>/dev/null || true
            if colima start; then
                echo "Waiting for Docker daemon (via Colima) to become ready..." >&2
                for i in {1..15}; do
                    if docker info &> /dev/null; then
                        echo "✅ Docker (Colima) is now running!" >&2
                        exit 0
                    fi
                    sleep 2
                done
            fi
        fi
        
        # 2. Try Docker Desktop if Colima didn't work or isn't installed
        echo "Attempting to start Docker Desktop..." >&2
        # Switch context back to default/desktop-linux
        docker context use desktop-linux &>/dev/null || docker context use default &>/dev/null || true
        
        if open -g -a Docker 2>/dev/null || open -a Docker 2>/dev/null; then
            echo "Waiting for Docker daemon (via Docker Desktop) to become ready (up to 30 seconds)..." >&2
            for i in {1..30}; do
                if docker info &> /dev/null; then
                    echo "✅ Docker (Docker Desktop) is now running!" >&2
                    exit 0
                fi
                sleep 1
            done
            echo "⚠️  Docker failed to start within 30 seconds." >&2
            exit 1
        else
            echo "⚠️  Could not launch Colima or Docker Desktop automatically. Please start Docker manually." >&2
            exit 1
        fi
    else
        echo "⚠️  Docker daemon is not running. Please start it on your system." >&2
        exit 1
    fi
fi

exit 0
