# compile_defects_priority_queue_supervisor_addition

One valid Silica program that today's type checker rejects (see
`defect_priority_queue_supervisor.silica` and the `compile_defects_addition` README for why a
compile-time defect gets a directory of its own). References the real stdlib
`Supervisor.silica`, `PriorityQueue.silica`, and the `brodal_okasaki*` modules it needs by
relative path rather than copying them.

- `defect_priority_queue_supervisor`: a supervisor child spec whose `initial_state` is a
  `PriorityQueue[int64, string, mem(normal)]` is rejected (E2003 "type mismatch: children") on
  the static `children` list literal. Naming the queue first (a `let`-style binding) does not
  help, ruling out the double-cast-literal defect in `supervisors_addition` as the cause; plain
  (non-supervisor) `spawn` with the same queue as actor state works today. Expected: the worker
  triages one animal and reports it back, exit 0.

Move the trial to `supervisors_addition` once the checker accepts a bracket collection type as a
child spec's `initial_state`.
