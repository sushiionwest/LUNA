# Luna VM Development - Quick Start Guide

## 🚀 Get Started in 3 Steps

### Step 1: Create the VM
```bash
cd luna-vm-dev/scripts
./create-vm.sh
```
This will:
- Create a VirtualBox VM with Ubuntu 22.04
- Configure networking with port forwarding
- Set up shared folders for development

### Step 2: Install Ubuntu
1. Start the VM: `VBoxManage startvm luna-development`
2. Install Ubuntu 22.04 with default settings
3. Create user: `ubuntu` (password: your choice)
4. Enable SSH during installation

### Step 3: Set Up Luna Environment
```bash
# SSH into the VM
ssh -p 2222 ubuntu@localhost

# Run the setup script
curl -fsSL https://raw.githubusercontent.com/yourusername/luna-agent/main/vm-setup/setup-luna-env.sh | bash

# Or if you have the script locally:
./setup-luna-env.sh
```

## 🎯 What You Get

After setup, you'll have:
- **Complete Luna development environment** in the VM
- **Auto-port forwarding**: Luna accessible at http://localhost:8080
- **Shared folder**: `/media/sf_luna-shared` for easy file transfer
- **SSH access**: `ssh -p 2222 ubuntu@localhost`
- **Development tools**: Node.js, Bun, Docker, automation tools

## 🔧 Development Workflow

### Daily Development
```bash
# SSH into VM
ssh -p 2222 ubuntu@localhost

# Start Luna development
luna-dev

# Or manually
cd ~/luna-agent
bun run dev
```

### Access Luna
- **Web Interface**: http://localhost:8080
- **Development**: http://localhost:3000
- **Health Check**: http://localhost:8080/health

### File Sharing
```bash
# Host machine: Put files here
./luna-vm-dev/luna-shared/

# VM: Access files at
/media/sf_luna-shared/
```

## 📦 Building for Distribution

When Luna is ready for distribution:

```bash
# Inside the VM
./package-vm.sh

# On host machine
VBoxManage export luna-development --output luna-agent-v1.0.ova
```

## 🛠️ VM Management Commands

```bash
# Start VM
VBoxManage startvm luna-development

# Stop VM  
VBoxManage controlvm luna-development poweroff

# Pause/Resume VM
VBoxManage controlvm luna-development pause
VBoxManage controlvm luna-development resume

# Take snapshot
VBoxManage snapshot luna-development take "working-luna-v1"

# Restore snapshot
VBoxManage snapshot luna-development restore "working-luna-v1"

# Clone VM
VBoxManage clonevm luna-development --name "luna-backup" --register
```

## 🔍 Troubleshooting

### VM Won't Start
```bash
# Check if VM exists
VBoxManage list vms

# Check VM info
VBoxManage showvminfo luna-development

# Reset if needed
VBoxManage modifyvm luna-development --defaultfrontend default
```

### Can't Connect to Luna
```bash
# Check port forwarding
VBoxManage showvminfo luna-development | grep "NIC 1 Rule"

# Test SSH connection
ssh -p 2222 ubuntu@localhost

# Check Luna status in VM
ssh -p 2222 ubuntu@localhost "luna-status"
```

### Performance Issues
```bash
# Increase VM memory
VBoxManage modifyvm luna-development --memory 6144

# Add more CPUs
VBoxManage modifyvm luna-development --cpus 4

# Enable hardware acceleration
VBoxManage modifyvm luna-development --paravirtprovider kvm
```

## 📁 Directory Structure

```
luna-vm-dev/
├── scripts/
│   ├── create-vm.sh          # Creates the VM
│   ├── setup-luna-env.sh     # Configures Luna environment  
│   └── package-vm.sh         # Packages for distribution
├── config/
│   └── vm-config.yaml        # VM configuration template
└── luna-shared/              # Shared folder with VM
    └── (development files)
```

## 🌙 Luna Development Tips

### Environment Variables
```bash
# Development mode
export NODE_ENV=development

# Enable debug logging
export LOG_LEVEL=debug

# Custom Luna config
export LUNA_CONFIG_PATH=/path/to/config
```

### Database Management
```bash
# Check Luna database
sqlite3 ~/luna-agent/data/luna.db ".tables"

# Reset database
rm ~/luna-agent/data/luna.db
luna-dev  # Will recreate on startup
```

### Automation Testing
```bash
# Test screen capture
luna-test screen-capture

# Test window automation  
luna-test window-control

# Test full automation flow
luna-test automation-flow
```

## 🎉 Next Steps

1. **Develop Luna features** in the VM environment
2. **Test automation capabilities** with Linux tools
3. **Build native app wrapper** for seamless distribution
4. **Package VM** for easy user deployment

The VM approach gives you the best development experience while ensuring users get the exact same environment you built Luna in!