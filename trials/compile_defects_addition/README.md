# compile_defects_addition

Valid Silica programs that today's compiler rejects. Each one fails its directory's compile step, and one
rejected unit aborts a suite's batch compile, so every such trial lives alone in a sibling directory
`compile_defects_<topic>_addition/` (Makefile, `compare_scout_multiset.sh` and
`run_integration_exit_after_marker.py` copied from an existing sibling, a README naming the topic suite, the
trial `.silica` with a header documenting expected vs observed, and a hand-derived `.scout`). Move a trial to the
suite that matches its topic once the compiler accepts it and its behaviour matches the golden, then delete its
directory. This directory holds only this README (it has no Makefile, so it is not a suite).

Open:

- From book-verification (2026-09-19, open): `compile_defects_priority_queue_supervisor_addition` (a supervisor
  child spec's `initial_state` typed `PriorityQueue[...]` is E2003 "type mismatch: children"); the field-annotation
  and field-projection half of the same bracket-representation family was fixed on 2026-09-19 (see below).

- From the effect checker (2026-09-19, open): `compile_defects_effect_propagation_addition` (a sequence that calls a
  function whose own sequence declares `proc[mem(normal)]` must declare that effect, spec §9.3.1 rule 4; the checker
  reports the declaration as unused, E3010, because effects never propagate through a user call. The fix was
  deliberately left out of the 2026-09-25 merge.)

- From defect batch 2 (2026-09-19, open): `compile_defects_float_literal_pattern_addition` (a float
  literal is not accepted as a case pattern -- on its own or as a tuple element -- because
  `case_parse_pattern` classifies numeric literals with `case_string_is_signed_int_literal`, E2005;
  the emitter matches a float tuple element already).

Resolved 2026-09-24: the float64 literal pool did not walk case nodes (`case x > 2.0 of` emitted `L_f64_-1`);
fixed in both float literal pools and the trial moved to `float64_addition/float_literal_case_scrutinee`.

Resolved 2026-09-19: the bare `ref?(L, normal, rec)` binding of SD-5 is invalid per spec §4.2.2 and is now the error trial
`error_enforcement_addition/rec_bare_ref_opt_binding_annotation` (E2010), by Lee's decision.

Fixed 2026-09-24 (front end: lexer, parser, type checker, SIR generator) and moved:

- `compile_defects_not_operator_addition` (`not` was not even a lexer keyword; every position was E2002,
  E2005 or E1069, guard clauses included) -> `boolean_addition/not_operator`: `not` is a keyword unary
  operator (spec §2.2.1, §3.3.1, §3.9), parsed like `bnot`, typed boolean -> boolean, lowered as the boolean
  `eq` prim against `false`, so no emitter changed.
- `compile_defects_lifetime_parameter_name_addition` (E2003 when the caller's lifetime name differed from
  the callee's) -> `memory_region_addition/lifetime_parameter_name`: ref/ref?/region/buf/atomic_ref
  arguments are compared modulo the lifetime binder (spec §12.1.4) at the identifier-argument site.
- `compile_defects_tagged_optional_type_addition` (`:none | (:some, T)` was E1040 in every type position)
  -> `case_addition/tagged_optional_type`: the parser accepts a tuple variant after `|`
  (parser_tuples@extract_atom_option_tail_from_slots), the OR-return-type validator accepts a tagged tuple, the
  type checker checks a tuple literal or tuple case pattern against the variant it builds or matches
  (type_checker_expressions_tuples@check_tuple_literal_against_sum,
  type_checker_tuple_decompose_helpers@case_tuple_variant_for_pattern), and the SIR lowering gives the tuple
  literal and the case branch that variant's type. Two lanes fixed this the same day; the 2026-09-25 merge kept
  one implementation. The emitter half (a tuple pattern with atom-literal or named elements) is
  `case_addition/tuple_pattern_atom_and_named_elements`, fixed with it, so no emitter_defect prefix.
- `compile_defects_nested_list_literal_addition` (created by the DIAGNOSTICS lane once its list-closer fix let
  `[[1], [2, 3]]` parse; the type checker then rejected the inner literal, E2001 "literal requires integer or
  float return type, got List[int64, normal]") -> `list_addition/nested_list_literal`: list spine pairs carry
  name ",", so a kind-20 node without it in the tail slot is one element for the type checker and the SIR
  lowering (its old second_len ran `length[...]` outside a sequence block and its golden miscounted).

Fixed 2026-09-19 (defect batch 2 and the parked September lanes, landed by the 2026-09-25 merge) and moved:

- `compile_defects_list_tuple_literal_addition` (a list literal of tuple literals, E1040) ->
  `list_addition/list_of_tuple_literals_length`.
- `compile_defects_ordered_set_field_annotation_addition` / `compile_defects_ordered_set_field_projection_addition`
  -> `ordered_data_structures/search_tree_collections/ordered_set_field_annotation` and
  `.../ordered_set_field_projection`; `compile_defects_ordered_map_field_annotation_addition` ->
  `ordered_data_structures/ordered_collections/ordered_map_field_annotation`;
  `compile_defects_tree_field_annotation_addition` -> `ordered_data_structures/tree_collections/tree_field_annotation`;
  `compile_defects_graph_field_annotation_addition` ->
  `ordered_data_structures/graph_collections/graph_field_annotation` (a bracket collection type nested inside a
  record field, or read back through a `.field` projection, is now recognized:
  type_checker_collections@collection_aware_types_compatible).
- `compile_defects_binary_tree_bracket_addition` (`BinaryTree[...]` missing from the representation table) ->
  `ordered_data_structures/binary_tree_core/binary_tree_bracket`.
- `compile_defects_graph_bracket_receiver_addition` -> `ordered_data_structures/graph_collections/graph_bracket_receiver`
  (+ `lib/ProbeDirected.silica`, `lib/StubGraphDirected.silica`); `compile_defects_bracket_receiver_arguments_addition`
  -> `ordered_data_structures/graph_collections/bracket_receiver_arguments` (+ `lib/PlusGraph.silica`,
  `lib/StubGraphUndirected.silica`).
- `compile_defects_trait_nested_placeholder_addition` -> `traits_addition/trait_nested_placeholder`;
  `compile_defects_trait_placeholder_return_addition` -> `traits_addition/trait_placeholder_return`;
  `compile_defects_trait_placeholder_result_dispatch_addition` -> `traits_addition/trait_placeholder_result_dispatch`
  (their trait and impl units under `traits_addition/traits/`).

Fixed 2026-09-18 (front end: parser and type checker) and moved:

- `defect_tuple_pattern_named_binding` (`(:ok, n: int64)` was E2002) ->
  `supervisors_addition/tuple_pattern_named_binding_in_behaviour`. The emitter half (every element kind
  of a tuple pattern, and a frame slot for each named element) was fixed on 2026-09-19; the isolated
  case is `case_addition/tuple_pattern_atom_and_named_elements`.
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
