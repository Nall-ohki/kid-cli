#!/bin/zsh
# Kid-friendly native zshrc
# Designed for speed, stability, and explicit control.

# 1. Environment & Paths
if [[ -z "$HOME" ]] || [[ "$HOME" == "/var/empty" ]]; then
  export HOME=$(eval echo "~$USER")
fi
export PATH="/usr/local/bin:/usr/local/sbin:$PATH"
if [[ -d "/kid/config" ]]; then
  export ZSH_ROOT="/kid/config"
elif [[ -d "/opt/kid-cli/config" ]]; then
  export ZSH_ROOT="/opt/kid-cli/config"
else
  export ZSH_ROOT="$HOME/.config/zsh"
fi

# Force Qt to use native Wayland (avoids falling back to xcb/Xwayland)
export QT_QPA_PLATFORM=wayland
export XDG_SESSION_TYPE=wayland
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/runtime-kid}"
mkdir -m 0700 -p "$XDG_RUNTIME_DIR" 2>/dev/null || true

# tmux's async initialization causes Pane 0 to drop environment variables because the 
# session environment is updated *after* the first pane starts. 
# To guarantee survival, we write them to a cache file right before jumping into tmux,
# and source them right after.
if [[ -o interactive ]]; then
    if [[ -z "$TMUX" ]]; then
        exec tmux new-session -A -s kid
    fi
fi

# 2. History & Options
HISTFILE="$HOME/.zsh_history"
HISTSIZE=10000
SAVEHIST=10000
setopt APPEND_HISTORY
setopt INC_APPEND_HISTORY
setopt HIST_IGNORE_DUPS
setopt HIST_IGNORE_SPACE
setopt HIST_REDUCE_BLANKS

# Disable AUTO_CD 
unsetopt AUTO_CD

# 3. Completions (Native)
autoload -Uz compinit
compinit

# Case-insensitive completion
zstyle ':completion:*' matcher-list 'm:{a-z}={A-Z}'

# 4. Prompt (Native & Pretty)
# A simple, high-contrast prompt: [CWD] >
# We use colors: %F{green}, %F{blue}, %f (reset)
PROMPT='%F{green}%~%f %F{blue}❯%f '

# 5. Aliases & Colors
alias ll='ls -la'
if [[ -o interactive ]] && command -v dircolors &>/dev/null; then
    eval "$(dircolors -b)"
    alias ls='ls --color=auto'
fi

# 6. Auto-launch tmux (interactive login only)
if [[ -o interactive ]]; then
    if [ -z "$TMUX" ] && command -v tmux &> /dev/null; then
        exec tmux new-session -A -s kid
    fi
fi

# 7. Safety Intercepts & Hooks (KEEP LAST)
# Load the restricted configuration which sets the final PATH and daemon hooks
if [[ -f "${ZSH_ROOT}/zshrc_restricted.zsh" ]]; then
    source "${ZSH_ROOT}/zshrc_restricted.zsh"
fi
