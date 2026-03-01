#!/bin/bash
# Debug script: compile comparator_ops with debug comments, show debug output, assemble, link, run
# Run from AArch64/silica-compiler/ directory

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/int16_addition"

# Backup and use comparator_ops-only config
cp silica.config silica.config.bak 2>/dev/null || true
cp silica.config.comparator_ops_only silica.config

# Compile - silica-compiler reads silica.config from cwd
COMPILER_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SILICA_COMPILER="${SILICA_COMPILER:-$COMPILER_ROOT/src/silica-compiler}"
if [ ! -x "$SILICA_COMPILER" ]; then
    echo "Error: silica-compiler not found at $SILICA_COMPILER"
    echo "Build the compiler first, then run: ./trials/debug_comparator_ops.sh"
    exit 1
fi

"$SILICA_COMPILER"

echo ""
echo "=== DEBUG output (prim_name from emitter) ==="
grep "DEBUG prim_name" comparator_ops.sams || echo "(no DEBUG lines found)"

echo ""
echo "=== Assembling and linking ==="
MACOS_SDK=$(xcrun --sdk macosx --show-sdk-path 2>/dev/null || echo "")
clang -mmacosx-version-min=26.0 -c -x assembler comparator_ops.sams -o comparator_ops.o
RUST_LLD=$(rustc --print sysroot 2>/dev/null)/lib/rustlib/$(rustc -vV 2>/dev/null | sed -n 's/^host: *//p')/bin/rust-lld
if [ -x "$RUST_LLD" ] && [ -n "$MACOS_SDK" ]; then
    $RUST_LLD -flavor darwin -arch arm64 -platform_version macos 26.0 26.0 -syslibroot "$MACOS_SDK" -lSystem -e main -o comparator_ops comparator_ops.o
else
    clang comparator_ops.o -Wl,-e,main -Wl,-macos_version_min,26.0 -o comparator_ops
fi

echo ""
echo "=== Running comparator_ops ==="
./comparator_ops 2>&1; echo "Exit: $?"

# Restore config
mv silica.config.bak silica.config 2>/dev/null || true
