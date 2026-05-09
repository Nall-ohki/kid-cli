#!/bin/bash
set -e

# Kid-CLI Global System Setup
# This script is intended to be run as root:
# curl -fsSL https://raw.githubusercontent.com/Nall-ohki/kid-cli/main/scripts/setup_system.sh | sudo bash

echo "=== Kid-CLI Global Setup ==="

# 1. Install System Dependencies
echo "--- Installing System Dependencies ---"
apt-get update
apt-get install -y git docker.io rsync curl

# 2. Clone Repository to /opt/kid-cli
GLOBAL_PATH="/opt/kid-cli"
if [ -d "$GLOBAL_PATH" ]; then
    echo "--- Existing installation found at $GLOBAL_PATH ---"
    read -p "Overwrite (delete and re-clone) or Update (git pull)? [o/u/Skip]: " -r ACTION < /dev/tty
    
    if [[ "$ACTION" == "o" || "$ACTION" == "O" ]]; then
        echo "Deleting existing installation..."
        rm -rf "$GLOBAL_PATH"
        echo "Cloning fresh repository..."
        git clone https://github.com/Nall-ohki/kid-cli.git "$GLOBAL_PATH"
    elif [[ "$ACTION" == "u" || "$ACTION" == "U" ]]; then
        echo "Updating existing installation..."
        git -C "$GLOBAL_PATH" pull
    else
        echo "Skipping repository sync."
    fi
else
    echo "--- Cloning repository to $GLOBAL_PATH ---"
    git clone https://github.com/Nall-ohki/kid-cli.git "$GLOBAL_PATH"
fi

# 3. Build and Initialize
echo "--- Bootstrapping System ---"
cd "$GLOBAL_PATH"

# Ensure we have the local kid binary ready (pre-compiled)
# Or build it if we are on a different architecture
if [ ! -f "bin/kid" ]; then
    echo "Warning: No pre-compiled binary found. You will need to install Rust and build manually."
else
    chmod +x bin/kid
    ./bin/kid admin init --skip-build
fi

echo ""
echo "=== Setup Complete! ==="
echo "You can now provision kids by running: sudo /opt/kid-cli/scripts/init.sh"
