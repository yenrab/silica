# compile_defects_float_literal_pattern_addition

One valid Silica program that today's compiler rejects (see `defect_float_literal_pattern.silica`
and the `compile_defects_addition` README for why a compile-time defect gets a directory of its
own).

- `defect_float_literal_pattern`: a float literal is a `literal_pattern` (spec §3.5
  `pattern ::= literal_pattern`, `literal_pattern ::= literal`), but the type checker knows only
  integer literal patterns: `type_checker_tuple_decompose_helpers@case_parse_pattern` classifies a
  numeric literal with `case_string_is_signed_int_literal`, so `1.5` in a pattern — on its own or as
  a tuple element — is E2005 "invalid or unsupported case pattern". Found 2026-09-19 while
  implementing the emitter half: `term_case_patterns@emit_tuple_case_pattern_asm` already compares a
  float element (loading the literal from the float literal pool, which now collects pattern
  literals), so only the front end is missing. A top-level float scrutinee (`case x of { 1.5 -> … }`)
  needs emitter work as well — the case scrutinee is loaded into a general register.

Move the trial to `case_addition/tuple_pattern_float_element` once the checker accepts it and its
behaviour matches the golden, then delete this directory.
