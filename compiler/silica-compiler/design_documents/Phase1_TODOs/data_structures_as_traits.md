# Standard Data Structures As Traits Proposal

Date: 2026-06-03

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

For unweighted graphs, the edge payload may be either the destination id itself
or an explicit record such as `{ to: NodeIdType }`. The explicit record form is more
uniform with weighted graphs and makes `edge_target` unambiguous.

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

## Impact On Current Phase 0.4 Work

This direction changes the meaning of Phase 0.4.

Instead of relying primarily on zero-receiver context resolution, Phase 0.4
should support:

1. First-argument trait dispatch for existing values.
2. Placeholder matching for trait required/provided functions.
3. Function-record-aware generated constructor resolution from declared collection
   type plus function-field signatures.
4. Compile-time validation that function records agree with declared
   collection type parameters.
5. Link-name mangling based on resolved concrete trait implementation types.

The existing `Collectable` placeholder work still matters, but constructor
function records become the primary way to make `Collectable` concrete for
constructors.

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

- The language may need clearer support for generic trait placeholders or
  associated type-like behavior.
- Generated constructors need function-record-aware resolution distinct from
  ordinary first-argument trait dispatch.
- Some existing generated modules and trials would need API migration.
- Provided trait algorithms over lists may need efficient traversal abstractions
  to avoid unnecessary materialization.
- Error messages must explain whether failure came from behavior dispatch,
  constructor resolution, function-record mismatch, comparator return type, or
  payload mismatch.
- Storing function fields in collection values may need a clear representation
  and linking rule in generated code.

## Open Questions

1. How should Silica spell trait-level type placeholders such as `NodeIdType`,
   `EdgePayloadType`, `KeyType`, `ValueType`, `PriorityType`, `WeightType`, and `SpaceType`?
2. Should constructor function fields be stored directly in collection values, or should
   generated specializations bake them into type-specific function bodies when
   possible?
3. Should unweighted graph edge payload be the destination id, or should it use
   an explicit `{ to: NodeIdType }` record for uniformity with weighted graphs?
4. Should algorithms prefer `neighbors(g, node)` returning a list, or
   `fold_neighbors(g, node, init, fn)` to support CSR/dense forms without list
   allocation?
5. How should invalid comparator atoms be reported in early generated modules?
6. Should generated modules continue exposing direct module functions alongside
   trait implementations during migration?

## Suggested Next Experiment

Create a small trait trial for ordered set behavior:

1. Define an `OrderedSet` trait with provided `compare_item`, `fold`,
   `contains`, and `size`.
2. Define a generated `btree_set@empty(item_functions)` constructor.
3. Require a binding such as
   `names: OrderedSet[string, mem(normal)] <- btree_set@empty({ compare_item: compare_string })`.
4. Verify the compiler rejects a function record whose comparator is not
   `fn(string, string) -> atom`.
5. Implement the representation-specific B-tree traversal used by
   `OrderedSet@fold`.
6. Verify `contains` and `size` work through the provided fold path.

Then repeat with `OrderedMap[KeyType, ValueType, SpaceType]` using both `compare_key` and
`compare_value`.
