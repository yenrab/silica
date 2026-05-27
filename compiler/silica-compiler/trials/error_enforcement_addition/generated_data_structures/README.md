# Generated data-structure validation failures (Phase 1+)

Compile-fail goldens for **generated** graph, B-tree set, balanced tree, and heap invariant checks.

Kept under `error_enforcement_addition/` (not `graph_addition` / `list_addition`) so success-path generated-structure trials stay separate from compiler-internal list trials.

## Layout

| Subdirectory (future) | When populated | Example trial name |
|----------------------|----------------|-------------------|
| `graph/` | Phase 1–3 `validate` helpers | `graph_adj_invalid_neighbor_id.silica` |
| `btree_set/` | Phase 5–6 set validation | `btree_set_nodeid_duplicate_key.silica` |
| `balanced_tree/` | Phase 7–8 tree validation | `btree_nodeid_unsorted_keys.silica` |
| `heap/` | Phase 9 heap validation | `heap_binary_min_heap_order_violation.silica` |

## Conventions (same as other `error_enforcement_addition` trials)

- One `.silica` source per check; expected compiler diagnostic in `.golden_fail`.
- Run via `make -C error_enforcement_addition integrate` once trials exist (parent Makefile includes this tree).
- Do **not** add success-path `.scout` / `.ascomp` here; those live in `graph_addition`, `btree_set_addition`, `balanced_tree_addition`, `heap_addition`, or `standard_data_structures_addition`.

## Phase 0 status

Directories and naming rules only. No `.silica` or `.golden_fail` files until generated `validate` APIs exist.
