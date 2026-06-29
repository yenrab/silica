# `Heap` Detailed Design

**Public trait:** `Heap`  
**Generated modules:** `brodal_okasaki_min`, `brodal_okasaki_max`  
**Shared core:** [`brodal_okasaki_queue.md`](brodal_okasaki_queue.md)

## 1. Abstract value

`Heap[ItemType, mem(SpaceType)]` is an immutable multiset with efficient access to one comparator-extreme item.

Min and max orientation are distinct generated concrete types even though both implement `Heap`. Duplicate comparator-equal items are retained and popped one at a time.

## 2. Constructor

```text
{ compare_item: fn(ItemType, ItemType) -> atom }
```

```silica
frontier: Heap[NodeIdType, mem(normal)]
    <- brodal_okasaki_min@empty({
        compare_item: compare_node_id
    });
```

The max module reverses ordering internally. User comparators always describe their natural ascending order.

## 3. Trait contract

```text
export trait Heap;
export len/1;
export is_empty/1;
export peek/1;
export compare_item/3;

required {
    fn len(heap: Heap) -> int64;
    fn peek(heap: Heap) -> {status: :not_found | :found, value: ItemType};
    fn compare_item(heap: Heap, a: ItemType, b: ItemType) -> atom;
}

provided {
    fn is_empty(heap: Heap) -> boolean;
}
```

`is_empty` is exactly `len(heap) == 0`.

## 4. Generated module surface

Both min and max modules export:

```text
export empty/1;
export singleton/2;
export push/2;
export peek/1;
export pop/1;
export meld/2;
export len/1;
export is_empty/1;
export from_list/2;
export validate/1;
```

Signatures:

```text
push(heap, item) -> Heap[ItemType, mem(SpaceType)]

pop(heap) -> {
    heap: Heap[ItemType, mem(SpaceType)],
    status: :not_found | :found,
    value: ItemType
}

meld(left, right) -> {
    heap: Heap[ItemType, mem(SpaceType)],
    compatible: boolean
}
```

## 5. Ordering behavior

For min heap, `peek`/`pop` select an item no greater than every other item.

For max heap, they select an item no less than every other item.

When several items compare equal, which representation is selected is unspecified. The heap is not stable by insertion order.

## 6. Operations

### Push

Adds exactly one occurrence, including when comparator-equal items already exist. It checks `len + 1` before allocation.

### Peek

Reads the distinguished root without allocation. Empty returns `status=:not_found`.

### Pop

Empty returns the identical heap and `status=:not_found`. Non-empty returns `status=:found`, the prior extreme value, and a new heap missing exactly that occurrence.

### Meld

Combines all occurrences from both inputs in `O(1)` worst-case via bootstrapping. It succeeds only for matching item type, space, orientation, and exact function-value ordering identity.

Incompatible meld returns the left heap with `compatible=false`; neither input changes.

### From list

Folds `push`. Because push is worst-case `O(1)`, construction is `O(n)`, not `O(n log n)`. The result retains all list elements, including duplicates.

## 7. Persistence and region rule

Push and meld allocate constant new structure and share both input roots. Pop allocates `O(log n)` nodes.

Because meld can make the result reference both operands, standard meld requires both heaps to have the same canonical arena identity in addition to the same memory-space type. Standard constructors for one generated specialization always resolve that canonical arena.

## 8. Empty/failure behavior

| Case | Result |
|---|---|
| peek empty | `status=:not_found` |
| pop empty | unchanged, `status=:not_found` |
| meld empty with compatible heap | other heap |
| meld incompatible ordering/orientation/region | left heap, `compatible=false` |
| invalid comparator atom | collection error |
| length overflow | no result published |

## 9. Invariants

The heap inherits every Brodal–Okasaki invariant and additionally guarantees:

1. public length equals logical occurrence count;
2. orientation is immutable;
3. item comparator, ordering identity, and arena are preserved;
4. min/max trait behavior matches orientation;
5. duplicates are counted independently.

## 10. Complexity

| Operation | Worst-case time | New structure |
|---|---:|---:|
| len/is empty/peek | `O(1)` | none |
| push | `O(1)` | `O(1)` |
| meld | `O(1)` | `O(1)` |
| pop | `O(log n)` | `O(log n)` |
| from list | `O(n)` | `O(n)` |
| validate | `O(n)` | diagnostics |

## 11. Example

```silica
h0: Heap[int64, mem(normal)] <- brodal_okasaki_min@empty({
    compare_item: compare_int64
});
h1: Heap[int64, mem(normal)] <- brodal_okasaki_min@push(h0, 9);
h2: Heap[int64, mem(normal)] <- brodal_okasaki_min@push(h1, 3);
p <- brodal_okasaki_min@pop(h2);
```

`p.value = 3`, `p.heap` contains `9`, and `h2` still contains both occurrences.

## 12. Exclusions

No handles, arbitrary delete, in-place mutation, stable tie order, or separate binary/d-ary heap representation is included.
