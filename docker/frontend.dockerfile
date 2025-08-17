# Frontend Dockerfile - Optimized Rust/WASM build with caching
# Production-grade build system for Zerg AI Agent Platform Frontend

# Cargo dependencies caching stage
FROM rust:1.89-slim AS dependencies

# Install system dependencies for Rust builds
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Add wasm target for dependency caching
RUN rustup target add wasm32-unknown-unknown

# Create a minimal project structure for dependency caching
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "pub fn dummy() {}" > src/lib.rs

# Set environment for WASM builds
ENV RUSTFLAGS="--cfg getrandom_backend=\"wasm_js\""

# Cache dependencies build
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --target wasm32-unknown-unknown && \
    rm -rf src target/wasm32-unknown-unknown/release/deps/agent_platform*

# WASM builder stage with optimizations
FROM rust:1.89-slim AS builder

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install wasm-pack with caching
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo install wasm-pack

# Add wasm target
RUN rustup target add wasm32-unknown-unknown

WORKDIR /app

# Copy dependency artifacts from cache stage
COPY --from=dependencies /usr/local/cargo /usr/local/cargo

# Copy source code
COPY . .

# Build argument for environment configuration
ARG BUILD_ENV=production
ARG API_BASE_URL=http://backend:8000

# Set build environment variables for WASM
ENV API_BASE_URL=${API_BASE_URL}
ENV RUSTFLAGS="--cfg getrandom_backend=\"wasm_js\""

# Build WASM with cache mounts for maximum performance
RUN --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/tmp/wasm_build \
    --mount=type=cache,target=/usr/local/cargo/registry \
    mkdir -p /tmp/wasm_build && \
    TMPDIR="/tmp/wasm_build" wasm-pack build \
    --release \
    --target web \
    --out-dir pkg

# Copy WASM artifacts to www directory
RUN cp pkg/agent_platform_frontend.js www/ && \
    cp pkg/agent_platform_frontend_bg.wasm www/ && \
    cp pkg/agent_platform_frontend.d.ts www/ && \
    cp pkg/agent_platform_frontend_bg.wasm.d.ts www/

# Generate production bootstrap.js with configurable backend URL
RUN echo 'import init, { init_api_config_js } from "./agent_platform_frontend.js";\
async function main() {\
  await init();\
  const url = window.API_BASE_URL || "'${API_BASE_URL}'";\
  init_api_config_js(url);\
}\
main();' > www/bootstrap.js

# Production nginx stage with security hardening
FROM nginx:1.25-alpine AS production

# Create non-root user for nginx
RUN addgroup -g 1000 zerg && \
    adduser -D -s /bin/sh -u 1000 -G zerg zerg

# Copy built frontend files
COPY --from=builder --chown=zerg:zerg /app/www /usr/share/nginx/html

# Copy nginx configuration
COPY nginx.conf /etc/nginx/nginx.conf

# Create nginx directories with proper permissions
RUN mkdir -p /var/cache/nginx /var/log/nginx /var/run && \
    chown -R zerg:zerg /var/cache/nginx /var/log/nginx /var/run /usr/share/nginx/html

# Switch to non-root user
USER zerg

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:80/ || exit 1

# Expose port 80
EXPOSE 80

# Start nginx in foreground
CMD ["nginx", "-g", "daemon off;"]

# Development stage with hot reload capabilities
FROM builder AS development

# Install additional development tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    inotify-tools \
    python3 \
    && rm -rf /var/lib/apt/lists/*

# Set development environment
ENV RUST_LOG=debug
ENV API_BASE_URL=http://localhost:8000

# Development server script for hot reload
RUN echo '#!/bin/bash\n\
cd /app/www\n\
python3 -m http.server 8080 &\n\
PID=$!\n\
echo "Development server started on port 8080 (PID: $PID)"\n\
\n\
# Watch for changes and rebuild\n\
while inotifywait -r -e modify,create,delete /app/src /app/Cargo.toml 2>/dev/null; do\n\
  echo "Changes detected, rebuilding..."\n\
  cd /app\n\
  wasm-pack build --dev --target web --out-dir pkg\n\
  cp pkg/*.js pkg/*.wasm pkg/*.d.ts www/\n\
  echo "Rebuild complete"\n\
done\n\
' > /usr/local/bin/dev-server.sh && \
chmod +x /usr/local/bin/dev-server.sh

EXPOSE 8080

# Development command
CMD ["/usr/local/bin/dev-server.sh"]