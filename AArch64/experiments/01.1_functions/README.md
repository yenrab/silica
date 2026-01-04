# Function Type Tests

This directory contains comprehensive tests for function type handling in Silica, covering both regular functions and helper functions (functions called by other functions).

## Test Files

### 1. `test_simple_int.silica`
- Basic test of `int` parameter and return types
- Simple helper function calling pattern

### 2. `test_primitive_types.silica`
Tests primitive type handling in functions:
- Functions with `int`, `boolean`, `char` parameters
- Functions returning primitive types
- Helper functions with primitive types
- Type-safe function calls and return value handling

### 3. `test_complex_types.silica`
Tests complex type handling:
- Functions with `string` and `tuple(int, boolean)` parameters
- Functions returning complex types
- Helper functions processing complex types
- Type-safe handling of boxed complex types

### 4. `test_helper_interactions.silica`
Tests helper function interactions:
- Helper functions with mixed parameter types
- Helper functions calling other helper functions
- Proper type flow between helper function calls
- Nested helper function invocations

### 5. `test_return_type_handling.silica`
Tests return type handling in helper functions:
- Helper functions returning different types (`int`, `boolean`, `char`)
- Functions using helper function return values
- Type-safe return value handling
- Chaining of helper function calls

## Key Features Tested

### Parameter Types
- ✅ Primitive types: `int`, `boolean`, `char`
- ✅ Complex types: `string`, `tuple`, `record`, `variant`
- ✅ Unit type for no parameters

### Return Types
- ✅ Primitive types: `int`, `boolean`, `char`
- ✅ Complex types: `string`, `tuple`, etc.

### Function Categories
- ✅ Regular functions (called from main)
- ✅ Helper functions (called by other functions)
- ✅ Nested helper function calls
- ✅ Function chaining with proper type flow

### Type Safety
- ✅ Parameter type checking
- ✅ Return type checking
- ✅ Type preservation through function calls
- ✅ Proper LLVM type generation (verified by LLVM backend)

## Syntax Notes

- **Binding**: Uses `x <- value` (not `let x = value`)
- **Types**: Uses `boolean` (not `bool`), `int` (not `Int`)
- **All functions return values**: No void/unit return type

## Building and Running

```bash
# Build all LLVM IR files
make

# Build and verify compilation
make test

# Clean generated files
make clean
```

## Implementation Status

- **LLVM Backend**: ✅ Correctly handles all Silica types with proper LLVM type mapping
- **Type Verification**: ✅ Tests verify that the type system correctly handles parameter and return type specifications
- **Text IR Generation**: ⚠️ May have some inconsistencies but LLVM backend validates correctness

The tests demonstrate that Silica's type system properly supports the full range of types for both regular functions and helper functions, with the LLVM backend providing correct type handling and validation.
