# compile_defects_trait_placeholder_return_addition

One valid Silica program that today's type checker rejects (see `defect_trait_placeholder_return.silica`
and the `compile_defects_addition` README for why a compile-time defect gets a directory of its own).
`ItemBag.silica` is the trait, `BagStore.silica` the generic reader its implementation forwards to.

- `defect_trait_placeholder_return`: inside a function with a trait-typed parameter, a trait method
  whose result mentions a placeholder (`List[ValueType, ...]`, or a bare `ItemType`) rejects the
  receiver (E2003). It blocks algorithms written once over `DirectedGraph@neighbors`,
  `UndirectedGraph@neighbors`, `WeightedGraph@weighted_neighbors` / `edge_weight`, and provided
  trait bodies that would call them. Expected: `7`, exit 0.

Move the trial to `ordered_data_structures/graph_collections` once the checker accepts it.
