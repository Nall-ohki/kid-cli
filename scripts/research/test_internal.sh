#!/bin/bash
# scripts/research/test_internal.sh
# This script runs INSIDE the research terminals to verify their capabilities.

echo "===================================================="
echo "         TERMINAL RESEARCH INTERNAL TEST            "
echo "===================================================="
echo "Terminal: $TERM"
echo "Resolution: $(tput cols)x$(tput lines)"
echo ""

# 1. Sixel Test
echo "--- 1. Sixel Graphics Test ---"
echo "If you see a small colored pattern below, Sixel is WORKING:"
# A simple Sixel pattern (Red and Green squares)
printf "\033Pq#0;2;0;0;0#1;2;100;0;0#2;2;0;100;0#1~~@@#2@@~~#1~~@@#2@@~~\033\\"
echo ""
echo ""

# 2. Push/Pop Test
echo "--- 2. Game Push/Pop Test ---"
echo "This will test releasing the display to a 'game' on TTY2."
echo "Press Enter to PUSH (switch to game)..."
read

GAME_CMD="bash -c '
echo \"[GAME MODE ON TTY2]\";
echo \"Display has been PUSHED away from the terminal.\";
for i in {5..1}; do 
    echo \"POPPING back in \$i...\"; 
    sleep 1; 
done'"

# Execute the handoff
# Note: This requires 'openvt' (kbd package)
if command -v openvt &> /dev/null; then
    sudo openvt -s -w -c 2 -- "$GAME_CMD"
    echo ""
    echo "--- POPPED BACK ---"
    echo "Check if fonts/graphics are still correct."
else
    echo "ERROR: 'openvt' not found. Please install 'kbd' package."
fi

echo ""
echo "Research test complete. Type 'exit' to close this terminal."
exec $SHELL
