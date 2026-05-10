#!/bin/bash
# scripts/test_vt_switch.sh
# Tests a real HD terminal emulator (mlterm-fb) with a game "push/pop" transition.

set -e

# 0. Check for SSH
if [ -n "$SSH_CONNECTION" ]; then
    echo "!!! WARNING: YOU ARE CONNECTED VIA SSH !!!"
    echo "Virtual Terminal switching (chvt/openvt) only affects the PHYSICAL monitor."
    echo "You will not see the screen change in this SSH window."
    echo ""
fi

# 1. Install mlterm-fb if missing
if ! command -v mlterm &> /dev/null; then
    echo "--- Installing mlterm (HD Terminal) ---"
    sudo apt-get update && sudo apt-get install -y mlterm-common mlterm-tiny
fi

# 2. Check if we are already inside mlterm
if [[ "$TERM" != "mlterm" ]]; then
    echo "--- Currently in a standard TTY ---"
    echo "Launching mlterm (HD Terminal)..."
    echo "NOTE: If this fails, make sure you are on the physical console."
    echo ""
    # Try to launch mlterm. If it fails, report it.
    if ! exec mlterm; then
        echo "Error: Failed to launch mlterm. Are you in a desktop environment or SSH?"
        exit 1
    fi
    exit
fi

# 3. We are now inside the HD Terminal!
echo "===================================================="
echo "   SUCCESS: YOU ARE NOW IN A HIGH-RES TERMINAL     "
echo "===================================================="
echo ""
echo "TESTING THE 'PUSH/POP' FEATURE:"
echo "1. We will 'push' this terminal session to the background."
echo "2. We will switch to TTY2 to launch a 'game'."
echo "3. We will then 'pop' back to this exact HD state."
echo ""
echo "Press Enter to start the push/pop..."
read -r

# Define a "Game" on TTY2
GAME_CMD="bash -c '
echo \"[GAME MODE ON TTY2]\";
echo \"The HD Terminal is currently suspended.\";
for i in {5..1}; do 
    echo \"Popping back to HD Terminal in \$i...\"; 
    sleep 1; 
done'"

echo "--- Executing openvt handoff to TTY 2 ---"
# -s: switch, -w: wait, -c 2: TTY2
if ! sudo openvt -s -w -c 2 -- "$GAME_CMD"; then
    echo "Error: openvt failed. Make sure you have sudo permissions and are on a real console."
    exit 1
fi

echo ""
echo "--- POPPED BACK TO HD TERMINAL ---"
echo "Verify that fonts and graphics are still intact."
