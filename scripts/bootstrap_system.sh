#!/bin/bash
set -e

# Kid-CLI Global System Setup
# curl -fsSL https://raw.githubusercontent.com/Nall-ohki/kid-cli/main/scripts/bootstrap_system.sh | bash

# Check for -y flag
ASSUME_YES=false
if [[ "$*" == *"-y"* ]] || [[ "$*" == *"--yes"* ]]; then
    ASSUME_YES=true
fi

# Helper for interactive prompts that works with curl | bash
prompt_confirm() {
    if [ "$ASSUME_YES" = true ]; then return 0; fi
    
    local message="$1"
    local response
    
    # Try to find a valid TTY for input
    if [ -c /dev/tty ]; then
        read -p "$message (y/N): " -r response < /dev/tty
    else
        echo "Error: No interactive terminal found. Use -y to bypass prompts."
        exit 1
    fi

    if [[ "$response" =~ ^[Yy]$ ]]; then
        return 0
    else
        return 1
    fi
}

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

if ! prompt_confirm "Are you sure you want to proceed?"; then
    echo "Aborted."
    exit 1
fi
echo ""

# 1. Clone Repository to /opt/kid-cli
GLOBAL_PATH="/opt/kid-cli"
if [ -d "$GLOBAL_PATH" ]; then
    echo "--- Existing installation found at $GLOBAL_PATH ---"
    
    if [ "$ASSUME_YES" = true ]; then
        ACTION="u"
    else
        read -p "Overwrite (delete and re-clone) or Update (git pull)? [o/u/Skip]: " -r ACTION < /dev/tty
    fi
    
    if [[ "$ACTION" == "o" || "$ACTION" == "O" ]]; then
        echo "Deleting existing installation..."
        sudo rm -rf "$GLOBAL_PATH"
        echo "Cloning fresh repository..."
        sudo git clone https://github.com/Nall-ohki/kid-cli.git "$GLOBAL_PATH"
        sudo chown -R $(id -u):$(id -g) "$GLOBAL_PATH"
    elif [[ "$ACTION" == "u" || "$ACTION" == "U" ]]; then
        echo "Updating existing installation..."
        sudo git -C "$GLOBAL_PATH" pull
        sudo chown -R $(id -u):$(id -g) "$GLOBAL_PATH"
    else
        echo "Skipping repository sync."
    fi
else
    echo "--- Cloning repository to $GLOBAL_PATH ---"
    sudo git clone https://github.com/Nall-ohki/kid-cli.git "$GLOBAL_PATH"
    sudo chown -R $(id -u):$(id -g) "$GLOBAL_PATH"
fi

# 2. System Dependencies & Build
echo "--- Installing System Dependencies ---"
bash "$GLOBAL_PATH/scripts/internal/install_deps.sh"

echo "--- Building Kid-CLI ---"
bash "$GLOBAL_PATH/scripts/internal/build_kid_binary.sh"


echo ""
echo "=== Setup Complete! ==="
echo "You can now provision kids by running: sudo kid admin kid create <name>"
