#!/bin/bash
# Create Luna Development VM with VirtualBox

set -e

VM_NAME="luna-development"
VM_MEMORY="4096"
VM_STORAGE="20480"  # 20GB
UBUNTU_ISO="ubuntu-22.04.3-live-server-amd64.iso"

echo "🌙 Creating Luna Development VM..."

# Check if VirtualBox is installed
if ! command -v VBoxManage &> /dev/null; then
    echo "❌ VirtualBox not found. Please install VirtualBox first."
    echo "   Download from: https://www.virtualbox.org/wiki/Downloads"
    exit 1
fi

# Check if VM already exists
if VBoxManage list vms | grep -q "$VM_NAME"; then
    echo "⚠️  VM '$VM_NAME' already exists."
    read -p "Delete and recreate? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        VBoxManage unregistervm "$VM_NAME" --delete
    else
        echo "Aborted."
        exit 0
    fi
fi

# Download Ubuntu if not present
if [ ! -f "$UBUNTU_ISO" ]; then
    echo "📥 Downloading Ubuntu 22.04 LTS..."
    wget -O "$UBUNTU_ISO" "https://releases.ubuntu.com/22.04/ubuntu-22.04.3-live-server-amd64.iso"
fi

echo "🔧 Creating virtual machine..."

# Create VM
VBoxManage createvm --name "$VM_NAME" --register --ostype "Ubuntu_64"

# Configure VM
VBoxManage modifyvm "$VM_NAME" \
    --memory "$VM_MEMORY" \
    --cpus 2 \
    --vram 128 \
    --graphicscontroller vmsvga \
    --boot1 dvd \
    --boot2 disk \
    --boot3 none \
    --boot4 none \
    --audio none \
    --usb off \
    --usbehci off \
    --rtcuseutc on \
    --biosbootmenu disabled \
    --bioslogofadein off \
    --bioslogofadeout off

# Create storage
echo "💾 Creating storage..."
VBoxManage createhd --filename "$VM_NAME/$VM_NAME.vdi" --size "$VM_STORAGE" --format VDI

# Add storage controller
VBoxManage storagectl "$VM_NAME" --name "SATA Controller" --add sata --controller IntelAhci

# Attach hard disk
VBoxManage storageattach "$VM_NAME" \
    --storagectl "SATA Controller" \
    --port 0 \
    --device 0 \
    --type hdd \
    --medium "$VM_NAME/$VM_NAME.vdi"

# Attach ISO
VBoxManage storageattach "$VM_NAME" \
    --storagectl "SATA Controller" \
    --port 1 \
    --device 0 \
    --type dvddrive \
    --medium "$UBUNTU_ISO"

# Configure networking
echo "🌐 Configuring networking..."
VBoxManage modifyvm "$VM_NAME" --nic1 nat

# Port forwarding for Luna development
VBoxManage modifyvm "$VM_NAME" --natpf1 "ssh,tcp,,2222,,22"
VBoxManage modifyvm "$VM_NAME" --natpf1 "luna-web,tcp,,8080,,8080"
VBoxManage modifyvm "$VM_NAME" --natpf1 "luna-dev,tcp,,3000,,3000"
VBoxManage modifyvm "$VM_NAME" --natpf1 "vnc,tcp,,5900,,5900"

# Create shared folder for development
mkdir -p ./luna-shared
VBoxManage sharedfolder add "$VM_NAME" \
    --name "luna-shared" \
    --hostpath "$(pwd)/luna-shared" \
    --automount

echo "✅ VM created successfully!"
echo ""
echo "Next steps:"
echo "1. Start VM: VBoxManage startvm $VM_NAME"
echo "2. Install Ubuntu (use regular settings)"
echo "3. After installation, run the VM setup script inside the VM"
echo ""
echo "Connection info:"
echo "- SSH: ssh -p 2222 ubuntu@localhost"
echo "- Luna Web: http://localhost:8080"
echo "- Development: http://localhost:3000"
echo ""
echo "Files:"
echo "- Shared folder: ./luna-shared (mounted as /media/sf_luna-shared in VM)"
echo "- VM location: ./$VM_NAME/"
