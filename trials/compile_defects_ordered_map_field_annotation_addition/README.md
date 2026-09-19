# compile_defects_ordered_map_field_annotation_addition

One valid Silica program that today's type checker rejects (see
`defect_ordered_map_field_annotation.silica`, the `compile_defects_addition` README, and
`compile_defects_ordered_set_field_annotation_addition` for the same defect against `OrderedSet`).
References the real stdlib `wbt_map.silica`/`OrderedMap.silica` by relative path rather than
copying them.

- `defect_ordered_map_field_annotation`: a record annotation with an
  `OrderedMap[string, int64, mem(normal)]`-typed field is rejected (E2003) by `wbt_map@insert`'s
  result the same way `OrderedSet` is. Expected: `r.inserted` is true, exit 1.

Move the trial to `ordered_data_structures` (wherever the fix lands) once the checker accepts a
bracket collection type nested inside a record field.
