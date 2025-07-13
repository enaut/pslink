FROM docker.io/library/alpine:latest

# Update package lists and install required packages

RUN apk update && \
    apk add --no-cache \
    pkgconfig \
    openssl-dev \
    openssl-libs-static \
    openssl \
    musl-dev \
    ca-certificates \
    build-base \
    curl \
    perl \
    bash \
    wget \
    xz

# Install musl-cross-make toolchain for aarch64
RUN cd /tmp && \
    wget https://musl.cc/aarch64-linux-musl-cross.tgz && \
    tar -xf aarch64-linux-musl-cross.tgz -C /opt && \
    rm aarch64-linux-musl-cross.tgz
ENV PATH="/opt/aarch64-linux-musl-cross/bin:${PATH}"

# Set CC and other environment variables for cross-compilation
ENV CC_aarch64_unknown_linux_musl=aarch64-linux-musl-gcc
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-musl-gcc

# Install Rust and Cargo
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --target aarch64-unknown-linux-musl
ENV PATH="/root/.cargo/bin:${PATH}"

# Install cargo-binstall for faster binary installations
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

# Install Dioxus CLI using binstall
RUN /root/.cargo/bin/cargo binstall -y dioxus-cli

# Set environment variables for OpenSSL (using Alpine's static libraries)
ENV OPENSSL_STATIC=1
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV PKG_CONFIG_ALL_STATIC=1
