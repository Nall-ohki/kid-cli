# Agent & System Architecture Guide

This document explains how the different parts of the `kid-cli` system interact, the overarching workflow, and how applications are seamlessly managed across different environments. It is designed to help agents and developers understand the holistic architecture of the project.

## 1. Workflows

### Simulation Environment (`scripts/dev/simulate.sh`)
The simulator provides a safe, contained environment to develop and test the kid environment natively on macOS. 
- It uses **Docker Compose** (`docker-compose.sim.yml`) to build an image based on Debian Bookworm Slim, compiling the Rust binary natively inside the container.
- **Container Lifecycle**: The container's primary process (PID 1) is `sleep infinity`, which keeps the environment alive in the background without prematurely spinning up user sessions.
- **GUI Mode (`simulate.sh run --gui`)**:
  - Starts a headless X11 server (`Xvfb` on `:99`) and bridges it to macOS via `x11vnc`.
  - Executes `cage foot` as the primary user `kid` inside the container. This bootstraps the Wayland ecosystem.

### Deploying to Hardware (e.g., Raspberry Pi)
Deployment to actual hardware (like a Raspberry Pi or a dedicated Debian/Ubuntu machine) bypasses Docker entirely and operates directly on the host system.
- **`scripts/bootstrap_system.sh`**: Acts as the global system installer. It can be invoked securely via `curl | bash`.
- **Global Setup**: It clones the repository to `/opt/kid-cli`, installs necessary apt dependencies (like Wayland compositors, Rust toolchains, and educational apps), and builds the `kid` binary.
- **Admin Initialization**: After bootstrapping, an admin uses `sudo kid admin kid create <name>` to provision real, locked-down user accounts on the host machine.

## 2. The Sandbox Stack (Docker/Host)
The environment relies on a deeply integrated stack of technologies that ensure a restricted, kiosk-like experience:

1. **Cage (Wayland Compositor)**: The root display server. It acts as a strict kiosk compositor, meaning it only displays **one full-screen window at a time**. It provides the `wayland-0` socket.
2. **Foot (Terminal Emulator)**: The first application launched by `cage`. It runs full-screen and serves as the visual shell for the user. It naturally inherits `WAYLAND_DISPLAY` from `cage`.
3. **Zsh (Interactive Shell)**: `foot` runs `zsh`. `zsh` sources `~/.zshrc` (symlinked by the Rust binary), applying the restricted path constraints.
4. **Tmux (Terminal Multiplexer)**: If `tmux` isn't running, `zshrc` automatically triggers `exec tmux new-session -A -s kid`. Because `tmux` is spawned by `foot`, the `tmux` server inherently receives all Wayland routing variables. 
   - `tmux.conf` restricts the UI (hiding the status bar) and handles companion panes for background audio/monitoring.
5. **Rust Binary (`kid`)**: Acts as both the overarching manager (provisioning users, modifying `zshrc`) and the primary command interceptor/launcher.

## 3. How Apps Are Launched
Applications in the kid environment are strictly controlled and launched via the Rust binary.

1. **Symlink Interception**: The user's `$PATH` is restricted to `/kid/wrap/bin`. Every command available to the user (e.g., `gcompris`, `tuxpaint`, `ls`) is actually a symlink to `/kid/bin/kid` (the Rust binary) or a wrapper script that delegates to `kid launch <app>`.
2. **Configuration (`commands.toml`)**: When `kid launch <app>` is invoked, it reads the globally deployed `commands.toml` file to determine the application's properties (e.g., `gui`, `persist`, `pane`, `lolcat`).
3. **CLI Applications**: 
   - Non-GUI apps are launched interactively. If they require a specific layout (like a `popup` or `companion`), the Rust binary invokes `tmux display-popup` or `tmux split-window` to orchestrate the pane natively.
4. **GUI Applications (`gui = true`)**:
   - The Rust binary evaluates the current environment for `$WAYLAND_DISPLAY` or `$DISPLAY`.
   - If a graphical session exists (which it will, because `tmux` successfully inherited it from `foot`), it executes the graphical binary directly in the background natively (`std::process::Command::new("/bin/sh")`).
   - The GUI app (e.g., `gcompris-qt`) connects to `cage` via the Wayland socket.
   - **Kiosk Takeover**: Because `cage` is a kiosk compositor, it immediately brings the new GUI window to the foreground, perfectly hiding the `foot` terminal. When the child finishes playing and closes the app, `cage` seamlessly drops back to the `foot` terminal session.
5. **Daemon Events**: Surrounding the launch execution, `kid` emits `app_start` and `app_stop` events. These trigger hooks (via `/kid/wrap/bin/kid event pre/post`) that manage background services like audio/music dimming during active gameplay.
