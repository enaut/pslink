# Stage 1: Builder with Rust environment
FROM --platform=linux/aarch64 ubuntu:latest AS builder

# Prepare the working directory
WORKDIR /app

# Copy the binary
COPY  target/dx/web/release/web/web /app/pslink

# Generate initial files
RUN cd /app && ./pslink demo

# Stage 2: Minimal image for execution
FROM --platform=linux/aarch64 ubuntu:latest

# Copy files from builder
COPY --from=builder /app/ /app/
COPY target/dx/web/release/web/public /app/public

# Set working directory
WORKDIR /app

# Expose port
EXPOSE 8080

# Start server
CMD ["/app/pslink", "runserver", "--hostip", "0.0.0.0", "--port", "8080"]