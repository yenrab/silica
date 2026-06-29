# `Tree` Detailed Design

**Public trait:** `Tree`  
**Generated module:** `tree_rose`  
**Representation:** persistent rose tree with skew-binary child-slot sequences

## 1. Abstract value

`Tree[ItemType, mem(SpaceType)]` is a non-empty rooted ordered rose tree. Each node has:

- one item;
- zero or more child **slots**;
- a stable slot number for every child ever appended at that parent.

Child order is append order. Removing a child vacates its slot and removes the entire child subtree from the logical tree. Surviving sibling slot numbers do not change.

This stable-slot rule makes logarithmic child removal compatible with a skew binary random-access list.

## 2. Constructor record and root creation

```text
{
    compare_item: fn(ItemType, ItemType) -> atom
}
```

The comparator supports deterministic item search and validation helpers; child placement never depends on item ordering.

```text
tree_rose@singleton(functions, root_item)
    -> Tree[ItemType, mem(SpaceType)]
```

There is no itemless empty `Tree`. Absence of a tree is represented by a query result's `status=:not_found`. This avoids an invalid root payload.

## 3. Path model

An expository `TreePath` is:

```text
List[int64, SpaceType]
```

The empty path addresses the root. Each component is a stable child slot at the current node.

Paths are value-relative: a path valid in an old tree version remains valid in a descendant version unless that path or an ancestor is removed. Child slots are never compacted, reused, or renumbered.

Negative components are invalid.

## 4. Node representation

Logical node:

```text
(
    item: ItemType,
    subtree_node_count: int64,
    live_child_count: int64,
    slot_count: int64,
    child_slots: SkewRAL[
        :vacant
        | (:occupied, ref(R, SpaceType, rec))
    ]
)
```

`child_slots` uses reverse physical orientation:

```text
physical_index = slot_count - 1 - logical_slot
```

Appending a child prepends one occupied physical slot in `O(1)`. Existing logical slots continue to address the same entries.

## 5. Trait contract

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

required {
    fn node_count(tree: Tree) -> int64;
    fn root_item(tree: Tree) -> ItemType;
    fn get(tree: Tree, path: List[int64, SpaceType])
        -> {status: :not_found | :found, value: ItemType};
    fn child_count(tree: Tree, path: List[int64, SpaceType])
        -> {status: :not_found | :found, count: int64};
    fn child_slot_count(tree: Tree, path: List[int64, SpaceType])
        -> {status: :not_found | :found, count: int64};
    fn child_at(tree: Tree, path: List[int64, SpaceType], slot: int64)
        -> {
            status: :not_found | :found,
            value: ItemType,
            child_path: List[int64, SpaceType]
        };
    fn fold_preorder(tree: Tree, init: AccType,
        step: fn(AccType, List[int64, SpaceType], ItemType) -> AccType
    ) -> AccType;
    fn compare_item(tree: Tree, a: ItemType, b: ItemType) -> atom;
}
```

Path lists returned during fold are immutable snapshots. An internal fold may use a cursor to avoid allocating a new full path at every node.

## 6. Generated module surface

```text
export singleton/2;
export replace_item/3;
export add_child/3;
export add_subtree/3;
export remove_child/3;
export get/2;
export child_at/3;
export find_first/2;
export fold_preorder/3;
export validate/1;
```

Normative update results:

```text
replace_item(tree, path, value) -> {
    tree: Tree[ItemType, mem(SpaceType)],
    changed: boolean
}

add_child(tree, parent_path, value) -> {
    tree: Tree[ItemType, mem(SpaceType)],
    added: boolean,
    child_slot: int64
}

add_subtree(tree, parent_path, subtree) -> {
    tree: Tree[ItemType, mem(SpaceType)],
    added: boolean,
    child_slot: int64,
    compatible: boolean
}

remove_child(tree, parent_path, child_slot) -> {
    tree: Tree[ItemType, mem(SpaceType)],
    removed: boolean,
    removed_node_count: int64
}
```

## 7. Path lookup

Starting from root, each path component:

1. checks `0 <= slot < slot_count`;
2. converts logical slot to physical index;
3. looks up the random-access-list entry;
4. fails if vacant;
5. descends through the occupied child reference.

An empty path succeeds at root.

If depth is `h` and branching slot counts along the path are `b_i`, lookup time is:

```text
O(sum(log b_i), i=0..h-1)
```

It is not correctly described as only `O(log n)` for the whole tree.

## 8. Add child

At the parent:

- allocate a singleton child;
- assign `child_slot = old slot_count`;
- prepend `(:occupied, child_ref)` physically;
- increment live count and slot count;
- add one to subtree count.

Every ancestor on the path is copied and has subtree count incremented. Its child-slot random-access list is persistently updated to point at the newly copied descendant, costing `O(log b_i)` at an ancestor with `b_i` slots. Unchanged sibling slots and subtrees are shared.

`add_subtree` adds the subtree's cached node count. It requires matching item type, space, comparator ordering identity, and canonical arena identity. Standard constructors for the same specialization and space already share the canonical arena.

## 9. Remove child

At the parent:

- reject out-of-range or vacant slot with `removed=false`;
- read the occupied child's cached subtree count `k`;
- persistent-update that slot to `:vacant`;
- decrement live-child count;
- keep slot count unchanged;
- subtract `k` from subtree count at the parent and every copied ancestor.

On the unwind path, each ancestor's child-slot sequence is persistently updated to reference the new descendant and its cached subtree count is decremented.

The removed subtree remains alive through old tree versions and any independently held subtree value.

No sibling is shifted or renumbered.

## 10. Replace and traversal

`replace_item` changes only the target node item and copies the root-to-target path. Counts and slot numbers are unchanged, but every ancestor copies the random-access-list path needed to replace its child reference.

Preorder visits:

1. node;
2. occupied child slots in ascending logical slot order;
3. each child subtree recursively.

Vacant slots are skipped.

`find_first` returns the first preorder path whose item compares equal.

## 11. No compaction

The Phase 1 `Tree` does not expose compaction. Vacant child slots remain tombstones in the canonical arena, and slot numbers are never reused or renumbered. This preserves stable paths without requiring a path-remapping type.

## 12. Empty/failure cases

| Case | Result |
|---|---|
| empty path | root |
| missing/vacant path component | query `status=:not_found`; update unchanged |
| remove vacant/out-of-range slot | `removed=false` |
| add at missing parent | `added=false` |
| incompatible subtree graft | unchanged, `compatible=false` |
| count/slot overflow | no new tree published |
| invalid comparator atom in search | collection error |

## 13. Invariants

For every node:

```text
slot_count = length(child_slots)
live_child_count = number of occupied slots
subtree_node_count
  = 1 + sum(occupied_child.subtree_node_count)
0 <= live_child_count <= slot_count
```

Additionally:

1. root count equals public node count;
2. all child references belong to one region and are acyclic;
3. no child node has two parents within one tree value;
4. random-access-list digit/tree invariants hold;
5. reverse physical orientation is used consistently;
6. comparator and ordering identity are preserved;
7. stable slots are never reused or compacted.

## 14. Validation

Validation walks all slots, validates every random-access list, counts occupied children and descendant nodes, verifies acyclicity/single parentage, and checks cached counts.

Time is `O(n + s)`, including tombstones. Stack depth is tree depth plus `O(log max_slots)` for RAL traversal.

## 15. Complexity

Let `h` be target depth and `b_i` slot count at each ancestor.

| Operation | Time | Allocation |
|---|---:|---:|
| root item/node count | `O(1)` | none |
| get child/path | `O(sum log b_i)` | returned path/list only |
| add leaf child | `O(sum log b_i + h)` | `O(h + sum log b_i)` |
| remove child | `O(sum log b_i + h)` | `O(h + sum log b_i)` |
| replace item | `O(sum log b_i + h)` | `O(h + sum log b_i)` |
| preorder fold | `O(n + s)` | accumulator/path-dependent |
| find first | `O(n + s)` worst | result path |
| validate | `O(n + s)` | diagnostics |

## 16. Example

```silica
t0: Tree[string, mem(normal)] <- tree_rose@singleton({
    compare_item: compare_string
}, "root");

a <- tree_rose@add_child(t0, []: List[int64, normal], "left");
b <- tree_rose@add_child(a.tree, []: List[int64, normal], "right");
r <- tree_rose@remove_child(b.tree, []: List[int64, normal], a.child_slot);
```

The `"right"` child keeps its original slot after `"left"` is removed.

## 17. Exclusions

This is not `SearchTree`; child items are not ordered by comparator. There is no hidden parent pointer, compaction, sibling renumbering on removal, or claim of whole-tree `O(log n)` path updates independent of depth.
