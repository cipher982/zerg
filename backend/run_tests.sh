#!/bin/bash

# Install dependencies if needed
uv sync

# Run all tests with verbose output
uv run pytest tests/ -v