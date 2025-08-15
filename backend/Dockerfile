# Multi-stage build for Zerg AI Agent Platform Backend
FROM ghcr.io/astral-sh/uv:python3.12-bookworm-slim AS builder

# uv environment variables for optimization
ENV UV_COMPILE_BYTECODE=1 UV_LINK_MODE=copy

# Set work directory
WORKDIR /app

# Install dependencies first (better caching)
RUN --mount=type=cache,target=/root/.cache/uv \
    --mount=type=bind,source=backend/uv.lock,target=uv.lock \
    --mount=type=bind,source=backend/pyproject.toml,target=pyproject.toml \
    --mount=type=bind,source=README.md,target=../README.md \
    uv sync --frozen --no-install-project --no-dev

# Copy application code and install project
COPY backend/ /app
COPY README.md ../README.md
RUN --mount=type=cache,target=/root/.cache/uv \
    uv sync --frozen --no-dev

# Production stage
FROM python:3.12-slim-bookworm

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    curl \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd --create-home --shell /bin/bash zerg

# Set work directory and copy application with venv
WORKDIR /app
COPY --from=builder --chown=zerg:zerg /app /app

# Create required directories with proper permissions
RUN mkdir -p /app/static/avatars && chown -R zerg:zerg /app/static

# Switch to non-root user
USER zerg

# Add venv to path
ENV PATH="/app/.venv/bin:$PATH"

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:8000/ || exit 1

# Default port
EXPOSE 8000

# Start the application
CMD ["python", "-m", "uvicorn", "zerg.main:app", "--host", "0.0.0.0", "--port", "8000"]