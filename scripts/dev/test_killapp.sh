#!/bin/bash
set -e

echo "=== Starting Full Killapp Test ==="

# Make sure we're in the right directory
cd "$(dirname "$0")/../.."

# Ensure Docker is running (macOS health check and auto-start, non-blocking)
./scripts/dev/ensure_docker.sh || true

# 1. Start a long-running app (e.g. sleep or a specific mock app)
echo "Starting mock target app (klettres)..."
docker exec -d kid-host-sim bash -c "exec -a klettres sleep 1000"
sleep 1

# 2. Check if it's running
echo "Checking if klettres is running..."
if docker exec kid-host-sim pgrep -f klettres > /dev/null; then
    echo "  [OK] klettres is running."
else
    echo "  [FAIL] klettres failed to start."
    exit 1
fi

# 3. Ensure the daemon is running
echo "Checking daemon..."
if ! docker exec kid-host-sim pgrep -f "kid watch" > /dev/null; then
    echo "  Daemon not running! Starting manually..."
    docker exec kid-host-sim sh -c "nohup /kid/bin/kid watch --daemon > /home/kid/.kid_watch.log 2>&1 &"
    sleep 2
fi

# 4. Trigger the panic hotkey
echo "Triggering killapp (F12 hold)..."
./scripts/dev/simulate.sh killapp

# 5. Verify the app was killed
echo "Verifying app termination..."
if docker exec kid-host-sim pgrep -f klettres > /dev/null; then
    echo "  [FAIL] klettres is STILL running!"
    exit 1
else
    echo "  [SUCCESS] klettres was successfully killed!"
fi

echo "=== Test Complete ==="
