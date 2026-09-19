# compile_defects_tagged_optional_type_addition

One valid Silica program that today's parser rejects (see `defect_tagged_optional_type.silica` and the
`compile_defects_addition` README for why a compile-time defect gets a directory of its own).

- `defect_tagged_optional_type`: an inline sum type with a tagged-tuple variant,
  `:none | (:some, int64)`, is E1040 in every type position. The dense matrix graph therefore stores
  attributed cells as an optional reference (`:none` absent, a reference to the stored value present)
  instead of CSR-D6's `:none | (:some, EdgeDataType)`. Expected: `0`, `7`, exit 0.

Move the trial to `tuples_addition` once the parser accepts it.
