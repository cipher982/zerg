#!/bin/bash


# Run all tests with verbose output
uv run pytest tests/ -p no:warnings --tb=short
