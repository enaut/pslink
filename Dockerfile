# Stage 1: Prepare application bundle and initialize demo database
FROM ubuntu:26.04 AS builder

# Prepare the working directory
WORKDIR /app

# Copy the bundled web directory (contains public/ and the server binary)
COPY target/dx/pslink/release/web/ /tmp/bundle/

# Normalize binary name (dx bundle outputs 'server', CI may rename it to 'pslink')
# and generate initial demo files (.env and links.db)
RUN if [ -f /tmp/bundle/pslink ]; then \
        cp /tmp/bundle/pslink /app/pslink; \
    elif [ -f /tmp/bundle/server ]; then \
        cp /tmp/bundle/server /app/pslink; \
    else \
        echo "Error: neither 'pslink' nor 'server' binary found in target/dx/pslink/release/web/" >&2; \
        exit 1; \
    fi \
    && chmod +x /app/pslink \
    && cp -r /tmp/bundle/public /app/public \
    && if [ -f /tmp/bundle/.manifest.json ]; then cp /tmp/bundle/.manifest.json /app/.manifest.json; fi \
    && ./pslink demo

# Stage 2: Minimal runtime image for execution
FROM ubuntu:26.04

# CA certificates are required to check https link targets
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy prepared files from builder
COPY --from=builder /app/ /app/

# Set working directory
WORKDIR /app

# Expose port
EXPOSE 8080

# Start server
CMD ["/app/pslink", "runserver", "--hostip", "0.0.0.0", "--port", "8080"]
