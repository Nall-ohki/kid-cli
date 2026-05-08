#!/bin/bash
set -e

# Kid Host Simulator Launcher
# Usage: ./scripts/simulate.sh [reset|build]

# 1. Ensure we are in the project root
cd "$(dirname "$0")/.."

COMPOSE_CMD="docker compose -f dev/docker-compose.sim.yml"

case "$1" in
  reset)
    echo "--- Wiping Simulator & Volumes ---"
    $COMPOSE_CMD down -v
    exit 0
    ;;
  build)
    echo "--- Building Simulator Image ---"
    $COMPOSE_CMD build
    exit 0
    ;;
  *)
    # Default: Start and Enter
    echo "--- Starting Simulator Container ---"
    $COMPOSE_CMD up -d --build
    echo "--- Entering Simulator (zsh) ---"
    docker exec -it kid-host-sim zsh
    ;;
esac
