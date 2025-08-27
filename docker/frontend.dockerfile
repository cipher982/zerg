# Frontend Dockerfile with runtime configuration
FROM rust:1.89-slim AS builder

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install wasm-pack
RUN cargo install wasm-pack

# Add wasm target
RUN rustup target add wasm32-unknown-unknown

WORKDIR /app
# Copy current directory (frontend) for build
COPY . .

# Set environment for WASM builds
ENV RUSTFLAGS="--cfg getrandom_backend=\"wasm_js\""

# Build with placeholder URL - will be replaced at runtime
RUN chmod +x build-debug.sh && \
    BUILD_ONLY=true \
    BACKEND_PORT=8001 \
    API_BASE_URL="RUNTIME_PLACEHOLDER" \
    ./build-debug.sh

# Production stage with nginx
FROM nginx:alpine

# Install runtime dependencies
RUN apk add --no-cache bash

# Copy custom nginx configuration with WASM support
COPY nginx.conf /etc/nginx/nginx.conf

# Copy built frontend files
COPY --from=builder /app/www/ /usr/share/nginx/html/

# Copy entrypoint script from context
COPY frontend-entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Validate nginx config
RUN nginx -t

# Verify critical files exist
RUN test -f /usr/share/nginx/html/index.html || (echo "ERROR: index.html missing" && exit 1)
RUN test -f /usr/share/nginx/html/agent_platform_frontend.js || (echo "ERROR: WASM JS missing" && exit 1)
RUN test -f /usr/share/nginx/html/agent_platform_frontend_bg.wasm || (echo "ERROR: WASM binary missing" && exit 1)

EXPOSE 80

# Use entrypoint for runtime configuration
ENTRYPOINT ["/entrypoint.sh"]