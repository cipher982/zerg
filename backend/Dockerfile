# Multi-stage build for Zerg AI Agent Platform Backend
FROM python:3.12-slim as builder

# Install system dependencies needed for Python packages
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install uv for fast Python package management
COPY --from=ghcr.io/astral-sh/uv:latest /uv /usr/local/bin/uv

# Set work directory
WORKDIR /app

# Copy dependency files
COPY pyproject.toml uv.lock ./

# Install dependencies with uv (much faster than pip)
RUN uv sync --frozen --no-dev

# Production stage
FROM python:3.12-slim as production

# Install only runtime dependencies
RUN apt-get update && apt-get install -y \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd --create-home --shell /bin/bash zerg

# Set work directory
WORKDIR /app

# Copy virtual environment from builder
COPY --from=builder /app/.venv /app/.venv

# Copy application code
COPY . .

# Change ownership to non-root user
RUN chown -R zerg:zerg /app

# Switch to non-root user
USER zerg

# Add venv to path
ENV PATH="/app/.venv/bin:$PATH"

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:8000/ || exit 1

# Default port (can be overridden)
EXPOSE 8000

# Start the application
CMD ["python", "-m", "uvicorn", "zerg.main:app", "--host", "0.0.0.0", "--port", "8000"]