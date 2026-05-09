#!/bin/bash
# scripts/test_vt_switch.sh
# Tests a real HD terminal emulator (mlterm-fb) with a game "push/pop" transition.

# 1. Install mlterm-fb if missing
if ! command -v mlterm &> /dev/null; then
    echo "Installing mlterm (HD Terminal with Sixel support)..."
    sudo apt-get update && sudo apt-get install -y mlterm-common mlterm-tiny
fi

# 2. Check if we are already inside mlterm
if [[ "$TERM" != "mlterm" ]]; then
    echo "--- Currently in a standard TTY ---"
    echo "Launching mlterm (HD Terminal)..."
    echo "Once inside, run this script again: ./scripts/test_vt_switch.sh"
    echo ""
    # Launch mlterm on the framebuffer. 
    # Note: On some Pi setups, you may need 'mlterm-fb' binary specifically.
    exec mlterm
    exit
fi

# 3. We are now inside the HD Terminal!
echo "===================================================="
echo "   SUCCESS: YOU ARE NOW IN A HIGH-RES TERMINAL     "
echo "   (Check your font quality and Unicode/Sixels)     "
echo "===================================================="
echo ""
echo "TESTING THE 'PUSH/POP' FEATURE:"
echo "1. We will 'push' this terminal session to the background."
echo "2. We will switch to TTY2 to launch a game."
echo "3. We will then 'pop' back to this exact HD state."
echo ""
echo "Press Enter to start the push/pop..."
read

# Define a "Game" on TTY2
GAME_CMD="bash -c '
echo \"[GAME MODE ON TTY2]\";
echo \"The HD Terminal is currently suspended.\";
for i in {5..1}; do 
    echo \"Popping back to HD Terminal in \$i...\"; 
    sleep 1; 
done'"

# Execute the handoff
# -s: switch, -w: wait, -c 2: TTY2
sudo openvt -s -w -c 2 -- "$GAME_CMD"

echo ""
echo "--- POPPED BACK TO HD TERMINAL ---"
echo "Verify that fonts and graphics are still intact."
