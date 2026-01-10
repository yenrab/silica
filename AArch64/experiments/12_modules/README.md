# Module Import Experiments

This directory tests Silica's module system - importing and using code from other modules. **Important**: Modules without main functions are intended to be statically linked into the applications that use them.

## Module System Design Principles

### Static Linking Model
- **Modules without `main`**: Treated as libraries that get compiled into the importing application
- **Single compilation unit**: When compiling `app.silica`, all imported modules (`calculator`, `string_utils`) are statically linked into the final executable
- **No separate module binaries**: Unlike traditional separate compilation, Silica modules are inlined into their users

### Module Resolution
- **File-based modules**: `filename.silica` automatically creates module `filename`
- **Import syntax**: `use module_name;` makes all exported functions available in current scope
- **Transitive dependencies**: If A imports B, and B imports C, A's compilation includes both B and C

## Test Files

### `math_utils.silica` (Library Module)
- **Purpose**: Basic mathematical utilities
- **Exports**: `add/2`, `multiply/2`, `square/1`
- **Usage**: Statically linked into `main.silica`, `calculator.silica`, and `app.silica`

### `string_utils.silica` (Library Module)
- **Purpose**: String manipulation utilities (placeholder implementation)
- **Exports**: `concat/2`, `length/1`
- **Usage**: Statically linked into `main.silica` and `app.silica`

### `calculator.silica` (Library Module)
- **Purpose**: Calculator that uses math utilities
- **Imports**: `math_utils` (statically linked)
- **Exports**: `calculate_expression/2`, `evaluate/1`
- **Usage**: Statically linked into `app.silica`

### `main.silica` (Executable)
- **Purpose**: Simple application using multiple modules
- **Imports**: `math_utils`, `string_utils` (both statically linked)
- **Contains**: `main()` function - this is an executable

### `app.silica` (Executable)
- **Purpose**: Complex application with multi-level dependencies
- **Imports**: `calculator`, `string_utils` (both statically linked)
- **Transitive**: Gets `math_utils` through `calculator`
- **Contains**: `main()` function - this is an executable

## Expected Behavior

1. **Static Linking**: When compiling `main.silica`, `math_utils.silica` and `string_utils.silica` are compiled directly into the `main` executable
2. **Transitive Dependencies**: When compiling `app.silica`, it includes `calculator.silica`, `string_utils.silica`, and `math_utils.silica` (through calculator)
3. **Single Executable**: Each file with `main()` produces one self-contained executable with all dependencies statically linked
4. **Export Validation**: All exported functions must exist with correct arity

## Compilation (Current Status)

```bash
# Individual modules compile successfully (bootstrap compiler can parse imports)
silica-boot math_utils.silica math_utils.ll -I ../stdlib
silica-boot calculator.silica calculator.ll -I ../stdlib  # Shows module loading

# Full static linking not yet implemented in bootstrap compiler
# Future: silica-boot app.silica app.exe  # Would include all dependencies
```

**Status**: Multi-module compilation has been implemented in the compiler. When built, it will automatically include all imported module code in the final executable.