#!/bin/bash

# Luna VM Development Environment Setup
# Implements Strategic Recommendation #2: VM Asset Development

set -e

echo "🚀 Luna VM Development Environment Setup"
echo "========================================"

# Configuration
LUNA_PROJECT_ROOT="/home/scrapybara/luna-project"
LUNA_VM_DIR="$LUNA_PROJECT_ROOT/vm-assets"
LUNA_INSTALLER_DIR="$LUNA_PROJECT_ROOT/installer"
LUNA_TESTING_DIR="$LUNA_PROJECT_ROOT/testing"
LUNA_DEPLOY_DIR="$LUNA_PROJECT_ROOT/deployment"

echo "📁 Creating Luna project structure..."

# Create main project structure
mkdir -p "$LUNA_PROJECT_ROOT"
mkdir -p "$LUNA_VM_DIR"/{base,scripts,configs,software}
mkdir -p "$LUNA_INSTALLER_DIR"/{linux,windows,macos,shared}
mkdir -p "$LUNA_TESTING_DIR"/{unit,integration,e2e,user-acceptance}
mkdir -p "$LUNA_DEPLOY_DIR"/{docker,kubernetes,terraform,github-actions}

echo "✅ Project structure created at: $LUNA_PROJECT_ROOT"
echo "🎯 Luna VM Development Environment Ready!"
