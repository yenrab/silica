# Graph Primitives: Bootstrap Compiler Code Generation Plan

## Overview

This document outlines the **additional implementation work** required beyond AST generation to compile graph primitives from Silica source code to runnable AArch64 binaries. This work enables the bootstrap compiler to generate executable code for graph primitives, allowing the Phase 2 compiler (written in Silica) to use graph primitives.

## Dependencies

**⚠️ CRITICAL DEPENDENCIES**:
1. **AST Plan**: `bootstrap_compiler_ast_plan.md` must be completed first
2. **Safe Chip Features Codegen Plan**: `../15_safe_chip_features/bootstrap_compiler_codegen_plan.md` must be completed first (graph primitives depend on SIMD operations)

Graph primitives require:
- SIMD vector types (`Vec128Int32`, `VecInt32`, `Pred`) to be type-checkable and code-generatable
- SIMD function calls (`load_128_int32()`, `simd_bulk_map_int32()`, etc.) to be callable and code-generatable
- Marker traits (`SIMDProcessable`, `PartiallySIMDProcessable`) to be resolvable during type checking
- Safe memory types (optional) to be available for graph structures

## Goal

Enable the bootstrap compiler to:
1. Type-check graph-related code (traits, types, functions)
2. Generate AArch64 assembly for graph operations
3. Generate AArch64 assembly for bulk operations (map, filter, reduce)
4. Generate AArch64 assembly for graph construction (builder pattern)
5. Link graph primitives into runnable binaries

## Phase 1: Type Checking Extensions

### 1.1 Trait Resolution

**Required**: Extend type checker to resolve trait relationships for graph traits

**Tasks**:
- Resolve trait inclusion relationships (`trait X includes Y, Z`)
- Build trait inclusion graph for graph traits (Graph, BulkTraversable, DenseGraph, etc.)
- Verify trait method implementations match trait declarations
- Check trait bounds in function signatures

**Deliverables**:
- Type checker can resolve `Graph` trait and all sub-traits
- Type checker can verify trait implementations
- Type checker can check trait bounds in function parameters

### 1.2 Graph Type Checking

**Required**: Extend type checker to validate graph type definitions

**Tasks**:
- Type-check graph structure types (DenseGraphStructure, SparseGraphStructure)
- Validate buffer types in graph structures (`buf(R, normal, NodeData, N)`)
- Type-check graph builder types
- Validate process types in graph operations (`proc[mem(normal)] Graph`)

**Deliverables**:
- Type checker can validate graph type definitions
- Type checker can check buffer types in graph structures
- Type checker can validate process types in graph operations

### 1.3 BulkTraversable Type Checking

**Required**: Extend type checker to validate BulkTraversable trait implementations

**Tasks**:
- Type-check `BulkTraversable` trait implementations for buffer types
- Validate function types in bulk operation parameters (`(T) -> T`, `(T) -> bool`)
- Check marker trait implementations (`SIMDProcessable`, `PartiallySIMDProcessable`)
- Verify bulk operation return types match trait declarations

**Deliverables**:
- Type checker can validate BulkTraversable implementations
- Type checker can check function types in bulk operations
- Type checker can resolve marker traits

### 1.4 Graph Builder Type Checking

**Required**: Extend type checker to validate graph builder pattern

**Tasks**:
- Type-check builder trait methods (`add_node`, `add_edge`, `build`)
- Validate builder construction process (mutable during construction)
- Check builder return types (process types returning graphs)
- Verify builder-to-graph conversion

**Deliverables**:
- Type checker can validate graph builder usage
- Type checker can check builder method calls
- Type checker can verify builder-to-graph conversion

## Phase 2: Code Generation - Graph Structures

### 2.1 Graph Structure Layout Generation

**Required**: Generate AArch64 memory layout for graph structures

**Tasks**:
- Generate memory layout for `DenseGraphStructure` (node_data, edge_from, edge_to buffers)
- Generate memory layout for `SparseGraphStructure` (nodes, edge_batches buffers)
- Generate alignment requirements (16-byte alignment for SIMD)
- Generate structure field offsets

**Deliverables**:
- Code generator can emit AArch64 data structures for graph types
- Code generator can generate aligned memory layouts
- Code generator can compute field offsets

### 2.2 Graph Allocation Code Generation

**Required**: Generate AArch64 code for graph structure allocation

**Tasks**:
- Generate allocation code for graph structures (using region allocators)
- Generate buffer allocation within graph structures
- Generate initialization code for graph metadata (node_count, edge_count)
- Generate SIMD alignment for graph buffers

**Deliverables**:
- Code generator can emit allocation code for graphs
- Code generator can generate buffer allocation
- Code generator can generate initialization code

### 2.3 Graph Access Code Generation

**Required**: Generate AArch64 code for graph field access

**Tasks**:
- Generate field access code (load graph structure fields)
- Generate buffer access code (load from graph buffers)
- Generate node/edge access code (`get_node`, `get_edge`)
- Generate bounds checking (if MTE not available)

**Deliverables**:
- Code generator can emit field access code
- Code generator can generate buffer access
- Code generator can generate bounds checking

## Phase 3: Code Generation - Graph Operations

### 3.1 Basic Graph Operation Code Generation

**Required**: Generate AArch64 code for basic graph operations

**Tasks**:
- Generate code for `node_count()` (load from graph structure)
- Generate code for `edge_count()` (load from graph structure)
- Generate code for `get_node()` (buffer access with bounds checking)
- Generate code for `in_degree()`, `out_degree()` (for directed graphs)

**Deliverables**:
- Code generator can emit basic graph operation code
- Code generator can generate graph metadata access
- Code generator can generate node/edge access

### 3.2 Neighbor Access Code Generation

**Required**: Generate AArch64 code for neighbor list operations

**Tasks**:
- Generate code for `in_neighbors()`, `out_neighbors()` (directed graphs)
- Generate code for `neighbors()` (undirected graphs)
- Generate code for neighbor list construction
- Generate code for neighbor list iteration

**Deliverables**:
- Code generator can emit neighbor access code
- Code generator can generate neighbor list construction
- Code generator can generate neighbor iteration

### 3.3 Graph Traversal Code Generation

**Required**: Generate AArch64 code for graph traversal operations

**Tasks**:
- Generate code for depth-first traversal
- Generate code for breadth-first traversal
- Generate code for recursive traversal (Silica uses recursion, not loops)
- Generate code for traversal with callbacks

**Deliverables**:
- Code generator can emit traversal code
- Code generator can generate recursive traversal
- Code generator can generate callback invocation

## Phase 4: Code Generation - Bulk Operations

### 4.1 Bulk Map Code Generation

**Required**: Generate AArch64 code for `bulk_map_nodes()` operation

**Tasks**:
- Generate SIMD-accelerated map code (using NEON/SVE from safe chip features)
- Generate loop unrolling for fixed-width operations (NEON)
- Generate adaptive SIMD code (NEON vs SVE detection)
- Generate scalar fallback code (when SIMD unavailable)
- Generate function application in SIMD context

**Deliverables**:
- Code generator can emit SIMD-accelerated map code
- Code generator can generate adaptive SIMD selection
- Code generator can generate scalar fallback

### 4.2 Bulk Filter Code Generation

**Required**: Generate AArch64 code for `bulk_filter_nodes()` operation

**Tasks**:
- Generate SVE predicate-based filter code (when SVE available)
- Generate NEON batch filter code (when only NEON available)
- Generate predicate mask generation
- Generate compaction code (SVE compress, NEON manual)
- Generate scalar fallback filter code

**Deliverables**:
- Code generator can emit SVE predicate filter code
- Code generator can generate NEON batch filter code
- Code generator can generate compaction operations

### 4.3 Bulk Reduce Code Generation

**Required**: Generate AArch64 code for `bulk_fold_nodes()` operation

**Tasks**:
- Generate SVE hardware reduction code (when SVE available)
- Generate NEON tree reduction code (when only NEON available)
- Generate reduction tree construction
- Generate scalar fallback reduction code

**Deliverables**:
- Code generator can emit SVE reduction code
- Code generator can generate NEON tree reduction
- Code generator can generate scalar reduction

### 4.4 Bulk Operation Integration

**Required**: Integrate bulk operations with graph structures

**Tasks**:
- Generate code to call bulk operations on graph buffers
- Generate code to construct new graphs from bulk operation results
- Generate code for bulk operation result allocation
- Generate immutability enforcement (new graph allocation)

**Deliverables**:
- Code generator can integrate bulk operations with graphs
- Code generator can generate result graph construction
- Code generator can enforce immutability

## Phase 5: Code Generation - Graph Builder

### 5.1 Builder Structure Code Generation

**Required**: Generate AArch64 code for graph builder structures

**Tasks**:
- Generate memory layout for builder structures
- Generate builder field initialization
- Generate builder buffer allocation
- Generate builder metadata tracking

**Deliverables**:
- Code generator can emit builder structure layouts
- Code generator can generate builder initialization
- Code generator can generate builder buffer management

### 5.2 Builder Method Code Generation

**Required**: Generate AArch64 code for builder methods

**Tasks**:
- Generate code for `add_node()` (append to builder buffer)
- Generate code for `add_edge()` (append edge data)
- Generate code for `add_weighted_edge()` (append with weight)
- Generate code for builder buffer growth (reallocation)

**Deliverables**:
- Code generator can emit builder method code
- Code generator can generate buffer append operations
- Code generator can generate buffer growth

### 5.3 Builder-to-Graph Conversion Code Generation

**Required**: Generate AArch64 code for `build()` method

**Tasks**:
- Generate code to allocate final graph structure
- Generate code to copy builder data to graph structure
- Generate code to compute graph metadata (node_count, edge_count)
- Generate code to determine dense vs sparse (if factory pattern used)
- Generate code to free builder resources

**Deliverables**:
- Code generator can emit builder-to-graph conversion
- Code generator can generate graph structure construction
- Code generator can generate resource cleanup

## Phase 6: Runtime Support

### 6.1 Graph Runtime Functions

**Required**: Implement runtime support functions for graph operations

**Tasks**:
- Implement graph structure allocation helpers
- Implement buffer allocation helpers (SIMD-aligned)
- Implement graph metadata access helpers
- Implement graph validation helpers (bounds checking)

**Deliverables**:
- Runtime functions for graph allocation
- Runtime functions for buffer management
- Runtime functions for graph validation

### 6.2 Bulk Operation Runtime Support

**Required**: Implement runtime support for bulk operations

**Tasks**:
- Implement SIMD capability detection (delegates to safe chip features)
- Implement adaptive SIMD selection (NEON vs SVE)
- Implement scalar fallback implementations
- Implement function application helpers

**Deliverables**:
- Runtime functions for SIMD detection
- Runtime functions for adaptive selection
- Runtime functions for scalar fallback

### 6.3 Graph Builder Runtime Support

**Required**: Implement runtime support for graph builders

**Tasks**:
- Implement builder buffer management
- Implement builder buffer growth
- Implement builder-to-graph conversion helpers
- Implement builder resource cleanup

**Deliverables**:
- Runtime functions for builder management
- Runtime functions for builder conversion
- Runtime functions for resource cleanup

## Phase 7: Standard Library Integration

### 7.1 Graph Primitive Standard Library

**Required**: Create standard library module for graph primitives

**Tasks**:
- Define standard graph trait implementations
- Define standard graph type implementations
- Define standard builder implementations
- Define standard bulk operation implementations

**Deliverables**:
- Standard library module for graph primitives
- Standard implementations for all graph traits
- Standard implementations for all graph types

### 7.2 Graph Factory Functions

**Required**: Implement factory functions for graph creation

**Tasks**:
- Implement `create_graph_builder()` factory function
- Implement dense vs sparse selection logic
- Implement graph construction helpers
- Implement graph validation functions

**Deliverables**:
- Factory functions for graph creation
- Dense/sparse selection logic
- Graph construction helpers

## Phase 8: Testing and Validation

### 8.1 Graph Structure Tests

**Required**: Create tests for graph structure code generation

**Tasks**:
- Test graph structure allocation
- Test graph structure access
- Test graph structure layout
- Test graph structure alignment

**Deliverables**:
- Test suite for graph structures
- Test cases for allocation
- Test cases for access patterns

### 8.2 Bulk Operation Tests

**Required**: Create tests for bulk operation code generation

**Tasks**:
- Test bulk map code generation
- Test bulk filter code generation
- Test bulk reduce code generation
- Test SIMD vs scalar code paths

**Deliverables**:
- Test suite for bulk operations
- Test cases for SIMD code paths
- Test cases for scalar fallback

### 8.3 Graph Builder Tests

**Required**: Create tests for graph builder code generation

**Tasks**:
- Test builder construction
- Test builder methods
- Test builder-to-graph conversion
- Test builder resource management

**Deliverables**:
- Test suite for graph builders
- Test cases for builder operations
- Test cases for conversion

### 8.4 Integration Tests

**Required**: Create integration tests for complete graph operations

**Tasks**:
- Test complete graph construction and usage
- Test bulk operations on real graphs
- Test graph immutability enforcement
- Test graph performance (SIMD acceleration)

**Deliverables**:
- Integration test suite
- Performance benchmarks
- Immutability validation tests

## Summary of Required Work

### Type Checking (Phase 1)
- [ ] Trait resolution for graph traits
- [ ] Graph type checking
- [ ] BulkTraversable type checking
- [ ] Graph builder type checking

### Code Generation - Structures (Phase 2)
- [ ] Graph structure layout generation
- [ ] Graph allocation code generation
- [ ] Graph access code generation

### Code Generation - Operations (Phase 3)
- [ ] Basic graph operation code generation
- [ ] Neighbor access code generation
- [ ] Graph traversal code generation

### Code Generation - Bulk Operations (Phase 4)
- [ ] Bulk map code generation
- [ ] Bulk filter code generation
- [ ] Bulk reduce code generation
- [ ] Bulk operation integration

### Code Generation - Builder (Phase 5)
- [ ] Builder structure code generation
- [ ] Builder method code generation
- [ ] Builder-to-graph conversion code generation

### Runtime Support (Phase 6)
- [ ] Graph runtime functions
- [ ] Bulk operation runtime support
- [ ] Graph builder runtime support

### Standard Library (Phase 7)
- [ ] Graph primitive standard library
- [ ] Graph factory functions

### Testing (Phase 8)
- [ ] Graph structure tests
- [ ] Bulk operation tests
- [ ] Graph builder tests
- [ ] Integration tests

## Estimated Time

- **Phase 1**: 3-5 days (type checking extensions)
- **Phase 2**: 2-3 days (structure code generation)
- **Phase 3**: 2-3 days (operation code generation)
- **Phase 4**: 5-7 days (bulk operation code generation - most complex)
- **Phase 5**: 2-3 days (builder code generation)
- **Phase 6**: 2-3 days (runtime support)
- **Phase 7**: 2-3 days (standard library)
- **Phase 8**: 3-5 days (testing)

**Total**: 21-32 days

## Success Criteria

✅ **Type Checking**:
- All graph traits can be type-checked
- All graph types can be validated
- All bulk operations can be type-checked
- All builder operations can be validated

✅ **Code Generation**:
- Graph structures can be allocated and accessed
- Graph operations can be executed
- Bulk operations generate SIMD-accelerated code
- Graph builders can construct graphs

✅ **Runtime**:
- Graph runtime functions work correctly
- SIMD operations are properly integrated
- Builder operations execute correctly

✅ **Testing**:
- All graph operations have test coverage
- SIMD code paths are validated
- Performance meets expectations

This provides the foundation for the bootstrap compiler to generate runnable binaries for graph primitives, enabling the Phase 2 compiler (written in Silica) to use graph primitives in its implementation.
