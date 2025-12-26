#!/bin/bash

# Silica Bootstrap Compiler - LLVM Setup and Build Script
# This script sets up LLVM at /opt/homebrew/opt/llvm@15 (for inkwell compatibility) and builds the Silica compiler

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_header() {
    echo -e "${BLUE}[SETUP]${NC} $1"
}

# Check if running on macOS
check_macos() {
    if [[ "$OSTYPE" != "darwin"* ]]; then
        print_error "This script is designed for macOS. Current OS: $OSTYPE"
        exit 1
    fi
}

# Check if LLVM is installed at the expected location
check_llvm_installation() {
    # Try multiple possible LLVM locations (prioritizing inkwell-compatible versions)
    if [[ -d "/opt/homebrew/opt/llvm@15" ]]; then
        LLVM_PATH="/opt/homebrew/opt/llvm@15"
    elif [[ -d "/opt/homebrew/opt/llvm@16" ]]; then
        LLVM_PATH="/opt/homebrew/opt/llvm@16"
    elif [[ -d "/opt/homebrew/opt/llvm" ]]; then
        LLVM_PATH="/opt/homebrew/opt/llvm"
    elif [[ -d "/opt/homebrew/opt/llvm@21" ]]; then
        LLVM_PATH="/opt/homebrew/opt/llvm@21"
    elif [[ -d "/opt/homebrew/opt/llvm@20" ]]; then
        LLVM_PATH="/opt/homebrew/opt/llvm@20"
    elif [[ -d "/opt/homebrew/opt/llvm@19" ]]; then
        LLVM_PATH="/opt/homebrew/opt/llvm@19"
    elif [[ -d "/usr/local/opt/llvm" ]]; then
        LLVM_PATH="/usr/local/opt/llvm"
    else
        print_error "LLVM not found in common locations:"
        print_error "  - /opt/homebrew/opt/llvm"
        print_error "  - /opt/homebrew/opt/llvm@21"
        print_error "  - /opt/homebrew/opt/llvm@20"
        print_error "  - /opt/homebrew/opt/llvm@19"
        print_error "  - /usr/local/opt/llvm"
        print_error ""
        print_error "Please install LLVM with: brew install llvm"
        exit 1
    fi

    if [[ ! -f "$LLVM_PATH/bin/llvm-config" ]]; then
        print_error "llvm-config not found at $LLVM_PATH/bin/llvm-config"
        print_error "LLVM installation appears incomplete"
        exit 1
    fi

    print_status "Found LLVM installation at $LLVM_PATH"
}

# Verify LLVM version compatibility
check_llvm_version() {
    LLVM_CONFIG="$LLVM_PATH/bin/llvm-config"
    LLVM_VERSION=$($LLVM_CONFIG --version 2>/dev/null | cut -d. -f1)

    if [[ -z "$LLVM_VERSION" ]]; then
        print_error "Could not determine LLVM version"
        exit 1
    fi

    if [[ "$LLVM_VERSION" -lt 15 ]]; then
        print_warning "LLVM version $LLVM_VERSION found, but version 15+ is recommended"
        print_warning "Some features may not work correctly"
    elif [[ "$LLVM_VERSION" -gt 21 ]]; then
        print_warning "LLVM version $LLVM_VERSION is newer than tested (21)"
        print_warning "Build may succeed or may require updated Cargo.toml dependencies"
    else
        print_status "LLVM version $LLVM_VERSION is compatible"
    fi
}

# Set up environment variables
setup_environment() {
    print_header "Setting up environment variables..."

    export LLVM_SYS_150_PREFIX="$LLVM_PATH"
    export PATH="$LLVM_PATH/bin:$PATH"

    # Verify environment variables
    if [[ -z "$LLVM_SYS_150_PREFIX" ]]; then
        print_error "Failed to set LLVM_SYS_150_PREFIX"
        exit 1
    fi

    print_status "LLVM_SYS_150_PREFIX set to: $LLVM_SYS_150_PREFIX"
    print_status "Added LLVM to PATH: $LLVM_PATH/bin"
}

# Test LLVM installation
test_llvm() {
    print_header "Testing LLVM installation..."

    # Test llvm-config
    if ! command -v llvm-config &> /dev/null; then
        print_error "llvm-config not found in PATH"
        exit 1
    fi

    LLVM_VER=$(llvm-config --version)
    print_status "llvm-config version: $LLVM_VER"

    # Test clang
    if ! command -v clang &> /dev/null; then
        print_error "clang not found in PATH"
        exit 1
    fi

    CLANG_VER=$(clang --version | head -n1)
    print_status "clang version: $CLANG_VER"

    # Test lli
    if ! command -v lli &> /dev/null; then
        print_error "lli not found in PATH"
        exit 1
    fi

    LLI_VER=$(lli --version | head -n1)
    print_status "lli version: $LLI_VER"

    # Check for required libraries
    if [[ ! -f "$LLVM_PATH/lib/libLLVM.dylib" ]]; then
        print_error "LLVM library not found at $LLVM_PATH/lib/libLLVM.dylib"
        exit 1
    fi

    print_status "All LLVM components found and working"
}

# Clean previous build artifacts
clean_build() {
    print_header "Cleaning previous build artifacts..."

    if [[ -d "target" ]]; then
        cargo clean
        print_status "Cleaned Cargo build artifacts"
    else
        print_status "No previous build artifacts to clean"
    fi
}

# Build the Silica Bootstrap Compiler
build_compiler() {
    print_header "Building Silica Bootstrap Compiler with LLVM backend..."

    # Build with LLVM backend
    print_status "Running: cargo build --release --features llvm_backend"
    cargo build --release --features llvm_backend

    if [[ $? -eq 0 ]]; then
        print_status "✅ Silica Bootstrap Compiler built successfully!"

        # Show binary location
        BINARY_PATH="target/release/silica-boot"
        if [[ -f "$BINARY_PATH" ]]; then
            print_status "Binary location: $(pwd)/$BINARY_PATH"
            print_status "Binary size: $(ls -lh "$BINARY_PATH" | awk '{print $5}')"
        fi
    else
        print_error "❌ Build failed!"
        exit 1
    fi
}

# Test the compiler build
test_compiler() {
    print_header "Testing Silica Bootstrap Compiler..."

    BINARY_PATH="target/release/silica-boot"

    if [[ ! -f "$BINARY_PATH" ]]; then
        print_error "Compiler binary not found at $BINARY_PATH"
        exit 1
    fi

    # Test basic help/version
    print_status "Testing compiler execution..."
    if "$BINARY_PATH" --help &>/dev/null; then
        print_status "✅ Compiler executes successfully"
    else
        print_warning "⚠️  Compiler execution test failed, but binary exists"
    fi

    # Test with a simple Silica file if it exists
    TEST_FILE="../../experiments/hello.silica"
    if [[ -f "$TEST_FILE" ]]; then
        print_status "Testing compilation of hello.silica..."
        if "$BINARY_PATH" "$TEST_FILE" test_output.bc 2>/dev/null; then
            print_status "✅ Successfully compiled test Silica file"
            if [[ -f "test_output.bc" ]]; then
                rm test_output.bc
            fi
        else
            print_warning "⚠️  Test compilation failed, but compiler built successfully"
        fi
    fi
}

# Display usage information
show_usage() {
    print_header "Silica Bootstrap Compiler - LLVM Setup Complete!"
    echo ""
    echo "Usage examples:"
    echo "  ./silica-boot input.silica output.bc                    # Basic compilation"
    echo "  ./silica-boot --opt standard input.silica output.bc     # With optimizations"
    echo "  ./silica-boot --opt aggressive input.silica output.bc   # Maximum optimization"
    echo ""
    echo "To run compiled LLVM bitcode:"
    echo "  lli output.bc"
    echo ""
    echo "Environment variables set for future sessions:"
    echo "  export LLVM_SYS_150_PREFIX=\"$LLVM_PATH\""
    echo "  export PATH=\"$LLVM_PATH/bin:\$PATH\""
}

# Main execution
main() {
    print_header "Silica Bootstrap Compiler - LLVM Setup Script"
    echo ""

    check_macos
    check_llvm_installation
    check_llvm_version
    setup_environment
    test_llvm
    clean_build
    build_compiler
    test_compiler

    echo ""
    show_usage

    print_status "🎉 Setup complete! You can now build and use the Silica Bootstrap Compiler."
}

# Run main function
main "$@"
