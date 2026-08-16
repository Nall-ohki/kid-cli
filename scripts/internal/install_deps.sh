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
  "cage"
  "foot"
  "fonts-noto-cjk"
  "fonts-font-awesome"
  "fonts-noto-color-emoji"
  "mame"
  "retroarch"
  "libretro-bsnes-mercury-performance"
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
if [ -d /run/systemd/system ]; then
    if ! systemctl is-active --quiet docker; then
        echo "Starting Docker service..."
        sudo systemctl enable --now docker
    fi
else
    echo "Systemd not detected. Skipping Docker service start/enable."
fi

# 4. Install Custom Font (Comic Shanns Mono)
if [ ! -d /usr/share/fonts/truetype/comic-shanns-mono ]; then
    echo "Installing Comic Shanns Mono font..."
    sudo rm -rf /tmp/comic-shanns-mono
    if git clone --depth 1 https://github.com/jesusmgg/comic-shanns-mono.git /tmp/comic-shanns-mono 2>/dev/null; then
        sudo mkdir -p /usr/share/fonts/truetype/comic-shanns-mono
        sudo cp /tmp/comic-shanns-mono/fonts/*.ttf /usr/share/fonts/truetype/comic-shanns-mono/
        sudo rm -rf /tmp/comic-shanns-mono
        sudo fc-cache -f >/dev/null 2>&1 || true
        echo "Comic Shanns Mono font installed."
    fi
fi

# Ensure docker socket has accessible permissions for non-root users
if [ -S /var/run/docker.sock ]; then
    sudo chmod 666 /var/run/docker.sock || true
fi

echo "--- System Dependencies Installed ---"
