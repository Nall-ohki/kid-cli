# 🚀 Kid-CLI

A safe, educational Linux environment for children ages 3-5 on Raspberry Pi 500.

## Quick Install (Raspberry Pi)

Run this one-liner on a fresh Raspberry Pi OS (64-bit) to bootstrap the entire system:

```bash
curl -fsSL https://raw.githubusercontent.com/Nall-ohki/kid-cli/main/scripts/setup_system.sh | sudo bash
```

## Setup & Usage

### 1. Provision Kids
After the one-liner setup is complete, use the initialization script to create the children's accounts and set their passwords:
```bash
sudo /opt/kid-cli/scripts/init.sh
```

### 2. Updating the System
If you want to pull the latest code and refresh the educational environment:
```bash
sudo kid admin deploy
```

### 3. Simulation (Development)
To test the environment on your local machine (Mac/Linux) without hardware:
```bash
./scripts/simulate.sh run
```

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

## Hardware Requirements
- **Raspberry Pi 5 / 400 / 4** (8GB recommended)
- **Raspberry Pi OS (64-bit)**
- **GitHub CLI (gh)** - Authenticated for repo access
