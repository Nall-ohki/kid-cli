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

# 1. Build locally on Mac (Fast)
echo "--- Building Kid-CLI for ARM64 (Mac) ---"
cargo build --release
cp target/release/kid bin/kid

# 2. Sync files to Pi
echo "--- Syncing files to $PI_HOST ---"
# We sync to a temp location then move to /opt/ to avoid permission issues during sync
rsync -azP --exclude target --exclude .git . "$PI_HOST:/tmp/kid-cli-sync"

echo "--- Installing on $PI_HOST ---"
ssh -t "$PI_HOST" "sudo rsync -a /tmp/kid_cli_sync/ /opt/kid-cli/ && sudo /usr/local/bin/kid admin deploy"

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
