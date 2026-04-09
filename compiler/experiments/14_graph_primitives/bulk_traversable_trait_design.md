# BulkTraversable Trait Design (No Generics)

## Overview

The `BulkTraversable` trait provides a unified interface for bulk operations (map, filter, reduce) across all types, without using generics. Each buffer type implements the trait for itself, allowing type-specific optimizations while maintaining a consistent API.

## Design Philosophy

**No Generics**: Silica doesn't support generics, so the trait system uses concrete types and trait implementations.

**Unified Interface**: All types use the same method names (`.bulk_map()`, `.bulk_filter()`, `.bulk_reduce()`), providing a consistent API.

**Type-Specific Optimizations**: Each implementation uses the best available optimization for that type:
- Numeric types: Full SIMD (4-16x speedup)
- Structs with numeric fields: Partial SIMD (2-4x speedup)
- Complex types: Vectorized memory access (1-2x speedup)
- Custom types: User-defined implementation

## Trait Definition

```silica
// Unified trait - each buffer type implements for itself
trait BulkTraversable {
    fn bulk_map(self: Self, f: FunctionType) -> proc[mem(normal)] Self;
    fn bulk_filter(self: Self, predicate: PredicateType) -> proc[mem(normal)] Self;
    fn bulk_reduce(self: Self, init: InitType, op: OpType) -> ResultType;
}

// Marker traits for optimization selection
trait SIMDProcessable {
    // Marker trait - no methods
    // Indicates type can use full SIMD operations (4-16x speedup)
}

trait PartiallySIMDProcessable {
    // Marker trait - no methods
    // Indicates type can use partial SIMD operations (2-4x speedup)
    // Typically for structs with numeric fields
}
```

## Built-In Implementations

### Numeric Types (Full SIMD)

```silica
// Marker trait implementations
impl SIMDProcessable for int8;
impl SIMDProcessable for int16;
impl SIMDProcessable for int32;
impl SIMDProcessable for int64;
impl SIMDProcessable for float32;
impl SIMDProcessable for float64;

// BulkTraversable implementations
impl BulkTraversable for buf(R, normal, int32, N) {
    fn bulk_map(self: buf(R, normal, int32, N), f: (int32) -> int32) 
        -> proc[mem(normal)] buf(R, normal, int32, N) {
        // Full SIMD implementation - 4-16x speedup
        simd_bulk_map_int32(self, f)
    }
    
    fn bulk_filter(self: buf(R, normal, int32, N), predicate: (int32) -> bool) 
        -> proc[mem(normal)] buf(R, normal, int32, N) {
        // SIMD filter with predicate masks - 3-16x speedup
        simd_bulk_filter_int32(self, predicate)
    }
    
    fn bulk_reduce(self: buf(R, normal, int32, N), init: int32, op: (int32, int32) -> int32) 
        -> int32 {
        // SIMD tree reduction - 4-16x speedup
        simd_bulk_reduce_int32(self, init, op)
    }
}

// Similar implementations for int8, int16, int64, float32, float64
```

### Structs with Numeric Fields (Partial SIMD)

```silica
type Point = { x: int32, y: int32, z: int32 }

impl PartiallySIMDProcessable for Point;

impl BulkTraversable for buf(R, normal, Point, N) {
    fn bulk_map(self: buf(R, normal, Point, N), f: (Point) -> Point) 
        -> proc[mem(normal)] buf(R, normal, Point, N) {
        // Option 1: Field-by-field SIMD (if operation is field-wise)
        // Extract x, y, z fields, SIMD process each, reconstruct
        // Performance: 2-4x speedup
        
        // Option 2: Vectorized memory access + scalar operations
        // Load 4 Points at once, apply function scalar, store
        // Performance: 1-2x speedup
        
        optimized_bulk_map_point(self, f)
    }
    
    fn bulk_filter(self: buf(R, normal, Point, N), predicate: (Point) -> bool) 
        -> proc[mem(normal)] buf(R, normal, Point, N) {
        // Vectorized memory access for loading Points
        // Scalar predicate evaluation
        // Efficient compaction
        optimized_bulk_filter_point(self, predicate)
    }
    
    fn bulk_reduce(self: buf(R, normal, Point, N), init: Point, op: (Point, Point) -> Point) 
        -> Point {
        // Scalar implementation with vectorized memory access
        scalar_bulk_reduce_point(self, init, op)
    }
}
```

### Tuples (Vectorized Memory Access)

```silica
type Pair = (int32, string)

impl BulkTraversable for buf(R, normal, Pair, N) {
    fn bulk_map(self: buf(R, normal, Pair, N), f: (Pair) -> Pair) 
        -> proc[mem(normal)] buf(R, normal, Pair, N) {
        // Vectorized memory loads/stores (4 elements at once)
        // Scalar operations (mixed types prevent SIMD)
        // Performance: 1-2x speedup (cache benefits)
        vectorized_memory_map(self, f)
    }
    
    fn bulk_filter(self: buf(R, normal, Pair, N), predicate: (Pair) -> bool) 
        -> proc[mem(normal)] buf(R, normal, Pair, N) {
        // Vectorized memory access
        // Scalar predicate evaluation
        vectorized_memory_filter(self, predicate)
    }
    
    fn bulk_reduce(self: buf(R, normal, Pair, N), init: Pair, op: (Pair, Pair) -> Pair) 
        -> Pair {
        // Scalar implementation
        scalar_bulk_reduce_pair(self, init, op)
    }
}
```

### Strings (Partial SIMD for Byte Operations)

```silica
impl BulkTraversable for buf(R, normal, string, N) {
    fn bulk_map(self: buf(R, normal, string, N), f: (string) -> string) 
        -> proc[mem(normal)] buf(R, normal, string, N) {
        // String operations are typically scalar
        // But can use SIMD for byte-level operations when applicable
        // Performance: 1-2x speedup (vectorized memory access)
        string_bulk_map(self, f)
    }
    
    fn bulk_filter(self: buf(R, normal, string, N), predicate: (string) -> bool) 
        -> proc[mem(normal)] buf(R, normal, string, N) {
        // Can use SIMD for byte comparison in predicates
        // Performance: 2-8x speedup for byte operations
        string_bulk_filter(self, predicate)
    }
    
    fn bulk_reduce(self: buf(R, normal, string, N), init: string, op: (string, string) -> string) 
        -> string {
        // Scalar implementation
        string_bulk_reduce(self, init, op)
    }
}
```

## User-Defined Type Implementations

Programmers can implement `BulkTraversable` for their custom types:

```silica
type MyCustomType = {
    id: int32,
    value: float32,
    metadata: string
}

// User chooses optimization strategy
impl BulkTraversable for buf(R, normal, MyCustomType, N) {
    fn bulk_map(self: buf(R, normal, MyCustomType, N), f: (MyCustomType) -> MyCustomType) 
        -> proc[mem(normal)] buf(R, normal, MyCustomType, N) {
        // Option 1: Partial SIMD for numeric fields
        // Extract id and value fields, SIMD process, reconstruct
        // Performance: 2-4x speedup for numeric operations
        
        // Option 2: Vectorized memory access
        // Load 4 MyCustomType at once, apply function scalar
        // Performance: 1-2x speedup
        
        // Option 3: Pure scalar
        // Performance: 1x (baseline)
        
        // User implements based on their needs
        custom_bulk_map(self, f)
    }
    
    fn bulk_filter(self: buf(R, normal, MyCustomType, N), predicate: (MyCustomType) -> bool) 
        -> proc[mem(normal)] buf(R, normal, MyCustomType, N) {
        // User's implementation
        custom_bulk_filter(self, predicate)
    }
    
    fn bulk_reduce(self: buf(R, normal, MyCustomType, N), init: MyCustomType, op: (MyCustomType, MyCustomType) -> MyCustomType) 
        -> MyCustomType {
        // User's implementation
        custom_bulk_reduce(self, init, op)
    }
}
```

## Performance Tiers

| Type Category | Trait Implementation | SIMD Support | Expected Speedup |
|---------------|---------------------|--------------|------------------|
| **Numeric** (int32, float32) | Built-in | Full SIMD | 4-16x |
| **Packed Structs** (all numeric) | Built-in | Partial SIMD | 2-4x |
| **Structs** (mixed fields) | Built-in | Vectorized memory | 1-2x |
| **Tuples** | Built-in | Vectorized memory | 1-2x |
| **Strings** (byte ops) | Built-in | Partial SIMD | 2-8x |
| **Strings** (semantic ops) | Built-in | Scalar | 1x |
| **Custom Types** | User-defined | User's choice | Varies |

## Helper Functions

Common optimization patterns are provided as helper functions:

```silica
// Full SIMD implementations
fn simd_bulk_map_int32(buf: buf(R, normal, int32, N), f: (int32) -> int32) 
    -> proc[mem(normal)] buf(R, normal, int32, N)

// Partial SIMD for structs
fn optimized_bulk_map_point(buf: buf(R, normal, Point, N), f: (Point) -> Point) 
    -> proc[mem(normal)] buf(R, normal, Point, N)

// Vectorized memory access
fn vectorized_memory_map<T>(buf: buf(R, normal, T, N), f: (T) -> T) 
    -> proc[mem(normal)] buf(R, normal, T, N)

// Scalar fallback
fn scalar_bulk_map<T>(buf: buf(R, normal, T, N), f: (T) -> T) 
    -> proc[mem(normal)] buf(R, normal, T, N)
```

## Benefits

### 1. Unified Interface
- All types use `.bulk_map()`, `.bulk_filter()`, `.bulk_reduce()`
- Consistent API regardless of type
- Easy to learn and use

### 2. Type Safety
- Each implementation is type-specific
- Compiler enforces correct function signatures
- No runtime type errors

### 3. Performance Tiers
- Each implementation uses best available optimization
- Graceful performance degradation for complex types
- Users can optimize their custom types

### 4. Extensibility
- Users can implement for their types
- Choose appropriate optimization level
- Full control over performance vs complexity trade-off

### 5. No Generics
- Works with Silica's trait system
- No generic syntax needed
- Explicit, clear implementations

## Usage Examples

### Numeric Types (Full SIMD)

```silica
let numbers: buf(R, normal, int32, 1000) <- alloc_buf_int32(1000);

// Full SIMD - 4-16x speedup
let doubled <- numbers.bulk_map((x) -> x * 2);
let filtered <- numbers.bulk_filter((x) -> x > 100);
let sum <- numbers.bulk_reduce(0, (acc, x) -> acc + x);
```

### Struct Types (Partial SIMD)

```silica
let points: buf(R, normal, Point, 1000) <- alloc_buf_point(1000);

// Partial SIMD or optimized scalar - 2-4x speedup
let scaled <- points.bulk_map((p) -> Point { x: p.x * 2, y: p.y * 2, z: p.z * 2 });
let filtered <- points.bulk_filter((p) -> p.x > 0);
```

### Custom Types (User-Defined)

```silica
let custom: buf(R, normal, MyCustomType, 1000) <- alloc_buf_custom(1000);

// User's implementation - performance depends on implementation
let mapped <- custom.bulk_map((c) -> MyCustomType { ... });
```

## Conclusion

The `BulkTraversable` trait provides a unified, type-safe interface for bulk operations across all types, without requiring generics. Each type implements the trait with appropriate optimizations, providing:

- **Maximum performance** for numeric types (4-16x)
- **Good performance** for structs with numeric fields (2-4x)
- **Reasonable performance** for complex types (1-2x)
- **Extensibility** for user-defined types

This design aligns perfectly with Silica's trait-based, no-generics philosophy while delivering significant performance gains where hardware supports it.
