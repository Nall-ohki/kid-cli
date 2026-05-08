#!/bin/bash
set -e

# Kid Management Suite - Integration Test
# Run this INSIDE the simulator environment.

echo "=== Starting Integration Test ==="

# 1. Bootstrap
echo "--- Step 1: Bootstrapping ---"
./bootstrap.sh

# 2. Create User
echo "--- Step 2: Creating Test User ---"
TEST_USER="test_kid_$(date +%s)"
sudo kid admin kid create "$TEST_USER"

# 3. Verify System User
echo "--- Step 3: Verifying System User ---"
id "$TEST_USER" > /dev/null || { echo "FAIL: User not created"; exit 1; }
groups "$TEST_USER" | grep -q "kid-users" || { echo "FAIL: User not in group"; exit 1; }
[ -f "/home/$TEST_USER/.zshrc" ] || { echo "FAIL: .zshrc missing"; exit 1; }
echo "PASS: System user looks correct."

# 4. Verify Docker Container
echo "--- Step 4: Verifying Docker Launch ---"
# We simulate a login by running the launcher logic
export USER="$TEST_USER"
sudo -E -u "$TEST_USER" docker compose -f /opt/kid-cli/docker-compose.yml -p "kid-$TEST_USER" up -d
docker ps | grep -q "kid-$TEST_USER" || { echo "FAIL: Docker container not running"; exit 1; }
echo "PASS: Docker container started successfully."

# 5. Verify Reset Logic
echo "--- Step 5: Testing Reset ---"
# Create a dummy creation file
sudo -u "$TEST_USER" touch "/home/$TEST_USER/creations/test_work.txt"
sudo kid admin kid reset "$TEST_USER"
[ -f "/home/$TEST_USER/creations/test_work.txt" ] || { echo "FAIL: Creations not preserved after reset"; exit 1; }
echo "PASS: Reset logic preserved data."

# 6. Cleanup
echo "--- Step 6: Cleanup ---"
sudo kid admin kid delete "$TEST_USER"
! id "$TEST_USER" > /dev/null 2>&1 || { echo "FAIL: User still exists after deletion"; exit 1; }
echo "PASS: Cleanup successful."

echo ""
echo "=== ALL TESTS PASSED! ==="
