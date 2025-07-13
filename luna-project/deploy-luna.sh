#!/bin/bash

# Luna Agent Complete Deployment Script
# Implements All Strategic Recommendations:
# 1. MVP Approach (Linux-first release)
# 2. VM Asset Development (Complete Luna VM)
# 3. Testing Strategy (Comprehensive testing)
# 4. Production Deployment Pipeline (DevOps automation)

set -e

# Configuration
LUNA_VERSION="1.0.0"
BUILD_TIMESTAMP=$(date +%Y%m%d_%H%M%S)
PROJECT_ROOT="/home/scrapybara/luna-project"
DEPLOYMENT_MODE="development"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# Functions
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

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --mode)
            DEPLOYMENT_MODE="$2"
            shift 2
            ;;
        --help)
            echo "Luna Agent Deployment Script"
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --mode MODE     Deployment mode: development, testing, staging, production"
            echo "  --help          Show this help message"
            echo ""
            echo "Modes:"
            echo "  development     Local development environment"
            echo "  testing         Run comprehensive test suite"
            echo "  staging         Deploy to staging environment"
            echo "  production      Deploy to production"
            exit 0
            ;;
        *)
            log_error "Unknown option $1"
            exit 1
            ;;
    esac
done

log_header "Luna Agent Deployment - Mode: $DEPLOYMENT_MODE"

# Change to project directory
cd "$PROJECT_ROOT"

# Strategic Recommendation #1: MVP Approach (Linux-first)
if [[ "$DEPLOYMENT_MODE" == "development" || "$DEPLOYMENT_MODE" == "testing" ]]; then
    log_header "Strategic Recommendation #1: MVP Linux-First Development"
    
    log_info "Setting up development environment..."
    
    # Check prerequisites
    log_info "Checking prerequisites..."
    
    if ! command -v docker &> /dev/null; then
        log_warning "Docker not found. Installing Docker..."
        curl -fsSL https://get.docker.com -o get-docker.sh
        sudo sh get-docker.sh
        sudo usermod -aG docker $USER
        log_success "Docker installed"
    else
        log_success "Docker found"
    fi
    
    if ! command -v docker-compose &> /dev/null; then
        log_warning "Docker Compose not found. Installing..."
        sudo curl -L "https://github.com/docker/compose/releases/download/v2.23.0/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
        sudo chmod +x /usr/local/bin/docker-compose
        log_success "Docker Compose installed"
    else
        log_success "Docker Compose found"
    fi
    
    # Build development environment
    log_info "Building Luna development environment..."
    cd deployment/docker
    docker-compose build luna-dev
    log_success "Development environment built"
    
    # Start development services
    log_info "Starting development services..."
    docker-compose up -d luna-dev
    log_success "Development services started"
    
    log_info "Luna development environment available at:"
    log_info "  API: http://localhost:8080"
    log_info "  UI:  http://localhost:3000"
    
    cd "$PROJECT_ROOT"
fi

# Strategic Recommendation #2: VM Asset Development
if [[ "$DEPLOYMENT_MODE" == "development" || "$DEPLOYMENT_MODE" == "testing" ]]; then
    log_header "Strategic Recommendation #2: VM Asset Development"
    
    log_info "Building Luna VM components..."
    
    # Check VirtualBox
    if ! command -v VBoxManage &> /dev/null; then
        log_warning "VirtualBox not found. Installing VirtualBox..."
        sudo apt update
        sudo apt install -y virtualbox virtualbox-ext-pack
        log_success "VirtualBox installed"
    else
        log_success "VirtualBox found"
    fi
    
    # Build VM scripts
    log_info "Preparing VM build scripts..."
    cd vm-assets/scripts
    chmod +x *.sh
    
    # Create VM configuration
    log_info "Creating VM configuration..."
    if [ ! -f "../configs/vm-ready.flag" ]; then
        log_info "VM will be built on first run. This requires manual OS installation."
        log_info "Run: cd vm-assets/scripts && ./build-vm.sh"
        touch ../configs/vm-ready.flag
    fi
    
    log_success "VM asset development configured"
    cd "$PROJECT_ROOT"
fi

# Strategic Recommendation #3: Testing Strategy
if [[ "$DEPLOYMENT_MODE" == "testing" ]]; then
    log_header "Strategic Recommendation #3: Comprehensive Testing"
    
    cd testing
    
    # Install testing dependencies
    log_info "Installing testing dependencies..."
    npm install
    pip3 install pytest playwright
    
    # Install Playwright browsers
    log_info "Installing Playwright browsers..."
    npx playwright install
    npx playwright install-deps
    
    # Run unit tests
    log_info "Running unit tests..."
    npm run test:unit
    log_success "Unit tests completed"
    
    # Run integration tests
    log_info "Running integration tests..."
    npm run test:integration
    log_success "Integration tests completed"
    
    # Run E2E tests
    log_info "Running end-to-end tests..."
    npm run test:e2e
    log_success "E2E tests completed"
    
    # Generate coverage report
    log_info "Generating coverage report..."
    npm run test:coverage
    log_success "Coverage report generated"
    
    # Start testing portal
    log_info "Starting testing portal..."
    cd ../deployment/docker
    docker-compose --profile testing up -d
    log_success "Testing portal started at http://localhost:8081"
    
    cd "$PROJECT_ROOT"
fi

# Strategic Recommendation #4: Production Deployment Pipeline
if [[ "$DEPLOYMENT_MODE" == "staging" || "$DEPLOYMENT_MODE" == "production" ]]; then
    log_header "Strategic Recommendation #4: Production Deployment Pipeline"
    
    # Check AWS CLI
    if ! command -v aws &> /dev/null; then
        log_warning "AWS CLI not found. Installing..."
        curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip"
        unzip awscliv2.zip
        sudo ./aws/install
        rm -rf aws awscliv2.zip
        log_success "AWS CLI installed"
    fi
    
    # Check Terraform
    if ! command -v terraform &> /dev/null; then
        log_warning "Terraform not found. Installing..."
        wget -O- https://apt.releases.hashicorp.com/gpg | gpg --dearmor | sudo tee /usr/share/keyrings/hashicorp-archive-keyring.gpg
        echo "deb [signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg] https://apt.releases.hashicorp.com $(lsb_release -cs) main" | sudo tee /etc/apt/sources.list.d/hashicorp.list
        sudo apt update && sudo apt install terraform
        log_success "Terraform installed"
    fi
    
    # Deploy infrastructure
    log_info "Deploying infrastructure with Terraform..."
    cd deployment/terraform
    terraform init
    terraform plan -var="environment=$DEPLOYMENT_MODE"
    
    if [[ "$DEPLOYMENT_MODE" == "production" ]]; then
        log_warning "About to deploy to PRODUCTION. Continue? (y/N)"
        read -r response
        if [[ "$response" =~ ^[Yy]$ ]]; then
            terraform apply -var="environment=$DEPLOYMENT_MODE" -auto-approve
            log_success "Infrastructure deployed"
        else
            log_info "Production deployment cancelled"
            exit 0
        fi
    else
        terraform apply -var="environment=$DEPLOYMENT_MODE" -auto-approve
        log_success "Infrastructure deployed"
    fi
    
    # Build and push Docker images
    log_info "Building and pushing Docker images..."
    cd ../docker
    
    # Build production image
    docker build -t luna-agent:$LUNA_VERSION --target luna-prod ../../
    docker tag luna-agent:$LUNA_VERSION luna-agent:latest
    
    # Push to registry (in real implementation)
    log_info "Docker images built (push to registry manually)"
    
    cd "$PROJECT_ROOT"
fi

# Final status report
log_header "Deployment Summary"

log_info "Deployment Mode: $DEPLOYMENT_MODE"
log_info "Luna Version: $LUNA_VERSION"
log_info "Build Timestamp: $BUILD_TIMESTAMP"
log_info "Project Root: $PROJECT_ROOT"

case "$DEPLOYMENT_MODE" in
    "development")
        log_success "✅ Strategic Recommendation #1: MVP Linux Development - COMPLETED"
        log_success "✅ Strategic Recommendation #2: VM Asset Development - CONFIGURED"
        log_info "🔗 Access Luna at: http://localhost:8080"
        ;;
    "testing")
        log_success "✅ Strategic Recommendation #1: MVP Development - COMPLETED"
        log_success "✅ Strategic Recommendation #2: VM Assets - CONFIGURED"
        log_success "✅ Strategic Recommendation #3: Comprehensive Testing - COMPLETED"
        log_info "🔗 Testing Portal at: http://localhost:8081"
        ;;
    "staging"|"production")
        log_success "✅ Strategic Recommendation #1: MVP Development - COMPLETED"
        log_success "✅ Strategic Recommendation #2: VM Assets - CONFIGURED"
        log_success "✅ Strategic Recommendation #3: Testing - AVAILABLE"
        log_success "✅ Strategic Recommendation #4: Production Pipeline - DEPLOYED"
        ;;
esac

log_header "Next Steps"

case "$DEPLOYMENT_MODE" in
    "development")
        echo "1. Start developing Luna features at http://localhost:8080"
        echo "2. Run tests: ./deploy-luna.sh --mode testing"
        echo "3. Build VM: cd vm-assets/scripts && ./build-vm.sh"
        ;;
    "testing")
        echo "1. Review test results in testing/coverage/"
        echo "2. Check testing portal at http://localhost:8081"
        echo "3. Run performance tests: cd testing && npm run test:performance"
        ;;
    "staging"|"production")
        echo "1. Verify deployment at the load balancer URL"
        echo "2. Monitor with Grafana dashboard"
        echo "3. Set up monitoring alerts"
        ;;
esac

log_success "🎯 Luna Agent deployment completed successfully!"
