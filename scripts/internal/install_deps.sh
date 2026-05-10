#!/bin/bash
set -e

# Kid-CLI Internal Dependency Installer
# Installs low-level system requirements: Git, Docker, Rsync, etc.

echo "--- Checking System Dependencies ---"

# 1. Define required packages
PACKAGES=(
  "git"
  "rsync"
  "curl"
  "build-essential"
  "pkg-config"
  "libssl-dev"
)

# Decide on Docker package
if apt-cache policy docker-ce >/dev/null 2>&1; then
    PACKAGES+=("docker-ce" "docker-ce-cli" "containerd.io")
else
    PACKAGES+=("docker.io")
fi

# 2. Identify missing packages
MISSING_PACKAGES=()
for pkg in "${PACKAGES[@]}"; do
    if ! dpkg -s "$pkg" >/dev/null 2>&1; then
        MISSING_PACKAGES+=("$pkg")
    fi
done

# 2. Install only if needed
if [ ${#MISSING_PACKAGES[@]} -gt 0 ]; then
    echo "Installing missing packages: ${MISSING_PACKAGES[*]}..."
    sudo apt-get update
    sudo apt-get install -y "${MISSING_PACKAGES[@]}"
else
    echo "All system packages are already installed."
fi

# 3. Ensure Docker is running
if ! systemctl is-active --quiet docker; then
    echo "Starting Docker service..."
    sudo systemctl enable --now docker
fi

echo "--- System Dependencies Installed ---"
