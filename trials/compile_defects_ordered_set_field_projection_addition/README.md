# compile_defects_ordered_set_field_projection_addition

One valid Silica program that today's type checker rejects (see
`defect_ordered_set_field_projection.silica`, the `compile_defects_addition` README, and the
sibling `compile_defects_ordered_set_field_annotation_addition` for the closely related defect
this is a second shape of). References the real stdlib `wbt_set.silica`/`OrderedSet.silica` by
relative path rather than copying them.

- `defect_ordered_set_field_projection`: binding `wbt_set@insert(...).set` (a `.field`
  projection) to an `OrderedSet[string, mem(normal)]`-typed name is rejected (E2003) the same way
  as the sibling trial's full record annotation; the checker's own error message additionally
  shows an unsubstituted `ItemType` placeholder on its "expected" side. Expected: exit 0.

Move the trial alongside `compile_defects_ordered_set_field_annotation_addition`'s destination
once fixed.
