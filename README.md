# Kid-Friendly Linux Learning Environment

A safe, educational Linux environment for children ages 3-5 on Raspberry Pi 500.

## Quick Start

On the child's Raspberry Pi:

```bash
# 1. Build the Docker environment
sudo docker compose build

# 2. Run the environment
sudo docker compose up -d

# 3. Access the Kid Shell
sudo docker exec -it kid-env-kid-1 /bin/zsh -l
```

> **Note:** The environment uses `--privileged` to access GPU (`/dev/dri`) and input devices (`/dev/input`) so that GUI Wayland applications (like Tux Paint) can render directly to the Raspberry Pi's display.

## What's Included

### User Environment
- **User**: `kid` with zsh shell
- **Theme**: Vibrant cyberpunk bullettrain prompt
- **Session**: Auto-launches tmux, exits on tmux quit
- **Git Protection**: Repository set to read-only

### Educational Software
- **Tux Paint** - Digital art (~/apps/art/tuxpaint)
- **KLettres** - Alphabet learning (~/apps/letters/klettres)
- **Tux Math** - Math games (~/apps/math/tuxmath)
- **Tux Typing** - Typing practice (~/apps/typing/tuxtype)
- **GCompris** - Educational suite (~/apps/learning/gcompris)
- **Scratch** - Visual programming (~/apps/programming/scratch)

### Directory Structure
```
~/apps/
  tuxpaint/
  klettres/
  tuxmath/
  tuxtype/
  gcompris/
  scratch/
~/creations/
  pictures/     # Saved artwork
  programs/     # Scratch projects
  games/        # Other creations
```

## Platform Support

- **Primary**: Raspberry Pi OS (Debian-based)

## Configuration Files

All configs are managed by the `kid` CLI:

- **ZSH**: `config/zshrc.zsh`, `config/prompt.zsh`
- **Tmux**: `config/tmux.conf`

## Installation

The environment is bootstrapped via the Rust CLI tool:

```bash
/kid/bin/kid install
```

This command:
1. Creates the directory structure (`~/apps`, `~/creations`).
2. Symlinks the shell and tmux configurations.
3. Installs educational application wrappers.
4. (Optional) With `--safebin`, creates the security proxies in `/kid/wrap/bin`.


