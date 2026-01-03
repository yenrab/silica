# Silica Experiments & Tests

This directory contains all development test files and experiments for the Silica programming language, organized by feature area and testing priority.

## Directory Structure & Testing Order

The subdirectories are numbered and ordered to ensure comprehensive testing - run tests in numerical order to verify all language features work correctly.

### 01_basic/
**Language Fundamentals** - Test the most basic Silica functionality first
- `hello.silica` - Basic hello world
- `simple_test.silica` - Simple expressions and functions
- `main.silica` - Entry point testing
- `types.silica` - Basic type system

### 02_expressions/
**Expressions & Control Flow** - Test operators, assignments, and control structures
- `test_if.silica` - Conditional expressions
- `test_assignment.silica` - Variable assignment
- `control_flow.silica` - Control flow constructs

### 03_data_types/
**Data Types** - Test all data type constructs (tuples, structs, enums)
- Tuple operations and decomposition
- Struct definitions and field access
- Sum types and pattern matching
- Type aliases

### 04_patterns/
**Pattern Matching** - Test case expressions and pattern matching
- Basic case expressions
- Pattern guards
- Complex pattern matching
- Exhaustiveness checking

### 05_memory/
**Memory Management** - Test region-based memory system
- Region creation and destruction
- Memory allocation and deallocation
- Reference operations (read/write)
- Memory safety guarantees
- **NEW:** Advanced memory scenarios (nested regions, lifecycle management)

### 06_advanced_features/
**Advanced Language Features** - Test generics, traits, and polymorphism
- Generic functions and types
- Trait definitions and implementations
- Type inference with generics
- Method resolution
- **NEW:** Complex generic scenarios (multiple parameters, higher-order generics)

### 07_concurrency/
**Concurrency & Actors** - Test actor-based concurrency
- Actor creation and spawning
- Message passing (send/receive)
- Actor mailboxes
- Concurrent programming patterns
- **NEW:** Advanced actor communication and lifecycle management

### 08_stdlib/
**Standard Library** - Test stdlib integration and built-in functions
- Standard library imports
- File I/O operations
- Built-in function usage
- **NEW:** Process execution testing
- **NEW:** Comprehensive stdlib integration testing

### 09_binding/
**Binding & Sequencing** - Test variable binding and control flow
- `<-` binding syntax in do expressions
- Variable scoping and shadowing
- Sequential execution (`do` blocks)

### 10_experimental/
**Experimental Features** - Advanced/experimental language features
- Effect systems
- Advanced data structures
- Functional programming constructs
- Research features
- **NEW:** Error handling edge cases and boundary conditions
- **NEW:** Full integration tests combining all language features

## Testing Strategy

1. **Run tests in order** (01 → 02 → 03 → ...)
2. **Start with basic functionality** before testing complex features
3. **Each directory** represents a layer of language complexity
4. **Verify dependencies** - later tests depend on earlier functionality working

## Usage

### Option 1: Manual Compilation

```bash
# Test basic functionality first
silica-boot experiments/01_basic/hello.silica

# Then expressions
silica-boot experiments/02_expressions/test_if.silica

# Continue through all directories in order
```

### Option 2: Automated Makefile Build

```bash
# Build all experiments in order (recommended)
make all

# Build only basic functionality for quick testing
make test-basic

# Build only executables across all directories
make executables

# Clean all generated files
make clean

# Show build status
make status
```

### Individual Directory Builds

Each subdirectory has its own Makefile:

```bash
# Build all files in a specific directory
cd experiments/01_basic && make all

# Build only executables in that directory
cd experiments/03_data_types && make executables

# Build only modules in that directory
cd experiments/03_data_types && make modules
```

## Adding New Tests

- Place new tests in the appropriate numbered directory
- Follow existing naming conventions
- Ensure tests can run independently
- Update this README if adding new directories
