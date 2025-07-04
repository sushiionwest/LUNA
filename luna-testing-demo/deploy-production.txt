#!/bin/bash

# Luna Testing Infrastructure Production Deployment Script
# This script automates the complete deployment process

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
ENVIRONMENT=${ENVIRONMENT:-production}
AWS_REGION=${AWS_REGION:-us-east-1}
CLUSTER_NAME=${CLUSTER_NAME:-luna-testing-cluster}
DOMAIN_NAME=${DOMAIN_NAME:-luna-testing.yourdomain.com}
NAMESPACE=${NAMESPACE:-luna-testing}

# Functions
log() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

check_prerequisites() {
    log "Checking prerequisites..."
    
    # Check required tools
    command -v docker >/dev/null 2>&1 || error "Docker is required but not installed"
    command -v kubectl >/dev/null 2>&1 || error "kubectl is required but not installed"
    command -v terraform >/dev/null 2>&1 || error "Terraform is required but not installed"
    command -v aws >/dev/null 2>&1 || error "AWS CLI is required but not installed"
    command -v helm >/dev/null 2>&1 || error "Helm is required but not installed"
    
    # Check environment variables
    [[ -z "$AWS_ACCESS_KEY_ID" ]] && error "AWS_ACCESS_KEY_ID environment variable is required"
    [[ -z "$AWS_SECRET_ACCESS_KEY" ]] && error "AWS_SECRET_ACCESS_KEY environment variable is required"
    
    success "Prerequisites check passed"
}

setup_terraform_backend() {
    log "Setting up Terraform backend..."
    
    # Create S3 bucket for Terraform state if it doesn't exist
    aws s3 ls s3://luna-testing-terraform-state >/dev/null 2>&1 || {
        log "Creating Terraform state bucket..."
        aws s3 mb s3://luna-testing-terraform-state --region $AWS_REGION
        aws s3api put-bucket-versioning --bucket luna-testing-terraform-state --versioning-configuration Status=Enabled
        aws s3api put-bucket-encryption --bucket luna-testing-terraform-state --server-side-encryption-configuration '{
            "Rules": [
                {
                    "ApplyServerSideEncryptionByDefault": {
                        "SSEAlgorithm": "AES256"
                    }
                }
            ]
        }'
    }
    
    success "Terraform backend configured"
}

deploy_infrastructure() {
    log "Deploying AWS infrastructure with Terraform..."
    
    cd terraform/aws
    
    # Initialize Terraform
    terraform init
    
    # Create terraform.tfvars if it doesn't exist
    if [[ ! -f terraform.tfvars ]]; then
        cat > terraform.tfvars << EOF
aws_region = "$AWS_REGION"
environment = "$ENVIRONMENT"
cluster_name = "$CLUSTER_NAME"
domain_name = "$DOMAIN_NAME"
EOF
    fi
    
    # Plan and apply
    terraform plan -out=tfplan
    terraform apply tfplan
    
    # Save outputs
    terraform output -json > ../../outputs.json
    
    cd ../..
    success "Infrastructure deployed successfully"
}

configure_kubectl() {
    log "Configuring kubectl for EKS cluster..."
    
    aws eks update-kubeconfig --region $AWS_REGION --name $CLUSTER_NAME
    
    # Test connection
    kubectl get nodes || error "Failed to connect to EKS cluster"
    
    success "kubectl configured successfully"
}

setup_cluster_addons() {
    log "Setting up cluster add-ons..."
    
    # Install AWS Load Balancer Controller
    curl -o iam_policy.json https://raw.githubusercontent.com/kubernetes-sigs/aws-load-balancer-controller/v2.7.2/docs/install/iam_policy.json
    
    aws iam create-policy \
        --policy-name AWSLoadBalancerControllerIAMPolicy \
        --policy-document file://iam_policy.json || true
    
    eksctl create iamserviceaccount \
        --cluster=$CLUSTER_NAME \
        --namespace=kube-system \
        --name=aws-load-balancer-controller \
        --role-name AmazonEKSLoadBalancerControllerRole \
        --attach-policy-arn=arn:aws:iam::$(aws sts get-caller-identity --query Account --output text):policy/AWSLoadBalancerControllerIAMPolicy \
        --approve || true
    
    # Add EKS chart repository
    helm repo add eks https://aws.github.io/eks-charts
    helm repo update
    
    # Install AWS Load Balancer Controller
    helm upgrade --install aws-load-balancer-controller eks/aws-load-balancer-controller \
        -n kube-system \
        --set clusterName=$CLUSTER_NAME \
        --set serviceAccount.create=false \
        --set serviceAccount.name=aws-load-balancer-controller
    
    # Install cert-manager
    kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.2/cert-manager.yaml
    
    # Wait for cert-manager to be ready
    kubectl wait --for=condition=Available --timeout=300s deployment/cert-manager -n cert-manager
    
    success "Cluster add-ons installed successfully"
}

build_and_push_image() {
    log "Building and pushing Docker image..."
    
    # Get AWS account ID
    AWS_ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
    ECR_REPOSITORY="${AWS_ACCOUNT_ID}.dkr.ecr.${AWS_REGION}.amazonaws.com/luna-testing"
    
    # Create ECR repository if it doesn't exist
    aws ecr describe-repositories --repository-names luna-testing --region $AWS_REGION >/dev/null 2>&1 || {
        log "Creating ECR repository..."
        aws ecr create-repository --repository-name luna-testing --region $AWS_REGION
    }
    
    # Login to ECR
    aws ecr get-login-password --region $AWS_REGION | docker login --username AWS --password-stdin $ECR_REPOSITORY
    
    # Build and tag image
    docker build -t luna-testing .
    docker tag luna-testing:latest $ECR_REPOSITORY:latest
    docker tag luna-testing:latest $ECR_REPOSITORY:$(date +%Y%m%d-%H%M%S)
    
    # Push image
    docker push $ECR_REPOSITORY:latest
    docker push $ECR_REPOSITORY:$(date +%Y%m%d-%H%M%S)
    
    # Update deployment manifest
    sed -i "s|luna-testing:latest|$ECR_REPOSITORY:latest|g" k8s/luna-deployment.yaml
    
    success "Docker image built and pushed successfully"
}

deploy_application() {
    log "Deploying Luna Testing application..."
    
    # Create namespace
    kubectl create namespace $NAMESPACE --dry-run=client -o yaml | kubectl apply -f -
    
    # Create secrets from Terraform outputs
    create_secrets
    
    # Apply Kubernetes manifests
    kubectl apply -f k8s/luna-deployment.yaml
    
    # Wait for deployment to be ready
    kubectl wait --for=condition=Available --timeout=600s deployment/luna-api -n $NAMESPACE
    
    success "Application deployed successfully"
}

create_secrets() {
    log "Creating Kubernetes secrets..."
    
    # Extract values from Terraform outputs
    DB_PASSWORD=$(jq -r '.database_password.value' outputs.json)
    REDIS_PASSWORD=$(jq -r '.redis_password.value' outputs.json)
    DB_ENDPOINT=$(jq -r '.database_endpoint.value' outputs.json)
    REDIS_ENDPOINT=$(jq -r '.redis_endpoint.value' outputs.json)
    S3_BUCKET=$(jq -r '.s3_bucket_name.value' outputs.json)
    
    # Create or update secrets
    kubectl create secret generic luna-secrets -n $NAMESPACE \
        --from-literal=POSTGRES_PASSWORD="$DB_PASSWORD" \
        --from-literal=REDIS_PASSWORD="$REDIS_PASSWORD" \
        --from-literal=JWT_SECRET="$(openssl rand -base64 32)" \
        --from-literal=EMAIL_API_KEY="${EMAIL_API_KEY:-placeholder}" \
        --from-literal=GRAFANA_PASSWORD="$(openssl rand -base64 16)" \
        --dry-run=client -o yaml | kubectl apply -f -
    
    # Create or update config map
    kubectl create configmap luna-config -n $NAMESPACE \
        --from-literal=DATABASE_URL="postgresql://luna:$DB_PASSWORD@$DB_ENDPOINT:5432/luna_testing" \
        --from-literal=REDIS_URL="redis://default:$REDIS_PASSWORD@$REDIS_ENDPOINT:6379" \
        --from-literal=S3_BUCKET="$S3_BUCKET" \
        --from-literal=AWS_REGION="$AWS_REGION" \
        --from-literal=DOMAIN_NAME="$DOMAIN_NAME" \
        --dry-run=client -o yaml | kubectl apply -f -
    
    success "Secrets created successfully"
}

setup_monitoring() {
    log "Setting up monitoring stack..."
    
    # Add Prometheus community Helm repository
    helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
    helm repo add grafana https://grafana.github.io/helm-charts
    helm repo update
    
    # Install Prometheus
    helm upgrade --install prometheus prometheus-community/kube-prometheus-stack \
        --namespace monitoring \
        --create-namespace \
        --set grafana.adminPassword="$(kubectl get secret luna-secrets -n $NAMESPACE -o jsonpath='{.data.GRAFANA_PASSWORD}' | base64 -d)" \
        --set prometheus.prometheusSpec.retention=30d \
        --set prometheus.prometheusSpec.storageSpec.volumeClaimTemplate.spec.resources.requests.storage=50Gi
    
    success "Monitoring stack deployed successfully"
}

configure_dns() {
    log "Configuring DNS and SSL..."
    
    # Create ClusterIssuer for Let's Encrypt
    cat << EOF | kubectl apply -f -
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: admin@$DOMAIN_NAME
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
    - http01:
        ingress:
          class: alb
EOF
    
    warning "Please update your DNS to point $DOMAIN_NAME to the Load Balancer created by the ingress controller"
    warning "You can find the Load Balancer hostname with: kubectl get ingress -n $NAMESPACE"
    
    success "DNS and SSL configuration applied"
}

run_health_checks() {
    log "Running health checks..."
    
    # Wait for ingress to get an address
    log "Waiting for ingress to get a load balancer address..."
    kubectl wait --for=condition=Ready --timeout=600s ingress/luna-ingress -n $NAMESPACE || warning "Ingress may still be provisioning"
    
    # Get the load balancer hostname
    LB_HOSTNAME=$(kubectl get ingress luna-ingress -n $NAMESPACE -o jsonpath='{.status.loadBalancer.ingress[0].hostname}')
    
    if [[ -n "$LB_HOSTNAME" ]]; then
        log "Load Balancer hostname: $LB_HOSTNAME"
        
        # Test health endpoint
        log "Testing health endpoint..."
        curl -f "http://$LB_HOSTNAME/health" || warning "Health check failed - application may still be starting"
        
        success "Health checks completed"
    else
        warning "Load Balancer not yet available - check ingress status manually"
    fi
}

cleanup() {
    log "Cleaning up temporary files..."
    rm -f iam_policy.json outputs.json
    success "Cleanup completed"
}

main() {
    log "Starting Luna Testing Infrastructure deployment to $ENVIRONMENT"
    
    check_prerequisites
    setup_terraform_backend
    deploy_infrastructure
    configure_kubectl
    setup_cluster_addons
    build_and_push_image
    deploy_application
    setup_monitoring
    configure_dns
    run_health_checks
    cleanup
    
    success "🎉 Luna Testing Infrastructure deployment completed successfully!"
    echo
    log "Next steps:"
    echo "  1. Update DNS to point $DOMAIN_NAME to the load balancer"
    echo "  2. Configure email service credentials in the luna-secrets secret"
    echo "  3. Access the application at https://$DOMAIN_NAME"
    echo "  4. Access monitoring at https://$DOMAIN_NAME/monitoring"
    echo "  5. Access admin dashboard at https://$DOMAIN_NAME/admin-dashboard.html"
    echo
    log "Useful commands:"
    echo "  kubectl get pods -n $NAMESPACE"
    echo "  kubectl logs -f deployment/luna-api -n $NAMESPACE"
    echo "  kubectl get ingress -n $NAMESPACE"
}

# Handle script arguments
case "${1:-}" in
    --check-only)
        check_prerequisites
        ;;
    --infrastructure-only)
        check_prerequisites
        setup_terraform_backend
        deploy_infrastructure
        ;;
    --application-only)
        check_prerequisites
        configure_kubectl
        build_and_push_image
        deploy_application
        ;;
    --monitoring-only)
        check_prerequisites
        configure_kubectl
        setup_monitoring
        ;;
    --destroy)
        warning "This will destroy all infrastructure!"
        read -p "Are you sure? (yes/no): " -r
        if [[ $REPLY =~ ^yes$ ]]; then
            cd terraform/aws
            terraform destroy -auto-approve
            success "Infrastructure destroyed"
        else
            log "Destruction cancelled"
        fi
        ;;
    *)
        main
        ;;
esac