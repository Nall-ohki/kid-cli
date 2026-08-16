#!/bin/bash
set -e

# Kid-CLI Remote Deployer (Runs on Mac)
# Usage: ./scripts/dev/deploy.sh <pi_hostname_or_ip> [--full]

PI_HOST=$1
FLAG=$2
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

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

ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "--- Pushing local changes to Git ---"
git push

# 2. Sync Unversioned Assets (ROMs) directly via SSH
if [ -d "$ROOT_DIR/assets/roms" ]; then
    echo "--- Syncing assets/roms directly to $PI_HOST (bypassing Git) ---"
    ssh "$PI_HOST" "sudo mkdir -p /opt/kid-cli/assets/roms && sudo chown -R \$(id -u):\$(id -g) /opt/kid-cli/assets/roms"
    rsync -avz --delete "$ROOT_DIR/assets/roms/" "$PI_HOST:/opt/kid-cli/assets/roms/"
fi

# 3. Handle Full Docker Sync
IMAGE_FLAG=""
if [[ "$FLAG" == "--full" ]]; then
    # Ensure Docker is running (macOS health check and auto-start)
    "$SCRIPT_DIR/ensure_docker.sh" || true
    if ! docker info &> /dev/null; then
        echo "❌ Error: Docker is not running." >&2
        echo "💡 Run './scripts/start_docker.sh' to start Docker, then try again." >&2
        exit 1
    fi

    echo "--- [FULL] Building and Exporting Docker Image (Slow) ---"
    docker build -t kid-env:latest "$ROOT_DIR"
    docker save kid-env:latest | gzip > kid-env.tar.gz
    
    echo "--- [FULL] Syncing Image to $PI_HOST ---"
    rsync -azP kid-env.tar.gz "$PI_HOST:/tmp/kid-env.tar.gz"
    IMAGE_FLAG="--image /tmp/kid-env.tar.gz"
fi

echo "--- Updating and Deploying on $PI_HOST ---"
ssh -t "$PI_HOST" "cd /opt/kid-cli && git pull && ( [ -f /opt/kid-cli/bin/kid ] || sudo ./scripts/internal/build_kid_binary.sh ) && sudo /usr/local/bin/kid admin deploy $IMAGE_FLAG && sudo rm -f /tmp/kid-env.tar.gz"

echo ""
echo "=== Remote Deployment Complete! ==="
