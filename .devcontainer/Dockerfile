FROM mcr.microsoft.com/devcontainers/base:ubuntu-24.04

# Install system dependencies
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
        curl \
        git \
        build-essential \
        pkg-config \
        libssl-dev \
        libsqlite3-dev \
        sqlite3 \
        python3 \
        python3-pip \
        python3-venv \
        ca-certificates \
        unzip \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js 20
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# Install Rust (stable) for the vscode user
USER vscode
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/home/vscode/.cargo/bin:${PATH}"

# Install Encore CLI for the vscode user
RUN curl -L https://encore.dev/install.sh | bash
ENV PATH="/home/vscode/.encore/bin:${PATH}"

USER root
