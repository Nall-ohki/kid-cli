#!/bin/bash
set -e

# Ensure we are in project root
cd "$(dirname "$0")/.."

echo "=== Kid-CLI Bootstrap ==="

# 1. Dependency Checks
command -v cargo >/dev/null 2>&1 || { echo >&2 "Error: Rust/Cargo is required. Install it via https://rustup.rs"; exit 1; }
command -v docker >/dev/null 2>&1 || { echo >&2 "Error: Docker is required. Please install it first."; exit 1; }

# 2. Compile the CLI
echo "--- Compiling Kid-CLI (Release Mode) ---"
cargo build --release

# 3. Run System Initialization
echo "--- Initializing System (Requires Sudo) ---"
# Ensure the docker socket is accessible within the simulator
sudo chmod 666 /var/run/docker.sock || true
sudo ./target/release/kid admin init

echo ""
echo "=== Initialization Complete! ==="
echo "You can now manage kids using: sudo kid [command]"
echo "Try: sudo kid create-kid <name>"
