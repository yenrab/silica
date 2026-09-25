# compile_defects_effect_propagation_addition

One valid Silica program that today's effect checker rejects (see
`defect_effect_propagation.silica` and the `compile_defects_addition` README for why a compile-time
defect gets a directory of its own).

- `defect_effect_propagation`: a sequence that calls a function whose own sequence declares
  `proc[mem(normal)]` must declare that effect (silica-specification §9.3.1 rule 4, "Effect
  Propagation", whose example is this program). The checker reports the declaration as unused
  (E3010) instead: effects never propagate through a user call, for `mem(...)` or for `device_io`,
  within a module or across modules. Expected: prints 1, exit 0.

Move the trial to `effects_addition` once the checker propagates a callee's effects.
