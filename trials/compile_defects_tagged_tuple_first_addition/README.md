# compile_defects_tagged_tuple_first_addition

One valid Silica program that today's compiler rejects (see `defect_tagged_tuple_first.silica` and
the `compile_defects_addition` README for why a compile-time defect gets a directory of its own).

- `defect_tagged_tuple_first`: an inline sum type spelled tuple first, `(:found, uint32) | :none`,
  in a parameter, a return type and a let annotation. Topic suite: `case_addition` (next to
  `tagged_optional_type`, the atom-first spelling).
