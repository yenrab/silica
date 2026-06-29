# Standard Data Structures As Traits

**Date:** 2026-06-29
**Status:** **Not implemented** — trait API and representation wiring specification only.
**Algorithm authority (locked):** [data_structure_to_algorithms.md](data_structure_to_algorithms.md)
**Detailed API and representation authority:** [data_structure_designs/README.md](data_structure_designs/README.md)

---

## Purpose

Silica standard collections expose **behavior through traits**. Generated representation modules implement those traits. Construction and updates use **inline constructor function records** checked against explicitly declared collection types.

This document specifies:

- Trait module surfaces (`required` / `provided` operations)
- Constructor function-record contracts
- Which **generated module** implements each trait (aligned with the algorithm map)
- What programmers vs generated code supply

It does **not** describe bootstrap stdlib modules (`btree_`*, `graph_adj_*`, `heap_binary_*`). Those are obsolete relative to the locked algorithm map.

---

## Architecture (trait → representation → algorithm)

```text
OrderedSet / OrderedMap / SearchTree  →  wbt_set / wbt_map           →  Adams WBT [Ada93]
DirectedGraph / UndirectedGraph       →  graph_wbt_*                 →  WBT + WBT neighbors [Ada93, KL95]
WeightedGraph                         →  graph_wbt_* (weighted)        →  WBT + WBT map (to → edge data)
CSR graph (snapshot)                    →  graph_csr_*                 →  O(V+E) freeze from live WBT [KL95]
Dense matrix graph                    →  graph_dense_*                 →  Skew binary random-access list [Oka95]
Tree                                  →  tree_rose                    →  Rose tree; random-access child list [Oka95]
Heap                                  →  brodal_okasaki_min / max      →  Brodal–Okasaki [BO96]
PriorityQueue                         →  brodal_okasaki_min (pairs)    →  Brodal–Okasaki [BO96]
```

**Not in scope:** dense bitset graph.

**Rejected parallel paths:** finger trees, Patricia tries, HAMT, lazy/bootstrapped primary heaps, persistent vectors, d-ary array heaps, Hinze priority-search queues — see algorithm map.

---

## Design principles

1. Traits are not inheritable; a concrete type may implement multiple traits.
2. Behavior traits dispatch from a receiver-like first argument (`OrderedSet@contains(set, key)`).
3. All mutators return **new values** (functional persistence); captured comparators are preserved on every update path.
4. Node ids, keys, values, priorities, directed edge payloads, direction-independent edge data, and weights are **typed collection parameters** — not assumed `int64`.
5. Counts, capacities, and internal indices may remain `int64`.
6. Generated constructors take **inline function records** (no named struct aliases for those records).
7. Collection variables declare explicit collection types including memory effects (`mem(normal)`, `mem(writethrough)`).
8. **One exported function per operation** per generated module (`export insert/2`, not per-width duplicates).
9. Algorithms in user code and stdlib should depend on **traits**, not generated module names.
10. Programmers may implement traits directly or supply **views/adapters** over custom storage.
11. Every constructor uses the canonical application-lifetime arena for its generated representation specialization and memory space.
12. Query status uses the atoms `:not_found | :found`; no named option type is introduced.

---

## Comparator contract

Ordering comparators return `**atom`** with valid results `:less`, `:equal`, and `:greater`.

```text
fn compare_string(a: string, b: string) -> atom { ... }
```

Any other atom is **invalid comparator behavior** and must be rejected (runtime validation in generated modules until stricter static checking exists).

**Generated spelling:** trait and generated signatures may use the sum type `:less | :equal | :greater` as a stricter witness until bare-`atom` validation is implemented.

---

## Constructor function record rule

Every standard collection constructor takes an inline function record. The compiler:

1. Checks the binding's declared collection type (e.g. `OrderedSet[string, mem(normal)]`).
2. Witnesses payload types from function-field signatures (e.g. `compare_item: fn(string, string) -> atom`).
3. Specializes the generated representation and trait dispatch to those concrete types.
4. Stores captured functions in the generated value for use by trait `provided` algorithms and representation updates.
5. Resolves the canonical application-lifetime arena for the generated specialization and memory space.

Memory space comes from the declared collection type, not from the function record.

Ordering compatibility is based on exact function-value identity. A top-level function symbol has one canonical identity; a closure identity includes its exact captured-environment instance, so separately created closures are incompatible even if behaviorally equivalent. Function-type equality is insufficient, and programmers cannot override identity with a declared token.

**Example (ordered set):**

```text
names: OrderedSet[string, mem(normal)] <- wbt_set@empty({
    compare_item: compare_string
});
names2: OrderedSet[string, mem(normal)] <- wbt_set@insert(names, "Ada");
```

**Example (mismatch — compile error):**

```text
names: OrderedSet[string, mem(normal)] <- wbt_set@empty({
    compare_item: fn(a: int64, b: int64) -> atom { ... }  // wrong: not fn(string, string)
});
```

**Assoc-type placeholders:** trait signatures use `ItemType`, `KeyType`, `ValueType`, `EdgePayloadType`, `EdgeDataType`, `WeightType`, and `SpaceType` (via `mem(SpaceType)`). The compiler resolves them from declared bracket types and constructor-record witnesses. Phase 1 graph vertex IDs are fixed to `int64` and therefore do not introduce a `NodeIdType` placeholder.

---

## Trait categories

### Behavior traits

Operations over an existing structure value:

```text
DirectedGraph@neighbors(g, node_id)
OrderedSet@contains(set, key)
OrderedMap@get(map, key)
Heap@peek(heap)
```

### Generated construction and updates

Factory and mutator functions on representation modules (`wbt_set@empty`, `wbt_set@insert`, `graph_wbt_directed@add_edge`, `brodal_okasaki_min@push`, …). These are **not** separate trait surfaces; they return new values and preserve captured functions from construction.

---

## Trait modules and generated backends


| Trait module      | Primary generated module(s)                                              | Algorithm                          |
| ----------------- | ------------------------------------------------------------------------ | ---------------------------------- |
| `OrderedSet`      | `wbt_set`                                                                | Adams WBT [Ada93]                  |
| `OrderedMap`      | `wbt_map`                                                                | Adams WBT map [Ada93]              |
| `SearchTree`      | `wbt_set` (same backing as ordered set)                                  | Adams WBT [Ada93]                  |
| `DirectedGraph`   | `graph_wbt_directed`, `graph_csr_directed`, `graph_dense_directed`       | WBT live / CSR freeze / dense list |
| `UndirectedGraph` | `graph_wbt_undirected`, `graph_csr_undirected`, `graph_dense_undirected` | same                               |
| `WeightedGraph`   | weighted variants of above                                               | WBT inner map `to → edge data`     |
| `Heap`            | `brodal_okasaki_min`, `brodal_okasaki_max`                               | Brodal–Okasaki [BO96]              |
| `PriorityQueue`   | `brodal_okasaki_min` (pair compare)                                      | Brodal–Okasaki [BO96]              |
| `Tree`            | `tree_rose`                                                              | Random-access child list [Oka95]   |


Thin **adapter modules** (`ordered_set_wbt_adapter`, `ordered_map_wbt_adapter`, …) may forward trait `impl fn` bodies to generated `@` operations on inner record shapes. Adapters are stdlib glue, not a separate public constructor style.

---

## OrderedSet

**Representation:** Adams weight-balanced tree with path copying [Ada93]. No integer-key specialization (no Patricia / crit-bit).

**Constructor function record:**

```text
{ compare_item: fn(ItemType, ItemType) -> atom }
```

**Trait surface:**

```text
export trait OrderedSet;

export contains/2;
export size/1;
export fold/3;
export compare_item/3;

provided {
    fn compare_item[SetType, ItemType](set: SetType, a: ItemType, b: ItemType) -> atom;
    fn fold[SetType, ItemType, AccType](
        set: SetType, init: AccType, step: fn(AccType, ItemType) -> AccType
    ) -> AccType;
    fn contains[SetType, ItemType](set: SetType, item: ItemType) -> boolean;
    fn size[SetType](set: SetType) -> int64;
}
```

**Representation-supplied `fold`:** in-order WBT traversal. Default `contains` and `size` may use `fold` + `compare_item`.

**Generated module `wbt_set`:**

```text
export empty/1;
export insert/2;
export delete/2;
export fold/3;           // required for trait provided block
export from_sorted/1;    // optional O(n) bulk build from sorted unique keys

fn empty[ItemType, SpaceType](
    item_functions: { compare_item: fn(ItemType, ItemType) -> atom }
) -> OrderedSet[ItemType, SpaceType];

fn insert[ItemType, SpaceType](
    set: OrderedSet[ItemType, SpaceType], item: ItemType
) -> { set: OrderedSet[ItemType, SpaceType], inserted: boolean };

fn delete[ItemType, SpaceType](
    set: OrderedSet[ItemType, SpaceType], item: ItemType
) -> { set: OrderedSet[ItemType, SpaceType], removed: boolean };
```

**Semantics:** duplicate insert → unchanged tree, `inserted = false`. Default bulk build: fold insert; optional `from_sorted` when keys are pre-sorted.

---

## OrderedMap

**Representation:** Adams WBT map [Ada93] — replace value on duplicate key.

**Constructor function record:**

```text
{
    compare_key: fn(KeyType, KeyType) -> atom,
    compare_value: fn(ValueType, ValueType) -> atom
}
```

Value comparator enables linear `find_value` via fold; primary storage remains key-indexed.

**Trait surface:**

```text
export trait OrderedMap;

export contains_key/2;
export find_value/2;
export get/2;
export size/1;
export fold/3;
export compare_key/3;
export compare_value/3;

provided {
    fn compare_key[MapType, KeyType](map: MapType, a: KeyType, b: KeyType) -> atom;
    fn compare_value[MapType, ValueType](map: MapType, a: ValueType, b: ValueType) -> atom;
    fn fold[MapType, KeyType, ValueType, AccType](
        map: MapType, init: AccType,
        step: fn(AccType, KeyType, ValueType) -> AccType
    ) -> AccType;
    fn get[MapType, KeyType, ValueType](map: MapType, key: KeyType)
        -> { status: :not_found | :found, value: ValueType };
    fn contains_key[MapType, KeyType, ValueType](map: MapType, key: KeyType) -> boolean;
    fn find_value[MapType, KeyType, ValueType](map: MapType, value: ValueType)
        -> { status: :not_found | :found, key: KeyType };
    fn size[MapType](map: MapType) -> int64;
}
```

**Generated module `wbt_map`:**

```text
export empty/1;
export insert/3;
export delete/2;
export get/2;
export fold/3;
export from_sorted/2;    // optional O(n) from sorted unique keys + parallel values

fn empty[KeyType, ValueType, SpaceType](
    map_functions: {
        compare_key: fn(KeyType, KeyType) -> atom,
        compare_value: fn(ValueType, ValueType) -> atom
    }
) -> OrderedMap[KeyType, ValueType, SpaceType];

fn insert[KeyType, ValueType, SpaceType](
    map: OrderedMap[KeyType, ValueType, SpaceType], key: KeyType, value: ValueType
) -> { map: OrderedMap[KeyType, ValueType, SpaceType], inserted: boolean, replaced: boolean };

fn get[KeyType, ValueType, SpaceType](
    map: OrderedMap[KeyType, ValueType, SpaceType], key: KeyType
) -> { status: :not_found | :found, value: ValueType };
```

---

## SearchTree

**Representation:** identical to `OrderedSet` (Adams WBT [Ada93]). Search-tree behavior is trait naming and documentation, not a separate storage family.

**Constructor function record:** same as `OrderedSet`.

**Trait surface:**

```text
export trait SearchTree;

export contains_key/2;
export compare_item/3;

provided {
    fn contains_key[TreeType, ItemType](tree: TreeType, key: ItemType) -> boolean;
    // Delegates to OrderedSet@contains when backing store is wbt_set-compatible.
}
```

No separate B-tree or node-id tree representation.

---

## Graph traits

### Live model (WBT + WBT adjacency)

```text
adj : WBT<int64, WBT<int64, Unit>>              -- unweighted (inner set as WBT)
adj : WBT<int64, WBT<int64, EdgeData>>           -- weighted / attributed
```

Outer and inner structures use WBT with `compare_node`. Undirected graphs: symmetric update on `(u, v)` and `(v, u)`.

**Vertex identity:** public vertex IDs are `int64` in every Phase 1 graph representation. Internal CSR/dense slots are also `int64`, but remain a distinct domain reached through an explicit node-to-slot map.

**Unweighted directed edge payload:** `EdgePayloadType = int64`.

**Weighted / attributed edge data:** target and direction-independent `EdgeDataType` are separate inputs. Generated neighbor views use the inline wrapper `{to: int64, data: EdgeDataType}`. Undirected graphs store two generated directional wrappers over one logical edge datum; programmers do not provide reverse-edge or retarget functions.

### DirectedGraph

**Constructor function record (directed unweighted minimum):**

```text
{
    compare_node: fn(int64, int64) -> atom,
    compare_edge: fn(EdgePayloadType, EdgePayloadType) -> atom,
    edge_target: fn(EdgePayloadType) -> int64
}
```

**Trait surface:**

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
    fn neighbors[GraphType, EdgePayloadType, SpaceType](
        g: GraphType, node_id: int64
    ) -> List[EdgePayloadType, SpaceType];
    fn fold_neighbors(g: DirectedGraph, id: int64, init: AccType,
        step: fn(AccType, EdgePayloadType) -> AccType) -> AccType;
    fn compare_node(g: DirectedGraph, a: int64, b: int64) -> atom;
    fn compare_edge(g: DirectedGraph, a: EdgePayloadType, b: EdgePayloadType) -> atom;
    fn edge_target(g: DirectedGraph, edge: EdgePayloadType) -> int64;
}

provided {
    fn out_degree(...);
    fn has_edge(...);
    fn reachable(...);
    fn max_out_degree/1;
    fn total_out_degree_sum/1;
}
```

**Generated live module `graph_wbt_directed`:**

```text
export empty/1;
export add_edge/3;
export remove_edge/3;
export add_vertex/2;

fn empty[EdgePayloadType, SpaceType](
    graph_functions: {
        compare_node: fn(int64, int64) -> atom,
        compare_edge: fn(EdgePayloadType, EdgePayloadType) -> atom,
        edge_target: fn(EdgePayloadType) -> int64
    }
) -> DirectedGraph[EdgePayloadType, mem(SpaceType)];

fn add_edge[EdgePayloadType, SpaceType](
    g: DirectedGraph[EdgePayloadType, mem(SpaceType)],
    from_id: int64,
    edge: EdgePayloadType
) -> {
    graph: DirectedGraph[EdgePayloadType, mem(SpaceType)],
    inserted: boolean,
    replaced: boolean
};

fn add_vertex[EdgePayloadType, SpaceType](
    g: DirectedGraph[EdgePayloadType, mem(SpaceType)],
    id: int64
) -> {
    graph: DirectedGraph[EdgePayloadType, mem(SpaceType)],
    inserted: boolean
};

fn remove_edge[EdgePayloadType, SpaceType](
    g: DirectedGraph[EdgePayloadType, mem(SpaceType)],
    from_id: int64,
    to_id: int64
) -> {
    graph: DirectedGraph[EdgePayloadType, mem(SpaceType)],
    removed: boolean
};
```

Vertices may be added dynamically when an edge references a new id (path-copying WBT insert).

### UndirectedGraph

`UndirectedGraph[EdgeDataType, mem(SpaceType)]` stores direction-independent edge data and generates directional neighbor wrappers internally.

Constructor record:

```text
{
    compare_node: fn(int64, int64) -> atom,
    compare_edge_data: fn(EdgeDataType, EdgeDataType) -> atom
}
```

Its trait surface mirrors `DirectedGraph`, with `degree/2` and `connected/3`, and uses:

```text
neighbors(g, id: int64) -> List[{to: int64, data: EdgeDataType}, SpaceType]
edge_target(g, edge: {to: int64, data: EdgeDataType}) -> int64
```

The general generated update is `add_edge(g, from, to, data)`. The unweighted `EdgeDataType = unit` specialization also exports `add_edge(g, from, to)`.

### WeightedGraph

`WeightedGraph[EdgeDataType, WeightType, mem(SpaceType)]` is an independent capability implemented alongside `DirectedGraph` or `UndirectedGraph`. Its constructor record is:

```text
{
    compare_node: fn(int64, int64) -> atom,
    compare_edge_data: fn(EdgeDataType, EdgeDataType) -> atom,
    edge_weight: fn(EdgeDataType) -> WeightType,
    compare_weight: fn(WeightType, WeightType) -> atom
}
```

Its behavior trait exposes receiver-based `compare_edge_data/3`, `edge_weight/2`, `compare_weight/3`, `weighted_neighbors/2`, `fold_weighted_neighbors/4`, and `weight_of/3`.

Algorithm-specific fields (`zero_weight`, `add_weight`) belong in algorithm function records, not necessarily in the graph value.

### CSR snapshot (read-only traversal)

**Module family:** `graph_csr_`*. Built by **freeze** from a live WBT graph: O(V + E) two-pass (degree count → prefix-sum → scatter). Live graph unchanged after freeze. CSR does not support incremental edge updates — re-freeze after live-graph edits.

Implements the same graph traits for query operations (`neighbors`, `has_edge`, …) on immutable snapshot values.

CSR stores public `int64` node IDs and internal `int64` dense slots as distinct domains. Runtime-sized internal buffers hold node IDs, offsets, neighbors, and—only for attributed/weighted specializations—a parallel edge-data buffer.

### Dense matrix graph (specialized)

**Module family:** `graph_dense_`*. Edge storage in a **skew binary random-access list** indexed by `from * V + to` [Oka95]. For small, dense `V`. Implements graph traits for set/clear/test edge operations via path copying.

Unweighted dense graphs use one boolean cell sequence. Attributed/weighted dense graphs use one `:none | (:some, EdgeDataType)` cell sequence; they do not maintain a redundant presence sequence.

CSR/dense extents are runtime-sized internal values and are not public graph type parameters. WBT, CSR, and dense families are distinct concrete generated types, with additional attributed/weighted specializations and static trait conformance. Their inline structural layouts are private to one compiler/standard-library version and are not stable source, FFI, serialization, or cross-version ABIs.

---

## Heap and PriorityQueue

**Representation:** Brodal–Okasaki optimal queue [BO96] — shared implementation for `Heap` and `PriorityQueue`. Max-heap uses reversed comparison. Arbitrary-entry deletion and decrease-key are not in the Phase 1 API.

### Heap

**Constructor function record:**

```text
{ compare_item: fn(ItemType, ItemType) -> atom }
```

**Trait surface:**

```text
export trait Heap;

export len/1;
export is_empty/1;
export peek/1;
export compare_item/3;

required {
    fn len(heap: Heap) -> int64;
    fn peek[HeapType, ItemType](heap: HeapType)
        -> { status: :not_found | :found, value: ItemType };
}

provided {
    fn is_empty(heap: Heap) -> boolean;
}
```

**Generated module `brodal_okasaki_min` / `brodal_okasaki_max`:**

```text
export empty/1;
export push/2;
export pop/1;
export meld/2;

fn empty[ItemType, SpaceType](
    item_functions: { compare_item: fn(ItemType, ItemType) -> atom }
) -> Heap[ItemType, SpaceType];

fn push[ItemType, SpaceType](
    heap: Heap[ItemType, SpaceType], item: ItemType
) -> Heap[ItemType, SpaceType];

fn pop[ItemType, SpaceType](
    heap: Heap[ItemType, SpaceType]
) -> {
    status: :not_found | :found,
    value: ItemType,
    heap: Heap[ItemType, SpaceType]
};
```

### PriorityQueue

**Constructor function record:**

```text
{
    compare_item: fn(ItemType, ItemType) -> atom,
    compare_priority: fn(PriorityType, PriorityType) -> atom
}
```

When priority is embedded in the item, add `priority_of: fn(ItemType) -> PriorityType`.

**Trait surface:**

```text
export trait PriorityQueue;

export len/1;
export is_empty/1;
export peek/1;
export peek_priority/1;
export peek_value/1;

required {
    fn len(queue: PriorityQueue) -> int64;
    fn peek(queue: PriorityQueue) -> {
        status: :not_found | :found,
        priority: PriorityType,
        value: ItemType
    };
}

provided {
    fn is_empty(queue: PriorityQueue) -> boolean;
    fn peek_priority(queue: PriorityQueue)
        -> { status: :not_found | :found, priority: PriorityType };
    fn peek_value(queue: PriorityQueue)
        -> { status: :not_found | :found, value: ItemType };
}
```

**Generated operations:** `empty_priority_queue`, `push_priority`, `peek_priority_entry`, `pop_priority`, `meld_priority`, `len`, `from_entries`, and `validate`, using the same Brodal–Okasaki core with lexicographic pair comparison (priority first, then value). No `delete_entry` or `decrease_priority` operation is generated.

---

## Tree

**Representation:** rose tree — each node holds a label and a **child sequence** stored in a skew binary random-access list [Oka95, Oka98 §5].

**Trait surface:**

```text
export trait Tree;

export node_count/1;
export root_item/1;
export get/2;
export child_count/2;
export child_slot_count/2;
export child_at/3;
export fold_preorder/3;
export compare_item/3;
```

**Generated module `tree_rose`:** `singleton`, `replace_item`, `add_child`, `add_subtree`, `remove_child`, `get`, `child_at`, `find_first`, `fold_preorder`, and `validate`. Updates use stable child slots and path copying; child slots are never compacted, reused, or renumbered. `node_count` counts logical rose-tree nodes, not WBT or random-access-list internal nodes.

**SearchTree** is documented above; it is not a separate rose-tree search structure.

---

## Views and adapters

Views let custom storage participate in trait algorithms without exposing the generated record shape:

```text
view: {
    graph: GraphType,
    neighbors_of: fn(GraphType, int64) -> List[EdgePayloadType, SpaceType]
} <- DirectedGraph@view(
    graph,
    {
        compare_node: compare_node,
        compare_edge: compare_edge,
        edge_target: edge_target
    },
    neighbors_of
);
```

The function record passed to `@view` supplies comparison semantics; `edge_target` is exposed through receiver-based `DirectedGraph@edge_target(view, edge)`. The view value holds only storage-specific access (`neighbors_of`).

Stdlib adapters (`ordered_set_wbt_adapter`, …) are the same pattern wired as `impl fn` glue from trait modules to generated backends.

---

## What programmers supply

- Explicit collection type declarations at bindings and function returns.
- Inline constructor function records with comparators (and extractors where required).
- For custom structures: either direct trait `impl fn` or a view/adaptor value.

**Examples:**

```text
names: OrderedSet[string, mem(normal)] <- wbt_set@empty({
    compare_item: compare_string
});

users: OrderedMap[string, User, mem(normal)] <- wbt_map@empty({
    compare_key: compare_string,
    compare_value: compare_user
});

frontier: Heap[int64, mem(normal)] <- brodal_okasaki_min@empty({
    compare_item: compare_node_id
});

g: DirectedGraph[Edge, mem(normal)] <- graph_wbt_directed@empty({
    compare_node: compare_node_id,
    compare_edge: compare_edge,
    edge_target: edge_target
});
```

---

## What generated modules supply

Each representation module supplies:

1. Constructor and update functions (`@empty`, `@insert`, `@add_edge`, `@push`, …).
2. Representation-specific traversals required by trait `provided` blocks (especially `fold` on WBT set/map).
3. Trait `impl fn` bodies (directly or via adapters) for `required` operations.
4. Optional `validate/1` helpers for development trials (not part of public trait surface unless exported by design).

Example responsibilities:


| Module               | Trait ops                                                | Module ops                                     |
| -------------------- | -------------------------------------------------------- | ---------------------------------------------- |
| `wbt_set`            | `OrderedSet@contains`, `@size`, `@fold`, `@compare_item` | `@empty`, `@insert`, `@delete`, `@from_sorted` |
| `wbt_map`            | `OrderedMap@get`, `@fold`, `@contains_key`, …            | `@empty`, `@insert`, `@delete`, `@get`         |
| `graph_wbt_directed` | `DirectedGraph@neighbors`, `@node_count`, …              | `@empty`, `@add_edge`                          |
| `graph_csr_directed` | query ops on frozen snapshot                             | `@freeze_from_wbt`                             |
| `brodal_okasaki_min` | `Heap@peek`, `@len`                                      | `@empty`, `@push`, `@pop`, `@meld`             |


---

## Compiler obligations (not implemented)

Required compiler support before stdlib acceptance:


| #   | Obligation                                                                              | Notes                                                  |
| --- | --------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| 1   | First-argument trait dispatch from concrete receiver type                               |                                                        |
| 2   | Placeholder matching for trait `required` / `provided` signatures                       | E2092-class errors                                     |
| 3   | Constructor function-record resolution from declared collection type + field signatures | E2017-class errors                                     |
| 4   | Bracket-type witnesses for all collection families                                      | `OrderedSet`, `OrderedMap`, `Heap`, `DirectedGraph`, … |
| 5   | Checking / codegen for `provided` trait block bodies                                    |                                                        |
| 6   | Link-name mangling from resolved concrete trait impl types                              |                                                        |
| 7   | Invalid comparator atom enforcement                                                     | runtime or static                                      |


`Collectable` placeholder resolution remains **list-specific**; standard structures use constructor function records, not public `Collectable` construction.

---

## Cross-representation pipelines


| Pipeline               | Trait / module involvement              |
| ---------------------- | --------------------------------------- |
| Edge list → live graph | Fold `graph_wbt_*@add_edge`             |
| Live WBT graph → CSR   | `graph_csr_*@freeze_from_wbt`           |
| Sorted keys → WBT set  | `wbt_set@from_sorted` or fold `@insert` |
| Multiple heaps → one   | `brodal_okasaki_*@meld`                 |
| SearchTree lookup      | `SearchTree@contains_key` → WBT backing |


---

## Resolved CSR/dense ABI decision

The CSR/dense decision is closed:

1. layouts are compiler-version-private structural records;
2. public vertex IDs and internal slots are both `int64`, but remain distinct domains;
3. extents are runtime-sized internal values, not public graph type parameters;
4. CSR uses parallel neighbor/edge-data buffers, while dense uses boolean cells for unweighted graphs and one tagged optional-data sequence for attributed/weighted graphs;
5. WBT, CSR, and dense module families are distinct concrete generated types with additional attributed/weighted specializations.

---

## Benefits

- Algorithms written once per trait, shared across WBT, CSR snapshot, and dense matrix graph backends where applicable.
- Custom user structures participate via views or direct impls.
- Payload types are explicit at construction; Phase 1 vertex IDs are deliberately fixed to `int64`.
- Representation modules become implementation details behind traits.

---

## Risks

- `provided` algorithms that default to `fold` may be asymptotically fine but allocation-heavy if `neighbors` materializes large lists every call.
- Error messages must distinguish trait dispatch, constructor resolution, function-record mismatch, and payload mismatch.
- Brodal–Okasaki and WBT codegen complexity is higher than array-backed bootstrap modules.

---

## Related documents


| Document                                                                               | Role                                              |
| -------------------------------------------------------------------------------------- | ------------------------------------------------- |
| [data_structure_to_algorithms.md](data_structure_to_algorithms.md)                     | Locked algorithms and complexity                  |
| [graph_representation_design.md](../graph_representation_design.md)                    | WBT graph shapes, CSR freeze, dense matrix        |
| [balanced_tree_and_heap_design.md](../balanced_tree_and_heap_design.md)                | WBT and Brodal–Okasaki module design              |
| [btree_set_design.md](../btree_set_design.md)                                          | Ordered set/map operations (WBT; filename legacy) |
| [silica-specification.md](../silica-specification.md)                                  | §8.2.4 standard generated structures              |
| [data_structures_implementation_command.md](data_structures_implementation_command.md) | Acceptance trials and phase order                 |
