# 🚀 Kid-CLI

A safe, educational Linux environment for children ages 3-5 on Raspberry Pi 500.

## Quick Install (Raspberry Pi)

Run this one-liner on a fresh Raspberry Pi OS (64-bit) to bootstrap the entire system:

```bash
curl -fsSL https://raw.githubusercontent.com/Nall-ohki/kid-cli/main/scripts/bootstrap_system.sh | sudo bash
```

> [!NOTE]
> The Docker environment builds **Just-In-Time (JIT)** when a kid logs in for the first time. To build it manually ahead of time, run `sudo kid admin build`.

## Setup & Usage

### 1. Provision Kids
After the one-liner bootstrap is complete, use the management script to create the children's accounts and set their passwords:
```bash
sudo /opt/kid-cli/scripts/manage_kids.sh
```

### 2. Check System Health
You can view a detailed status report of dependencies, kid accounts, and Docker images:
```bash
sudo kid admin status
```

### 3. Updating the System
Pull the latest code and re-build the administrative tools. The educational environment will update automatically on the next login.
```bash
sudo kid admin deploy
```

### 3. Development & Simulation
To test the environment on your local machine (Mac/Linux) or deploy from your dev box:
- **Simulator**: `./scripts/dev/simulate.sh run`
- **Remote Deploy**: `./scripts/dev/deploy_remote.sh <pi_ip_address>`

## Project Structure
- `scripts/bootstrap_system.sh`: Master system installer (one-liner).
- `scripts/manage_kids.sh`: Interactive kid account manager.
- `scripts/dev/`: Developer tools (deployment_remote, simulation).
- `scripts/internal/`: Automation helpers (dependency install, binary building).
