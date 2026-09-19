# codegen_defects_addition

Trials that document open compiler defects whose failure mode is a broken **assembly or link**
step (compile-time rejections of valid programs live in `compile_defects_addition`). A suite's assemble loop stops at the first failure, so such a trial in an ordinary suite
would hide every other trial's result; here each one fails on its own. Move a trial back to the
suite that matches its topic once the defect is fixed and it assembles.

- `sd16_print_bool_in_case_arm_block` (from project-bees SD-16): `print_bool` inside a case-arm
  block loses its helper and the emitted assembly does not assemble. Expected output: `true`.
- `defect_supervisor_init_in_other_unit` was fixed on 2026-09-18 (the spawning unit no longer emits a
  start trampoline for a supervisor whose `init` it does not define) and moved to its topic suite as
  `supervisors_addition/supervisor_init_in_other_unit` (helpers in `supervisors_addition/traits/`).
