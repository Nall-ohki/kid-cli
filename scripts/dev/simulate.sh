#!/bin/bash
set -e

# Kid Host Simulator Launcher
# Usage: ./scripts/simulate.sh [reset|build]

# 1. Ensure we are in the project root
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

COMPOSE_FILE="$SCRIPT_DIR/docker-compose.sim.yml"
COMPOSE_CMD="docker compose -f $COMPOSE_FILE"

case "$1" in
  reset)
    echo "--- Wiping Simulator & Volumes ---"
    $COMPOSE_CMD down -v
    exit 0
    ;;
  build)
    echo "--- Building Simulator Image ---"
    $COMPOSE_CMD build
    exit 0
    ;;
  run)
    echo "--- Starting Simulator Container ---"
    # Ensure any existing container with this name is removed to avoid conflicts
    docker rm -f kid-host-sim >/dev/null 2>&1 || true
    $COMPOSE_CMD up -d --build
    
    GUI_MODE=0
    for arg in "$@"; do
      if [[ "$arg" == "--gui" ]]; then
        GUI_MODE=1
      elif [[ "$arg" == "--no-enter" ]]; then
        exit 0
      fi
    done

    if [[ "$GUI_MODE" == 1 && "$(uname -s)" == "Darwin" ]]; then
      echo "--- Setting up macOS Wayland Simulation via VNC ---"
      echo "ℹ  TIP: If this is a new simulator, run './scripts/manage_kids.sh [names...]' in a separate terminal."
      
      echo "Starting local X11 Server (Xvfb) and VNC Bridge inside Docker..."
      # We use Xvfb so cage can use MIT-SHM locally inside the container (bypassing macOS XQuartz bugs entirely).
      docker exec -d kid-host-sim /bin/bash -c "Xvfb :99 -screen 0 1024x768x24 >/dev/null 2>&1 & sleep 1 && x11vnc -display :99 -forever -passwd kid -quiet -listen 0.0.0.0 >/dev/null 2>&1 &"
      
      echo "Starting cage and foot inside Simulator..."
      docker exec -u kid -d kid-host-sim /bin/bash -c "export DISPLAY=:99; export XDG_RUNTIME_DIR=/tmp/runtime-kid; mkdir -p \$XDG_RUNTIME_DIR; chmod 0700 \$XDG_RUNTIME_DIR; exec cage foot"
      
      echo "Waiting for VNC server to become ready..."
      sleep 2
      echo "Opening macOS Screen Sharing (VNC)..."
      open vnc://localhost:5900
      
      echo "--- Simulator is running in VNC! ---"
      echo "To stop, press Ctrl+C here, or close the Screen Sharing window."
      docker logs -f kid-host-sim > /dev/null
    else
      echo "--- Entering Simulator (zsh) ---"
      echo "ℹ  TIP: If this is a new simulator, run './scripts/manage_kids.sh [names...]' to provision kids."
      docker exec -it kid-host-sim zsh
    fi
    ;;
  killapp)
    echo "--- Simulating F12 Kiosk Exit ---"
    docker exec -u kid kid-host-sim /kid/bin/kid panic
    echo "--- Done ---"
    exit 0
    ;;
  *)
    echo "Usage: $0 {run|reset|build|killapp}"
    echo ""
    echo "  run     - Start and enter the simulator shell"
    echo "  reset   - Wipe simulator and persistent volumes"
    echo "  build   - Force rebuild the simulator image"
    echo "  killapp - Inject an F12 virtual hardware keypress to test the global kiosk exit"
    exit 1
    ;;
esac
