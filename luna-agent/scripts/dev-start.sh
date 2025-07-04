#!/bin/bash
echo "🌙 Starting Luna Development Environment..."
cd ~/luna-agent

# Start Luna in development mode
echo "Starting Luna on http://localhost:8080"
/home/scrapybara/.bun/bin/bun run dev
