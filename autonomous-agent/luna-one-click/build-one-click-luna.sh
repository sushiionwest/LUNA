#!/bin/bash
# build-one-click-luna.sh - Build complete one-click Luna installer

set -e

echo "🌙 Building One-Click Luna Agent..."

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="$PROJECT_ROOT/dist"
VM_ASSETS_DIR="$PROJECT_ROOT/vm-assets"

# Clean previous builds
echo "🧹 Cleaning previous builds..."
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"
mkdir -p "$VM_ASSETS_DIR"

# Step 1: Build the VM image
echo "📦 Building Luna VM image..."
cd "$PROJECT_ROOT/../luna-vm-dev"

# Create optimized VM for distribution
if [ ! -f "$VM_ASSETS_DIR/luna-vm.7z" ]; then
    echo "Creating VM image (this may take a while)..."
    
    # Start with our development VM
    if ! VBoxManage list vms | grep -q "luna-development"; then
        echo "❌ Luna development VM not found. Please create it first:"
        echo "   cd ../luna-vm-dev && ./start-luna-development.sh"
        exit 1
    fi
    
    # Clone VM for distribution
    VBoxManage clonevm luna-development \
        --name "luna-distribution" \
        --options link \
        --register
    
    # Export to OVA
    VBoxManage export luna-distribution \
        --output "$BUILD_DIR/luna-base.ova"
    
    # Compress for embedding
    7z a -t7z -m0=lzma2 -mx=9 -mfb=64 -md=32m -ms=on \
        "$VM_ASSETS_DIR/luna-vm.7z" \
        "$BUILD_DIR/luna-base.ova"
    
    # Clean up temporary VM
    VBoxManage unregistervm luna-distribution --delete
    rm "$BUILD_DIR/luna-base.ova"
    
    echo "✅ VM image created: $(du -h "$VM_ASSETS_DIR/luna-vm.7z" | cut -f1)"
fi

# Step 2: Build native application
echo "🔨 Building native application..."
cd "$PROJECT_ROOT/native-app"

# Install dependencies
npm install

# Build TypeScript
npm run build

# Step 3: Package for all platforms
echo "📦 Packaging for distribution..."

# Build for Windows
echo "Building Windows installer..."
npm run dist -- --win --x64
mv dist/*.exe "$BUILD_DIR/Luna-Agent-Setup-Windows.exe" 2>/dev/null || true

# Build for macOS
if [ "$(uname)" = "Darwin" ]; then
    echo "Building macOS installer..."
    npm run dist -- --mac --x64 --arm64
    mv dist/*.dmg "$BUILD_DIR/Luna-Agent-Setup-macOS.dmg" 2>/dev/null || true
fi

# Build for Linux
echo "Building Linux AppImage..."
npm run dist -- --linux --x64
mv dist/*.AppImage "$BUILD_DIR/Luna-Agent-Linux.AppImage" 2>/dev/null || true

# Step 4: Create distribution info
echo "📄 Creating distribution info..."

cat > "$BUILD_DIR/README.txt" << 'DIST_EOF'
Luna Agent - One-Click Automation

INSTALLATION:
1. Download the installer for your platform:
   - Windows: Luna-Agent-Setup-Windows.exe
   - macOS: Luna-Agent-Setup-macOS.dmg  
   - Linux: Luna-Agent-Linux.AppImage

2. Run the installer (no configuration needed!)

3. Launch Luna Agent from your desktop

4. Start automating your tasks!

SYSTEM REQUIREMENTS:
- 4GB RAM minimum (8GB recommended)
- 2GB free disk space
- Hardware virtualization support
- Internet connection for updates

SUPPORT:
- Help Center: https://luna-agent.com/help
- Email: support@luna-agent.com

Luna Agent automatically manages all technical complexity.
Just click and start automating!
DIST_EOF

# Step 5: Generate checksums
echo "🔐 Generating checksums..."
cd "$BUILD_DIR"
find . -name "*.exe" -o -name "*.dmg" -o -name "*.AppImage" | while read file; do
    sha256sum "$file" > "$file.sha256"
done

# Step 6: Display build summary
echo ""
echo "✅ One-Click Luna Build Complete!"
echo "=================================="
echo ""
echo "📁 Distribution files:"
ls -lh "$BUILD_DIR"/*.exe "$BUILD_DIR"/*.dmg "$BUILD_DIR"/*.AppImage 2>/dev/null || true

echo ""
echo "📊 Build Summary:"
echo "- VM Image Size: $(du -h "$VM_ASSETS_DIR/luna-vm.7z" | cut -f1)"
echo "- Total Build Size: $(du -sh "$BUILD_DIR" | cut -f1)"
echo ""

echo "🚀 Ready for distribution!"
echo "Users can now download and run Luna with a single click!"
echo ""
echo "Next steps:"
echo "1. Test installers on clean systems"
echo "2. Upload to distribution platform"
echo "3. Update website with download links"
