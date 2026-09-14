# Growable and Shrinkable Actor Stacks — Implementation Plan

**Scope:** chunk 1 after the first fixed point on every emitter path (see [ROADMAP.md](../../../../ROADMAP.md)).
**Authority:** [silica-specification.md](../silica-specification.md) §15.1.2.2 *Actor Stack Architecture*.

## What the specification requires

Each actor has a dedicated stack **maintained by the Silica runtime, not the OS**. Several actors may
execute on one OS thread. Stacks are "theoretically infinite": they grow on demand up to the memory the
machine has, with no hard maximum, and when a message handler returns the stack pointer resets for the
next message. `initial_stack_size` is configurable. A fixed virtual-address reserve is **not** an
acceptable implementation: a reserve is a hidden maximum.

## What the code does today (measured 2026-09-07)

- Both `pthread_create` sites pass `attr = NULL`, so an actor runs on the OS default **512 KB** thread
  stack, and nothing in the runtime ever moves `SP`.
- `_silica_rt_actor_init_region` maps a 1 GB `PROT_NONE` reserve with 8 MB committed, but that region only
  feeds `_silica_rt_region_alloc`'s bump pointer (a heap arena, not frames). It has no bounds check and no
  growth: the fault handler's grow path accepts only `SIGSEGV`, while macOS delivers a `PROT_NONE` touch
  as `SIGBUS`, so past 8 MB the process exits 70.
- With no current actor (the compiler's own `main`), `region_alloc` falls back to `malloc` and never
  frees; that is the multi-GB compile RSS.

## Agreed design: segmented, runtime-managed stacks

The stack is a chain of segments allocated from the machine the same way `L_region_grow` allocates
arena blocks.

1. **Probe in every prologue.** Before `STP X29, X30, [SP, #-16]!` the emitter writes
   `SUB X16, SP, #D` / `STR XZR, [X16]`, where `D` is the frame size plus slack. Frames too large for the
   guard call `_silica_rt_stack_ensure` instead of probing.
2. **Guard page per segment.** A 1 MB `PROT_NONE` guard sits under each segment; the probe faults there.
3. **Growth in the fault handler.** The handler (accepting both `SIGSEGV` and `SIGBUS`) allocates a new
   segment sized from the probe distance, redirects `uc.sp` into it, stashes the real link register in
   the segment header, and sets `uc.x30 = _silica_rt_stack_shrink`.
4. **Shrink on return.** `_silica_rt_stack_shrink` restores `SP` to the previous segment and branches to
   the stashed link register, preserving `X0`, `X1`, and `X8` (scalar, pair, and indirect results). A
   segment is released when control leaves it.
5. **Foreign calls.** Every Fifi call already goes through a guarded entry; `guarded_enter` demands
   contiguous headroom so C code never runs on a segment boundary.
6. **Arena.** The per-actor arena chains blocks the same way and never relies on fault-driven growth.

## Insertion points

| Piece | Where |
| --- | --- |
| Prologue text | `emitter/<target>/control/control.silica`, `function_prologue` |
| Frame size for the probe distance | `emitter/<target>/emitter_core.silica`, `frame_spill_bytes(fn_val)` |
| Thread entry and actor control block | runtime `_actor_thread_main`; the ACB is 384 bytes (`mov w1, #384` at the two `calloc` sites) and grows by the segment fields |
| Fault handler and shrink trampoline | runtime shim (`silica_rt_shim.s`, `runtime_asm/linux_aarch64/silica_rt_shim.s`; the ESP32-S3 port table for Xtensa) |

## Consequences and gate

- Every function prologue changes, so every assembly golden in the trial tree churns once; refresh them
  from a build whose runtime output is unchanged, in one commit.
- Trials to add: deep recursion inside an actor that exceeds 512 KB and then several segments; a handler
  whose stack shrinks back after the message; a foreign call from deep inside a grown stack; the
  compiler itself running with a bounded RSS.
- The chunk closes when those trials pass under the selfhost built by the selfhost and `make fixpoint`
  passes again.

Related: [region_memory_safety_todo.md](region_memory_safety_todo.md) (arena ownership and release),
[porting_for_os_free_targets.md](../porting_for_os_free_targets.md) (board stacks and the port table).
