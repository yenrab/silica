# Built-In Chip Features Design

## Design Philosophy

**AArch64-Native First**: Since Silica is designed specifically for AArch64, chip features should be **built into the language**, not optional modules.

**Rationale**:
- Silica targets AArch64 exclusively
- These features are core to the language's performance and safety
- No need for module imports - they're always available
- Compiler can optimize knowing these features exist
- Simpler mental model: "It's part of the language"

## Built-In Types and Operations

### 1. SIMD Vector Types (Built-In)

```silica
// Built-in vector types - always available
type Vec128<T>  // 128-bit NEON vector (4 × int32, 2 × int64, etc.)
type Vec<T>     // Scalable SVE vector (adapts to hardware)
type Pred       // SVE predicate mask

// Type constraints ensure only supported types
// Vec128 supports: int8, int16, int32, int64, float32
// Vec supports: int8, int16, int32, int64, float16, float32, float64
```

### 2. NEON Operations (Built-In)

```silica
// Built-in NEON operations - no module import needed

// Load/Store
fn load_128<T>(ptr: *T) -> Vec128<T>
fn store_128<T>(ptr: *T, vec: Vec128<T>) -> unit

// Arithmetic
fn add_128<T>(a: Vec128<T>, b: Vec128<T>) -> Vec128<T>
fn sub_128<T>(a: Vec128<T>, b: Vec128<T>) -> Vec128<T>
fn mul_128<T>(a: Vec128<T>, b: Vec128<T>) -> Vec128<T>

// Comparisons
fn compare_eq_128<T>(a: Vec128<T>, b: Vec128<T>) -> Vec128<bool>
fn compare_gt_128<T>(a: Vec128<T>, b: Vec128<T>) -> Vec128<bool>
fn compare_lt_128<T>(a: Vec128<T>, b: Vec128<T>) -> Vec128<bool>

// Lane operations
fn extract_lane_128<T>(vec: Vec128<T>, lane: int) -> T
fn insert_lane_128<T>(vec: Vec128<T>, lane: int, value: T) -> Vec128<T>
fn broadcast_128<T>(value: T) -> Vec128<T>

// Horizontal operations
fn hadd_128<T>(vec: Vec128<T>) -> Vec128<T>  // Horizontal add
fn test_any_true(vec: Vec128<bool>) -> bool
fn test_all_true(vec: Vec128<bool>) -> bool
```

### 3. SVE Operations (Built-In)

```silica
// Built-in SVE operations - no module import needed

// Load/Store with predicates
fn load_vector<T>(ptr: *T, pred: option<Pred>) -> Vec<T>
fn store_vector<T>(ptr: *T, vec: Vec<T>, pred: option<Pred>) -> unit

// Arithmetic
fn add_vectors<T>(a: Vec<T>, b: Vec<T>) -> Vec<T>
fn mul_vectors<T>(a: Vec<T>, b: Vec<T>) -> Vec<T>

// Predicate operations
fn create_pred_true(len: int) -> Pred
fn create_pred_from_mask(mask: Vec<bool>) -> Pred
fn test_any_true(pred: Pred) -> bool
fn test_all_true(pred: Pred) -> bool

// Reductions
fn reduce_add_vector<T>(vec: Vec<T>, pred: Pred) -> T
fn reduce_max_vector<T>(vec: Vec<T>, pred: Pred) -> T
fn reduce_min_vector<T>(vec: Vec<T>, pred: Pred) -> T

// Compression
fn compress_vector<T>(vec: Vec<T>, pred: Pred) -> Vec<T>
fn count_matches(pred: Pred) -> int
```

### 4. Memory Tagging Extensions (MTE) - Built-In

```silica
// Built-in MTE types and operations

// Tagged pointer type
type tagged_ptr<T>

// Tagged buffer type
type tagged_buf<T>

// Allocation
fn alloc_tagged<T>(size: int) -> proc[mem(normal)] tagged_ptr<T>
fn alloc_tagged_buf<T>(size: int, capacity: int) 
    -> proc[mem(normal)] tagged_buf<T>

// Deallocation
fn free_tagged<T>(ptr: tagged_ptr<T>) -> proc[mem(normal)] unit

// Tag operations
fn set_tag<T>(ptr: tagged_ptr<T>, tag: int) -> tagged_ptr<T>
fn get_tag<T>(ptr: tagged_ptr<T>) -> int
fn check_tag<T>(ptr: tagged_ptr<T>) -> bool  // Hardware check

// Tagged buffer operations
fn read_tagged_buf<T>(buf: tagged_buf<T>, index: int) 
    -> proc[mem(normal)] T  // Hardware validates tag + bounds

fn write_tagged_buf<T>(buf: tagged_buf<T>, index: int, value: T) 
    -> proc[mem(normal)] unit  // Hardware validates tag + bounds
```

### 5. Pointer Authentication (PAC) - Built-In

```silica
// Built-in PAC types and operations

// Authenticated pointer type
type pac_ptr<T>

// Authenticated function pointer type
type pac_fn_ptr<F>

// Signing
fn sign_ptr<T>(ptr: ref(R, Space, T), context: int) -> pac_ptr<T>
fn sign_function_ptr<F>(fn_ptr: F, context: int) -> pac_fn_ptr<F>

// Authentication
fn auth_ptr<T>(ptr: pac_ptr<T>, context: int) 
    -> proc[mem(Space)] ref(R, Space, T)  // Hardware validates

fn auth_call<F, Args, Ret>(fn_ptr: pac_fn_ptr<F>, args: Args) 
    -> proc[] Ret  // Hardware validates before call

// Validation check
fn auth_fail<T>(ptr: pac_ptr<T>, context: int) -> bool
```

### 6. Prefixed Pointers - Built-In

```silica
// Built-in prefixed pointer type
type prefixed_ptr<T> = {
    prefix: int,      // Metadata prefix (validated by hardware)
    ptr: ref(R, Space, T)  // Actual reference
}

// Operations
fn create_prefixed<T>(ptr: ref(R, Space, T), prefix: int) 
    -> proc[mem(Space)] prefixed_ptr<T>

fn deref_prefixed<T>(pptr: prefixed_ptr<T>) 
    -> proc[mem(Space)] T  // Hardware validates prefix

fn update_prefixed<T>(pptr: prefixed_ptr<T>, new_ptr: ref(R, Space, T)) 
    -> proc[mem(Space)] prefixed_ptr<T>
```

## Updated Graph Examples (No Module Imports)

### Example 1: SIMD Bulk Map (Built-In)

```silica
// No module import needed - SIMD is built-in!

fn ultra_fast_bulk_map(graph: TaggedGraph, op: SIMDOperation) 
    -> proc[mem(normal)] TaggedGraph {
    
    // Recursive helper for SIMD batch processing
    fn bulk_map_recursive(graph: TaggedGraph, result: TaggedGraph,
                         op: SIMDOperation, i: int) 
        -> proc[mem(normal)] TaggedGraph {
        
        case i >= graph.node_count - 4 of {
            true -> {
                // Handle remainder
                bulk_map_remainder(graph, result, op, i)
            };
            false -> {
                do
                    // Built-in NEON operations - no import needed!
                    nodes_vec <- load_128(graph.node_data[i]);
                    
                    // Apply SIMD operation
                    result_vec <- op.apply_neon(nodes_vec);
                    
                    // Built-in store
                    store_128(result.node_data[i], result_vec);
                    
                    // Recursive call for next batch
                    bulk_map_recursive(graph, result, op, i + 4)
                end
            }
        }
    }
    
    do
        result <- alloc_tagged_graph(graph.node_count, graph.edge_count);
        bulk_map_recursive(graph, result, op, 0)
    end
}
```

### Example 2: MTE-Protected Graph (Built-In)

```silica
// No module import needed - MTE is built-in!

type TaggedGraph = {
    nodes: tagged_buf<NodeData>,  // Built-in tagged buffer type
    edges: tagged_buf<Edge>
}

fn access_tagged_node(graph: TaggedGraph, index: int) 
    -> proc[mem(normal)] NodeData {
    
    // Built-in MTE operation - hardware validates automatically
    read_tagged_buf(graph.nodes, index)
}

fn build_tagged_graph(node_count: int, edge_count: int) 
    -> proc[mem(normal)] TaggedGraph {
    
    do
        // Built-in tagged allocation
        nodes <- alloc_tagged_buf<NodeData>(node_count);
        edges <- alloc_tagged_buf<Edge>(edge_count);
        
        TaggedGraph {
            nodes: nodes,
            edges: edges
        }
    end
}
```

### Example 3: PAC-Protected Graph (Built-In)

```silica
// No module import needed - PAC is built-in!

type SecureGraph = {
    nodes: pac_ptr<NodeArray>,  // Built-in authenticated pointer
    context: int
}

fn access_secure_node(graph: SecureGraph, index: int) 
    -> proc[mem(normal)] NodeData {
    
    do
        // Built-in PAC authentication
        node_array <- auth_ptr(graph.nodes, graph.context);
        
        // Now safe to access
        read_buf(node_array, index)
    end
}

fn create_secure_graph(nodes: ref(R, normal, NodeArray)) 
    -> proc[mem(normal)] SecureGraph {
    
    do
        context <- generate_auth_context();
        
        // Built-in PAC signing
        node_ptr <- sign_ptr(nodes, context);
        
        SecureGraph {
            nodes: node_ptr,
            context: context
        }
    end
}
```

### Example 4: Combined Features (All Built-In)

```silica
// All chip features built-in - no imports needed!

type UltraSafeGraph = {
    nodes: tagged_buf<NodeData>,      // MTE
    node_ptr: pac_ptr<NodeArray>,     // PAC
    edge_ptr: prefixed_ptr<EdgeArray>, // Prefixed
    context: int
}

fn maximum_performance_bulk_map(graph: UltraSafeGraph, op: SIMDOperation) 
    -> proc[mem(normal)] UltraSafeGraph {
    
    // Recursive helper
    fn bulk_map_recursive(node_array: NodeArray, result: UltraSafeGraph,
                         op: SIMDOperation, i: int) 
        -> proc[mem(normal)] UltraSafeGraph {
        
        case i >= node_array.length - 4 of {
            true -> {
                bulk_map_remainder(node_array, result, op, i)
            };
            false -> {
                do
                    // Built-in NEON load (hardware validates MTE tag in parallel)
                    nodes_vec <- load_128(node_array[i]);
                    
                    // Built-in SIMD operation
                    result_vec <- op.apply_neon(nodes_vec);
                    
                    // Built-in NEON store
                    store_128(result.node_data[i], result_vec);
                    
                    // Recursive call
                    bulk_map_recursive(node_array, result, op, i + 4)
                end
            }
        }
    }
    
    do
        // Built-in PAC authentication (once)
        node_ptr <- auth_ptr(graph.node_ptr, graph.context);
        
        // Built-in prefixed dereference (once)
        node_array <- deref_prefixed(node_ptr);
        
        // Built-in tagged graph allocation
        result <- alloc_ultra_safe_graph(graph.node_count, graph.edge_count);
        
        // SIMD operations with MTE validation in parallel
        bulk_map_recursive(node_array, result, op, 0)
    end
}
```

## Design Benefits

### 1. Simplicity
- No module imports needed
- Always available - part of the language
- Simpler mental model

### 2. Performance
- Compiler knows these features exist
- Can optimize more aggressively
- No runtime feature detection needed

### 3. Type Safety
- Built-in types are part of the type system
- Compiler enforces correct usage
- Better error messages

### 4. Consistency
- All AArch64 features treated equally
- No distinction between "core" and "optional"
- Unified design philosophy

## Compiler Behavior

### Feature Detection
- Compiler detects available hardware features at compile time
- Generates code for available features
- Provides fallbacks for missing features (if needed)

### Optimization
- Compiler can optimize knowing these features exist
- Can inline operations more aggressively
- Can eliminate redundant checks

### Code Generation
- Direct mapping to AArch64 instructions
- No abstraction overhead
- Maximum performance

## Migration from Module-Based Design

### Old (Module-Based)
```silica
use module arch.neon
use module arch.mte

fn example() {
    vec <- neon.load_128(ptr);
    buf <- mte.alloc_tagged_buf(100);
}
```

### New (Built-In)
```silica
// No imports needed!

fn example() {
    vec <- load_128(ptr);  // Built-in
    buf <- alloc_tagged_buf(100);  // Built-in
}
```

## Summary

**All chip features are now built-in language features:**

- ✅ **NEON**: `Vec128<T>`, `load_128()`, `store_128()`, etc.
- ✅ **SVE**: `Vec<T>`, `Pred`, `load_vector()`, `reduce_add_vector()`, etc.
- ✅ **MTE**: `tagged_ptr<T>`, `tagged_buf<T>`, `alloc_tagged()`, etc.
- ✅ **PAC**: `pac_ptr<T>`, `sign_ptr()`, `auth_ptr()`, etc.
- ✅ **Prefixed**: `prefixed_ptr<T>`, `create_prefixed()`, `deref_prefixed()`, etc.

**No module imports needed** - these are part of the core language!

**Result**: Simpler, faster, more consistent with Silica's AArch64-native design philosophy.
