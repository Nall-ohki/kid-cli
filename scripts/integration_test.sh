#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/.."

# Handle --sim flag (Run inside Docker)
if [[ "$1" == "--sim" ]]; then
  echo "--- Preparing Simulator ---"
  docker compose -f dev/docker-compose.sim.yml build
  echo "--- Launching Test in Simulator ---"
  # Use docker compose to run this script inside the simulator
  docker compose -f dev/docker-compose.sim.yml run --rm host-sim /opt/kid-cli/scripts/integration_test.sh
  exit $?
fi

echo "=== Starting Integration Test ==="

# 1. Bootstrap
echo "--- Step 1: Bootstrapping ---"
"$SCRIPT_DIR/bootstrap.sh"

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
echo "--- Step 5: Testing Reset & Data Integrity ---"
# Create a dummy creation with content
TEST_CONTENT="Kids important work $(date)"
sudo -u "$TEST_USER" bash -c "echo \"$TEST_CONTENT\" > /home/$TEST_USER/creations/test_work.txt"

sudo kid admin kid reset "$TEST_USER"

[ -f "/home/$TEST_USER/creations/test_work.txt" ] || { echo "FAIL: Creations not preserved after reset"; exit 1; }
ACTUAL_CONTENT=$(cat "/home/$TEST_USER/creations/test_work.txt")
if [ "$ACTUAL_CONTENT" != "$TEST_CONTENT" ]; then
    echo "FAIL: Data corruption detected after reset!"
    exit 1
fi
echo "PASS: Reset logic preserved data integrity."

# 6. Multi-User Isolation
echo "--- Step 6: Testing Multi-User Isolation ---"
USER_A="kid_a_$(date +%s)"
USER_B="kid_b_$(date +%s)"
sudo kid admin kid create "$USER_A"
sudo kid admin kid create "$USER_B"

# Write unique data for each
sudo -u "$USER_A" bash -c "echo 'data_a' > /home/$USER_A/creations/secret.txt"
sudo -u "$USER_B" bash -c "echo 'data_b' > /home/$USER_B/creations/secret.txt"

# Delete A, verify B
echo "Deleting $USER_A, verifying $USER_B..."
sudo kid admin kid delete "$USER_A"
[ -f "/home/$USER_B/creations/secret.txt" ] || { echo "FAIL: User B data lost after User A deletion"; exit 1; }
grep -q "data_b" "/home/$USER_B/creations/secret.txt" || { echo "FAIL: User B data corrupted"; exit 1; }

# Cleanup B
sudo kid admin kid delete "$USER_B"
echo "PASS: Multi-user isolation verified."

# 7. Final Cleanup
echo "--- Step 7: Final Cleanup ---"
sudo kid admin kid delete "$TEST_USER"
! id "$TEST_USER" > /dev/null 2>&1 || { echo "FAIL: User still exists after deletion"; exit 1; }
echo "PASS: Cleanup successful."

echo ""
echo "=== ALL TESTS PASSED! ==="
