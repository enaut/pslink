# Stage 1: Builder with Rust environment
FROM --platform=linux/x86_64 alpine AS builder

# Prepare the working directory
WORKDIR /app

# Copy the binary
COPY target/x86_64-unknown-linux-musl/release/web /app/pslink

# Generate initial files
RUN cd /app && ./pslink demo

# Stage 2: Minimal image for execution
FROM --platform=linux/x86_64 scratch

# Copy files from builder
COPY --from=builder /app/ /app/
COPY target/dx/web/release/web/public /app/public

# Set working directory
WORKDIR /app

# Expose port
EXPOSE 8080

# Start server
CMD ["/app/pslink", "runserver", "--hostip", "0.0.0.0", "--port", "8080"]