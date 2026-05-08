#!/bin/bash
set -e

echo "--- Installing Rust (Local User) ---"

# 1. Check if rustup is already installed
if ! command -v rustup >/dev/null 2>&1; then
    echo "Installing rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
    source "$HOME/.cargo/env"
else
    echo "rustup is already installed. Updating..."
    rustup update stable
fi

# 2. Force version 1.95.0 (to match project parity)
echo "Ensuring Rust version 1.95.0..."
rustup install 1.95.0
rustup default 1.95.0

# 3. Add to .zshrc if not already present
ZSHRC="$HOME/.zshrc"
if [ -f "$ZSHRC" ]; then
    if ! grep -q ".cargo/env" "$ZSHRC"; then
        echo "Adding rustup to $ZSHRC..."
        echo '' >> "$ZSHRC"
        echo '# Rust Toolchain' >> "$ZSHRC"
        echo '. "$HOME/.cargo/env"' >> "$ZSHRC"
    fi
fi

echo "--- Installation Complete! ---"
echo "Please run: source ~/.zshrc"
echo "Or restart your terminal to start using rustc $(rustc --version)"
