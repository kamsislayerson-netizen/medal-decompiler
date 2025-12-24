# Use a slim image for the runtime
FROM debian:bookworm-slim

# Install SSL certificates for outbound requests if needed
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && rm -rf /var/lib/apt/lists/*

# Setup application directory
WORKDIR /app
RUN groupadd -r medal && useradd -r -g medal medal

# 1. Copy the binary from your repo root
COPY medal-x86_64-linux-musl /usr/local/bin/medal
RUN chmod +x /usr/local/bin/medal

# 2. Copy the UI folder from your repo root
COPY public ./public

# Final permissions
RUN chown -R medal:medal /app
USER medal

# Render default port
EXPOSE 10000

# Start command
CMD ["medal", "serve", "--luau", "--port", "10000"]
