#!/bin/bash

# Luna VM Startup Script - Real Implementation
# Starts all Luna services in the correct order

set -e

echo "🌙 Starting Luna Agent VM..."
echo "============================="

# Function to log with timestamp
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1"
}

# Start display server
log "🖥️ Starting virtual display..."
Xvfb :99 -screen 0 1920x1080x24 -ac +extension GLX +render -noreset &
export DISPLAY=:99

# Wait for display to be ready
sleep 2

# Start window manager
log "🪟 Starting window manager..."
fluxbox -display :99 &

# Start VNC server
log "🔗 Starting VNC server..."
x11vnc -display :99 -forever -usepw -create &

# Start SSH server (as root, then switch back)
log "🔑 Starting SSH server..."
sudo service ssh start

# Change to Luna user home
cd /opt/luna-agent

# Create necessary directories
mkdir -p screenshots downloads logs temp ui

# Set up environment
export LUNA_HOME=/opt/luna-agent
export PYTHONPATH=$LUNA_HOME:$PYTHONPATH

# Start Luna Agent
log "🤖 Starting Luna Agent API server..."
python3 main.py &
LUNA_PID=$!

# Function to handle shutdown
shutdown_handler() {
    log "🛑 Shutting down Luna VM..."
    kill $LUNA_PID 2>/dev/null || true
    pkill -f "Xvfb" 2>/dev/null || true
    pkill -f "x11vnc" 2>/dev/null || true
    pkill -f "fluxbox" 2>/dev/null || true
    log "✅ Luna VM shutdown complete"
    exit 0
}

# Set up signal handlers
trap shutdown_handler SIGTERM SIGINT

# Wait for Luna Agent to start
sleep 5

# Test Luna Agent
log "🧪 Testing Luna Agent..."
if curl -f http://localhost:8080/status >/dev/null 2>&1; then
    log "✅ Luna Agent is running successfully!"
else
    log "❌ Luna Agent failed to start properly"
fi

# Display status
log "📊 Luna VM Status:"
log "   - API Server: http://localhost:8080"
log "   - Web UI: http://localhost:8080/ui"
log "   - VNC: localhost:5900 (password: luna123)"
log "   - SSH: ssh luna@localhost -p 22222 (password: luna123)"

log "🌙 Luna Agent VM is ready for automation!"

# Keep the container running
wait $LUNA_PID
