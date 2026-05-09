#!/bin/bash
set -e

# Kid-CLI Global Rust Installer
# Installs Rust 1.95.0 system-wide to /usr/local/rustup and /usr/local/cargo

REQUIRED_VERSION="1.95.0"

if [ "$EUID" -ne 0 ]; then
  echo "Error: Please run as root (sudo)."
  exit 1
fi

echo "--- Installing Rust ($REQUIRED_VERSION) Globally ---"

# 1. Set Global Installation Paths
export RUSTUP_HOME=/usr/local/rustup
export CARGO_HOME=/usr/local/cargo

# 2. Download and Run Rustup Installer
# -y: skip prompts
# --no-modify-path: we handle symlinks manually for global access
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
    -y --default-toolchain "$REQUIRED_VERSION" --no-modify-path

# 3. Make toolchain available globally via symlinks
echo "Creating global symlinks in /usr/local/bin..."
ln -sf /usr/local/cargo/bin/rustc /usr/local/bin/rustc
ln -sf /usr/local/cargo/bin/cargo /usr/local/bin/cargo
ln -sf /usr/local/cargo/bin/rustup /usr/local/bin/rustup

# 4. Set Permissions (Allow kid-users group to use cargo cache if needed)
chmod -R a+rX /usr/local/rustup /usr/local/cargo

echo "Rust $REQUIRED_VERSION installed globally to /usr/local/cargo"
