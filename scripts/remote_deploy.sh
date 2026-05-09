#!/bin/bash
set -e

# Kid-CLI Remote Deployer (Runs on Mac)
# Usage: ./scripts/remote_deploy.sh <pi_hostname_or_ip> [--full]

PI_HOST=$1
FLAG=$2

if [ -z "$PI_HOST" ]; then
    echo "Usage: $0 <pi_hostname_or_ip> [--full]"
    echo "Example: $0 192.168.1.50"
    exit 1
fi

# 1. Build and Push from Mac
echo "--- Pushing local changes to Git ---"
git push

echo "--- Building Kid-CLI for ARM64 (Mac) ---"
cargo build --release

# 2. Sync ONLY the binary to Pi
echo "--- Syncing binary to $PI_HOST ---"
rsync -azP target/release/kid "$PI_HOST:/tmp/kid-binary"

echo "--- Updating and Deploying on $PI_HOST ---"
# Pull latest code, install the new binary, then run deploy
ssh -t "$PI_HOST" "cd /opt/kid-cli && sudo git pull && sudo cp /tmp/kid-binary /usr/local/bin/kid && sudo /usr/local/bin/kid admin deploy"

# 3. Optional: Full Docker Sync
if [[ "$FLAG" == "--full" ]]; then
    echo "--- [FULL] Building and Exporting Docker Image (Slow) ---"
    docker build -t kid-env:latest .
    docker save kid-env:latest | gzip > kid-env.tar.gz
    
    echo "--- [FULL] Syncing Image to $PI_HOST ---"
    rsync -azP kid-env.tar.gz "$PI_HOST:~/kid-env.tar.gz"
    
    echo "--- [FULL] Loading Image on $PI_HOST ---"
    ssh "$PI_HOST" "docker load < ~/kid-env.tar.gz && rm ~/kid-env.tar.gz"
fi

echo ""
echo "=== Remote Deployment Complete! ==="
