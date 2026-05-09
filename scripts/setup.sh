#!/bin/bash
set -e

# Kid-CLI Global System Setup
# This script is intended to be run as root:
# curl -fsSL https://raw.githubusercontent.com/Nall-ohki/kid-cli/main/scripts/setup_system.sh | sudo bash

echo "=== Kid-CLI Global Setup ==="

# 1. Install System Dependencies
echo "--- Installing System Dependencies ---"
apt-get update
apt-get install -y git docker.io rsync curl build-essential

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

# 3. Internal Toolchain & Bootstrap
echo "--- Setting up Rust toolchain ---"
"$GLOBAL_PATH/scripts/internal/install_rust.sh"

echo "--- Bootstrapping System ---"
"$GLOBAL_PATH/scripts/internal/build_kid_binary.sh"

echo ""
echo "=== Setup Complete! ==="
echo "You can now provision kids by running: sudo /opt/kid-cli/scripts/manage_kids.sh"
