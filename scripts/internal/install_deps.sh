#!/bin/bash
set -e

# Kid-CLI Internal Dependency Installer
# Installs low-level system requirements: Git, Docker, Rsync, etc.

echo "--- Checking System Dependencies ---"

if [ "$EUID" -ne 0 ]; then
  echo "Error: Please run as root (sudo)."
  exit 1
fi

# 1. Update Package List
apt-get update

# 2. Install Required Packages
PACKAGES=(
  "git"
  "docker.io"
  "rsync"
  "curl"
  "build-essential"
  "pkg-config"
  "libssl-dev"
)

echo "Installing packages: ${PACKAGES[*]}..."
apt-get install -y "${PACKAGES[@]}"

# 3. Ensure Docker is running
systemctl enable --now docker

echo "--- System Dependencies Installed ---"
