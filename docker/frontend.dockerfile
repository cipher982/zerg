# Simple frontend Dockerfile - Build WASM and serve with nginx
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
COPY . .

# Set environment for WASM builds
ENV RUSTFLAGS="--cfg getrandom_backend=\"wasm_js\""

# Build WASM (no optimization to avoid timeouts)
RUN wasm-pack build --release --target web --no-opt --out-dir www/pkg

# Serve with nginx
FROM nginx:alpine
COPY --from=builder /app/www /usr/share/nginx/html

# Simple nginx config for SPA
RUN echo 'server { \
    listen 80; \
    root /usr/share/nginx/html; \
    index index.html; \
    location / { \
        try_files $uri $uri/ /index.html; \
    } \
}' > /etc/nginx/conf.d/default.conf

EXPOSE 80