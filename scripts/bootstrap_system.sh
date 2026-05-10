#!/bin/bash
set -e

# Kid-CLI Global System Setup
# This script is intended to be run as root:
# curl -fsSL https://raw.githubusercontent.com/Nall-ohki/kid-cli/main/scripts/bootstrap_system.sh | sudo bash

echo "=== Kid-CLI Global Setup ==="
echo ""
echo "!!! WARNING !!!"
echo "This script will perform the following system-wide actions:"
echo "1. Install system dependencies (Git, Docker, Rsync, etc.)"
echo "2. Install the Rust toolchain (version 1.95.0)"
echo "3. Create a global installation at /opt/kid-cli"
echo "4. Create a system group 'kid-users'"
echo "5. Install a global symlink at /usr/local/bin/kid"
echo ""
read -p "Are you sure you want to proceed? (y/N): " -r CONFIRM < /dev/tty
if [[ ! "$CONFIRM" =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi
echo ""

# 1. Clone Repository to /opt/kid-cli
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

# 2. System Dependencies & Build
echo "--- Installing System Dependencies ---"
bash "$GLOBAL_PATH/scripts/internal/install_deps.sh"

echo "--- Building Kid-CLI ---"
bash "$GLOBAL_PATH/scripts/internal/build_kid_binary.sh"


echo ""
echo "=== Setup Complete! ==="
echo "You can now provision kids by running: sudo /opt/kid-cli/scripts/manage_kids.sh"
