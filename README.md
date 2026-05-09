# 🚀 Kid-CLI

A safe, educational Linux environment for children ages 3-5 on Raspberry Pi 500.

## Quick Install (Raspberry Pi)

Run this one-liner on a fresh Raspberry Pi OS (64-bit) to bootstrap the entire system:

```bash
curl -fsSL https://raw.githubusercontent.com/Nall-ohki/kid-cli/main/scripts/setup.sh | sudo bash
```

## Setup & Usage

### 1. Provision Kids
After the one-liner setup is complete, use the management script to create the children's accounts and set their passwords:
```bash
sudo /opt/kid-cli/scripts/manage_kids.sh
```

### 2. Updating the System
If you want to pull the latest code and refresh the educational environment:
```bash
sudo kid admin deploy
```

### 3. Development & Simulation
To test the environment on your local machine (Mac/Linux) or deploy from your dev box:
- **Simulator**: `./scripts/dev/simulate.sh run`
- **Remote Deploy**: `./scripts/dev/deploy.sh <pi_ip_address>`

## What's Included

### User Environment
- **User Isolation**: Isolated Docker container per user
- **UI**: Automatic tmux session management with restricted shell
- **Theme**: Vibrant cyberpunk bullettrain prompt

### Educational Software
- **Tux Paint** - Digital art
- **KLettres** - Alphabet learning
- **Tux Math** - Math games
- **Tux Typing** - Typing practice
- **GCompris** - Full educational suite
- **Scratch** - Visual programming
- **CMatrix/Nyancat** - Fun terminal toys

## Project Structure
- `scripts/setup.sh`: Master system installer.
- `scripts/manage_kids.sh`: Interactive kid account manager.
- `scripts/dev/`: Developer tools (deployment, simulation).
- `scripts/internal/`: Automation helpers (binary building, toolchain install).
