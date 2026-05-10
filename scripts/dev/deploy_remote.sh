#!/bin/bash
set -e

# Kid-CLI Remote Deployer (Runs on Mac)
# Usage: ./scripts/dev/deploy.sh <pi_hostname_or_ip> [--full]

PI_HOST=$1
FLAG=$2

if [ -z "$PI_HOST" ]; then
    echo "Usage: $0 <pi_hostname_or_ip> [--full]"
    echo "Example: $0 192.168.1.50"
    exit 1
fi

# 1. Pre-flight Checks
FORCE=false
if [[ "$*" == *"--force"* ]] || [[ "$*" == *"-f"* ]]; then
    FORCE=true
fi

echo "--- Checking connectivity to $PI_HOST ---"
if ! ssh -q -o ConnectTimeout=5 "$PI_HOST" exit; then
    echo "Error: Cannot reach $PI_HOST via SSH. Check your connection/IP."
    exit 1
fi

if [ "$FORCE" = false ]; then
    # Check for uncommitted changes
    if [[ -n $(git status --porcelain) ]]; then
        echo "Error: You have uncommitted changes. Please commit them or use --force to ignore."
        exit 1
    fi

    # Check branch parity
    LOCAL_BRANCH=$(git rev-parse --abbrev-ref HEAD)
    REMOTE_BRANCH=$(ssh "$PI_HOST" "cd /opt/kid-cli && git rev-parse --abbrev-ref HEAD" 2>/dev/null || echo "unknown")
    
    if [ "$REMOTE_BRANCH" != "unknown" ] && [ "$LOCAL_BRANCH" != "$REMOTE_BRANCH" ]; then
        echo "Error: Branch mismatch! Mac is on '$LOCAL_BRANCH', but Pi is on '$REMOTE_BRANCH'."
        echo "Please switch branches or use --force."
        exit 1
    fi
fi

echo "--- Pushing local changes to Git ---"
git push

# 2. Handle Full Docker Sync
IMAGE_FLAG=""
if [[ "$FLAG" == "--full" ]]; then
    echo "--- [FULL] Building and Exporting Docker Image (Slow) ---"
    docker build -t kid-cli-kid:latest .
    docker save kid-cli-kid:latest | gzip > kid-env.tar.gz
    
    echo "--- [FULL] Syncing Image to $PI_HOST ---"
    rsync -azP kid-env.tar.gz "$PI_HOST:/tmp/kid-env.tar.gz"
    IMAGE_FLAG="--image /tmp/kid-env.tar.gz"
fi

echo "--- Updating and Deploying on $PI_HOST ---"
ssh -t "$PI_HOST" "sudo kid admin deploy $IMAGE_FLAG && sudo rm -f /tmp/kid-env.tar.gz"

echo ""
echo "=== Remote Deployment Complete! ==="
