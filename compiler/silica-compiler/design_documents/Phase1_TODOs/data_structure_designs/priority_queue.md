# `PriorityQueue` Detailed Design

**Public trait:** `PriorityQueue`  
**Generated core:** `brodal_okasaki_min` over priority/value entries  
**Shared core:** [`brodal_okasaki_queue.md`](brodal_okasaki_queue.md)

## 1. Abstract value

`PriorityQueue[PriorityType, ItemType, mem(SpaceType)]` is an immutable multiset of `(priority, value)` entries.

Ordering is lexicographic:

1. compare priorities;
2. only when priorities compare `:equal`, compare values.

This deterministic value tie-break is part of the design. It is not FIFO stability.

## 2. Constructor

Separate-priority form:

```text
{
    compare_priority: fn(PriorityType, PriorityType) -> atom,
    compare_item: fn(ItemType, ItemType) -> atom
}
```

Embedded-priority adapters may additionally capture:

```text
priority_of: fn(ItemType) -> PriorityType
```

The standard generated storage still holds explicit `(priority,value)` entries so that later extractor behavior cannot change stored priority.

## 3. Trait contract

```text
export trait PriorityQueue;
export len/1;
export is_empty/1;
export peek_priority/1;
export peek_value/1;
export peek/1;

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
        -> {status: :not_found | :found, priority: PriorityType};
    fn peek_value(queue: PriorityQueue)
        -> {status: :not_found | :found, value: ItemType};
}
```

The two projected peek methods must describe the same entry as `peek`.

## 4. Generated module surface

The priority-queue specialization exports:

```text
export empty_priority_queue/1;
export push_priority/3;
export peek_priority_entry/1;
export pop_priority/1;
export meld_priority/2;
export len/1;
export from_entries/2;
export validate/1;
```

Representation-specific names avoid arity/type ambiguity with the ordinary heap surface when emitted in the same module family.

Normative results:

```text
pop_priority(queue) -> {
    queue: PriorityQueue[PriorityType, ItemType, mem(SpaceType)],
    status: :not_found | :found,
    priority: PriorityType,
    value: ItemType
}

meld_priority(left, right) -> {
    queue: PriorityQueue[PriorityType, ItemType, mem(SpaceType)],
    compatible: boolean
}
```

## 5. Duplicate and tie behavior

Every push adds one occurrence. Two entries may compare equal in both priority and value and still coexist.

Pop chooses an arbitrary representation among fully comparator-equal entries. If stable insertion ordering is later required, it needs an explicit monotonic sequence field and a different public ordering contract.

## 6. Push, peek, pop, and meld

These are the Heap operations applied to inline entries:

```text
{priority: PriorityType, value: ItemType}
```

The entry comparator validates both comparator results. Priority comparison short-circuits value comparison unless equal.

Meld compatibility covers the exact priority- and item-comparator function values, orientation, canonical arena identity, and entry representation.

## 7. No arbitrary deletion or decrease-key

The Phase 1 `PriorityQueue` exposes neither arbitrary-entry deletion nor decrease-key. The selected Brodal–Okasaki core has no persistent node handles and supplies only empty, push, peek, pop, and meld semantics.

`delete_entry` and `decrease_priority` are not generated, optional, or hidden operations. Algorithms requiring reprioritization need a separately designed adapter with its own correctness and complexity contract; that adapter is not part of this data structure.

## 8. Bulk construction

`from_entries` folds push in `O(n)` worst-case total. Input type:

```text
List[{priority: PriorityType, value: ItemType}, SpaceType]
```

No entry is deduplicated.

## 9. Empty/failure behavior

Empty peek/pop return `status=:not_found`; payload fields are inaccessible. Successful peek/pop return `status=:found`. Incompatible meld returns the left queue unchanged. Length overflow or comparator failure publishes no partial queue.

## 10. Persistence

The queue uses the shared bootstrapped heap core. Old queues remain valid. Priority/value fields move together through every link and pop.

## 11. Invariants

1. shared Brodal–Okasaki invariants hold for entry comparison;
2. each stored node contains one complete priority/value pair;
3. public length counts occurrences;
4. priority and value comparator bundle is uniform;
5. the root is lexicographically minimal;
6. projected peek methods agree;
7. both fields remain aligned across persistence and meld.

## 12. Complexity

| Operation | Worst-case time |
|---|---:|
| len/is empty/peek | `O(1)` |
| push | `O(1)` |
| meld | `O(1)` |
| pop | `O(log n)` |
| from entries | `O(n)` |
| validate | `O(n)` |

## 13. Example

```silica
q0: PriorityQueue[int64, string, mem(normal)]
    <- brodal_okasaki_min@empty_priority_queue({
        compare_priority: compare_int64,
        compare_item: compare_string
    });

q1 <- brodal_okasaki_min@push_priority(q0, 10, "later");
q2 <- brodal_okasaki_min@push_priority(q1, 2, "now");
p <- brodal_okasaki_min@pop_priority(q2);
```

`p.priority = 2` and `p.value = "now"`.

## 14. Explicit exclusion

Arbitrary-entry deletion, decrease-key, persistent handles, and an auxiliary priority index are outside the Phase 1 `PriorityQueue` contract.
