#!/bin/bash
echo "🔧 Optimizing VM for development..."

# Clean package cache
sudo apt autoremove -y
sudo apt autoclean

# Clear logs
sudo journalctl --vacuum-time=7d

# Clear caches
rm -rf ~/.cache/*
sudo rm -rf /tmp/*

echo "✅ VM optimized!"
