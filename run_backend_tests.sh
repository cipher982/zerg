#!/bin/bash


# Run all tests with verbose output
cd backend; uv run pytest tests/ -p no:warnings --tb=short