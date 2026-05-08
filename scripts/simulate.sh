#!/bin/bash
set -e

# Kid Host Simulator Launcher
# This script automates the build and entry into the Linux simulation environment.

echo "=== Kid Host Simulator ==="

# 1. Ensure we are in the project root
cd "$(dirname "$0")/.."

# 2. Start/Build the simulator
echo "--- Starting Simulator Container ---"
docker compose -f dev/docker-compose.sim.yml up -d --build

# 3. Enter the simulator shell
echo "--- Entering Simulator (zsh) ---"
echo "Tip: Run './bootstrap.sh' once inside to globalize the install."
docker exec -it kid-host-sim zsh
