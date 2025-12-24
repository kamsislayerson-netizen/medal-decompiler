# ============================================================================
# Runtime - Use locally committed binary
# ============================================================================
FROM debian:bookworm-slim

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r medal && useradd -r -g medal medal

# Copy binary from repository root (you committed this)
COPY medal-x86_64-linux-musl /usr/local/bin/medal
RUN chmod +x /usr/local/bin/medal

# Switch to non-root user
USER medal

# Expose port
EXPOSE 10000

# Run the server
CMD ["medal", "serve", "--luau"]
