## Frontend Dockerfile – Build WASM and serve via nginx
FROM rust:1.89-slim AS builder

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install wasm-pack and wasm target
RUN cargo install wasm-pack && rustup target add wasm32-unknown-unknown

WORKDIR /app

# Copy frontend sources
COPY . .

# Build artifacts with explicit API_BASE_URL (overridable via build-arg)
ARG API_BASE_URL=https://api.swarmlet.com
ENV RUSTFLAGS="--cfg getrandom_backend=\"wasm_js\""

# Use the project build script to generate WASM, index.html (with CSP), and bootstrap.js
RUN chmod +x build-debug.sh && \
    BUILD_ONLY=true \
    API_BASE_URL=${API_BASE_URL} \
    ./build-debug.sh

FROM nginx:alpine

# Copy nginx config and built assets
COPY nginx.conf /etc/nginx/nginx.conf
COPY --from=builder /app/www/ /usr/share/nginx/html/

# Health: ensure key files exist
RUN test -f /usr/share/nginx/html/index.html && \
    test -f /usr/share/nginx/html/agent_platform_frontend.js && \
    test -f /usr/share/nginx/html/agent_platform_frontend_bg.wasm

EXPOSE 80
