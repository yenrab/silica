# Heap generated-structure trials (Phase 9)

`RegionBinaryMinHeap`, `RegionBinaryMaxHeap`, and `RegionDaryMinHeap` generated code. See `balanced_tree_and_heap_design.md` §6-§7.

## Trials

| Trial | Module | Coverage |
|-------|--------|----------|
| `heap_binary_min_empty` | `heap_binary_min` | empty, len, is_empty, peek, validate |
| `heap_binary_min_push_pop` | `heap_binary_min` | push, peek, pop, immutability from empty |
| `heap_binary_min_validate_invalid` | `heap_binary_min` | heap-order validation failure |
| `heap_binary_min_priority_push_pop` | `heap_binary_min` | priority/value variant (§6.3) |
| `heap_binary_max_push_pop` | `heap_binary_max` | max-heap push/pop (step 9.2) |
| `heap_dary_min_empty` | `heap_dary_min` | D=4 empty, len, is_empty, peek, arity, validate |
| `heap_dary_min_push_pop` | `heap_dary_min` | D=4 push, peek, pop, validate |
| `heap_dary_min_validate_invalid` | `heap_dary_min` | D=4 heap-order validation failure |

Binary min, binary max, and d-ary min modules compile in separate batches (E4011: same `operation[brackets]`).
