#!/bin/bash
set -e

# Kid-CLI Internal Builder
# This script ensures the correct Rust toolchain is present, 
# compiles the binary, and performs global system initialization.

# 1. Ensure Dependencies
command -v docker >/dev/null 2>&1 || { echo >&2 "Error: Docker is required. Please install it first."; exit 1; }

# 2. Ensure Rust 1.95.0 is installed
REQUIRED_VERSION="1.95.0"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

check_rust() {
    if command -v rustc >/dev/null 2>&1; then
        VERSION=$(rustc --version | awk '{print $2}')
        if [[ "$VERSION" == "$REQUIRED_VERSION" ]]; then
            return 0
        fi
    fi
    return 1
}

if ! check_rust; then
    echo "--- Rust $REQUIRED_VERSION not found. Installing... ---"
    "$SCRIPT_DIR/install_rust.sh"
    # Ensure the global paths are active in this subshell
    export RUSTUP_HOME=/usr/local/rustup
    export CARGO_HOME=/usr/local/cargo
    export PATH="/usr/local/cargo/bin:$PATH"
fi

# 2. Build the binary
echo "--- Compiling Kid-CLI ($REQUIRED_VERSION) ---"
cargo build --release
# 3. Run System Initialization
echo "--- Initializing System ---"
FLAG=""
if [[ "$*" == *"--skip-docker-build"* ]]; then
    FLAG="--skip-docker-build"
fi

sudo ./target/release/kid admin init $FLAG

echo ""
echo "=== Initialization Complete! ==="
echo "You can now manage kids using: sudo kid [command]"
echo "Try: sudo kid admin kid create <name>"
