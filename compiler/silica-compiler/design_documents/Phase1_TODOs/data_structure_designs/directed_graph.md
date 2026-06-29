# `DirectedGraph` Detailed Design

**Public trait:** `DirectedGraph`
**Generated modules:** `graph_wbt_directed`, `graph_csr_directed`, `graph_dense_directed`
**Default live representation:** [`live_wbt_graph.md`](live_wbt_graph.md)

## 1. Abstract graph

`DirectedGraph[EdgePayloadType, mem(SpaceType)]` is a finite directed simple graph whose public vertex IDs are `int64`:

- vertices are unique under `compare_node`;
- at most one directed edge exists for `(from,to)`;
- self-loops are permitted;
- parallel edges are not;
- isolated vertices are retained;
- every edge endpoint is a vertex.

For the unweighted generated variant, `EdgePayloadType = int64`. A custom attributed directed variant may use a payload whose `int64` target is extracted by `edge_target`. The standard weighted variant keeps direction-independent `EdgeDataType` separate and generates the internal directed wrapper `{to,data}`.

## 2. Constructor record

```text
{
    compare_node: fn(int64, int64) -> atom,
    compare_edge: fn(EdgePayloadType, EdgePayloadType) -> atom,
    edge_target: fn(EdgePayloadType) -> int64
}
```

The three functions are captured in the graph value and define its ordering identity.

Unweighted construction uses the same node comparator for `compare_edge` and identity for `edge_target`.

## 3. Trait contract

```text
export trait DirectedGraph;
export node_count/1;
export edge_count/1;
export has_vertex/2;
export neighbors/2;
export fold_neighbors/4;
export compare_node/3;
export compare_edge/3;
export edge_target/2;
export out_degree/2;
export has_edge/3;
export reachable/3;

required {
    fn node_count(g: DirectedGraph) -> int64;
    fn edge_count(g: DirectedGraph) -> int64;
    fn has_vertex(g: DirectedGraph, id: int64) -> boolean;
    fn neighbors(g: DirectedGraph, id: int64)
        -> List[EdgePayloadType, SpaceType];
    fn fold_neighbors(
        g: DirectedGraph,
        id: int64,
        init: AccType,
        step: fn(AccType, EdgePayloadType) -> AccType
    ) -> AccType;
    fn compare_node(g: DirectedGraph, a: int64, b: int64) -> atom;
    fn compare_edge(g: DirectedGraph, a: EdgePayloadType, b: EdgePayloadType) -> atom;
    fn edge_target(g: DirectedGraph, edge: EdgePayloadType) -> int64;
}

provided {
    fn out_degree(g: DirectedGraph, id: int64) -> int64;
    fn has_edge(g: DirectedGraph, from: int64, to: int64) -> boolean;
    fn reachable(g: DirectedGraph, from: int64, to: int64) -> boolean;
}
```

### Receiver dispatch

`edge_target/2` has `g` first so dispatch can access the extractor captured by a particular graph value. The same receiver rule applies to comparators.

`fold_neighbors` is required alongside materializing `neighbors`; provided algorithms use it to avoid a temporary list.

## 4. Live module surface

```text
export empty/1;
export add_vertex/2;
export add_edge/3;
export remove_edge/3;
export has_vertex/2;
export has_edge/3;
export neighbors/2;
export fold_vertices/3;
export fold_edges/3;
export node_count/1;
export edge_count/1;
export validate/1;
```

Signatures:

```text
add_vertex(g, id) -> {
    graph: DirectedGraph[EdgePayloadType, mem(SpaceType)],
    inserted: boolean
}

add_edge(g, from, edge) -> {
    graph: DirectedGraph[EdgePayloadType, mem(SpaceType)],
    inserted: boolean,
    replaced: boolean
}

remove_edge(g, from, to) -> {
    graph: DirectedGraph[EdgePayloadType, mem(SpaceType)],
    removed: boolean
}
```

Unweighted duplicate insertion has both flags false. Attributed/weighted insertion on an existing pair has `inserted=false`, `replaced=true`.

## 5. Vertex and edge update semantics

`add_vertex` retains isolated vertices.

`add_edge` derives `to` from the payload and automatically inserts missing `from` and `to` vertices. This rule is specific to the live WBT representation.

`remove_edge` removes only the edge. It never removes an endpoint or another incident edge.

A future `remove_vertex` must be designed separately because removing incoming edges requires a full graph traversal in this outgoing-adjacency representation.

## 6. Query semantics

Neighbor order is ascending by target node comparator. `neighbors` on a missing or isolated node returns an empty list; `has_vertex` distinguishes those states.

`out_degree` is zero for missing or isolated nodes.

`has_edge` is false if either endpoint is absent.

`reachable`:

- false if either endpoint is absent;
- true for identical existing endpoints, including an isolated vertex;
- otherwise performs graph search using an `OrderedSet` visited set under the graph's node comparator;
- follows outgoing edges only;
- terminates on finite cyclic graphs.

Traversal order does not affect the boolean result.

## 7. Representation-specific construction

- `graph_wbt_directed@empty` produces a dynamically growing live graph.
- `graph_csr_directed@freeze_from_wbt` produces an immutable snapshot.
- `graph_dense_directed@empty_for_nodes` produces a fixed-vertex dense graph.

All three implement the query trait. Only their own generated modules expose valid updates.

The three module families produce distinct concrete generated types. Attributed and weighted forms are additional specializations of those families. Their compiler-version-private inline layouts are not a stable source or cross-version ABI.

## 8. Counts

`node_count` is vertex cardinality. `edge_count` is the number of directed pairs. A self-loop counts once.

For the directed representations:

```text
edge_count = sum(out_degree(v))
```

## 9. Invariants

1. all node and edge identity functions satisfy the common comparator contract;
2. every edge target exists as a vertex;
3. each `(from,to)` occurs at most once;
4. extracted payload target equals the adjacency key;
5. counts are exact and non-negative;
6. neighbor order is strict by target;
7. representation-specific invariants hold;
8. captured function bundle, region, and ordering identity are uniform.

## 10. Complexity

Live WBT:

| Operation | Time |
|---|---:|
| add vertex | `O(log V)` |
| add/remove/has edge | `O(log V + log d(from))` |
| out degree | `O(log V)` |
| neighbors | `O(log V + d(from))` plus list allocation |
| reachable | `O((V_r + E_r) log V)` with WBT visited set |

CSR and dense bounds are specified in their representation files.

## 11. Example

```silica
g0: DirectedGraph[int64, mem(normal)]
    <- graph_wbt_directed@empty({
        compare_node: compare_int64,
        compare_edge: compare_int64,
        edge_target: identity_int64
    });

g1 <- graph_wbt_directed@add_edge(g0, 10, 20);
yes: boolean <- DirectedGraph@has_edge(g1.graph, 10, 20);
no: boolean <- DirectedGraph@has_edge(g1.graph, 20, 10);
```

`yes` is true and `no` is false.

## 12. Exclusions

No implicit reverse edge, parallel edge, dense-bitset backend, or mutable CSR update is included. Phase 1 deliberately fixes public vertex IDs to `int64`; accepting another vertex-ID type requires a later trait/type expansion.
