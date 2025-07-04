#!/bin/bash

# Luna Testing Infrastructure Startup Script

echo "🌙 Starting Luna Testing Infrastructure..."
echo ""

# Check if Bun is available
if ! command -v /home/scrapybara/.bun/bin/bun &> /dev/null; then
    echo "❌ Bun is not installed. Please install Bun first."
    exit 1
fi

# Check if dependencies are installed
if [ ! -d "node_modules" ]; then
    echo "📦 Installing dependencies..."
    /home/scrapybara/.bun/bin/bun install
fi

# Create uploads directory if it doesn't exist
if [ ! -d "uploads" ]; then
    echo "📁 Creating uploads directory..."
    mkdir -p uploads/{sessions,participants,screenshots,recordings,logs,exports}
fi

# Set default environment variables if not set
export NODE_ENV=${NODE_ENV:-development}
export PORT=${PORT:-3001}
export DATABASE_PATH=${DATABASE_PATH:-./luna_testing.db}

echo "🚀 Starting server..."
echo "   Environment: $NODE_ENV"
echo "   Port: $PORT"
echo "   Database: $DATABASE_PATH"
echo ""

# Start the server
/home/scrapybara/.bun/bin/bun run index.ts
