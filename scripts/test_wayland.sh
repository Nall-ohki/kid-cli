#!/bin/bash
# scripts/test_wayland.sh
# Tests the Cage + Foot approach for a chrome-less, high-res terminal environment.

# 1. Install dependencies if missing (Raspberry Pi OS / Debian)
if ! command -v cage &> /dev/null || ! command -v foot &> /dev/null; then
    echo "Installing cage and foot..."
    sudo apt-get update && sudo apt-get install -y cage foot
fi

echo "--- Starting Wayland Kiosk Mode ---"
echo "Architecture: [GPU] -> [Cage] -> [Foot]"
echo ""
echo "HOW TO TEST:"
echo "1. A fullscreen terminal (Foot) should appear with NO chrome."
echo "2. Inside that terminal, launch a 'game' (e.g., type 'foot' again or launch 'weston-terminal')."
echo "3. Notice how Cage automatically maximizes the new window over the old one."
echo "4. Close the 'game' window to return to your primary terminal."
echo ""
echo "Press Enter to launch..."
read

# Launch foot inside cage. 
# We use -m to disable the server/client mode for this simple test.
cage foot
