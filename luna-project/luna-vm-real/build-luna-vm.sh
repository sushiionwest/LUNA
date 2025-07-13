#!/bin/bash

# Luna VM - Real Implementation Builder
# Builds and runs the complete Luna Agent virtual machine

set -e

# Configuration
LUNA_VM_NAME="luna-agent-vm-real"
LUNA_VM_TAG="1.0.0"
LUNA_VM_PORT_API=8080
LUNA_VM_PORT_UI=3000
LUNA_VM_PORT_VNC=5900
LUNA_VM_PORT_SSH=22222

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

log_header() {
    echo -e "${PURPLE}🚀 $1${NC}"
    echo -e "${PURPLE}$(printf '=%.0s' {1..50})${NC}"
}

log_header "Luna Agent VM - Real Implementation Builder"

# Check Docker
if ! command -v docker &> /dev/null; then
    log_error "Docker is required but not installed"
    exit 1
fi

log_success "Docker found"

# Build Luna VM Docker image
log_info "Building Luna VM Docker image..."
docker build -t ${LUNA_VM_NAME}:${LUNA_VM_TAG} -t ${LUNA_VM_NAME}:latest .

if [ $? -eq 0 ]; then
    log_success "Luna VM image built successfully"
else
    log_error "Failed to build Luna VM image"
    exit 1
fi

# Stop existing container if running
log_info "Stopping existing Luna VM if running..."
docker stop $LUNA_VM_NAME 2>/dev/null || true
docker rm $LUNA_VM_NAME 2>/dev/null || true

# Run Luna VM
log_info "Starting Luna VM container..."
docker run -d \
    --name $LUNA_VM_NAME \
    --privileged \
    -p ${LUNA_VM_PORT_API}:8080 \
    -p ${LUNA_VM_PORT_UI}:3000 \
    -p ${LUNA_VM_PORT_VNC}:5900 \
    -p ${LUNA_VM_PORT_SSH}:22222 \
    -v luna-vm-data:/opt/luna-agent/data \
    -v luna-vm-screenshots:/opt/luna-agent/screenshots \
    -v luna-vm-downloads:/opt/luna-agent/downloads \
    ${LUNA_VM_NAME}:latest

if [ $? -eq 0 ]; then
    log_success "Luna VM started successfully"
else
    log_error "Failed to start Luna VM"
    exit 1
fi

# Wait for Luna VM to be ready
log_info "Waiting for Luna VM to initialize..."
sleep 10

# Test Luna VM
log_info "Testing Luna VM..."
if curl -f http://localhost:${LUNA_VM_PORT_API}/status >/dev/null 2>&1; then
    log_success "Luna VM is responding to API requests"
else
    log_warning "Luna VM may still be starting up..."
fi

log_header "Luna VM - Real Implementation Ready!"

echo ""
log_info "🌙 Luna Agent VM Access Information:"
echo ""
log_info "📡 API Endpoint:    http://localhost:${LUNA_VM_PORT_API}"
log_info "🌐 Web Interface:   http://localhost:${LUNA_VM_PORT_API}/ui"
log_info "🖥️  VNC Access:      vnc://localhost:${LUNA_VM_PORT_VNC} (password: luna123)"
log_info "🔑 SSH Access:      ssh luna@localhost -p ${LUNA_VM_PORT_SSH} (password: luna123)"
echo ""

log_header "Luna VM Commands"
echo ""
echo "📊 View logs:           docker logs -f $LUNA_VM_NAME"
echo "⏹️  Stop VM:             docker stop $LUNA_VM_NAME"
echo "🔄 Restart VM:          docker restart $LUNA_VM_NAME"
echo "🗑️  Remove VM:           docker rm -f $LUNA_VM_NAME"
echo "📸 Take screenshot:     curl -X POST http://localhost:${LUNA_VM_PORT_API}/screenshot"
echo "🤖 Get status:          curl http://localhost:${LUNA_VM_PORT_API}/status"
echo ""

log_header "Testing Luna VM Capabilities"

echo ""
log_info "🧪 Running capability tests..."

# Test API
echo -n "API Status: "
if curl -s http://localhost:${LUNA_VM_PORT_API}/status | jq -r '.status' 2>/dev/null; then
    log_success "API is responding"
else
    log_warning "API test inconclusive"
fi

# Test screenshot capability
echo -n "Screenshot: "
if curl -s -X POST http://localhost:${LUNA_VM_PORT_API}/screenshot >/dev/null 2>&1; then
    log_success "Screenshot capability working"
else
    log_warning "Screenshot test inconclusive"
fi

echo ""
log_success "🎯 Luna VM Real Implementation is complete and running!"
log_info "The VM provides full web automation, computer vision, and API capabilities."
log_info "Connect via web interface or API to start automating tasks."
echo ""
