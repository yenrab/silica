# B-tree Set Design (silica-compiler)

## 1. Purpose and scope

This document specifies generated B-tree set representations for Silica code. It specializes [balanced_tree_and_heap_design.md](balanced_tree_and_heap_design.md) and reuses the storage vocabulary from [graph_representation_design.md](graph_representation_design.md): node ids, inline structural records, `List[T, S]`, region handles, and region buffers. **Set keys use types that implement the language `Collectable` trait** (§4.0). **Immutability and uniform inline types** follow [graph_representation_design.md](graph_representation_design.md) §2.7–§2.8.

The names `NodeIDBTreeSet` and `CsrBTreeSet` are **design/generator names** using **`List`-aligned bracket syntax** (§4.1; [graph_representation_design.md](graph_representation_design.md) §2.11), not Silica type aliases. Bracket forms identify **registry keys** and **function-call instantiation**. **Emitted module filenames** use **representation only** (§8) — for example `btree_set_nodeid.silica`, not `btree_set_nodeid_int64_mem_normal.silica`. Key type and **`mem(Space)`** appear on **function calls**, not in module names. Generated Silica must use inline structural record types everywhere a type is required (or expand a compiler-known shorthand to that inline type).

Primary first target:

```text
NodeIDBTreeSet[int64, mem(normal)]
CsrBTreeSet[int64, mem(normal)]
```

Later target variants can use other concrete key types, but this document assumes `int64` keys so the generator can emit monomorphic comparison code.

## 2. Relationship to graph and B-tree designs

This document avoids repeating the graph-level model. Use [graph_representation_design.md](graph_representation_design.md) for:

- The rule that design names are not custom Silica types.
- The memory-space vocabulary.
- The requirement that generated structures with buffers carry their owning region.
- The naming convention idea of deterministic generated function prefixes.
- The preference for node ids and packed buffers instead of recursive custom node objects.
- **`Collectable`** payload rules, buffer encoding, immutability, and uniform inline types (§2.4–§2.8).

Use [balanced_tree_and_heap_design.md](balanced_tree_and_heap_design.md) for:

- B-tree terminology: `order`, `max_keys`, `max_children`, and minimum occupancy.
- The general `NodeIDBTree` vs. `CsrBTree` tradeoff.
- Search, split, validation, and CSR finalization concepts.

This document narrows those ideas to **sets**:

```text
keys only
no per-key values
membership queries
insert/delete policies for unique keys
optional conversion from build form to packed query form
```

## 3. When to use each set

### 3.1 Use `NodeIDBTreeSet` when

Use `NodeIDBTreeSet` for a clear, flexible set that is still being built or changed.

Good uses:

- Small and medium sets.
- Generated code that humans will inspect.
- Bootstrap compiler data structures.
- Data structures whose construction logic is still being validated.
- Sets that need insertions during a pass.
- Debug-friendly intermediate sets before packed finalization.

Tradeoffs:

```text
node lookup is list-backed
updates rebuild lists
memory layout is less compact
search is slower than CSR form
```

### 3.2 Use `CsrBTreeSet` when

Use `CsrBTreeSet` for a compact, query-oriented set after construction is complete.

Good uses:

- Large sets.
- Static sets generated from known sorted keys.
- Symbol/id membership tables.
- Keyword or atom-id membership tables.
- Compiler analysis sets that are built once and queried many times.
- Cache-friendly search-heavy data.

Tradeoffs:

```text
requires concrete buffer capacities
pack layout is less readable than NodeIDBTreeSet
functional insert allocates a new region per call
bootstrap capacity is fixed at compile time
```

### 3.3 Recommended pipeline

`CsrBTreeSet` is a **first-class, directly insertable** data structure. Both representations support the same functional immutable insert API; choose based on query and layout characteristics.

For direct construction via sequential insert (either representation):

```text
empty -> insert -> insert -> ... -> contains/range queries
```

For static sorted input (CSR only, no insert overhead):

```text
CsrBTreeSet directly via from_static_sorted
```

For sets that will be converted to CSR after batch construction (future Phase 6):

```text
NodeIDBTreeSet -> validate -> to_csr -> CsrBTreeSet -> contains/range queries
```

For tiny sets:

```text
List[int64, S] may be simpler than either B-tree set
```

For dense integer sets over a bounded range:

```text
dense bitset buffer may be better than a B-tree set
```

## 4. Shared B-tree set model

### 4.0 `Collectable` set keys

For B-tree **sets**, any API parameter that carries **stored key data** — insert, `from_list` elements, `contains`, range endpoints, `delete` — uses a concrete **`Key: Collectable`** type in emitted signatures (design-level placeholder **`Key`** in abstract descriptions).

The **first** monomorphic generators use **`int64`** keys. Storage layouts (`List[int64, S]`, `buf(R, S, int64, ...)`) remain concrete inline types. Buffer encoding follows [graph_representation_design.md](graph_representation_design.md) §2.6.

Structural **`int64` node ids** inside tree nodes are not set keys and are not typed as **`Collectable`**.

There is no separate storage marker trait beyond language **`Collectable`**.

### 4.1 Design-name bracket syntax (`List`-aligned)

B-tree set families use the same bracket convention as lists and graphs ([list_implementation_design.md](list_implementation_design.md) §3.5; [graph_representation_design.md](graph_representation_design.md) §2.11):

- **Sets:** `[Key, mem(Space)]` — for example `NodeIDBTreeSet[int64, mem(normal)]`, `CsrBTreeSet[int64, mem(atomic)]`.
- Bracket forms are **design/generator names**; emitted Silica still uses **full inline structural record types** at every type position unless the compiler expands the shorthand.
- B-tree **`order`** and CSR **buffer capacities** are separate generator inputs, not bracket parameters.

**Emitted module names** (one per representation family):

```text
btree_set_nodeid
btree_set_csr
```

Do **not** encode key type or memory space in the module name. Callers write `btree_set_nodeid@empty[int64, mem(normal)]()` — aligned with **`List`** (`empty[int64, normal]()`).

**Generated operation calls** use **`module@operation`** with explicit bracket instantiation — for example `btree_set_nodeid@empty[int64, mem(normal)]()`, `btree_set_csr@contains[int64, mem(normal)](tree, key)`.

See also [balanced_tree_and_heap_design.md](balanced_tree_and_heap_design.md) §2.6 for map and general B-tree bracket arity (`[Key, Value, mem(Space)]`).

### 4.2 Key rules

First generated implementation:

```text
key type: int64
comparison: numeric ascending
duplicates: ignored or reported as already present
```

Recommended duplicate policy:

```text
set semantics: inserting an existing key returns inserted = false
```

A set does not replace values because it has none.

### 4.3 Order and occupancy

The generator must choose a concrete `order`.

Recommended first values:

```text
order = 8 for easier tests
order = 16 for better cache behavior once stable
```

Derived values:

```text
max_keys = order - 1
max_children = order
min_keys_non_root = ceil(order / 2) - 1
min_children_non_root = ceil(order / 2)
```

### 4.4 Invariants

Every generated set must preserve:

```text
keys are unique
keys in each node are sorted ascending
internal node child_count == key_count + 1
leaf node child_count == 0
all child ids are valid node ids
all non-root nodes have at least min_keys_non_root keys after finalized construction
all nodes have at most max_keys keys
all leaves have the same depth
left child keys < separator key < right child keys
```

Empty set convention:

```text
root_id = -1
node_count = 0
key_count_total = 0, for CSR
```

## 5. `NodeIDBTreeSet`

### 5.1 Summary

`NodeIDBTreeSet` stores B-tree nodes as inline records in a list. Each node stores its sorted keys and child node ids as lists. It is a set-only specialization of `NodeIDBTree`.

Design name:

```text
NodeIDBTreeSet[int64, mem(S)]
```

Concrete generator family examples:

```text
NodeIDBTreeSet[int64, mem(normal)]
NodeIDBTreeSet[int64, mem(atomic)]
```

**Naming in §5–§7:** subsection headings use `{module}_{operation}` labels for readability in this design spec. Emitted Silica uses **short export names** and **`module@operation[brackets]`** call syntax (§8).

### 5.2 Inline Silica shape

Generic shape using design placeholder `S`:

```silica
{
    root_id: int64,
    node_count: int64,
    order: int64,
    nodes: List[
        {
            id: int64,
            key_count: int64,
            is_leaf: bool,
            keys: List[int64, S],
            children: List[int64, S]
        },
        S
    ]
}
```

Concrete `normal` shape:

```silica
{
    root_id: int64,
    node_count: int64,
    order: int64,
    nodes: List[
        {
            id: int64,
            key_count: int64,
            is_leaf: bool,
            keys: List[int64, normal],
            children: List[int64, normal]
        },
        normal
    ]
}
```

The generator must repeat the concrete shape in every generated function signature.

### 5.3 Empty construction

Generated function name:

```text
btree_set_nodeid_empty[Key, mem(Space)]
```

Example:

```text
btree_set_nodeid_empty[int64, mem(normal)]
```

Behavior:

```text
return root_id = -1, node_count = 0, order = ORDER, nodes = empty list
```

Because lists allocate storage, construction must occur under `sequence proc[mem(S)]` when the emitted body creates list values.

Pseudo-shape:

```silica
fn empty[int64, mem(normal)]() -> {
    root_id: int64,
    node_count: int64,
    order: int64,
    nodes: List[{ id: int64, key_count: int64, is_leaf: bool, keys: List[int64, normal], children: List[int64, normal] }, normal]
} {
    sequence proc[mem(normal)]
        nodes: List[{ id: int64, key_count: int64, is_leaf: bool, keys: List[int64, normal], children: List[int64, normal] }, normal] <- empty[{ id: int64, key_count: int64, is_leaf: bool, keys: List[int64, normal], children: List[int64, normal] }, normal]();
        tree: {
            root_id: int64,
            node_count: int64,
            order: int64,
            nodes: List[{ id: int64, key_count: int64, is_leaf: bool, keys: List[int64, normal], children: List[int64, normal] }, normal]
        } <- { root_id: -1, node_count: 0, order: ORDER, nodes: nodes };
    produces
        pure tree
    end
}
```

`ORDER` is a generator constant emitted as an integer literal.

### 5.4 Membership query

Abstract API (design level):

```text
btree_set_nodeid_contains[Key, mem(Space)](tree: <NodeIDBTreeSet type>, key: Collectable) -> bool
btree_set_csr_contains[Key, mem(Space)](tree: <CsrBTreeSet type>, key: Collectable) -> bool
```

Generated function:

```text
btree_set_nodeid_contains[Key, mem(Space)]
```

Return type:

```text
bool
```

Algorithm:

```text
contains(tree, key: Collectable):
    if tree.root_id == -1:
        false
    else:
        contains_node(tree, tree.root_id, key)
```

Node search:

```text
contains_node(tree, node_id, key: Collectable):
    node_result = find_node(tree.nodes, node_id)
    if node_result.found == false:
        false
    node = node_result.node
    pos = search_key_list(node.keys, key, 0)
    if pos.found:
        true
    else if node.is_leaf:
        false
    else:
        child_id = nth_int64(node.children, pos.index)
        contains_node(tree, child_id, key)
```

`search_key_list` result shape:

```silica
{ found: bool, index: int64 }
```

The returned `index` is the child slot where the key should be found or inserted if not already present.

### 5.5 Insert

Abstract API (design level):

```text
btree_set_nodeid_insert[Key, mem(Space)](tree, key: Collectable) -> { tree: <NodeIDBTreeSet type>, inserted: bool }
```

Generated function:

```text
btree_set_nodeid_insert[Key, mem(Space)]
```

Return shape:

```silica
{
    tree: <full inline NodeIDBTreeSet type>,
    inserted: bool
}
```

Set duplicate behavior:

```text
if key already exists:
    return inserted = false and tree unchanged
```

Recommended algorithm: top-down B-tree insertion.

High-level steps (`key` is **`Collectable`**):

```text
insert(tree, key: Collectable):
    if tree.root_id == -1:
        create root leaf with key
        return inserted = true

    if contains(tree, key):
        return inserted = false

    root = find_node(tree.nodes, tree.root_id)
    if root.key_count == order - 1:
        new_root_id = tree.node_count
        create empty internal root with old root as first child
        split_child(new_root_id, 0)
        insert_nonfull(new_root_id, key)
    else:
        insert_nonfull(root_id, key)
```

`insert_nonfull(tree, node_id, key: Collectable)`:

```text
node = find_node(node_id)
if node.is_leaf:
    insert key into node.keys at sorted position
    increment key_count
    replace node in tree.nodes
else:
    pos = search_key_list(node.keys, key, 0)
    child_id = nth_int64(node.children, pos.index)
    child = find_node(child_id)
    if child.key_count == order - 1:
        tree = split_child(tree, node_id, pos.index)
        node = find_node(node_id)
        if key > key_at(node.keys, pos.index):
            pos.index = pos.index + 1
    child_id2 = nth_int64(node.children, pos.index)
    insert_nonfull(tree, child_id2, key)
```

`split_child(tree, parent_id, child_index)`:

```text
parent = find_node(parent_id)
full_child_id = nth_int64(parent.children, child_index)
full_child = find_node(full_child_id)
mid_index = max_keys / 2
promoted_key = key_at(full_child.keys, mid_index)

left_keys = keys before mid_index
right_keys = keys after mid_index

if full_child is leaf:
    left_children = empty
    right_children = empty
else:
    left_children = first mid_index + 1 children
    right_children = remaining children

update full_child as left node
new_right_id = tree.node_count
create right node with right_keys and right_children
insert promoted_key into parent.keys at child_index
insert new_right_id into parent.children at child_index + 1
increment tree.node_count
replace full_child and parent
prepend new right node
```

List helper operations the generator must provide or inline:

```text
nth_int64(list, index)
insert_int64_at(list, index, value)
take_int64(list, count)
drop_int64(list, count)
length_int64(list)
replace_node_by_id(nodes, node)
find_node_by_id(nodes, id)
```

All list helper functions must use the same memory space `S`.

### 5.6 Delete

Deletion is optional for the first generated set.

Abstract API when generated:

```text
btree_set_nodeid_delete[Key, mem(Space)](tree, key: Collectable) -> { tree: <NodeIDBTreeSet type>, removed: bool }
```

If generated, use B-tree top-down deletion:

```text
delete(tree, key: Collectable) -> { tree, removed: bool }
```

Rules:

1. Ensure the child has enough keys before descending.
2. Borrow from left sibling if possible.
3. Else borrow from right sibling if possible.
4. Else merge with a sibling.
5. Remove directly from a leaf.
6. For internal deletion, replace key with predecessor or successor, then delete that replacement key from the child.
7. If root becomes empty and has one child, make the child the new root.

Recommended first version:

```text
do not generate delete until insert, split, validation, and CsrBTreeSet finalization are stable
```

### 5.7 Validation

Generated function:

```text
btree_set_nodeid_validate[Key, mem(Space)]
```

Return shape:

```silica
{ ok: bool, error_code: int64 }
```

Recommended error codes:

```text
0 = ok
1 = invalid root
2 = duplicate node id
3 = invalid child id
4 = key_count mismatch
5 = keys not sorted
6 = duplicate key
7 = child_count mismatch
8 = occupancy violation
9 = leaves at different depths
10 = parent range violation
```

Validation checks:

```text
empty convention is correct
node_count equals number of node records
node ids are unique
every node id is in [0, node_count)
root id exists when node_count > 0
key_count equals length(keys)
key_count <= order - 1
non-root key_count >= min_keys_non_root, after finalized construction
keys sorted ascending
no duplicate keys within node
leaf nodes have no children
internal nodes have key_count + 1 children
children point to existing node ids
no non-root node has more than one parent
all leaves have equal depth
all child key ranges obey parent separator keys
```

### 5.8 Cost model

Because node lookup scans the node list:

```text
contains: O(height * (node_count + order))
insert: O(height * (node_count + order + list rebuild cost))
validate: O(node_count^2) acceptable for debug/build stages
```

This cost model is the main reason to finalize to `CsrBTreeSet` for large search-heavy sets.

## 6. `CsrBTreeSet`

### 6.1 Summary

`CsrBTreeSet` stores B-tree metadata and keys in region buffers. Each node id indexes metadata buffers. Keys and children live in packed buffers, with per-node start/count metadata.

Design name:

```text
CsrBTreeSet[int64, mem(S)]
```

Generator constants (not bracket parameters): region id **`R`**, **`NODE_CAP`**, **`KEY_CAP`**, **`CHILD_CAP`**.
```

This is the recommended query representation for generated sets.

### 6.2 Inline Silica shape

Generic shape using design placeholders:

```silica
{
    region: region(R, S),
    root_id: int64,
    node_count: int64,
    key_count_total: int64,
    order: int64,
    node_key_start: buf(R, S, int64, NODE_CAP),
    node_key_count: buf(R, S, int64, NODE_CAP),
    node_child_start: buf(R, S, int64, NODE_CAP),
    node_child_count: buf(R, S, int64, NODE_CAP),
    node_is_leaf: buf(R, S, int64, NODE_CAP),
    keys: buf(R, S, int64, KEY_CAP),
    children: buf(R, S, int64, CHILD_CAP)
}
```

Concrete `normal` shape:

```silica
{
    region: region(R, normal),
    root_id: int64,
    node_count: int64,
    key_count_total: int64,
    order: int64,
    node_key_start: buf(R, normal, int64, NODE_CAP),
    node_key_count: buf(R, normal, int64, NODE_CAP),
    node_child_start: buf(R, normal, int64, NODE_CAP),
    node_child_count: buf(R, normal, int64, NODE_CAP),
    node_is_leaf: buf(R, normal, int64, NODE_CAP),
    keys: buf(R, normal, int64, KEY_CAP),
    children: buf(R, normal, int64, CHILD_CAP)
}
```

The `region` field is mandatory. Returning buffers without the owning region is invalid.

### 6.3 Buffer semantics

For node id `n`:

```text
key_start = node_key_start[n]
key_count = node_key_count[n]
child_start = node_child_start[n]
child_count = node_child_count[n]
is_leaf = node_is_leaf[n]
```

Node keys:

```text
keys[key_start ... key_start + key_count - 1]
```

Node children:

```text
children[child_start ... child_start + child_count - 1]
```

Leaf convention:

```text
is_leaf = 1
child_count = 0
child_start may be 0
```

Internal convention:

```text
is_leaf = 0
child_count = key_count + 1
```

### 6.4 Static construction from sorted keys

Generated function:

```text
btree_set_csr_from_static_sorted[Key, mem(Space)]
```

Use this when the generator already knows all set keys.

Generator-side algorithm:

1. Deduplicate sorted keys.
2. Choose `order`.
3. Build leaf nodes with up to `order - 1` keys each.
4. Build parent layers until one root remains.
5. Assign node ids breadth-first:

```text
root = 0
then next level left-to-right
then next level left-to-right
```

6. Compute `NODE_CAP`, `KEY_CAP`, and `CHILD_CAP`.
7. Emit concrete buffer allocations.
8. Emit `write_buf` calls for node metadata.
9. Emit `write_buf` calls for packed keys and children.
10. Return the full CSR set record with the owning region.

Generated Silica body shape:

```silica
fn from_static_sorted[int64, mem(normal)]() -> {
    region: region(R, normal),
    root_id: int64,
    node_count: int64,
    key_count_total: int64,
    order: int64,
    node_key_start: buf(R, normal, int64, NODE_CAP),
    node_key_count: buf(R, normal, int64, NODE_CAP),
    node_child_start: buf(R, normal, int64, NODE_CAP),
    node_child_count: buf(R, normal, int64, NODE_CAP),
    node_is_leaf: buf(R, normal, int64, NODE_CAP),
    keys: buf(R, normal, int64, KEY_CAP),
    children: buf(R, normal, int64, CHILD_CAP)
} {
    sequence proc[mem(normal)]
        R: lifetime <- fresh_lifetime();
        r: region(R, normal) <- alloc_region(normal);
        node_key_start: buf(R, normal, int64, NODE_CAP) <- alloc_buf(r, NODE_CAP);
        node_key_count: buf(R, normal, int64, NODE_CAP) <- alloc_buf(r, NODE_CAP);
        node_child_start: buf(R, normal, int64, NODE_CAP) <- alloc_buf(r, NODE_CAP);
        node_child_count: buf(R, normal, int64, NODE_CAP) <- alloc_buf(r, NODE_CAP);
        node_is_leaf: buf(R, normal, int64, NODE_CAP) <- alloc_buf(r, NODE_CAP);
        keys: buf(R, normal, int64, KEY_CAP) <- alloc_buf(r, KEY_CAP);
        children: buf(R, normal, int64, CHILD_CAP) <- alloc_buf(r, CHILD_CAP);
        _: atom <- write_buf(node_key_start, 0, NODE0_KEY_START);
        ...
    produces
        pure {
            region: r,
            root_id: 0,
            node_count: NODE_COUNT,
            key_count_total: KEY_COUNT_TOTAL,
            order: ORDER,
            node_key_start: node_key_start,
            node_key_count: node_key_count,
            node_child_start: node_child_start,
            node_child_count: node_child_count,
            node_is_leaf: node_is_leaf,
            keys: keys,
            children: children
        }
    end
}
```

All uppercase identifiers are generator constants that must be emitted as concrete literals or valid generated bindings.

### 6.5 Finalization from `NodeIDBTreeSet`

Generated function:

```text
btree_set_nodeid_to_csr[Key, mem(Space)]
```

Input:

```text
NodeIDBTreeSet[int64, mem(S)]
```

Output:

```text
CsrBTreeSet[int64, mem(S)]
```

Generator constants (not bracket parameters): region id **`R`**, **`NODE_CAP`**, **`KEY_CAP`**, **`CHILD_CAP`**.
```

Finalization algorithm:

1. Validate the `NodeIDBTreeSet`.
2. Choose an output node order. Recommended first choice: breadth-first from root.
3. Build a mapping from old node id to new dense node id.
4. Count:

```text
node_count
key_count_total
child_count_total
```

5. Allocate all CSR buffers.
6. Traverse nodes in output order.
7. For each node:

```text
write node_key_start[new_id]
write node_key_count[new_id]
write node_child_start[new_id]
write node_child_count[new_id]
write node_is_leaf[new_id]
copy node keys into keys buffer
copy remapped child ids into children buffer
```

8. Return the CSR set with `root_id = 0`.

If the compiler lacks a convenient temporary map, the generator can make finalization a generator-side operation for static data, or use a list of `{ old_id: int64, new_id: int64 }` pairs for small sets.

### 6.6 Membership query

Generated function:

```text
btree_set_csr_contains[Key, mem(Space)]
```

Return:

```text
bool
```

Algorithm:

```text
contains(tree, key: Collectable):
    if tree.root_id == -1:
        false
    else:
        contains_node(tree, tree.root_id, key)
```

CSR node search:

```text
contains_node(tree, node_id, key: Collectable):
    key_start = read_buf(tree.node_key_start, node_id)
    key_count = read_buf(tree.node_key_count, node_id)
    pos = search_key_range(tree.keys, key_start, key_count, key)
    if pos.found:
        true
    else:
        is_leaf = read_buf(tree.node_is_leaf, node_id)
        if is_leaf == 1:
            false
        else:
            child_start = read_buf(tree.node_child_start, node_id)
            child_id = read_buf(tree.children, child_start + pos.index)
            contains_node(tree, child_id, key)
```

`search_key_range` should return:

```silica
{ found: bool, index: int64 }
```

Recommended search strategy:

```text
if order <= 16:
    linear scan inside node
else:
    binary search inside node
```

### 6.7 Insert and delete policy

`CsrBTreeSet` uses a **functional-programming design**: `insert` returns a new `CsrBTreeSet` value without modifying the caller's existing value. The old value remains valid and is not copied — its region is a separate allocation that the caller can continue to use. This follows the same immutability and structural-sharing principle as `NodeIDBTreeSet` and Silica's `List`.

Generated function:

```text
btree_set_csr_insert[Key, mem(Space)](tree, key) -> { tree, inserted }
```

Behavior:

```text
key not present and capacity available  → new tree with key inserted; inserted = 1
key already present (set semantics)     → original tree returned unchanged; inserted = 0
bootstrap KEY_CAP exhausted             → original tree returned unchanged; inserted = 0
```

The bootstrap implementation (NODE_CAP=1, KEY_CAP=7) supports single-leaf sets of up to seven keys. Each insert allocates a fresh region containing updated metadata and key buffers; keys not carried forward are not referenced (bootstrap nodes do not structurally share buffers). The full multi-node implementation with path-level structural sharing (only modified-path nodes are reallocated; unmodified subtrees are referenced from the new root) is deferred to Phase 6.

Delete is not generated in the first pass. When delete is needed, use `NodeIDBTreeSet` or implement the CSR delete by rebuilding the affected region.

### 6.8 Validation

Generated function:

```text
btree_set_csr_validate[Key, mem(Space)]
```

Return shape:

```silica
{ ok: bool, error_code: int64 }
```

Validation checks:

```text
root convention is correct
node_count <= NODE_CAP
key_count_total <= KEY_CAP
all node metadata indices are inside capacity
node_is_leaf values are 0 or 1
key ranges do not exceed KEY_CAP
child ranges do not exceed CHILD_CAP
key ranges for nodes do not overlap unexpectedly if packed exactly
keys sorted within every node range
no duplicate keys
leaf nodes have child_count == 0
internal nodes have child_count == key_count + 1
children are valid node ids
no non-root node has more than one parent
all leaves have equal depth
parent separator ranges are respected
```

Recommended error codes should match `NodeIDBTreeSet` where possible, with CSR-specific additions:

```text
20 = key range out of bounds
21 = child range out of bounds
22 = invalid node metadata
```

### 6.9 Cost model

For `order <= 16` with linear node search:

```text
contains: O(order * height)
validation: O(node_count * order + child_count_total)
storage: O(node_count + key_count_total + child_count_total)
```

With binary node search:

```text
contains: O(log(order) * height)
```

In practice, linear scan inside a small B-tree node is often better because the keys are contiguous and the branch structure is simple.

## 7. Common set operations

All operations that take a **lookup or mutation key** use **`key: Collectable`** (§4.0) in abstract signatures.

### 7.1 `contains`

Abstract signatures:

```text
btree_set_nodeid_contains[Key, mem(Space)](tree, key: Collectable) -> bool
btree_set_csr_contains[Key, mem(Space)](tree, key: Collectable) -> bool
```

Both representations must generate:

```text
btree_set_nodeid_contains[Key, mem(Space)]
btree_set_csr_contains[Key, mem(Space)]
```

Return:

```text
bool
```

### 7.2 `insert`

Both representations generate `insert`. Both are **functional and immutable**: the returned tree is a new value; the caller's original tree is unchanged.

Abstract signatures:

```text
btree_set_nodeid_insert[Key, mem(Space)](tree, key: Collectable) -> { tree: <NodeIDBTreeSet type>, inserted: bool }
btree_set_csr_insert[Key, mem(Space)](tree, key: Collectable) -> { tree: <CsrBTreeSet type>, inserted: bool }
```

Generated functions:

```text
btree_set_nodeid_insert[Key, mem(Space)]
btree_set_csr_insert[Key, mem(Space)]
```

Return shape (both representations):

```silica
{
    tree: <full inline tree type>,
    inserted: bool
}
```

`inserted = true` (1) means the key was newly added; `inserted = false` (0) means the key was already present. Bootstrap `CsrBTreeSet` also returns `inserted = 0` when `KEY_CAP` is exhausted.

### 7.3 `delete`

Optional later (operands **`Collectable`**):

```text
btree_set_nodeid_delete[Key, mem(Space)](tree, key: Collectable) -> { tree: <NodeIDBTreeSet type>, removed: bool }
```

Do not generate CSR deletion in the first pass.

### 7.4 `from_list`

Generated helper:

```text
btree_set_nodeid_from_list[Key, mem(Space)]
```

Algorithm:

```text
start with empty NodeIDBTreeSet
for each key (`Collectable`) in input List[int64, S]:
    insert key
return final tree
```

Duplicate keys naturally return `inserted = false` and do not change set contents.

### 7.5 `to_csr`

Generated helper:

```text
btree_set_nodeid_to_csr[Key, mem(Space)]
```

Use after dynamic construction when query performance matters.

### 7.6 Range scan

Optional later:

```text
btree_set_nodeid_range[Key, mem(Space)]
btree_set_csr_range[Key, mem(Space)]
```

Abstract signatures:

```text
btree_set_nodeid_range[Key, mem(Space)](tree, low: Collectable, high: Collectable) -> List[int64, S]
btree_set_csr_range[Key, mem(Space)](tree, low: Collectable, high: Collectable) -> List[int64, S]
```

Result should be a `List[int64, S]` of keys in ascending order.

Range query (`low` and `high` are **`Collectable`** bounds):

```text
range(tree, low: Collectable, high: Collectable):
    include keys k where low <= k and k <= high
```

This is useful for compiler interval-like structures, but it requires careful in-order traversal generation.

## 8. Generated naming rules

**Design/registry names** use bracket syntax (§4.1). **Module names** and **operation names** follow **`List`** conventions: the module identifies the representation family; key type and memory space are **bracket parameters on function calls**.

### Module names

One Silica module per set representation family:

```text
btree_set_nodeid
btree_set_csr
```

Examples: `btree_set_nodeid.silica`, `btree_set_csr.silica`.

**Do not** suffix module names with key type or memory space (for example `btree_set_nodeid_int64_mem_normal` is **incorrect**).

Import and call (short operation name after `@`; do not repeat the module name):

```text
use btree_set_nodeid;
btree_set_nodeid@empty[int64, mem(normal)]()
btree_set_nodeid@contains[int64, mem(normal)](tree, key)
```

**Single-file `use` rule:** do not `use` two generated modules in one source file when both export the same `operation[brackets]` (for example both export `empty[int64, mem(normal)]` — E4011). Cross-representation trials should call through one module (for example `btree_set_nodeid@validate_csr`, `btree_set_nodeid@contains_csr` re-export CSR helpers) or split into separate compilation units.

### Operation names

**Exported function names** are short operation verbs — for example `empty`, `contains`, `insert`, `validate`, `to_csr`, `from_static_sorted`. They **do not** repeat the module name.

**Module-qualified call syntax:** `btree_set_csr@contains[int64, mem(normal)](tree, key)`

Bracket parameters at call sites (final slot is always **`mem(Space)`**):

```text
[Key, mem(Space)]
```

Examples:

```text
btree_set_nodeid@empty[int64, mem(normal)]()
btree_set_nodeid@contains[int64, mem(normal)](tree, key)
btree_set_nodeid@insert[int64, mem(normal)](tree, key)
btree_set_nodeid@validate[int64, mem(normal)](tree)
btree_set_nodeid@to_csr[int64, mem(normal)](tree)

btree_set_csr@from_static_sorted[int64, mem(normal)](...)
btree_set_csr@contains[int64, mem(normal)](tree, key)
btree_set_csr@validate[int64, mem(normal)](tree)
```

**Linker symbols:** the compiler prefixes assembly labels with the module name (for example `btree_set_csr_contains_int64_mem_normal__`) so short export names can link together in one executable.

Internal helpers are module-local (not exported for `module@` calls). Examples: `find_node`, `split_child`, `insert_nonfull`, `search_key_range`, `contains_node`.

## 9. Generator requirements

### 9.1 Inputs

Set generator inputs:

```text
representation: nodeid_btree_set | csr_btree_set
key_type: int64       // monomorphic specialization; must implement Collectable (§4.0); bracket slot Key
memory_space: normal | normal_writethrough | normal_noncacheable | atomic
order: int64
generate_insert: bool
generate_delete: bool
generate_range: bool
generate_validate: bool
```

**Registry key (bracket form, §4.1):** `NodeIDBTreeSet[<key_type>, mem(<memory_space>)]`, `CsrBTreeSet[<key_type>, mem(<memory_space>)]` — for example `NodeIDBTreeSet[int64, mem(normal)]`.

Additional CSR inputs:

```text
node_capacity: int64
key_capacity: int64
child_capacity: int64
static_sorted_keys: optional list of int64
```

### 9.2 Type-string generation

The generator must produce canonical inline type strings.

For `NodeIDBTreeSet[int64, mem(normal)]`, define internally:

```text
NODE_TYPE =
{ id: int64, key_count: int64, is_leaf: bool, keys: List[int64, normal], children: List[int64, normal] }

TREE_TYPE =
{ root_id: int64, node_count: int64, order: int64, nodes: List[NODE_TYPE, normal] }
```

For emitted Silica, expand `NODE_TYPE` inside `TREE_TYPE`; do not emit aliases.

For `CsrBTreeSet[int64, mem(normal)]`, define internally:

```text
CSR_TREE_TYPE =
{
  region: region(R, normal),
  root_id: int64,
  node_count: int64,
  key_count_total: int64,
  order: int64,
  node_key_start: buf(R, normal, int64, NODE_CAP),
  node_key_count: buf(R, normal, int64, NODE_CAP),
  node_child_start: buf(R, normal, int64, NODE_CAP),
  node_child_count: buf(R, normal, int64, NODE_CAP),
  node_is_leaf: buf(R, normal, int64, NODE_CAP),
  keys: buf(R, normal, int64, KEY_CAP),
  children: buf(R, normal, int64, CHILD_CAP)
}
```

Then expand the full shape into signatures.

### 9.3 Required helper generation

For `NodeIDBTreeSet`, generate or inline:

```text
list_length_int64
list_nth_int64
list_insert_int64_at
list_take_int64
list_drop_int64
list_contains_int64
node_find_by_id
node_replace_by_id
node_insert_new
key_search_list
child_id_at
```

For `CsrBTreeSet`, generate or inline:

```text
key_range_linear_search
key_range_binary_search, optional
csr_node_key_start
csr_node_key_count
csr_node_child_start
csr_node_child_count
csr_node_is_leaf
csr_child_at
```

### 9.4 Result records

Membership:

```text
bool
```

Search position:

```silica
{ found: bool, index: int64 }
```

Validation:

```silica
{ ok: bool, error_code: int64 }
```

Insert:

```silica
{
    tree: <full inline NodeIDBTreeSet type>,
    inserted: bool
}
```

Delete:

```silica
{
    tree: <full inline NodeIDBTreeSet type>,
    removed: bool
}
```

### 9.5 Emission order

Recommended order in generated module:

1. Leaf list helpers.
2. Node list helpers.
3. Key search helpers.
4. Empty constructor.
5. Contains.
6. Validation.
7. Split helpers.
8. Insert helpers.
9. Delete helpers, if generated.
10. CSR conversion or CSR static constructor.
11. CSR contains.
12. CSR validation.
13. CSR insert helpers (csr_insert_pos_from, csr_out_key, build_leaf_csr, insert_nonempty_csr).
14. CSR insert.

This order keeps helper dependencies mostly top-down.

## 10. Implementation staging

Recommended project staging:

1. Generate `NodeIDBTreeSet[int64, mem(normal)]` empty and contains over hand-built trees.
2. Generate `NodeIDBTreeSet[int64, mem(normal)]` validation.
3. Generate `NodeIDBTreeSet[int64, mem(normal)]` insert with top-down split.
4. Generate `from_list` by repeated insert.
5. Generate `CsrBTreeSet[int64, mem(normal)]` from static sorted keys.
6. Generate `CsrBTreeSet[int64, mem(normal)]` contains and validation.
7. Generate `NodeIDBTreeSet -> CsrBTreeSet` finalization.
8. Add range scan.
9. Add deletion only after the previous stages are stable.

## 11. Open implementation questions

1. Dynamic buffer capacities: `CsrBTreeSet` is easiest when capacities are generator constants.
2. Temporary old-id to new-id mapping during finalization: a future map representation would simplify this.
3. Delete complexity: deletion should wait until generated split/merge validation is reliable.
4. Non-`int64` keys: each key type needs concrete comparison helpers.
5. Dense integer domains: for small bounded domains, a bitset set may be better than a B-tree set and should be documented separately if needed.

## 12. References

- **`Collectable`** — language trait for set keys (§4.0; silica-spec §8.2.4).
- **Immutability and type invariance** — [graph_representation_design.md](graph_representation_design.md) §2.7–§2.8; [balanced_tree_and_heap_design.md](balanced_tree_and_heap_design.md) §2.5.
- [graph_representation_design.md](graph_representation_design.md) - graph storage families and generator conventions reused here.
- [balanced_tree_and_heap_design.md](balanced_tree_and_heap_design.md) - B-tree and heap design this set document specializes.
- [silica-specification.md](silica-specification.md) - inline structural types, lists, regions, effects.
- [list_implementation_design.md](list_implementation_design.md) - `List[T, S]` storage and memory-space alignment.
- [region_memory_safety_todo.md](Phase1_TODOs/region_memory_safety_todo.md) - region lifetime implementation gaps relevant to returned buffers.
