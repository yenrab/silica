# compile_defects_tree_field_annotation_addition

One valid Silica program that today's type checker rejects (see
`defect_tree_field_annotation.silica`, the `compile_defects_addition` README, and
`compile_defects_ordered_set_field_annotation_addition` for the same defect against `OrderedSet`).
References the real stdlib `tree_rose.silica`/`Tree.silica` and their `skew_ral*` dependencies by
relative path rather than copying them.

- `defect_tree_field_annotation`: a record annotation with a `Tree[string, mem(normal)]`-typed
  field is rejected (E2003) by `tree_rose@add_leaf`'s result the same way `OrderedSet` is.
  Expected: `grown.added` is true, exit 1.

Move the trial to `ordered_data_structures` (wherever the fix lands) once the checker accepts a
bracket collection type nested inside a record field.
