# Balanced Tree and Heap Design (silica-compiler)

## 1. Purpose and scope

This document specifies balanced tree and heap representations that can be generated as Silica code without custom type declarations. It extends [graph_representation_design.md](graph_representation_design.md): trees and heaps are treated as constrained graph-like structures over integer node ids and region-backed storage. **Stored keys, map values, and heap elements use types that implement the language `Collectable` trait** ([silica-specification.md](silica-specification.md) §8.2.4), following [graph_representation_design.md](graph_representation_design.md) §2.4. **Immutability and uniform inline types** follow graph §2.7–§2.8.

The names in this document are **design/generator names**, not Silica type aliases. Generated Silica must still use inline structural record types in every parameter, return type, local binding, and pattern annotation.

Primary families:

1. `NodeIDBTree` - a clear, list-oriented B-tree representation using node ids and inline records.
2. `CsrBTree` - a packed-buffer B-tree representation using CSR-like ranges for keys, values, and children.
3. `RegionBinaryHeap` - an array-backed binary heap in a region buffer.
4. `RegionDaryHeap` - an array-backed d-ary heap for workloads that benefit from shallower trees.

First implementation target:

```text
NodeIDBTreeInt64
CsrBTreeInt64
RegionBinaryMinHeapInt64
```

Later variants can add weights/payloads, max-heaps, d-ary heaps, and non-`int64` key/value element types once the generator has stable templates.

## 2. Shared constraints

### 2.1 No custom surface types

Do not emit:

```silica
type BTree = ...
struct BTree { ... }
enum HeapKind { ... }
```

Instead emit function signatures that repeat the inline shape:

```silica
fn btree_nodeid_int64_normal_root_key_count(
    tree: {
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
) -> int64 {
    ...
}
```

The generator may use `NodeIDBTreeInt64Normal` internally as a template name, but emitted Silica type positions must contain the full record shape.

### 2.2 Memory spaces

Every representation that allocates storage must choose one concrete memory space:

```text
normal
normal_writethrough
normal_noncacheable
atomic
```

Use `normal` by default. Use `atomic` only for coordination cells or shared mutable metadata; ordinary tree and heap topology should not use atomic space unless the generated operations are specifically concurrent.

All allocation and mutation during construction must occur inside:

```silica
sequence proc[mem(S)]
    ...
produces
    pure result
end
```

The same `S` must appear consistently in `List[..., S]`, `region(R, S)`, and `buf(R, S, T, N)`.

### 2.3 Integer key assumption

The first generated tree and heap variants should use:

```text
key type: int64
value type: int64, when values are stored
```

This avoids needing generic type parameters or polymorphic comparison. Later generator variants can suffix the design name with the concrete type:

```text
NodeIDBTreeUint64
CsrBTreeStringKeyInt64Value
RegionBinaryMinHeapUint32
```

The first pass should not attempt generic B-trees.

### 2.4 `Collectable` keys, values, and heap elements

Generated balanced-tree and heap APIs use **`Collectable`** for **stored user data** in add/find/remove operations:

- Tree **keys** and map **values** (`insert`, `contains`, `get`, `delete`, search bounds).
- Heap **elements** (`push`, peek, pop).

**Plain types (not `Collectable`):** structural **`int64` node ids**, `order`, `node_count`, capacities, region handles, and internal indices that are not user keys.

**Monomorphic generators** emit the concrete inline key/value/element type (for example `int64`). Design-level signatures may use placeholders **`Key`**, **`Value`**, or **`Element`** where the concrete type is **`Collectable`**. Buffer and list storage use the same **`Collectable` buffer encoding** as graphs ([graph_representation_design.md](graph_representation_design.md) §2.6; [list_implementation_design.md](list_implementation_design.md) §4).

There is no separate storage marker trait beyond language **`Collectable`**.

### 2.5 Immutability and type invariance

Generated trees and heaps are **immutable values** ([graph_representation_design.md](graph_representation_design.md) §2.7):

- **`insert`**, **`delete`**, and **`push`** return a **new** tree or heap record with `produces pure … end`.
- Packed **CSR** tree/set forms are immutable after **`freeze`** or static construction.
- **Mutable builders** use **`_builder_`** or **`_mutable_`** name suffixes.

The **same inline tree or heap record type** must appear at every boundary for one value flow (uniform types, graph §2.7). Constructor return types pin the concrete **`Key`** / **`Value`** / **`Element`** spellings for later operations (graph §2.8).

## 3. B-tree terminology

A B-tree is a rooted, balanced multiway search tree.

For a B-tree of order `order`:

```text
max_keys = order - 1
max_children = order
min_keys_non_root = ceil(order / 2) - 1
min_children_non_root = ceil(order / 2)
```

This document uses `order` as "maximum children per internal node".

Recommended first implementation:

```text
order = 8 or order = 16
```

`order = 8` is simpler for testing. `order = 16` is more cache-friendly when node content is packed into buffers.

B-tree invariants:

```text
root_id is in [0, node_count), unless node_count == 0 and root_id == -1
each node has 0 <= key_count <= order - 1
keys inside a node are sorted ascending
leaf nodes have child_count == 0
internal nodes have child_count == key_count + 1
every child id is in [0, node_count)
every non-root node has at least min_keys_non_root keys after construction
all leaves have the same depth
for each internal node, child key ranges partition around parent keys
```

Duplicate-key policy must be selected by the generator:

| Policy | Meaning |
|--------|---------|
| `reject_duplicates` | Insert returns unchanged tree plus `inserted: false`. |
| `replace_value` | Existing key's value is replaced. |
| `allow_duplicates_right` | Equal keys are inserted into the right-side range. |

Recommended default:

```text
replace_value
```

It is deterministic and useful for map-like compiler tables.

## 4. `NodeIDBTree`

### 4.1 Summary

`NodeIDBTree` represents B-tree nodes as inline records stored in a `List`. Children are node ids, not recursive references. Keys and children inside each node are also lists.

This is the most readable B-tree representation and the best first target for generated code because it avoids buffer offset arithmetic. It is a natural specialization of `NodeIdAdjacencyGraph`: it is a directed, rooted, acyclic graph with one parent per non-root node, sorted keys in each node, bounded out-degree, and all leaves at the same depth.

Use `NodeIDBTree` when:

- You want generated code that is easy to inspect and debug.
- The tree is small or medium sized.
- The tree is built during compiler bootstrap or diagnostics-oriented phases.
- Insert/delete correctness matters more than raw traversal speed.
- The generator is still being validated.
- You need a flexible mutable-by-rebuild representation before lowering to packed buffers.

Avoid `NodeIDBTree` when:

- The tree is large.
- Search is hot and frequent.
- Node lookup by id must be O(1).
- You care about compact memory layout.
- You need to persist stable packed storage. Use `CsrBTree`.

### 4.2 Set-only shape

Design name:

```text
NodeIDBTreeInt64[S]
```

Silica inline shape:

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

Empty tree convention:

```text
root_id = -1
node_count = 0
nodes = empty list
```

### 4.3 Map shape

Design name:

```text
NodeIDBTreeMapInt64ToInt64[S]
```

Silica inline shape:

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
            values: List[int64, S],
            children: List[int64, S]
        },
        S
    ]
}
```

Invariant:

```text
length(values) == key_count
values[i] belongs to keys[i]
```

### 4.4 Node lookup

Because nodes are stored as a list, generated helper `find_node(tree, id)` scans `tree.nodes`.

Recommended return shape:

```silica
{ found: bool, node: { id: int64, key_count: int64, is_leaf: bool, keys: List[int64, S], children: List[int64, S] } }
```

For map shape, include `values`.

Cost:

```text
find_node: O(node_count)
search: O(height * (node_count + order))
```

This is intentionally not the final high-performance form. It is for clarity and bootstrapping.

### 4.5 Search algorithm

Generated search over one node (`key` is **`Collectable`**):

```text
node_search_position(keys, key: Collectable, index):
    if index == key_count:
        return { found: false, index: index }
    current = keys[index]
    if key == current:
        return { found: true, index: index }
    if key < current:
        return { found: false, index: index }
    recurse with index + 1
```

Tree search:

```text
search_node(tree, node_id, key: Collectable):
    node = find_node(tree, node_id)
    pos = node_search_position(node.keys, key, 0)
    if pos.found:
        return found
    if node.is_leaf:
        return not found
    child_id = child_at(node.children, pos.index)
    return search_node(tree, child_id, key)
```

Generated function names:

```text
btree_nodeid_int64_<space>_contains
btree_nodeid_map_int64_int64_<space>_get
btree_nodeid_int64_<space>_find_node
```

Abstract operand types: **`contains(..., key: Collectable)`**; **`get(..., key: Collectable)`** returns map values typed **`Collectable`** when present (`find_node` uses internal **`int64` node ids**, not `Collectable`, because those are structural graph ids, not user keys).

### 4.6 Insertion strategy

Use the standard top-down split strategy because it keeps recursion simpler:

1. If the root is full, allocate a new root.
2. Split the old root into two children.
3. Descend into a child that is guaranteed not full.
4. Before descending into any full child, split it.
5. Insert into a non-full leaf.

For `NodeIDBTree`, "allocate" means produce a new node record with the next id and prepend it to `nodes`, then rebuild modified ancestor nodes. This is list-heavy but generator-friendly.

Recommended generated result shape:

```silica
{
    tree: <full inline tree type>,
    inserted: bool,
    replaced: bool
}
```

For set-only trees, `replaced` can always be `false`.

Internal helper shapes (key/value operands **`Collectable`**):

```text
btree_nodeid_int64_<space>_split_child(tree, parent_id, child_index) -> { tree, promoted_key: Collectable }
btree_nodeid_int64_<space>_insert_nonfull(tree, node_id, key: Collectable) -> { tree, inserted: bool, replaced: bool }
btree_nodeid_map_int64_int64_<space>_insert_nonfull(tree, node_id, key: Collectable, value: Collectable) -> { tree, inserted: bool, replaced: bool }
```

### 4.7 Deletion strategy

Deletion is more complex than insertion. The first generated version may omit deletion and document the tree as append/update oriented.

When deletion is generated, use the standard top-down B-tree deletion algorithm:

1. Before descending, ensure the child has at least `min_children_non_root` keys when possible.
2. Borrow from left sibling if it has extra keys.
3. Else borrow from right sibling if it has extra keys.
4. Else merge with a sibling and pull down a separator key from the parent.
5. Delete from leaf directly.
6. Delete from internal node by replacing with predecessor or successor, or by merging children.
7. If the root becomes empty and has one child, make that child the new root.

Abstract public signature when deletion is emitted:

```text
delete(tree, key: Collectable) -> { tree, removed: bool }
```

Recommended initial generator stance:

```text
generate search and insert first
generate delete only after split/merge helpers are well-tested
```

### 4.8 Validation

Generated `validate` should check:

```text
root convention for empty/non-empty tree
node ids unique
all child ids point to existing nodes
no non-root node has multiple parents
root has no parent
key_count equals length(keys)
map shape: key_count equals length(values)
leaf child list is empty
internal child count is key_count + 1
keys sorted ascending
node key ranges obey parent separators
all leaves have same depth
node_count equals number of node records
```

For `NodeIDBTree`, validation can be slow. It is still valuable for generated-code tests.

## 5. `CsrBTree`

### 5.1 Summary

`CsrBTree` stores B-tree nodes and their variable-length key/child ranges in region buffers. Each node has offsets into packed key, value, and child buffers.

This is the performance-oriented representation and the most natural production specialization of `CompressedSparseRowGraph`: each node has a contiguous child range, plus a contiguous key range that defines the search partitions for that child range.

Use `CsrBTree` when:

- The tree is large.
- Search performance matters.
- The tree is built once and queried many times.
- The tree should be compact and cache-friendly.
- The generator has static data or can run a build/finalize phase.
- You want a stable representation for compiler indexes or symbol tables.

Avoid `CsrBTree` when:

- You need simple handwritten or easily inspected generated code.
- You will insert/delete frequently after construction.
- Node capacity or total key count is not known or bounded.
- You are still validating B-tree algorithm correctness. Start with `NodeIDBTree`.

### 5.2 Set-only shape

Design name:

```text
CsrBTreeInt64[R, S, NODE_CAP, KEY_CAP, CHILD_CAP]
```

Silica inline shape:

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

`node_is_leaf[id]` convention:

```text
1 = leaf
0 = internal
```

For node `id`:

```text
keys for node:
    keys[node_key_start[id] ... node_key_start[id] + node_key_count[id] - 1]

children for node:
    children[node_child_start[id] ... node_child_start[id] + node_child_count[id] - 1]
```

### 5.3 Map shape

Design name:

```text
CsrBTreeMapInt64ToInt64[R, S, NODE_CAP, KEY_CAP, CHILD_CAP]
```

Silica inline shape:

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
    values: buf(R, S, int64, KEY_CAP),
    children: buf(R, S, int64, CHILD_CAP)
}
```

Invariant:

```text
values[i] belongs to keys[i]
```

### 5.4 Why CSR-like storage fits B-trees

B-tree nodes are small sorted arrays. CSR storage gives every node a range into the `keys` buffer and every internal node a range into the `children` buffer.

This is not exactly graph CSR, because graph CSR usually has one `offsets` buffer. B-tree CSR has several offset/count buffers:

```text
node_key_start
node_key_count
node_child_start
node_child_count
```

That extra metadata is what lets a generator store both ordered separators and child links.

### 5.5 Static construction

When the tree is generated from static sorted data, prefer direct packed construction:

1. Choose `order`.
2. Partition sorted keys into leaf nodes.
3. Build parent levels bottom-up by selecting separator keys.
4. Assign node ids level by level or pre-order. Level order is easy to validate; pre-order is often locality-friendly.
5. Emit all node metadata buffers.
6. Emit packed `keys`, `values`, and `children`.

Recommended node id assignment for first generator:

```text
root = 0
then breadth-first level order
```

Reason: debugging and validation are easier because node ids increase by level.

### 5.6 Dynamic build/finalize strategy

For dynamic insertion-heavy builds, generate `NodeIDBTree` first, then finalize to `CsrBTree`.

Finalize steps:

1. Validate `NodeIDBTree`.
2. Count nodes, total keys, and total child links.
3. Allocate CSR buffers with concrete capacities.
4. Traverse nodes in chosen output order.
5. Write node metadata.
6. Copy keys into packed `keys`.
7. Copy values into packed `values`, for map shape.
8. Copy children into packed `children`.
9. Return `CsrBTree` with owning region.

This split keeps insertion logic readable and produces a compact search representation.

Recommended generated function:

```text
btree_nodeid_int64_<space>_to_csr
btree_nodeid_map_int64_int64_<space>_to_csr
```

### 5.7 Search algorithm

Search in a CSR node (`key` is **`Collectable`**):

```text
search_csr_node(tree, node_id, key: Collectable):
    key_start = read_buf(node_key_start, node_id)
    key_count = read_buf(node_key_count, node_id)
    pos = search_key_range(keys, key_start, key_count, key)
    if pos.found:
        return found
    is_leaf = read_buf(node_is_leaf, node_id)
    if is_leaf == 1:
        return not found
    child_start = read_buf(node_child_start, node_id)
    child_id = read_buf(children, child_start + pos.index)
    return search_csr_node(tree, child_id, key)
```

For small `order`, generated linear search inside a node is acceptable:

```text
O(order * height)
```

If `order` is large, generate binary search over each node's key range:

```text
O(log(order) * height)
```

Recommended default:

```text
linear node search for order <= 16
binary node search for order > 16
```

### 5.8 Updates

Packed `CsrBTree` should be treated as immutable after construction in the first implementation. In-place insertion and deletion require shifting packed key and child ranges or using spare capacity per node, which complicates invariants.

If mutable CSR B-trees are later generated, use a different design name:

```text
MutablePagedCsrBTree
```

That future representation should reserve fixed-size per-node pages:

```text
node_keys: buf(R, S, int64, NODE_CAP * MAX_KEYS)
node_values: buf(R, S, int64, NODE_CAP * MAX_KEYS)
node_children: buf(R, S, int64, NODE_CAP * MAX_CHILDREN)
```

This is no longer compressed CSR; it is a paged array B-tree.

### 5.9 Validation

Generated `validate` should check:

```text
root convention
node_count <= NODE_CAP
key_count_total <= KEY_CAP
all child ranges lie inside CHILD_CAP
all key ranges lie inside KEY_CAP
node_is_leaf values are 0 or 1
leaf nodes have child_count == 0
internal nodes have child_count == key_count + 1
keys sorted inside every node range
all child ids are in [0, node_count)
no non-root node has multiple parents
all leaves have same depth
parent separator ranges are respected
```

### 5.10 `NodeIDBTree` vs `CsrBTree`

Use `NodeIDBTree` when you want a representation that is easy to generate, inspect, mutate functionally, and validate. It is the right first form for bootstrapping tree algorithms.

Use `CsrBTree` when you want a compact, cache-friendly query structure. It is the right final form for generated compiler indexes, symbol lookup tables, static routing tables, and search-heavy runtime data.

Decision table:

| Question | Prefer `NodeIDBTree` | Prefer `CsrBTree` |
|----------|----------------------|-------------------|
| Is this the first generated version? | Yes | No |
| Is insertion/deletion frequent? | Yes | No, unless using a future paged mutable form |
| Is search performance critical? | No | Yes |
| Is the tree large? | No | Yes |
| Is the tree generated from static sorted data? | Maybe | Yes |
| Should humans inspect the emitted code? | Yes | Maybe |
| Are buffer capacities known? | Not required | Required |
| Is compact memory layout important? | No | Yes |

Recommended pipeline:

```text
dynamic construction -> NodeIDBTree -> validate -> CsrBTree -> query
static known keys -> CsrBTree directly
small debug structure -> NodeIDBTree only
```

## 6. `RegionBinaryHeap`

### 6.1 Summary

`RegionBinaryHeap` is an array-backed heap stored in a region buffer. It is the natural heap specialization of the dense/id-buffer graph style: the parent/child edges are implicit in array indices and do not need adjacency storage.

For index `i`:

```text
parent(i) = (i - 1) / 2
left(i) = 2 * i + 1
right(i) = 2 * i + 2
```

Use it when:

- You need a priority queue.
- You need repeated `push` and `pop_min` or `pop_max`.
- You do not need ordered traversal of all elements.
- You can choose a fixed capacity.
- You want simple, fast generated code.

Avoid it when:

- You need sorted iteration. Use a tree or sort after extraction.
- You need lookup by key. Use a B-tree/map-like representation.
- Capacity cannot be bounded.

### 6.2 Min-heap shape

Design name:

```text
RegionBinaryMinHeapInt64[R, S, CAP]
```

Silica inline shape:

```silica
{
    region: region(R, S),
    len: int64,
    capacity: int64,
    values: buf(R, S, int64, CAP)
}
```

Min-heap invariant:

```text
for every index i > 0:
    values[parent(i)] <= values[i]
```

Max-heap variant:

```text
RegionBinaryMaxHeapInt64[R, S, CAP]
for every index i > 0:
    values[parent(i)] >= values[i]
```

### 6.3 Key/value heap shape

For priority queues where payload differs from priority (both operand types **`Collectable`** in abstract push/pop APIs):


Design name:

```text
RegionBinaryMinHeapPriorityInt64ValueInt64[R, S, CAP]
```

Silica inline shape:

```silica
{
    region: region(R, S),
    len: int64,
    capacity: int64,
    priorities: buf(R, S, int64, CAP),
    values: buf(R, S, int64, CAP)
}
```

Invariant:

```text
priorities[parent(i)] <= priorities[i]
values[i] moves together with priorities[i]
```

This shape is the recommended first priority-queue variant for generated graph algorithms like Dijkstra once decrease-key policy is decided.

### 6.4 Construction

Empty heap:

```silica
fn heap_binary_min_int64_normal_empty() -> {
    region: region(R, normal),
    len: int64,
    capacity: int64,
    values: buf(R, normal, int64, CAP)
} {
    sequence proc[mem(normal)]
        R: lifetime <- fresh_lifetime();
        r: region(R, normal) <- alloc_region(normal);
        values: buf(R, normal, int64, CAP) <- alloc_buf(r, CAP);
    produces
        pure { region: r, len: 0, capacity: CAP, values: values }
    end
}
```

From static list:

1. Allocate buffer.
2. Copy values into `values[0..len)`.
3. Heapify bottom-up from `parent(len - 1)` down to `0`.

Heapify is O(n). Repeated push is O(n log n). Prefer heapify for known static inputs.

### 6.5 Push

The pushed element is **stored data**; the abstract API is **`push(heap, value: Collectable)`** (monomorphic generators emit `int64` and rely on `impl int64` for `Collectable`).

Generated `push` should return a result record:

```silica
{
    heap: <full inline heap type>,
    ok: bool
}
```

Algorithm:

```text
if heap.len == heap.capacity:
    return { heap: heap, ok: false }
write value (type Collectable) at index heap.len
new_len = heap.len + 1
sift_up(index = heap.len)
return updated heap with len = new_len, ok = true
```

Sift up for min-heap:

```text
if index == 0:
    done
p = parent(index)
if values[p] <= values[index]:
    done
swap values[p], values[index]
sift_up(p)
```

### 6.6 Peek and pop

Peek result (stored element typed **`Collectable`** in the abstract API):

```silica
{ ok: bool, value: Collectable }
```

`peek_min`:

```text
if len == 0:
    { ok: false, value: 0 }
else:
    { ok: true, value: values[0] }
```

Pop result:

```silica
{
    heap: <full inline heap type>,
    ok: bool,
    value: Collectable
}
```

`pop_min`:

```text
if len == 0:
    return { heap: heap, ok: false, value: 0 }
min_value = values[0]
last = values[len - 1]
new_len = len - 1
if new_len > 0:
    values[0] = last
    sift_down(0, new_len)
return { heap: heap with len = new_len, ok: true, value: min_value }
```

Sift down for min-heap:

```text
left = 2 * index + 1
right = 2 * index + 2
smallest = index
if left < len and values[left] < values[smallest]:
    smallest = left
if right < len and values[right] < values[smallest]:
    smallest = right
if smallest == index:
    done
swap values[index], values[smallest]
sift_down(smallest, len)
```

### 6.7 Validation

Generated `validate` should check:

```text
0 <= len <= capacity
capacity == CAP
for every i in [1, len):
    values[parent(i)] <= values[i]    // min-heap
```

For key/value heap:

```text
priorities obey heap order
values capacity matches priorities capacity
swaps always move priority and value together
```

## 7. `RegionDaryHeap`

### 7.1 Summary

`RegionDaryHeap` generalizes the binary heap by giving each node `D` children.

For index `i`:

```text
parent(i) = (i - 1) / D
child(i, k) = D * i + 1 + k, where 0 <= k < D
```

Use it when:

- The heap is large.
- `pop` operations dominate and fewer levels help.
- The target benefits from scanning several child priorities in tight code.
- `D` is a generator constant.

Avoid it when:

- Simplicity matters more than tuning.
- Heap sizes are small.
- Division by `D` is expensive and `D` is not a power of two.

Recommended first d-ary specialization:

```text
RegionFourAryMinHeapInt64
```

`D = 4` is a good practical default: shallower than binary, still cheap to scan children.

### 7.2 Shape

Design name:

```text
RegionDaryMinHeapInt64[R, S, CAP, D]
```

Silica inline shape:

```silica
{
    region: region(R, S),
    len: int64,
    capacity: int64,
    arity: int64,
    values: buf(R, S, int64, CAP)
}
```

Invariant:

```text
arity == D
for every index i > 0:
    values[parent(i)] <= values[i]
```

### 7.3 Operations

`push` is the same as binary heap except `parent(i)` uses `D`.

`pop_min` is the same as binary heap except `sift_down` chooses the smallest of up to `D` children:

```text
best = index
for k in 0..D:
    c = D * index + 1 + k
    if c < len and values[c] < values[best]:
        best = c
if best == index:
    done
swap index with best
sift_down(best)
```

Generated code should unroll child checks when `D` is a small constant, especially for `D = 4`.

## 8. Naming rules

Generated names should be deterministic:

```text
btree_nodeid_<shape>_<space>_<operation>
btree_csr_<shape>_<space>_<operation>
heap_binary_<kind>_<shape>_<space>_<operation>
heap_dary_<arity>_<kind>_<shape>_<space>_<operation>
```

Where:

```text
shape = int64 | map_int64_int64 | priority_int64_value_int64
kind = min | max
space = normal | normal_writethrough | normal_noncacheable | atomic
arity = 4 | 8 | ...
```

Examples:

```text
btree_nodeid_int64_normal_empty
btree_nodeid_int64_normal_insert
btree_nodeid_map_int64_int64_normal_get
btree_csr_int64_normal_contains
btree_csr_map_int64_int64_normal_get
heap_binary_min_int64_normal_empty
heap_binary_min_int64_normal_push
heap_binary_min_int64_normal_pop
heap_dary_4_min_int64_normal_pop
```

## 9. Generator requirements

### 9.1 Inputs

B-tree generator inputs:

```text
representation: nodeid_btree | csr_btree
shape: set_int64 | map_int64_int64
memory_space: normal | normal_writethrough | normal_noncacheable | atomic
order: int64
duplicate_policy: reject_duplicates | replace_value | allow_duplicates_right
node_capacity: int64, required for CSR
key_capacity: int64, required for CSR
child_capacity: int64, required for CSR
static_sorted_input: bool
generate_delete: bool
```

Heap generator inputs:

```text
representation: binary_heap | dary_heap
kind: min | max
shape: int64 | priority_int64_value_int64
memory_space: normal | normal_writethrough | normal_noncacheable | atomic
capacity: int64
arity: int64, for dary_heap
```

### 9.2 Emitted B-tree functions

Minimum `NodeIDBTree` set:

```text
empty
contains
insert
validate
height
key_count
```

Map extension:

```text
get
put
```

CSR set:

```text
from_static_sorted
from_nodeid
contains
validate
height
key_count
```

CSR map extension:

```text
get
```

Optional later:

```text
delete
range_scan
lower_bound
upper_bound
```

### 9.3 Emitted heap functions

Minimum heap set:

```text
empty
len
is_empty
is_full
peek
push
pop
validate
```

Priority/value extension:

```text
peek_priority
peek_value
push_priority_value
pop_priority_value
```

Optional later:

```text
heapify
replace_top
clear
```

### 9.4 Result shapes

Use records rather than exceptions for ordinary capacity or lookup failure.

Lookup (map **`get`** and similar; the retrieved payload is **`Collectable`**):

```silica
{ found: bool, value: Collectable }
```

Insert:

```silica
{
    tree: <full inline tree type>,
    inserted: bool,
    replaced: bool
}
```

Heap push:

```silica
{
    heap: <full inline heap type>,
    ok: bool
}
```

Heap pop (popped element **`Collectable`**):

```silica
{
    heap: <full inline heap type>,
    ok: bool,
    value: Collectable
}
```

### 9.5 Structural type emission

The generator must:

1. Produce the full inline record type for each generated family.
2. Reuse that exact string everywhere.
3. Keep record field order stable.
4. Include the owning region in any returned value that contains buffers.
5. Use exact concrete buffer capacities in type positions.
6. Keep `List` and `buf` memory spaces aligned with the enclosing `sequence proc[mem(S)]`.
7. Generate monomorphic comparison logic for the key type.
8. Declare **add / find / remove** operands (`key`, `value`, heap `value`, priority-queue priority and value where applicable) using concrete **`Collectable`** payload types in emitted signatures (see §2.4).

## 10. Implementation staging

Recommended staging:

1. Generate `RegionBinaryMinHeapInt64` first. It is compact and easy to validate.
2. Generate `NodeIDBTreeInt64` search and validation.
3. Generate `NodeIDBTreeInt64` insertion with top-down splitting.
4. Generate `NodeIDBTreeMapInt64ToInt64` by moving values with keys.
5. Generate `CsrBTreeInt64` from static sorted input.
6. Generate `NodeIDBTree -> CsrBTree` finalization.
7. Add `RegionDaryHeap`, starting with `D = 4`.
8. Add B-tree deletion only after split, merge, borrow, and validation helpers are stable.

## 11. References

- **`Collectable`** — language trait for keys/values and heap elements (§2.4; silica-spec §8.2.4).
- **Immutability and type invariance** — §2.5; graph §2.7–§2.8.
- [graph_representation_design.md](graph_representation_design.md) - graph families that these tree representations specialize (includes §2.4 `Collectable` for graph operands).
- [silica-specification.md](silica-specification.md) - inline structural types, effects, lists, regions, buffers.
- [list_implementation_design.md](list_implementation_design.md) - list storage and memory-space alignment.
- [region_memory_safety_todo.md](Phase1_TODOs/region_memory_safety_todo.md) - region lifetime implementation gaps relevant to returned buffers.
