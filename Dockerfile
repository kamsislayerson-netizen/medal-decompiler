# ============================================================================
# Stage 1: Builder - Compile the Rust workspace
# ============================================================================
FROM rust:1.75-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy all Cargo.toml files for dependency caching
COPY Cargo.toml ./
COPY cfg/Cargo.toml ./cfg/
COPY ast/Cargo.toml ./ast/
COPY lua51-lifter/Cargo.toml ./lua51-lifter/
COPY lua51-deserializer/Cargo.toml ./lua51-deserializer/
COPY restructure/Cargo.toml ./restructure/
COPY luau-lifter/Cargo.toml ./luau-lifter/
COPY luau-worker/Cargo.toml ./luau-worker/
COPY medal/Cargo.toml ./medal/

# Generate stub files to build dependency layers
RUN mkdir -p cfg/src ast/src lua51-lifter/src lua51-deserializer/src \
    restructure/src luau-lifter/src luau-worker/src medal/src && \
    for d in lua51-lifter lua51-deserializer restructure luau-lifter medal; do \
        echo "fn main() {}" > $d/src/main.rs; \
    done && \
    for d in cfg ast luau-worker; do \
        echo "pub fn stub() {}" > $d/src/lib.rs; \
    done

# Build dependencies (cached layer)
RUN cargo generate-lockfile && cargo build --release

# Copy actual source code and build final binary
COPY . .
RUN cargo build --release --bin medal

# ============================================================================
# Stage 2: Runtime - Minimal production image
# ============================================================================
FROM debian:bookworm-slim

# Install runtime dependencies (ca-certificates for HTTPS)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r medal && useradd -r -g medal medal

# Copy the built binary from builder stage
COPY --from=builder /app/target/release/medal /usr/local/bin/medal
RUN chmod +x /usr/local/bin/medal

# Switch to non-root user
USER medal

# Expose port (Render will override this with $PORT)
EXPOSE 8080

# Health check (for Docker)
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:${PORT:-8080}/health || exit 1

# Run the server (renders like serve.rs)
CMD ["medal", "serve", "--luau", "--lua51"]
