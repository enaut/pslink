# Stage 1: Builder mit Rust-Umgebung
FROM rust:slim AS builder

# Installation notwendiger Build-Tools
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    curl \
    musl-tools \
    musl-dev \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Rust-Komponenten und Tools installieren
RUN rustup target add wasm32-unknown-unknown
# Install rustup and set the target platform
ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
      "linux/amd64") rustup target add x86_64-unknown-linux-musl ;; \
      "linux/arm64") rustup target add aarch64-unknown-linux-musl ;; \
    esac
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall -y dioxus-cli

# Arbeitsverzeichnis vorbereiten
WORKDIR /build
COPY . .

# Build ausführen
RUN mkdir -p /pslink

RUN dx bundle --platform web --package web --release --out-dir /pslink

ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
      "linux/amd64") dx bundle --platform server --package web --out-dir /pslink/server --release -- --target x86_64-unknown-linux-musl ;; \
      "linux/arm64") dx bundle --platform server --package web --out-dir /pslink/server --release -- --target aarch64-unknown-linux-musl ;; \
    esac
RUN mv musl/server/web/web /pslink/pslink
RUN rm -f /pslink/server
WORKDIR /pslink
RUN /pslink/pslink demo

# Stage 2: Minimales Image für die Ausführung 
FROM scratch

# Arbeitsverzeichnis erstellen
WORKDIR /app

# Statisch kompilierte Binaries und Assets kopieren
COPY --from=builder /pslink/ /app/

# Port freigeben
EXPOSE 8080

# Server starten
CMD ["/app/pslink", "runserver", "--hostip", "0.0.0.0", "--port", "8080"]