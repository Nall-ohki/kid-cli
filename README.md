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

All configs are symlinked from `~/login/kid/config`:

- **ZSH**: `config/zshrc.zsh`, `config/prompt.zsh`
- **Tmux**: `config/tmux.conf.local`

## Installation Scripts

Located in `kid/install/`:

1. `install_kid_user.sh` - Creates kid user
2. `install_kid_structure.sh` - Creates directory tree
3. `install_kid_software.sh` - Installs educational apps
4. `install_kid_launchers.sh` - Creates app launchers
5. `install_kid_shell.sh` - Calls existing install_zsh.sh and install_tmux.sh as kid user
6. `install_kid_links.sh` - Creates config symlinks
7. `install_kid.sh` - Main installer (runs all above)


