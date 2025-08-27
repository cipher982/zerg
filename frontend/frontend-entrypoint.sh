#!/bin/sh
set -e

# Runtime configuration script for frontend
# This script runs when the container starts and configures the API endpoint

# Get environment variables with defaults
BACKEND_PORT="${BACKEND_PORT:-47300}"
API_BASE_URL="${API_BASE_URL:-http://backend:${BACKEND_PORT}}"

echo "Configuring frontend with API_BASE_URL: ${API_BASE_URL}"

# Create config.js with runtime values
cat > /usr/share/nginx/html/config.js <<EOF
// Runtime configuration - generated at container startup
window.__APP_CONFIG__ = window.__APP_CONFIG__ || {};
window.__APP_CONFIG__.BUILD = 'production';
window.API_BASE_URL = '${API_BASE_URL}';
console.log('Frontend configured with API_BASE_URL:', window.API_BASE_URL);
EOF

# Update CSP in index.html to allow the backend URL
# Extract the backend host from API_BASE_URL
BACKEND_HOST=$(echo "${API_BASE_URL}" | sed -E 's|^https?://([^/]+).*|\1|')
BACKEND_WS_HOST="${BACKEND_HOST}"

# Replace CSP placeholders in index.html
sed -i "s|{{BACKEND_URL}}|${API_BASE_URL}|g" /usr/share/nginx/html/index.html
sed -i "s|{{BACKEND_WS_URL}}|ws://${BACKEND_WS_HOST}|g" /usr/share/nginx/html/index.html

echo "Frontend configuration complete"

# Start nginx in foreground
exec nginx -g 'daemon off;'