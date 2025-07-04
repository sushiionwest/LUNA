#!/bin/bash
# Luna Development Environment Setup Script
# Run this INSIDE the VM after Ubuntu installation

set -e

echo "🌙 Setting up Luna Development Environment..."
echo "This will install all dependencies and configure Luna for development."
echo ""

# Update system
echo "📦 Updating system packages..."
sudo apt update && sudo apt upgrade -y

# Install essential packages
echo "🔧 Installing development tools..."
sudo apt install -y \
    curl wget git vim nano htop \
    build-essential python3 python3-pip \
    software-properties-common apt-transport-https \
    ca-certificates gnupg lsb-release

# Install Node.js 20
echo "📗 Installing Node.js..."
curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key | sudo gpg --dearmor -o /usr/share/keyrings/nodesource.gpg
echo "deb [signed-by=/usr/share/keyrings/nodesource.gpg] https://deb.nodesource.com/node_20.x nodistro main" | sudo tee /etc/apt/sources.list.d/nodesource.list
sudo apt update && sudo apt install -y nodejs

# Install Bun (faster package manager)
echo "🏃 Installing Bun..."
curl -fsSL https://bun.sh/install | bash
export PATH="$HOME/.bun/bin:$PATH"
echo 'export PATH="$HOME/.bun/bin:$PATH"' >> ~/.bashrc

# Install Docker
echo "🐳 Installing Docker..."
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
sudo apt update && sudo apt install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin

# Add user to docker group
sudo usermod -aG docker $USER

# Install automation tools
echo "🤖 Installing automation tools..."
sudo apt install -y \
    xvfb x11vnc websockify \
    chromium-browser firefox-esr \
    wmctrl xdotool x11-utils \
    sqlite3 \
    nginx

# Install Python packages
echo "🐍 Installing Python packages..."
pip3 install --user \
    fastapi uvicorn \
    selenium webdriver-manager \
    pillow opencv-python-headless \
    tweepy requests \
    psutil

# Create Luna project structure
echo "📁 Creating Luna project structure..."
mkdir -p ~/luna-agent/{src,config,data,logs,scripts,vm-dist}

# Set up development environment
cat > ~/luna-agent/.env.development << 'EOF'
NODE_ENV=development
LUNA_MODE=development
WEB_PORT=8080
DEV_PORT=3000
LOG_LEVEL=debug

# Database
DATABASE_PATH=/home/ubuntu/luna-agent/data/luna.db

# API Keys (set these)
OPENAI_API_KEY=your_openai_key_here
ANTHROPIC_API_KEY=your_anthropic_key_here
TWITTER_API_KEY=your_twitter_key_here
TWITTER_API_SECRET=your_twitter_secret_here

# Security
JWT_SECRET=dev_secret_change_in_production
ENCRYPTION_KEY=dev_encryption_key_change_in_production
