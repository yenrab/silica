# Graph Representation Design (silica-compiler)

## 1. Purpose and scope

This document specifies graph representations that can be generated for Silica code without relying on custom type declarations. It is intended as input for compiler/code-generation tools that need to emit graph data structures and graph operations using only the current Silica surface:

- Inline structural record types: `{ field: Type, ... }`
- Tuples and tagged tuples where useful
- `List[T, Space]`
- Region handles, region references, and buffers: `region(R, Space)`, `ref(R, Space, T)`, `buf(R, Space, T, N)`
- `sequence proc[mem(Space)] ... produces pure ... end` for graph allocation and mutation during construction

Silica has no user-defined custom type names. Graph names in this document are therefore **design/generator names**, not Silica type aliases. The Phase 1 standard-structure model is trait-oriented: generated graph representations implement graph behavior traits, and generated constructors take inline typed function records as described in [Phase1_TODOs/data_structures_as_traits.md](Phase1_TODOs/data_structures_as_traits.md). **Emitted module filenames** use **representation + directedness only** (§2.2, §8.4) — for example `graph_adj_directed.silica`, not `graph_adj_directed_int64_mem_normal.silica`. Node id type, payload types, and **`mem(Space)`** are fixed by the declared collection type and checked through constructor function-record fields. Emitted Silica type positions must repeat the full inline structural type (or expand a compiler-known shorthand to that inline type).

**Typing model:** public graph behavior is generic in **`NodeIdType`** and **`EdgePayloadType`**. Dense physical slots, counts, capacities, and CSR offsets may remain **`int64`**, but user-visible node ids and edge payloads are not assumed to be `int64`. Generated graphs are **immutable values** with **uniform inline types** (§2.7–§2.8).

This document covers three primary families:

1. `NodeIdAdjacencyGraph` - adjacency lists keyed by integer node ids.
2. `CompressedSparseRowGraph` - packed CSR buffers.
3. `DenseMatrixGraph` / `DenseBitsetGraph` - dense adjacency storage.

The families correspond to the practical graph forms that fit Silica today while avoiding named recursive node types.

## 2. Shared model

### 2.1 Node identity

Graph behavior uses an explicit **`NodeIdType`**. Node ids are values, not pointers. They are compared by the constructor record's `compare_node: fn(NodeIdType, NodeIdType) -> atom` field.

For the bootstrap physical representations already implemented, `NodeIdType = int64` and the recommended dense-id invariant is:

```text
0 <= node_id < node_count
```

Generators should prefer contiguous dense ids when they can because they make CSR and dense graphs straightforward and make adjacency-list validation cheap. When a graph exposes non-`int64` node ids, the representation must either store those ids directly or maintain a generated mapping between `NodeIdType` values and internal dense `int64` slots.

If source data uses non-contiguous ids, generate a normalization pass outside the graph representation:

```text
external id -> dense int64 id
dense int64 id -> external id
```

Until a map type exists, the normalization table should be a sorted `List[{ external_id: NodeIdType, dense_id: int64 }, S]` for small inputs or a generated direct buffer when the external id range is bounded and `NodeIdType` supports that encoding.

### 2.2 Directedness

Directedness is a design-time property. Do not store it as a runtime boolean unless the caller needs one function to operate on both directed and undirected data.

Generator names:

| Name suffix | Meaning |
|-------------|---------|
| `Directed` | `u -> v` is stored once. |
| `Undirected` | Each logical edge `{u, v}` is stored as both `u -> v` and `v -> u`, unless `u == v`. |

Example generated **module names** (Silica filename without `.silica`):

```text
graph_adj_directed
graph_adj_undirected
graph_csr_directed
graph_csr_undirected
graph_dense_directed
graph_dense_undirected
graph_bitset_directed
graph_bitset_undirected
```

Do **not** encode node id type, payload type, weightedness, or memory space in the module name. These are fixed by each **graph value's inline type** and by the constructor function record (§2.11): one module per representation family; callers write `graph_adj_directed@empty({ ... })` under an explicitly typed graph binding.

### 2.3 Edge and node payload

Public topology uses **`NodeIdType`** (§2.1). **User payload** on nodes and edges uses the concrete types declared by the graph collection type. There is no separate graph storage marker trait; behavior is provided by graph traits and generated representation impls.

Use separate representation names when edge payload shape differs. This avoids nullable or variant edge payloads in tight traversal code.

| Name suffix | Edge topology | Edge payload (`EdgePayloadType`) |
|-------------|---------------|---------------------------|
| `Unweighted` | Neighbor id only. | None (no parallel payload buffer). |
| Edge payload | Neighbor id in `neighbors`. | `EdgePayloadType` in parallel `edge_data` (or list slot). Bootstrap examples use `int64`. |

Generators may emit other **`EdgePayloadType`** and **`NodeDataType`** specializations. The first edge-payload family uses **`EdgePayloadType = int64`** as a bootstrap specialization rather than a separate weightedness suffix.

Optional **`NodeDataType`** on each vertex is stored in adjacency node records and, for CSR/dense families, in a parallel **`node_data`** buffer indexed by internal dense slot.

### 2.4 Constructor function records and API operands

Generated graph constructors take an inline function record that witnesses graph-relevant types and behavior.

Directed graph constructor record:

```text
{
    compare_node: fn(NodeIdType, NodeIdType) -> atom,
    compare_edge: fn(EdgePayloadType, EdgePayloadType) -> atom,
    edge_target: fn(EdgePayloadType) -> NodeIdType
}
```

Weighted graph records add the weight comparator when weight is separate from the edge payload:

```text
{
    compare_node: fn(NodeIdType, NodeIdType) -> atom,
    compare_edge: fn(EdgePayloadType, EdgePayloadType) -> atom,
    compare_weight: fn(WeightType, WeightType) -> atom,
    edge_target: fn(EdgePayloadType) -> NodeIdType
}
```

The compiler checks these fields against the declared graph collection type. Comparator results are atoms with valid values `:less`, `:equal`, and `:greater`.

**Typed operands (abstract generator signatures):**

- **`NodeIdType`** — endpoints and lookup keys.
- **`NodeDataType`** — `set_node_data`, `get_node_data`, node record **`data`** fields.
- **`EdgePayloadType`** — `add_edge` edge payload, `edge_data_at`, weighted neighbor records, dense **`cell_data`**.
- **`WeightType`** — weighted graph values when stored separately from edge payload.

**Plain structural types:**

- **`node_count`**, **`edge_count`**, buffer capacities, generator constants, region handles.
- Dense physical slots, CSR offsets, and internal buffer indices.

Design-level prose may still use names **`NodeDataType`** / **`EdgePayloadType`**. In generated Silica, fields and function parameters use concrete inline types established by the declared collection type and constructor function record.

### 2.5 Topology vs payload storage

| Layer | CSR / dense | Adjacency |
|-------|-------------|-----------|
| **Topology** | `offsets`, `present` / bitset — **`int64` physical slots**; `neighbors` may store dense slots or `NodeIdType` values depending on representation | `neighbors: List[NodeIdType, S]` or `List[{ edge: EdgePayloadType, ... }, S]` |
| **Node payload** | `node_data: buf(R, S, NodeData, N)` | `data: NodeData` on each node record |
| **Edge payload** | `edge_data: buf(R, S, EdgePayloadType, M)` parallel to `neighbors` | `{ to: NodeIdType, data: EdgePayloadType }` or weight field in neighbor record |

Unweighted graphs omit **`edge_data`**. Graphs without vertex attributes omit **`node_data`** or use a generator **`none`** node-data mode.

Construction may build from **`List[EdgePayloadType, S]`** when `edge_target` extracts the destination, or from **`List[{ from: NodeIdType, edge: EdgePayloadType }, S]`** when source id is not implicit, then **freeze** into CSR/dense buffers (§4.5, §2.7).

### 2.6 Payload buffer encoding

**`buf(R, S, T, N)`** where **`T`** is a payload type uses the **same per-element encoding as `List[T, S]`** ([list_implementation_design.md](list_implementation_design.md) §4.1–§4.2, §9.2):

- Scalars and fixed packed compounds → **inline** in the buffer cell.
- Non-primitive values (strings, nested lists, structs with indirect fields) → **region indirection** inside the cell, with payload objects allocated under the graph's **owning region**.

Every graph value that contains buffers must carry the owning **`region(R, S)`** (§4.8). Payload and topology buffers share one region bundle.

### 2.7 Immutability and type invariance

Generated graphs are **immutable values**, analogous to **`List[T, S]`** ([silica-specification.md](silica-specification.md) §4.2.4).

**Value immutability:**

- **Adjacency:** every mutating operation (`add_edge`, `set_node_data`, …) returns a **new graph record** with `produces pure … end`. No in-place mutation of a graph value held by the caller.
- **CSR / dense:** immutable **after construction or `freeze`**. Public query APIs do not mutate topology or payload buffers.
- **Construction:** allocation and buffer writes occur only inside **`sequence proc[mem(S)]`**; the exported constructor returns a **pure** graph value.
- **Mutable builders:** only in modules whose names include **`_builder_`** or **`_mutable_`**. Default generated families are immutable.

**Type invariance (uniform graph types):**

The **same inline graph record type** must appear at formal parameters, arguments, locals, return types, and **`case`** patterns for one graph data flow — parallel to **uniform list types** (silica-spec §4.2.4). Mixing different inline graph shapes for the same value is ill-formed.

**Schema pinning:**

The graph type returned by **`empty`**, **`from_static_edges`**, or **`from_edges`** embeds concrete **`NodeData`**, **`EdgeData`**, and topology fields. That constructor return type is the schema anchor for subsequent **`add_edge`**, **`get_node_data`**, and inspection calls (see §2.8).

**Structural invariants** (id ranges, offset monotonicity, edge counts) are checked by optional **`validate`** helpers (§3.4, §8.2) at build or debug time, not by the type checker alone.

### 2.8 Type checking

Type safety for graph payload **reads and writes** is enforced through **structural typing** on the graph record ([silica-specification.md](silica-specification.md) §8.2.2) plus constructor function-record checking. Graph behavior itself is exposed through standard graph traits.

| Direction | Call site | Checked against |
|-----------|-----------|-----------------|
| **Write payload** | `add_edge(g, …, data)`, `set_node_data(g, id, data)` | **`EdgeData`** / **`NodeData`** fields embedded in **`g`**'s inline type |
| **Read payload** | `get_node_data(g, id).value`, `edge_data_at(g, slot).value` | Return **`value`** type extracted from the **`graph`** parameter type of the generated helper |
| **Topology** | `has_edge(g, from_id, to_id)`, `neighbors(g, id)` | **`from_id`**, **`to_id`**, **`id`** have **`NodeIdType`**; graph argument matches the helper's full inline graph parameter |
| **Constructor record** | `graph_adj_directed@empty({ ... })` | record fields have required function types, such as `compare_node: fn(NodeIdType, NodeIdType) -> atom` and `edge_target: fn(EdgePayloadType) -> NodeIdType` |

Generated helpers take the **full inline graph type** as their first parameter. The compiler verifies that the **`graph`** argument is structurally identical to that parameter and that payload bindings match the **`NodeData`** / **`EdgeData`** slots declared in the graph record (for example `node_data: buf(R, S, NodeData, N)`).

Optional **`graphpayload.silica`** registry entries (`impl { … full graph record … };`) may reuse a large inline graph shape without repeating it at every call site, following the same identity rules as **`actormessage.silica`** (silica-spec §16.3.2).

### 2.9 Memory space

Every generated graph representation that allocates storage must carry a concrete memory space `S`.

Recommended spaces:

| Space | Use |
|-------|-----|
| `normal` | Default for compiler graphs, local analysis graphs, and most application graphs. |
| `normal_writethrough` | Producer-consumer graph state visible to other cores or DMA-like readers. |
| `normal_noncacheable` | Device-shared graph buffers. Rare for compiler data. |
| `atomic` | Shared mutable counters or coordination cells. Avoid for ordinary immutable graph topology. |

Graph construction or mutation must occur in `sequence proc[mem(S)]`. The same `S` must appear in `List[..., S]`, `region(R, S)`, `ref(R, S, T)`, and `buf(R, S, T, N)`.

### 2.10 Generated operation categories

Each graph family should expose operations in three groups.

Construction:

```text
empty / allocate
add_edge or fill_edge
set_node_data, when NodeData is present
freeze or finalize, if the representation has a mutable build stage
```

Inspection:

```text
node_count
edge_count
has_edge
out_degree
neighbors traversal
get_node_data / edge_data_at, when payload buffers or fields are present
```

Algorithms:

```text
dfs or bfs where a queue/stack representation is available
topological pass for directed acyclic inputs
reachability
degree summaries
```

For the first generated code pass, prioritize construction plus inspection. Algorithms can then be generated over a stable traversal API.

### 2.11 Constructor records, traits, and call syntax

Standard generated graph families follow [Phase1_TODOs/data_structures_as_traits.md](Phase1_TODOs/data_structures_as_traits.md):

- **Preferred constructors:** one exported constructor per arity whose first argument is an inline function record. Example: `graph_adj_directed@empty({ compare_node: compare_node_id, compare_edge: compare_edge, edge_target: edge_to })` under an explicitly typed graph binding.
- **Generated updates:** representation module operations such as `add_edge` preserve the compare/extract functions captured at construction.
- **Behavior traits:** standard operations such as `node_count`, `edge_count`, `neighbors`, `has_edge`, and `reachable` are available through graph traits (`DirectedGraph@neighbors(g, node_id)`) and dispatch from the concrete generated receiver value.
- The **representation family name** (for example `NodeIdAdjacencyGraphDirected`) stays outside any function-record fields. **Directedness** is part of the family name, not a type argument.
- **Registry / design names** may still use bracket notation (`NodeIdAdjacencyGraphDirected[NodeIdType, EdgePayloadType, mem(S)]`) for documentation and generator keys; that is not a requirement to duplicate exported functions per payload type in one module.
- There is no user-defined generic polymorphism (silica-spec §1.2). The compiler **specializes** each value flow to concrete `NodeData` / `EdgeData` before codegen.

| Graph shape | Registry form | Constructor record fields |
|-------------|---------------|----------------------------|
| Unweighted, no vertex attributes | `[NodeIdType, mem(S)]` | `compare_node` |
| Edge payload only | `[NodeIdType, EdgePayloadType, mem(S)]` | `compare_node`, `compare_edge`, `edge_target` |
| Vertex and edge payload | `[NodeIdType, NodeDataType, EdgePayloadType, mem(S)]` | `compare_node`, `compare_edge`, `edge_target`, plus node-data functions when required |

**Not in brackets:** region id **`R`**, CSR/dense **buffer capacities** (`N_PLUS_ONE`, `M`, `N_TIMES_N`, `WORD_COUNT`), and runtime topology flags. Those remain separate generator inputs (§8.1).

The first edge-payload trial family uses **`EdgePayloadType = int64`**.

**Generated operation calls:**

```text
graph_adj_directed@empty({ compare_node: compare_node_id, compare_edge: compare_edge, edge_target: edge_target })
graph_adj_directed@add_edge(g, from_id, to_id)
graph_csr_directed@has_edge(g, from_id, to_id)
graph_csr_directed@weight_at(g, slot)
```

Constructor examples:

```text
g: DirectedGraph[string, Edge, mem(normal)] <- graph_adj_directed@empty({
    compare_node: compare_string,
    compare_edge: compare_edge,
    edge_target: edge_target
});
g2 <- graph_adj_directed@add_edge(g, "root", { to: "leaf", label: :walk });
DirectedGraph@has_edge(g2, "root", "leaf")
```

The **module** name (`graph_adj_directed`, `graph_csr_directed`, …) identifies the representation family (§2.2). **Do not** repeat the module name in the function identifier after `@`. **Exports** use short operation verbs and arity only (`export empty/1`, `export add_edge/3`, …), not one export line per payload spelling.

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
- Random `has_edge(u: int64, v: int64)` needs to be fast.
- The graph is very large.
- Node ids are dense and the graph will be traversed many times. Use CSR instead.

### 3.2 Unweighted shape

Design name:

```text
NodeIdAdjacencyGraphDirected[NodeIdType, mem(S)]
```

Silica inline shape (no vertex attributes):

```silica
{
    node_count: int64,
    edge_count: int64,
    nodes: List[
        {
            id: NodeIdType,
            neighbors: List[NodeIdType, S]
        },
        S
    ]
}
```

With optional **`NodeDataType`** vertex attributes:

```silica
{
    node_count: int64,
    edge_count: int64,
    nodes: List[
        {
            id: NodeIdType,
            data: NodeDataType,
            neighbors: List[NodeIdType, S]
        },
        S
    ]
}
```

Bootstrap concrete shape for `NodeIdType = int64` and `normal`:

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
NodeIdAdjacencyGraphDirected[int64, mem(S)]
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
fn empty[mem(normal)](
    node_count: int64
) -> {
    node_count: int64,
    edge_count: int64,
    nodes: List[{ id: int64, neighbors: List[int64, mem(normal)] }, mem(normal)]
} {
    sequence proc[mem(normal)]
        ...
    produces
        pure graph
    end
}
```

Bootstrap add edge (`NodeIdType = int64`; optional edge payload is concrete `EdgePayloadType`):

```silica
fn add_edge[mem(normal)](
    graph: {
        node_count: int64,
        edge_count: int64,
        nodes: List[{ id: int64, neighbors: List[int64, mem(normal)] }, mem(normal)]
    },
    from_id: int64,
    to_id: int64
) -> {
    node_count: int64,
    edge_count: int64,
    nodes: List[{ id: int64, neighbors: List[int64, mem(normal)] }, mem(normal)]
} {
    sequence proc[mem(normal)]
        ...
    produces
        pure updated_graph
    end
}
```

Weighted or attributed add edge prepends `{ to: to_id, data: edge_data }` (or `{ to: to_id, weight: weight }` when **`EdgeData`** is **`int64`**) and returns a **new** graph value (§2.7).

### 3.6 Traversal strategy

Generated traversal should expose neighbor lists directly. The **node id** operand that selects whose neighbors to return is **`NodeIdType`**; bootstrap int64 modules use an `int64` topology index:

```text
neighbors(graph, id: NodeIdType) -> List[NodeIdType, S]
weighted_neighbors(graph, id: NodeIdType) -> List[{ to: NodeIdType, data: EdgePayloadType }, S]
get_node_data(graph, id: NodeIdType) -> { ok: bool, value: NodeDataType }
```

Implementation scans `graph.nodes` until it finds `node.id == id`.

`has_edge(graph, from_id: NodeIdType, to_id: NodeIdType)`:

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
directedness x constructor-record/type family x memory space
```

Examples:

```text
graph_adj_directed@empty({ compare_node: compare_node, compare_edge: compare_edge, edge_target: edge_target })
graph_adj_directed@add_edge(g, from_id, edge)
DirectedGraph@neighbors(g, id)
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
CompressedSparseRowGraphDirected[mem(S)]
```

Generator constants (not bracket parameters): region id **`R`**, **`N_PLUS_ONE`**, **`M`**.

Silica inline shape (topology only):

```silica
{
    region: region(R, S),
    node_count: int64,
    edge_count: int64,
    offsets: buf(R, S, int64, N_PLUS_ONE),
    neighbors: buf(R, S, int64, M)
}
```

With optional **`NodeData`** and **`EdgeData`** payload buffers (§2.5):

```silica
{
    region: region(R, S),
    node_count: int64,
    edge_count: int64,
    offsets: buf(R, S, int64, N_PLUS_ONE),
    neighbors: buf(R, S, int64, M),
    node_data: buf(R, S, NodeData, N),
    edge_data: buf(R, S, EdgeData, M)
}
```

Omit **`node_data`** / **`edge_data`** when the generator specialization has no vertex or edge payload.

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
CompressedSparseRowGraphDirected[int64, mem(S)]
```

Generator constants (not bracket parameters): region id **`R`**, **`N_PLUS_ONE`**, **`M`**.
```

Silica inline shape (`EdgeData = int64`; **`weights`** is the **`edge_data`** buffer for this family):

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

General **`EdgePayloadType`** uses field name **`edge_data`** instead of **`weights`** (§2.5).

Invariant:

```text
weights[i] (or edge_data[i]) is the payload for the edge to neighbors[i]
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
edges: List[{ from: NodeIdType, to: NodeIdType }, S]
```

For edge payload **`EdgePayloadType`**:

```text
edges: List[{ from: NodeIdType, edge: EdgePayloadType }, S]
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
fn from_static_edges[mem(normal)]() -> {
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

`has_edge(g, u: int64, v: int64)` scans `neighbors[start..end)`. If sorted adjacency is guaranteed, generate binary search; otherwise generate linear scan.

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
graph_csr_<directedness>_<operation>
```

Examples (constructor-record/trait-oriented calls — §2.11):

```text
g: <CSR graph record> <- graph_csr_directed@from_static_edges(...)
graph_csr_directed@out_degree(g, id)
graph_csr_directed@has_edge(g, from_id, to_id)
graph_csr_undirected@weight_at(g, slot)
```

The graph record must contain the owning region. Returning buffers without the region handle is invalid because the buffers would outlive their region.

CSR graph values are **immutable after construction or `freeze`** (§2.7). Public query APIs must not mutate topology or payload buffers in place. If generated code mutates CSR buffers after freeze, the module must use **`_builder_`** or **`_mutable_`** in its name so callers do not assume persistent immutable topology.

## 5. `DenseMatrixGraph`

### 5.1 Summary

`DenseMatrixGraph` stores one cell per possible directed edge. Cell `(from, to)` lives at:

```text
index = from * node_count + to
```

Use it when:

- The graph is dense.
- `has_edge(u: NodeIdType, v: NodeIdType)` must be O(1) after mapping to dense physical slots.
- `node_count` is small enough that `node_count * node_count` storage is acceptable.
- You need simple generated code and predictable indexing.

Avoid it when:

- The graph is sparse.
- `node_count * node_count` is too large.
- Traversal of outgoing neighbors dominates and most cells are empty. Use CSR.

### 5.2 Unweighted shape

Design name:

```text
DenseMatrixGraphDirected[mem(S)]
```

Generator constant (not a bracket parameter): **`N_TIMES_N`**.
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
DenseMatrixGraphDirected[int64, mem(S)]
```

Generator constant (not a bracket parameter): **`N_TIMES_N`**.
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
3. For each edge, compute `index = from_slot * node_count + to_slot` (`from_slot` and `to_slot` are dense **`int64`** physical slots derived from `NodeIdType`).
4. Write `1` to presence.
5. For weighted or attributed graphs, write **`EdgePayloadType`** to **`cell_data`** or **`weights`**.
6. For undirected graphs, also write the mirror edge when `from != to`.

Generated constructor shape:

```silica
fn graph_dense_directed_empty[mem(normal)](
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

`has_edge(g, from_id: int64, to_id: int64)`:

```text
idx = from_id * g.node_count + to_id
read cells[idx] == 1
```

`out_degree(g, from_id: int64)`:

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
DenseBitsetGraphDirected[mem(S)]
```

Generator constant (not a bracket parameter): **`WORD_COUNT`**.
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
mask = one shl bit_offset
```

### 6.3 Operations

Operands `from` and `to` are **`int64`** topology indices.

`set_edge(g, from: int64, to: int64)`:

```text
word = read_buf(words, word_index)
new_word = word bor mask
write_buf(words, word_index, new_word)
```

`has_edge(g, from: int64, to: int64)`:

```text
(read_buf(words, word_index) band mask) != zero
```

`clear_edge(g, from: int64, to: int64)`:

```text
word = read_buf(words, word_index)
new_word = word band (bnot mask)
write_buf(words, word_index, new_word)
```

Undirected insertion sets both bits.

### 6.4 Code-generation note

Only generate `DenseBitsetGraph` when the current compiler path supports the required keyword bitwise operators (`bor`, `band`, `bnot`, `shl`, `shr`). If those are not available for the target stage, generate `DenseMatrixGraphDirected[mem(S)]` instead. The matrix form has worse storage use but simpler generated code.

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
weightedness: unweighted | weighted | edge_payload
node_id_type: concrete inline NodeIdType
node_data_type: none | concrete inline NodeDataType
edge_payload_type: none | concrete inline EdgePayloadType
cell_data_type: none | concrete inline CellDataType   // dense only
memory_space: normal | normal_writethrough | normal_noncacheable | atomic
graph_functions: { compare_node: fn(NodeIdType, NodeIdType) -> atom, compare_edge: fn(EdgePayloadType, EdgePayloadType) -> atom, edge_target: fn(EdgePayloadType) -> NodeIdType }
node_count_known: bool
node_count: int64, when known
edge_count: int64, when known
sorted_neighbors: bool
module_name: string   // Silica module / filename stem: graph_<repr>_<directedness> (§8.4); no payload or mem suffix
```

**`module_name`:** the representation family module stem — for example `graph_adj_directed`, `graph_csr_undirected`. Node id type, payload types, and **`mem(Space)`** are **not** part of this string; they are fixed by the declared collection type and constructor function record.

**Type witness rule:** each graph value flow uses concrete inline types for **`NodeIdType`**, **`NodeDataType`**, **`EdgePayloadType`**, and **`CellDataType`** (for example `int64`, `string`, or `(int8, string, atom, { x: int64 })`). Generators emit **one module per representation family** (§2.2, §8.4) with **arity-only exports**; constructor function records witness and check the concrete types (§2.4, §2.8).

**Registry key (bracket form, §2.11):** combine representation family, directedness, and type slots — for example `NodeIdAdjacencyGraphDirected[int64, mem(normal)]` (bootstrap unweighted), `NodeIdAdjacencyGraphDirected[string, Edge, mem(normal)]`, `CompressedSparseRowGraphUndirected[NodeIdType, NodeDataType, EdgePayloadType, mem(normal)]` when both payload buffers are present.

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
freeze, when a list or builder stage precedes CSR/dense (§2.7)
node_count
edge_count
has_edge
out_degree
neighbors traversal helper
validate, optional but recommended
```

When **`NodeData`** is present:

```text
get_node_data / set_node_data (set returns new graph — §2.7)
```

Weighted or attributed edge modules should also emit:

```text
edge_data_at (or weight_at for EdgeData = int64), returning an explicit found flag plus payload
```

For **`edge_data_at`**, **`get_node_data`**, **`add_edge`**, and **`set_node_data`**, payload operands and return **`value`** fields use the concrete **`EdgePayloadType`** / **`NodeDataType`** embedded in the graph parameter type (§2.4, §2.8). Topology operands (**`from_id`**, **`to_id`**, **`id`**) are **`NodeIdType`**.

Recommended return shape for **`edge_data_at`** / **`weight_at`**:

```silica
{ found: bool, value: EdgeData }
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
has_edge[mem(normal)]
has_edge_unchecked[mem(normal)]
```

### 8.4 Naming rules

Generated names should be deterministic. **Design/registry names** may use bracket syntax (§2.11). **Module names** and **operation names** identify the representation family and operation; node id type, payload type, and memory space are checked through the declared collection type and constructor function record.

#### Module names

One Silica module per **representation + directedness** family (§2.2). The filename (without `.silica`) is the module name:

```text
graph_<repr>_<directedness>
```

Where:

```text
repr = adj | csr | dense | bitset
directedness = directed | undirected
```

Examples: `graph_adj_directed.silica`, `graph_csr_undirected.silica`, `graph_dense_directed.silica`.

**Do not** suffix module names with payload type, weightedness, or memory space (for example `graph_adj_directed_int64_mem_normal` is **incorrect**). Mutable builder modules append **`_builder_`** or **`_mutable_`** only (§2.7).

Import and call (short operation name after `@`; do not repeat the module name):

```text
use graph_adj_directed;
g: DirectedGraph[string, Edge, mem(normal)] <- graph_adj_directed@empty({
    compare_node: compare_string,
    compare_edge: compare_edge,
    edge_target: edge_target
});
graph_adj_directed@add_edge(g, from_id, to_id)
```

**Single-file `use` rule:** do not `use` two generated graph modules in one source file when both export the same `operation/arity` and overload resolution would be ambiguous (E4011). Prefer one graph module per compilation unit, or distinct operation names for cross-representation wrappers.

#### Operation names

**Exported function names** are short operation verbs inside the module file — for example `empty`, `add_edge`, `has_edge`, `validate`. They **do not** repeat the module name. **Export declarations** use arity only (`export add_edge/3`), not per-payload bracket suffixes.

**Module-qualified generated update syntax:**

```text
<module>@<operation>(<args>)
```

**Trait behavior syntax:**

```text
<Trait>@<operation>(<receiver>, ...)
```

Constructor records check the typed **`graph`** binding and preserve comparator/extractor functions. **Internal** helpers stay module-local. **Linker symbols:** the compiler mangles by module, operation, arity, and resolved structure/payload types (for example suffix from the first-parameter graph record type) so one export name can link correctly.

### 8.5 Structural type emission

Because Silica has no custom type names, the generator must:

1. Build the full inline graph type string.
2. Reuse the exact same string in function parameters, returns, locals, and pattern annotations.
3. Keep field order stable for record types.
4. Use the same memory space everywhere in a graph family.
5. Include the owning region in every returned value that contains buffers or references.
6. Enforce **uniform graph types** (§2.7): the same inline graph spelling at every boundary for one graph value flow.

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
2. Bitwise operator coverage: `DenseBitsetGraph` depends on `bor`, `band`, `bnot`, `shl`, and `shr`. Until those are uniformly available, dense matrix is the fallback.
3. Map support: if Silica gains a map/dictionary representation, adjacency lookup can become faster without CSR conversion.
4. Region lifetime ergonomics: graph records with buffers must carry the owning region. Future region-bundle ergonomics may reduce the amount of repeated inline type text.
5. Sorting support: sorted CSR adjacency enables binary search for `has_edge`; unsorted CSR preserves insertion order and simpler construction.
6. **`graphpayload.silica` registry:** optional type-level registrations for large inline graph shapes (§2.8).

## 10. References

- **Constructor function record** — inline structural value whose function fields witness node id, edge payload, and weight types and preserve compare/extract behavior (§2.4; [Phase1_TODOs/data_structures_as_traits.md](Phase1_TODOs/data_structures_as_traits.md)).
- **Immutability and type invariance** — §2.7; **type checking** — §2.8.
- [silica-specification.md](silica-specification.md) - inline structural types, lists, regions, effects.
- [list_implementation_design.md](list_implementation_design.md) - `List[T, S]` as region-backed storage and bundle model.
- [recursive_tuple_specification.md](recursive_tuple_specification.md) - why recursive pointer-shaped data uses inline `rec` and regions instead of named recursive types.
- [region_memory_safety_todo.md](Phase1_TODOs/region_memory_safety_todo.md) - current region safety implementation gaps.
- [atom_actor_registry_direct_index_design.md](atom_actor_registry_direct_index_design.md) - example of preferring direct buffers over a surface `Map`.
