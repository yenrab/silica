# Graph Primitives: Bootstrap Compiler AST Development Plan

## Overview

This document outlines the **minimum AST implementation work** required for the Phase 2 Silica compiler to parse and represent graph primitives in the Abstract Syntax Tree (AST). This is focused on AST representation only - not full type checking, code generation, or runtime implementation.

## Dependencies

**⚠️ CRITICAL DEPENDENCY**: This plan **depends on** the `safe_chip_features/bootstrap_compiler_ast_plan.md` being completed first.

Graph primitives require the following from safe chip features:
1. **SIMD Vector Types**: Graph primitives use `Vec128Int32`, `VecInt32`, `Pred`, etc. in their implementations
2. **SIMD Function Calls**: Graph primitives call built-in functions like `load_128_int32()`, `simd_bulk_map_int32()`, etc.
3. **Marker Traits**: Graph primitives use `SIMDProcessable` and `PartiallySIMDProcessable` traits
4. **Safe Memory Types** (optional): Graph primitives may use `TaggedBufInt`, `PacPtrInt`, etc. for performance

**Implementation Order**:
1. ✅ Complete `safe_chip_features/bootstrap_compiler_ast_plan.md` first
2. ✅ Then proceed with `graph_primitives/bootstrap_compiler_ast_plan.md`

Without the safe chip features AST support, the graph primitives AST cannot properly represent:
- SIMD vector types in function signatures
- SIMD function calls in method bodies
- Marker trait references in trait implementations
- Safe memory type usage in graph structures

## Goal

Enable the Phase 2 compiler to:
1. Parse graph-related trait declarations
2. Parse graph-related type declarations
3. Parse graph-related function signatures
4. Build AST nodes representing graph primitives
5. Store graph-related information in the AST for later phases

## Phase 1: AST Node Extensions

### 1.1 Trait Declaration Support (Already Exists)

**Status**: ✅ Already supported in current AST

The existing `TraitDecl` structure supports:
- Trait name
- Included traits (sub-traits via `includes`)
- Trait methods
- Associated types

**Required Extensions**: None - existing structure is sufficient

**Example AST Representation**:
```rust
TraitDecl {
    name: "Graph",
    included_traits: vec![],
    methods: vec![
        TraitMethod {
            name: "node_count",
            params: vec![Parameter { name: "self", type_: Type::Named("Self"), .. }],
            return_type: Some(Type::Int),
            ..
        },
        // ... more methods
    ],
    ..
}
```

### 1.2 Type Declaration Support (Already Exists)

**Status**: ✅ Already supported in current AST

The existing `TypeDecl` structure supports:
- Type name
- Type definition

**Required Extensions**: None - existing structure is sufficient

**Example AST Representation**:
```rust
TypeDecl {
    name: "DenseGraphStructure",
    type_: Type::Record(vec![
        ("node_data", Type::Buffer { .. }),
        ("edge_from", Type::Buffer { .. }),
        // ...
    ]),
    ..
}
```

### 1.3 Function Declaration Support (Already Exists)

**Status**: ✅ Already supported in current AST

The existing `FunctionDecl` structure supports:
- Function name
- Parameters
- Return type
- Effects
- Body (statements)

**Required Extensions**: None - existing structure is sufficient

**Example AST Representation**:
```rust
FunctionDecl {
    name: "bulk_map_nodes",
    parameters: vec![
        Parameter { name: "self", type_: Type::Named("Self"), .. },
        Parameter { name: "f", type_: Type::Function { .. }, .. },
    ],
    return_type: Some(Type::Process { .. }),
    effects: vec![Effect::Memory(MemorySpace::Normal)],
    ..
}
```

### 1.4 Trait Implementation Support (Already Exists)

**Status**: ✅ Already supported in current AST

The existing `ImplDecl` structure supports:
- Trait name (optional for inherent impls)
- Type being implemented for
- Methods
- Associated types

**Required Extensions**: None - existing structure is sufficient

## Phase 2: Graph-Specific AST Elements

### 2.1 Graph Trait Hierarchy AST Representation

**Required**: No new AST nodes - use existing `TraitDecl` with `includes`

**Graph Traits to Represent**:
```rust
// Base Graph trait
TraitDecl {
    name: "Graph",
    included_traits: vec![],
    methods: vec![
        // node_count, edge_count, get_node
    ],
    ..
}

// BulkTraversable trait
TraitDecl {
    name: "BulkTraversable",
    included_traits: vec!["Graph"],  // includes Graph
    methods: vec![
        // bulk_map_nodes, bulk_filter_nodes, bulk_reduce_nodes
    ],
    ..
}

// DenseGraph trait
TraitDecl {
    name: "DenseGraph",
    included_traits: vec!["Graph", "BulkTraversable", "BulkSearchable"],
    methods: vec![],
    ..
}
```

**Deliverables**:
- AST can represent trait hierarchy via `included_traits`
- Parser can parse `trait X includes Y, Z` syntax
- AST stores trait relationships

### 2.2 Graph Type Definitions AST Representation

**Required**: Use existing `TypeDecl` and `Type::Record`

**Graph Types to Represent**:
```rust
// DenseGraphStructure type
TypeDecl {
    name: "DenseGraphStructure",
    type_: Type::Record(vec![
        ("node_data", Type::Buffer {
            region: Box::new(Type::Named("R")),
            space: MemorySpace::Normal,
            element_type: Box::new(Type::Named("NodeData")),
            capacity: 0,  // Variable capacity
        }),
        ("edge_from", Type::Buffer { .. }),
        ("edge_to", Type::Buffer { .. }),
        ("node_count", Type::Int),
        ("edge_count", Type::Int),
    ]),
    ..
}

// SparseGraphStructure type
TypeDecl {
    name: "SparseGraphStructure",
    type_: Type::Record(vec![
        ("nodes", Type::Buffer { .. }),
        ("edge_batches", Type::Buffer { .. }),
        // ...
    ]),
    ..
}
```

**Deliverables**:
- AST can represent graph structure types
- Parser can parse graph type definitions
- AST stores graph type information

### 2.3 Graph Builder Pattern AST Representation

**Required**: Use existing `TraitDecl` and `TypeDecl`

**Builder Traits and Types**:
```rust
// GraphBuilder trait
TraitDecl {
    name: "GraphBuilder",
    methods: vec![
        TraitMethod {
            name: "add_node",
            params: vec![
                Parameter { name: "self", type_: Type::Named("Self"), .. },
                Parameter { name: "data", type_: Type::Named("NodeData"), .. },
            ],
            return_type: Some(Type::Process {
                effects: vec![Effect::Memory(MemorySpace::Normal)],
                result_type: Box::new(Type::Named("Self")),
            }),
            ..
        },
        // ... more builder methods
    ],
    ..
}

// DenseGraphBuilder type
TypeDecl {
    name: "DenseGraphBuilder",
    type_: Type::Record(vec![
        ("node_buffer", Type::Buffer { .. }),
        ("edge_from_buffer", Type::Buffer { .. }),
        ("node_count", Type::Int),
        ("edge_count", Type::Int),
    ]),
    ..
}
```

**Deliverables**:
- AST can represent builder traits
- AST can represent builder types
- Parser can parse builder pattern

### 2.4 BulkTraversable Trait AST Representation

**Required**: Use existing `TraitDecl` and `ImplDecl`

**BulkTraversable Trait**:
```rust
TraitDecl {
    name: "BulkTraversable",
    included_traits: vec![],
    methods: vec![
        TraitMethod {
            name: "bulk_map",
            params: vec![
                Parameter { name: "self", type_: Type::Named("Self"), .. },
                Parameter { 
                    name: "f", 
                    type_: Type::Function {
                        parameters: vec![Type::Named("ElementType")],
                        return_type: Box::new(Type::Named("ElementType")),
                    },
                    ..
                },
            ],
            return_type: Some(Type::Process {
                effects: vec![Effect::Memory(MemorySpace::Normal)],
                result_type: Box::new(Type::Named("Self")),
            }),
            ..
        },
        // bulk_filter, bulk_reduce
    ],
    ..
}
```

**Marker Traits**:
```rust
// SIMDProcessable marker trait
// NOTE: This trait is defined in safe_chip_features, but graph primitives reference it
TraitDecl {
    name: "SIMDProcessable",
    included_traits: vec![],
    methods: vec![],  // Marker trait - no methods
    ..
}

// PartiallySIMDProcessable marker trait
// NOTE: This trait is defined in safe_chip_features, but graph primitives reference it
TraitDecl {
    name: "PartiallySIMDProcessable",
    included_traits: vec![],
    methods: vec![],  // Marker trait - no methods
    ..
}
```

**Dependency Note**: The `SIMDProcessable` and `PartiallySIMDProcessable` marker traits are defined as part of safe chip features. The graph primitives AST plan assumes these traits are already parseable and available in the compiler context from the safe chip features implementation.

**Deliverables**:
- AST can represent BulkTraversable trait
- AST can reference marker traits (defined in safe chip features)
- Parser can parse marker trait references

## Phase 3: Parser Extensions

### 3.1 Trait `includes` Syntax Parsing

**Current Status**: Need to verify parser supports `includes` keyword

**Required Parser Rules**:
```rust
// Parser rule for trait with includes
trait_declaration: 
    "trait" identifier 
    ("includes" identifier ("," identifier)*)? 
    "{" trait_method* "}"
```

**Deliverables**:
- Parser can parse `trait X includes Y, Z` syntax
- AST stores included traits in `TraitDecl.included_traits`

### 3.2 Process Type Syntax Parsing

**Current Status**: Need to verify parser supports `proc[effects] Type` syntax

**Required Parser Rules**:
```rust
// Parser rule for process types
process_type:
    "proc" "[" effect_list "]" type
```

**Deliverables**:
- Parser can parse `proc[mem(normal)] Type` syntax
- AST stores process types correctly

### 3.3 Buffer Type Syntax Parsing

**Current Status**: Need to verify parser supports buffer types

**Required Parser Rules**:
```rust
// Parser rule for buffer types
buffer_type:
    "buf" "(" region "," space "," element_type "," capacity ")"
```

**Deliverables**:
- Parser can parse `buf(R, normal, T, N)` syntax
- AST stores buffer types correctly

### 3.4 Function Type Syntax Parsing

**Current Status**: Need to verify parser supports function types in parameters

**Required Parser Rules**:
```rust
// Parser rule for function types
function_type:
    "(" parameter_type_list ")" "->" return_type
```

**Deliverables**:
- Parser can parse `(T) -> T` function types
- AST stores function types in parameters correctly

## Phase 4: AST Validation (Minimal)

### 4.1 Trait Name Validation

**Required**: Basic validation that trait names are valid identifiers

**Deliverables**:
- AST validation ensures trait names are valid
- Error messages for invalid trait names

### 4.2 Type Name Validation

**Required**: Basic validation that type names are valid identifiers

**Deliverables**:
- AST validation ensures type names are valid
- Error messages for invalid type names

### 4.3 Method Signature Validation

**Required**: Basic validation that method signatures are well-formed

**Deliverables**:
- AST validation ensures method parameters have types
- AST validation ensures return types are valid
- Error messages for invalid signatures

## Phase 5: AST Storage and Retrieval

### 5.1 Trait Registry

**Required**: Store trait declarations in compiler context

**Implementation**:
```rust
pub struct CompilerContext {
    pub traits: HashMap<String, TraitDecl>,
    pub types: HashMap<String, TypeDecl>,
    pub functions: HashMap<String, FunctionDecl>,
    // ...
}
```

**Deliverables**:
- Compiler context stores all trait declarations
- Can look up traits by name
- Can check if trait exists

### 5.2 Type Registry

**Required**: Store type declarations in compiler context

**Deliverables**:
- Compiler context stores all type declarations
- Can look up types by name
- Can check if type exists

### 5.3 Trait Relationship Tracking

**Required**: Track trait inclusion relationships

**Implementation**:
```rust
pub struct TraitRegistry {
    pub traits: HashMap<String, TraitDecl>,
    pub includes_graph: HashMap<String, Vec<String>>,  // trait -> included traits
}
```

**Deliverables**:
- Can query which traits a trait includes
- Can build trait inclusion graph
- Can check trait relationships

## Summary of Required Work

### AST Nodes (Already Exist)
- ✅ `TraitDecl` - for trait declarations
- ✅ `TypeDecl` - for type declarations
- ✅ `FunctionDecl` - for function declarations
- ✅ `ImplDecl` - for trait implementations
- ✅ `Type` enum - for all type representations

### Parser Extensions Needed
- [ ] Verify `includes` keyword parsing in trait declarations
- [ ] Verify `proc[effects] Type` syntax parsing
- [ ] Verify `buf(R, S, T, N)` syntax parsing
- [ ] Verify function type `(T) -> T` syntax in parameters

### AST Validation Needed
- [ ] Basic trait name validation
- [ ] Basic type name validation
- [ ] Basic method signature validation

### Compiler Context Extensions
- [ ] Trait registry (HashMap<String, TraitDecl>)
- [ ] Type registry (HashMap<String, TypeDecl>)
- [ ] Trait inclusion graph tracking

## Example: Complete Graph Trait AST

```rust
// Example: Graph trait AST representation
let graph_trait = TraitDecl {
    name: "Graph".to_string(),
    included_traits: vec![],
    associated_types: vec![],
    methods: vec![
        TraitMethod {
            name: "node_count".to_string(),
            params: vec![
                Parameter {
                    name: "self".to_string(),
                    type_: Type::Named("Self".to_string()),
                    pattern: None,
                    location: SourceLocation::default(),
                },
            ],
            return_type: Some(Type::Int),
            location: SourceLocation::default(),
        },
        TraitMethod {
            name: "edge_count".to_string(),
            params: vec![
                Parameter {
                    name: "self".to_string(),
                    type_: Type::Named("Self".to_string()),
                    pattern: None,
                    location: SourceLocation::default(),
                },
            ],
            return_type: Some(Type::Int),
            location: SourceLocation::default(),
        },
        TraitMethod {
            name: "get_node".to_string(),
            params: vec![
                Parameter {
                    name: "self".to_string(),
                    type_: Type::Named("Self".to_string()),
                    pattern: None,
                    location: SourceLocation::default(),
                },
                Parameter {
                    name: "node_id".to_string(),
                    type_: Type::Int,
                    pattern: None,
                    location: SourceLocation::default(),
                },
            ],
            return_type: Some(Type::Process {
                effects: vec![Effect::Memory(MemorySpace::Normal)],
                result_type: Box::new(Type::Named("NodeData".to_string())),
            }),
            location: SourceLocation::default(),
        },
    ],
    location: SourceLocation::default(),
};
```

## Deliverables Checklist

### Phase 1: AST Node Extensions
- [x] Verify existing AST nodes support graph primitives
- [x] Document which AST nodes are used

### Phase 2: Graph-Specific AST Elements
- [ ] Document trait hierarchy AST representation
- [ ] Document graph type definitions AST representation
- [ ] Document builder pattern AST representation
- [ ] Document BulkTraversable trait AST representation

### Phase 3: Parser Extensions
- [ ] Verify/implement `includes` keyword parsing
- [ ] Verify/implement `proc[effects] Type` parsing
- [ ] Verify/implement `buf(R, S, T, N)` parsing
- [ ] Verify/implement function type parsing in parameters

### Phase 4: AST Validation
- [ ] Implement trait name validation
- [ ] Implement type name validation
- [ ] Implement method signature validation

### Phase 5: AST Storage
- [ ] Implement trait registry in compiler context
- [ ] Implement type registry in compiler context
- [ ] Implement trait inclusion graph tracking

## Estimated Time

**Note**: These estimates assume `safe_chip_features/bootstrap_compiler_ast_plan.md` is completed first.

- **Phase 1**: 0 days (already exists)
- **Phase 2**: 1-2 days (documentation and verification)
- **Phase 3**: 2-3 days (parser extensions)
- **Phase 4**: 1-2 days (basic validation)
- **Phase 5**: 1-2 days (registry implementation)

**Total**: 5-9 days (after safe chip features AST is complete)

## Success Criteria

✅ **AST can represent**:
- All graph trait declarations (Graph, BulkTraversable, DenseGraph, etc.)
- All graph type definitions (DenseGraphStructure, SparseGraphStructure, etc.)
- All graph function signatures (bulk_map_nodes, bulk_filter_nodes, etc.)
- Trait inclusion relationships
- Builder pattern types and traits

✅ **Parser can parse**:
- Trait declarations with `includes`
- Process types `proc[effects] Type`
- Buffer types `buf(R, S, T, N)`
- Function types in parameters

✅ **Compiler context can store**:
- All trait declarations
- All type declarations
- Trait inclusion relationships

This provides the foundation for Phase 2 compiler to build ASTs representing graph primitives, enabling later phases to perform type checking, code generation, and optimization.
