#!/bin/bash
set -e

# Kid-CLI Login Gateway
# Usage: ./scripts/login.sh <name> [--sim]

KID_NAME=$1
FLAG=$2

if [ -z "$KID_NAME" ]; then
    echo "Usage: $0 <kid_name> [--sim]"
    exit 1
fi

# 1. Ensure we are in project root
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/.."

# 2. Handle Simulation Mode
if [[ "$KID_NAME" == "--sim" ]] || [[ "$FLAG" == "--sim" ]]; then
    # If --sim was the first arg, shift
    if [[ "$KID_NAME" == "--sim" ]]; then
        KID_NAME=$2
        if [ -z "$KID_NAME" ]; then
            echo "Error: Must specify kid name when using --sim"
            exit 1
        fi
    fi

    echo "--- [SIM] Orchestrating Environment for $KID_NAME ---"
    
    # Start simulator
    ./scripts/simulate.sh run --no-enter
    
    # Bootstrap & Init (Inside simulator)
    docker exec kid-host-sim /opt/kid-cli/scripts/init.sh
    
    # Ensure THIS specific kid exists (in case it's not 'kid')
    docker exec kid-host-sim sudo kid admin kid create "$KID_NAME" || true
    
    # Hand off to standard login
    FLAG=""
fi

# 3. Standard Login Flow
echo "--- Logging in as $KID_NAME ---"
docker compose -f dev/docker-compose.sim.yml exec host-sim sudo su - "$KID_NAME"
