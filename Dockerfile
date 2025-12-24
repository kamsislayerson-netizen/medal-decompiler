# ============================================================================
# Stage 1: Builder - Compile the Rust workspace
# ============================================================================
FROM rust:1.75-slim as builder

WORKDIR /app

# Install build dependencies (include sed for text replacement)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev sed && rm -rf /var/lib/apt/lists/*

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
    for d in cfg ast lua51-lifter lua51-deserializer restructure luau-lifter luau-worker; do \
        echo "pub fn stub() {}" > $d/src/lib.rs; \
    done && \
    echo "fn main() {}" > medal/src/main.rs

# Build dependencies (cached layer)
RUN cargo generate-lockfile && cargo build --release

# Copy actual source code
COPY . .

# Fix let chains syntax for Rust 2021 compatibility
RUN sed -i 's/&& let Some((_, next)) = iter.peek()/&& matches!(iter.peek(), Some((_, _)))/g' ast/src/formatter.rs && \
    sed -i 's/&& let RValue::Closure(closure) = &assign.right\[0\]/&& matches!(&assign.right[0], RValue::Closure(_))/g' ast/src/formatter.rs && \
    sed -i 's/if let box RValue::Literal(Literal::String(key)) = &index.right/if let RValue::Literal(Literal::String(key)) = &*index.right/g' ast/src/formatter.rs && \
    sed -i 's/while let (block, parent_stat_index) = self.graph.node_weight(node).unwrap()/while let Some((block, parent_stat_index)) = self.graph.node_weight(node)/g' ast/src/local_declarations.rs && \
    sed -i 's/.unwrap()//g' ast/src/local_declarations.rs

RUN cargo build --release --bin medal

# ============================================================================
# Stage 2: Runtime - Minimal production image
# ============================================================================
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r medal && useradd -r -g medal medal

# Copy the built binary from builder stage
COPY --from=builder /app/target/release/medal /usr/local/bin/medal
RUN chmod +x /usr/local/bin/medal

# Switch to non-root user
USER medal

# Expose port
EXPOSE 10000

# Run the server
CMD ["medal", "serve", "--luau"]
