# Graph Representation Design (silica-compiler)

## 1. Purpose and scope

This document specifies graph representations that can be generated for Silica code without relying on custom type declarations. It is intended as input for compiler/code-generation tools that need to emit graph data structures and graph operations using only the current Silica surface:

- Inline structural record types: `{ field: Type, ... }`
- Tuples and tagged tuples where useful
- `List[T, Space]`
- Region handles, region references, and buffers: `region(R, Space)`, `ref(R, Space, T)`, `buf(R, Space, T, N)`
- `sequence proc[mem(Space)] ... produces pure ... end` for graph allocation and mutation during construction

Silica has no user-defined custom type names. The graph names in this document are therefore **design/generator names**, not Silica type aliases. A generator may use these names for emitted module names and function prefixes, but emitted Silica type positions must repeat the full inline structural type.

This document covers three primary families:

1. `NodeIdAdjacencyGraph` - adjacency lists keyed by integer node ids.
2. `CompressedSparseRowGraph` - packed CSR buffers.
3. `DenseMatrixGraph` / `DenseBitsetGraph` - dense adjacency storage.

The families correspond to the practical graph forms that fit Silica today while avoiding named recursive node types.

## 2. Shared model

### 2.1 Node identity

All graph families use `int64` node ids. Node ids are values, not pointers.

Recommended invariant:

```text
0 <= node_id < node_count
```

Generators should prefer contiguous node ids because they make CSR and dense graphs straightforward and make adjacency-list validation cheap.

If source data uses non-contiguous ids, generate a normalization pass outside the graph representation:

```text
external id -> dense int64 id
dense int64 id -> external id
```

Until a map type exists, the normalization table should be a sorted `List[{ external_id: int64, dense_id: int64 }, S]` for small inputs or a generated direct buffer when the external id range is bounded.

### 2.2 Directedness

Directedness is a design-time property. Do not store it as a runtime boolean unless the caller needs one function to operate on both directed and undirected data.

Generator names:

| Name suffix | Meaning |
|-------------|---------|
| `Directed` | `u -> v` is stored once. |
| `Undirected` | Each logical edge `{u, v}` is stored as both `u -> v` and `v -> u`, unless `u == v`. |

Example generated module prefixes:

```text
graph_adj_directed_
graph_adj_undirected_
graph_csr_directed_
graph_csr_undirected_
graph_dense_directed_
graph_dense_undirected_
```

### 2.3 Weightedness

Use separate representation names for weighted and unweighted graphs. This avoids nullable or variant edge payloads in tight traversal code.

Recommended weight type for first implementation: `int64`.

| Name suffix | Edge payload |
|-------------|--------------|
| `Unweighted` | Neighbor id only. |
| `WeightedInt64` | Neighbor id plus `int64` weight. |

### 2.4 Memory space

Every generated graph representation that allocates storage must carry a concrete memory space `S`.

Recommended spaces:

| Space | Use |
|-------|-----|
| `normal` | Default for compiler graphs, local analysis graphs, and most application graphs. |
| `normal_writethrough` | Producer-consumer graph state visible to other cores or DMA-like readers. |
| `normal_noncacheable` | Device-shared graph buffers. Rare for compiler data. |
| `atomic` | Shared mutable counters or coordination cells. Avoid for ordinary immutable graph topology. |

Graph construction or mutation must occur in `sequence proc[mem(S)]`. The same `S` must appear in `List[..., S]`, `region(R, S)`, `ref(R, S, T)`, and `buf(R, S, T, N)`.

### 2.5 Generated operation categories

Each graph family should expose operations in three groups.

Construction:

```text
empty / allocate
add_edge or fill_edge
freeze or finalize, if the representation has a mutable build stage
```

Inspection:

```text
node_count
edge_count
has_edge
out_degree
neighbors traversal
```

Algorithms:

```text
dfs or bfs where a queue/stack representation is available
topological pass for directed acyclic inputs
reachability
degree summaries
```

For the first generated code pass, prioritize construction plus inspection. Algorithms can then be generated over a stable traversal API.

## 3. `NodeIdAdjacencyGraph`

### 3.1 Summary

`NodeIdAdjacencyGraph` stores one node record per node id. Each node record stores a list of outgoing neighbors. This is the clearest general graph representation in current Silica because it uses inline records and `List[T, S]` rather than custom recursive node types.

Use it when:

- The graph is small to medium sized.
- The graph is built functionally with `prepend`.
- Human-readable generated code matters.
- You want direct expression of "node has neighbors".
- You need a flexible intermediate representation before lowering to CSR.

Avoid it when:

- Graph traversal is performance critical.
- Random `has_edge(u, v)` needs to be fast.
- The graph is very large.
- Node ids are dense and the graph will be traversed many times. Use CSR instead.

### 3.2 Unweighted shape

Design name:

```text
NodeIdAdjacencyGraphDirectedUnweighted[S]
```

Silica inline shape:

```silica
{
    node_count: int64,
    edge_count: int64,
    nodes: List[
        {
            id: int64,
            neighbors: List[int64, S]
        },
        S
    ]
}
```

Example concrete shape for `normal`:

```silica
{
    node_count: int64,
    edge_count: int64,
    nodes: List[
        {
            id: int64,
            neighbors: List[int64, normal]
        },
        normal
    ]
}
```

### 3.3 Weighted shape

Design name:

```text
NodeIdAdjacencyGraphDirectedWeightedInt64[S]
```

Silica inline shape:

```silica
{
    node_count: int64,
    edge_count: int64,
    nodes: List[
        {
            id: int64,
            neighbors: List[
                { to: int64, weight: int64 },
                S
            ]
        },
        S
    ]
}
```

### 3.4 Invariants

Generators should emit validation helpers that check:

```text
graph.node_count >= 0
graph.edge_count >= 0
every node.id is in [0, node_count)
every neighbor id is in [0, node_count)
node ids appear at most once
edge_count equals total neighbor count for directed graphs
edge_count equals logical edge count for undirected graphs if the graph stores logical count
```

For generated undirected adjacency graphs:

```text
for every u -> v, v -> u should also exist, unless u == v
```

Because list scans are linear, validators are intended for build-time or debug-time use, not hot traversal.

### 3.5 Construction strategy

Recommended construction from an edge list:

1. Generate an initial node list containing `node_count` records with empty neighbor lists.
2. For each directed edge `{from, to}`, rebuild the node list with `to` prepended to the `from` node's neighbor list.
3. For undirected graphs, also insert `{to, from}` when `from != to`.
4. Return a new graph record with updated `edge_count`.

This construction is simple but O(node_count * edge_count) because updating one node requires scanning/rebuilding the node list. That is acceptable for small graphs and code-generation bootstrap use, but not for large graphs.

Generated function signatures should be monomorphic. For `normal` unweighted directed graphs:

```silica
fn graph_adj_directed_unweighted_normal_empty(
    node_count: int64
) -> {
    node_count: int64,
    edge_count: int64,
    nodes: List[{ id: int64, neighbors: List[int64, normal] }, normal]
} {
    sequence proc[mem(normal)]
        ...
    produces
        pure graph
    end
}
```

Add edge:

```silica
fn graph_adj_directed_unweighted_normal_add_edge(
    graph: {
        node_count: int64,
        edge_count: int64,
        nodes: List[{ id: int64, neighbors: List[int64, normal] }, normal]
    },
    from_id: int64,
    to_id: int64
) -> {
    node_count: int64,
    edge_count: int64,
    nodes: List[{ id: int64, neighbors: List[int64, normal] }, normal]
} {
    sequence proc[mem(normal)]
        ...
    produces
        pure updated_graph
    end
}
```

Weighted add edge adds `weight: int64` and prepends `{ to: to_id, weight: weight }`.

### 3.6 Traversal strategy

Generated traversal should expose neighbor lists directly:

```text
neighbors(graph, id) -> List[int64, S]
weighted_neighbors(graph, id) -> List[{ to: int64, weight: int64 }, S]
```

Implementation scans `graph.nodes` until it finds `node.id == id`.

`has_edge(graph, from_id, to_id)`:

1. `neighbors(graph, from_id)`
2. scan list for `to_id`

Cost:

```text
neighbors: O(node_count)
has_edge: O(node_count + out_degree(from_id))
```

### 3.7 Code-generation notes

The generator should emit one concrete family per:

```text
directedness x weightedness x memory space
```

Function prefix format:

```text
graph_adj_<directedness>_<weightedness>_<space>_
```

Examples:

```text
graph_adj_directed_unweighted_normal_empty
graph_adj_directed_unweighted_normal_add_edge
graph_adj_directed_unweighted_normal_neighbors
graph_adj_undirected_weighted_int64_normal_add_edge
```

Do not generate a type alias. Repeat the inline structural type in each signature.

## 4. `CompressedSparseRowGraph`

### 4.1 Summary

`CompressedSparseRowGraph` stores all outgoing adjacency in contiguous buffers. For node `u`, outgoing neighbors occupy:

```text
neighbors[offsets[u] ... offsets[u + 1] - 1]
```

This is the recommended representation for generated compiler graphs, dependency graphs, control-flow graphs, and other dense-id sparse graphs once the graph is built.

Use it when:

- Node ids are dense: `0..node_count - 1`.
- The graph is sparse.
- Traversal performance matters.
- The graph is built once and queried many times.
- You need compact memory and predictable scans.

Avoid it when:

- The graph changes frequently after construction.
- Node ids are sparse and cannot be normalized cheaply.
- You need ergonomic handwritten graph code.

### 4.2 Unweighted shape

Design name:

```text
CompressedSparseRowGraphDirectedUnweighted[R, S, N_PLUS_ONE, M]
```

Silica inline shape:

```silica
{
    region: region(R, S),
    node_count: int64,
    edge_count: int64,
    offsets: buf(R, S, int64, N_PLUS_ONE),
    neighbors: buf(R, S, int64, M)
}
```

`N_PLUS_ONE` must be `node_count + 1`.

`M` must be the number of stored directed edges. For undirected graphs, `M` is normally twice the number of non-loop logical edges plus one per self-loop.

Concrete `normal` shape:

```silica
{
    region: region(R, normal),
    node_count: int64,
    edge_count: int64,
    offsets: buf(R, normal, int64, N_PLUS_ONE),
    neighbors: buf(R, normal, int64, M)
}
```

### 4.3 Weighted shape

Design name:

```text
CompressedSparseRowGraphDirectedWeightedInt64[R, S, N_PLUS_ONE, M]
```

Silica inline shape:

```silica
{
    region: region(R, S),
    node_count: int64,
    edge_count: int64,
    offsets: buf(R, S, int64, N_PLUS_ONE),
    neighbors: buf(R, S, int64, M),
    weights: buf(R, S, int64, M)
}
```

Invariant:

```text
weights[i] is the weight for edge to neighbors[i]
```

### 4.4 Invariants

Generators should emit validation helpers that check:

```text
node_count >= 0
edge_count >= 0
offsets[0] == 0
offsets[node_count] == edge_count
offsets[i] <= offsets[i + 1] for every i in [0, node_count)
every neighbors[j] is in [0, node_count)
for weighted CSR, weights has capacity M and is indexed exactly like neighbors
```

If the generated algorithms require sorted adjacency lists, add:

```text
neighbors[k] <= neighbors[k + 1] inside each node's range
```

Do not require sorted neighbors by default; insertion order is often useful for compiler diagnostics.

### 4.5 Build strategy from edge list

CSR should usually be built in a construction phase. The simplest generator-friendly staged algorithm is:

Input:

```text
node_count: int64
edges: List[{ from: int64, to: int64 }, S]
```

For weighted:

```text
edges: List[{ from: int64, to: int64, weight: int64 }, S]
```

Build steps:

1. Allocate `offsets` with length `node_count + 1`.
2. Allocate a temporary `degree` buffer with length `node_count`.
3. Initialize `degree[i] = 0`.
4. Scan edges and increment `degree[from]`.
5. Prefix-sum degrees into offsets:

```text
offsets[0] = 0
offsets[i + 1] = offsets[i] + degree[i]
```

6. Allocate `neighbors` with length `edge_count`.
7. For weighted graphs, allocate `weights` with length `edge_count`.
8. Allocate a temporary `cursor` buffer with length `node_count`.
9. Initialize `cursor[i] = offsets[i]`.
10. Scan edges again:

```text
slot = cursor[from]
neighbors[slot] = to
weights[slot] = weight   // weighted only
cursor[from] = slot + 1
```

11. Return a graph record that contains the owning `region`, offsets, neighbors, and weights if present.

Construction requires mutation of buffers and therefore must run inside `sequence proc[mem(S)]`.

### 4.6 Direct fill strategy

For generated static graphs, a generator may skip edge-list staging and emit direct buffer writes:

1. Emit known `offsets`.
2. Emit known `neighbors`.
3. Emit known `weights`, if weighted.

This is preferred when the graph is generated from compile-time data because it avoids temporary degree/cursor buffers.

Generated constructor shape:

```silica
fn graph_csr_directed_unweighted_normal_from_static_edges() -> {
    region: region(R, normal),
    node_count: int64,
    edge_count: int64,
    offsets: buf(R, normal, int64, N_PLUS_ONE),
    neighbors: buf(R, normal, int64, M)
} {
    sequence proc[mem(normal)]
        R: lifetime <- fresh_lifetime();
        r: region(R, normal) <- alloc_region(normal);
        offsets: buf(R, normal, int64, N_PLUS_ONE) <- alloc_buf(r, N_PLUS_ONE);
        neighbors: buf(R, normal, int64, M) <- alloc_buf(r, M);
        _: atom <- write_buf(offsets, 0, 0);
        ...
        graph: {
            region: region(R, normal),
            node_count: int64,
            edge_count: int64,
            offsets: buf(R, normal, int64, N_PLUS_ONE),
            neighbors: buf(R, normal, int64, M)
        } <- {
            region: r,
            node_count: NODE_COUNT,
            edge_count: EDGE_COUNT,
            offsets: offsets,
            neighbors: neighbors
        };
    produces
        pure graph
    end
}
```

Note: `N_PLUS_ONE`, `M`, `NODE_COUNT`, and `EDGE_COUNT` are generator constants. The emitted Silica must use concrete numeric buffer sizes where the compiler requires them.

### 4.7 Traversal strategy

Out-degree:

```text
out_degree(g, u) = offsets[u + 1] - offsets[u]
```

Neighbor iteration:

```text
start = offsets[u]
end = offsets[u + 1]
for i from start to end - 1:
    v = neighbors[i]
```

Weighted neighbor iteration:

```text
v = neighbors[i]
w = weights[i]
```

Generated recursive traversal helper shape:

```text
walk_neighbors(g, u, i, end, acc)
```

`has_edge(g, u, v)` scans `neighbors[start..end)`. If sorted adjacency is guaranteed, generate binary search; otherwise generate linear scan.

Cost:

```text
out_degree: O(1)
neighbors traversal: O(out_degree(u))
has_edge unsorted: O(out_degree(u))
has_edge sorted: O(log out_degree(u))
```

### 4.8 Code-generation notes

Function prefix format:

```text
graph_csr_<directedness>_<weightedness>_<space>_
```

Examples:

```text
graph_csr_directed_unweighted_normal_from_static_edges
graph_csr_directed_unweighted_normal_out_degree
graph_csr_directed_unweighted_normal_has_edge
graph_csr_undirected_weighted_int64_normal_weight_at
```

The graph record must contain the owning region. Returning buffers without the region handle is invalid because the buffers would outlive their region.

CSR graph values are best treated as immutable after construction. If generated code mutates CSR buffers later, the function name should include `mutable` or `builder` so callers do not assume persistent immutable topology.

## 5. `DenseMatrixGraph`

### 5.1 Summary

`DenseMatrixGraph` stores one cell per possible directed edge. Cell `(from, to)` lives at:

```text
index = from * node_count + to
```

Use it when:

- The graph is dense.
- `has_edge(u, v)` must be O(1).
- `node_count` is small enough that `node_count * node_count` storage is acceptable.
- You need simple generated code and predictable indexing.

Avoid it when:

- The graph is sparse.
- `node_count * node_count` is too large.
- Traversal of outgoing neighbors dominates and most cells are empty. Use CSR.

### 5.2 Unweighted shape

Design name:

```text
DenseMatrixGraphDirectedUnweighted[R, S, N_TIMES_N]
```

Silica inline shape:

```silica
{
    region: region(R, S),
    node_count: int64,
    edge_count: int64,
    cells: buf(R, S, int64, N_TIMES_N)
}
```

Cell convention:

```text
0 = no edge
1 = edge exists
```

### 5.3 Weighted shape

Design name:

```text
DenseMatrixGraphDirectedWeightedInt64[R, S, N_TIMES_N]
```

Silica inline shape:

```silica
{
    region: region(R, S),
    node_count: int64,
    edge_count: int64,
    present: buf(R, S, int64, N_TIMES_N),
    weights: buf(R, S, int64, N_TIMES_N)
}
```

`present[index]` uses the same `0`/`1` convention. `weights[index]` is meaningful only when `present[index] == 1`.

Do not use `0` weight as "missing" unless the graph domain forbids zero-weight edges.

### 5.4 Invariants

Generators should emit validation helpers that check:

```text
node_count >= 0
edge_count >= 0
N_TIMES_N == node_count * node_count
every present/cell value is 0 or 1
edge_count equals the count of present edges for directed graphs
for undirected graphs, cell(u, v) == cell(v, u), and weights mirror when present
```

### 5.5 Construction strategy

Static generation:

1. Allocate `cells` or `present`/`weights`.
2. Initialize all cells to `0`.
3. For each edge, compute `index = from * node_count + to`.
4. Write `1` to presence.
5. For weighted graphs, write the weight.
6. For undirected graphs, also write the mirror edge when `from != to`.

Generated constructor shape:

```silica
fn graph_dense_directed_unweighted_normal_empty(
    node_count: int64
) -> {
    region: region(R, normal),
    node_count: int64,
    edge_count: int64,
    cells: buf(R, normal, int64, N_TIMES_N)
} {
    sequence proc[mem(normal)]
        R: lifetime <- fresh_lifetime();
        r: region(R, normal) <- alloc_region(normal);
        cells: buf(R, normal, int64, N_TIMES_N) <- alloc_buf(r, N_TIMES_N);
        ...
    produces
        pure { region: r, node_count: node_count, edge_count: 0, cells: cells }
    end
}
```

For dynamic `node_count`, code generation must either:

- choose a maximum capacity constant and store actual `node_count`, or
- wait until the compiler supports fully dynamic buffer sizes in type positions.

### 5.6 Traversal strategy

`has_edge(g, from_id, to_id)`:

```text
idx = from_id * g.node_count + to_id
read cells[idx] == 1
```

`out_degree(g, from_id)`:

```text
scan to_id from 0 to node_count - 1
count present cells
```

Neighbors traversal scans an entire matrix row:

```text
for to_id in 0..node_count:
    if has_edge(g, from_id, to_id):
        visit(to_id)
```

Cost:

```text
has_edge: O(1)
out_degree: O(node_count)
neighbors traversal: O(node_count)
storage: O(node_count * node_count)
```

## 6. `DenseBitsetGraph`

### 6.1 Summary

`DenseBitsetGraph` is the packed unweighted dense representation. It stores one bit per possible edge, packed into `int64` words.

Use it when:

- The graph is dense or moderately dense.
- The graph is unweighted.
- Memory footprint matters more than simple cell writes.
- Bulk set operations will eventually be useful.

Avoid it when:

- Weighted edges are needed.
- Simpler generated code is more important.
- Bit operations are not yet convenient in the target compiler phase.

### 6.2 Shape

Design name:

```text
DenseBitsetGraphDirectedUnweighted[R, S, WORD_COUNT]
```

Silica inline shape:

```silica
{
    region: region(R, S),
    node_count: int64,
    edge_count: int64,
    words: buf(R, S, int64, WORD_COUNT)
}
```

Constants:

```text
bit_count = node_count * node_count
WORD_COUNT = ceil(bit_count / 64)
```

Indexing:

```text
bit_index = from * node_count + to
word_index = bit_index / 64
bit_offset = bit_index % 64
mask = 1 << bit_offset
```

### 6.3 Operations

`set_edge(g, from, to)`:

```text
word = read_buf(words, word_index)
new_word = word | mask
write_buf(words, word_index, new_word)
```

`has_edge(g, from, to)`:

```text
(read_buf(words, word_index) & mask) != 0
```

Undirected insertion sets both bits.

### 6.4 Code-generation note

Only generate `DenseBitsetGraph` when the current compiler path supports the required bitwise operators (`|`, `&`, shift). If those are not available for the target stage, generate `DenseMatrixGraphDirectedUnweighted` instead. The matrix form has worse storage use but simpler generated code.

## 7. Choosing a graph representation

| Representation | Best for | Avoid when | Main cost |
|----------------|----------|------------|-----------|
| `NodeIdAdjacencyGraph` | Clear generated code, small/medium graphs, flexible build stage | Large hot graphs | Linear node lookup |
| `CompressedSparseRowGraph` | Sparse dense-id graphs, compiler IR graphs, repeated traversal | Frequent topology mutation | Build/finalize complexity |
| `DenseMatrixGraph` | Small dense graphs, O(1) edge test | Sparse or large graphs | O(N*N) storage |
| `DenseBitsetGraph` | Unweighted dense graphs with compact storage | Weighted graphs or missing bit ops | Bit manipulation complexity |

Recommended defaults:

```text
small and ergonomic: NodeIdAdjacencyGraph
sparse and performance-oriented: CompressedSparseRowGraph
dense and small: DenseMatrixGraph
dense, unweighted, and memory-sensitive: DenseBitsetGraph
```

For compiler internals, prefer:

```text
initial construction: NodeIdAdjacencyGraph or edge list
final analysis representation: CompressedSparseRowGraph
very small relation tables: DenseMatrixGraph
large unweighted relation closure: DenseBitsetGraph, once bit ops are ready
```

## 8. Generator requirements

### 8.1 Inputs

A graph code generator should take:

```text
representation: adjacency | csr | dense_matrix | dense_bitset
directedness: directed | undirected
weightedness: unweighted | weighted_int64
memory_space: normal | normal_writethrough | normal_noncacheable | atomic
node_count_known: bool
node_count: int64, when known
edge_count: int64, when known
sorted_neighbors: bool
module_prefix: string
```

CSR and dense buffer generators also require concrete buffer capacities:

```text
N_PLUS_ONE
M
N_TIMES_N
WORD_COUNT
```

### 8.2 Emitted function families

Every generated graph module should emit:

```text
empty or allocate
from_static_edges, when source edges are known
node_count
edge_count
has_edge
out_degree
neighbors traversal helper
validate, optional but recommended
```

Weighted graph modules should also emit:

```text
weight_at, returning an explicit found flag plus weight
```

Recommended return shape for `weight_at`:

```silica
{ found: bool, weight: int64 }
```

### 8.3 Error handling

Generated safe helpers should reject invalid node ids by returning tagged or record-shaped results rather than reading invalid buffers.

Recommended result shape:

```silica
{ ok: bool, value: int64 }
```

For boolean queries:

```silica
{ ok: bool, value: bool }
```

For traversal helpers where invalid ids are programmer errors, a generator may emit unchecked internal helpers and checked public wrappers:

```text
graph_csr_directed_unweighted_normal_has_edge
graph_csr_directed_unweighted_normal_has_edge_unchecked
```

### 8.4 Naming rules

Generated names should be deterministic:

```text
graph_<repr>_<directedness>_<weightedness>_<space>_<operation>
```

Where:

```text
repr = adj | csr | dense | bitset
directedness = directed | undirected
weightedness = unweighted | weighted_int64
space = normal | normal_writethrough | normal_noncacheable | atomic
```

Examples:

```text
graph_adj_directed_unweighted_normal_neighbors
graph_csr_directed_weighted_int64_normal_weight_at
graph_dense_undirected_unweighted_normal_has_edge
graph_bitset_directed_unweighted_normal_set_edge
```

### 8.5 Structural type emission

Because Silica has no custom type names, the generator must:

1. Build the full inline graph type string.
2. Reuse the exact same string in function parameters, returns, locals, and pattern annotations.
3. Keep field order stable for record types.
4. Use the same memory space everywhere in a graph family.
5. Include the owning region in every returned value that contains buffers or references.

Do not emit:

```silica
type Graph = ...
struct Graph { ... }
enum GraphKind { ... }
```

Instead emit:

```silica
fn use_graph(g: { node_count: int64, edge_count: int64, ... }) -> int64 {
    ...
}
```

## 9. Open implementation questions

1. Dynamic buffer sizes in type positions: CSR and dense graphs are easiest when capacities are generator constants. If dynamic buffer types become available, this document should add dynamic forms.
2. Bitwise operator coverage: `DenseBitsetGraph` depends on bit operations. Until those are uniformly available, dense matrix is the fallback.
3. Map support: if Silica gains a map/dictionary representation, adjacency lookup can become faster without CSR conversion.
4. Region lifetime ergonomics: graph records with buffers must carry the owning region. Future region-bundle ergonomics may reduce the amount of repeated inline type text.
5. Sorting support: sorted CSR adjacency enables binary search for `has_edge`; unsorted CSR preserves insertion order and simpler construction.

## 10. References

- [silica-specification.md](silica-specification.md) - inline structural types, lists, regions, effects.
- [list_implementation_design.md](list_implementation_design.md) - `List[T, S]` as region-backed storage and bundle model.
- [recursive_tuple_specification.md](recursive_tuple_specification.md) - why recursive pointer-shaped data uses inline `rec` and regions instead of named recursive types.
- [region_memory_safety_todo.md](region_memory_safety_todo.md) - current region safety implementation gaps.
- [atom_actor_registry_direct_index_design.md](atom_actor_registry_direct_index_design.md) - example of preferring direct buffers over a surface `Map`.
