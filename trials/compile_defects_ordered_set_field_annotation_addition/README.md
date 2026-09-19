# compile_defects_ordered_set_field_annotation_addition

One valid Silica program that today's type checker rejects (see
`defect_ordered_set_field_annotation.silica` and the `compile_defects_addition` README for why a
compile-time defect gets a directory of its own). References the real stdlib
`wbt_set.silica`/`OrderedSet.silica` by relative path (see `silica.config`'s recipe in the
Makefile) rather than copying them, since a hand-written stub would not carry the real checker
registration this defect depends on.

- `defect_ordered_set_field_annotation`: a record type annotation whose field is a bracket
  collection type -- `{ set: OrderedSet[string, mem(normal)], inserted: boolean }` -- is rejected
  (E2003) even though the design document's own worked example
  (`ordered_set_trait.md` §10) uses exactly this shape, and `OrderedSet[string, mem(normal)]`
  alone (not nested in a record) is accepted everywhere else. The same pattern reproduces for
  `OrderedMap`, `Tree`, and `UndirectedGraph` (see the sibling `compile_defects_ordered_map_*`,
  `compile_defects_tree_*`, `compile_defects_graph_*` directories) and for a `.field` projection
  instead of a full record annotation (`compile_defects_ordered_set_field_projection_addition`).
  Expected: `Ada` inserts into an empty set, `r.inserted` is true, exit 1.

Move the trial to `ordered_data_structures/search_tree_collections` (or wherever the fix lands)
once the checker accepts a bracket collection type nested inside a record field.
