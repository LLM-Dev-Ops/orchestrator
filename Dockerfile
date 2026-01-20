# Multi-stage build for minimal production image
FROM rustlang/rust:nightly AS builder

WORKDIR /build

# Copy all source code (no dummy file caching - cleaner approach)
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build the release binary
RUN cargo build --release --bin llm-orchestrator

# Verify the binary was built
RUN ls -la /build/target/release/llm-orchestrator && \
    /build/target/release/llm-orchestrator --version

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 orchestrator

# Copy binary from builder
COPY --from=builder /build/target/release/llm-orchestrator /usr/local/bin/llm-orchestrator

# Set ownership
RUN chown orchestrator:orchestrator /usr/local/bin/llm-orchestrator

# Switch to non-root user
USER orchestrator

# Set working directory
WORKDIR /home/orchestrator

# Health check - uses the HTTP health endpoint
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD curl -sf http://localhost:8080/health || exit 1

# Default command - serve for Cloud Run
ENTRYPOINT ["llm-orchestrator"]
CMD ["serve"]
