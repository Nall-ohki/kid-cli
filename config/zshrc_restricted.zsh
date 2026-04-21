# Restricted Shell Configuration
# Sourced at the logic end of .zshrc to ensure it overrides everything else

# 0. Save Infrastructure PATH
# Save the full infrastructure PATH before restricting
export _INFRA_PATH="$PATH:/usr/games"

# Set EDITOR to full vim path for Ctrl-X Ctrl-E command editing
export EDITOR="/usr/bin/vim"

# Disable the % indicator for output without trailing newline
export PROMPT_EOL_MARK=''

# Disable Ctrl-D exiting the shell
setopt IGNORE_EOF

# 1. Restrict PATH to kid directories only
# /kid/wrap/bin       - student-facing wrappers (take priority)
# /kid/allow/bin      - allowed system commands (symlinks)
# /kid/restricted/bin - security intercepts (sudo, ssh, etc)
export PATH="/kid/wrap/bin:/kid/allow/bin:/kid/restricted/bin"

# 2. Clear aliases that might shadow kid binaries
unalias ls 2>/dev/null || true
unalias grep 2>/dev/null || true
unalias cat 2>/dev/null || true
unalias rm 2>/dev/null || true
unalias cp 2>/dev/null || true
unalias mv 2>/dev/null || true
unalias home 2>/dev/null || true
unalias ll 2>/dev/null || true

# Re-hash to ensure shell knows about new path instantly
rehash

# Define home function
home() {
  # Validate then move
  /kid/wrap/bin/home && builtin cd ~
}

# Override cd builtin
cd() {
  # Validate directory then change
  /kid/wrap/bin/cd "$@" && builtin cd "$@"
}

# Override exit
exit() {
  if [[ -o interactive ]]; then
    /kid/wrap/bin/exit "$@"
  else
    builtin exit "$@"
  fi
}

# 3. Safety Intercepts
# Blocked commands are handled by binaries in /kid/restricted/bin

# 4. Daemon Hooks
preexec() {
  # Don't fire 'pre' for cd, handled by chpwd
  [[ "$1" == cd* ]] || /kid/bin/kid event pre "$1" "$TMUX_PANE" &!
}

chpwd() {
  /kid/bin/kid event pre "cd" "$TMUX_PANE" &!
}

precmd() {
  /kid/bin/kid event post "$?" "$TMUX_PANE" &!
}

# 5. Daemon Auto-Launch (Session-Aware Take-over)
# Only attempt startup if inside a tmux session
if [[ -n "$TMUX" ]]; then
  /kid/bin/kid watch --daemon >/dev/null 2>&1
  # Signal readiness to test driver
  touch /tmp/kid_ready
fi

