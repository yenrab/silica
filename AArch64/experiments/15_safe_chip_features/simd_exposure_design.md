# Direct SIMD Exposure for Graph Operations

## Design Philosophy

**Explicit Control**: Silica's philosophy is explicit control over hardware features. We should expose direct SIMD access so users can:
1. **Optimize graph construction** with SIMD-accelerated building
2. **Write custom bulk operations** using direct chip access
3. **Compose high-level operations** from low-level primitives
4. **Maximum performance** when needed

**Immutability**: All operations return new structures - graphs are immutable. References/pointers are immutable values.

## Layered API Design

### Layer 1: Direct Chip Access (Low-Level)

Expose raw SIMD primitives from `arch.neon` and `arch.sve` modules:

```silica
use module arch.neon
use module arch.sve

// Direct access to chip functionality
// Users can write custom SIMD-optimized operations
```

### Layer 2: Graph-Specific SIMD Operations (Mid-Level)

Provide graph-optimized SIMD operations that leverage chip features:

```silica
trait GraphSIMDOps {
    // Construction operations
    fn simd_build_node_array(self: Self, data: buf(R, normal, int, N)) -> proc[mem(normal)] Self;
    fn simd_build_edge_array(self: Self, from: buf(R, normal, int, E), to: buf(R, normal, int, E)) -> proc[mem(normal)] Self;
    
    // Bulk operations with direct SIMD
    fn simd_map_nodes(self: Self, op: SIMDOperation) -> proc[mem(normal)] Self;
    fn simd_filter_nodes(self: Self, predicate: SIMDPredicate) -> proc[mem(normal)] Self;
    fn simd_reduce_nodes(self: Self, op: SIMDReduction) -> int;
}
```

### Layer 3: High-Level Convenience (High-Level)

Provide convenient bulk operations that use SIMD internally:

```silica
trait BulkTraversable includes Graph {
    fn bulk_map_nodes(self: Self, f: (int) -> int) -> proc[mem(normal)] Self;
    fn bulk_filter_nodes(self: Self, predicate: (int) -> bool) -> proc[mem(normal)] Self;
    fn bulk_reduce_nodes(self: Self, init: int, op: (int, int) -> int) -> int;
}
```

## Direct SIMD Access for Graph Construction

### 1. SIMD-Accelerated Graph Building

```silica
use module arch.neon

// Build graph using SIMD for efficient construction
fn build_graph_simd(node_data: buf(R, normal, int, N), 
                   edge_from: buf(R, normal, int, E),
                   edge_to: buf(R, normal, int, E))
    -> proc[mem(normal)] DenseGraphStructure {
    
    // Step 1: SIMD-accelerated node array initialization
    // Initialize node metadata in batches of 4
    node_metadata <- alloc_node_metadata(N);
    
    // Recursive helper for node initialization
    fn init_nodes_recursive(node_data: buf(R, normal, int, N),
                           node_metadata: buf(R, normal, int, N),
                           i: int) -> proc[mem(normal)] unit {
        case i >= N - 4 of {
            true -> {
                // Handle remainder
                init_nodes_remainder(node_data, node_metadata, i)
            };
            false -> {
                do
                    // Load 4 node values
                    nodes_vec <- neon.load_128(node_data[i]);
                    
                    // Initialize metadata for all 4 nodes in parallel
                    metadata_vec <- neon.broadcast_128(0);
                    neon.store_128(node_metadata[i], metadata_vec);
                    
                    // Recursive call for next batch
                    init_nodes_recursive(node_data, node_metadata, i + 4)
                end
            }
        }
    }
    
    init_nodes_recursive(node_data, node_metadata, 0);
    
    // Step 2: SIMD-accelerated edge index building
    // Count edges per node using vectorized operations
    edge_counts <- alloc_edge_counts(N);
    
    // Recursive helper for edge counting
    fn count_edges_recursive(edge_from: buf(R, normal, int, E),
                            edge_counts: buf(R, normal, int, N),
                            j: int) -> proc[mem(normal)] unit {
        case j >= E - 4 of {
            true -> {
                // Handle remainder
                count_edges_remainder(edge_from, edge_counts, j)
            };
            false -> {
                do
                    // Load 4 source nodes
                    from_vec <- neon.load_128(edge_from[j]);
                    
                    // For each source, increment count
                    // (This requires scatter operations - may need scalar fallback)
                    // But we can vectorize the counting logic
                    
                    // Recursive call for next batch
                    count_edges_recursive(edge_from, edge_counts, j + 4)
                end
            }
        }
    }
    
    count_edges_recursive(edge_from, edge_counts, 0);
    
    // Step 3: Build adjacency index using SIMD
    // Sort edges by source node, build index
    // SIMD can accelerate sorting and indexing
    
    // Return constructed graph
    DenseGraphStructure {
        node_data: node_data,
        edge_from: edge_from,
        edge_to: edge_to,
        node_edge_start: edge_starts,
        node_edge_count: edge_counts,
        simd_capable: true,
        vector_width: 4
    }
}
```

### 2. SIMD-Accelerated Edge Sorting

```silica
use module arch.neon

// Sort edges by source node using SIMD-accelerated sorting
fn simd_sort_edges_by_source(edges: EdgeList) -> proc[mem(normal)] EdgeList {
    // Use SIMD for comparison operations in sorting
    // Compare 4 pairs at once
    // Vectorized swap operations
    
    // Bitonic sort or other SIMD-friendly algorithms
    // ...
}
```

## Direct SIMD Access for Bulk Operations

### 1. Custom SIMD Operations Trait

```silica
// Trait for operations that can be vectorized
trait SIMDOperation {
    // NEON implementation
    fn apply_neon(self: Self, vec: Vec128<int>) -> Vec128<int>;
    
    // SVE implementation  
    fn apply_sve(self: Self, vec: Vec<int>, pred: Pred) -> Vec<int>;
    
    // Scalar fallback
    fn apply_scalar(self: Self, value: int) -> int;
}

// Example: Multiply operation
type MultiplyOp = { factor: int };

impl SIMDOperation for MultiplyOp {
    fn apply_neon(self: MultiplyOp, vec: Vec128<int>) -> Vec128<int> {
        factor_vec <- neon.broadcast_128(self.factor);
        neon.mul_128(vec, factor_vec)
    }
    
    fn apply_sve(self: MultiplyOp, vec: Vec<int>, pred: Pred) -> Vec<int> {
        factor_vec <- sve.broadcast_vector(self.factor);
        sve.mul_vectors(vec, factor_vec)
    }
    
    fn apply_scalar(self: MultiplyOp, value: int) -> int {
        value * self.factor
    }
}
```

### 2. Direct SIMD Map

```silica
use module arch.neon

// Direct SIMD map - user controls the operation
fn simd_map_direct(graph: DenseGraphStructure, op: SIMDOperation)
    -> proc[mem(normal)] DenseGraphStructure {
    
    result <- alloc_dense_graph(graph.node_count, graph.edge_count);
    
    // Recursive helper for SIMD map
    fn simd_map_recursive(graph: DenseGraphStructure, result: DenseGraphStructure,
                         op: SIMDOperation, i: int) 
        -> proc[mem(normal)] DenseGraphStructure {
        
        case i >= graph.node_count - 4 of {
            true -> {
                // Handle remainder with scalar
                simd_map_remainder(graph, result, op, i)
            };
            false -> {
                do
                    nodes_vec <- neon.load_128(graph.node_data[i]);
                    
                    // Apply user's SIMD operation
                    result_vec <- op.apply_neon(nodes_vec);
                    
                    neon.store_128(result.node_data[i], result_vec);
                    
                    // Recursive call for next batch
                    simd_map_recursive(graph, result, op, i + 4)
                end
            }
        }
    }
    
    simd_map_recursive(graph, result, op, 0)
    
    copy_edges(graph, result);
    result
}
```

### 3. Direct SIMD Filter with Predicates

```silica
use module arch.sve

// Direct SIMD filter using SVE predicates
fn simd_filter_direct(graph: DenseGraphStructure, predicate: SIMDPredicate)
    -> proc[mem(normal)] DenseGraphStructure {
    
    pred <- sve.create_pred_true(graph.node_count);
    nodes_vec <- sve.load_vector(graph.node_data, Some(pred));
    
    // User's predicate evaluated on all elements
    matches <- predicate.evaluate_sve(nodes_vec, pred);
    
    // SVE compress for efficient compaction
    filtered_vec <- sve.compress_vector(nodes_vec, matches);
    
    // Build result with filtered nodes
    count <- sve.count_matches(matches);
    result <- alloc_dense_graph(count, graph.edge_count);
    
    sve.store_vector(result.node_data, filtered_vec, Some(matches));
    
    // Copy matching edges
    copy_matching_edges(graph, result, matches);
    result
}
```

### 4. Direct SIMD Reduce

```silica
use module arch.sve

// Direct SIMD reduce using hardware reduction
fn simd_reduce_direct(graph: DenseGraphStructure, op: SIMDReduction) -> int {
    pred <- sve.create_pred_true(graph.node_count);
    nodes_vec <- sve.load_vector(graph.node_data, Some(pred));
    
    // Use hardware reduction instruction
    result <- op.reduce_sve(nodes_vec, pred);
    result
}

// Reduction operation trait
trait SIMDReduction {
    fn reduce_sve(self: Self, vec: Vec<int>, pred: Pred) -> int;
    fn reduce_neon(self: Self, vec: Vec128<int>) -> int;
    fn reduce_scalar(self: Self, a: int, b: int) -> int;
}

// Sum reduction
type SumReduction = {};

impl SIMDReduction for SumReduction {
    fn reduce_sve(self: SumReduction, vec: Vec<int>, pred: Pred) -> int {
        sve.reduce_add_vector(vec, pred)
    }
    
    fn reduce_neon(self: SumReduction, vec: Vec128<int>) -> int {
        // Horizontal add
        sum <- neon.hadd_128(vec);
        neon.extract_lane_128(sum, 0) + neon.extract_lane_128(sum, 1)
    }
    
    fn reduce_scalar(self: SumReduction, a: int, b: int) -> int {
        a + b
    }
}
```

## Graph Builder with SIMD

```silica
// Graph builder that uses SIMD for construction
trait SIMDGraphBuilder includes GraphBuilder {
    // SIMD-accelerated construction methods
    fn simd_add_nodes_batch(self: Self, data: buf(R, normal, int, N)) 
        -> proc[mem(normal)] Self;
    
    fn simd_add_edges_batch(self: Self, 
                            from: buf(R, normal, int, E),
                            to: buf(R, normal, int, E))
        -> proc[mem(normal)] Self;
    
    fn simd_build(self: Self) -> proc[mem(normal)] Graph;
}

impl SIMDGraphBuilder for DenseGraphBuilder {
    fn simd_add_nodes_batch(self: DenseGraphBuilder, data: buf(R, normal, int, N))
        -> proc[mem(normal)] DenseGraphBuilder {
        
        use module arch.neon
        
        // Recursive helper for adding nodes
        fn add_nodes_recursive(self: DenseGraphBuilder, 
                              data: buf(R, normal, int, N),
                              i: int) -> proc[mem(normal)] DenseGraphBuilder {
            case i >= N - 4 of {
                true -> {
                    // Handle remainder
                    add_nodes_remainder(self, data, i)
                };
                false -> {
                    do
                        // Load 4 nodes
                        nodes_vec <- neon.load_128(data[i]);
                        
                        // Store to graph structure
                        neon.store_128(self.node_buffer[self.node_count + i], nodes_vec);
                        
                        // Recursive call for next batch
                        add_nodes_recursive(self, data, i + 4)
                    end
                }
            }
        }
        
        do
            builder <- add_nodes_recursive(self, data, 0);
            builder.node_count <- builder.node_count + N;
            builder
        end
        
        self.node_count <- self.node_count + N;
        self
    }
    
    fn simd_add_edges_batch(self: DenseGraphBuilder,
                           from: buf(R, normal, int, E),
                           to: buf(R, normal, int, E))
        -> proc[mem(normal)] DenseGraphBuilder {
        
        use module arch.neon
        
        // Recursive helper for adding edges
        fn add_edges_recursive(self: DenseGraphBuilder,
                              from: buf(R, normal, int, E),
                              to: buf(R, normal, int, E),
                              j: int) -> proc[mem(normal)] DenseGraphBuilder {
            case j >= E - 4 of {
                true -> {
                    // Handle remainder
                    add_edges_remainder(self, from, to, j)
                };
                false -> {
                    do
                        // Load 4 source nodes
                        from_vec <- neon.load_128(from[j]);
                        // Load 4 target nodes
                        to_vec <- neon.load_128(to[j]);
                        
                        // Store to graph structure
                        neon.store_128(self.edge_from_buffer[self.edge_count + j], from_vec);
                        neon.store_128(self.edge_to_buffer[self.edge_count + j], to_vec);
                        
                        // Recursive call for next batch
                        add_edges_recursive(self, from, to, j + 4)
                    end
                }
            }
        }
        
        do
            builder <- add_edges_recursive(self, from, to, 0);
            builder.edge_count <- builder.edge_count + E;
            builder
        end
        
        self.edge_count <- self.edge_count + E;
        self
    }
}
```

## Usage Examples

### Example 1: Custom SIMD Operation

```silica
use module arch.neon

// User defines custom SIMD operation
type CustomMultiply = { factor: int, offset: int };

impl SIMDOperation for CustomMultiply {
    fn apply_neon(self: CustomMultiply, vec: Vec128<int>) -> Vec128<int> {
        factor_vec <- neon.broadcast_128(self.factor);
        offset_vec <- neon.broadcast_128(self.offset);
        
        // (vec * factor) + offset - all in SIMD!
        multiplied <- neon.mul_128(vec, factor_vec);
        neon.add_128(multiplied, offset_vec)
    }
    
    // ... SVE and scalar implementations
}

// Use it directly
graph <- simd_map_direct(graph, CustomMultiply { factor: 2, offset: 10 });
```

### Example 2: SIMD-Accelerated Construction

```silica
use module arch.neon

// Build graph with SIMD-accelerated construction
builder <- create_simd_graph_builder(1000, 5000);

// Add nodes in batches using SIMD
node_batch <- load_nodes_from_file("nodes.dat");
builder <- builder.simd_add_nodes_batch(node_batch);

// Add edges in batches using SIMD
edge_batch <- load_edges_from_file("edges.dat");
builder <- builder.simd_add_edges_batch(edge_batch.from, edge_batch.to);

// Build final graph (uses SIMD for index construction)
graph <- builder.simd_build();
```

### Example 3: High-Level with SIMD Under the Hood

```silica
// High-level API uses SIMD automatically
graph <- graph.bulk_map_nodes((x) -> x * 2);

// Internally calls:
// - Detects arithmetic operation
// - Creates SIMDOperation wrapper
// - Calls simd_map_direct with SIMD operation
// - Falls back to scalar if needed
```

## Benefits of Direct Exposure

### 1. Maximum Performance
- Users can write custom SIMD operations
- No abstraction overhead
- Direct access to chip features

### 2. Flexibility
- Custom operations not covered by high-level API
- Specialized optimizations for specific use cases
- Compose complex operations from primitives

### 3. Construction Optimization
- SIMD-accelerated graph building
- Batch operations for better performance
- Efficient memory initialization

### 4. Educational Value
- Users learn SIMD programming
- Understand hardware capabilities
- Build expertise in vectorization

## API Summary

```silica
// Layer 1: Direct chip access (arch.neon, arch.sve modules)
use module arch.neon
use module arch.sve

// Layer 2: Graph-specific SIMD operations
trait GraphSIMDOps {
    fn simd_build_node_array(...)
    fn simd_map_nodes(op: SIMDOperation)
    fn simd_filter_nodes(predicate: SIMDPredicate)
    fn simd_reduce_nodes(op: SIMDReduction)
}

// Layer 3: High-level convenience
trait BulkTraversable {
    fn bulk_map_nodes(f: (int) -> int)
    fn bulk_filter_nodes(predicate: (int) -> bool)
    fn bulk_reduce_nodes(init: int, op: (int, int) -> int)
}
```

## Conclusion

**YES** - Exposing direct chip functionality is the right approach:

1. ✅ **Graph Construction**: SIMD-accelerated building
2. ✅ **Bulk Operations**: Direct SIMD access for custom operations
3. ✅ **Layered Design**: Low-level control + high-level convenience
4. ✅ **Maximum Performance**: No abstraction overhead when needed
5. ✅ **Silica Philosophy**: Explicit control over hardware features

This gives users the power to optimize both construction and operations while maintaining convenient high-level APIs!
