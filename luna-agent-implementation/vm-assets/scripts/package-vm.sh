#!/bin/bash

# Luna VM Packaging Script
# Packages the Luna VM for distribution with the installer
# Implements Strategic Recommendation #2: VM Asset Development

set -e

echo "📦 Packaging Luna VM for distribution..."
echo "======================================="

VM_NAME="Luna-Agent-VM"
OUTPUT_DIR="../installer/shared/vm-assets"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Stop VM if running
echo "⏹️ Stopping VM if running..."
VBoxManage controlvm "$VM_NAME" poweroff 2>/dev/null || true
sleep 5

# Clean up VM before packaging
echo "🧹 Preparing VM for packaging..."
VBoxManage modifyvm "$VM_NAME" --dvd none

# Export VM to OVA format
echo "📤 Exporting VM to OVA format..."
VBoxManage export "$VM_NAME" \
    --output "$OUTPUT_DIR/Luna-Agent-VM.ova" \
    --options manifest \
    --vsys 0 \
    --product "Luna Agent VM" \
    --producturl "https://luna-agent.com" \
    --vendor "Luna Team" \
    --version "1.0.0"

# Calculate file size and checksum
VM_SIZE_BYTES=$(stat -c%s "$OUTPUT_DIR/Luna-Agent-VM.ova")
VM_SIZE_MB=$((VM_SIZE_BYTES / 1024 / 1024))
VM_CHECKSUM=$(sha256sum "$OUTPUT_DIR/Luna-Agent-VM.ova" | cut -d' ' -f1)

# Create VM metadata
cat > "$OUTPUT_DIR/vm-metadata.json" << METADATA_EOF
{
  "name": "Luna Agent VM",
  "version": "1.0.0",
  "build_date": "$(date -Iseconds)",
  "format": "OVA",
  "size_bytes": $VM_SIZE_BYTES,
  "size_mb": $VM_SIZE_MB,
  "checksum_sha256": "$VM_CHECKSUM",
  "filename": "Luna-Agent-VM.ova",
  "requirements": {
    "virtualbox_min": "6.0",
    "memory_mb": 2048,
    "storage_gb": 25,
    "cpu_cores": 2
  },
  "features": [
    "Ubuntu 22.04 LTS",
    "Luna Agent v1.0.0",
    "Python automation stack",
    "Selenium & Playwright",
    "Docker runtime",
    "Headless browsers",
    "API server on port 8080"
  ],
  "network": {
    "ports": {
      "8080": "Luna Agent API",
      "3000": "Luna Agent UI",
      "22222": "SSH Access"
    },
    "type": "NAT with port forwarding"
  }
}
METADATA_EOF

# Create installation instructions
cat > "$OUTPUT_DIR/INSTALLATION.md" << 'INSTALL_EOF'
# Luna Agent VM Installation

## Prerequisites
- VirtualBox 6.0 or later
- 4GB RAM available (2GB for VM)
- 25GB free disk space

## Installation Steps

1. **Import the VM:**
   ```bash
   VBoxManage import Luna-Agent-VM.ova
   ```

2. **Start the VM:**
   ```bash
   VBoxManage startvm "Luna-Agent-VM" --type headless
   ```

3. **Access Luna Agent:**
   - API: http://localhost:8080
   - UI: http://localhost:3000
   - SSH: ssh luna@localhost -p 22222

## Default Credentials
- Username: luna
- Password: luna123 (change after first login)

## Service Management
```bash
# Check status
sudo systemctl status luna-agent

# Start/Stop/Restart
sudo systemctl start luna-agent
sudo systemctl stop luna-agent
sudo systemctl restart luna-agent

# View logs
journalctl -u luna-agent -f
```

## Troubleshooting
- Ensure VirtualBox is installed and updated
- Check available system resources
- Verify network ports are not in use
- Check VM logs in VirtualBox GUI
INSTALL_EOF

# Create verification script
cat > "$OUTPUT_DIR/verify-vm.sh" << 'VERIFY_EOF'
#!/bin/bash

# Luna VM Verification Script
echo "🔍 Verifying Luna VM package..."

OVA_FILE="Luna-Agent-VM.ova"
METADATA_FILE="vm-metadata.json"

# Check files exist
if [ ! -f "$OVA_FILE" ]; then
    echo "❌ OVA file not found: $OVA_FILE"
    exit 1
fi

if [ ! -f "$METADATA_FILE" ]; then
    echo "❌ Metadata file not found: $METADATA_FILE"
    exit 1
fi

# Verify checksum
echo "🔐 Verifying checksum..."
EXPECTED_CHECKSUM=$(jq -r '.checksum_sha256' "$METADATA_FILE")
ACTUAL_CHECKSUM=$(sha256sum "$OVA_FILE" | cut -d' ' -f1)

if [ "$EXPECTED_CHECKSUM" != "$ACTUAL_CHECKSUM" ]; then
    echo "❌ Checksum mismatch!"
    echo "Expected: $EXPECTED_CHECKSUM"
    echo "Actual: $ACTUAL_CHECKSUM"
    exit 1
fi

echo "✅ Luna VM package verified successfully"
echo "📁 Package size: $(jq -r '.size_mb' "$METADATA_FILE") MB"
echo "🏷️ Version: $(jq -r '.version' "$METADATA_FILE")"
echo "📅 Build date: $(jq -r '.build_date' "$METADATA_FILE")"
