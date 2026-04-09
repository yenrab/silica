# Built-In Chip Features Design (Trait-Based)

## Design Philosophy

**AArch64-Native First**: Since Silica is designed specifically for AArch64, chip features should be **built into the language**, not optional modules.

**Trait-Based Design**: All types use traits instead of generics, consistent with Silica's design.

## Built-In Traits

### 1. SIMD Element Traits

```silica
// Built-in traits for SIMD-compatible types

// Trait for types that can be in NEON 128-bit vectors
trait Vec128Element {
    // Marker trait - no methods, just indicates compatibility
}

// Trait for types that can be in SVE scalable vectors
trait VecElement {
    // Marker trait - no methods, just indicates compatibility
}

// Implementations for supported types
impl Vec128Element for int8;
impl Vec128Element for int16;
impl Vec128Element for int32;
impl Vec128Element for int64;
impl Vec128Element for float32;

impl VecElement for int8;
impl VecElement for int16;
impl VecElement for int32;
impl VecElement for int64;
impl VecElement for float16;
impl VecElement for float32;
impl VecElement for float64;
```

### 2. SIMD Vector Types (Concrete Types)

```silica
// Built-in concrete vector types - one for each element type

// NEON 128-bit vectors
type Vec128Int8    // 16 × int8
type Vec128Int16   // 8 × int16
type Vec128Int32   // 4 × int32
type Vec128Int64   // 2 × int64
type Vec128Float32 // 4 × float32
type Vec128Bool    // Boolean vector for comparisons

// SVE scalable vectors (adapt to hardware)
type VecInt8
type VecInt16
type VecInt32
type VecInt64
type VecFloat16
type VecFloat32
type VecFloat64
type VecBool

// SVE predicate mask
type Pred
```

### 3. NEON Operations (Trait-Based)

```silica
// Built-in NEON operations - work with concrete types or traits

// Load/Store - concrete type versions
fn load_128_int32(ptr: *int32) -> Vec128Int32
fn load_128_int64(ptr: *int64) -> Vec128Int64
fn load_128_float32(ptr: *float32) -> Vec128Float32

fn store_128_int32(ptr: *int32, vec: Vec128Int32) -> unit
fn store_128_int64(ptr: *int64, vec: Vec128Int64) -> unit
fn store_128_float32(ptr: *float32, vec: Vec128Float32) -> unit

// Arithmetic - concrete type operations (no generics, no trait needed)
fn add_128_int32(a: Vec128Int32, b: Vec128Int32) -> Vec128Int32
fn sub_128_int32(a: Vec128Int32, b: Vec128Int32) -> Vec128Int32
fn mul_128_int32(a: Vec128Int32, b: Vec128Int32) -> Vec128Int32

fn add_128_int64(a: Vec128Int64, b: Vec128Int64) -> Vec128Int64
fn sub_128_int64(a: Vec128Int64, b: Vec128Int64) -> Vec128Int64
fn mul_128_int64(a: Vec128Int64, b: Vec128Int64) -> Vec128Int64

fn add_128_float32(a: Vec128Float32, b: Vec128Float32) -> Vec128Float32
fn sub_128_float32(a: Vec128Float32, b: Vec128Float32) -> Vec128Float32
fn mul_128_float32(a: Vec128Float32, b: Vec128Float32) -> Vec128Float32

// Or use trait for abstraction when needed
trait Vec128Arithmetic {
    fn add_128(self: Self, other: Self) -> Self;
    fn sub_128(self: Self, other: Self) -> Self;
    fn mul_128(self: Self, other: Self) -> Self;
}

impl Vec128Arithmetic for Vec128Int32 {
    fn add_128(self: Vec128Int32, other: Vec128Int32) -> Vec128Int32 {
        add_128_int32(self, other)  // Calls concrete function
    }
    fn sub_128(self: Vec128Int32, other: Vec128Int32) -> Vec128Int32 {
        sub_128_int32(self, other)
    }
    fn mul_128(self: Vec128Int32, other: Vec128Int32) -> Vec128Int32 {
        mul_128_int32(self, other)
    }
}

// Comparisons
fn compare_eq_128_int32(a: Vec128Int32, b: Vec128Int32) -> Vec128Bool
fn compare_gt_128_int32(a: Vec128Int32, b: Vec128Int32) -> Vec128Bool
fn compare_lt_128_int32(a: Vec128Int32, b: Vec128Int32) -> Vec128Bool

// Lane operations
fn extract_lane_128_int32(vec: Vec128Int32, lane: int) -> int32
fn insert_lane_128_int32(vec: Vec128Int32, lane: int, value: int32) -> Vec128Int32
fn broadcast_128_int32(value: int32) -> Vec128Int32

// Horizontal operations
fn hadd_128_int32(vec: Vec128Int32) -> Vec128Int32  // Horizontal add
fn test_any_true(vec: Vec128Bool) -> bool
fn test_all_true(vec: Vec128Bool) -> bool
```

### 4. SVE Operations (Trait-Based)

```silica
// Built-in SVE operations - concrete types

// Load/Store with predicates
fn load_vector_int32(ptr: *int32, pred: option<Pred>) -> VecInt32
fn store_vector_int32(ptr: *int32, vec: VecInt32, pred: option<Pred>) -> unit

// Arithmetic
fn add_vectors_int32(a: VecInt32, b: VecInt32) -> VecInt32
fn mul_vectors_int32(a: VecInt32, b: VecInt32) -> VecInt32

// Predicate operations
fn create_pred_true(len: int) -> Pred
fn create_pred_from_mask(mask: VecBool) -> Pred
fn test_any_true(pred: Pred) -> bool
fn test_all_true(pred: Pred) -> bool

// Reductions
fn reduce_add_vector_int32(vec: VecInt32, pred: Pred) -> int32
fn reduce_max_vector_int32(vec: VecInt32, pred: Pred) -> int32
fn reduce_min_vector_int32(vec: VecInt32, pred: Pred) -> int32

// Compression
fn compress_vector_int32(vec: VecInt32, pred: Pred) -> VecInt32
fn count_matches(pred: Pred) -> int
```

### 5. Memory Tagging Extensions (MTE) - Trait-Based

```silica
// Built-in MTE types - trait-based design

// Trait for types that can be tagged (marker trait, no methods)
trait TaggedElement {
    // Marker trait - indicates type can be in tagged buffers
}

// All types can be tagged
impl TaggedElement for int;
impl TaggedElement for int64;
impl TaggedElement for NodeData;
impl TaggedElement for Edge;

// Tagged pointer - trait-based
trait TaggedPointer {
    fn get_tag(self: Self) -> int;
    fn set_tag(self: Self, tag: int) -> Self;
    fn check_tag(self: Self) -> bool;
}

// Concrete tagged pointer types
type TaggedPtrInt
type TaggedPtrInt64
type TaggedPtrNodeData
type TaggedPtrEdge

impl TaggedPointer for TaggedPtrInt {
    fn get_tag(self: TaggedPtrInt) -> int { /* hardware read */ }
    fn set_tag(self: TaggedPtrInt, tag: int) -> TaggedPtrInt { /* hardware write */ }
    fn check_tag(self: TaggedPtrInt) -> bool { /* hardware check */ }
}

// Tagged buffer - trait-based with concrete implementations
trait TaggedBuffer {
    // Trait for tagged buffer operations
    // Each concrete type implements this with its specific element type
}

// Concrete tagged buffer types
type TaggedBufInt
type TaggedBufInt64
type TaggedBufNodeData
type TaggedBufEdge

impl TaggedBuffer for TaggedBufInt;

// Concrete operations for each type
fn read_tagged_buf_int(buf: TaggedBufInt, index: int) -> proc[mem(normal)] int
fn write_tagged_buf_int(buf: TaggedBufInt, index: int, value: int) -> proc[mem(normal)] unit

fn read_tagged_buf_node_data(buf: TaggedBufNodeData, index: int) -> proc[mem(normal)] NodeData
fn write_tagged_buf_node_data(buf: TaggedBufNodeData, index: int, value: NodeData) -> proc[mem(normal)] unit

// Allocation operations
fn alloc_tagged_int(size: int) -> proc[mem(normal)] TaggedPtrInt
fn alloc_tagged_buf_int(size: int, capacity: int) -> proc[mem(normal)] TaggedBufInt
fn free_tagged_int(ptr: TaggedPtrInt) -> proc[mem(normal)] unit
```

### 6. Pointer Authentication (PAC) - Trait-Based

```silica
// Built-in PAC types - trait-based design

// Trait for types that can be authenticated (marker trait)
trait Authenticatable {
    // Marker trait - no methods
}

// Reference types can be authenticated
// (Note: ref types themselves implement this conceptually)

// Authenticated pointer - trait-based
trait AuthenticatedPointer {
    fn auth_fail(self: Self, context: int) -> bool;
    // auth() not in trait since return types differ per implementation
}

// Concrete authenticated pointer types
type PacPtrInt
type PacPtrNodeData
type PacPtrEdge

impl AuthenticatedPointer for PacPtrInt {
    fn auth_fail(self: PacPtrInt, context: int) -> bool {
        // Check if authentication would fail
    }
}

// Concrete authentication operations (different return types)
fn auth_ptr_int(ptr: PacPtrInt, context: int) -> proc[mem(normal)] ref(R, normal, int) {
    // Hardware validates signature
}

fn auth_ptr_node_data(ptr: PacPtrNodeData, context: int) 
    -> proc[mem(normal)] ref(R, normal, NodeData) {
    // Hardware validates signature
}

// Signing operations
fn sign_ptr_int(ptr: ref(R, Space, int), context: int) -> PacPtrInt
fn sign_ptr_node_data(ptr: ref(R, Space, NodeData), context: int) -> PacPtrNodeData
```

### 7. Prefixed Pointers - Trait-Based

```silica
// Built-in prefixed pointer - trait-based design

// Trait for prefixed pointers
trait PrefixedPointer {
    fn get_prefix(self: Self) -> int;
    // deref_prefixed returns different types for each implementation
}

// Trait for types that can be prefixed (marker trait)
trait PrefixedElement {
    // Marker trait - no methods
}

impl PrefixedElement for int;
impl PrefixedElement for NodeData;
impl PrefixedElement for Edge;

// Concrete prefixed pointer types
type PrefixedPtrInt = {
    prefix: int,
    ptr: ref(R, Space, int)
}

type PrefixedPtrNodeData = {
    prefix: int,
    ptr: ref(R, Space, NodeData)
}

impl PrefixedPointer for PrefixedPtrInt {
    fn get_prefix(self: PrefixedPtrInt) -> int {
        self.prefix
    }
}

// Concrete dereference operations (not in trait, since return types differ)
fn deref_prefixed_int(pptr: PrefixedPtrInt) -> proc[mem(Space)] int {
    // Hardware validates prefix
}

fn deref_prefixed_node_data(pptr: PrefixedPtrNodeData) -> proc[mem(Space)] NodeData {
    // Hardware validates prefix
}

// Operations
fn create_prefixed_int(ptr: ref(R, Space, int), prefix: int) 
    -> proc[mem(Space)] PrefixedPtrInt
fn update_prefixed_int(pptr: PrefixedPtrInt, new_ptr: ref(R, Space, int)) 
    -> proc[mem(Space)] PrefixedPtrInt
```

## Updated Graph Examples (Trait-Based)

### Example 1: SIMD Bulk Map (Trait-Based)

```silica
// All features built-in - no imports, no generics!

// Trait for SIMD operations - no generics, concrete types
trait SIMDOperation {
    fn apply_vec128_int32(self: Self, vec: Vec128Int32) -> Vec128Int32;
    fn apply_vec128_int64(self: Self, vec: Vec128Int64) -> Vec128Int64;
    fn apply_vec128_float32(self: Self, vec: Vec128Float32) -> Vec128Float32;
    // Add methods for other vector types as needed
}

// Example operation
type MultiplyOp = { factor: int32 };

impl SIMDOperation for MultiplyOp {
    fn apply_vec128_int32(self: MultiplyOp, vec: Vec128Int32) -> Vec128Int32 {
        factor_vec <- broadcast_128_int32(self.factor);
        mul_128_int32(vec, factor_vec)  // Built-in operation
    }
    fn apply_vec128_int64(self: MultiplyOp, vec: Vec128Int64) -> Vec128Int64 {
        factor_vec <- broadcast_128_int64(self.factor);
        mul_128_int64(vec, factor_vec)
    }
    fn apply_vec128_float32(self: MultiplyOp, vec: Vec128Float32) -> Vec128Float32 {
        factor_vec <- broadcast_128_float32(self.factor);
        mul_128_float32(vec, factor_vec)
    }
}

// Graph with concrete types
type TaggedGraph = {
    nodes: TaggedBufNodeData,  // Concrete type, not generic
    edges: TaggedBufEdge
}

// Recursive bulk map
fn bulk_map_recursive(graph: TaggedGraph, result: TaggedGraph,
                     op: SIMDOperation, i: int) 
    -> proc[mem(normal)] TaggedGraph {
    
    case i >= graph.node_count - 4 of {
        true -> {
            bulk_map_remainder(graph, result, op, i)
        };
        false -> {
            do
                    // Built-in NEON load - concrete type, no generics
                    nodes_vec <- load_128_int32(graph.node_data[i]);
                    
                    // Apply SIMD operation - trait method
                    result_vec <- op.apply_vec128_int32(nodes_vec);
                    
                    // Built-in NEON store - concrete function
                    store_128_int32(result.node_data[i], result_vec);
                
                // Recursive call
                bulk_map_recursive(graph, result, op, i + 4)
            end
        }
    }
}
```

### Example 2: MTE-Protected Graph (Trait-Based)

```silica
// Graph with MTE protection - trait-based

type TaggedGraph = {
    nodes: TaggedBufNodeData,  // Concrete tagged buffer type
    edges: TaggedBufEdge
}

fn access_tagged_node(graph: TaggedGraph, index: int) 
    -> proc[mem(normal)] NodeData {
    
    // Built-in MTE operation - concrete function
    read_tagged_buf_node_data(graph.nodes, index)
}

fn build_tagged_graph(node_count: int, edge_count: int) 
    -> proc[mem(normal)] TaggedGraph {
    
    do
        // Built-in tagged allocation - concrete types
        nodes <- alloc_tagged_buf_node_data(node_count);
        edges <- alloc_tagged_buf_edge(edge_count);
        
        TaggedGraph {
            nodes: nodes,
            edges: edges
        }
    end
}
```

### Example 3: PAC-Protected Graph (Trait-Based)

```silica
// Graph with PAC protection - trait-based

type SecureGraph = {
    nodes: PacPtrNodeData,  // Concrete authenticated pointer type
    context: int
}

fn access_secure_node(graph: SecureGraph, index: int) 
    -> proc[mem(normal)] NodeData {
    
    do
        // Built-in PAC authentication - concrete function
        node_array <- auth_ptr_node_data(graph.nodes, graph.context);
        
        // Now safe to access
        read_buf(node_array, index)
    end
}

fn create_secure_graph(nodes: ref(R, normal, NodeData)) 
    -> proc[mem(normal)] SecureGraph {
    
    do
        context <- generate_auth_context();
        
        // Built-in PAC signing - concrete type
        node_ptr <- sign_ptr_node_data(nodes, context);
        
        SecureGraph {
            nodes: node_ptr,
            context: context
        }
    end
}
```

## Design Benefits

### 1. No Generics
- ✅ All types are concrete
- ✅ Compiler knows exact types
- ✅ Better optimization opportunities

### 2. Trait-Based Abstraction
- ✅ Operations work through traits
- ✅ Can still abstract over types
- ✅ Type-safe polymorphism

### 3. Built-In Features
- ✅ No module imports needed
- ✅ Always available
- ✅ Part of the language

### 4. Performance
- ✅ Compiler knows exact types
- ✅ Can inline operations
- ✅ Maximum optimization

## Summary

**All chip features are built-in and trait-based (NO generics):**

- ✅ **NEON**: `Vec128Int32`, `Vec128Int64`, `Vec128Float32`, etc. - Concrete types
- ✅ **SVE**: `VecInt32`, `VecInt64`, `VecFloat32`, etc. - Concrete types
- ✅ **MTE**: `TaggedBufInt`, `TaggedBufNodeData`, etc. - Concrete types with marker traits
- ✅ **PAC**: `PacPtrInt`, `PacPtrNodeData`, etc. - Concrete types with operation traits
- ✅ **Prefixed**: `PrefixedPtrInt`, etc. - Concrete types with operation traits

**Key Design Points:**
- ✅ **No generics** - All types are concrete
- ✅ **No module imports** - Everything is built-in
- ✅ **Trait-based abstraction** - Operations work through traits when needed
- ✅ **Concrete functions** - Direct operations for each type
- ✅ **Marker traits** - For type classification (Vec128Element, TaggedElement, etc.)

**Result**: Fully trait-based design with no generic syntax, consistent with Silica's design philosophy!
