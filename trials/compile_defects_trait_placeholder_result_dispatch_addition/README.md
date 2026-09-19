# compile_defects_trait_placeholder_result_dispatch_addition

One valid Silica program that today's type checker rejects (see
`defect_trait_placeholder_result_dispatch.silica` and the `compile_defects_addition` README for why a
compile-time defect gets a directory of its own). `Items.silica` is a trait with two implementations,
`ItemStore.silica` the generic readers they forward to.

- `defect_trait_placeholder_result_dispatch`: a trait method whose result mentions a placeholder is
  resolved against the last implementation in the trait unit, not the argument's; the call on the
  first implementation's record is rejected (E2003). With the graph records the call compiles and
  runs the wrong implementation (`ordered_data_structures/graph_collections/
  emitter_defect_trait_placeholder_result_dispatch`), which is why only the last-listed graph
  representation may use `neighbors`, `fold_neighbors`, `edge_target`, `weighted_neighbors`,
  `fold_weighted_neighbors`, `weight_of` and `edge_weight` through the traits today.
  Expected: `7`, exit 0.

Move the trial to `traits_addition` once the checker accepts it.
