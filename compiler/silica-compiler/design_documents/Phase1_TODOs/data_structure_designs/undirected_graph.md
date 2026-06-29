# `UndirectedGraph` Detailed Design

**Public trait:** `UndirectedGraph`
**Generated modules:** `graph_wbt_undirected`, `graph_csr_undirected`, `graph_dense_undirected`

## 1. Abstract graph

`UndirectedGraph[EdgeDataType, mem(SpaceType)]` is a finite undirected simple graph whose public vertex IDs are `int64`:

- unordered endpoint pairs identify edges;
- one self-loop per vertex is permitted;
- parallel logical edges are not;
- isolated vertices are retained;
- every endpoint exists as a vertex.

Physical representations may store two directions, but those directions form one logical edge.

## 2. Edge data and internal directional wrapper

The public `EdgeDataType` is direction-independent. The generated representation wraps it internally at each adjacency position:

```text
{
    to: int64,
    data: EdgeDataType
}
```

For a non-loop logical edge `(u,v,data)`, the representation stores `{to:v,data}` under `u` and `{to:u,data}` under `v`. A self-loop stores one wrapper. The programmer does not supply `edge_target`, `retarget_edge`, or a reverse-edge function.

The unweighted specialization uses `EdgeDataType = unit`; its convenience `add_edge/3` supplies `()` internally. The general attributed operation is `add_edge/4`.

Constructor:

```text
{
    compare_node: fn(int64, int64) -> atom,
    compare_edge_data: fn(EdgeDataType, EdgeDataType) -> atom
}
```

## 3. Trait contract

The trait mirrors `DirectedGraph` but names undirected degree:

```text
export trait UndirectedGraph;
export node_count/1;
export edge_count/1;
export has_vertex/2;
export neighbors/2;
export fold_neighbors/4;
export compare_node/3;
export compare_edge_data/3;
export edge_target/2;
export degree/2;
export has_edge/3;
export connected/3;

required {
    fn node_count(g: UndirectedGraph) -> int64;
    fn edge_count(g: UndirectedGraph) -> int64;
    fn has_vertex(g: UndirectedGraph, id: int64) -> boolean;
    fn neighbors(g: UndirectedGraph, id: int64)
        -> List[{to: int64, data: EdgeDataType}, SpaceType];
    fn fold_neighbors(g, id, init, step) -> AccType;
    fn compare_node(g, a, b) -> atom;
    fn compare_edge_data(g, a: EdgeDataType, b: EdgeDataType) -> atom;
    fn edge_target(g, edge: {to: int64, data: EdgeDataType}) -> int64;
}

provided {
    fn degree(g: UndirectedGraph, id: int64) -> int64;
    fn has_edge(g: UndirectedGraph, a: int64, b: int64) -> boolean;
    fn connected(g: UndirectedGraph, a: int64, b: int64) -> boolean;
}
```

As in `DirectedGraph`, graph-receiver `edge_target/2` replaces the non-dispatchable parent sketch.

## 4. Live module surface

```text
export empty/1;
export add_vertex/2;
export add_edge/3;        // unweighted convenience
export add_edge/4;        // graph, from, to, data
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

`add_edge(g, from, to, data)` receives endpoints separately from direction-independent edge data. The unweighted `add_edge(g, from, to)` supplies unit data.

Result flags match directed graph insertion:

```text
{graph, inserted: boolean, replaced: boolean}
```

## 5. Symmetric update semantics

For distinct endpoints `u` and `v`:

- insertion publishes both adjacency entries or none;
- removal publishes deletion of both or none;
- duplicate unweighted insert is a no-op;
- attributed/weighted replacement updates the data in both directional wrappers;
- one logical edge changes `edge_count` by one.

For `u == v`, only one diagonal adjacency entry exists.

Missing endpoints are auto-added by live insertion. Removal never removes vertices.

## 6. Degree and neighbors

Neighbor materialization emits one `{to,data}` entry per stored adjacency entry in ascending target order.

Graph-theoretic degree:

```text
degree(v) = non_loop_adjacency_entries(v) + 2 * self_loops(v)
```

Thus a self-loop appears once in `neighbors` but contributes two to `degree`.

Missing and isolated vertices both yield degree zero; `has_vertex` distinguishes them.

## 7. Edge fold

`fold_edges` visits every logical edge once:

- self-loop once;
- non-loop edge only from the endpoint that compares less under `compare_node`.

This canonical orientation is for traversal only and does not make the graph directed.

## 8. Connected

`connected(g,a,b)`:

- false if either vertex is absent;
- true if comparator-equal existing vertices;
- otherwise performs ordinary visited-set graph search;
- terminates in cycles and self-loops.

Because adjacency is symmetric, traversal direction is irrelevant.

## 9. Counts

Let `L` be self-loops and `N` non-loop logical edges:

```text
edge_count = L + N
adjacency_entry_count = L + 2N
sum(degree(v)) = 2 * edge_count
```

All equations use checked arithmetic.

## 10. Representation behavior

- Live WBT graphs grow vertices dynamically and maintain symmetric inner WBTs.
- CSR snapshots preserve both physical directions and logical edge count.
- Dense graphs use mirrored matrix cells and a single diagonal cell.

All implement identical query semantics.

The WBT, CSR, and dense module families produce distinct concrete generated types. Attributed and weighted forms are additional specializations. Their compiler-version-private inline layouts are not a stable source or cross-version ABI.

## 11. Invariants

1. every non-loop adjacency has exactly one reverse adjacency;
2. reverse entries carry comparator-equal `EdgeDataType` values;
3. self-loops are stored once;
4. endpoint and count invariants hold;
5. logical edge fold's canonical orientation is unique;
6. all representation-specific ordering and persistence invariants hold.

Asymmetry is a validation failure, not an alternate graph state.

## 12. Complexity

Live WBT:

| Operation | Time |
|---|---:|
| add vertex | `O(log V)` |
| add/remove non-loop edge | `O(log V + log d(u) + log d(v))` |
| self-loop update | `O(log V + log d(u))` |
| has edge | `O(log V + log d(u))` |
| neighbors | `O(log V + stored_degree(u))` |
| connected | `O((V_c + A_c) log V)` with WBT visited set |

## 13. Example

```silica
g0: UndirectedGraph[unit, mem(normal)]
    <- graph_wbt_undirected@empty({
        compare_node: compare_int64,
        compare_edge_data: compare_unit
    });

g1 <- graph_wbt_undirected@add_edge(g0, 10, 20);
ab <- UndirectedGraph@has_edge(g1.graph, 10, 20);
ba <- UndirectedGraph@has_edge(g1.graph, 20, 10);
```

Both queries are true and `edge_count = 1`.

## 14. Exclusions

No parallel logical edges, half-edge public state, implicit vertex deletion, or self-loop double storage is allowed.
