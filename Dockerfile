# ============================================================================
# Runtime - Download and run pre-compiled binary
# ============================================================================
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl wget && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r medal && useradd -r -g medal medal

# Download binary with retry and verification
RUN BINARY_URL="https://github.com/kamsislayerson-netizen/medal-decompiler/releases/latest/download/medal-x86_64-linux-musl" && \
    echo "Downloading from $BINARY_URL..." && \
    wget --timeout=30 --tries=3 --retry-connrefused "$BINARY_URL" -O /tmp/medal && \
    # Verify it's actually a binary (not HTML error)
    file /tmp/medal | grep -q "ELF 64-bit LSB executable" || (echo "Download failed or not a binary!" && exit 1) && \
    mv /tmp/medal /usr/local/bin/medal && \
    chmod +x /usr/local/bin/medal && \
    # Verify it can execute
    /usr/local/bin/medal --version || (echo "Binary is corrupted!" && exit 1)

# Switch to non-root user
USER medal

# Expose port
EXPOSE 10000

# Run the server
CMD ["medal", "serve", "--luau"]
