# Immutability Requirements for Graph Primitives

## Core Principle

**Silica is a functional language - all data structures are immutable, including graphs.**

## Immutability Rules

### 1. References/Pointers Are Immutable Values

**References cannot be reassigned or modified:**
```silica
// ✅ CORRECT: References are immutable values
ref1: ref(R, normal, int) <- alloc_ref(region, 42);
// ref1 is an immutable value - cannot be reassigned

// ❌ WRONG: Cannot reassign references
// ref1 <- alloc_ref(region, 100);  // Does not exist in Silica

// ✅ CORRECT: Create new reference
ref2: ref(R, normal, int) <- alloc_ref(region, 100);
```

### 2. Graph Structures Are Immutable

**Graphs cannot be modified after creation:**
```silica
// ✅ CORRECT: All operations return new graphs
graph1: Graph <- build_graph();
graph2: Graph <- graph1.add_node(new_node);  // Returns NEW graph
graph3: Graph <- graph2.add_edge(from, to);   // Returns NEW graph

// ❌ WRONG: Cannot mutate existing graph
// graph1.add_node(new_node);  // Mutation not allowed
// graph1.nodes[0] <- new_value;  // Mutation not allowed
```

### 3. Buffer Operations in Graph Context

**For immutable graphs, buffers are read-only:**
```silica
// ✅ CORRECT: Read operations only
node <- read_buf(graph.nodes, index);
value <- read_tagged_buf_node_data(graph.nodes, index);

// ❌ WRONG: Write operations not allowed on immutable graphs
// write_buf(graph.nodes, index, new_value);  // Not allowed
// write_tagged_buf_node_data(graph.nodes, index, new_value);  // Not allowed
```

**Note**: `write_buf` and `write_tagged_buf` exist in Silica for mutable buffers, but **graphs use immutable buffers** - all modifications return new graphs with new buffers.

### 4. Tagged Pointer Operations

**Tag operations return new values (functional style):**
```silica
// ✅ CORRECT: set_tag returns new tagged pointer
tagged_ptr1: TaggedPtrInt <- alloc_tagged_int(100);
tagged_ptr2: TaggedPtrInt <- set_tag(tagged_ptr1, 5);  // Returns NEW pointer

// ❌ WRONG: Cannot mutate existing tagged pointer
// tagged_ptr1.set_tag(5);  // Mutation not allowed
```

### 5. Prefixed Pointer Operations

**All operations return new values:**
```silica
// ✅ CORRECT: update_prefixed returns new prefixed pointer
pptr1: PrefixedPtrInt <- create_prefixed_int(ptr, prefix1);
pptr2: PrefixedPtrInt <- update_prefixed_int(pptr1, new_ptr);  // Returns NEW pointer

// ❌ WRONG: Cannot mutate existing prefixed pointer
// pptr1.prefix <- new_prefix;  // Mutation not allowed
// pptr1.ptr <- new_ptr;  // Mutation not allowed
```

### 6. PAC Operations

**Authentication returns new reference (doesn't mutate):**
```silica
// ✅ CORRECT: auth_ptr returns new reference
pac_ptr: PacPtrInt <- sign_ptr_int(ptr, context);
new_ref: ref(R, normal, int) <- auth_ptr_int(pac_ptr, context);  // Returns NEW reference

// ❌ WRONG: Cannot mutate authenticated pointer
// pac_ptr.ptr <- new_ptr;  // Mutation not allowed
```

## Graph Operation Patterns

### Correct: Immutable Operations

```silica
// All graph operations return new graphs
fn add_node(graph: Graph, node: NodeData) -> proc[mem(normal)] Graph {
    // Create NEW graph with additional node
    // Original graph unchanged
}

fn add_edge(graph: Graph, from: int, to: int) -> proc[mem(normal)] Graph {
    // Create NEW graph with additional edge
    // Original graph unchanged
}

fn bulk_map_nodes(graph: Graph, op: SIMDOperation) -> proc[mem(normal)] Graph {
    // Create NEW graph with mapped nodes
    // Original graph unchanged
}
```

### Incorrect: Mutating Operations

```silica
// ❌ WRONG: These patterns don't exist in functional Silica
fn add_node_mutating(graph: Graph, node: NodeData) -> proc[mem(normal)] unit {
    // Cannot modify graph in place
    // graph.nodes[graph.node_count] <- node;  // Mutation not allowed
}

fn modify_node(graph: Graph, index: int, new_value: NodeData) -> proc[mem(normal)] unit {
    // Cannot modify existing node
    // write_buf(graph.nodes, index, new_value);  // Mutation not allowed
}
```

## Builder Pattern (Mutable During Construction Only)

**Graph builders are mutable during construction, but result is immutable:**
```silica
// Builder is mutable during construction
builder: GraphBuilder <- create_graph_builder(100, 500);
builder <- builder.add_node(node1);  // Builder state changes
builder <- builder.add_node(node2);  // Builder state changes
builder <- builder.add_edge(0, 1);   // Builder state changes

// build() returns IMMUTABLE graph
graph: Graph <- builder.build();  // Graph is now immutable

// After build(), graph cannot be modified
// graph.add_node(node3);  // Not allowed - graph is immutable
```

## Summary

**Key Points:**
- ✅ **References are immutable values** - cannot be reassigned
- ✅ **Graphs are immutable** - all operations return new graphs
- ✅ **Buffers in graphs are read-only** - no write operations
- ✅ **Tag/prefixed pointer operations return new values** - functional style
- ✅ **Builder pattern** - mutable during construction, immutable result

**Forbidden Patterns:**
- ❌ Reassigning references
- ❌ Mutating graph structures
- ❌ Writing to graph buffers
- ❌ Modifying pointers in place

**Result**: Fully functional, immutable graph design consistent with Silica's functional programming philosophy.
