#!/bin/bash
set -e

# Kid-CLI Docker Startup Helper
# Automatically detects and starts the appropriate container runtime (Colima or Docker Desktop on macOS, systemd on Linux).

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# 1. Check if Docker CLI is installed
if ! command -v docker &> /dev/null; then
    echo "❌ Error: 'docker' CLI not found."
    if [[ "$(uname -s)" == "Darwin" ]]; then
        echo "💡 On macOS, you can install Colima + Docker CLI via Homebrew:"
        echo "   brew install colima docker docker-compose"
    fi
    exit 1
fi

# 2. Check if Docker is already running
if docker info &> /dev/null; then
    ACTIVE_CONTEXT=$(docker context show 2>/dev/null || echo "default")
    echo "✅ Docker is already running! (Context: $ACTIVE_CONTEXT)"
    exit 0
fi

echo "🐳 Docker daemon is not running. Starting container runtime..."

# 3. Handle macOS Runtime (Colima prioritized, then Docker Desktop)
if [[ "$(uname -s)" == "Darwin" ]]; then
    # Try Colima first
    if command -v colima &> /dev/null; then
        echo "🚀 Starting Colima..."
        docker context use colima &>/dev/null || true
        if colima start; then
            echo "Waiting for Docker daemon to become responsive..."
            for i in {1..20}; do
                if docker info &> /dev/null; then
                    echo "✅ Docker (Colima) is now ready!"
                    exit 0
                fi
                sleep 1
            done
        fi
        echo "⚠️  Colima start did not succeed. Checking Docker Desktop fallback..."
    fi

    # Fallback to Docker Desktop
    echo "🚀 Attempting to launch Docker Desktop..."
    docker context use desktop-linux &>/dev/null || docker context use default &>/dev/null || true
    
    if open -g -a Docker 2>/dev/null || open -a Docker 2>/dev/null; then
        echo "Waiting for Docker Desktop to initialize (up to 30s)..."
        for i in {1..30}; do
            if docker info &> /dev/null; then
                echo "✅ Docker (Docker Desktop) is now ready!"
                exit 0
            fi
            sleep 1
        done
    fi

    echo "❌ Failed to automatically start Docker."
    echo "💡 Please ensure either Colima ('colima start') or Docker Desktop is installed and running."
    exit 1

# 4. Handle Linux Runtime (systemd)
elif [[ "$(uname -s)" == "Linux" ]]; then
    echo "🚀 Starting Docker service via systemctl..."
    if sudo systemctl start docker; then
        if docker info &> /dev/null; then
            echo "✅ Docker service started successfully!"
            exit 0
        fi
    fi
    echo "❌ Failed to start Docker service. Run 'sudo systemctl status docker' for details."
    exit 1

else
    echo "❌ Unsupported OS for automated Docker start. Please start Docker manually."
    exit 1
fi
