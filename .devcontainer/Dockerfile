FROM mcr.microsoft.com/devcontainers/base:ubuntu

# Install Rust toolchain as vscode user
USER vscode
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && . "$HOME/.cargo/env" \
    && rustup component add rust-analyzer
ENV PATH="/home/vscode/.cargo/bin:${PATH}"
