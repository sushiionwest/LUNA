#!/bin/bash

# Luna VM Builder
# Creates a minimal Ubuntu VM with Luna Agent pre-installed
# Implements Strategic Recommendation #2: VM Asset Development

set -e

echo "🔨 Building Luna VM..."
echo "====================="

VM_NAME="Luna-Agent-VM"
VM_SIZE="20480"  # 20GB in MB
VM_MEMORY="2048"
ISO_URL="http://archive.ubuntu.com/ubuntu/dists/jammy/main/installer-amd64/current/legacy-images/netboot/mini.iso"
ISO_FILE="ubuntu-minimal.iso"

# Check if VirtualBox is available
if ! command -v VBoxManage &> /dev/null; then
    echo "❌ VirtualBox not found. Installing VirtualBox..."
    sudo apt update
    sudo apt install -y virtualbox virtualbox-ext-pack
fi

echo "✅ VirtualBox detected"

# Create VM directory
mkdir -p ./vm-build
cd ./vm-build

# Download Ubuntu minimal ISO if not exists
if [ ! -f "$ISO_FILE" ]; then
    echo "📥 Downloading Ubuntu minimal ISO..."
    curl -L -o "$ISO_FILE" "$ISO_URL"
    echo "✅ ISO downloaded"
fi

# Remove existing VM if it exists
if VBoxManage list vms | grep -q "$VM_NAME"; then
    echo "🗑️ Removing existing VM..."
    VBoxManage controlvm "$VM_NAME" poweroff 2>/dev/null || true
    sleep 2
    VBoxManage unregistervm "$VM_NAME" --delete 2>/dev/null || true
fi

echo "🔧 Creating virtual machine..."

# Create VM
VBoxManage createvm --name "$VM_NAME" --ostype "Ubuntu_64" --register

# Configure VM hardware
VBoxManage modifyvm "$VM_NAME" \
    --memory "$VM_MEMORY" \
    --cpus 2 \
    --vram 128 \
    --graphicscontroller vmsvga \
    --audio none \
    --usb off \
    --usbehci off \
    --usbxhci off \
    --natpf1 "luna-api,tcp,,8080,,8080" \
    --natpf1 "luna-ui,tcp,,3000,,3000" \
    --natpf1 "ssh,tcp,,22222,,22"

# Create and attach storage
echo "💾 Creating storage..."
VBoxManage createhd --filename "$VM_NAME.vdi" --size "$VM_SIZE" --format VDI

# Create storage controller
VBoxManage storagectl "$VM_NAME" --name "SATA Controller" --add sata --controller IntelAhci

# Attach hard drive
VBoxManage storageattach "$VM_NAME" --storagectl "SATA Controller" --port 0 --device 0 --type hdd --medium "$VM_NAME.vdi"

# Attach ISO for installation
VBoxManage storageattach "$VM_NAME" --storagectl "SATA Controller" --port 1 --device 0 --type dvddrive --medium "$ISO_FILE"

echo "✅ VM hardware configured"
echo ""
echo "🚀 VM created successfully: $VM_NAME"
echo "📁 VM files location: $(pwd)"
echo ""
echo "Next steps:"
echo "1. VBoxManage startvm $VM_NAME"
echo "2. Install Ubuntu (use preseed for automation)"
echo "3. Run Luna Agent installation script"
echo "4. Package VM for distribution"
echo ""
