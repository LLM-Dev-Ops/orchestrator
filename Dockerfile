# Multi-stage build for minimal production image
FROM rust:1.75 as builder

WORKDIR /build

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build dependencies (cached layer)
RUN mkdir -p crates/llm-orchestrator-cli/src && \
    echo "fn main() {}" > crates/llm-orchestrator-cli/src/main.rs && \
    cargo build --release && \
    rm -rf crates/llm-orchestrator-cli/src

# Copy source code
COPY crates/ ./crates/

# Build application
RUN cargo build --release --bin llm-orchestrator

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    ca-certificates \
    libssl3 \
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

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD llm-orchestrator --version || exit 1

# Default command
ENTRYPOINT ["llm-orchestrator"]
CMD ["--help"]
