#!/bin/bash
# Complete Luna VM Development Setup
# This script automates the entire process

set -e

VM_NAME="luna-development"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🌙 Luna VM Development Setup"
echo "============================"
echo ""

# Check if VirtualBox is installed
if ! command -v VBoxManage &> /dev/null; then
    echo "❌ VirtualBox not found."
    echo "Please install VirtualBox from: https://www.virtualbox.org/wiki/Downloads"
    exit 1
fi

echo "✅ VirtualBox found"

# Check if VM already exists and is running
if VBoxManage list runningvms | grep -q "$VM_NAME"; then
    echo "✅ Luna VM is already running!"
    echo "🌐 Access Luna at: http://localhost:8080"
    echo "🔗 SSH access: ssh -p 2222 ubuntu@localhost"
    
    read -p "Open Luna in browser? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        # Try to open in browser
        if command -v xdg-open >/dev/null 2>&1; then
            xdg-open "http://localhost:8080"
        elif command -v open >/dev/null 2>&1; then
            open "http://localhost:8080"
        elif command -v start >/dev/null 2>&1; then
            start "http://localhost:8080"
        else
            echo "🌐 Open http://localhost:8080 in your browser"
        fi
    fi
    exit 0
fi

# Check if VM exists but is not running
if VBoxManage list vms | grep -q "$VM_NAME"; then
    echo "🔄 Luna VM exists but is stopped. Starting..."
    VBoxManage startvm "$VM_NAME" --type headless
    
    echo "⏳ Waiting for Luna to start..."
    sleep 30
    
    # Check if Luna is ready
    max_attempts=30
    for i in $(seq 1 $max_attempts); do
        if curl -s http://localhost:8080/health >/dev/null 2>&1; then
            echo "✅ Luna is ready!"
            echo "🌐 Access Luna at: http://localhost:8080"
            break
        fi
        echo "   Attempt $i/$max_attempts - waiting for Luna..."
        sleep 10
    done
    
    exit 0
fi

# VM doesn't exist, need to create it
echo "🆕 Luna VM not found. Let's create it!"
echo ""
echo "This will:"
echo "1. Create a new Ubuntu VM"
echo "2. Download Ubuntu 22.04 ISO (if needed)"
echo "3. Set up networking and shared folders"
echo "4. Guide you through Ubuntu installation"
echo "5. Set up Luna development environment"
echo ""

read -p "Continue with VM creation? (Y/n): " -n 1 -r
echo
if [[ $REPLY =~ ^[Nn]$ ]]; then
    echo "Aborted."
    exit 0
fi

# Create the VM
echo "🔧 Creating Luna development VM..."
cd "$SCRIPT_DIR/scripts"
./create-vm.sh

echo ""
echo "✅ VM created successfully!"
echo ""
echo "📋 Next steps:"
echo "1. Start the VM: VBoxManage startvm $VM_NAME"
echo "2. Install Ubuntu 22.04 (use default settings)"
echo "3. Create user 'ubuntu' with your preferred password"
echo "4. After Ubuntu installation, run this script again"
echo ""

read -p "Start VM installation now? (Y/n): " -n 1 -r
echo
if [[ $REPLY =~ ^[Nn]$ ]]; then
    echo "VM created but not started."
    echo "Start manually with: VBoxManage startvm $VM_NAME"
    exit 0
fi

# Start VM for installation
echo "🚀 Starting VM for Ubuntu installation..."
VBoxManage startvm "$VM_NAME"

echo ""
echo "📝 Ubuntu Installation Instructions:"
echo "1. Install Ubuntu Server 22.04 with default settings"
echo "2. Create user: ubuntu"
echo "3. Enable OpenSSH server when prompted"
echo "4. Complete installation and reboot"
echo "5. Run this script again to continue setup"
echo ""
echo "💡 Tip: The VM window should open automatically"
echo "If not, open VirtualBox and start '$VM_NAME'"

