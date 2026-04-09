# Implementing Recursion: Explicit Frames, Preemption, and Tail Optimization

This document captures an implementation path for treating general recursion in a runtime that must respect **stack limits**, **controlled allocation**, **debuggability**, and **preemptibility** (save/resume with fair scheduling). It reflects a design where the primary story is an **explicit recursion frame stack** and a small **interpreter loop**, not universal CPS with heap closures.

## Goals

| Goal | Approach |
|------|----------|
| Bounded native stack | Flat C/stack frames; depth tracked on an explicit frame stack with a configurable maximum. |
| Predictable allocation | Arena- or pool-backed frame storage; grow rarely; reuse frames across invocations. |
| Useful backtraces | Each frame carries function identity, source span (or stable PC mapping), and live locals or pointers to them. |
| Preemption | Serialize `frames + program counter + operand/temp state`; resume later. Decrement **reductions** or **fuel** on **back-edges** (loops, tail calls) so tight recursion cannot starve the scheduler. |

## Ranked design choices (options)

1. **Primary:** Explicit frame stack + interpreter-style dispatch loop (pooled or arena-backed).
2. **Refinement:** Defunctionalized “what’s next” (tag + payload) inside frames—avoid heap-allocated continuations for the default path.
3. **Optimization:** Native tail calls where proven, **still** with fuel/checkpoints on the back-edge unless work per burst is provably bounded.
4. **Fallback:** Trampoline only on platforms without reliable tail-call codegen; avoid per-step allocation (reuse thunk or use a stack of small opcodes).
5. **Avoid as default:** Naive CPS with heap closures—uniform on paper but usually worse for allocation and opaque stacks.

## Implementation steps (suggested order)

### Phase 1 — Core representation

1. Define a **process-local frame** type: callee identity, return PC or resume point, locals slot region or pointers, optional **defunctionalized continuation** tag + payload for pending work.
2. Define a **frame stack** abstraction: push, pop, depth, `max_depth` enforcement with a clear, source-mapped error on overflow.
3. Back the stack with a **growable buffer** or **arena slab**; **reuse** freed slots where possible to limit allocator traffic.
4. Define the **saved execution state** for preemption: at minimum `frame_sp`, `pc`, and any registers/operand stack the IR uses.

### Phase 2 — Lowering and execution

5. Lower direct and mutually recursive calls into **explicit push** of a frame and **dispatch** to the target code (bytecode, threaded code, or a tight switch).
6. Lower returns into **pop**, **merge result** into the caller’s continuation (per your IR), and **branch** to the resume PC.
7. Introduce **defunctionalized** continuation states for non-tail patterns (e.g. “after recursive call, combine with `n`”) so pending work is first-order and debug-printable.

### Phase 3 — Preemption and fairness

8. Attach a **reduction or fuel counter** to the runnable unit (process/actor); decrement on each **logical step** or every **N** steps as policy dictates.
9. On **back-edges** (tail call, loop head, recursive jump), ensure fuel is checked; **yield** when exhausted, persisting the full saved state from step 4.
10. Document and test **worst-case** run length between checks so tail-heavy code cannot monopolize the scheduler.

### Phase 4 — Debugging and observability

11. Maintain a **stable mapping** from PC (or IR node) to source location for each active frame.
12. Implement **stack trace** formatting from the explicit frame list (filter internal trampoline frames if any exist).
13. Optional: **dev-only** heavier tracing (frame push/pop logging) gated behind flags.

### Phase 5 — Optimizations (optional)

14. Identify **provably tail-recursive** sites; optionally emit **native tail calls** (`musttail`-style) **without** removing fuel checks on the equivalent back-edge unless you prove bounded work per invocation burst.
15. Profile hot paths; consider **special cases** (e.g. known shallow recursion) that stay on a lighter path without changing semantics.

### Phase 6 — Fallbacks

16. If tail-call codegen is unavailable on a target, use a **trampoline** or keep the frame VM; **never** rely on unbounded native recursion for general functions.

## Out of scope for the default path

- Universal **CPS with heap closures** as the main user-visible execution model.
- Relying solely on **platform TCO** without explicit depth limits and preemption checkpoints.

## Relationship to “all recursion as tail recursion”

Source-level tail recursion is a **special case** of a machine where every transition is a **tail jump** in the **lowered** loop. General recursion is expressed as **explicit frames** plus **defunctionalized** pending work, not as a requirement that every Silica function be written with accumulators.

## References

- Related runtime containment and process semantics: `beam_like_crash_containment_design_notes.md` in this directory.
