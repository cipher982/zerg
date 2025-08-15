# Frontend Dockerfile - Build WASM from source
FROM rust:1.89-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Install wasm-pack
RUN cargo install wasm-pack

# Set work directory
WORKDIR /app

# Copy source code
COPY frontend/ .

# Set environment for build (production values)
ENV API_BASE_URL=http://backend:8000
ENV RUSTFLAGS="--cfg getrandom_backend=\"wasm_js\""

# Build WASM (following build-only.sh process)
RUN mkdir -p /tmp/wasm_build
RUN TMPDIR="/tmp/wasm_build" wasm-pack build --dev --target web --out-dir pkg

# Copy WASM artifacts to existing www directory (following build-only.sh)
RUN cp pkg/agent_platform_frontend.js www/ && \
    cp pkg/agent_platform_frontend_bg.wasm www/ && \
    cp pkg/agent_platform_frontend.d.ts www/ && \
    cp pkg/agent_platform_frontend_bg.wasm.d.ts www/

# Generate bootstrap.js (overwrites existing one with production backend URL)
RUN echo 'import init, { init_api_config_js } from "./agent_platform_frontend.js";\n\
async function main() {\n\
  await init();\n\
  const url = window.API_BASE_URL || "http://backend:8000";\n\
  init_api_config_js(url);\n\
}\n\
\n\
main();' > www/bootstrap.js

# Production stage - nginx to serve files
FROM nginx:alpine

# Copy built frontend files
COPY --from=builder /app/www /usr/share/nginx/html

# Copy nginx config from project root
COPY nginx.conf /etc/nginx/nginx.conf

# Expose port 80
EXPOSE 80