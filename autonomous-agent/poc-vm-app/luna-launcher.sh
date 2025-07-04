#!/bin/bash
# Luna Agent Launcher - Seamless VM Experience
# This script demonstrates how a user would simply "click and run" Luna

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_NAME="luna-agent"
LUNA_PORT=8080
VM_CONFIG="$SCRIPT_DIR/config/vm-config.json"

echo "🌙 Starting Luna Agent..."

# Function to check if Luna is ready
check_luna_ready() {
    curl -s http://localhost:$LUNA_PORT/health >/dev/null 2>&1
}

# Function to wait for Luna to be ready
wait_for_luna() {
    echo "   Initializing Luna... (this may take a moment)"
    local max_attempts=30
    local attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        if check_luna_ready; then
            echo "   ✅ Luna is ready!"
            return 0
        fi
        
        printf "   ⏳ Starting up... (%d/%d)\r" $attempt $max_attempts
        sleep 2
        ((attempt++))
    done
    
    echo "   ❌ Failed to start Luna after $max_attempts attempts"
    return 1
}

# Function to start VM (simulated - would use actual VM commands)
start_vm() {
    echo "   🖥️  Starting Luna environment..."
    
    # In real implementation, this would:
    # - Check for existing VM
    # - Start VM with appropriate hypervisor
    # - Configure port forwarding
    # - Wait for VM to boot
    
    # Simulated VM startup
    $SCRIPT_DIR/vm-manager/start-vm.sh &
    VM_PID=$!
    
    echo "   🔧 Configuring environment..."
    sleep 3  # Simulate VM boot time
}

# Function to open Luna interface
open_luna_interface() {
    echo "   🚀 Launching Luna interface..."
    
    # In real implementation, this would open a native app window
    # For demo, we'll just show the URL
    if command -v python3 >/dev/null 2>&1; then
        # Open simple local web server for demo
        cd "$SCRIPT_DIR/native-app"
        python3 -m http.server 3000 >/dev/null 2>&1 &
        DEMO_PID=$!
        
        # Try to open in browser (cross-platform)
        if command -v xdg-open >/dev/null 2>&1; then
            xdg-open "http://localhost:3000/app.html" 2>/dev/null
        elif command -v open >/dev/null 2>&1; then
            open "http://localhost:3000/app.html" 2>/dev/null
        elif command -v start >/dev/null 2>&1; then
            start "http://localhost:3000/app.html" 2>/dev/null
        else
            echo "   📱 Open http://localhost:3000/app.html in your browser"
        fi
    else
        echo "   📱 Luna would open here: http://localhost:$LUNA_PORT"
    fi
}

# Function to cleanup on exit
cleanup() {
    echo ""
    echo "🌙 Shutting down Luna..."
    
    # Kill demo processes
    [ ! -z "$VM_PID" ] && kill $VM_PID 2>/dev/null || true
    [ ! -z "$DEMO_PID" ] && kill $DEMO_PID 2>/dev/null || true
    
    # In real implementation, would properly shutdown VM
    echo "   ✅ Luna shutdown complete"
}

# Set up cleanup trap
trap cleanup EXIT INT TERM

# Main execution flow
main() {
    # Check if Luna is already running
    if check_luna_ready; then
        echo "   ✅ Luna is already running!"
    else
        # Start VM and Luna agent
        start_vm
        
        # Wait for Luna to be ready
        if ! wait_for_luna; then
            echo "   ❌ Failed to start Luna. Please check your system configuration."
            exit 1
        fi
    fi
    
    # Open the interface
    open_luna_interface
    
    echo ""
    echo "🎉 Luna Agent is now running!"
    echo "   Interface: http://localhost:$LUNA_PORT"
    echo "   Press Ctrl+C to stop Luna"
    echo ""
    
    # Keep running until user stops
    wait
}

# Run main function
main "$@"
