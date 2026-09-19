# compile_defects_list_tuple_literal_addition

One valid Silica program that today's parser rejects (see `defect_list_tuple_literal.silica` and
the `compile_defects_addition` README for why a compile-time defect gets a directory of its own).

- `defect_list_tuple_literal`: a list literal whose elements are tuple literals --
  `[(3, 4), (1, 2)]` -- is rejected (E1040) at the comma after the first element, as if the list
  literal ended there. List literals of scalars and of record literals both work today. Expected:
  `length(xs) == 2`, exit 2.

Move the trial to `list_addition` or `tuples_addition` once the parser accepts a list literal of
tuple literals.
