# ============================================================================
# Runtime - Download and run pre-compiled binary
# ============================================================================
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r medal && useradd -r -g medal medal

# Download the Linux binary from GitHub Releases
RUN curl -L https://github.com/kamsislayerson-netizen/medal-decompiler/releases/latest/download/medal-x86_64-linux-musl -o /usr/local/bin/medal && \
    chmod +x /usr/local/bin/medal

# Switch to non-root user
USER medal

# Expose port
EXPOSE 10000

# Run the server
CMD ["medal", "serve", "--luau"]
