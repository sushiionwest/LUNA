#!/bin/bash
# Package Luna VM for Distribution
# Run this INSIDE the VM when ready to create distributable version

set -e

echo "📦 Packaging Luna VM for distribution..."

# Stop Luna service if running
sudo systemctl stop luna-dev || true

# Clean up development artifacts
echo "🧹 Cleaning up development files..."
sudo apt autoremove -y
sudo apt autoclean
sudo apt clean

# Remove development tools (keep runtime only)
sudo apt remove -y \
    build-essential \
    git \
    vim

# Clear temporary files
sudo rm -rf /tmp/*
sudo rm -rf /var/tmp/*
sudo rm -rf ~/.cache/*
sudo rm -rf ~/.bash_history

# Clear logs
sudo journalctl --vacuum-time=1d
sudo truncate -s 0 /var/log/*.log 2>/dev/null || true

# Configure for production
echo "⚙️ Configuring for production..."
cp ~/luna-agent/.env.development ~/luna-agent/.env.production

# Set production environment
cat > ~/luna-agent/.env.production << 'PROD_EOF'
NODE_ENV=production
LUNA_MODE=production
WEB_PORT=8080
LOG_LEVEL=info

# Database
DATABASE_PATH=/home/ubuntu/luna-agent/data/luna.db

# Set your production API keys here
OPENAI_API_KEY=set_your_openai_key_here
ANTHROPIC_API_KEY=set_your_anthropic_key_here
TWITTER_API_KEY=set_your_twitter_key_here
TWITTER_API_SECRET=set_your_twitter_secret_here

# Security (generate new keys for production!)
JWT_SECRET=generate_secure_jwt_secret_here
ENCRYPTION_KEY=generate_secure_encryption_key_here
PROD_EOF

# Build Luna for production
echo "🔨 Building Luna for production..."
cd ~/luna-agent
~/.bun/bin/bun run build

# Enable auto-start for production
sudo systemctl enable luna-dev

# Remove SSH host keys (will be regenerated on first boot)
sudo rm -f /etc/ssh/ssh_host_*

# Remove machine-specific identifiers
sudo rm -f /var/lib/dbus/machine-id
sudo rm -f /etc/machine-id

# Zero out free space to reduce VM size
echo "💾 Optimizing disk space..."
sudo dd if=/dev/zero of=/zerofillfile bs=1M count=1024 2>/dev/null || true
sudo rm -f /zerofillfile

# Clear network interfaces
sudo rm -f /etc/netplan/*.yaml

# Create production startup script
cat > ~/luna-agent/start-production.sh << 'START_EOF'
#!/bin/bash
echo "🌙 Starting Luna Agent..."
cd /home/ubuntu/luna-agent
export NODE_ENV=production
/home/ubuntu/.bun/bin/bun run start
START_EOF

chmod +x ~/luna-agent/start-production.sh

echo "✅ VM prepared for distribution!"
echo "⚠️  IMPORTANT: Before packaging:"
echo "1. Set production API keys in .env.production"
echo "2. Update JWT_SECRET and ENCRYPTION_KEY"
echo "3. Test Luna startup"
echo "4. Shutdown VM: sudo shutdown -h now"
echo "5. Package with VirtualBox: VBoxManage export luna-development --output luna-agent-v1.0.ova"
