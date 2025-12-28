#!/bin/bash
# Setup script for Silica development with LLVM 15

export SILICA_LLVM_PATH="/Users/leebarney/1TB/llvm-15/bin"
export PATH="$SILICA_LLVM_PATH:$PATH"

echo "Silica environment ready!"
echo "LLVM 15 path: $SILICA_LLVM_PATH"
echo "Usage:"
echo "  Compile: ./silica-boot program.silica [output.bc]"
echo "  Run:     lli output.bc  # or lli output.ll"
echo ""
lli --version
