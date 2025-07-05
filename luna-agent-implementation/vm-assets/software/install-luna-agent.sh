#!/bin/bash

# Luna Agent Installation Script
# Installs all Luna Agent components and automation tools in the VM
# Implements Strategic Recommendation #2: VM Asset Development

set -e

echo "🤖 Installing Luna Agent Software..."
echo "===================================="

# Update system
echo "📥 Updating system packages..."
sudo apt update && sudo apt upgrade -y

# Install system dependencies
echo "🔧 Installing system dependencies..."
sudo apt install -y \
    python3 \
    python3-pip \
    python3-venv \
    nodejs \
    npm \
    docker.io \
    curl \
    wget \
    git \
    xvfb \
    chromium-browser \
    firefox \
    openssh-server \
    htop \
    vim \
    unzip \
    jq \
    build-essential

# Configure Docker
echo "🐳 Configuring Docker..."
sudo usermod -aG docker $USER
sudo systemctl enable docker
sudo systemctl start docker

# Install Node.js LTS
echo "📦 Installing Node.js LTS..."
curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
sudo apt-get install -y nodejs

# Create Luna Agent directory
echo "📁 Setting up Luna Agent directory..."
sudo mkdir -p /opt/luna-agent
sudo chown $USER:$USER /opt/luna-agent

# Create Python virtual environment
echo "🐍 Creating Python environment..."
cd /opt/luna-agent
python3 -m venv luna-env
source luna-env/bin/activate

# Install Python packages for automation
echo "📦 Installing Python automation packages..."
pip install --upgrade pip
pip install \
    selenium \
    playwright \
    opencv-python \
    pillow \
    requests \
    fastapi \
    uvicorn \
    websockets \
    pyautogui \
    psutil \
    schedule \
    python-dotenv \
    aiofiles \
    httpx

# Install Playwright browsers
echo "🎭 Installing Playwright browsers..."
playwright install chromium firefox webkit
playwright install-deps

# Create Luna Agent main application
cat > /opt/luna-agent/main.py << 'PYTHON_EOF'
#!/usr/bin/env python3
"""
Luna Agent - AI-Powered Automation System
Main application entry point
"""

import asyncio
import logging
from fastapi import FastAPI, WebSocket
from fastapi.staticfiles import StaticFiles
from fastapi.responses import HTMLResponse
import uvicorn
import json
import os
from datetime import datetime

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("luna-agent")

# FastAPI app
app = FastAPI(title="Luna Agent", version="1.0.0")

# WebSocket connections
connections = []

@app.get("/")
async def root():
    return {"message": "Luna Agent is running", "version": "1.0.0", "status": "active"}

@app.get("/status")
async def status():
    return {
        "agent": "Luna",
        "status": "online",
        "uptime": "calculating...",
        "capabilities": [
            "web_automation",
            "computer_vision", 
            "task_scheduling",
            "api_integration"
        ]
    }

@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    await websocket.accept()
    connections.append(websocket)
    try:
        while True:
            data = await websocket.receive_text()
            logger.info(f"Received: {data}")
            await websocket.send_text(f"Luna: {data}")
    except:
        connections.remove(websocket)

if __name__ == "__main__":
    logger.info("🌙 Starting Luna Agent...")
    uvicorn.run(app, host="0.0.0.0", port=8080)
PYTHON_EOF

# Create Luna Agent configuration
cat > /opt/luna-agent/config.json << 'CONFIG_EOF'
{
  "agent": {
    "name": "Luna",
    "version": "1.0.0",
    "api_port": 8080,
    "ui_port": 3000
  },
  "automation": {
    "selenium_grid": false,
    "headless_mode": true,
    "screenshot_path": "/opt/luna-agent/screenshots",
    "download_path": "/opt/luna-agent/downloads"
  },
  "security": {
    "api_key_required": false,
    "rate_limiting": true,
    "allowed_hosts": ["localhost", "127.0.0.1"]
  },
  "logging": {
    "level": "INFO",
    "file": "/opt/luna-agent/luna.log",
    "max_size_mb": 100
  }
}
CONFIG_EOF

# Create directories
mkdir -p /opt/luna-agent/{screenshots,downloads,logs,temp}

# Create Luna Agent systemd service
echo "⚙️ Creating Luna Agent service..."
sudo tee /etc/systemd/system/luna-agent.service > /dev/null << 'SERVICE_EOF'
[Unit]
Description=Luna AI Agent Service
After=network.target docker.service
Requires=docker.service

[Service]
Type=simple
User=luna
Group=luna
WorkingDirectory=/opt/luna-agent
Environment=PATH=/opt/luna-agent/luna-env/bin:/usr/local/bin:/usr/bin:/bin
ExecStartPre=/bin/bash -c 'source /opt/luna-agent/luna-env/bin/activate'
ExecStart=/opt/luna-agent/luna-env/bin/python /opt/luna-agent/main.py
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
SERVICE_EOF

# Create Luna user
echo "👤 Creating Luna user..."
sudo useradd -m -s /bin/bash luna || true
sudo usermod -aG docker,sudo luna
sudo chown -R luna:luna /opt/luna-agent

# Enable Luna Agent service
sudo systemctl daemon-reload
sudo systemctl enable luna-agent

# Create startup script
cat > /opt/luna-agent/start-luna.sh << 'START_EOF'
#!/bin/bash
cd /opt/luna-agent
source luna-env/bin/activate
echo "🌙 Starting Luna Agent..."
python main.py
START_EOF
chmod +x /opt/luna-agent/start-luna.sh

# Create simple web UI
mkdir -p /opt/luna-agent/ui
cat > /opt/luna-agent/ui/index.html << 'UI_EOF'
<!DOCTYPE html>
<html>
<head>
    <title>Luna Agent</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; background: #f5f5f5; }
        .container { max-width: 800px; margin: 0 auto; background: white; padding: 30px; border-radius: 10px; }
        .status { padding: 20px; background: #e8f5e8; border-radius: 5px; margin: 20px 0; }
        .luna-title { color: #667eea; font-size: 2em; margin-bottom: 10px; }
    </style>
</head>
<body>
    <div class="container">
        <h1 class="luna-title">🌙 Luna Agent</h1>
        <div class="status">
            <strong>Status:</strong> Online and Ready<br>
            <strong>Version:</strong> 1.0.0<br>
            <strong>Capabilities:</strong> Web Automation, Computer Vision, Task Scheduling
        </div>
        <p>Luna Agent is running and ready to assist with your automation tasks.</p>
        <p><strong>API Endpoint:</strong> http://localhost:8080</p>
        <p><strong>Documentation:</strong> <a href="/docs">/docs</a></p>
    </div>
</body>
</html>
UI_EOF

echo "✅ Luna Agent software installation complete!"
echo ""
echo "🎯 Installation Summary:"
echo "- Luna Agent installed in: /opt/luna-agent"
echo "- Service: luna-agent (systemctl start luna-agent)"
echo "- API Port: 8080"
echo "- UI: /opt/luna-agent/ui/index.html"
echo "- Logs: journalctl -u luna-agent -f"
echo ""
echo "🚀 To start Luna Agent:"
echo "sudo systemctl start luna-agent"
echo ""
