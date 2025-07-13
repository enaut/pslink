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
    bash

# Install Rust and Cargo
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --target x86_64-unknown-linux-musl
ENV PATH="/root/.cargo/bin:${PATH}"

# Install cargo-binstall for faster binary installations
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

# Install Dioxus CLI using binstall
RUN /root/.cargo/bin/cargo binstall -y dioxus-cli

# Set environment variables for OpenSSL (using Alpine's static libraries)
ENV OPENSSL_STATIC=1
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV PKG_CONFIG_ALL_STATIC=1
