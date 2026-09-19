# compile_defects_lifetime_parameter_name_addition

One valid Silica program that today's type checker rejects (see
`defect_lifetime_parameter_name.silica` and the `compile_defects_addition` README for why a
compile-time defect gets a directory of its own).

- `defect_lifetime_parameter_name`: a function parameter typed `ref(L, normal, int64)` rejects
  (E2003 "type mismatch") an argument whose region was created under the lifetime name `L1`;
  renaming the caller's lifetime to `L` (matching the callee's spelling) makes the identical
  program compile and run. Spec §12.1.4 says a signature's lifetime name is a polymorphic
  parameter ("functions may take `L: lifetime` as a parameter for polymorphism"), so the checker
  should accept any lifetime here, matched structurally, not by source spelling. Expected: `42`.

Move the trial to `memory_region_addition` once the checker treats a signature's lifetime name as
polymorphic rather than requiring the caller's lifetime to share its spelling.
