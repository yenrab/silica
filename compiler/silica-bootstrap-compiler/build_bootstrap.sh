#!/bin/bash
# Silica Bootstrap Compiler Build Script
# This script builds the Silica bootstrap compiler with proper LLVM configuration

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "src" ]; then
    print_error "Please run this script from the silica-bootstrap-compiler directory"
    exit 1
fi

print_status "Starting Silica Bootstrap Compiler build..."

# Check if LLVM 15 is available
print_status "Checking for LLVM 15..."

# Try to find LLVM 15 in common locations
LLVM_PATHS=(
    "/usr/local/llvm-15"
    "/opt/homebrew/opt/llvm@15"
    "/usr/local/opt/llvm@15"
    "$HOME/llvm-15"
    "$HOME/1TB/llvm-15"  # User's specific location
)

LLVM_FOUND=false
for path in "${LLVM_PATHS[@]}"; do
    if [ -d "$path" ] && [ -x "$path/bin/llvm-config" ]; then
        LLVM_PREFIX="$path"
        LLVM_FOUND=true
        print_success "Found LLVM 15 at: $LLVM_PREFIX"
        break
    fi
done

if [ "$LLVM_FOUND" = false ]; then
    print_warning "LLVM 15 not found in standard locations."
    print_warning "The compiler will be built without LLVM backend support."
    print_warning "You can still compile Silica programs to LLVM IR text."
    BUILD_LLVM=false
else
    print_success "LLVM 15 found. Will build with LLVM backend support."
    BUILD_LLVM=true
fi

# Function to build with LLVM support
build_with_llvm() {
    print_status "Building with LLVM backend support..."
    export LLVM_SYS_150_PREFIX="$LLVM_PREFIX"
    export PATH="$LLVM_PREFIX/bin:$PATH"

    print_status "LLVM_SYS_150_PREFIX=$LLVM_SYS_150_PREFIX"
    print_status "PATH includes: $LLVM_PREFIX/bin"

    if cargo build --release --features llvm_backend; then
        print_success "Compiler built successfully with LLVM backend!"
        print_status "You can now compile Silica programs and execute them with:"
        print_status "  ./target/release/silica-boot program.silica"
        print_status "  lli output.ll  # Execute the generated LLVM IR"
        return 0
    else
        print_warning "LLVM backend build failed. Falling back to text-only build..."
        return 1
    fi
}

# Function to build without LLVM support
build_without_llvm() {
    print_status "Building without LLVM backend (text IR only)..."

    if cargo build --release; then
        print_success "Compiler built successfully (text IR only)!"
        print_status "You can compile Silica programs to LLVM IR text:"
        print_status "  ./target/release/silica-boot program.silica"
        print_status "  lli output.ll  # Execute the generated LLVM IR"
        print_warning "Note: Binary bitcode generation requires LLVM backend."
        return 0
    else
        print_error "Build failed!"
        return 1
    fi
}

# Main build logic
if [ "$BUILD_LLVM" = true ]; then
    if build_with_llvm; then
        exit 0
    else
        # LLVM build failed, try text-only build
        build_without_llvm
    fi
else
    build_without_llvm
fi

# Post-build checks
if [ -f "target/release/silica-boot" ]; then
    print_success "Build completed successfully!"
    print_status "Binary location: $(pwd)/target/release/silica-boot"

    # Show version info
    print_status "Testing compiler..."
    if ./target/release/silica-boot --help 2>/dev/null || ./target/release/silica-boot 2>&1 | head -5; then
        print_success "Compiler is working!"
    fi
else
    print_error "Build failed - binary not found"
    exit 1
fi

print_success "Silica Bootstrap Compiler build script completed!"
print_status "To use the compiler:"
echo "  export PATH=\"\$HOME/1TB/llvm-15/bin:\$PATH\"  # If using LLVM backend"
echo "  ./target/release/silica-boot program.silica    # Compile"
echo "  lli output.ll                                 # Run"
