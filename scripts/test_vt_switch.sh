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

# 2. Check for HD Terminal
USE_HD=true
if [[ "$TERM" != "mlterm" ]]; then
    echo "--- Currently in a standard TTY ---"
    echo "Attempting to launch mlterm (HD Terminal)..."
    
    # Check if we are in a GUI (which would block mlterm-fb)
    if [ -n "$WAYLAND_DISPLAY" ] || [ -n "$DISPLAY" ]; then
        echo "!!! GUI DETECTED: mlterm-fb cannot run while a desktop is active."
        echo "Falling back to Standard TTY mode for the switch test..."
        USE_HD=false
    else
        # Try to launch mlterm. We use a subshell check instead of exec so we don't lose the shell.
        if command -v mlterm > /dev/null; then
             echo "Handoff to mlterm... (if screen blanks and returns to prompt, mlterm failed)"
             # If we are in tmux, mlterm might fail. We'll try to run it.
             # We won't 'exec' yet so we can catch the failure.
             if ! mlterm --help > /dev/null 2>&1; then
                echo "Warning: mlterm binary exists but is failing. Falling back..."
                USE_HD=false
             fi
        else
             USE_HD=false
        fi
    fi
fi

if [ "$USE_HD" = true ] && [[ "$TERM" != "mlterm" ]]; then
    echo "Entering HD Mode..."
    exec mlterm
    exit
fi

echo "--- PROCEEDING IN $([ "$USE_HD" = true ] && echo "HD" || echo "STANDARD") MODE ---"

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
