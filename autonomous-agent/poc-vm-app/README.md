# Luna VM Agent - Proof of Concept

This directory contains a simplified proof-of-concept for the seamless VM agent application.

## Quick Start Demo

```bash
# Simulate the user experience
./luna-launcher.sh
```

This would:
1. Check if VM is running
2. Start VM if needed (background)
3. Wait for Luna agent to be ready
4. Open native window pointing to VM
5. Hide all VM complexity from user

## Architecture

```
luna-launcher.sh (entry point)
├── vm-manager/ (VM lifecycle)
├── luna-vm-image/ (pre-built VM)
├── native-app/ (GUI wrapper)
└── config/ (auto-configuration)
```

## Files Structure

- `luna-launcher.sh` - Main entry point
- `vm-manager/start-vm.sh` - VM startup logic
- `native-app/app.html` - Simple GUI wrapper
- `config/vm-config.json` - VM configuration

This demonstrates the core concept with minimal implementation complexity.