#!/bin/bash
set -e

# Port conflict detection and resolution script
# Ensures BACKEND_PORT and FRONTEND_PORT are available, prompts for alternatives if not

ENV_FILE=".env"
TEMP_ENV_FILE=".env.tmp"

# Load current .env values
if [[ ! -f "$ENV_FILE" ]]; then
    echo "❌ .env file not found. Please create one based on .env.example"
    exit 1
fi

# Load and extract current port values
source "$ENV_FILE" 2>/dev/null || true

# Get port values with fallback defaults
BACKEND_PORT=${BACKEND_PORT:-8001}
FRONTEND_PORT=${FRONTEND_PORT:-8002}

echo "🔍 Checking port availability..."
echo "  Backend:  $BACKEND_PORT"
echo "  Frontend: $FRONTEND_PORT"

# Function to check if a port is in use
check_port() {
    local port=$1
    if lsof -ti:$port >/dev/null 2>&1; then
        return 0  # Port is in use
    else
        return 1  # Port is free
    fi
}

# Function to find next available port starting from a base port
find_free_port() {
    local start_port=$1
    local port=$start_port
    
    while check_port $port; do
        ((port++))
        if [[ $port -gt 65535 ]]; then
            echo "❌ No available ports found above $start_port"
            exit 1
        fi
    done
    
    echo $port
}

# Function to automatically resolve port conflicts
auto_resolve_port() {
    local service_name=$1
    local current_port=$2
    local suggested_port=$3
    
    echo "⚠️  Port conflict: $service_name port $current_port is in use" >&2
    echo "✅ Auto-resolved: Using port $suggested_port instead" >&2
    echo $suggested_port  # This goes to stdout for capture
}

# Function to update .env file with new port
update_env_port() {
    local port_name=$1
    local new_port=$2
    
    # Create temp file with updated port
    sed "s/^${port_name}=.*/${port_name}=${new_port}/" "$ENV_FILE" > "$TEMP_ENV_FILE"
    mv "$TEMP_ENV_FILE" "$ENV_FILE"
    
    echo "✅ Updated $port_name to $new_port in .env"
}

# Check backend port
BACKEND_PORT_CHANGED=false
if check_port $BACKEND_PORT; then
    suggested_backend=$(find_free_port $BACKEND_PORT)
    new_backend_port=$(auto_resolve_port "Backend" $BACKEND_PORT $suggested_backend)
    update_env_port "BACKEND_PORT" $new_backend_port
    BACKEND_PORT=$new_backend_port
    BACKEND_PORT_CHANGED=true
fi

# Check frontend port (make sure it doesn't conflict with new backend port)
FRONTEND_PORT_CHANGED=false  
if check_port $FRONTEND_PORT || [[ "$FRONTEND_PORT" == "$BACKEND_PORT" ]]; then
    # Start search from original frontend port, but skip backend port if needed
    suggested_frontend=$(find_free_port $FRONTEND_PORT)
    while [[ "$suggested_frontend" == "$BACKEND_PORT" ]]; do
        ((suggested_frontend++))
        suggested_frontend=$(find_free_port $suggested_frontend)
    done
    new_frontend_port=$(auto_resolve_port "Frontend" $FRONTEND_PORT $suggested_frontend)
    update_env_port "FRONTEND_PORT" $new_frontend_port  
    FRONTEND_PORT=$new_frontend_port
    FRONTEND_PORT_CHANGED=true
fi

# Update API_BASE_URL if backend port changed
if [[ "$BACKEND_PORT_CHANGED" == "true" ]]; then
    echo "🔄 Updating API_BASE_URL to match new backend port..."
    sed -i.bak "s|API_BASE_URL=.*|API_BASE_URL=\"http://localhost:${BACKEND_PORT}\"|" "$ENV_FILE" && rm -f "${ENV_FILE}.bak"
    echo "✅ Updated API_BASE_URL to http://localhost:${BACKEND_PORT}"
fi

# Final verification
echo ""
echo "✅ Port check complete!"
echo "  Backend:  $BACKEND_PORT ($(check_port $BACKEND_PORT && echo "⚠️  still in use" || echo "✅ available"))"
echo "  Frontend: $FRONTEND_PORT ($(check_port $FRONTEND_PORT && echo "⚠️  still in use" || echo "✅ available"))"

# Export ports for use by make
export BACKEND_PORT
export FRONTEND_PORT
echo ""
echo "🚀 Ports configured successfully. Starting servers..."