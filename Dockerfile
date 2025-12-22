# Build stage for Rust binary
FROM rust:latest as builder

WORKDIR /app

# Install system dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy root workspace files
COPY Cargo.toml ./

# Copy all member Cargo.toml files
COPY cfg/Cargo.toml ./cfg/
COPY ast/Cargo.toml ./ast/
COPY lua51-lifter/Cargo.toml ./lua51-lifter/
COPY lua51-deserializer/Cargo.toml ./lua51-deserializer/
COPY restructure/Cargo.toml ./restructure/
COPY luau-lifter/Cargo.toml ./luau-lifter/
COPY luau-worker/Cargo.toml ./luau-worker/
COPY medal/Cargo.toml ./medal/

# Create stub src files for dependency caching
# Comments removed from the multi-line RUN command below
RUN mkdir -p cfg/src ast/src lua51-lifter/src lua51-deserializer/src \
    restructure/src luau-lifter/src luau-worker/src medal/src && \
    for d in lua51-lifter lua51-deserializer restructure luau-lifter medal; do \
        echo "fn main() {}" > $d/src/main.rs; \
    done && \
    for d in cfg ast luau-worker; do \
        echo "pub fn stub() {}" > $d/src/lib.rs; \
    done

# Generate fresh lock file and build dependencies
RUN cargo generate-lockfile && cargo build --release

# Copy actual source code (overwrites stubs)
COPY . .

# Build the final binary - medal is the main binary
RUN cargo build --release --bin medal

# Runtime stage
FROM node:22-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends lua5.1 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Copy Node.js files
COPY package*.json ./
RUN npm install --only=production

# Copy application code
COPY server.js ./

# Copy the built Rust binary from builder stage
COPY --from=builder /app/target/release/medal /usr/local/bin/medal

# Make binary executable and create non-root user
RUN chmod +x /usr/local/bin/medal && \
    groupadd -r nodejs && useradd -r -g nodejs nodejs
USER nodejs

EXPOSE 8080
CMD ["node", "server.js"]
