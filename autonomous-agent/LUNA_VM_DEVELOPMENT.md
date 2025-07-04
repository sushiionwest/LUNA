# Luna Development VM Setup Guide

## Development Strategy

**Smart Approach**: Build Luna inside a Linux VM, then package that entire VM for distribution.

```
Development Flow:
Linux VM (Dev) → Configure & Build Luna → Package VM → Distribute to Users

User Experience:
Download Luna App → VM starts automatically → Luna ready to use
```

## VM Setup for Luna Development

### Recommended VM Configuration

```yaml
# luna-dev-vm-config.yaml
vm_specs:
  name: "luna-development"
  os: "Ubuntu 22.04 LTS"
  memory: "4GB"
  storage: "20GB"
  cpus: 2
  
dev_mode:
  shared_folders: true
  port_forwarding:
    - host: 8080, guest: 8080  # Luna web interface
    - host: 3000, guest: 3000  # Development server
    - host: 5900, guest: 5900  # VNC (optional)
  
networking:
  mode: "NAT"
  internet_access: true
```

### Quick VM Setup Scripts

#### VirtualBox Setup
```bash
#!/bin/bash
# setup-luna-dev-vm.sh

VM_NAME="luna-development"
ISO_PATH="ubuntu-22.04.3-live-server-amd64.iso"

# Create VM
VBoxManage createvm --name "$VM_NAME" --register
VBoxManage modifyvm "$VM_NAME" \
  --memory 4096 \
  --cpus 2 \
  --vram 128 \
  --boot1 dvd \
  --boot2 disk \
  --boot3 none \
  --boot4 none \
  --audio none \
  --usb off \
  --rtcuseutc on

# Create and attach storage
VBoxManage createhd --filename "$VM_NAME.vdi" --size 20480
VBoxManage storagectl "$VM_NAME" --name "SATA" --add sata
VBoxManage storageattach "$VM_NAME" \
  --storagectl "SATA" \
  --port 0 \
  --device 0 \
  --type hdd \
  --medium "$VM_NAME.vdi"

# Attach ISO
VBoxManage storageattach "$VM_NAME" \
  --storagectl "SATA" \
  --port 1 \
  --device 0 \
  --type dvddrive \
  --medium "$ISO_PATH"

# Network configuration
VBoxManage modifyvm "$VM_NAME" --nic1 nat
VBoxManage modifyvm "$VM_NAME" --natpf1 "luna-web,tcp,,8080,,8080"
VBoxManage modifyvm "$VM_NAME" --natpf1 "luna-dev,tcp,,3000,,3000"

# Shared folder for development
VBoxManage sharedfolder add "$VM_NAME" \
  --name "luna-dev" \
  --hostpath "$(pwd)/luna-development" \
  --automount

echo "✅ VM created: $VM_NAME"
echo "🚀 Start with: VBoxManage startvm $VM_NAME"
```

#### VMware Setup
```bash
#!/bin/bash
# setup-luna-vmware.sh

VM_NAME="luna-development"
VM_DIR="./luna-vm"

mkdir -p "$VM_DIR"

cat > "$VM_DIR/luna-dev.vmx" << EOF
#!/usr/bin/vmware
config.version = "8"
virtualHW.version = "19"

displayName = "Luna Development"
guestOS = "ubuntu-64"

memsize = "4096"
numvcpus = "2"

scsi0.present = "TRUE"
scsi0.virtualDev = "lsilogic"

scsi0:0.present = "TRUE"
scsi0:0.fileName = "luna-dev.vmdk"
scsi0:0.deviceType = "scsi-hardDisk"

ide1:0.present = "TRUE"
ide1:0.fileName = "ubuntu-22.04.3-live-server-amd64.iso"
ide1:0.deviceType = "cdrom-image"

ethernet0.present = "TRUE"
ethernet0.virtualDev = "e1000"
ethernet0.networkName = "NAT"

# Port forwarding for development
nat.portForwarding.enabled = "TRUE"
nat.portForwarding.tcp.8080 = "8080"
nat.portForwarding.tcp.3000 = "3000"

# Shared folder
sharedFolder0.present = "TRUE"
sharedFolder0.enabled = "TRUE"
sharedFolder0.readAccess = "TRUE"
sharedFolder0.writeAccess = "TRUE"
sharedFolder0.hostPath = "./luna-development"
sharedFolder0.guestName = "luna-dev"
EOF

# Create virtual disk
vmware-vdiskmanager -c -s 20GB -a lsilogic -t 0 "$VM_DIR/luna-dev.vmdk"

echo "✅ VMware VM created in $VM_DIR"
echo "🚀 Open luna-dev.vmx in VMware"
```

## Luna Development Environment Setup

### Automated VM Configuration Script
```bash
#!/bin/bash
# configure-luna-vm.sh
# Run this INSIDE the VM after Ubuntu installation

set -e

echo "🌙 Setting up Luna Development Environment..."

# Update system
sudo apt update && sudo apt upgrade -y

# Install essential development tools
sudo apt install -y \
  curl wget git vim nano \
  build-essential python3 python3-pip \
  nodejs npm \
  docker.io docker-compose \
  xvfb x11vnc websockify \
  chromium-browser firefox \
  wmctrl xdotool \
  sqlite3 \
  nginx \
  supervisor

# Install Bun (faster than npm)
curl -fsSL https://bun.sh/install | bash
source ~/.bashrc

# Install modern Node.js
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt install -y nodejs

# Configure Docker
sudo usermod -aG docker $USER
sudo systemctl enable docker
sudo systemctl start docker

# Install Python packages for Luna
pip3 install --user \
  fastapi uvicorn \
  selenium webdriver-manager \
  pillow opencv-python \
  tweepy requests \
  sqlite3

# Create Luna development structure
mkdir -p ~/luna-agent/{src,config,data,logs,vm-dist}

# Set up development aliases
cat >> ~/.bashrc << 'EOF'
# Luna Development Aliases
alias luna-dev='cd ~/luna-agent && bun run dev'
alias luna-build='cd ~/luna-agent && bun run build'
alias luna-test='cd ~/luna-agent && bun run test'
alias luna-logs='tail -f ~/luna-agent/logs/luna.log'

# Quick VM info
alias vm-info='echo "Luna Dev VM - $(hostname) - $(date)"'
EOF

# Configure VNC for remote access (optional)
mkdir -p ~/.vnc
echo "#!/bin/bash
xrdb $HOME/.Xresources
xsetroot -solid grey
x-terminal-emulator -geometry 80x24+10+10 -ls -title '\$VNCDESKTOP Desktop' &
/etc/X11/Xsession &
" > ~/.vnc/xstartup
chmod +x ~/.vnc/xstartup

# Set up auto-start services
sudo tee /etc/systemd/system/luna-dev.service > /dev/null << 'EOF'
[Unit]
Description=Luna Development Environment
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/home/ubuntu/luna-agent
ExecStart=/home/ubuntu/.bun/bin/bun run dev
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

echo "✅ Luna development environment configured!"
echo "🔧 Reboot the VM to complete setup"
echo "🌐 After reboot, Luna will be available at http://localhost:8080"
```

## Development Workflow

### 1. Clone Luna Project in VM
```bash
# Inside the VM
cd ~/luna-agent
git clone https://github.com/yourusername/luna-agent .

# Install dependencies
bun install

# Set up environment
cp .env.example .env
nano .env  # Configure for development
```

### 2. Development Commands
```bash
# Start Luna in development mode
bun run dev

# Build for production
bun run build

# Test automation features
bun run test:automation

# Package for distribution
bun run vm:package
```

### 3. VM-to-VM Distribution Script
```bash
#!/bin/bash
# package-luna-vm.sh
# Run this to package the development VM for distribution

echo "📦 Packaging Luna VM for distribution..."

# Clean up development artifacts
sudo apt autoremove -y
sudo apt autoclean
rm -rf ~/.cache/* /tmp/* /var/tmp/*

# Remove development tools (keep runtime only)
sudo apt remove -y build-essential git vim
sudo apt autoremove -y

# Configure for production
sudo systemctl enable luna-agent
sudo systemctl enable docker

# Create production config
cat > ~/luna-agent/.env.production << 'EOF'
NODE_ENV=production
LUNA_MODE=production
WEB_PORT=8080
LOG_LEVEL=info
EOF

# Clean SSH keys and machine-specific data
sudo rm -f /etc/ssh/ssh_host_*
sudo rm -f ~/.ssh/authorized_keys
sudo rm -f ~/.bash_history

# Zero out free space (reduces VM size)
sudo dd if=/dev/zero of=/zerofillfile bs=1M || true
sudo rm -f /zerofillfile

# Shutdown for packaging
sudo shutdown -h now
```

## VM Distribution Strategy

### 1. Create Base VM Image
```bash
# After VM shutdown, create distributable image
VBoxManage clonevm luna-development \
  --name "luna-agent-v1.0" \
  --options link \
  --register

# Export for distribution
VBoxManage export luna-agent-v1.0 \
  --output luna-agent-v1.0.ova \
  --manifest \
  --vsys 0 \
  --product "Luna Agent" \
  --producturl "https://luna-agent.com" \
  --vendor "Your Company"
```

### 2. Native App Wrapper
```typescript
// Native app that manages the VM
class LunaVMManager {
  private vmPath: string;
  private vmName: string = 'luna-agent';
  
  async initialize() {
    // Check if VM exists
    if (!await this.vmExists()) {
      await this.importVM();
    }
    
    await this.startVM();
    await this.waitForLuna();
  }
  
  private async importVM() {
    const ovaPath = path.join(__dirname, 'vm', 'luna-agent-v1.0.ova');
    
    // Import VM from OVA
    await this.runVBoxCommand([
      'import', ovaPath,
      '--vsys', '0',
      '--vmname', this.vmName
    ]);
  }
  
  private async startVM() {
    await this.runVBoxCommand(['startvm', this.vmName, '--type', 'headless']);
  }
  
  private async waitForLuna() {
    // Poll until Luna is ready
    const maxAttempts = 60;
    for (let i = 0; i < maxAttempts; i++) {
      try {
        const response = await fetch('http://localhost:8080/health');
        if (response.ok) return;
      } catch (e) {
        // Luna not ready yet
      }
      await new Promise(resolve => setTimeout(resolve, 2000));
    }
    throw new Error('Luna failed to start');
  }
}
```

## Benefits of This Approach

### For Development
- **Consistent Environment**: Same Linux setup every time
- **Full Control**: Configure exactly what Luna needs
- **Easy Testing**: Test the exact environment users get
- **Isolation**: Development doesn't affect host machine

### For Distribution
- **Proven Environment**: Users get exact same setup as development
- **Pre-configured**: Everything installed and configured
- **Cross-platform**: Same VM works on Windows/Mac/Linux hosts
- **Updatable**: Can push new VM versions

### For Users
- **Just Works**: No configuration needed
- **Consistent**: Same experience regardless of host OS
- **Secure**: Isolated from host system
- **Professional**: Enterprise-grade deployment

## VM Optimization for Distribution

### Size Optimization
```bash
# Minimize VM size for distribution
sudo apt autoremove -y
sudo apt autoclean
sudo journalctl --vacuum-time=1d

# Remove unnecessary files
sudo rm -rf /var/log/*.log
sudo rm -rf /tmp/*
sudo rm -rf ~/.cache/*

# Zero out free space
sudo dd if=/dev/zero of=/fillfile bs=1M count=1024
sudo rm -f /fillfile

# Compact VM disk
VBoxManage modifymedium disk luna-agent.vdi --compact
```

### Performance Optimization
```bash
# Optimize for VM performance
echo 'vm.swappiness=10' | sudo tee -a /etc/sysctl.conf
echo 'vm.dirty_ratio=5' | sudo tee -a /etc/sysctl.conf

# Disable unnecessary services
sudo systemctl disable snapd
sudo systemctl disable unattended-upgrades
```

## Development Tips

### 1. Shared Development
```bash
# Mount host directory in VM for live development
VBoxManage sharedfolder add luna-development \
  --name "source" \
  --hostpath "/path/to/luna-source" \
  --automount

# In VM: 
sudo mount -t vboxsf source /mnt/luna-dev
```

### 2. Remote Development
```bash
# Enable SSH for remote development
sudo systemctl enable ssh
sudo systemctl start ssh

# From host:
ssh ubuntu@localhost -p 2222  # Configure port forwarding
```

### 3. Debugging
```bash
# Monitor Luna in VM
journalctl -u luna-agent -f

# VM resource monitoring
htop
iostat 1
```

This approach gives you the best of both worlds: a controlled development environment that becomes the exact product users receive!