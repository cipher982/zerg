# Simple frontend Dockerfile - Build WASM and serve with nginx
FROM rust:1.89-slim AS builder

# Accept build argument for production API URL (same origin for production)
ARG API_BASE_URL=""

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
COPY . .

# Set environment for WASM builds
ENV RUSTFLAGS="--cfg getrandom_backend=\"wasm_js\""

# Build WASM (no optimization to avoid timeouts) - output to www root for direct access
RUN wasm-pack build --release --target web --no-opt --out-dir www

# Generate bootstrap.js with production API URL
# In Coolify deployment, both services are behind same reverse proxy
# Frontend: https://swarmlet.com/ 
# Backend:  https://swarmlet.com/ (Coolify routes internally)
RUN printf 'import init, { init_api_config_js } from "./agent_platform_frontend.js";\n\nasync function main() {\n  await init();\n  // Use same origin - Coolify reverse proxy handles backend routing\n  const url = window.API_BASE_URL || window.location.origin;\n  init_api_config_js(url);\n}\n\nmain();\n' > www/bootstrap.js

# Serve with nginx
FROM nginx:alpine
COPY --from=builder /app/www /usr/share/nginx/html

# Replace default nginx config entirely
RUN printf 'events {\n    worker_connections 1024;\n}\n\nhttp {\n    include /etc/nginx/mime.types;\n    default_type application/octet-stream;\n    \n    server {\n        listen 80;\n        root /usr/share/nginx/html;\n        index index.html;\n        \n        location / {\n            try_files $uri $uri/ /index.html;\n        }\n    }\n}' > /etc/nginx/nginx.conf

# CRITICAL: Validate nginx config during build - fail fast if malformed  
RUN nginx -t

# CRITICAL: Verify our application files exist and are accessible
RUN test -f /usr/share/nginx/html/index.html || (echo "ERROR: index.html missing" && exit 1)
RUN test -f /usr/share/nginx/html/agent_platform_frontend.js || (echo "ERROR: WASM JS missing" && exit 1)
RUN test -f /usr/share/nginx/html/agent_platform_frontend_bg.wasm || (echo "ERROR: WASM binary missing" && exit 1)

EXPOSE 80