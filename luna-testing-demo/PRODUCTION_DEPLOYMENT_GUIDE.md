# 🚀 Luna Testing Infrastructure - Production Deployment Guide

## Overview

This guide walks you through deploying the Luna Testing Infrastructure to a production environment with enterprise-grade scalability, security, and monitoring.

## Architecture

```
┌─────────────────────┐    ┌──────────────────┐    ┌─────────────────────┐
│                     │    │                  │    │                     │
│   Users/Browsers    │───►│  AWS ALB/CloudFront  │───►│  EKS Cluster        │
│   (Internet)        │    │  (Load Balancer) │    │  (Kubernetes)       │
│                     │    │                  │    │                     │
└─────────────────────┘    └──────────────────┘    └─────────────────────┘
                                                              │
                           ┌──────────────────┐              │
                           │                  │              │
                           │  RDS PostgreSQL  │◄─────────────┤
                           │  (Database)      │              │
                           │                  │              │
                           └──────────────────┘              │
                                                              │
                           ┌──────────────────┐              │
                           │                  │              │
                           │ ElastiCache Redis│◄─────────────┤
                           │ (Session Store)  │              │
                           │                  │              │
                           └──────────────────┘              │
                                                              │
                           ┌──────────────────┐              │
                           │                  │              │
                           │    S3 Bucket     │◄─────────────┘
                           │ (File Storage)   │
                           │                  │
                           └──────────────────┘
```

## Prerequisites

### Required Tools
- **Docker** (v20.0+)
- **kubectl** (v1.28+)
- **Terraform** (v1.0+)
- **AWS CLI** (v2.0+)
- **Helm** (v3.0+)
- **eksctl** (v0.150+)

### AWS Requirements
- AWS Account with appropriate permissions
- AWS CLI configured with access keys
- Route53 hosted zone (for custom domain)
- ACM certificate (for SSL/TLS)

### Environment Variables
```bash
export AWS_ACCESS_KEY_ID="your-access-key"
export AWS_SECRET_ACCESS_KEY="your-secret-key"
export AWS_REGION="us-east-1"
export DOMAIN_NAME="luna-testing.yourdomain.com"
export EMAIL_API_KEY="your-sendgrid-api-key"
```

## Quick Start (Automated Deployment)

### 1. Clone and Setup
```bash
git clone <your-repo-url>
cd luna-testing-infrastructure
chmod +x deploy-production.txt
mv deploy-production.txt deploy-production.sh
```

### 2. Configure Environment
```bash
cp .env.production .env
# Edit .env with your actual values
```

### 3. Deploy Everything
```bash
./deploy-production.sh
```

This automated script will:
- ✅ Check prerequisites
- ✅ Create AWS infrastructure with Terraform
- ✅ Set up EKS cluster with addons
- ✅ Build and push Docker images
- ✅ Deploy application to Kubernetes
- ✅ Configure monitoring and logging
- ✅ Set up SSL certificates
- ✅ Run health checks

## Manual Deployment (Step by Step)

### Phase 1: Infrastructure Setup

#### 1.1 Terraform Backend
```bash
# Create S3 bucket for Terraform state
aws s3 mb s3://luna-testing-terraform-state-$(date +%s) --region us-east-1
```

#### 1.2 Deploy AWS Infrastructure
```bash
cd terraform/aws
terraform init
terraform plan -var="domain_name=luna-testing.yourdomain.com"
terraform apply
```

**Resources Created:**
- VPC with public/private subnets
- EKS cluster with managed node group
- RDS PostgreSQL database
- ElastiCache Redis cluster  
- S3 bucket for file storage
- Security groups and IAM roles

#### 1.3 Configure kubectl
```bash
aws eks update-kubeconfig --region us-east-1 --name luna-testing-cluster
kubectl get nodes  # Verify connection
```

### Phase 2: Cluster Setup

#### 2.1 Install Cluster Add-ons
```bash
# AWS Load Balancer Controller
curl -o iam_policy.json https://raw.githubusercontent.com/kubernetes-sigs/aws-load-balancer-controller/v2.7.2/docs/install/iam_policy.json
aws iam create-policy --policy-name AWSLoadBalancerControllerIAMPolicy --policy-document file://iam_policy.json

eksctl create iamserviceaccount \
  --cluster=luna-testing-cluster \
  --namespace=kube-system \
  --name=aws-load-balancer-controller \
  --role-name AmazonEKSLoadBalancerControllerRole \
  --attach-policy-arn=arn:aws:iam::ACCOUNT-ID:policy/AWSLoadBalancerControllerIAMPolicy \
  --approve

helm repo add eks https://aws.github.io/eks-charts
helm install aws-load-balancer-controller eks/aws-load-balancer-controller \
  -n kube-system \
  --set clusterName=luna-testing-cluster \
  --set serviceAccount.create=false \
  --set serviceAccount.name=aws-load-balancer-controller
```

#### 2.2 Install cert-manager
```bash
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.2/cert-manager.yaml
```

### Phase 3: Application Deployment

#### 3.1 Build and Push Container Image
```bash
# Get AWS account ID and create ECR repository
AWS_ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
aws ecr create-repository --repository-name luna-testing --region us-east-1

# Build and push image
aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin $AWS_ACCOUNT_ID.dkr.ecr.us-east-1.amazonaws.com
docker build -t luna-testing .
docker tag luna-testing:latest $AWS_ACCOUNT_ID.dkr.ecr.us-east-1.amazonaws.com/luna-testing:latest
docker push $AWS_ACCOUNT_ID.dkr.ecr.us-east-1.amazonaws.com/luna-testing:latest
```

#### 3.2 Create Kubernetes Secrets
```bash
# Extract database credentials from Terraform output
DB_PASSWORD=$(terraform output -raw database_password)
REDIS_PASSWORD=$(terraform output -raw redis_password)

kubectl create namespace luna-testing
kubectl create secret generic luna-secrets -n luna-testing \
  --from-literal=POSTGRES_PASSWORD="$DB_PASSWORD" \
  --from-literal=REDIS_PASSWORD="$REDIS_PASSWORD" \
  --from-literal=JWT_SECRET="$(openssl rand -base64 32)" \
  --from-literal=EMAIL_API_KEY="$EMAIL_API_KEY"
```

#### 3.3 Deploy Application
```bash
# Update image in deployment manifest
sed -i "s|luna-testing:latest|$AWS_ACCOUNT_ID.dkr.ecr.us-east-1.amazonaws.com/luna-testing:latest|g" k8s/luna-deployment.yaml

# Apply Kubernetes manifests
kubectl apply -f k8s/luna-deployment.yaml

# Wait for deployment
kubectl wait --for=condition=Available --timeout=600s deployment/luna-api -n luna-testing
```

### Phase 4: Monitoring and Observability

#### 4.1 Install Prometheus Stack
```bash
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm install prometheus prometheus-community/kube-prometheus-stack \
  --namespace monitoring \
  --create-namespace \
  --set grafana.adminPassword="secure_password_here"
```

#### 4.2 Configure Alerts
```bash
# Create custom alert rules
kubectl apply -f - << EOF
apiVersion: monitoring.coreos.com/v1
kind: PrometheusRule
metadata:
  name: luna-testing-alerts
  namespace: monitoring
spec:
  groups:
  - name: luna-testing
    rules:
    - alert: LunaAPIDown
      expr: up{job="luna-api"} == 0
      for: 5m
      labels:
        severity: critical
      annotations:
        summary: "Luna API is down"
    - alert: HighErrorRate
      expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.1
      for: 10m
      labels:
        severity: warning
      annotations:
        summary: "High error rate detected"
EOF
```

### Phase 5: Domain and SSL Configuration

#### 5.1 Configure DNS
```bash
# Get load balancer hostname
LB_HOSTNAME=$(kubectl get ingress luna-ingress -n luna-testing -o jsonpath='{.status.loadBalancer.ingress[0].hostname}')

# Create Route53 record (replace with your hosted zone ID)
aws route53 change-resource-record-sets --hosted-zone-id Z1234567890 --change-batch '{
  "Changes": [{
    "Action": "UPSERT",
    "ResourceRecordSet": {
      "Name": "luna-testing.yourdomain.com",
      "Type": "CNAME",
      "TTL": 300,
      "ResourceRecords": [{"Value": "'$LB_HOSTNAME'"}]
    }
  }]
}'
```

#### 5.2 Set up SSL with Let's Encrypt
```bash
kubectl apply -f - << EOF
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: admin@yourdomain.com
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
    - http01:
        ingress:
          class: alb
EOF
```

## Production Configuration

### Database Migration
```bash
# Connect to RDS instance and initialize schema
DB_ENDPOINT=$(terraform output -raw database_endpoint)
PGPASSWORD=$DB_PASSWORD psql -h $DB_ENDPOINT -U luna -d luna_testing -f database/init.sql.txt
```

### Email Service Setup
Choose one of the following email services:

#### Option 1: SendGrid
```bash
kubectl patch secret luna-secrets -n luna-testing -p='{"data":{"EMAIL_API_KEY":"'$(echo -n "$SENDGRID_API_KEY" | base64)'"}}'
```

#### Option 2: AWS SES
```bash
# Configure SES and update environment variables
kubectl patch configmap luna-config -n luna-testing -p='{"data":{"EMAIL_SERVICE":"ses"}}'
```

### Environment Variables
Update the ConfigMap with production values:
```bash
kubectl patch configmap luna-config -n luna-testing -p='{
  "data": {
    "NODE_ENV": "production",
    "CORS_ORIGIN": "https://luna-testing.yourdomain.com",
    "ENABLE_ANALYTICS": "true",
    "LOG_LEVEL": "info"
  }
}'
```

## Scaling Configuration

### Horizontal Pod Autoscaler
```bash
kubectl apply -f - << EOF
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: luna-api-hpa
  namespace: luna-testing
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: luna-api
  minReplicas: 3
  maxReplicas: 20
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
EOF
```

### Cluster Autoscaler
```bash
# Update node group to enable autoscaling
aws eks update-nodegroup-config \
  --cluster-name luna-testing-cluster \
  --nodegroup-name luna-testing-nodes \
  --scaling-config minSize=2,maxSize=50,desiredSize=3
```

## Security Hardening

### Network Policies
```bash
kubectl apply -f - << EOF
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: luna-network-policy
  namespace: luna-testing
spec:
  podSelector: {}
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: kube-system
  - from:
    - namespaceSelector:
        matchLabels:
          name: luna-testing
  egress:
  - to: []
EOF
```

### Pod Security Standards
```bash
kubectl label namespace luna-testing pod-security.kubernetes.io/enforce=restricted
kubectl label namespace luna-testing pod-security.kubernetes.io/audit=restricted
kubectl label namespace luna-testing pod-security.kubernetes.io/warn=restricted
```

## Monitoring and Alerting

### Access Monitoring
- **Grafana**: `https://luna-testing.yourdomain.com/monitoring`
- **Prometheus**: Accessible through Grafana or port-forward
- **Application Logs**: `kubectl logs -f deployment/luna-api -n luna-testing`

### Key Metrics to Monitor
- Application response time and error rates
- Database connection pool usage
- Redis memory utilization
- Pod CPU and memory usage
- Network traffic and latency

### Alert Channels
Configure alerts to be sent to:
- Slack/Teams channels
- Email notifications
- PagerDuty for critical alerts

## Backup and Disaster Recovery

### Database Backups
```bash
# Automated daily backups are configured in RDS
# Manual backup:
aws rds create-db-snapshot \
  --db-instance-identifier luna-testing-postgres \
  --db-snapshot-identifier luna-manual-backup-$(date +%Y%m%d)
```

### Application State Backup
```bash
# Backup Kubernetes configurations
kubectl get all -n luna-testing -o yaml > luna-k8s-backup-$(date +%Y%m%d).yaml
```

## Troubleshooting

### Common Issues

#### 1. Pods Not Starting
```bash
kubectl describe pod <pod-name> -n luna-testing
kubectl logs <pod-name> -n luna-testing
```

#### 2. Database Connection Issues
```bash
# Test database connectivity
kubectl run -it --rm debug --image=postgres:15 --restart=Never -- psql -h <db-endpoint> -U luna -d luna_testing
```

#### 3. SSL Certificate Issues
```bash
kubectl describe certificate luna-tls-secret -n luna-testing
kubectl describe clusterissuer letsencrypt-prod
```

#### 4. Load Balancer Not Working
```bash
kubectl describe ingress luna-ingress -n luna-testing
kubectl logs -n kube-system deployment/aws-load-balancer-controller
```

### Health Checks
```bash
# Application health
curl https://luna-testing.yourdomain.com/health

# Database health
curl https://luna-testing.yourdomain.com/api/participants/analytics/summary

# Comprehensive system check
./scripts/health-check.sh
```

## Maintenance

### Regular Updates
1. **Application Updates**: Use CI/CD pipeline for automated deployments
2. **Security Patches**: Update base images and dependencies monthly
3. **Infrastructure Updates**: Apply Terraform updates during maintenance windows
4. **Certificate Renewal**: Automatic with cert-manager

### Performance Optimization
1. **Database Query Analysis**: Use RDS Performance Insights
2. **Application Profiling**: Implement APM tools
3. **Resource Right-sizing**: Monitor and adjust based on usage patterns

## Cost Optimization

### Resource Optimization
- Use Spot instances for non-critical workloads
- Implement resource requests and limits
- Schedule scaling down during off-hours
- Use Reserved Instances for predictable workloads

### Monitoring Costs
- Set up AWS Budget alerts
- Use AWS Cost Explorer to track resource usage
- Implement resource tagging for cost allocation

## Support and Documentation

### Runbooks
- Incident response procedures
- Deployment rollback procedures
- Scaling procedures
- Backup and restore procedures

### Contact Information
- **Infrastructure Team**: infrastructure@yourcompany.com
- **Application Team**: luna-dev@yourcompany.com
- **On-call Rotation**: Use PagerDuty or similar

---

## Success Criteria

✅ **Availability**: 99.9% uptime SLA  
✅ **Performance**: <200ms API response time  
✅ **Scalability**: Auto-scale 2-50 pods based on load  
✅ **Security**: All traffic encrypted, security scanning enabled  
✅ **Monitoring**: Full observability with alerts  
✅ **Backup**: Automated daily backups with 30-day retention  

**🎉 Your Luna Testing Infrastructure is now production-ready and enterprise-grade!**