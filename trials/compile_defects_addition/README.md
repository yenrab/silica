# compile_defects_addition

Valid Silica programs that today's compiler rejects. Each one fails its directory's compile step, and one
rejected unit aborts a suite's batch compile, so every such trial lives alone in a sibling directory
`compile_defects_<topic>_addition/` (Makefile, `compare_scout_multiset.sh` and
`run_integration_exit_after_marker.py` copied from an existing sibling, a README naming the topic suite, the
trial `.silica` with a header documenting expected vs observed, and a hand-derived `.scout`). Move a trial to the
suite that matches its topic once the compiler accepts it and its behaviour matches the golden, then delete its
directory. This directory holds only this README (it has no Makefile, so it is not a suite).

Open:

- From book-verification (2026-09-19, open): `compile_defects_not_operator_addition` (unary `not`
  not implemented as a general prefix operator outside guard clauses, E2002/E2005/E1069 depending
  on position), `compile_defects_list_tuple_literal_addition` (a list literal of tuple literals is
  E1040 at the second element), `compile_defects_ordered_set_field_annotation_addition` and
  `compile_defects_ordered_set_field_projection_addition` (a bracket collection type nested inside
  a record field, or read back through a `.field` projection, is not recognized as its own
  concrete representation, E2003), `compile_defects_ordered_map_field_annotation_addition` /
  `compile_defects_tree_field_annotation_addition` / `compile_defects_graph_field_annotation_addition`
  (the same defect against `OrderedMap`/`Tree`/`UndirectedGraph`),
  `compile_defects_priority_queue_supervisor_addition` (a supervisor child spec's `initial_state`
  typed `PriorityQueue[...]` is E2003 "type mismatch: children", a third shape of the same
  bracket-representation family), `compile_defects_binary_tree_bracket_addition`
  (`BinaryTree[...]` is missing from `type_checker_collections.silica`'s
  `representation_id_from_module`, so it is E1040 wherever used), and
  `compile_defects_lifetime_parameter_name_addition` (a function's lifetime parameter name is
  compared by source spelling instead of treated as a polymorphic binder, E2003; spec §12.1.4).

- From the graph work (open): `compile_defects_trait_nested_placeholder_addition` (an impl cannot bind a
  placeholder nested inside `List[...]` to a record, E2092), `compile_defects_graph_bracket_receiver_addition` (a
  `DirectedGraph[...]`/`WeightedGraph[...]` value is rejected by trait calls, E2003),
  `compile_defects_bracket_receiver_arguments_addition` (an `UndirectedGraph[...]` value with two fitting impls is
  rejected by a method with a placeholder argument, E2003), `compile_defects_trait_placeholder_return_addition` (a
  placeholder-result trait method cannot be called on a trait-typed parameter, E2003),
  `compile_defects_trait_placeholder_result_dispatch_addition` (a placeholder-result trait method is resolved
  against the last implementation, E2003) and `compile_defects_tagged_optional_type_addition`
  (`:none | (:some, T)` is E1040 in a type position).

Resolved 2026-09-19: the bare `ref?(L, normal, rec)` binding of SD-5 is invalid per spec §4.2.2 and is now the error trial
`error_enforcement_addition/rec_bare_ref_opt_binding_annotation` (E2010), by Lee's decision.

Fixed 2026-09-18 (front end: parser and type checker) and moved:

- `defect_tuple_pattern_named_binding` (`(:ok, n: int64)` was E2002) ->
  `supervisors_addition/emitter_defect_tuple_pattern_named_binding`: now accepted, but the emitter never matches a
  tuple pattern with atom or named elements (open emitter defect; also isolated as
  `case_addition/emitter_defect_tuple_pattern_atom_element`). No `.ascomp` until the emitter is fixed.
- `compile_defects_priority_type_addition` (PriorityType never substituted in composite types, E2003) ->
  `ordered_data_structures/heap_collections/pq_priority_type_composite_result` (+ `lib/StubPriorityQueue.silica`).
- `compile_defects_item_type_addition` (ItemType resolved to the first PriorityQueue bracket parameter, E2001) ->
  `ordered_data_structures/heap_collections/pq_item_type_is_value`.
- `compile_defects_ref_opt_spelled_type_addition` (SD-4, E2003) -> `memory_region_addition/ref_opt_spelled_type`,
  plus `memory_region_addition/ref_opt_spelled_field_store` (the store half of SD-4) and
  `memory_region_addition/ref_opt_binding_annotation` (the spelled SD-5 binding).
- `compile_defects_float_literal_in_comparison_addition` (SD-13, E2003) ->
  `float64_addition/float_literal_in_comparison`.
- `compile_defects_impl_item_type_addition` (trait impl returning an ItemType-valued call, E2003; direct tuple
  destructure of a generic call, E2005) -> `generic_modules_addition/impl_item_type_return`
  (+ `StubTree.silica`, `stub_tree_core.silica`).
- `compile_defects_search_tree_parameter_addition` (SearchTree/OrderedSet bracket parameter or binding rejected a
  concrete record from an update, E2003) -> `ordered_data_structures/search_tree_collections` (moved by the
  coordinator).
