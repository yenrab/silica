# Silica Graph Primitives Design

## Design Goals

1. **Minimal RAM usage** - Optimize memory footprint
2. **Maximum speed** - Leverage SIMD/NEON/SVE for bulk operations
3. **Functional immutability** - Graphs are immutable once created
4. **Bulk operations priority** - Optimize for batch processing
5. **Adaptive SIMD** - Support both NEON and SVE, adapt to available hardware
6. **Dense/Sparse separation** - Different implementations for different graph characteristics

## Immutability Requirements

**CRITICAL**: Silica is a functional language - all data structures are immutable.

**Key Rules**:
- ✅ **References/pointers are immutable values** - cannot be reassigned
- ✅ **Graphs are immutable** - all operations return new graphs
- ✅ **Buffers in graphs are read-only** - no write operations on graph buffers
- ✅ **All graph operations return new graphs** - original graph unchanged
- ✅ **Builder pattern** - mutable during construction only, immutable result

See `immutability_requirements.md` for complete immutability rules.

## Core Trait Hierarchy

```silica
// Base graph trait - no generics, uses trait composition
trait Graph {
    fn node_count(self: Self) -> int;
    fn edge_count(self: Self) -> int;
    fn get_node(self: Self, node_id: int) -> proc[mem(normal)] NodeData;
}

// Directed vs Undirected
trait DirectedGraph includes Graph {
    fn in_degree(self: Self, node_id: int) -> int;
    fn out_degree(self: Self, node_id: int) -> int;
    fn in_neighbors(self: Self, node_id: int) -> proc[mem(normal)] NeighborList;
    fn out_neighbors(self: Self, node_id: int) -> proc[mem(normal)] NeighborList;
}

trait UndirectedGraph includes Graph {
    fn degree(self: Self, node_id: int) -> int;
    fn neighbors(self: Self, node_id: int) -> proc[mem(normal)] NeighborList;
}

// Weighted graphs
trait WeightedGraph includes Graph {
    fn edge_weight(self: Self, from: int, to: int) -> proc[mem(normal)] int;
}

// Bulk operations - THE PRIORITY
trait BulkTraversable includes Graph {
    // Vectorized traversal operations
    fn bulk_map_nodes(self: Self, f: (NodeData) -> NodeData) -> proc[mem(normal)] Self;
    fn bulk_filter_nodes(self: Self, predicate: (NodeData) -> bool) -> proc[mem(normal)] Self;
    fn bulk_fold_nodes(self: Self, init: int, f: (int, NodeData) -> int) -> int;
}

trait BulkSearchable includes Graph {
    // Vectorized search operations
    fn bulk_find_nodes(self: Self, predicate: (NodeData) -> bool) -> proc[mem(normal)] NodeList;
    fn bulk_count_edges(self: Self, predicate: (int, int) -> bool) -> int;
    fn bulk_find_edges(self: Self, predicate: (int, int) -> bool) -> proc[mem(normal)] EdgeList;
}

// Dense vs Sparse specializations
trait DenseGraph includes Graph, BulkTraversable, BulkSearchable {
    // Optimized for high edge density
    // Uses contiguous arrays, perfect for SIMD
}

trait SparseGraph includes Graph, BulkTraversable, BulkSearchable {
    // Optimized for low edge density
    // Uses reference-based storage, vectorized edge batches
}

// Derived structures
trait Tree includes Graph {
    fn root(self: Self) -> int;
    fn is_acyclic(self: Self) -> bool;
    fn parent(self: Self, node_id: int) -> proc[mem(normal)] int;
    fn children(self: Self, node_id: int) -> proc[mem(normal)] NeighborList;
}

trait List includes Tree {
    fn is_linear(self: Self) -> bool;
    fn next(self: Self, node_id: int) -> proc[mem(normal)] int;
    fn prev(self: Self, node_id: int) -> proc[mem(normal)] int;
}
```

## Immutable Graph Construction

Since graphs are immutable, we use a builder pattern for construction:

```silica
// Graph builder - mutable during construction, immutable result
trait GraphBuilder {
    fn add_node(self: Self, data: NodeData) -> proc[mem(normal)] Self;
    fn add_edge(self: Self, from: int, to: int) -> proc[mem(normal)] Self;
    fn add_weighted_edge(self: Self, from: int, to: int, weight: int) -> proc[mem(normal)] Self;
    fn build(self: Self) -> proc[mem(normal)] Graph;  // Returns immutable graph
}

// Factory determines dense vs sparse based on edge density
fn create_graph_builder(node_count: int, estimated_edges: int) -> proc[mem(normal)] GraphBuilder {
    // Heuristic: if edges > (nodes * nodes / 4), use dense
    density <- estimated_edges * 4 / (node_count * node_count);
    if density > 1 {
        DenseGraphBuilder { nodes: node_count, edges: estimated_edges }
    } else {
        SparseGraphBuilder { nodes: node_count, edges: estimated_edges }
    }
}
```

## SIMD Adaptation Strategy

```silica
// Runtime SIMD capability detection
trait SIMDCapable {
    fn has_neon(self: Self) -> bool;
    fn has_sve(self: Self) -> bool;
    fn vector_width(self: Self) -> int;  // Returns 4 for NEON, variable for SVE
}

// Adaptive bulk operations
trait AdaptiveBulkOps includes BulkTraversable {
    // Automatically uses best available SIMD
    fn vectorized_map(self: Self, f: (NodeData) -> NodeData) -> proc[mem(normal)] Self;
    fn vectorized_filter(self: Self, predicate: (NodeData) -> bool) -> proc[mem(normal)] Self;
}

// Implementation strategy:
// 1. Check SIMD capabilities at graph creation time
// 2. Store capability flags in graph metadata
// 3. Use appropriate SIMD operations in bulk methods
```

## Dense Graph Implementation

**Memory Layout:**
```silica
type DenseGraphStructure = {
    node_count: int,
    edge_count: int,
    
    // Contiguous arrays - 16-byte aligned for SIMD
    node_data: buf(R, normal, NodeData, N),           // Node values
    edge_from: buf(R, normal, int, E),                // Source nodes (SIMD-aligned)
    edge_to: buf(R, normal, int, E),                  // Target nodes (SIMD-aligned)
    edge_weights: buf(R, normal, int, E),              // Weights (if weighted)
    
    // Adjacency index - O(1) lookup
    node_edge_start: buf(R, normal, int, N),          // Start index per node
    node_edge_count: buf(R, normal, int, N),          // Count per node
    
    // SIMD capabilities
    simd_capable: bool,
    vector_width: int  // 4 for NEON, variable for SVE
}

impl DenseGraph for DenseGraphStructure {
    // Bulk operations use SIMD
    fn bulk_map_nodes(self: DenseGraphStructure, f: (NodeData) -> NodeData) 
        -> proc[mem(normal)] DenseGraphStructure {
        
        if self.simd_capable {
            if self.vector_width == 4 {
                // Use NEON - process 4 nodes at a time
                self.neon_bulk_map(f)
            } else {
                // Use SVE - process vector_width nodes at a time
                self.sve_bulk_map(f)
            }
        } else {
            // Scalar fallback
            self.scalar_bulk_map(f)
        }
    }
}
```

**NEON Bulk Operations - Efficient Map:**

For arithmetic operations, we can truly vectorize:

```silica
// Built-in NEON types and operations - no module import needed!

// Note: BulkTraversable is implemented for buffer types, not element types
// Each buffer type (buf(R, normal, int32, N), buf(R, normal, Point, N), etc.)
// implements BulkTraversable with type-specific optimizations:
// - Numeric types: Full SIMD (4-16x)
// - Structs with numeric fields: Partial SIMD (2-4x)
// - Complex types: Vectorized memory access (1-2x)
// - Custom types: User-defined implementation

// Vectorizable function trait - functions that can be applied with SIMD
trait VectorizableOp {
    fn apply_vectorized(self: Self, vec: Vec128<int>) -> Vec128<int>;
}

// Example: Multiply by 2 - fully vectorized
impl VectorizableOp for MultiplyByTwo {
    fn apply_vectorized(self: MultiplyByTwo, vec: Vec128<int>) -> Vec128<int> {
        two_vec <- broadcast_128(2);  // Built-in: Replicate 2 to all lanes
        mul_128(vec, two_vec)          // Built-in: Multiply all 4 elements at once!
    }
}

fn neon_bulk_map_arithmetic(self: DenseGraphStructure, op: VectorizableOp) 
    -> proc[mem(normal)] DenseGraphStructure {
    
    // Recursive helper for arithmetic map
    fn map_recursive(self: DenseGraphStructure, result: DenseGraphStructure,
                    op: VectorizableOp, i: int) 
        -> proc[mem(normal)] DenseGraphStructure {
        
        case i >= self.node_count - 4 of {
            true -> {
                map_remainder(self, result, op, i)
            };
            false -> {
                do
                    // Built-in NEON load - 4 node values simultaneously
                    nodes_vec <- load_128(self.node_data[i]);
                    
                    // Apply operation to ALL 4 elements in parallel
                    result_vec <- op.apply_vectorized(nodes_vec);
                    
                    // Built-in NEON store - 4 results at once
                    store_128(result.node_data[i], result_vec);
                    
                    // Recursive call for next batch
                    map_recursive(self, result, op, i + 4)
                end
            }
        }
    }
    
    do
        result <- alloc_dense_graph(self.node_count, self.edge_count);
        map_recursive(self, result, op, 0)
    end
    
    // Handle remainder (scalar)
    while i < self.node_count {
        node <- read_buf(self.node_data, i);
        mapped <- op.apply_scalar(node);  // Fallback for remainder
        // Note: result is a NEW graph being constructed
        // write_buf is used during construction, not on existing graphs
        write_buf(result.node_data, i, mapped);
        i <- i + 1;
    }
    
    copy_edges(self, result);
    result
}

// Performance: 4x faster for arithmetic operations!
// Memory: Single load/store per 4 elements (cache-friendly)
```

**For non-arithmetic functions**, we still get benefits from vectorized loads/stores:
```silica
fn neon_bulk_map_generic(self: DenseGraphStructure, f: (int) -> int) 
    -> proc[mem(normal)] DenseGraphStructure {
    
    result <- alloc_dense_graph(self.node_count, self.edge_count);
    
    // Process nodes in batches of 4
    i <- 0;
    while i < self.node_count - 4 {
        // Vectorized load (4 elements at once)
        nodes_vec <- neon.load_128(self.node_data[i]);
        
        // Extract, apply function, insert (scalar function application)
        // Still faster than scalar loads due to cache efficiency
        node0 <- neon.extract_lane_128(nodes_vec, 0);
        node1 <- neon.extract_lane_128(nodes_vec, 1);
        node2 <- neon.extract_lane_128(nodes_vec, 2);
        node3 <- neon.extract_lane_128(nodes_vec, 3);
        
        mapped0 <- f(node0);
        mapped1 <- f(node1);
        mapped2 <- f(node2);
        mapped3 <- f(node3);
        
        result_vec <- neon.insert_lane_128(nodes_vec, 0, mapped0);
        result_vec <- neon.insert_lane_128(result_vec, 1, mapped1);
        result_vec <- neon.insert_lane_128(result_vec, 2, mapped2);
        result_vec <- neon.insert_lane_128(result_vec, 3, mapped3);
        
        // Vectorized store (4 elements at once)
        neon.store_128(result.node_data[i], result_vec);
        
        i <- i + 4;
    }
    
    // Handle remainder
    while i < self.node_count {
        node <- read_buf(self.node_data, i);
        mapped <- f(node);
        // Note: result is a NEW graph being constructed
        // write_buf is used during construction, not on existing graphs
        write_buf(result.node_data, i, mapped);
        i <- i + 1;
    }
    
    copy_edges(self, result);
    result
}

// Performance: ~2x faster due to vectorized memory operations
// Even non-arithmetic functions benefit from SIMD loads/stores
```

**SVE Bulk Operations:**
```silica
// Built-in SVE operations - no module import needed!

fn sve_bulk_map(self: DenseGraphStructure, f: (NodeData) -> NodeData) 
    -> proc[mem(normal)] DenseGraphStructure {
    
    do
        result <- alloc_dense_graph(self.node_count, self.edge_count);
        
        // Built-in SVE: automatically adapts to hardware vector width
        pred <- create_pred_true(self.node_count);
        
        // Built-in SVE: Load all nodes at once (scales with hardware!)
        nodes_vec <- load_vector(self.node_data, Some(pred));
        
        // Apply function (would need vectorized function, or extract/apply/insert)
        // For complex functions, process in chunks
        // ...
        
        // Built-in SVE: Store results
        store_vector(result.node_data, nodes_vec, Some(pred));
        
        copy_edges(self, result);
        result
    end
}
```

## Sparse Graph Implementation

**Memory Layout:**
```silica
type SparseGraphStructure = {
    node_count: int,
    edge_count: int,
    
    // Nodes as references (flexible)
    nodes: buf(R, normal, ref(R, normal, Node), N),
    
    // SIMD capabilities
    simd_capable: bool,
    vector_width: int
}

type Node = {
    data: NodeData,
    // Edges stored in batches of 4 (NEON) or vector_width (SVE)
    edge_batches: buf(R, normal, ref(R, normal, EdgeBatch), M),
    edge_count: int
}

// Edge batch - aligned for SIMD
type EdgeBatch = {
    targets: buf(R, normal, int, 4),      // 4 target node IDs (16-byte aligned)
    weights: buf(R, normal, int, 4),       // 4 weights (if weighted)
    count: int                             // Actual edges (1-4)
}

impl SparseGraph for SparseGraphStructure {
    fn bulk_find_nodes(self: SparseGraphStructure, predicate: (NodeData) -> bool) 
        -> proc[mem(normal)] NodeList {
        
        if self.simd_capable {
            if self.vector_width == 4 {
                self.neon_bulk_find(predicate)
            } else {
                self.sve_bulk_find(predicate)
            }
        } else {
            self.scalar_bulk_find(predicate)
        }
    }
}
```

**NEON Edge Operations:**
```silica
use module arch.neon

fn neon_bulk_find_edges(self: SparseGraphStructure, from_node: int) 
    -> proc[mem(normal)] EdgeList {
    
    node <- self.nodes[from_node];
    result <- alloc_edge_list();
    
    // Recursive helper for processing edge batches
    fn neon_bulk_find_edges_recursive(node: Node, result: EdgeList,
                                      from_node: int, batch_idx: int) 
        -> proc[mem(normal)] EdgeList {
        
        case batch_idx >= node.edge_batches.length of {
            true -> result;
            false -> {
                do
                    batch <- node.edge_batches[batch_idx];
                    
                    // Load 4 targets at once
                    targets_vec <- neon.load_128(batch.targets);
                    
                    // Compare all 4 with from_node simultaneously
                    from_vec <- neon.broadcast_128(from_node);
                    matches <- neon.compare_eq_128(targets_vec, from_vec);
                    
                    // Extract matching edges
                    new_result <- case neon.test_any_true(matches) of {
                        true -> {
                            // Add matching edges to result
                            add_matching_edges(result, batch, matches)
                        };
                        false -> result
                    };
                    
                    // Recursive call for next batch
                    neon_bulk_find_edges_recursive(node, new_result, from_node, batch_idx + 1)
                end
            }
        }
    }
    
    neon_bulk_find_edges_recursive(node, result, from_node, 0)
    
    result
}
```

## Efficient Bulk Operations: Map, Filter, Reduce

### 1. Map Operations

**Arithmetic Map (Fully Vectorized - 4x faster):**
```silica
// Multiply all nodes by 2 - TRUE SIMD vectorization
graph <- graph.bulk_map_nodes((node) -> node * 2);

// Implementation uses NEON/SVE arithmetic operations
// Processes 4-16 elements simultaneously
// Performance: O(N/vector_width) instead of O(N)
```

**Generic Map (Vectorized Memory - ~2x faster):**
```silica
// Apply arbitrary function - benefits from vectorized loads/stores
graph <- graph.bulk_map_nodes((node) -> complex_function(node));

// Even non-arithmetic functions benefit from:
// - Vectorized memory loads (4 elements at once)
// - Cache-friendly access patterns
// - Reduced memory bandwidth usage
```

### 2. Filter Operations (SVE Predicate-Based)

**Efficient Filtering with SVE:**
```silica
use module arch.sve

fn sve_bulk_filter_nodes(self: DenseGraphStructure, predicate: (int) -> bool) 
    -> proc[mem(normal)] DenseGraphStructure {
    
    // Create predicate for all nodes
    pred <- sve.create_pred_true(self.node_count);
    
    // Load all nodes
    nodes_vec <- sve.load_vector(self.node_data, Some(pred));
    
    // Evaluate predicate on ALL nodes simultaneously
    // SVE can compare all elements in parallel
    threshold <- 100;
    threshold_vec <- sve.broadcast_vector(threshold);
    matches <- sve.compare_gt_vector(nodes_vec, threshold_vec);
    
    // matches is a predicate mask - tells us which nodes pass
    // Count matches efficiently
    count <- sve.count_matches(matches);
    
    // Allocate result with exact size
    result <- alloc_dense_graph(count, self.edge_count);
    
    // Compress: extract only matching nodes using predicate
    // SVE has compress instructions for this!
    filtered_vec <- sve.compress_vector(nodes_vec, matches);
    sve.store_vector(result.node_data, filtered_vec, Some(matches));
    
    // Copy matching edges
    copy_matching_edges(self, result, matches);
    
    result
}

// Performance: O(N/vector_width) for predicate evaluation
// SVE compress handles variable-length output efficiently
// Much faster than scalar filter + compaction
```

**NEON Filter (Batch Processing):**
```silica
use module arch.neon

fn neon_bulk_filter_nodes(self: DenseGraphStructure, predicate: (int) -> bool) 
    -> proc[mem(normal)] DenseGraphStructure {
    
    // Process in batches of 4
    matches <- alloc_match_buffer(self.node_count);
    i <- 0;
    match_count <- 0;
    
    while i < self.node_count - 4 {
        // Load 4 nodes
        nodes_vec <- neon.load_128(self.node_data[i]);
        
        // Evaluate predicate on all 4 (if predicate is simple comparison)
        threshold <- 100;
        threshold_vec <- neon.broadcast_128(threshold);
        comparison <- neon.compare_gt_128(nodes_vec, threshold_vec);
        
        // Extract comparison results
        match0 <- neon.extract_lane_128(comparison, 0) != 0;
        match1 <- neon.extract_lane_128(comparison, 1) != 0;
        match2 <- neon.extract_lane_128(comparison, 2) != 0;
        match3 <- neon.extract_lane_128(comparison, 3) != 0;
        
        // Store matches in NEW result buffer (being constructed)
        // Note: matches is a new buffer being built, not mutating existing graph
        new_matches <- case match0 of {
            true -> {
                do
                    updated <- write_buf(matches, match_count, i);
                    (updated, match_count + 1)
                end
            };
            false -> (matches, match_count)
        };
        // Continue with match1, match2, match3...
        
        i <- i + 4;
    }
    
    // Handle remainder and build result
    // ...
}

// Performance: ~3x faster than scalar (predicate evaluation vectorized)
```

### 3. Reduce Operations (Tree Reduction)

**Efficient Reduction with Tree Pattern:**
```silica
use module arch.neon

fn neon_bulk_reduce_nodes(self: DenseGraphStructure, init: int, op: (int, int) -> int) 
    -> int {
    
    // Tree reduction pattern for parallel reduction
    
    // Recursive helper for partial reductions
    fn neon_bulk_reduce_partial(self: DenseGraphStructure, 
                                partial_results: buf(R, normal, int, N),
                                op: (int, int) -> int,
                                i: int, partial_idx: int) 
        -> proc[mem(normal)] (int, int) {
        
        case i >= self.node_count - 4 of {
            true -> {
                // Handle remainder
                do
                    remainder_sum <- neon_bulk_reduce_remainder(self, op, i, 0);
                    (partial_idx, remainder_sum)
                end
            };
            false -> {
                do
                    // Load 4 nodes
                    nodes_vec <- neon.load_128(self.node_data[i]);
                    
                    // Reduce 4 elements to 1 using SIMD
                    sum <- neon.hadd_128(nodes_vec);
                    total <- neon.extract_lane_128(sum, 0) + neon.extract_lane_128(sum, 1);
                    
                    // partial_results is a NEW buffer being constructed
                    // write_buf used during construction, not on existing graph
                    write_buf(partial_results, partial_idx, total);
                    
                    // Recursive call for next batch
                    neon_bulk_reduce_partial(self, partial_results, op, i + 4, partial_idx + 1)
                end
            }
        }
    }
    
    // Recursive helper for reducing partial results
    fn reduce_partials(partial_results: buf(R, normal, int, N),
                      op: (int, int) -> int,
                      init: int, j: int, count: int) -> int {
        case j >= count of {
            true -> init;
            false -> {
                partial <- read_buf(partial_results, j);
                new_init <- op(init, partial);
                reduce_partials(partial_results, op, new_init, j + 1, count)
            }
        }
    }
    
    do
        partial_results <- alloc_partial_results(self.node_count / 4);
        (partial_count, remainder_sum) <- neon_bulk_reduce_partial(self, partial_results, op, 0, 0);
        partial_sum <- reduce_partials(partial_results, op, init, 0, partial_count);
        op(partial_sum, remainder_sum)
    end
    
    final
}

// Performance: O(N/vector_width + log(partial_count))
// 4x faster initial reduction, then logarithmic combination
```

**SVE Reduction (Even Better):**
```silica
use module arch.sve

fn sve_bulk_reduce_nodes(self: DenseGraphStructure, init: int, op: (int, int) -> int) 
    -> int {
    
    pred <- sve.create_pred_true(self.node_count);
    nodes_vec <- sve.load_vector(self.node_data, Some(pred));
    
    // SVE has built-in reduction operations!
    // For sum:
    total <- sve.reduce_add_vector(nodes_vec, pred);
    
    // For max:
    // max_val <- sve.reduce_max_vector(nodes_vec, pred);
    
    // For min:
    // min_val <- sve.reduce_min_vector(nodes_vec, pred);
    
    op(init, total)
}

// Performance: O(N/vector_width) - single pass!
// Hardware does the reduction tree internally
```

### 4. Combined Operations (Map-Reduce Pattern)

```silica
// Efficient map-reduce: map then reduce
fn bulk_map_reduce(self: DenseGraphStructure, 
                   map_fn: (int) -> int, 
                   reduce_op: (int, int) -> int, 
                   init: int) -> int {
    
    // Option 1: Map first, then reduce (two passes)
    mapped_graph <- self.bulk_map_nodes(map_fn);
    result <- mapped_graph.bulk_reduce_nodes(init, reduce_op);
    result
}

// Or fused map-reduce (single pass, better cache usage)
fn bulk_fused_map_reduce(self: DenseGraphStructure,
                         map_fn: (int) -> int,
                         reduce_op: (int, int) -> int,
                         init: int) -> int {
    
    // Process in chunks: map chunk, reduce chunk, combine
    // Better cache locality than two-pass approach
    // ...
}
```

## Performance Summary

| Operation | Scalar | NEON | SVE | Speedup |
|-----------|--------|------|-----|---------|
| **Map (arithmetic)** | O(N) | O(N/4) | O(N/vw) | **4-16x** |
| **Map (generic)** | O(N) | O(N/2) | O(N/vw) | **2-8x** |
| **Filter** | O(N) | O(N/3) | O(N/vw) | **3-16x** |
| **Reduce** | O(N) | O(N/4 + log) | O(N/vw) | **4-16x** |
| **Map-Reduce** | O(2N) | O(N/2) | O(N/vw) | **4-16x** |

**Key Benefits:**
- ✅ **Map**: 2-16x faster depending on operation type
- ✅ **Filter**: 3-16x faster with SVE predicate operations
- ✅ **Reduce**: 4-16x faster with tree reduction patterns
- ✅ **Memory**: Vectorized loads/stores reduce bandwidth
- ✅ **Cache**: Contiguous access patterns maximize cache hits

## Memory Optimization Strategies

### Dense Graphs
- **Contiguous arrays**: Maximize cache locality
- **16-byte alignment**: Required for NEON, optimal for SVE
- **Interleaved vs Separate**: Store edge_from/edge_to separately for better SIMD access
- **Memory**: ~8 bytes/node + ~16 bytes/edge (minimal overhead)

### Sparse Graphs
- **Reference-based nodes**: Flexible, minimal waste
- **Edge batches**: 16-byte aligned batches of 4 edges
- **Adaptive batching**: Use larger batches for high-degree nodes
- **Memory**: ~24 bytes/node + ~16 bytes per 4-edge batch

## Performance Targets

### Bulk Operations (Priority)
- **Node mapping**: O(N/vector_width) - 4-16x faster than scalar
- **Edge filtering**: O(E/vector_width) - 4-16x faster than scalar
- **Bulk traversal**: O(N/vector_width) - 4-16x faster than scalar

### Memory
- **Dense**: <10% overhead vs raw arrays
- **Sparse**: <20% overhead vs minimal representation

## Direct SIMD Exposure

**Key Design Decision**: Expose direct chip functionality for maximum performance.

See `simd_exposure_design.md` for detailed design of:
- Direct SIMD access for graph construction
- Custom SIMD operations for bulk map/filter/reduce
- Layered API: low-level control + high-level convenience
- SIMD-accelerated graph builders

**Benefits**:
- ✅ SIMD-accelerated graph building
- ✅ Custom operations with direct chip access
- ✅ Maximum performance when needed
- ✅ Aligns with Silica's explicit control philosophy

## Built-In Chip Features (Trait-Based, No Generics)

**Key Design Decision**: All chip features (SIMD, NEON, SVE, MTE, PAC, Prefixed) are **built into the language**, not modules. **All types are concrete, no generics.**

See `built_in_chip_features.md` for complete trait-based design.

**Built-In Features (Trait-Based)**:
- ✅ **NEON**: `Vec128Int32`, `Vec128Int64`, `Vec128Float32`, etc. - Concrete types
- ✅ **SVE**: `VecInt32`, `VecInt64`, `VecFloat32`, etc. - Concrete types
- ✅ **MTE**: `TaggedBufInt`, `TaggedBufNodeData`, etc. - Concrete types with marker traits
- ✅ **PAC**: `PacPtrInt`, `PacPtrNodeData`, etc. - Concrete types with operation traits
- ✅ **Prefixed**: `PrefixedPtrInt`, etc. - Concrete types with operation traits

**No Generics, No Module Imports**: Everything is built-in and trait-based!

**Benefits**:
- ✅ Simpler: No imports, always available, no generic syntax
- ✅ Faster: Compiler knows exact types, can optimize aggressively
- ✅ Consistent: All AArch64 features treated equally
- ✅ Type-safe: Concrete types with trait-based abstraction

## Safe Memory Chip Features

**Key Design Decision**: Expose ALL safe memory chip behaviors, ZERO unsafe behaviors.

See `safe_memory_chip_features.md` for detailed design of:
- Memory Tagging Extensions (MTE) - hardware-accelerated bounds checking
- Pointer Authentication Codes (PAC) - cryptographic pointer signing
- Prefixed Pointers - metadata-validated pointers
- Region-based memory (already safe)

**Explicitly Excluded**:
- ❌ NO raw pointers (`*T` does not exist)
- ❌ NO pointer arithmetic
- ❌ NO unsafe blocks
- ❌ NO unsafe casts

**Benefits**:
- ✅ Hardware-accelerated safety (MTE, PAC, Prefixed)
- ✅ Zero-cost safety features
- ✅ No unsafe escape hatches
- ✅ Maximum safety with maximum performance

## Next Steps

1. Implement SIMD capability detection
2. Create dense graph builder with SIMD construction
3. Create sparse graph builder with SIMD construction
4. Implement direct SIMD operation traits
5. Implement NEON bulk operations
6. Implement SVE bulk operations
7. Add scalar fallbacks
8. Create tree/list specializations
9. Expose direct chip access API (see simd_exposure_design.md)
