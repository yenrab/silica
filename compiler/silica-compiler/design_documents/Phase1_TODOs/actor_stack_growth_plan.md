# Growable and Shrinkable Actor Stacks — Implementation Plan

**Scope:** chunk 1 after the first fixed point on every emitter path (see [ROADMAP.md](../../../../ROADMAP.md)).
**Authority:** [silica-specification.md](../silica-specification.md) §15.1.2.2 *Actor Stack Architecture*, as
revised on 2026-09-17, together with §15.4.11.2 (failure reasons) and §22.4 (`get_actor_memory_usage`).
**Mechanics:** [actor_growable_stack_design.md](../actor_growable_stack_design.md) §2.1, §2.2, §4.2, §4.4 and §7,
revised the same day. The specification wins where the two differ.

## What the specification requires (2026-09-17 revision)

- One contiguous, inaccessible reservation per actor, made at spawn. Default size: the machine's memory
  plus swap, read once at runtime start, page-rounded, just under the total where the host's overcommit
  heuristic refuses the exact amount. `stack_policy(reserve, release)` at spawn lowers it.
- Nothing committed at spawn. Growth in the fault handler: one platform page, doubling per fault to 1 MB,
  then 1 MB per fault. The platform's page size, not a fixed one.
- Two mappings per actor, the committed run and the remainder; no guard mapping. A fault inside the
  reservation is growth, tested before the guarded-foreign-call recovery.
- Release at the message boundary, in the chunks it grew by, down to what the actor's release algorithm
  retains. Five named algorithms, `:release_on_return`, `:keep_last_message`, `:track_recent_peak`,
  `:release_when_idle` and `:keep_high_water` (spec §15.1.2.2; design §4.2.1), and `stack_policy` is a
  required argument of every spawn form, so every actor names one.
- No per-actor maximum, no runtime cap, no `:oom`. Exhausting a lowered reservation is
  `(:explicit, :stack_exhausted)`; a reservation that cannot be made is `(:explicit, :stack_reserve_failed)`
  on the spawning actor.
- `main` is not an actor and runs on the platform's ordinary stack.
- `get_actor_memory_usage` returns reserved, committed, retained and high-water bytes.

The segmented design this plan described until 2026-09-17 (a probe under each frame, a guard page under
each segment, frames hopping between segments) is withdrawn. It removes the address-space limit but
pointers into frames cannot cross segments, every call pays a limit check, and the languages that tried
it abandoned it. The reservation is not a hidden maximum in the sense the earlier text meant: by default
it exceeds what the machine can commit, and a program only sees a smaller one when it asked for one.

## What the code does today (measured 2026-09-17)

- `_silica_rt_actor_init_region` maps a 1 GB `PROT_NONE` reservation per actor with 8 MB committed and
  grows it a page at a time in the fault handler (`sbase`/`ssize` in the fault banner). It never shrinks.
- The fault handler tries the guarded-foreign-call recovery **before** the stack-growth test; that order
  is the reverse of the specification and must be swapped.
- On Linux the emitted `main` is a trampoline onto a private 256 MB stack, and on macOS the compiler is
  linked with a 256 MB main stack (`-Wl,-stack_size`). Both exist only because the compiler runs its
  whole pipeline on the main thread. Neither is allowed by the revised §15.1.2.2.
- The Pi 5 kernel and macOS use 16 KB pages; the runtime's growth code assumes 4 KB in places.
- Debian sets `vm.max_map_count` to about one million on the Pi image and 65,530 on stock kernels; two
  mappings per actor means the limit is reached at half that many actors.

## Work

1. **Reservation.** Replace the 1 GB constant with the default computed at runtime start (physical memory
   plus swap; `sysconf`/`sysctl` on the hosts) and the per-spawn override from `stack_policy`. Reserve
   with no access rights. Fail the spawn with `:stack_reserve_failed` when the reservation cannot be made.
2. **Growth.** Commit nothing at spawn. Chunked growth in the fault handler, doubling from one page to
   1 MB, using the platform page size read at start. Move the reservation-membership test ahead of the
   guarded-foreign-call recovery. Exhaustion of a lowered reservation fails the actor with
   `:stack_exhausted`; running out of memory does nothing of ours.
3. **Release.** At message return, release down to the retained amount in the same chunk sizes
   (`MADV_FREE` / `MADV_FREE_REUSABLE`); an idle sweep releases the rest after the grace period. The
   five algorithms and their control-block state are in design §4.2.1; `:release_when_idle` needs an idle
   sweep on the carrier threads.
4. **Spawn.** `stack_policy(reserve, release)` as the last required argument of every spawn form, before
   the optional core id: type checker, SIR, and all three emitters. Every spawn in the trials, the
   standard library and the compiler then names its policy; that is one mechanical pass over the tree.
5. **Report.** `get_actor_memory_usage` returns the four-field record.
6. **The compiler's `main`.** Make it spawn one actor holding the pipeline and `call` it, returning the
   reply as the exit code, so the compiler runs on an actor stack. Then delete the Linux `main`
   trampoline in `emitter/linux_aarch64/emitter_core.silica` and the macOS `-Wl,-stack_size` link flag,
   so `main` runs on the platform's ordinary stack on both hosts. Measure the per-invocation cost, since
   the process-per-unit loop restarts the compiler hundreds of times per build.
7. **NUMA.** Node-local commitment and lazy migration (design §4.3, §5.3) only on platforms that report
   NUMA nodes; nothing on the Mac or the Pi.

## Insertion points

| Piece | Where |
| --- | --- |
| Reservation, commit, release | `emitter/<target>/terms/prims/prims_actors_runtime_asm.silica`, `_silica_rt_actor_init_region` and the fault handler in `terms/ffi_fault_runtime_asm.silica` |
| Fault precedence | `terms/ffi_fault_runtime_asm.silica`, `_silica_rt_sync_fault_handler`: reservation test first, then `_silica_rt_ffi_guarded_fault_handler_try` |
| Actor control block | the ACB gains reserve, committed, retained, high-water, next-chunk and algorithm fields |
| `stack_policy` | type checker, SIR generator and the three emitters, following the same chain as `core_id` |
| `main` | `emitter/linux_aarch64/emitter_core.silica` (`program_main_stack_trampoline`, to be removed), `src_selfhost/Makefile` (`LDFLAGS_STACK`), `src_selfhost/main.silica` |

## Still to decide

- Operating-system prerequisites: whether `vm.max_map_count` is documented or checked by the runtime at
  start, and what targets without demand paging (ESP32-S3) do instead. Awaiting discussion.

## Gate

- Trials: deep recursion inside an actor well past 8 MB; a handler whose committed stack shrinks back
  after the message; a foreign call from deep inside a grown stack that grows further; a million idle
  actors holding no stack memory; a lowered reservation being exhausted and reported; the compiler
  running as an actor with a bounded RSS.
- The chunk closes when those trials pass under the selfhost built by the selfhost on both hosts, and
  `make fixpoint` passes again on both.

Related: [region_memory_safety_todo.md](region_memory_safety_todo.md) (arena ownership and release),
[porting_for_os_free_targets.md](../porting_for_os_free_targets.md) (board stacks and the port table).
