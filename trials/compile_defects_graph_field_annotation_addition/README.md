# compile_defects_graph_field_annotation_addition

One valid Silica program that today's type checker rejects (see
`defect_graph_field_annotation.silica`, the `compile_defects_addition` README, and
`compile_defects_ordered_set_field_annotation_addition` for the same defect against `OrderedSet`).
References the real stdlib `graph_wbt_undirected.silica`/`graph_wbt_core.silica` and their
`wbt_set`/`wbt_map` dependencies by relative path rather than copying them.

- `defect_graph_field_annotation`: a record annotation with an
  `UndirectedGraph[string, int64, mem(normal)]`-typed field is rejected (E2003) by
  `graph_wbt_undirected@add_edge`'s result the same way `OrderedSet` is. Expected: `r.inserted`
  is true, exit 1.

Move the trial to `ordered_data_structures/graph_collections` (or wherever the fix lands) once
the checker accepts a bracket collection type nested inside a record field.
