# dev/sim.Dockerfile
FROM rust:bookworm

# 1. Install System Dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    apt-transport-https \
    ca-certificates \
    curl \
    gnupg \
    lsb-release \
    sudo \
    zsh \
    git \
    rsync \
    procps \
    && rm -rf /var/lib/apt/lists/*

# 2. Install Docker CLI (to talk to the host socket)
RUN mkdir -p /etc/apt/keyrings \
    && curl -fsSL https://download.docker.com/linux/debian/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/debian $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null \
    && apt-get update && apt-get install -y docker-ce-cli \
    && rm -rf /var/lib/apt/lists/*

# 3. Install GitHub CLI (gh)
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg \
    && chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
    && apt-get update && apt-get install -y gh \
    && groupadd -f docker \
    && rm -rf /var/lib/apt/lists/*

# 4. Setup 'admin' user with passwordless sudo
RUN useradd -m -s /bin/zsh admin \
    && usermod -aG sudo admin \
    && echo "admin ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/admin \
    && touch /home/admin/.zshrc && chown admin:admin /home/admin/.zshrc

USER admin
WORKDIR /opt/kid-cli

# Set ZSH as default for the shell
ENV SHELL=/bin/zsh
ENTRYPOINT ["/bin/zsh"]
