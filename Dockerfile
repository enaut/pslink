# Stage 1: Use debian-slim as the base image for building
FROM ubuntu:latest AS builder

# Install necessary libraries
RUN apt-get update && apt-get install -y \
    libssl3 \
    libcrypto++8 \
    libgcc-s1 \
    libstdc++6 \
    zlib1g \
    && rm -rf /var/lib/apt/lists/*

# Stage 2: Create the final image
FROM ubuntu:latest

# Copy necessary libraries from the builder stage
COPY --from=builder /usr/lib/x86_64-linux-gnu/libssl.so.3 /usr/lib/x86_64-linux-gnu/
COPY --from=builder /usr/lib/x86_64-linux-gnu/libcrypto.so.3 /usr/lib/x86_64-linux-gnu/
COPY --from=builder /lib/x86_64-linux-gnu/libgcc_s.so.1 /lib/x86_64-linux-gnu/
COPY --from=builder /lib/x86_64-linux-gnu/libm.so.6 /lib/x86_64-linux-gnu/
COPY --from=builder /lib/x86_64-linux-gnu/libc.so.6 /lib/x86_64-linux-gnu/
COPY --from=builder /lib64/ld-linux-x86-64.so.2 /lib64/
COPY --from=builder /lib/x86_64-linux-gnu/libz.so.1 /lib/x86_64-linux-gnu/

# Create a non-root user to run the application
RUN useradd -m appuser

# Copy the server binary and set permissions
COPY target/dx/web/release/web/server /app/server
RUN chmod +x /app/server

# Copy the public directory
COPY target/dx/web/release/web/public /app/public

# Set the working directory
WORKDIR /app

# Change ownership of the application files to the non-root user
RUN chown -R appuser:appuser /app

# Switch to the non-root user
USER appuser

# Expose the necessary port
EXPOSE 8080

# Run demo data creation and then start the server
CMD ["/bin/bash", "-c", "/app/server demo && /app/server runserver"]
