# compile_defects_float_record_field_binding_addition

One valid Silica program whose assembly today's compiler emits wrongly (see
`defect_float_record_field_binding.silica` and the `compile_defects_addition` README for why a
defect of this kind gets a directory of its own).

- `defect_float_record_field_binding`: a float64 field bound in a record case pattern
  (`{ x: a, y: b }`) is loaded into an integer register and handed to the float path as
  `FMOV S2, X1`, which does not assemble. Found 2026-10-02 while writing the record-shape trials
  (case_addition/sum_record_variants_float_and_int_fields uses wildcards for its float arm until
  this is fixed).
