# Layer 0 §6.3 — Requirements-to-Trials Ledger

**Recorded:** 2026-06-29  
**Status:** Initial coverage map (Layer 0), amended 2026-07-02 for BinaryTree; trial stems are planned until their acceptance gates pass
**Authority:** [`standard_data_structures_implementation_plan.md`](../standard_data_structures_implementation_plan.md) §6.3–§6.4  
**Trial root:** [`trials/standard_data_structures_phase1/`](../../trials/standard_data_structures_phase1/)

## Purpose

Every numbered section in the Phase 1 detailed design suite must appear here with at least one coverage artifact:

| Kind | Code | Meaning |
|---|---|---|
| **Trial** | `T` | Runnable acceptance trial under a dependency leaf (`.silica` + `.ascomp` + `.scout`, or runtime-error `.scout`) |
| **Compile-time** | `C` | Expected compile failure (`.golden_fail`) or compile-only fixture |
| **Invariant** | `I` | Non-executable mathematical or structural rule; checked by `validate/1` or design review until a trial exists |
| **Out-of-scope** | `O` | Explicit exclusion copied from the design; must not be implemented |

**Maintenance rule:** When a `T` or `C` trial lands, update the **Artifact** column and remove any provisional `I` for the same requirement. When §6.4 CSR/dense contract rows gain trials, update those rows in [§6.4](#64-closed-csrdense-representation-contract).

**Layer 0 smoke only:** `*/smoke_harness_ready.silica` verifies harness integration; it does **not** satisfy design-section coverage.

---

## Algorithm map — locked decisions

Source: [`data_structure_to_algorithms.md`](../data_structure_to_algorithms.md) §Locked decisions

| # | Decision | Kind | Artifact |
|---|---|---|---|
| 1 | Adams WBT for all `OrderedSet` key types | T | `wbt_core/` → `wbt_set_insert_lookup` (planned) |
| 2 | No integer-key Patricia / crit-bit specialization | O | "None — no Patricia / crit-bit trie" |
| 3 | `OrderedMap` uses WBT map | T | `wbt_core/` → `wbt_map_insert_replace`, `wbt_map_insert_orders`, `wbt_map_canonical_key`, `wbt_map_value_pairing`, `wbt_map_compare_value_not_called`, `wbt_map_replace_persistence`, `wbt_map_insert_payload_shapes`, `wbt_map_insert_tuple_value`, `trial_collection_error_wbt_map_insert_invalid_comparator` (**§8A.6 pass**) |
| 4 | `SearchTree` same as `OrderedSet` (WBT) | T | `terminal_structures/` → `search_tree_wbt_adapter` (planned) |
| 5 | Graph vertex index is WBT keyed by `compare_node` | T | `live_graphs/` → `graph_outer_wbt_vertex` (planned) |
| 6 | Unweighted neighbors: inner WBT **set** of targets | T | `live_graphs/` → `graph_unweighted_inner_set` (planned) |
| 7 | Weighted neighbors: inner WBT map `to → edge data` | T | `live_graphs/` → `graph_weighted_inner_map` (planned) |
| 8 | CSR = live WBT + optional O(V+E) freeze | T | `snapshot_graphs/` → `csr_freeze_from_live` (planned) |
| 9 | Dense matrix = skew RAL cells (not bitset) | T | `snapshot_graphs/` → `dense_ral_cells` (planned) |
| 10 | Dense bitset graph not in scope | O | "Dense bitset graph — family removed" |
| 11 | `Tree` children = skew binary RAL | T | `skew_ral_core/` → `ral_child_sequence` (planned) |
| 12 | `Heap` = Brodal–Okasaki | T | `brodal_okasaki_core/` → `bo_push_pop_meld` (planned) |
| 13 | `PriorityQueue` = Brodal–Okasaki on pairs | T | `terminal_structures/` → `pq_lexicographic_order` (planned) |
| 14 | No decrease-key / arbitrary PQ deletion in Phase 1 | O | "Decrease-key — not in Phase 1 PriorityQueue API" |
| 15 | Bulk WBT: fold insert + optional `from_sorted` O(n) | T | `wbt_core/` → `wbt_from_sorted_linear` (planned) |
| 16 | `BinaryTree` = fixed left/right persistent tree + zipper | T | `binary_tree_core/` → `binary_tree_path_copy`, `binary_tree_zipper_roundtrip` (planned) |

---

## `common_contract.md`

| Sec | Summary | Kind | Artifact |
|---|---|---|---|
| 1 | Public bracket types for ten collection families | C | nine-family `collection_bracket_type_parse` (**§7.6 pass**); `BinaryTree` extension → `binary_tree_bracket_type_parse` (§7.10 planned) |
| 1.1 | Overriding genericity rule — no hard-coded payload types | C | `error_enforcement/` → `constructor_witness_item_mismatch` (planned) |
| 2 | No user-defined surface aliases; inline shapes only | I | Design §2; emitter uses inline records only |
| 3 | Constructor resolves from binding type + possibly-empty function record + explicit value witnesses | C | nine-family constructor trials (**§7.5 pass**); BinaryTree `{}` trials → `binary_tree_empty_constructor_record`, `trial_compile_fail_binary_tree_empty_unwitnessed` (§7.10 planned) |
| 4 | Comparator returns only `:less \| :equal \| :greater` | T | `error_enforcement/` → `comparator_invalid_atom` (planned) |
| 5 | Exact function-value ordering identity; no override | T | `compiler_substrate/` → `ordering_identity_top_level`, `ordering_identity_closure_copy`, `ordering_identity_closures`, `ordering_identity_orientation`, `ordering_identity_meld_reject` (**§7.2 pass**) |
| 6 | Query status `{status, value}`; update result shapes | T | `ordered_collections/` → `lookup_status_not_found` (planned) |
| 7 | Canonical arena; path copying; no in-place mutation | T | `compiler_substrate/` → `canonical_arena_reuse`, `canonical_arena_different_specialization`, `canonical_arena_different_space` (**§7.1 pass**) |
| 8 | Recursive tuples, `ref?`, `alloc_rec` encoding | T | `compiler_substrate/` → `recursive_tuple_alloc_fixture`, `recursive_tuple_multi_node`, `recursive_tuple_fn_in_node`, `recursive_tuple_list_in_node`; `error_enforcement/` → `trial_compile_fail_recursive_type_mismatch`, `trial_compile_fail_unguarded_rec` (**§7.3 pass**) |
| 7.4 | Trait dispatch; required/provided; link-name mangling | T | `compiler_substrate/` → `trait_dispatch_provided_fold_contains`, `trait_dispatch_dual_trait_record`, `trait_dispatch_override_provided`, `trait_dispatch_link_mangle_specializations`; `error_enforcement/` → `trial_compile_fail_trait_dispatch_no_impl`, `trial_compile_fail_trait_assoc_unresolved`, `trial_compile_fail_trait_missing_required_impl` (**§7.4 pass**) |
| 7.6 | Collection type witnesses and representation registry | C | `compiler_substrate/` → `collection_bracket_type_parse`, `collection_registry_specialization_distinct`, `collection_record_not_collection`; `error_enforcement/` → `trial_compile_fail_collection_bracket_missing_mem`, `trial_compile_fail_collection_unregistered_module` (**§7.6 pass, integrate verified**) |
| 9 | Checked `int64` arithmetic rejects overflow | T | `compiler_substrate/` → `checked_int64_overflow`, `runtime_buf_dynamic_size` (**§7.8 pass, integrate verified**) |
| 7.9 | Constructor runtime lowering; ordering bundles on merge | C | `compiler_substrate/` → `constructor_canonical_arena_lowering`, `constructor_record_field_order`, `constructor_record_resolution`, `constructor_stub_empty_run`, `constructor_ordering_bundle` (**§7.9 pass, integrate verified `60 0`**; goldens re-recorded 2026-07-02 after constructor-arg marshaling fix) |
| 7.10 | BinaryTree family registration; empty-record lowering without ordering bundle | C | `compiler_substrate/` → `binary_tree_bracket_type_parse`, `binary_tree_empty_constructor_record`, `binary_tree_stub_run`; `error_enforcement/` → `trial_compile_fail_binary_tree_empty_unwitnessed`, `trial_compile_fail_binary_tree_constructor_extra_field` (planned) |
| 10 | Trait vs generated-module separation | I | Design §10; stdlib module layout review |
| 11 | Materialization policy; internal fold hooks | T | `live_graphs/` → `neighbors_fold_no_temp` (planned) |
| 12 | `validate` result shape `{valid, error, logical_count}` | T | `wbt_core/` → `validate_malformed_fixture`; `binary_tree/` → `binary_tree_validate` (planned) |

---

## `weight_balanced_tree.md`

| Sec | Summary | Kind | Artifact |
|---|---|---|---|
| 1 | WBT core boundary (shared set/map/graph index) | T | `stdlib/data_structures/wbt_set.silica`, `wbt_map.silica`; `wbt_core/` → `wbt_empty_representation`, `wbt_constructor_ordering_bundle` (**§8A.1 pass**) |
| 2 | Mathematical BST model with cached size | I | Design §2 |
| 3 | `(DELTA, GAMMA) = (3, 2)` balance definition | I | Design §3; HY11 reference |
| 4 | Logical node shapes (set vs map) | T | `wbt_core/` → `wbt_representation_specializations`, `wbt_representation_string_specialization`, `wbt_generic_payload_shapes`, `wbt_generic_tuple_map_empty` (**§8A.1 pass**) |
| 5 | Smart constructor with overflow-safe size | T | `stdlib/data_structures/wbt_set.silica`, `wbt_map.silica`; `wbt_core/` → `wbt_smart_node_size`, `wbt_smart_node_arena`, `wbt_smart_node_production_path`, `trial_collection_error_wbt_smart_node_overflow`; `check-wbt-alloc-rec-gate` (**§8A.3 pass**) |
| 6 | Search helpers | T | `stdlib/data_structures/wbt_set.silica`, `wbt_map.silica` (read-only exports); `wbt_core/` → `wbt_read_only_empty`, `wbt_search_contains`, `wbt_minimum_maximum`, `trial_collection_error_wbt_search_invalid_comparator` (**§8A.2 pass**) |
| 7 | `balance_left` / `balance_right` contract | T | `stdlib/data_structures/wbt_set.silica`, `wbt_map.silica`; `wbt_core/` → `wbt_balance_boundaries`, `wbt_rotation_single_left`, `wbt_rotation_single_right`, `wbt_rotation_double_left`, `wbt_rotation_double_right`, `wbt_rotation_gamma_equality`, `wbt_rebalance_adversarial`, `trial_collection_error_wbt_balance_missing_child` (**§8A.4 pass**) |
| 8 | Set insertion + duplicate no-op | T | `stdlib/data_structures/wbt_set.silica` (`insert/2`); `wbt_core/` → `wbt_set_insert_duplicate`, `wbt_set_insert_orders`, `wbt_set_insert_adversarial`, `wbt_set_insert_persistence`, `wbt_set_insert_sharing`, `wbt_set_insert_payload_shapes` (string payload matrix), `trial_collection_error_wbt_set_insert_invalid_comparator`; `compiler_substrate/` → `collection_ordered_set_space_matrix` (all supported `SpaceType` keys) (**§8A.5 pass**) |
| 9 | Map insert/replace on duplicate key | T | `stdlib/data_structures/wbt_map.silica` (`insert/3`); `wbt_core/` → `wbt_map_insert_replace`, `wbt_map_insert_orders`, `wbt_map_canonical_key`, `wbt_map_value_pairing`, `wbt_map_compare_value_not_called`, `wbt_map_replace_persistence`, `wbt_map_insert_payload_shapes`, `wbt_map_insert_tuple_value`, `trial_collection_error_wbt_map_insert_invalid_comparator`; `check-wbt-alloc-rec-gate` (**§8A.6 pass**) |
| 10 | Deletion + successor/min extraction | T | `stdlib/data_structures/wbt_set.silica`, `wbt_map.silica` (`delete/2`, `delete_min`, `delete_max`); `wbt_core/` → `wbt_delete_absent`, `wbt_delete_leaf`, `wbt_delete_one_child`, `wbt_delete_two_child`, `wbt_delete_root`, `wbt_delete_extreme`, `wbt_delete_heavier_side`, `wbt_delete_rebalance`, `wbt_delete_persistence`, `wbt_delete_payload_shapes`, `trial_collection_error_wbt_{set,map}_delete_invalid_comparator` (**§8A.7 pass**) |
| 11 | Ordered fold / early-exit fold order | T | `wbt_core/` → `wbt_fold_ascending`, `wbt_read_only_alloc_free`, `wbt_weight_checked` (**§8A.2 pass**) |
| 12 | `from_sorted` linear O(n) builder | T | **Not green / incomplete** — `wbt_core/` → `wbt_from_sorted_empty` covers empty set/map success paths, `wbt_from_sorted_singleton` covers singleton set/map `from_sorted` plus `from_sorted_counted(..., 1)`, and `wbt_from_sorted_two` covers two-element set/map `from_sorted` plus `from_sorted_counted(..., 2)`; all three are enumerated, recorded, and passing. Planned deterministic-shape / malformed-input / count-mismatch / linearity / persistence `wbt_from_sorted_*` gate trials are still not passing; do not count §8A.8 complete until they are restored, recorded, and green without hangs. |
| 13 | Optional join/split (same balance law) | O | "Public set/map APIs do not initially require union or split" |
| 14 | Structural invariants (size, order, balance) | T | `wbt_core/` → `wbt_validate_invariants` (planned) |
| 15 | Postorder validation algorithm | T | `wbt_core/` → `wbt_validate_detect_cycle` (planned) |
| 16 | Complexity table | I | Design §16 bounds |
| 17 | References | I | Bibliography only |

---

## `persistent_binary_tree.md`

| Sec | Summary | Kind | Artifact |
|---|---|---|---|
| 1 | Fixed-role persistent binary-tree boundary | I | Design §1 |
| 2 | Empty/node mathematical model and logical counts | I | Design §2 |
| 3 | Inline recursive tuple; no node alias | C | `binary_tree_core/` → `binary_tree_inline_recursive_shape` (planned) |
| 4 | Canonical arena + specialization owning record | T | `binary_tree_core/` → `binary_tree_arena_specialization` (planned) |
| 5 | Sole smart constructor with checked counts | T | `binary_tree_core/` → `binary_tree_smart_node_count`, `trial_collection_error_binary_tree_count_overflow` (planned) |
| 6 | Constant-time root/child queries | T | `binary_tree_core/` → `binary_tree_direct_child_queries` (planned) |
| 7 | `:left \| :right` path model | T | `binary_tree_core/` → `binary_tree_path_lookup` (planned) |
| 8 | Persistent item/child replacement | T | `binary_tree_core/` → `binary_tree_path_copy` (planned) |
| 9 | Subtree graft arena compatibility | T | `binary_tree_core/` → `binary_tree_graft_compatibility` (planned) |
| 10 | Pre/in/postorder folds and shape-preserving maps | T | `binary_tree_core/` → `binary_tree_fold_orders`, `binary_tree_map_shape` (planned) |
| 11 | Inline functional zipper | T | `binary_tree_core/` → `binary_tree_zipper_roundtrip` (planned) |
| 12 | Structural sharing and logical multiplicity | T | `binary_tree_core/` → `binary_tree_sharing_multiplicity` (planned) |
| 13 | Failure behavior | T | `binary_tree_core/` → `binary_tree_missing_path_noop` (planned) |
| 14 | Structural invariants | T | `binary_tree_core/` → `binary_tree_validate_invariants` (planned) |
| 15 | Active-path cycle validation | T | `binary_tree_core/` → `binary_tree_validate_cycle_and_shared_subtree` (planned) |
| 16 | Complexity bounds | I | Design §16; operation-counter trial `binary_tree_complexity_observations` (planned) |
| 17 | Exclusions + Huet zipper reference | O | Design §17 exclusion list; bibliography |

---

## `skew_binary_random_access_list.md`

| Sec | Summary | Kind | Artifact |
|---|---|---|---|
| 1 | Abstract immutable sequence | I | Design §1 |
| 2 | Skew weights and digit invariant | I | Design §2 |
| 3 | Tree + forest spine encoding | T | `skew_ral_core/` → `ral_encoding_roundtrip` (planned) |
| 4 | Prepend | T | `skew_ral_core/` → `ral_prepend_head_tail` (planned) |
| 5 | Head and tail | T | `skew_ral_core/` → `ral_prepend_head_tail` (planned) |
| 6 | Logarithmic lookup | T | `skew_ral_core/` → `ral_lookup_boundaries` (planned) |
| 7 | Persistent update at index | T | `skew_ral_core/` → `ral_update_persistence` (planned) |
| 8 | Append convention for tree/dense consumers | T | `skew_ral_core/` → `ral_append_convention` (planned) |
| 9 | Bulk construction | T | `skew_ral_core/` → `ral_bulk_build` (planned) |
| 10 | Fold and range traversal | T | `skew_ral_core/` → `ral_fold_range` (planned) |
| 11 | Forest invariants | T | `skew_ral_core/` → `ral_validate_invariants` (planned) |
| 12 | Validation | T | `skew_ral_core/` → `ral_validate_invariants` (planned) |
| 13 | Complexity | I | Design §13 bounds |
| 14 | Exclusions (no finger-tree RAL variant) | O | Design §14 explicit exclusions |

---

## `brodal_okasaki_queue.md`

| Sec | Summary | Kind | Artifact |
|---|---|---|---|
| 1 | Brodal–Okasaki algorithm identity | I | Design §1 |
| 2 | Min orientation; max via adapter (no negation) | T | `brodal_okasaki_core/` → `bo_max_orientation_adapter` (planned) |
| 3 | Skew-binomial tree shape | I | Design §3 |
| 4 | Primitive forest invariant | T | `brodal_okasaki_core/` → `bo_forest_invariant` (planned) |
| 5 | Bootstrapped representation | T | `brodal_okasaki_core/` → `bo_bootstrap_meld` (planned) |
| 6 | Empty, peek, length | T | `brodal_okasaki_core/` → `bo_empty_peek` (planned) |
| 7 | Insert O(1) | T | `brodal_okasaki_core/` → `bo_push_duplicate` (planned) |
| 8 | Meld O(1); compatibility checks | T | `brodal_okasaki_core/` → `bo_meld_incompatible` (planned) |
| 9 | Delete-min | T | `brodal_okasaki_core/` → `bo_pop_persistence` (planned) |
| 10 | Delete-min normalization | T | `brodal_okasaki_core/` → `bo_pop_persistence` (planned) |
| 11 | Persistence and strict bootstrapping | T | `brodal_okasaki_core/` → `bo_old_heap_valid_after_pop` (planned) |
| 12 | Rank/size/arena invariants | T | `brodal_okasaki_core/` → `bo_validate_invariants` (planned) |
| 13 | Validation | T | `brodal_okasaki_core/` → `bo_validate_invariants` (planned) |
| 14 | Complexity bounds | I | Design §14 table |
| 15 | Out of scope (decrease-key, array heaps, …) | O | Design §15 bullet list |

---

## `live_wbt_graph.md`

| Sec | Summary | Kind | Artifact |
|---|---|---|---|
| 1 | Outer WBT map + inner set/map adjacency | T | `live_graphs/` → `graph_adjacency_shape` (planned) |
| 2 | Outer record fields | I | Design §2 layout |
| 3 | Explicit vertices; auto-add on edge (directed) | T | `live_graphs/` → `graph_add_vertex_isolated` (planned) |
| 4 | Directed insertion semantics | T | `live_graphs/` → `graph_directed_add_edge` (planned) |
| 5 | Undirected symmetric insertion | T | `live_graphs/` → `graph_undirected_mirror` (planned) |
| 6 | Removal without vertex delete | T | `live_graphs/` → `graph_remove_edge_only` (planned) |
| 7 | Queries (`has_vertex`, `has_edge`, degree) | T | `live_graphs/` → `graph_query_has_edge` (planned) |
| 8 | Ordering of neighbors | T | `live_graphs/` → `graph_neighbors_sorted` (planned) |
| 9 | Count invariants (self-loop rules) | T | `live_graphs/` → `graph_self_loop_counts` (planned) |
| 10 | Path copying persistence | T | `live_graphs/` → `graph_persistence_old_adj_valid` (planned) |
| 11 | Validation complexity | T | `live_graphs/` → `graph_validate_pass_fail` (planned) |
| 12 | Complexity table | I | Design §12 bounds |

---

## `csr_graph_snapshot.md`

| Sec | Summary | Kind | Artifact |
|---|---|---|---|
| 1 | Snapshot-only semantic role | I | Design §1; no incremental CSR update |
| 2 | Deterministic dense slot assignment | T | `snapshot_graphs/` → `csr_node_to_slot_sorted` (planned) |
| 3 | Physical buffer shape | I | Design §3; see [§6.4](#64-closed-csrdense-representation-contract) |
| 4 | Freeze algorithm O(V+A) | T | `snapshot_graphs/` → `csr_freeze_two_pass` (planned) |
| 5 | Offset monotonicity invariant | T | `snapshot_graphs/` → `csr_validate_offsets` (planned) |
| 6 | Query behavior via traits | T | `cross_representation/` → `csr_query_neighbors` (planned) |
| 7 | Directed/undirected count equations | T | `snapshot_graphs/` → `csr_undirected_symmetry` (planned) |
| 8 | Immutable trait conformance | I | Design §8; no runtime mutation API |
| 9 | Freeze failure leaves live graph valid | T | `snapshot_graphs/` → `csr_freeze_overflow_fail` (planned) |
| 10 | Validation checks | T | `snapshot_graphs/` → `csr_validate_malformed` (planned) |
| 11 | Complexity | I | Design §11 table |

---

## `dense_matrix_graph.md`

| Sec | Summary | Kind | Artifact |
|---|---|---|---|
| 1 | Fixed-vertex dense use case | I | Design §1 boundary |
| 2 | `node_to_slot` WBT index | T | `snapshot_graphs/` → `dense_node_to_slot` (planned) |
| 3 | Cell index `from * V + to` | I | Design §3 formula |
| 4 | Physical RAL cell sequence | I | Design §4; see [§6.4](#64-closed-csrdense-representation-contract) |
| 5 | Construction for fixed V | T | `snapshot_graphs/` → `dense_empty_for_nodes` (planned) |
| 6 | Edge updates (no auto-add vertex) | T | `snapshot_graphs/` → `dense_set_clear_edge` (planned) |
| 7 | Neighbor traversal scans row | T | `snapshot_graphs/` → `dense_neighbors_scan` (planned) |
| 8 | Query/update bounds | I | Design §8 |
| 9 | Persistence via RAL path copy | T | `snapshot_graphs/` → `dense_persistence` (planned) |
| 10 | Invariants | T | `snapshot_graphs/` → `dense_validate_invariants` (planned) |
| 11 | Validation | T | `snapshot_graphs/` → `dense_validate_invariants` (planned) |
| 12 | Complexity | I | Design §12 table |
| 13 | Representation choice rule | I | Design §13 vs live WBT |

---

## `ordered_set_trait.md`

| Sec | Summary | Kind | Artifact |
|---|---|---|---|
| 1 | Immutable multiset with unique comparator classes | I | Design §1 |
| 2 | Type + `{compare_item}` constructor | C | `error_enforcement/` → `ordered_set_constructor_record` (planned) |
| 3 | Trait `required`/`provided` split | T | `ordered_collections/` → `ordered_set_trait_dispatch` (planned) |
| 4 | `wbt_set` module surface | T | `ordered_collections/` → `wbt_set_empty_insert` (planned) |
| 5 | Insert/delete/contains semantics | T | `ordered_collections/` → `ordered_set_duplicate_insert` (planned) |
| 6 | Empty + invalid comparator failures | T | `ordered_collections/` → `ordered_set_invalid_comparator` (planned) |
| 7 | WBT invariants delegated to core | T | `wbt_core/` → `wbt_validate_invariants` (planned) |
| 8 | Persistence and memory effects | T | `ordered_collections/` → `ordered_set_persistence` (planned) |
| 9 | Complexity | I | Design §9 table |
| 10 | Example usage | T | `ordered_collections/` → `ordered_set_string_example` (planned) |
| 11 | Exclusions | O | "No hash-set, insertion-order iteration, integer trie, …" |

---

## `ordered_map_trait.md`

| Sec | Summary | Kind | Artifact |
|---|---|---|---|
| 1 | Finite key→value map | I | Design §1 |
| 2 | `{compare_key, compare_value}` constructor | C | `error_enforcement/` → `ordered_map_constructor_record` (planned) |
| 3 | Trait contract | T | `ordered_collections/` → `ordered_map_trait_dispatch` (planned) |
| 4 | `wbt_map` module surface | T | `ordered_collections/` → `wbt_map_get_insert` (planned) |
| 5 | Key identity; replace on duplicate | T | `ordered_collections/` → `ordered_map_insert_replace` (planned) |
| 6 | Get/contains/fold/find_value | T | `ordered_collections/` → `ordered_map_find_value_linear` (planned) |
| 7 | Delete by key | T | `ordered_collections/` → `ordered_map_delete_absent` (planned) |
| 8 | `from_list` / `from_sorted` bulk | T | `ordered_collections/` → `ordered_map_from_sorted` (planned) |
| 9 | Empty/failure behavior | T | `ordered_collections/` → `ordered_map_not_found_status` (planned) |
| 10 | Invariants | T | `wbt_core/` → `wbt_validate_invariants` (planned) |
| 11 | Persistence | T | `ordered_collections/` → `ordered_map_persistence` (planned) |
| 12 | Complexity | I | Design §12 table |
| 13 | Example | T | `ordered_collections/` → `ordered_map_string_example` (planned) |
| 14 | Exclusions | O | Design §14 exclusion list |

---

## `search_tree_trait.md`

| Sec | Summary | Kind | Artifact |
|---|---|---|---|
| 1 | Behavioral view over WBT set | I | Design §1 |
| 2 | Same record as `OrderedSet` | T | `terminal_structures/` → `search_tree_same_record` (planned) |
| 3 | `contains_key` + `compare_item` required | T | `terminal_structures/` → `search_tree_contains_key` (planned) |
| 4 | Updates via `wbt_set` only | T | `terminal_structures/` → `search_tree_wbt_update` (planned) |
| 5 | Comparator-class semantics | T | `terminal_structures/` → `search_tree_contains_key` (planned) |
| 6 | Distinct trait rationale | I | Design §6 documentation |
| 7 | Invariants + complexity | I | Design §7 inherits OrderedSet |
| 8 | Non-goals | O | "No range-search, predecessor/successor cursor, …" |

---

## `directed_graph_trait.md`

| Sec | Summary | Kind | Artifact |
|---|---|---|---|
| 1 | Directed graph abstract value | I | Design §1 |
| 2 | Constructor `{compare_node, compare_edge, edge_target}` | C | `error_enforcement/` → `directed_graph_constructor_record` (planned) |
| 3 | Trait contract | T | `live_graphs/` → `directed_graph_trait_dispatch` (planned) |
| 4 | `graph_wbt_directed` module surface | T | `live_graphs/` → `directed_graph_add_edge` (planned) |
| 5 | Vertex retain; auto-add endpoints on edge | T | `live_graphs/` → `directed_graph_auto_add_vertex` (planned) |
| 6 | Query semantics (`reachable`, empty neighbors) | T | `live_graphs/` → `directed_graph_reachable` (planned) |
| 7 | WBT / CSR / dense module families | T | `live_graphs/` + `snapshot_graphs/` (planned) |
| 8 | Count definitions | I | Design §8 equations |
| 9 | Invariants | T | `live_graphs/` → `directed_graph_validate` (planned) |
| 10 | Complexity | I | Design §10 table |
| 11 | Example | T | `live_graphs/` → `directed_graph_example` (planned) |
| 12 | Exclusions (no `remove_vertex` in Phase 1) | O | Design §12 exclusion list |

---

## `undirected_graph_trait.md`

| Sec | Summary | Kind | Artifact |
|---|---|---|---|
| 1 | Undirected abstract graph | I | Design §1 |
| 2 | `{to, data}` wrapper model | I | Design §2 |
| 3 | Trait contract | T | `live_graphs/` → `undirected_graph_trait_dispatch` (planned) |
| 4 | Live module surface (`add_edge/3` + `/4`) | T | `live_graphs/` → `undirected_graph_add_edge` (planned) |
| 5 | Symmetric update semantics | T | `live_graphs/` → `undirected_graph_mirror_validate` (planned) |
| 6 | Degree and neighbors | T | `live_graphs/` → `undirected_graph_degree` (planned) |
| 7 | Edge fold | T | `live_graphs/` → `undirected_graph_fold_neighbors` (planned) |
| 8 | `connected/3` | T | `live_graphs/` → `undirected_graph_connected` (planned) |
| 9 | Counts (self-loop double count) | T | `live_graphs/` → `undirected_graph_self_loop_degree` (planned) |
| 10 | CSR/dense query backends | T | `cross_representation/` (planned) |
| 11 | Invariants | T | `live_graphs/` → `undirected_graph_validate` (planned) |
| 12 | Complexity | I | Design §12 table |
| 13 | Example | T | `live_graphs/` → `undirected_graph_example` (planned) |
| 14 | Exclusions | O | Design §14 exclusion list |

---

## `weighted_graph_trait.md`

| Sec | Summary | Kind | Artifact |
|---|---|---|---|
| 1 | Orthogonal capability trait | I | Design §1 |
| 2 | `WeightedGraph[EdgeData, Weight, mem]` | C | `error_enforcement/` → `weighted_graph_type_witness` (planned) |
| 3 | Constructor record | C | `error_enforcement/` → `weighted_graph_constructor_record` (planned) |
| 4 | Trait contract (`weight_of`, weighted neighbors) | T | `live_graphs/` → `weighted_graph_weight_of` (planned) |
| 5 | Edge identity separate from weight | I | Design §5 |
| 6 | Directed update semantics | T | `live_graphs/` → `weighted_directed_add_edge` (planned) |
| 7 | Undirected wrapper model | T | `live_graphs/` → `weighted_undirected_neighbors` (planned) |
| 8 | Query semantics | T | `live_graphs/` → `weighted_graph_weighted_neighbors` (planned) |
| 9 | CSR/dense forms | T | `cross_representation/` → `weighted_csr_conformance` (planned) |
| 10 | Weight validity in algorithms not graph | I | Design §10 |
| 11 | Invariants | T | `live_graphs/` → `weighted_graph_validate` (planned) |
| 12 | Complexity | I | Design §12 table |
| 13 | Example | T | `live_graphs/` → `weighted_graph_example` (planned) |
| 14 | Exclusions | O | Design §14 exclusion list |

---

## `heap_trait.md`

| Sec | Summary | Kind | Artifact |
|---|---|---|---|
| 1 | Immutable multiset; min/max modules | I | Design §1 |
| 2 | `{compare_item}` constructor | C | `error_enforcement/` → `heap_constructor_record` (planned) |
| 3 | Trait contract | T | `ordered_collections/` → `heap_trait_dispatch` (planned) |
| 4 | `brodal_okasaki_min/max` surface | T | `ordered_collections/` → `heap_push_pop_meld` (planned) |
| 5 | Orientation behavior | T | `brodal_okasaki_core/` → `bo_max_orientation_adapter` (planned) |
| 6 | push/pop/meld/singleton | T | `ordered_collections/` → `heap_pop_field_order` (planned) |
| 7 | Persistence and arena | T | `ordered_collections/` → `heap_persistence` (planned) |
| 8 | Empty peek/pop status | T | `ordered_collections/` → `heap_empty_peek_not_found` (planned) |
| 9 | Invariants | T | `brodal_okasaki_core/` → `bo_validate_invariants` (planned) |
| 10 | Complexity | I | Design §10 table |
| 11 | Example | T | `ordered_collections/` → `heap_int64_example` (planned) |
| 12 | Exclusions | O | Design §12 exclusion list |

---

## `priority_queue_trait.md`

| Sec | Summary | Kind | Artifact |
|---|---|---|---|
| 1 | Lexicographic (priority, value) multiset | I | Design §1 |
| 2 | Separate comparators constructor | C | `error_enforcement/` → `priority_queue_constructor_record` (planned) |
| 3 | Trait contract | T | `terminal_structures/` → `priority_queue_trait_dispatch` (planned) |
| 4 | Priority module surface | T | `terminal_structures/` → `priority_queue_push_pop` (planned) |
| 5 | Duplicate/tie behavior (not FIFO) | T | `terminal_structures/` → `priority_queue_equal_priority_value` (planned) |
| 6 | Core heap operations on entries | T | `brodal_okasaki_core/` → `bo_push_pop_meld` (planned) |
| 7 | No decrease-key / delete-entry | O | "Neither arbitrary-entry deletion nor decrease-key" |
| 8 | `from_entries` bulk | T | `terminal_structures/` → `priority_queue_from_entries` (planned) |
| 9 | Empty/failure | T | `terminal_structures/` → `priority_queue_empty_pop` (planned) |
| 10 | Persistence | T | `terminal_structures/` → `priority_queue_persistence` (planned) |
| 11 | Invariants | T | `terminal_structures/` → `priority_queue_validate` (planned) |
| 12 | Complexity | I | Design §12 table |
| 13 | Example | T | `terminal_structures/` → `priority_queue_example` (planned) |
| 14 | Explicit exclusion | O | Design §14 exclusion list |

---

## `tree_trait.md`

| Sec | Summary | Kind | Artifact |
|---|---|---|---|
| 1 | Rose tree abstract value | I | Design §1 |
| 2 | `{compare_item}` + `with_root/2` | C | `error_enforcement/` → `tree_constructor_record` (planned) |
| 3 | Path model (root + child indices) | I | Design §3 |
| 4 | Node + RAL child slots | T | `terminal_structures/` → `tree_node_ral_shape` (planned) |
| 5 | Trait contract | T | `terminal_structures/` → `tree_trait_dispatch` (planned) |
| 6 | `tree_rose` module surface | T | `terminal_structures/` → `tree_with_root_add_child` (planned) |
| 7 | Path lookup `get/2` | T | `terminal_structures/` → `tree_get_path` (planned) |
| 8 | Add child path cost | T | `terminal_structures/` → `tree_add_child_persistence` (planned) |
| 9 | Remove child (tombstone/stable slot) | T | `terminal_structures/` → `tree_remove_child_slot` (planned) |
| 10 | Replace + preorder fold | T | `terminal_structures/` → `tree_fold_preorder` (planned) |
| 11 | No compaction / slot reuse | O | "Child slots are never compacted, reused, or renumbered" |
| 12 | Empty/failure cases | T | `terminal_structures/` → `tree_invalid_path` (planned) |
| 13 | Invariants | T | `terminal_structures/` → `tree_validate` (planned) |
| 14 | Validation | T | `terminal_structures/` → `tree_validate` (planned) |
| 15 | Complexity (path-dependent) | I | Design §15; not whole-tree O(log n) |
| 16 | Example | T | `terminal_structures/` → `tree_string_example` (planned) |
| 17 | Exclusions | O | Design §17 exclusion list |

---

## `binary_tree_trait.md`

| Sec | Summary | Kind | Artifact |
|---|---|---|---|
| 1 | Possibly empty fixed-role binary-tree abstract value | I | Design §1 |
| 2 | `BinaryTree[...]` + exact empty constructor record `{}` | C | `compiler_substrate/` → `binary_tree_empty_constructor_record`; compile-fail unwitnessed/extra-field trials (planned §7.10) |
| 3 | No named node/path/frame/zipper surface types | C | `binary_tree/` → `binary_tree_inline_surface_compile` (planned) |
| 4 | Private owning record + recursive node | T | `binary_tree/` → `binary_tree_runtime_shape` (planned) |
| 5 | `List[:left \| :right, SpaceType]` paths | T | `binary_tree/` → `binary_tree_get_path` (planned) |
| 6 | Trait required/provided contract | T | `binary_tree/` → `binary_tree_trait_dispatch` (planned) |
| 7 | `tree_binary` module surface | T | `binary_tree/` → `binary_tree_module_surface` (planned) |
| 8 | Empty/root/direct-child behavior | T | `binary_tree/` → `binary_tree_empty_root_children` (planned) |
| 9 | Node construction and graft compatibility | T | `binary_tree/` → `binary_tree_node_graft` (planned) |
| 10 | Item replacement | T | `binary_tree/` → `binary_tree_replace_item` (planned) |
| 11 | Left/right replacement and clearing | T | `binary_tree/` → `binary_tree_replace_children` (planned) |
| 12 | Fold orders and shape-preserving maps | T | `binary_tree/` → `binary_tree_fold_map` (planned) |
| 13 | Inline zipper operations | T | `binary_tree/` → `binary_tree_zipper_public_surface` (planned) |
| 14 | Result/failure conventions | T | `binary_tree/` → `binary_tree_failure_matrix` (planned) |
| 15 | Invariants and validation | T | `binary_tree/` → `binary_tree_validate` (planned) |
| 16 | Persistence and memory effects | T | `binary_tree/` → `binary_tree_persistence` (planned) |
| 17 | Complexity | I | Design §17; operation counters in `binary_tree_complexity_observations` (planned) |
| 18 | Example | T | `binary_tree/` → `binary_tree_string_example` (planned) |
| 19 | Exclusions; AST migration is downstream | O | Design §19 exclusion list; bootstrap plan Phase 7 |

---

## 6.4 Closed CSR/dense representation contract

**Normative contract:** [`csr_dense_representation_contract.md`](csr_dense_representation_contract.md)  
Source: implementation plan §6.4; designs `csr_graph_snapshot.md`, `dense_matrix_graph.md`, `common_contract.md` §1

| ID | Requirement | Kind | Artifact |
|---|---|---|---|
| CSR-D1 | Compiler-version-private inline layouts (generated modules only) | I | [`csr_dense_representation_contract.md`](csr_dense_representation_contract.md) §CSR-D1; compile-fail `error_enforcement/graph_layout_not_in_source` (planned) |
| CSR-D2 | Public `NodeIdType` IDs ≠ internal `int64` dense-slot domain | T | [`csr_dense_representation_contract.md`](csr_dense_representation_contract.md) §CSR-D2; `snapshot_graphs/csr_node_id_not_slot`, `dense_node_id_not_slot` (planned) |
| CSR-D3 | Runtime-sized internal extents not in public graph type params | T | [`csr_dense_representation_contract.md`](csr_dense_representation_contract.md) §CSR-D3; `snapshot_graphs/csr_runtime_extents`, `dense_v_squared_overflow` (planned) |
| CSR-D4 | Parallel CSR neighbor + edge-data buffers (attributed/weighted) | T | [`csr_dense_representation_contract.md`](csr_dense_representation_contract.md) §CSR-D4; `snapshot_graphs/csr_weighted_parallel_buffers` (planned) |
| CSR-D5 | Dense unweighted: one boolean cell sequence | T | [`csr_dense_representation_contract.md`](csr_dense_representation_contract.md) §CSR-D5; `snapshot_graphs/dense_unweighted_boolean_cells` (planned) |
| CSR-D6 | Dense attributed/weighted: `:none \| (:some, EdgeDataType)` cells | T | [`csr_dense_representation_contract.md`](csr_dense_representation_contract.md) §CSR-D6; `snapshot_graphs/dense_weighted_tagged_cells` (planned) |
| CSR-D7 | Distinct WBT, CSR, dense concrete generated types (+ specializations) | T | [`csr_dense_representation_contract.md`](csr_dense_representation_contract.md) §CSR-D7; `cross_representation/graph_distinct_concrete_types`, `wbt_csr_dense_trait_agree` (planned) |

---

## Trial leaf index

| Leaf directory | Primary design sources |
|---|---|
| `compiler_substrate/` | `common_contract` §3, §5, §7–§9; impl plan Layer 1 |
| `wbt_core/` | `weight_balanced_tree.md` |
| `skew_ral_core/` | `skew_binary_random_access_list.md` |
| `brodal_okasaki_core/` | `brodal_okasaki_queue.md` |
| `binary_tree_core/` | `persistent_binary_tree.md` |
| `binary_tree/` | `binary_tree_trait.md` |
| `ordered_collections/` | `ordered_set_trait`, `ordered_map_trait`, `heap_trait` |
| `live_graphs/` | `live_wbt_graph`, `directed_graph_trait`, `undirected_graph_trait`, `weighted_graph_trait` |
| `terminal_structures/` | `search_tree_trait`, `priority_queue_trait`, `tree_trait` |
| `snapshot_graphs/` | `csr_graph_snapshot`, `dense_matrix_graph` |
| `error_enforcement/` | Constructor/type/comparator compile failures (`C` rows) |
| `cross_representation/` | CSR/dense vs WBT conformance through public traits |

---

## §6.3 acceptance checklist

- [x] Every numbered section in `data_structure_designs/*.md` has a ledger row  
- [x] Algorithm map locked decisions (1–16) recorded
- [x] Closed CSR/dense contract (§6.4) recorded in ledger  
- [x] Each row uses exactly one coverage kind (`T`, `C`, `I`, or `O`)  
- [x] Trial leaf assignment for all `T` and `C` rows  

**Next step:** Layer 1 §§7.1–§7.9 and WBT §§8A.1–§8A.7 are re-verified (2026-07-10) — proceed to §8A.8 `from_sorted`. Skew RAL and Brodal–Okasaki may proceed in parallel on Layer 2. BinaryTree requires §7.10 before §8D. §7.2 meld before-allocation rejection — Layer 2 §8C exit gate, Layer 3 §9C re-verification.
