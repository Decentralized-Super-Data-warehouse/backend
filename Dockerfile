# Stage 1: Build the Rust project
FROM rust:1.80.0-slim AS builder

WORKDIR /app

# Install OpenSSL development packages and other necessary tools
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev build-essential && \
    rm -rf /var/lib/apt/lists/*

# Copy your source code
COPY . .

# Build the project
RUN cargo build --release

# Stage 2: Create the runtime image
FROM debian:bookworm-slim

WORKDIR /app

# Install OpenSSL library
RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/spectra-backend .

# Copy the .env file
COPY .env .

EXPOSE 8080

# Run the binary
CMD ["./spectra-backend"]
