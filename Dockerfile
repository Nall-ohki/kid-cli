# Phase 1: Build the Rust binary
FROM rust:bookworm AS builder
WORKDIR /build

# Cache dependencies
COPY Cargo.toml Cargo.lock build.rs ./
# Copy assets early so build.rs can use them even during dependency caching
# (Optional: if assets are large, this might bloat the cache layer, but here they are small)
COPY assets/ ./assets/
RUN mkdir src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

# Build actual source
COPY src/ ./src/
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim

# Prevent interactive prompts during apt-get
ENV DEBIAN_FRONTEND=noninteractive

# Layer 1: System packages (Heavy lifting)
RUN apt-get update && apt-get install -y --no-install-recommends \
    zsh tmux cage foot kbd xwayland git curl ca-certificates sudo gnupg \
    tuxpaint klettres gcompris-qt tuxmath tuxtype scratch xvfb x11vnc qtwayland5 \
    mame retroarch libretro-bsnes-mercury-performance \
    sl cowsay figlet nyancat cmatrix lolcat \
    vim less file libgl1-mesa-dri rsync \
    locales procps python3-evdev \
    && rm -rf /var/lib/apt/lists/* \
    && echo "en_US.UTF-8 UTF-8" > /etc/locale.gen \
    && locale-gen en_US.UTF-8

# Layer 1.5: Install modern Docker CLI from official repo
RUN install -m 0755 -d /etc/apt/keyrings \
    && curl -fsSL https://download.docker.com/linux/debian/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg \
    && chmod a+r /etc/apt/keyrings/docker.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/debian bookworm stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null \
    && apt-get update && apt-get install -y --no-install-recommends docker-ce-cli \
    && rm -rf /var/lib/apt/lists/*

# Layer 2: Environment Setup & UV
ENV LANG=en_US.UTF-8
ENV LANGUAGE=en_US:en
ENV LC_ALL=en_US.UTF-8
RUN curl -LsSf https://astral.sh/uv/install.sh | env UV_INSTALL_DIR="/usr/local/bin" UV_NO_MODIFY_PATH=1 sh
ENV PATH="/kid/bin:/home/kid/.local/bin:${PATH}"

# Layer 3: The Kid User + /kid runtime directories
RUN getent group render || groupadd render \
    && getent group input || groupadd input \
    && useradd -m -s /bin/zsh -c "Kid User" kid \
    && usermod -aG render,video,tty,input kid \
    && echo "kid ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/kid \
    && mkdir -p /kid/bin /kid/wrap/bin /kid/allow/bin /kid/tools /kid/restricted/bin /kid/emulation/disks \
    && chown kid:kid /kid/emulation/disks

USER kid
WORKDIR /home/kid

# Layer 4: Configuration Migration (Move to hidden .config/zsh)
RUN mkdir -p /home/kid/.config/zsh
COPY --chown=kid:kid config/ /home/kid/.config/zsh/

# Layer 5: Assets and ROMs
COPY --chown=kid:kid assets/roms/ /kid/emulation/disks/

# Layer 7: Final Assembly & Hardening
USER root
COPY --from=builder /build/target/release/kid /kid/bin/kid
RUN chmod +x /kid/bin/kid

# Bootstrap kid config
USER kid
RUN rm -rf /home/kid/.config/kid && /kid/bin/kid install --user

# Hardening as root
USER root
# Create intercept binaries (COPY instead of symlink for reliability)
RUN for cmd in sudo su ssh scp sftp wall; do \
       rm -f /usr/bin/$cmd && ln -sf /kid/bin/kid /usr/bin/$cmd; \
       done \
    && HOME=/home/kid /kid/bin/kid install --system

USER kid
WORKDIR /home/kid
# Default entrypoint is fine
