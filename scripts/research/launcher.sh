#!/bin/bash
# scripts/research/launcher.sh [wayland|vt] [terminal]
# Main launcher for Kid-CLI Terminal Research.

APPROACH=$1
TERM_BIN=$2
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
INTERNAL_TEST="$SCRIPT_DIR/test_internal.sh"

usage() {
    echo "Usage: $0 [wayland|vt] [terminal]"
    echo ""
    echo "Approaches:"
    echo "  wayland  - Uses 'cage' kiosk compositor"
    echo "  vt       - Runs directly on framebuffer/TTY"
    echo ""
    echo "Terminals:"
    echo "  wayland: foot, ghostty, contour, wezterm, mlterm"
    echo "  vt:      mlterm, yaft"
    echo ""
    echo "Example: $0 wayland foot"
    exit 1
}

if [[ -z "$APPROACH" || -z "$TERM_BIN" ]]; then
    usage
fi

# 1. Ensure internal test is executable
chmod +x "$INTERNAL_TEST"

# 2. Dependency Check & Install
PACKAGES=()
if [[ "$APPROACH" == "wayland" ]]; then PACKAGES+=("cage"); fi
if [[ "$TERM_BIN" == "mlterm" ]]; then PACKAGES+=("mlterm-common" "mlterm-tiny"); fi
if [[ "$TERM_BIN" == "foot" ]]; then PACKAGES+=("foot"); fi
# Add others as needed...

if [ ${#PACKAGES[@]} -gt 0 ]; then
    echo "Checking dependencies: ${PACKAGES[*]}"
    # On Pi, we would apt-get install here if needed
    # sudo apt-get update && sudo apt-get install -y "${PACKAGES[@]}"
fi

# 3. Launching
echo "--- Launching Scenario: $APPROACH / $TERM_BIN ---"

case "$APPROACH" in
    wayland)
        if ! command -v cage &> /dev/null; then echo "Error: cage not found"; exit 1; fi
        # Launch terminal inside cage, running the internal test script
        cage "$TERM_BIN" -e "$INTERNAL_TEST"
        ;;
    vt)
        # For VT, we just execute the terminal directly on the current TTY
        # Running it with -e ensures the test script starts immediately
        case "$TERM_BIN" in
            mlterm)
                exec mlterm -e "$INTERNAL_TEST"
                ;;
            yaft)
                exec yaft "$INTERNAL_TEST"
                ;;
            *)
                echo "Error: Terminal '$TERM_BIN' not supported in VT mode."
                usage
                ;;
        esac
        ;;
    *)
        usage
        ;;
esac
