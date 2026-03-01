# Safe Chip Features: Bootstrap Compiler AST Development Plan

## Overview

This document outlines the **minimum AST implementation work** required for the Phase 2 Silica compiler to parse and represent safe chip features (SIMD, NEON, SVE, MTE, PAC, Prefixed Pointers) in the Abstract Syntax Tree (AST). This is focused on AST representation only - not full type checking, code generation, or runtime implementation.

## Goal

Enable the Phase 2 compiler to:
1. Parse built-in chip feature type declarations
2. Parse built-in chip feature function declarations
3. Parse compiler target specification flags (--arch, --ext, --cpu)
4. Build AST nodes representing chip features
5. Store chip feature information in the AST and compiler context

## Phase 1: Built-In Type AST Representation

### 1.1 SIMD Vector Types

**Required**: Extend `Type` enum to include built-in SIMD vector types

**New AST Type Variants**:
```rust
pub enum Type {
    // ... existing types ...
    
    // NEON 128-bit vector types
    Vec128Int8,
    Vec128Int16,
    Vec128Int32,
    Vec128Int64,
    Vec128Float32,
    Vec128Bool,
    
    // SVE scalable vector types
    VecInt8,
    VecInt16,
    VecInt32,
    VecInt64,
    VecFloat16,
    VecFloat32,
    VecFloat64,
    VecBool,
    
    // SVE predicate
    Pred,
}
```

**Parser Extensions**:
```rust
// Parser rules for built-in vector types
builtin_type:
    "Vec128Int8" | "Vec128Int16" | "Vec128Int32" | "Vec128Int64" 
    | "Vec128Float32" | "Vec128Bool"
    | "VecInt8" | "VecInt16" | "VecInt32" | "VecInt64"
    | "VecFloat16" | "VecFloat32" | "VecFloat64" | "VecBool"
    | "Pred"
```

**Deliverables**:
- AST `Type` enum includes all SIMD vector types
- Parser recognizes built-in vector type names
- AST stores vector types correctly

### 1.2 Safe Memory Types

**Required**: Extend `Type` enum to include MTE, PAC, and Prefixed Pointer types

**New AST Type Variants**:
```rust
pub enum Type {
    // ... existing types ...
    
    // MTE tagged pointer types
    TaggedPtrInt,
    TaggedPtrInt64,
    TaggedPtrNodeData,
    TaggedPtrEdge,
    
    // MTE tagged buffer types
    TaggedBufInt,
    TaggedBufInt64,
    TaggedBufNodeData,
    TaggedBufEdge,
    
    // PAC authenticated pointer types
    PacPtrInt,
    PacPtrNodeData,
    PacPtrEdge,
    
    // Prefixed pointer types
    PrefixedPtrInt,
    PrefixedPtrNodeData,
    PrefixedPtrEdge,
}
```

**Parser Extensions**:
```rust
// Parser rules for safe memory types
safe_memory_type:
    "TaggedPtrInt" | "TaggedPtrInt64" | "TaggedPtrNodeData" | "TaggedPtrEdge"
    | "TaggedBufInt" | "TaggedBufInt64" | "TaggedBufNodeData" | "TaggedBufEdge"
    | "PacPtrInt" | "PacPtrNodeData" | "PacPtrEdge"
    | "PrefixedPtrInt" | "PrefixedPtrNodeData" | "PrefixedPtrEdge"
```

**Deliverables**:
- AST `Type` enum includes all safe memory types
- Parser recognizes safe memory type names
- AST stores safe memory types correctly

### 1.3 OptionPred Variant Type

**Required**: Support variant type syntax `Some(Pred) | None`

**Current Status**: Need to verify variant type parsing

**AST Representation**:
```rust
Type::Variant(vec![
    ("Some", Some(Type::Pred)),
    ("None", None),
])
```

**Parser Extensions**:
```rust
// Parser rule for variant types
variant_type:
    variant_constructor ("|" variant_constructor)*

variant_constructor:
    identifier ("(" type ")")?
```

**Deliverables**:
- AST can represent `OptionPred = Some(Pred) | None`
- Parser can parse variant type syntax
- AST stores variant types correctly

### 1.4 RuntimeFeatures Type

**Required**: Support struct type for runtime feature query

**AST Representation**:
```rust
TypeDecl {
    name: "RuntimeFeatures",
    type_: Type::Record(vec![
        ("sve_vector_length", Type::Int),
        ("mte_available", Type::Bool),
        ("pac_available", Type::Bool),
        ("prefixed_available", Type::Bool),
        ("cache_line_size", Type::Int),
        ("numa_nodes", Type::Int),
    ]),
    ..
}
```

**Deliverables**:
- AST can represent RuntimeFeatures struct type
- Parser can parse RuntimeFeatures type definition

## Phase 2: Built-In Function AST Representation

### 2.1 SIMD Operation Functions

**Required**: Parse built-in SIMD function declarations

**Function Categories**:
1. **NEON Load/Store**: `load_128_int32`, `store_128_int32`, etc.
2. **NEON Arithmetic**: `add_128_int32`, `mul_128_int32`, etc.
3. **NEON Comparisons**: `compare_eq_128_int32`, etc.
4. **NEON Lane Ops**: `extract_lane_128_int32`, `broadcast_128_int32`, etc.
5. **SVE Load/Store**: `load_vector_int32`, `store_vector_int32`, etc.
6. **SVE Arithmetic**: `add_vectors_int32`, etc.
7. **SVE Predicates**: `create_pred_true`, `test_any_true`, etc.
8. **SVE Reductions**: `reduce_add_vector_int32`, etc.

**AST Representation**:
```rust
FunctionDecl {
    name: "load_128_int32",
    parameters: vec![
        Parameter {
            name: "ptr",
            type_: Type::Pointer(Box::new(Type::Int32)),
            ..
        },
    ],
    return_type: Some(Type::Vec128Int32),
    effects: vec![],  // Built-in functions may have no effects
    body: vec![],  // Built-in - no body in source
    ..
}
```

**Parser Extensions**:
```rust
// Parser rule for built-in function declarations
builtin_function:
    "fn" builtin_function_name "(" parameter_list ")" "->" return_type
```

**Deliverables**:
- AST can represent built-in SIMD function declarations
- Parser can parse built-in function signatures
- Compiler context stores built-in functions separately from user functions

### 2.2 Safe Memory Operation Functions

**Required**: Parse built-in safe memory function declarations

**Function Categories**:
1. **MTE Operations**: `alloc_tagged_int`, `read_tagged_buf_int`, `set_tag`, etc.
2. **PAC Operations**: `sign_ptr_int`, `auth_ptr_int`, etc.
3. **Prefixed Operations**: `create_prefixed_int`, `deref_prefixed_int`, etc.

**AST Representation**:
```rust
FunctionDecl {
    name: "alloc_tagged_int",
    parameters: vec![
        Parameter {
            name: "size",
            type_: Type::Int,
            ..
        },
    ],
    return_type: Some(Type::Process {
        effects: vec![Effect::Memory(MemorySpace::Normal)],
        result_type: Box::new(Type::TaggedPtrInt),
    }),
    effects: vec![Effect::Memory(MemorySpace::Normal)],
    body: vec![],  // Built-in - no body
    ..
}
```

**Deliverables**:
- AST can represent built-in safe memory function declarations
- Parser can parse safe memory function signatures
- Compiler context stores built-in functions

### 2.3 Runtime Feature Query Function

**Required**: Parse `query_runtime_features()` function

**AST Representation**:
```rust
FunctionDecl {
    name: "query_runtime_features",
    parameters: vec![],
    return_type: Some(Type::Named("RuntimeFeatures")),
    effects: vec![],  // Query only, no side effects
    body: vec![],  // Built-in - implemented in runtime
    ..
}
```

**Deliverables**:
- AST can represent runtime feature query function
- Parser can parse function signature

## Phase 3: Compiler Flag AST Representation

### 3.1 Compiler Target Specification

**Required**: New AST node for compiler target specification

**New AST Node**:
```rust
/// Compiler target specification
#[derive(Debug, Clone)]
pub struct TargetSpec {
    pub architecture: Option<String>,  // armv8-a, armv9-a, etc.
    pub extensions: Vec<String>,       // +neon, +sve, +mte, etc.
    pub cpu: Option<String>,           // cortex-a78, neoverse-n2, etc.
    pub auto_detect: bool,            // --auto-detect flag
    pub location: SourceLocation,
}
```

**Compiler Context Extension**:
```rust
pub struct CompilerContext {
    // ... existing fields ...
    pub target_spec: Option<TargetSpec>,
    pub available_features: HashSet<String>,  // Computed from target_spec
}
```

**Parser Extensions** (Command-line, not source code):
```rust
// Command-line flag parsing (not source code parsing)
target_flag: "--arch" architecture_name
extension_flag: "--ext" extension_list
cpu_flag: "--cpu" cpu_name
auto_detect_flag: "--auto-detect"
```

**Deliverables**:
- AST node for target specification
- Command-line parser for target flags
- Compiler context stores target specification
- Feature availability computed from target spec

### 3.2 Extension Validation

**Required**: Track valid extensions and validate

**Implementation**:
```rust
pub struct ExtensionRegistry {
    pub valid_extensions: HashSet<String>,  // +neon, +sve, +mte, etc.
}

impl ExtensionRegistry {
    pub fn validate(&self, ext: &str) -> bool {
        self.valid_extensions.contains(ext)
    }
    
    pub fn validate_all(&self, exts: &[String]) -> Vec<String> {
        // Returns invalid extensions
        exts.iter()
            .filter(|ext| !self.valid_extensions.contains(*ext))
            .cloned()
            .collect()
    }
}
```

**Deliverables**:
- Extension registry with valid extensions
- Validation function for extensions
- Informational messages for invalid extensions

## Phase 4: Trait AST Representation

### 4.1 Marker Traits

**Required**: Parse marker traits (no methods)

**AST Representation**:
```rust
// SIMDProcessable marker trait
TraitDecl {
    name: "SIMDProcessable",
    included_traits: vec![],
    methods: vec![],  // Marker trait - empty methods
    ..
}

// Vec128Element marker trait
TraitDecl {
    name: "Vec128Element",
    included_traits: vec![],
    methods: vec![],  // Marker trait
    ..
}
```

**Deliverables**:
- AST can represent marker traits (empty method lists)
- Parser can parse marker traits

### 4.2 SIMD Operation Traits

**Required**: Parse SIMD operation traits

**AST Representation**:
```rust
// Vec128Arithmetic trait
TraitDecl {
    name: "Vec128Arithmetic",
    included_traits: vec![],
    methods: vec![
        TraitMethod {
            name: "add_128",
            params: vec![
                Parameter { name: "self", type_: Type::Named("Self"), .. },
                Parameter { name: "other", type_: Type::Named("Self"), .. },
            ],
            return_type: Some(Type::Named("Self")),
            ..
        },
        // sub_128, mul_128
    ],
    ..
}
```

**Deliverables**:
- AST can represent SIMD operation traits
- Parser can parse trait method signatures

### 4.3 Safe Memory Traits

**Required**: Parse safe memory traits

**AST Representation**:
```rust
// TaggedPointer trait
TraitDecl {
    name: "TaggedPointer",
    included_traits: vec![],
    methods: vec![
        TraitMethod {
            name: "get_tag",
            params: vec![
                Parameter { name: "self", type_: Type::Named("Self"), .. },
            ],
            return_type: Some(Type::Int),
            ..
        },
        TraitMethod {
            name: "set_tag",
            params: vec![
                Parameter { name: "self", type_: Type::Named("Self"), .. },
                Parameter { name: "tag", type_: Type::Int, .. },
            ],
            return_type: Some(Type::Named("Self")),  // Returns new pointer
            ..
        },
        // check_tag
    ],
    ..
}
```

**Deliverables**:
- AST can represent safe memory traits
- Parser can parse trait methods

## Phase 5: Built-In Type and Function Registry

### 5.1 Built-In Type Registry

**Required**: Registry for built-in types

**Implementation**:
```rust
pub struct BuiltInTypeRegistry {
    pub simd_vector_types: HashSet<String>,  // Vec128Int32, VecInt32, etc.
    pub safe_memory_types: HashSet<String>,  // TaggedPtrInt, PacPtrInt, etc.
    pub all_builtin_types: HashSet<String>,
}

impl BuiltInTypeRegistry {
    pub fn new() -> Self {
        let mut registry = BuiltInTypeRegistry {
            simd_vector_types: HashSet::new(),
            safe_memory_types: HashSet::new(),
            all_builtin_types: HashSet::new(),
        };
        
        // Register SIMD types
        registry.simd_vector_types.insert("Vec128Int8".to_string());
        registry.simd_vector_types.insert("Vec128Int32".to_string());
        // ... all SIMD types
        
        // Register safe memory types
        registry.safe_memory_types.insert("TaggedPtrInt".to_string());
        registry.safe_memory_types.insert("PacPtrInt".to_string());
        // ... all safe memory types
        
        // Combine
        registry.all_builtin_types.extend(&registry.simd_vector_types);
        registry.all_builtin_types.extend(&registry.safe_memory_types);
        
        registry
    }
    
    pub fn is_builtin_type(&self, name: &str) -> bool {
        self.all_builtin_types.contains(name)
    }
}
```

**Deliverables**:
- Built-in type registry
- Functions to check if type is built-in
- Functions to check type category (SIMD, safe memory, etc.)

### 5.2 Built-In Function Registry

**Required**: Registry for built-in functions

**Implementation**:
```rust
pub struct BuiltInFunctionRegistry {
    pub neon_functions: HashSet<String>,      // load_128_int32, etc.
    pub sve_functions: HashSet<String>,        // load_vector_int32, etc.
    pub mte_functions: HashSet<String>,        // alloc_tagged_int, etc.
    pub pac_functions: HashSet<String>,        // sign_ptr_int, etc.
    pub all_builtin_functions: HashSet<String>,
}

impl BuiltInFunctionRegistry {
    pub fn new() -> Self {
        let mut registry = BuiltInFunctionRegistry {
            neon_functions: HashSet::new(),
            sve_functions: HashSet::new(),
            mte_functions: HashSet::new(),
            pac_functions: HashSet::new(),
            all_builtin_functions: HashSet::new(),
        };
        
        // Register NEON functions
        registry.neon_functions.insert("load_128_int32".to_string());
        registry.neon_functions.insert("store_128_int32".to_string());
        // ... all NEON functions
        
        // Register SVE functions
        registry.sve_functions.insert("load_vector_int32".to_string());
        // ... all SVE functions
        
        // Register MTE functions
        registry.mte_functions.insert("alloc_tagged_int".to_string());
        // ... all MTE functions
        
        // Register PAC functions
        registry.pac_functions.insert("sign_ptr_int".to_string());
        // ... all PAC functions
        
        // Combine
        registry.all_builtin_functions.extend(&registry.neon_functions);
        registry.all_builtin_functions.extend(&registry.sve_functions);
        registry.all_builtin_functions.extend(&registry.mte_functions);
        registry.all_builtin_functions.extend(&registry.pac_functions);
        
        registry
    }
    
    pub fn is_builtin_function(&self, name: &str) -> bool {
        self.all_builtin_functions.contains(name)
    }
}
```

**Deliverables**:
- Built-in function registry
- Functions to check if function is built-in
- Functions to check function category

## Phase 6: Parser Extensions

### 6.1 Built-In Type Name Recognition

**Required**: Parser recognizes built-in type names as keywords or special identifiers

**Parser Extensions**:
```rust
// Option 1: Add to keyword list (if treating as keywords)
keywords: "Vec128Int32" | "VecInt32" | "Pred" | ...

// Option 2: Special identifier recognition (if treating as identifiers)
builtin_type_identifier:
    "Vec128" ("Int8" | "Int16" | "Int32" | "Int64" | "Float32" | "Bool")
    | "Vec" ("Int8" | "Int16" | "Int32" | "Int64" | "Float16" | "Float32" | "Float64" | "Bool")
    | "Pred"
    | "TaggedPtr" ("Int" | "Int64" | "NodeData" | "Edge")
    | "TaggedBuf" ("Int" | "Int64" | "NodeData" | "Edge")
    | "PacPtr" ("Int" | "NodeData" | "Edge")
    | "PrefixedPtr" ("Int" | "NodeData" | "Edge")
```

**Deliverables**:
- Parser recognizes built-in type names
- Parser can distinguish built-in types from user types

### 6.2 Built-In Function Name Recognition

**Required**: Parser recognizes built-in function names

**Parser Extensions**:
```rust
// Built-in function name patterns
builtin_function_name:
    "load_128_" (element_type)
    | "store_128_" (element_type)
    | "add_128_" (element_type)
    | "load_vector_" (element_type)
    | "alloc_tagged_" (element_type)
    | "sign_ptr_" (element_type)
    | "query_runtime_features"
    | ...
```

**Deliverables**:
- Parser recognizes built-in function names
- Parser can distinguish built-in functions from user functions

### 6.3 Variant Type Parsing

**Required**: Parser for variant type syntax

**Parser Extensions**:
```rust
// Variant type syntax: Some(Pred) | None
variant_type:
    variant_constructor ("|" variant_constructor)*

variant_constructor:
    identifier ("(" type ")")?
```

**Deliverables**:
- Parser can parse `OptionPred = Some(Pred) | None`
- AST stores variant types correctly

## Phase 7: Compiler Context Extensions

### 7.1 Target Specification Storage

**Required**: Store target specification in compiler context

**Implementation**:
```rust
pub struct CompilerContext {
    // ... existing fields ...
    pub target_spec: Option<TargetSpec>,
    pub available_features: HashSet<String>,
}

impl CompilerContext {
    pub fn set_target_spec(&mut self, spec: TargetSpec) {
        self.target_spec = Some(spec.clone());
        self.available_features = self.compute_available_features(&spec);
    }
    
    fn compute_available_features(&self, spec: &TargetSpec) -> HashSet<String> {
        let mut features = HashSet::new();
        
        // Baseline: always armv8-a
        features.insert("armv8-a".to_string());
        
        // Add extensions
        for ext in &spec.extensions {
            features.insert(ext.clone());
        }
        
        // Add CPU-specific features
        if let Some(cpu) = &spec.cpu {
            let cpu_features = self.get_cpu_features(cpu);
            features.extend(cpu_features);
        }
        
        features
    }
    
    pub fn has_feature(&self, feature: &str) -> bool {
        self.available_features.contains(feature)
    }
}
```

**Deliverables**:
- Compiler context stores target specification
- Available features computed from target spec
- Can query feature availability

### 7.2 Built-In Registry Integration

**Required**: Integrate built-in registries into compiler context

**Implementation**:
```rust
pub struct CompilerContext {
    // ... existing fields ...
    pub builtin_types: BuiltInTypeRegistry,
    pub builtin_functions: BuiltInFunctionRegistry,
    pub extension_registry: ExtensionRegistry,
}
```

**Deliverables**:
- Compiler context includes built-in registries
- Can check if type/function is built-in
- Can validate extensions

## Summary of Required Work

### AST Extensions
- [ ] Extend `Type` enum with SIMD vector types (Vec128Int32, VecInt32, etc.)
- [ ] Extend `Type` enum with safe memory types (TaggedPtrInt, PacPtrInt, etc.)
- [ ] Verify variant type support (`Some(Pred) | None`)
- [ ] Add `TargetSpec` AST node for compiler flags

### Parser Extensions
- [ ] Parse built-in type names (Vec128Int32, Pred, etc.)
- [ ] Parse built-in function names (load_128_int32, etc.)
- [ ] Parse variant type syntax (`Some(T) | None`)
- [ ] Parse command-line target flags (--arch, --ext, --cpu)

### Compiler Context Extensions
- [ ] Add built-in type registry
- [ ] Add built-in function registry
- [ ] Add extension registry
- [ ] Add target specification storage
- [ ] Add feature availability tracking

### Validation
- [ ] Validate extension names (informational messages for invalid)
- [ ] Validate built-in type usage
- [ ] Validate built-in function usage

## Example: Complete Built-In Function AST

```rust
// Example: load_128_int32 built-in function AST
let load_function = FunctionDecl {
    name: "load_128_int32".to_string(),
    parameters: vec![
        Parameter {
            name: "ptr".to_string(),
            type_: Type::Pointer(Box::new(Type::Int32)),
            pattern: None,
            location: SourceLocation::default(),
        },
    ],
    return_type: Some(Type::Vec128Int32),
    effects: vec![],
    body: vec![],  // Built-in - no body
    location: SourceLocation::default(),
};
```

## Deliverables Checklist

### Phase 1: Built-In Type AST
- [ ] Extend `Type` enum with SIMD vector types
- [ ] Extend `Type` enum with safe memory types
- [ ] Verify variant type support
- [ ] Document all built-in types

### Phase 2: Built-In Function AST
- [ ] Document SIMD function signatures
- [ ] Document safe memory function signatures
- [ ] Document runtime feature query function
- [ ] Create AST representations for all built-in functions

### Phase 3: Compiler Flag AST
- [ ] Create `TargetSpec` AST node
- [ ] Implement command-line flag parsing
- [ ] Implement extension validation
- [ ] Store target spec in compiler context

### Phase 4: Trait AST
- [ ] Document marker trait AST representation
- [ ] Document SIMD operation trait AST representation
- [ ] Document safe memory trait AST representation

### Phase 5: Built-In Registries
- [ ] Implement built-in type registry
- [ ] Implement built-in function registry
- [ ] Integrate registries into compiler context

### Phase 6: Parser Extensions
- [ ] Parse built-in type names
- [ ] Parse built-in function names
- [ ] Parse variant type syntax

### Phase 7: Compiler Context
- [ ] Add target specification storage
- [ ] Add feature availability tracking
- [ ] Add built-in registries

## Estimated Time

- **Phase 1**: 2-3 days (extend Type enum, verify variant support)
- **Phase 2**: 1-2 days (document function signatures)
- **Phase 3**: 2-3 days (command-line parsing, target spec)
- **Phase 4**: 1 day (trait AST documentation)
- **Phase 5**: 2-3 days (registry implementation)
- **Phase 6**: 2-3 days (parser extensions)
- **Phase 7**: 1-2 days (compiler context integration)

**Total**: 11-17 days

## Success Criteria

✅ **AST can represent**:
- All built-in SIMD vector types (Vec128Int32, VecInt32, Pred, etc.)
- All built-in safe memory types (TaggedPtrInt, PacPtrInt, etc.)
- All built-in function signatures (load_128_int32, alloc_tagged_int, etc.)
- Variant types (OptionPred = Some(Pred) | None)
- Compiler target specification

✅ **Parser can parse**:
- Built-in type names
- Built-in function names
- Variant type syntax
- Command-line target flags

✅ **Compiler context can store**:
- Target specification
- Available features
- Built-in type registry
- Built-in function registry

This provides the foundation for Phase 2 compiler to build ASTs representing safe chip features, enabling later phases to perform type checking, code generation, and optimization based on available hardware features.
