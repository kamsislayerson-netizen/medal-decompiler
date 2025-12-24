# ============================================================================
# Runtime - Use locally committed binary
# ============================================================================
FROM debian:bookworm-slim

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r medal && useradd -r -g medal medal

# 1. Copy the binary
COPY medal-x86_64-linux-musl /usr/local/bin/medal
RUN chmod +x /usr/local/bin/medal

# 2. COPY THE UI FILES (This is the missing step)
# This copies your 'public' folder from your computer into the Docker image
WORKDIR /home/medal
COPY public ./public

# Switch to non-root user
RUN chown -R medal:medal /home/medal
USER medal

# Expose port (Render usually uses 10000 by default)
EXPOSE 10000

# Run the server
# Added '0.0.0.0' and port flag to ensure it binds to Render's network
CMD ["medal", "serve", "--luau", "--port", "10000"]
