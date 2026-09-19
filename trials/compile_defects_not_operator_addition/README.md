# compile_defects_not_operator_addition

One valid Silica program that today's compiler rejects (see `defect_not_operator.silica` and the
`compile_defects_addition` README for why a compile-time defect gets a directory of its own).

- `defect_not_operator`: the unary `not` operator (spec §2.2 keyword list, §3.3.3 `not p` /
  `not (p and q)`, §3.3.5 guard use) is not implemented as a general prefix operator outside
  guard clauses. A plain `<-` binding of `not true` fails with E2002 "undefined identifier: not"
  (the type checker falls through to treating the keyword as an unresolved identifier); the
  header comment also records two further shapes tried (`not` as a call argument: E1069; `not`
  as a case scrutinee: E2005). Expected: `false`, exit 0.

Move the trial to `boolean_addition` (logical operators; `negation_addition` covers arithmetic
unary minus, not this) once the checker accepts `not` outside guard clauses.
