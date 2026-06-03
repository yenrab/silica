# Standard Data Structures As Traits Proposal

Date: 2026-06-03

This proposal explores a trait-oriented direction for Silica standard data structures. It is not yet an implementation plan. It records a possible design shift away from treating generated graph, tree, heap, and set modules only as concrete modules with context-resolved constructors.

The core idea is:

- Standard data-structure **behavior** should be expressed as traits.
- Concrete generated representations should implement those traits.
- User-defined structures should be able to implement the same traits.
- Constructors and incremental updates should live in builder traits because they do not dispatch from an existing structure value in the same way behavior methods do.

## Motivation

The current generated data-structure plan relies on context resolution for calls such as:

```silica
g: SomeGraphShape <- graph_adj_directed@empty(3);
```

This requires the compiler to infer the concrete graph type from the binding or function return context. That inference is valid, but fragile when the callee declaration itself has a concrete return type such as:

```silica
fn empty(initial_node_count: int64) -> {
    node_count: int64,
    edge_count: int64,
    n0_neighbors: List[int64, mem(normal)],
    n1_neighbors: List[int64, mem(normal)],
    n2_neighbors: List[int64, mem(normal)]
}
```

Such a function cannot become a weighted or non-`int64` graph through inference. The return type has already fixed the graph shape.

A trait-oriented model moves most operations to first-argument dispatch:

```silica
DirectedGraph@has_edge(g, from_id, to_id)
```

Here, the compiler can resolve the implementation from the concrete type of `g`. This is simpler than resolving from a zero-receiver constructor.

## Design Principles

1. Traits are not inheritable.
2. A concrete type may implement multiple traits.
3. Behavior traits operate on existing values.
4. Builder traits create or update values and therefore require different resolution rules from behavior traits.
5. Node ids, keys, priorities, and payloads must not be assumed to be `int64`.
6. Counts and lengths may remain `int64`.
7. Generated representations should implement standard traits automatically.
8. Programmers may implement standard traits for custom structures.
9. Algorithms should depend on behavior traits, not on generated representation modules.
10. Constructors should avoid relying solely on return-context inference when a sample value can make the type clear.

## Trait Categories

### Behavior Traits

Behavior traits define operations over an existing structure value.

Examples:

- `DirectedGraph`
- `UndirectedGraph`
- `WeightedGraph`
- `OrderedSet`
- `Map`
- `Heap`
- `PriorityQueue`
- `Tree`
- `SearchTree`

These traits dispatch from a receiver-like first argument:

```silica
DirectedGraph@neighbors(g, node_id)
OrderedSet@contains(set, key)
Heap@peek(heap)
```

The compiler can resolve the implementation from `g`, `set`, or `heap`.

### Builder Traits

Builder traits describe creation and incremental construction. They include factory-style operations such as `empty_like` and `with_first_edge`, plus update operations such as `insert`, `push`, or `add_edge`.

Examples:

- `DirectedGraphBuilder`
- `SetBuilder`
- `MapBuilder`
- `HeapBuilder`

These should not be folded into behavior traits because calls such as:

```silica
DirectedGraphBuilder@empty_like(sample_node_id, sample_edge, node_count)
```

do not have an existing graph value to inspect. Resolution comes from sample arguments, the builder adapter, and/or an expected binding type.

Builders may return immutable values after each operation or use an explicit mutable/builder representation when the design document allows it.

## Graph Traits

Graph ids are not always `int64`. The graph traits must treat node ids as a type variable or placeholder.

Counts may remain `int64`:

```silica
node_count(g) -> int64
edge_count(g) -> int64
```

Endpoint ids should be generic:

```silica
neighbors(g, node_id: NodeId) -> List[EdgePayload, Space]
edge_target(edge: EdgePayload) -> NodeId
```

### DirectedGraph

Design sketch:

```silica
// directed_graph.silica

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

    fn neighbors[NodeId, EdgePayload, Space](
        g: DirectedGraph,
        node_id: NodeId
    ) -> List[EdgePayload, Space];

    fn edge_target[NodeId, EdgePayload](
        edge: EdgePayload
    ) -> NodeId;
}

provided {
    fn out_degree[NodeId, EdgePayload, Space](
        g: DirectedGraph,
        node_id: NodeId
    ) -> int64 {
        length[EdgePayload, Space](DirectedGraph@neighbors(g, node_id))
    }

    fn has_edge[NodeId, EdgePayload, Space](
        g: DirectedGraph,
        from_id: NodeId,
        to_id: NodeId
    ) -> int64 {
        DirectedGraph@has_edge_in_neighbors(
            DirectedGraph@neighbors(g, from_id),
            to_id
        )
    }

    fn has_edge_in_neighbors[NodeId, EdgePayload, Space](
        edges: List[EdgePayload, Space],
        to_id: NodeId
    ) -> int64 {
        case edges of {
            []: List[EdgePayload, Space] -> 0;
            [edge: EdgePayload, rest: List[EdgePayload, Space]] ->
                case DirectedGraph@edge_target(edge) == to_id of {
                    true -> 1;
                    false -> DirectedGraph@has_edge_in_neighbors(rest, to_id)
                }
        }
    }
}
```

This is a design sketch. Current parser/type-system support may need adjustment before generic trait parameters like `NodeId`, `EdgePayload`, and `Space` can be expressed exactly this way.

### WeightedGraph

Weighted graph behavior should not assume the weight type is `int64`.

```silica
export trait WeightedGraph;

export edge_weight/1;
export weight_at/3;

required {
    fn edge_weight[EdgePayload, Weight](edge: EdgePayload) -> Weight;
}

provided {
    fn weight_at[Graph, NodeId, EdgePayload, Space, Weight](
        g: Graph,
        from_id: NodeId,
        to_id: NodeId
    ) -> { found: int64, weight: Weight } {
        // Traverse DirectedGraph@neighbors(g, from_id), compare edge_target(edge),
        // return edge_weight(edge) when found.
    }
}
```

A weighted adjacency graph with string node ids might implement:

```silica
impl fn edge_target(edge: { to: string, weight: int64 }) -> string {
    edge.to
}

impl fn edge_weight(edge: { to: string, weight: int64 }) -> int64 {
    edge.weight
}
```

### DirectedGraphBuilder

Constructors and edge insertion helpers belong in a separate builder trait because no graph value exists yet for `empty_like`, and update operations should share the same type-witness machinery.

```silica
export trait DirectedGraphBuilder;

export empty_like/3;
export with_first_edge/3;

required {
    fn empty_like[NodeId, EdgePayload, Space](
        sample_node_id: NodeId,
        sample_edge: EdgePayload,
        node_count: int64
    ) -> DirectedGraphBuilder;

    fn with_first_edge[NodeId, EdgePayload, Space](
        node_count: int64,
        from_id: NodeId,
        edge: EdgePayload
    ) -> DirectedGraphBuilder;
}
```

The concrete return type still needs to be resolved from the implementing builder or expected binding type. The important improvement is that `sample_node_id` and `sample_edge` expose `NodeId` and `EdgePayload` directly, reducing reliance on return-context-only inference.

Example calls:

```silica
g0 <- graph_adj_directed@empty_like("A", { to: "B", weight: 0 }, 3);
g1 <- graph_adj_directed@with_first_edge(3, "A", { to: "B", weight: 42 });
```

For unweighted graphs:

```silica
g0 <- graph_adj_directed@empty_like(0, 0, 3);
g1 <- graph_adj_directed@with_first_edge(3, 0, 1);
```

Open question: `with_first_edge/3` for unweighted graphs can use the edge payload as the destination id, while weighted graphs use a record payload. Alternatively, unweighted and weighted builders can have separate operation names to avoid overloading too much meaning into `EdgePayload`.

## Set Traits

### OrderedSet

Set keys are not always `int64`. They should be `Key`, with a comparator requirement or a `Comparable` constraint.

```silica
export trait OrderedSet;

export contains/2;
export size/1;
export validate/1;

required {
    fn contains[Set, Key](set: Set, key: Key) -> int64;
    fn size(set: OrderedSet) -> int64;
    fn validate(set: OrderedSet) -> int64;
}
```

### OrderedSetBuilder

```silica
export trait OrderedSetBuilder;

export empty_like/1;
export insert/2;
export delete/2;

required {
    fn empty_like[Key](sample_key: Key) -> OrderedSetBuilder;
    fn insert[Set, Key](set: Set, key: Key) -> Set;
}
```

Delete can remain deferred if the representation design is not ready.

## Map Traits

Maps have separate key and value types.

```silica
export trait Map;

export contains/2;
export get/2;
export size/1;
export validate/1;

required {
    fn contains[MapType, Key](map: MapType, key: Key) -> int64;
    fn get[MapType, Key, Value](map: MapType, key: Key) -> { found: int64, value: Value };
    fn size(map: Map) -> int64;
    fn validate(map: Map) -> int64;
}
```

Builder:

```silica
export trait MapBuilder;

export empty_like/2;
export insert/3;

required {
    fn empty_like[Key, Value](sample_key: Key, sample_value: Value) -> MapBuilder;
    fn insert[MapType, Key, Value](map: MapType, key: Key, value: Value) -> {
        map: MapType,
        inserted: int64,
        replaced: int64
    };
}
```

## Heap And Priority Queue Traits

Heaps have element and priority types. In simple heaps, the element itself may be the priority.

```silica
export trait Heap;

export len/1;
export peek/1;
export validate/1;

required {
    fn len(heap: Heap) -> int64;
    fn peek[HeapType, Element](heap: HeapType) -> { found: int64, value: Element };
    fn validate(heap: Heap) -> int64;
}
```

Builder:

```silica
export trait HeapBuilder;

export empty_like/1;
export push/2;
export pop/1;

required {
    fn empty_like[Element](sample_element: Element) -> HeapBuilder;
    fn push[HeapType, Element](heap: HeapType, element: Element) -> HeapType;
    fn pop[HeapType, Element](heap: HeapType) -> { found: int64, value: Element, heap: HeapType };
}
```

Priority heaps:

```silica
export trait PriorityQueue;

export push_with_priority/3;
export peek_priority/1;

required {
    fn push_with_priority[Queue, Element, Priority](
        q: Queue,
        value: Element,
        priority: Priority
    ) -> Queue;

    fn peek_priority[Queue, Priority](
        q: Queue
    ) -> { found: int64, priority: Priority };
}
```

## Trees

Tree traits should distinguish:

- General tree shape.
- Search tree ordering.
- B-tree-specific invariants.
- Map-like key/value behavior.

Search keys should be generic and should require `Comparable` or an explicit comparator function.

```silica
export trait SearchTree;

export contains/2;
export validate/1;

required {
    fn contains[Tree, Key](tree: Tree, key: Key) -> int64;
    fn validate(tree: SearchTree) -> int64;
}
```

Map tree:

```silica
export trait SearchMapTree;

export get/2;
export insert/3;

required {
    fn get[Tree, Key, Value](tree: Tree, key: Key) -> { found: int64, value: Value };
    fn insert[Tree, Key, Value](tree: Tree, key: Key, value: Value) -> {
        tree: Tree,
        inserted: int64,
        replaced: int64
    };
}
```

## What Generated Modules Would Supply

Generated modules such as `graph_adj_directed.silica`, `btree_set_nodeid.silica`, `btree_nodeid.silica`, and `heap_binary_min.silica` would supply implementations for their concrete inline record types.

For example, generated `graph_adj_directed` can supply:

- `DirectedGraph@node_count`
- `DirectedGraph@edge_count`
- `DirectedGraph@neighbors`
- `DirectedGraph@edge_target`
- `DirectedGraphBuilder@empty_like`
- `DirectedGraphBuilder@with_first_edge`
- representation-specific `validate_storage`

Then provided trait functions can supply:

- `DirectedGraph@out_degree`
- `DirectedGraph@has_edge`
- `DirectedGraph@reachable`
- `DirectedGraph@max_out_degree`
- `DirectedGraph@total_out_degree_sum`

## What Programmers Would Supply

For a custom graph, the programmer supplies only the representation-specific pieces:

```silica
impl fn node_count(g: MyGraph) -> int64 { ... }
impl fn edge_count(g: MyGraph) -> int64 { ... }
impl fn neighbors(g: MyGraph, node_id: MyNodeId) -> List[MyEdge, mem(normal)] { ... }
impl fn edge_target(edge: MyEdge) -> MyNodeId { ... }
```

If the programmer wants construction helpers:

```silica
impl fn empty_like(sample_node: MyNodeId, sample_edge: MyEdge, node_count: int64) -> MyGraph { ... }
impl fn with_first_edge(node_count: int64, from_id: MyNodeId, edge: MyEdge) -> MyGraph { ... }
```

For maps and sets, programmers supply comparison-sensitive operations or an explicit comparator path.

## Lambda-Supplied Operations

Some behavior should be passed as lambdas instead of fixed into traits.

Good lambda candidates:

- `compare_keys(a, b) -> int64`
- `edge_cost(edge) -> Cost`
- `merge_edge(old, new) -> EdgePayload`
- `visit_node(node_id) -> unit`
- `visit_edge(from_id, edge) -> unit`
- `priority_of(value) -> Priority`
- `combine_values(old, new) -> Value`

This avoids overfitting standard traits to one interpretation of payloads.

## View And Adapter Values

A promising refinement is to let users bundle the small representation-specific lambdas into a **view** or **adapter** value. The view acts as a typed bridge between arbitrary user storage and supplied standard algorithms.

Instead of requiring the programmer to implement every required trait function, the programmer supplies a few lambdas:

```silica
neighbors_of(g, node_id) -> List[EdgePayload, Space]
edge_target(edge) -> NodeId
node_equals(a, b) -> boolean
```

Those lambdas define the relevant types:

- `Graph`
- `NodeId`
- `EdgePayload`
- `Space`

For weighted graphs, the adapter can also include:

```silica
edge_weight(edge) -> Weight
```

The algorithm code remains generic. The user only explains how to read their storage.

### Directed Graph View

Design sketch:

```silica
view: {
    graph: Graph,
    neighbors_of: fn(Graph, NodeId) -> List[EdgePayload, Space],
    edge_target: fn(EdgePayload) -> NodeId,
    node_equals: fn(NodeId, NodeId) -> boolean
} <- DirectedGraph@view(graph, neighbors_of, edge_target, node_equals);
```

Then supplied functions can operate on the view:

```silica
DirectedGraph@has_edge(view, from_id, to_id)
DirectedGraph@out_degree(view, node_id)
DirectedGraph@reachable(view, from_id, to_id)
```

The view makes first-argument dispatch straightforward: the supplied algorithm dispatches on the view record, not on the original storage type. The view record carries both the original graph and the lambdas needed to inspect it.

### Weighted Graph View

```silica
view: {
    graph: Graph,
    neighbors_of: fn(Graph, NodeId) -> List[EdgePayload, Space],
    edge_target: fn(EdgePayload) -> NodeId,
    edge_weight: fn(EdgePayload) -> Weight,
    node_equals: fn(NodeId, NodeId) -> boolean
} <- WeightedGraph@view(graph, neighbors_of, edge_target, edge_weight, node_equals);
```

Then supplied algorithms can use:

```silica
WeightedGraph@weight_at(view, from_id, to_id)
```

### Adapter Benefits

The view/adapter approach reduces user code because the user does not need to write:

```silica
impl fn has_edge(...)
impl fn out_degree(...)
impl fn reachable(...)
impl fn max_out_degree(...)
```

Instead, the user writes or passes:

```silica
neighbors_of
edge_target
node_equals
```

The standard library supplies the algorithms.

The lambdas also serve as type witnesses. For example:

```silica
edge_target: fn({ to: string, weight: int64 }) -> string
```

defines:

- `EdgePayload = { to: string, weight: int64 }`
- `NodeId = string`

This reduces reliance on return-context-only inference and avoids assuming node ids are `int64`.

### Builder Adapter

Construction can use a separate builder adapter:

```silica
builder: {
    empty_storage: fn(NodeId, EdgePayload, int64) -> Graph,
    insert_edge: fn(Graph, NodeId, EdgePayload) -> Graph,
    make_edge: fn(NodeId, Payload) -> EdgePayload
} <- DirectedGraphBuilder@builder(empty_storage, insert_edge, make_edge);
```

Then supplied construction helpers can call:

```silica
DirectedGraphBuilder@empty_like(builder, sample_node_id, sample_edge, node_count)
DirectedGraphBuilder@with_first_edge(builder, node_count, from_id, to_id, payload)
```

This keeps behavior views and construction builders separate:

- A **view** reads an existing structure.
- A **builder** creates or updates a structure.

### Passing Lambdas Directly Versus Bundling A View

Both forms are possible:

```silica
DirectedGraphAlgo@has_edge(g, from_id, to_id, neighbors_of, edge_target, node_equals)
```

or:

```silica
view <- DirectedGraph@view(g, neighbors_of, edge_target, node_equals);
DirectedGraph@has_edge(view, from_id, to_id)
```

The adapter form is preferred because:

- Type errors can point to adapter construction.
- Multiple algorithms reuse the same typed view.
- Algorithm calls stay short.
- The view can carry cached metadata later if useful.
- The original graph does not need to implement the full trait directly.

### Relationship To Trait Implementations

Generated structures can still implement traits directly. A generated adjacency graph may not need a view because the generated module can supply `neighbors`, `edge_target`, and other required functions automatically.

User structures have two choices:

1. Implement the trait directly.
2. Build a view/adapter by supplying lambdas.

The adapter path is the lower-code path. Direct trait implementation is better when the user wants their type to behave as a graph everywhere without constructing a view value.

## Constructor Direction

Replace or supplement ambiguous `empty(...)` constructors with type-revealing builders:

```silica
empty_like(sample, ...)
with_first_edge(...)
with_first_entry(...)
```

Examples:

```silica
graph_adj_directed@empty_like("A", { to: "B", weight: 0 }, 3)
graph_adj_directed@with_first_edge(3, "A", { to: "B", weight: 42 })

btree_set_nodeid@empty_like("sample-key")

btree_nodeid@empty_like("sample-key", 0)

heap_binary_min@empty_like(0)
```

This makes the type information available through ordinary arguments rather than requiring the compiler to infer everything from the target binding.

## Impact On Current Phase 0.4 Work

This direction changes the meaning of Phase 0.4.

Instead of relying primarily on zero-receiver context resolution, Phase 0.4 should support:

1. First-argument trait dispatch for existing values.
2. Placeholder matching for trait required/provided functions.
3. Builder resolution from sample arguments, builder adapters, and expected binding type.
4. Return-context resolution only as a fallback for constructors that cannot be made type-revealing.
5. Link-name mangling based on resolved concrete trait implementation types.

The existing `Collectable` placeholder work still matters, but it becomes less central for constructors if builder arguments expose the key/node/payload types.

## Benefits

- Algorithms can be written against traits rather than generated concrete modules.
- Custom user structures can participate in standard algorithms.
- Node ids, keys, values, priorities, and edge payloads are no longer accidentally fixed to `int64`.
- Constructor calls become more explicit and easier for the compiler to resolve.
- The standard library can provide common algorithms once per trait instead of once per representation.
- Generated modules can become representation implementations rather than the only public API.

## Costs And Risks

- The language may need clearer support for generic trait placeholders or associated type-like behavior.
- Builder traits need a resolution rule distinct from ordinary first-argument trait dispatch for creation operations.
- Some existing generated modules and trials would need API migration.
- Provided trait algorithms over lists may need efficient traversal abstractions to avoid unnecessary materialization.
- Error messages must explain whether failure came from behavior dispatch, builder resolution, comparator absence, or payload mismatch.

## Open Questions

1. How should Silica spell trait-level type placeholders such as `NodeId`, `EdgePayload`, `Key`, `Value`, `Priority`, and `Space`?
2. Should builder traits return the concrete implementor type directly, or a trait placeholder resolved by binding context?
3. Should `empty_like` require sample values for all type variables, or only those not otherwise inferable?
4. Should unweighted graph edge payload be the destination id, or should it use an explicit `{ to: NodeId }` record for uniformity with weighted graphs?
5. Should algorithms prefer `neighbors(g, node)` returning a list, or `fold_neighbors(g, node, init, fn)` to support CSR/dense forms without list allocation?
6. Should key comparison be a trait constraint (`Comparable`) or an explicit comparator lambda for ordered structures?
7. Should generated modules continue exposing direct module functions alongside trait implementations during migration?

## Suggested Next Experiment

Create a small trait trial for directed graph behavior:

1. Define a `DirectedGraph` trait with required `node_count`, `edge_count`, `neighbors`, and `edge_target`.
2. Add provided `out_degree` and `has_edge`.
3. Implement the trait for one generated adjacency graph shape.
4. Add a custom programmer-defined graph record implementing the same required functions.
5. Verify both can call `DirectedGraph@has_edge(g, from, to)`.
6. Add `DirectedGraphBuilder@empty_like` only after behavior dispatch is working.

This keeps constructor complexity separate from behavior dispatch while proving whether traits actually simplify the type issues.
