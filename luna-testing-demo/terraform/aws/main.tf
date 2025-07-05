# Luna Testing Infrastructure - AWS Terraform Configuration

terraform {
  required_version = ">= 1.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.0"
    }
    helm = {
      source  = "hashicorp/helm"
      version = "~> 2.0"
    }
  }
  
  backend "s3" {
    bucket = "luna-testing-terraform-state"
    key    = "infrastructure/terraform.tfstate"
    region = "us-east-1"
  }
}

provider "aws" {
  region = var.aws_region
  
  default_tags {
    tags = {
      Project     = "Luna Testing Infrastructure"
      Environment = var.environment
      ManagedBy   = "Terraform"
    }
  }
}

# Variables
variable "aws_region" {
  description = "AWS region for resources"
  type        = string
  default     = "us-east-1"
}

variable "environment" {
  description = "Environment name"
  type        = string
  default     = "production"
}

variable "cluster_name" {
  description = "EKS cluster name"
  type        = string
  default     = "luna-testing-cluster"
}

variable "domain_name" {
  description = "Domain name for the application"
  type        = string
  default     = "luna-testing.yourdomain.com"
}

# Data sources
data "aws_availability_zones" "available" {
  state = "available"
}

data "aws_caller_identity" "current" {}

# VPC
resource "aws_vpc" "luna_vpc" {
  cidr_block           = "10.0.0.0/16"
  enable_dns_hostnames = true
  enable_dns_support   = true

  tags = {
    Name = "luna-testing-vpc"
  }
}

# Internet Gateway
resource "aws_internet_gateway" "luna_igw" {
  vpc_id = aws_vpc.luna_vpc.id

  tags = {
    Name = "luna-testing-igw"
  }
}

# Public Subnets
resource "aws_subnet" "public" {
  count = 2

  vpc_id                  = aws_vpc.luna_vpc.id
  cidr_block              = "10.0.${count.index + 1}.0/24"
  availability_zone       = data.aws_availability_zones.available.names[count.index]
  map_public_ip_on_launch = true

  tags = {
    Name = "luna-testing-public-${count.index + 1}"
    Type = "public"
  }
}

# Private Subnets
resource "aws_subnet" "private" {
  count = 2

  vpc_id            = aws_vpc.luna_vpc.id
  cidr_block        = "10.0.${count.index + 10}.0/24"
  availability_zone = data.aws_availability_zones.available.names[count.index]

  tags = {
    Name = "luna-testing-private-${count.index + 1}"
    Type = "private"
  }
}

# NAT Gateways
resource "aws_eip" "nat" {
  count = 2
  domain = "vpc"

  tags = {
    Name = "luna-testing-nat-eip-${count.index + 1}"
  }
}

resource "aws_nat_gateway" "luna_nat" {
  count = 2

  allocation_id = aws_eip.nat[count.index].id
  subnet_id     = aws_subnet.public[count.index].id

  tags = {
    Name = "luna-testing-nat-${count.index + 1}"
  }
}

# Route Tables
resource "aws_route_table" "public" {
  vpc_id = aws_vpc.luna_vpc.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.luna_igw.id
  }

  tags = {
    Name = "luna-testing-public-rt"
  }
}

resource "aws_route_table" "private" {
  count = 2

  vpc_id = aws_vpc.luna_vpc.id

  route {
    cidr_block     = "0.0.0.0/0"
    nat_gateway_id = aws_nat_gateway.luna_nat[count.index].id
  }

  tags = {
    Name = "luna-testing-private-rt-${count.index + 1}"
  }
}

# Route Table Associations
resource "aws_route_table_association" "public" {
  count = 2

  subnet_id      = aws_subnet.public[count.index].id
  route_table_id = aws_route_table.public.id
}

resource "aws_route_table_association" "private" {
  count = 2

  subnet_id      = aws_subnet.private[count.index].id
  route_table_id = aws_route_table.private[count.index].id
}

# EKS Cluster
resource "aws_eks_cluster" "luna_cluster" {
  name     = var.cluster_name
  role_arn = aws_iam_role.cluster_role.arn
  version  = "1.28"

  vpc_config {
    subnet_ids = concat(aws_subnet.public[*].id, aws_subnet.private[*].id)
    
    endpoint_config {
      private_access = true
      public_access  = true
      public_access_cidrs = ["0.0.0.0/0"]
    }
  }

  depends_on = [
    aws_iam_role_policy_attachment.cluster_policy,
    aws_iam_role_policy_attachment.vpc_resource_controller,
  ]

  tags = {
    Name = var.cluster_name
  }
}

# EKS Node Group
resource "aws_eks_node_group" "luna_nodes" {
  cluster_name    = aws_eks_cluster.luna_cluster.name
  node_group_name = "luna-testing-nodes"
  node_role_arn   = aws_iam_role.node_role.arn
  subnet_ids      = aws_subnet.private[*].id
  instance_types  = ["t3.medium"]

  scaling_config {
    desired_size = 3
    max_size     = 10
    min_size     = 2
  }

  update_config {
    max_unavailable = 1
  }

  depends_on = [
    aws_iam_role_policy_attachment.node_policy,
    aws_iam_role_policy_attachment.cni_policy,
    aws_iam_role_policy_attachment.registry_readonly,
  ]

  tags = {
    Name = "luna-testing-nodes"
  }
}

# IAM Roles
resource "aws_iam_role" "cluster_role" {
  name = "luna-eks-cluster-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "eks.amazonaws.com"
        }
      }
    ]
  })
}

resource "aws_iam_role" "node_role" {
  name = "luna-eks-node-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ec2.amazonaws.com"
        }
      }
    ]
  })
}

# IAM Role Policy Attachments
resource "aws_iam_role_policy_attachment" "cluster_policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKSClusterPolicy"
  role       = aws_iam_role.cluster_role.name
}

resource "aws_iam_role_policy_attachment" "vpc_resource_controller" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKSVPCResourceController"
  role       = aws_iam_role.cluster_role.name
}

resource "aws_iam_role_policy_attachment" "node_policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKSWorkerNodePolicy"
  role       = aws_iam_role.node_role.name
}

resource "aws_iam_role_policy_attachment" "cni_policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKS_CNI_Policy"
  role       = aws_iam_role.node_role.name
}

resource "aws_iam_role_policy_attachment" "registry_readonly" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEC2ContainerRegistryReadOnly"
  role       = aws_iam_role.node_role.name
}

# RDS PostgreSQL
resource "aws_db_subnet_group" "luna_db_subnet_group" {
  name       = "luna-testing-db-subnet-group"
  subnet_ids = aws_subnet.private[*].id

  tags = {
    Name = "Luna Testing DB subnet group"
  }
}

resource "aws_security_group" "rds_sg" {
  name        = "luna-testing-rds-sg"
  description = "Security group for Luna Testing RDS"
  vpc_id      = aws_vpc.luna_vpc.id

  ingress {
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    cidr_blocks = [aws_vpc.luna_vpc.cidr_block]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "luna-testing-rds-sg"
  }
}

resource "aws_db_instance" "luna_postgres" {
  identifier = "luna-testing-postgres"
  
  engine         = "postgres"
  engine_version = "15.4"
  instance_class = "db.t3.micro"
  
  allocated_storage     = 20
  max_allocated_storage = 100
  storage_type         = "gp2"
  storage_encrypted    = true
  
  db_name  = "luna_testing"
  username = "luna"
  password = random_password.db_password.result
  
  vpc_security_group_ids = [aws_security_group.rds_sg.id]
  db_subnet_group_name   = aws_db_subnet_group.luna_db_subnet_group.name
  
  backup_retention_period = 7
  backup_window          = "03:00-04:00"
  maintenance_window     = "sun:04:00-sun:05:00"
  
  skip_final_snapshot = false
  final_snapshot_identifier = "luna-testing-postgres-final-snapshot"
  
  tags = {
    Name = "luna-testing-postgres"
  }
}

# ElastiCache Redis
resource "aws_elasticache_subnet_group" "luna_cache_subnet_group" {
  name       = "luna-testing-cache-subnet-group"
  subnet_ids = aws_subnet.private[*].id
}

resource "aws_security_group" "redis_sg" {
  name        = "luna-testing-redis-sg"
  description = "Security group for Luna Testing Redis"
  vpc_id      = aws_vpc.luna_vpc.id

  ingress {
    from_port   = 6379
    to_port     = 6379
    protocol    = "tcp"
    cidr_blocks = [aws_vpc.luna_vpc.cidr_block]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "luna-testing-redis-sg"
  }
}

resource "aws_elasticache_replication_group" "luna_redis" {
  replication_group_id       = "luna-testing-redis"
  description                = "Redis cluster for Luna Testing"
  
  port                       = 6379
  parameter_group_name       = "default.redis7"
  node_type                 = "cache.t3.micro"
  num_cache_clusters         = 2
  
  subnet_group_name          = aws_elasticache_subnet_group.luna_cache_subnet_group.name
  security_group_ids         = [aws_security_group.redis_sg.id]
  
  at_rest_encryption_enabled = true
  transit_encryption_enabled = true
  auth_token                 = random_password.redis_password.result
  
  tags = {
    Name = "luna-testing-redis"
  }
}

# S3 Bucket for file uploads
resource "aws_s3_bucket" "luna_uploads" {
  bucket = "luna-testing-uploads-${random_id.bucket_suffix.hex}"

  tags = {
    Name = "luna-testing-uploads"
  }
}

resource "aws_s3_bucket_public_access_block" "luna_uploads_pab" {
  bucket = aws_s3_bucket.luna_uploads.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_versioning" "luna_uploads_versioning" {
  bucket = aws_s3_bucket.luna_uploads.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "luna_uploads_encryption" {
  bucket = aws_s3_bucket.luna_uploads.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

# Random passwords
resource "random_password" "db_password" {
  length = 16
  special = true
}

resource "random_password" "redis_password" {
  length = 32
  special = false
}

resource "random_id" "bucket_suffix" {
  byte_length = 4
}

# Outputs
output "cluster_endpoint" {
  description = "EKS cluster endpoint"
  value       = aws_eks_cluster.luna_cluster.endpoint
}

output "cluster_name" {
  description = "EKS cluster name"
  value       = aws_eks_cluster.luna_cluster.name
}

output "database_endpoint" {
  description = "PostgreSQL database endpoint"
  value       = aws_db_instance.luna_postgres.endpoint
  sensitive   = true
}

output "database_password" {
  description = "PostgreSQL database password"
  value       = random_password.db_password.result
  sensitive   = true
}

output "redis_endpoint" {
  description = "Redis cluster endpoint"
  value       = aws_elasticache_replication_group.luna_redis.primary_endpoint_address
  sensitive   = true
}

output "redis_password" {
  description = "Redis authentication token"
  value       = random_password.redis_password.result
  sensitive   = true
}

output "s3_bucket_name" {
  description = "S3 bucket name for uploads"
  value       = aws_s3_bucket.luna_uploads.bucket
}