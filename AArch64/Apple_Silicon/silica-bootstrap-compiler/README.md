# Silica Bootstrap Compiler

This is the bootstrap compiler for the Silica programming language, written in Rust.

## Building

Use the provided build script for the easiest experience:

```bash
./build_bootstrap.sh
```

This script will:
- Automatically detect if LLVM 15 is available
- Build with LLVM backend support if LLVM is found
- Fall back to text-only LLVM IR generation if LLVM is not available
- Provide helpful status messages throughout the build process

### Manual Build

If you prefer to build manually:

```bash
# Build without LLVM backend (text IR only)
cargo build --release

# Build with LLVM backend (requires LLVM 15)
export LLVM_SYS_150_PREFIX=/path/to/llvm-15
cargo build --release --features llvm_backend
```

## Usage

### Setup Environment

If you have LLVM 15 installed, source the setup script:

```bash
# From the Silica Language root directory
source setup_silica.sh
```

### Compile Silica Programs

```bash
# Compile a Silica program (outputs LLVM bitcode)
./target/release/silica-boot program.silica

# Specify output file (.bc for bitcode, .ll for text)
./target/release/silica-boot program.silica output.bc
./target/release/silica-boot program.silica output.ll

# Compile with optimization
./target/release/silica-boot program.silica --opt aggressive
```

### Run Compiled Programs

```bash
# Execute LLVM bitcode (preferred)
lli output.bc

# Or execute LLVM text IR
lli output.ll

# Check the exit code (program return value)
echo "Exit code: $?"
```

## LLVM Backend Support

The compiler supports two modes:

### Text IR Mode (Default)
- Generates human-readable LLVM IR text files (`.ll`)
- Works without LLVM installation
- Can be executed directly with `lli`

### Binary Backend Mode (With LLVM 15)
- Generates actual LLVM bitcode (`.bc`)
- Requires LLVM 15 to be installed
- Better performance and full LLVM integration

## Project Structure

- `src/main.rs` - Command-line interface
- `src/lib.rs` - Main compiler library
- `src/lexer.rs` - Lexical analysis
- `src/parser.rs` - Syntax parsing
- `src/ast.rs` - Abstract Syntax Tree definitions
- `src/types.rs` - Type checking and inference
- `src/codegen.rs` - LLVM code generation
- `src/runtime.rs` - Runtime system interfaces
- `src/module_resolver.rs` - Module loading and resolution

## Development

### Adding New Features

1. Update the AST in `src/ast.rs`
2. Add parsing logic in `src/parser.rs`
3. Implement type checking in `src/types.rs`
4. Add code generation in `src/codegen.rs`

### Testing

Compile and run test programs from the `experiments/` directory:

```bash
./target/release/silica-boot experiments/hello.silica
lli output.ll
```

## Dependencies

- **Required**: Rust 1.70+
- **Optional**: LLVM 15 (for binary backend)
- **Build Tools**: Standard Rust toolchain (cargo, rustc)

## Troubleshooting

### LLVM Not Found
If the build script can't find LLVM 15, it will automatically fall back to text-only mode. To enable full LLVM support:

1. Install LLVM 15 in a standard location, or
2. Set `LLVM_SYS_150_PREFIX` to your LLVM installation path

### Build Failures
- Ensure you have the latest Rust stable
- Try `cargo clean` then rebuild
- Check that all dependencies are available

### Runtime Issues
- Make sure `lli` is in your PATH when running compiled programs
- For macOS, you may need to adjust library paths if using the LLVM backend

## License

See the main Silica project for licensing information.
