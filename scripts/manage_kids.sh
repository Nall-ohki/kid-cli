#!/bin/bash
set -e

# Kid-CLI Full Environment Initializer
# This script bootstraps the system and provisions the initial users.
# Usage: ./scripts/init.sh [kid_name1] [kid_name2] ...
# Defaults to: kid

if [[ "$1" == "--help" ]] || [[ "$1" == "-h" ]]; then
    echo "Usage: $0 [kid_name1] [kid_name2] ..."
    echo "Initializes the system and creates the specified kid user accounts."
    echo "If no names are provided, defaults to: kid"
    exit 0
fi

# 1. Ensure we are in the project root
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/.."

# 2. Run the system build and install
echo "--- Building Kid-CLI System ---"
"$SCRIPT_DIR/internal/build_kid_binary.sh"

# 3. Create the initial users
# Default kid if none provided as arguments
KIDS=("${@:-kid}")

for NAME in "${KIDS[@]}"; do
    if id "$NAME" >/dev/null 2>&1; then
        echo "--- User '$NAME' already exists ---"
        read -p "Recreate environment (preserve data) for $NAME? (y/N): " CONFIRM
        if [[ "$CONFIRM" == "y" || "$CONFIRM" == "Y" ]]; then
            echo "Resetting environment for $NAME (preserving data)..."
            sudo kid admin kid reset "$NAME"
            # Set/refresh default password
            echo "$NAME:$NAME$NAME" | sudo chpasswd
        else
            echo "Skipping $NAME."
        fi
        continue
    fi

    echo "--- Provisioning Kid: $NAME ---"
    sudo kid admin kid create "$NAME"
    # Set default password: name + name (e.g. kidkid)
    echo "$NAME:$NAME$NAME" | sudo chpasswd
done

echo ""
echo "=== System Ready! ==="
echo "Users created: ${KIDS[*]}"
echo "Try logging in as one: ./scripts/login.sh ${KIDS[0]} --sim"
