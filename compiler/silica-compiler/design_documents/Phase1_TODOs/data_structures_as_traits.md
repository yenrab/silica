# Standard Data Structures As Traits Proposal

Date: 2026-06-03 (implementation status updated 2026-06-16)

This proposal records the Phase 1 direction for Silica standard data structures.
It replaces the earlier constructor-inference discussion with a single rule:

- Standard data-structure behavior is expressed as traits.
- Concrete generated representations implement those traits.
- Generated constructors take typed function records.
- Collection variables still declare their collection types explicitly.
- Constructor function fields are checked against the declared collection type and are
  used as compile-time type witnesses for generated implementation resolution.
- Constructor function records are inline structural values. Silica does not use
  named or aliased struct types for these records.
- Memory effects such as `mem(normal)` and `mem(writethrough)` are part of the
  collection type and must remain visible in collection declarations.

This keeps type knowledge visible at call sites while giving constructors enough
ordinary argument information to avoid fragile return-context-only inference.

Execution steps, Phase 0.5 reconciliation work, and per-family completion notes live in
[standard_data_structures_implementation_plan.md](standard_data_structures_implementation_plan.md).

## Implementation Status And Staging

This document is the **long-term authority** for trait APIs (`provided` + `fold`, full
export lists, boolean `found` shapes). The stdlib and compiler may implement that
authority in stages. Do not treat today's `required` + per-representation `impl fn`
smoke modules as the final trait shape.

### Phase 0.4 compiler — complete (set / map / heap)

| # | Obligation | Status |
| - | ---------- | ------ |
| 1 | First-argument trait dispatch | Done |
| 2 | Placeholder matching for trait required/provided signatures (E2092) | Done |
| 3 | Constructor function-record resolution and witnesses (E2017) | Done |
| 4 | Assoc-type placeholders (`ItemType`, `KeyType`, `ValueType`) on bracket collection types | Done for `OrderedSet[T, mem(S)]`, `OrderedMap[K, V, mem(S)]`, `Heap[T, mem(S)]` |
| 5 | Link-name mangling from resolved concrete trait impl types | Done |

**Not started:** bracket-type witnesses for `DirectedGraph[Node, Edge, mem(S)]`; checking
bodies of `provided` trait blocks; strict invalid-comparator-atom enforcement.

**Key modules:** `type_checker/type_checker_collections.silica`,
`type_checker/traits/type_checker_trait_placeholders.silica`, SIR qualified-call manglers.

**Trials:** `trials/standard_data_structures_phase04_addition/` (positive);
`trials/error_enforcement_addition/standard_data_structures_phase04/` (E2017, E2092, E2003).

### Phase 0.4 stdlib smoke — partial

Standard trait **names** and minimal **exports** exist under `src/standard_data_structures/`.
Set and map traits connect to generated families through **adapter modules** (same pattern as
views, but wired as `use` + `@` calls inside trait `impl fn` bodies):

| Adapter | Generated backend |
| ------- | ----------------- |
| `ordered_set_nodeid_adapter` | `btree_set_nodeid` |
| `ordered_set_csr_adapter` | `btree_set_csr` |
| `ordered_map_nodeid_adapter` | `btree_nodeid` |
| `ordered_map_csr_adapter` | `btree_csr_map` |

**Phase04 toy modules** (`graph_phase04`, `heap_phase04`, `btree_set_phase04`) support
compiler smoke only; they are not design-complete generated families.

**Stdlib batch:** `silica.config.phase04_traits` lists trait modules, adapters, and btree
backends; `make` in `src/standard_data_structures/` succeeds.

### Phase 0.5 — next (pre–Phase 1 gate)

Fix known violations before graph migration copies wrong patterns. See implementation plan
Phase 0.5:

1. Trait impls must delegate to **captured** comparators (nodeid map `compare_value`, CSR set `compare_item`).
2. `OrderedSet@size` on nodeid btree must return **item count**, not B-tree node count.
3. CSR generated values (`btree_set_csr`, `btree_csr_map`) must **store** comparators from `empty/1` in the value.

### Migration staging decisions

| Topic | Current code | This document (target) |
| ----- | ------------ | ---------------------- |
| Trait module shape | `required` + per-shape `impl fn` | `provided` + `fold` for sets/maps where specified below |
| `fold`, map `find_value`, map `size` | Not on trait exports yet | Exported; default algorithms via `fold` |
| Comparator return | `:less \| :equal \| :greater` in generated signatures | Prose uses `atom`; sum type accepted as stricter staging spelling |
| `get` / `peek` | `{ found: int64, … }`, bare `int64` peek in smoke | `{ found: boolean, value: T }` |
| Module vs trait API | Both during migration | Trait acceptance is exit criterion |
| Graph / heap acceptance | `graph_phase04`, `heap_phase04` toys | Real `graph_adj_*`, `heap_binary_*` with function records |

### Generated-family migration snapshot

| Family | Constructor record | Comparators in value | Trait wired | Notes |
| ------ | ------------------ | -------------------- | ----------- | ----- |
| `btree_set_nodeid` | Yes | Yes | Via adapter | Phase 0.5: fix `size` semantics in trait impl |
| `btree_set_csr` | Accepts record | **No** (Phase 0.5) | Via adapter | Trait CSR `compare_item` hardcodes ordering today |
| `btree_nodeid` | Yes | Yes | Via adapter | Phase 0.5: nodeid trait `compare_value` must use captured fn |
| `btree_csr_map` | Accepts record | **No** (Phase 0.5) | Via adapter | CSR trait path delegates `compare_value` correctly |
| `graph_adj_directed` | No (bootstrap) | N/A | No | Width-specialized exports; Phase 1 retarget |
| `heap_binary_min` | No (bootstrap) | N/A | No | Phase 5 retarget |

## Comparator Result

Standard ordering comparators return `atom`, not `int64`.

Valid comparator results are:

```text
:less
:equal
:greater
```

Design sketch:

```text
fn compare_string(a: string, b: string) -> atom {
    ...
}
```

The standard library must treat any other atom result as invalid comparator
behavior. The exact enforcement path can be a runtime validation failure in
early generated modules, then become stricter once effect/error handling for
standard library contracts is settled.

**Generated spelling (staging):** bootstrap and trait modules currently use the sum type
`:less | :equal | :greater` rather than bare `atom`. Witness checking and trials use that
spelling. Invalid-atom validation remains deferred (open question §5 below).

## Design Principles

1. Traits are not inheritable.
2. A concrete type may implement multiple traits.
3. Behavior traits operate on existing values.
4. Generated constructors and update functions preserve the comparison,
   extraction, and hashing functions established at construction.
5. Node ids, keys, values, priorities, edge payloads, and weights must not be
   assumed to be `int64`.
6. Counts and lengths may remain `int64`.
7. Generated representations should implement standard traits automatically.
8. Programmers may implement standard traits for custom structures.
9. Algorithms should depend on behavior traits, not generated representation
   modules.
10. Generated constructors must use typed function records instead of relying on
    hidden return-context-only inference.
11. Collection variable declarations remain explicit; constructor function
    records check and specialize, but do not hide the collection type.

## Constructor Function Record Rule

Every standard collection constructor takes an inline function record. The record
contains one comparator for each stored or searchable type that the collection
needs to compare, plus extractor functions when one stored type
contains another type.

The compiler uses function-field signatures as type witnesses.

Memory effects are also type parameters. They are usually not witnessed by a
comparator; they come from the declared collection type and from the selected
generated representation.

For example:

```text
names: OrderedSet[string, mem(normal)] <- btree_set@empty({
    compare_item: compare_string
});
```

The compiler checks:

- `names` is declared as `OrderedSet[string, mem(normal)]`.
- The inline record's `compare_item` field has type
  `fn(string, string) -> atom`.
- The record therefore agrees with the declared collection type.
- The generated concrete `btree_set` implementation can be specialized for
  `string` in `mem(normal)`.

If the function record disagrees with the declared type, the program fails at compile
time:

```text
names: OrderedSet[string, mem(normal)] <- btree_set@empty({
    compare_item: fn(a: int64, b: int64) -> atom {
        case a < b of {
            true -> :less;
            false ->
                case a == b of {
                    true -> :equal;
                    false -> :greater
                }
        }
    }
});
```

This is an error because the inline comparator is not
`fn(string, string) -> atom`.

**Assoc-type placeholders (implemented):** trait required signatures and call sites use
placeholders `ItemType`, `KeyType`, `ValueType`, and `SpaceType` (via `mem(SpaceType)` on
bracket collection types). The compiler resolves them from the declared collection type and
from constructor-record function-field witnesses. Module-level placeholder consistency across
multiple impls is still permissive; call-site checking is strict.

## Trait Categories

### Behavior Traits

Behavior traits define operations over an existing structure value.

Examples:

- `DirectedGraph`
- `UndirectedGraph`
- `WeightedGraph`
- `OrderedSet`
- `OrderedMap`
- `Heap`
- `PriorityQueue`
- `Tree`
- `SearchTree`

These traits dispatch from a receiver-like first argument:

```text
DirectedGraph@neighbors(g, node_id)
OrderedSet@contains(set, key)
OrderedMap@contains_key(map, key)
Heap@peek(heap)
```

The compiler resolves the implementation from the concrete type of `g`, `set`,
`map`, or `heap`.

### Generated Construction And Updates

Construction and incremental updates are generated module functions, not
separate trait surfaces. They include factory-style operations such as `empty`,
plus update operations such as `insert`, `push`, or `add_edge`.

These operations must preserve the comparison, extraction, and hashing functions
established at construction.

```text
names: OrderedSet[string, mem(normal)] <- btree_set@empty({
    compare_item: compare_string
});
names2: OrderedSet[string, mem(normal)] <- btree_set@insert(names, "Ada");
```

The `insert` call is checked from the receiver type. The inserted value must be
`string`; the comparator is already part of the typed set value.

## Set Traits

An ordered set has one stored and searchable type, `ItemType`.

Constructor function record:

```text
{
    compare_item: fn(ItemType, ItemType) -> atom
}
```

Behavior:

```text
export trait OrderedSet;

export contains/2;
export size/1;
export fold/3;
export compare_item/3;

provided {
    fn compare_item[SetType, ItemType](
        set: SetType,
        a: ItemType,
        b: ItemType
    ) -> atom {
        // Provided by calling the compare_item function captured in the set
        // value during construction.
    }

    fn fold[SetType, ItemType, AccType](
        set: SetType,
        init: AccType,
        step: fn(AccType, ItemType) -> AccType
    ) -> AccType {
        // Provided by the representation-specific trait or generated module.
        // A B-tree, CSR set, list-backed set, and custom view each supply the
        // traversal algorithm that matches its storage layout. The caller
        // supplies only init and step.
    }

    fn contains[SetType, ItemType](set: SetType, item: ItemType) -> boolean {
        // Provided by folding over items and using compare_item(set, current, item).
    }

    fn size[SetType](set: SetType) -> int64 {
        // Provided by folding over items and incrementing a count.
    }
}
```

Generated construction and update functions:

```text
export empty/1;
export insert/2;
export delete/2;

fn empty[ItemType, SpaceType](
    item_functions: { compare_item: fn(ItemType, ItemType) -> atom }
) -> OrderedSet[ItemType, SpaceType];

fn insert[ItemType, SpaceType](
    set: OrderedSet[ItemType, SpaceType],
    item: ItemType
) -> OrderedSet[ItemType, SpaceType];
```

Delete can remain deferred if the representation design is not ready.

### Staging: current `OrderedSet.silica`

Target shape is the `provided` block above. Current stdlib staging:

```text
export trait OrderedSet;
export contains/2;
export size/1;
export compare_item/3;    // fold/3 not exported yet

required {
    fn contains(set: OrderedSet, item: ItemType) -> boolean;
    fn size(set: OrderedSet) -> int64;
    fn compare_item(set: OrderedSet, left: ItemType, right: ItemType) -> :less | :equal | :greater;
}
```

Per-representation `impl fn` blocks cover: phase04 toy record, nodeid btree (via
`ordered_set_nodeid_adapter`), CSR btree (via `ordered_set_csr_adapter`). Migrate to
`provided` + representation-supplied `fold` after Phase 0.5 and the first `fold` impl lands.

## Map Traits

Maps have separate key and value types. To keep collection creation uniform and
to support slow unordered value search when requested, ordered maps use two
comparators:

```text
{
    compare_key: fn(KeyType, KeyType) -> atom,
    compare_value: fn(ValueType, ValueType) -> atom
}
```

The primary map representation is still key-indexed. Value search is allowed to
be linear and unordered unless a specialized value-indexed or bidirectional
representation is selected later.

Behavior:

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
    fn compare_key[MapType, KeyType](
        map: MapType,
        a: KeyType,
        b: KeyType
    ) -> atom {
        // Provided by calling the compare_key function captured in the map
        // value during construction.
    }

    fn compare_value[MapType, ValueType](
        map: MapType,
        a: ValueType,
        b: ValueType
    ) -> atom {
        // Provided by calling the compare_value function captured in the map
        // value during construction.
    }

    fn fold[MapType, KeyType, ValueType, AccType](
        map: MapType,
        init: AccType,
        step: fn(AccType, KeyType, ValueType) -> AccType
    ) -> AccType {
        // Provided by the representation-specific trait or generated module.
        // The caller supplies only init and step.
    }

    fn get[MapType, KeyType, ValueType](
        map: MapType,
        key: KeyType
    ) -> { found: boolean, value: ValueType } {
        // Default provided behavior can fold entries and use compare_key(map, current, key).
        // KeyType-indexed representations may also provide a faster specialized get.
    }

    fn contains_key[MapType, KeyType, ValueType](
        map: MapType,
        key: KeyType
    ) -> boolean {
        // Provided from get(map, key).found.
    }

    fn find_value[MapType, KeyType, ValueType](
        map: MapType,
        value: ValueType
    ) -> { found: boolean, key: KeyType } {
        // Provided by folding entries and using compare_value(map, current, value).
    }

    fn size[MapType](map: MapType) -> int64 {
        // Provided by folding over entries and incrementing a count.
    }
}
```

Generated construction and update functions:

```text
export empty/1;
export insert/3;

fn empty[KeyType, ValueType, SpaceType](
    map_functions: {
        compare_key: fn(KeyType, KeyType) -> atom,
        compare_value: fn(ValueType, ValueType) -> atom
    }
) -> OrderedMap[KeyType, ValueType, SpaceType];

fn insert[KeyType, ValueType, SpaceType](
    map: OrderedMap[KeyType, ValueType, SpaceType],
    key: KeyType,
    value: ValueType
) -> {
    map: OrderedMap[KeyType, ValueType, SpaceType],
    inserted: boolean,
    replaced: boolean
};
```

### Staging: current `OrderedMap.silica`

Target exports include `find_value/2`, `size/1`, and `fold/3`. Current stdlib staging:

```text
export trait OrderedMap;
export contains_key/2;
export get/2;
export compare_key/3;
export compare_value/3;

required {
    fn contains_key(map: OrderedMap, key: KeyType) -> boolean;
    fn get(map: OrderedMap, key: KeyType) -> { found: int64, value: ValueType };
    fn compare_key(...);
    fn compare_value(...);
}
```

`get` uses `{ found: int64, … }` instead of `{ found: boolean, … }`. Nodeid and CSR btree
shapes wired via `ordered_map_nodeid_adapter` / `ordered_map_csr_adapter`. Phase 0.5 fixes
listed in Implementation Status.

## Heap And Priority Queue Traits

Simple heaps have one stored and ordered type, `ItemType`.

Constructor function record:

```text
{
    compare_item: fn(ItemType, ItemType) -> atom
}
```

Behavior:

```text
export trait Heap;

export len/1;
export peek/1;

required {
    fn len(heap: Heap) -> int64;
    fn peek[HeapType, ItemType](heap: HeapType) -> { found: boolean, value: ItemType };
}
```

Generated construction and update functions:

```text
export empty/1;
export push/2;
export pop/1;

fn empty[ItemType, SpaceType](
    item_functions: { compare_item: fn(ItemType, ItemType) -> atom }
) -> Heap[ItemType, SpaceType];

fn push[ItemType, SpaceType](
    heap: Heap[ItemType, SpaceType],
    item: ItemType
) -> Heap[ItemType, SpaceType];

fn pop[ItemType, SpaceType](
    heap: Heap[ItemType, SpaceType]
) -> { found: boolean, value: ItemType, heap: Heap[ItemType, SpaceType] };
```

Priority queues have an element type and a priority type. The constructor
function record should compare both so the collection family keeps the same shape
as sets, maps, and graphs:

```text
{
    compare_item: fn(ItemType, ItemType) -> atom,
    compare_priority: fn(PriorityType, PriorityType) -> atom
}
```

If the representation stores only `(priority, item)` entries, no extractor is
needed. If the priority is embedded in the item, add:

```text
priority_of: fn(ItemType) -> PriorityType
```

### Staging: current `Heap.silica` and `PriorityQueue.silica`

**Heap (target vs staging):** target `peek` returns `{ found: boolean, value: ItemType }`;
staging exports `compare_priority/3` and implements `peek` as bare `int64` on `heap_phase04`
record shape only.

**PriorityQueue:** trait module exists with `peek_priority/1` and `peek_value/1` smoke impls
on the heap_phase04-shaped record; no generated priority-queue family wired yet.

## Tree Traits

Tree traits should distinguish:

- General tree shape.
- Search tree ordering.
- B-tree-specific invariants.
- Map-like key/value behavior.

Search trees follow the same constructor function record rule as ordered sets:

```text
{
    compare_item: fn(ItemType, ItemType) -> atom
}
```

Search map trees follow the same constructor function record rule as ordered
maps:

```text
{
    compare_key: fn(KeyType, KeyType) -> atom,
    compare_value: fn(ValueType, ValueType) -> atom
}
```

### Staging: `Tree.silica` and `SearchTree.silica`

Minimal E2092 smoke only (`node_count` on graph-shaped record; `contains_key` on set-shaped
record). No generated tree family acceptance yet.

## Graph Traits

Graphs have at least two independent types:

- `NodeIdType`
- `EdgePayloadType`

Weighted graphs add:

- `WeightType`

A directed graph constructor function record should include comparators for
stored/searchable types and an extractor that identifies the target node inside
an edge payload:

```text
{
    compare_node: fn(NodeIdType, NodeIdType) -> atom,
    compare_edge: fn(EdgePayloadType, EdgePayloadType) -> atom,
    edge_target: fn(EdgePayloadType) -> NodeIdType
}
```

The node comparator defines node identity and ordering. The edge comparator
defines edge identity and supports edge lookup, duplicate detection, validation,
and deterministic traversal where the representation cares about edge order.

Counts remain `int64`:

```text
node_count(g) -> int64
edge_count(g) -> int64
```

Endpoint ids are generic:

```text
neighbors(g, node_id: NodeIdType) -> List[EdgePayloadType, SpaceType]
edge_target(edge: EdgePayloadType) -> NodeIdType
```

### DirectedGraph

Design sketch:

```text
export trait DirectedGraph;

export node_count/1;
export edge_count/1;
export neighbors/2;
export edge_target/1;
export out_degree/2;
export has_edge/3;
export reachable/3;

required {
    fn node_count(g: DirectedGraph) -> int64;

    fn edge_count(g: DirectedGraph) -> int64;

    fn neighbors[GraphType, NodeIdType, EdgePayloadType, SpaceType](
        g: GraphType,
        node_id: NodeIdType
    ) -> List[EdgePayloadType, SpaceType];

    fn edge_target[EdgePayloadType, NodeIdType](
        edge: EdgePayloadType
    ) -> NodeIdType;
}
```

Provided functions can supply:

- `out_degree`
- `has_edge`
- `reachable`
- `max_out_degree`
- `total_out_degree_sum`

Generated implementations should use the graph value's captured `compare_node`,
`compare_edge`, and `edge_target` functions where equality, ordering, or payload
interpretation is needed.

### WeightedGraph

Weighted graph constructor function records extend directed graph function
records:

```text
{
    compare_node: fn(NodeIdType, NodeIdType) -> atom,
    compare_edge: fn(EdgePayloadType, EdgePayloadType) -> atom,
    edge_target: fn(EdgePayloadType) -> NodeIdType,
    edge_weight: fn(EdgePayloadType) -> WeightType,
    compare_weight: fn(WeightType, WeightType) -> atom
}
```

Algorithms that combine weights may require additional fields:

```text
zero_weight: WeightType,
add_weight: fn(WeightType, WeightType) -> WeightType
```

Those fields belong in algorithm-specific function records when they are not
needed by the graph structure itself.

### Directed Graph Construction And Updates

```text
export empty/2;
export add_edge/3;

fn empty[NodeIdType, EdgePayloadType, SpaceType](
    graph_functions: {
        compare_node: fn(NodeIdType, NodeIdType) -> atom,
        compare_edge: fn(EdgePayloadType, EdgePayloadType) -> atom,
        edge_target: fn(EdgePayloadType) -> NodeIdType
    },
    node_count: int64
) -> DirectedGraph[NodeIdType, EdgePayloadType, SpaceType];

fn add_edge[NodeIdType, EdgePayloadType, SpaceType](
    g: DirectedGraph[NodeIdType, EdgePayloadType, SpaceType],
    from_id: NodeIdType,
    edge: EdgePayloadType
) -> DirectedGraph[NodeIdType, EdgePayloadType, SpaceType];
```

**Unweighted edge payload (resolved — Option A):** for unweighted graphs,
`EdgePayloadType = NodeIdType`. Neighbor lists store destination node ids;
`add_edge(g, from_id, to_id)` passes the destination as the edge payload;
`edge_target(edge)` returns `edge` unchanged; `compare_edge` compares destination
ids. Weighted and attributed graphs continue to use explicit neighbor records
(`{ to: NodeIdType, weight: WeightType }` or `{ to: NodeIdType, data: EdgePayloadType }`)
per `graph_representation_design.md` §3.6. The alternate `{ to: NodeIdType }`-only
record form for unweighted graphs was not adopted.

### Staging: current graph trait modules

| Trait | Target required ops | Current staging |
| ----- | ------------------- | --------------- |
| `DirectedGraph` | `node_count`, `edge_count`, `neighbors`, `edge_target` | `node_count`, `edge_count`, `has_edge` only; impl on `graph_phase04` toy |
| `UndirectedGraph` | Same pattern as directed + undirected `has_edge` | Smoke impl on `graph_phase04` record |
| `WeightedGraph` | `weight_at` and weight-aware ops | Smoke `weight_at` on `graph_phase04` record |

`graph_adj_directed.silica` remains bootstrap (width exports, no function record). Phase 1
retargets adjacency to constructor records and replaces `graph_phase04` for acceptance.

## View And Adapter Values

Views and adapters remain useful, but they should follow the same constructor
function record rule instead of introducing a separate constructor style.

A view reads existing user storage through functions provided at creation:

```text
view: {
    graph: GraphType,
    neighbors_of: fn(GraphType, NodeIdType) -> List[EdgePayloadType, SpaceType]
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

The inline function record passed to `DirectedGraph@view` provides comparison
and edge-target semantics. The view value exposes only the storage-specific
access function:

```text
neighbors_of: fn(GraphType, NodeIdType) -> List[EdgePayloadType, SpaceType]
```

The adapter path is the low-code path for custom structures. Direct trait
implementation remains better when the user wants their concrete type to behave
as a graph, set, map, heap, or tree everywhere without constructing a view value.

**In use today:** set/map trait modules use thin adapter modules
(`ordered_set_nodeid_adapter`, etc.) that forward to generated `@` operations on inner
record shapes. This matches the adapter idea but is wired as stdlib `impl fn` glue rather
than user-facing `OrderedSet@view(...)`.

## What Generated Modules Would Supply

Generated modules such as `graph_adj_directed.silica`,
`btree_set_nodeid.silica`, `btree_nodeid.silica`, and
`heap_binary_min.silica` would supply implementations for their concrete inline
record types.

For example, generated `graph_adj_directed` can supply:

- `DirectedGraph@node_count`
- `DirectedGraph@edge_count`
- `DirectedGraph@neighbors`
- `DirectedGraph@edge_target`
- `graph_adj_directed@empty`
- `graph_adj_directed@add_edge`
- representation-specific storage checking helpers, if needed outside the trait
  surface

Then provided trait functions can supply graph algorithms once per trait.

For maps, generated modules can supply:

- `OrderedMap@contains_key`
- `OrderedMap@find_value`
- `OrderedMap@get`
- `btree_map@empty`
- `btree_map@insert`

`find_value` may be a linear traversal in ordinary key-indexed maps.

## What Programmers Would Supply

For standard generated structures, programmers supply inline function records and
explicit collection variable declarations:

```text
users: OrderedMap[string, User, mem(normal)] <- btree_map@empty({
    compare_key: compare_string,
    compare_value: compare_user
});
```

For a custom graph, the programmer can either implement the required trait
functions directly or create a view:

```text
view: {
    graph: MyGraph,
    neighbors_of: fn(MyGraph, MyNodeId) -> List[MyEdge, mem(normal)]
} <- DirectedGraph@view(
    my_graph,
    {
        compare_node: compare_my_node_id,
        compare_edge: compare_my_edge,
        edge_target: my_edge_target
    },
    my_neighbors_of
);
```

## Constructor Direction

Replace ambiguous `empty(...)`, `empty_like(...)`, and sample-value-only
constructors with constructors that take inline function records:

```text
names: OrderedSet[string, mem(normal)] <- btree_set@empty({
    compare_item: compare_string
});

users: OrderedMap[string, User, mem(normal)] <- btree_map@empty({
    compare_key: compare_string,
    compare_value: compare_user
});

frontier: Heap[NodeIdType, mem(normal)] <- heap_binary_min@empty({
    compare_item: compare_node_id
});

g: DirectedGraph[NodeIdType, Edge, mem(normal)] <-
    graph_adj_directed@empty({
        compare_node: compare_node_id,
        compare_edge: compare_edge,
        edge_target: edge_target
    }, 3);
```

Explicit collection type declarations are required at bindings where constructor
resolution would otherwise hide the concrete collection type. Return types on
functions should likewise declare the collection type.

## Phase 0.4 Compiler Obligations

Phase 0.4 meant trait-oriented constructor records and dispatch, not zero-receiver-only
inference. Required compiler support:

1. First-argument trait dispatch for existing values — **done**
2. Placeholder matching for trait required/provided functions — **done** (E2092)
3. Function-record-aware constructor resolution from declared collection type plus
   function-field signatures — **done** (E2017)
4. Compile-time validation that function records agree with collection type parameters —
   **done** for set/map/heap bracket types
5. Link-name mangling based on resolved concrete trait implementation types — **done**

The existing `Collectable` placeholder work remains list-specific coverage; constructor
function records are the standard data-structure construction model.

**Still open for Phase 1+:** graph bracket witnesses; `provided`-block implementation;
invalid comparator atoms.

## Benefits

- Algorithms can be written against traits rather than generated concrete
  modules.
- Custom user structures can participate in standard algorithms.
- Node ids, keys, values, priorities, edge payloads, and weights are no longer
  accidentally fixed to `int64`.
- Constructor calls become explicit and easier for the compiler to resolve.
- The standard library can provide common algorithms once per trait instead of
  once per representation.
- Generated modules can become representation implementations rather than the
  only public API.
- Collection type declarations remain visible in user code.
- Comparator semantics are clear because comparators return `:less`, `:equal`,
  or `:greater`.

## Costs And Risks

- Generic trait placeholders / assoc-type behavior — **partially addressed** for set/map/heap;
  graph types remain.
- Function-record-aware constructor resolution — **implemented** (E2017); error-message clarity
  still improvable.
- API migration of bootstrap modules and old trials — **in progress**; dual API during migration.
- Provided trait algorithms over lists may need efficient traversal abstractions
  (`fold_neighbors`, representation `fold`) to avoid unnecessary materialization.
- Error messages must explain whether failure came from behavior dispatch,
  constructor resolution, function-record mismatch, comparator return type, or
  payload mismatch.
- Function fields in collection values — **in use** for nodeid set/map; linking rule proven in
  phase04 trials; CSR storage gap is Phase 0.5 work.

## Open Questions

1. **Trait-level type placeholders** — Partially resolved: `ItemType`, `KeyType`, `ValueType`
   used as assoc-type placeholders; `NodeIdType`, `EdgePayloadType`, etc. follow the same
   pattern when graph bracket witnesses land in Phase 1.
2. **Store function fields in values vs bake into specializations** — Staging choice: store in
   value for nodeid set/map; CSR families must catch up in Phase 0.5. Open for performance
   specialization later.
3. **Unweighted graph edge payload** — **Resolved (Option A):** `EdgePayloadType =
   NodeIdType` for unweighted graphs; bare destination id in neighbor lists;
   `edge_target` is identity. Weighted graphs use explicit `{ to, … }` neighbor records.
4. **`neighbors` list vs `fold_neighbors`** — Still open; list form is the initial trait
   surface; fold variant may follow for CSR/dense graphs.
5. **Invalid comparator atoms** — Deferred; sum-type comparators catch invalid shapes at
   compile time where used; runtime validation path TBD.
6. **Dual module + trait API during migration** — Resolved for staging: **yes**, both coexist;
   trait-oriented trials are acceptance; old `*_addition` trials remain reference until
   retargeted.

## Suggested Next Work

Phase 0.4 compiler smoke experiments below are **largely complete**. Next execution order
(see implementation plan):

1. **Phase 0.5** — comparator preservation and trait semantics fixes (set/map CSR and nodeid).
2. **Phase 1** — `graph_adj_directed@empty({ compare_node, compare_edge, edge_target }, n)`;
   `DirectedGraph[Node, Edge, mem(S)]` witnesses; trait impls for `neighbors` and
   `edge_target`; retire `graph_phase04` from acceptance trials.
3. **Trait richness** — after first representation implements `fold`, migrate `OrderedSet` /
   `OrderedMap` toward `provided` + `fold` as specified in Set/Map Traits above.

Completed smoke experiments (for audit):

1. `OrderedSet` trait with staging `required` + `impl fn` (not yet `provided` + `fold`).
2. `btree_set_nodeid@empty(item_functions)` and witness trials (`ordered_set_witness_int64`,
   `ordered_set_witness_string`).
3. Compiler rejects mismatched function records (E2017) and trait impl mismatches (E2092).
4. `OrderedSet@contains` / `@compare_item` / `@size` on nodeid btree via trait dispatch.
5. Link-name mangling golden (`trait_link_name_dual_impl`).

Repeat fold-based provided algorithms when B-tree set `fold` is implemented per representation.
