# =============================================================================
# Jarvis Daemon - Docker Image
# =============================================================================
# Multi-stage build: compile Rust binary, then minimal runtime image.
#
# Build (from jarvis-ext/):  docker build -t jarvis-daemon -f daemon/Dockerfile .
# Run:  docker run -d --env-file .env -v jarvis-data:/home/jarvis/.jarvis jarvis-daemon
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Build
# -----------------------------------------------------------------------------
FROM rust:1.83-bookworm AS builder

WORKDIR /build

# Install build dependencies (openssl for reqwest, pkg-config)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy full workspace (required for cargo build -p jarvis-daemon)
# jarvis-ext/ depends on codex-rs/core, so both must be present
COPY . .

# Build release binary
RUN cargo build --release --bin jarvis-daemon

# -----------------------------------------------------------------------------
# Stage 2: Runtime
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim

# Install runtime dependencies (ca-certificates for HTTPS, libssl for reqwest)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 jarvis

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/jarvis-daemon /app/jarvis-daemon

# Daemon runs as non-root; data goes to /home/jarvis/.jarvis
ENV HOME=/home/jarvis
ENV JARVIS_DAEMON_DB=/home/jarvis/.jarvis/daemon.db

USER jarvis

# Ensure data directory exists
RUN mkdir -p /home/jarvis/.jarvis/credentials

# Default: run the daemon (override with docker run)
ENTRYPOINT ["/app/jarvis-daemon"]
CMD ["run", "--tick-interval-sec", "60", "--max-concurrent", "3"]
