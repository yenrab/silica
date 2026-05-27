# Actor Implementation Plan: Phase 1 TODOs and Status

**Date**: April 7, 2026  
**Last source audit**: May 26, 2026  
**Status**: Core actor runtime, registered actors, supervisors, Apple Silicon topology helpers, actor memory regions, and guarded-FFI actor fault delivery are substantially implemented. Several APIs that once appeared deferred now have type/SIR/emitter/runtime paths, but some are limited implementations rather than full semantic completion.

**References**:
- [silica-specification.md](../silica-specification.md) §15, §16, §22
- [silica-specification-additional.md](../silica-specification-additional.md) §4
- [actor_growable_stack_design.md](../actor_growable_stack_design.md)
- [cpu_topology_implementation_plan.md](cpu_topology_implementation_plan.md)
- [macos_crash_handling_for_silica.md](../macos_crash_handling_for_silica.md)

---

## Current State Summary

The public actor pipeline includes `spawn`, `spawn_dangerous`, `spawn_registered`, `spawn_dangerous_registered`, `spawn_registered_supervisor`, `call`, `call_registered`, `cast`, `cast_registered`, `cast_dangerous_registered`, `send`, and `self`. `recv` is runtime-internal and user code is rejected. `spawn_linked` is no longer part of the public language surface; the runtime still uses linked spawn internally for supervisors.

The Apple Silicon runtime uses `pthread_create` / `pthread_detach` for actors and a pthread key for current-actor TLS. The actor control block is currently 384 bytes and includes supervisor fields, mailbox queues, restart bookkeeping, active-call teardown state, and FFI arena metadata. Per-actor synchronization now uses `os_unfair_lock` plus `___ulock_wait` / `___ulock_wake` CV chunks, not embedded `pthread_mutex_t` / `pthread_cond_t`.

Actor-local memory is backed by a reserved 1GB virtual region with an initial 8MB mapping. `_silica_rt_region_alloc` bump-allocates from the current actor's region and falls back to `_malloc` outside actor context. Synchronous fault handlers are shared with guarded FFI handling: guarded FFI faults are routed through the guarded-FFI bridge; actor-region `SIGSEGV` can grow the mapping or mark the actor dead on overflow.

---

## Implemented Surface

| Area | Current source status |
|------|-----------------------|
| Actor message marker | Implemented. `expr impl ActorMessage {}` is parsed and checked for `call`, `cast`, and `send`. |
| Message type matching | Implemented for actor refs with behavior metadata tracked in the type environment. |
| `call()` reply extraction | Implemented from behavior return type `(:reply, Reply, State)`. |
| Behavior non-recursion | Implemented by AST self-call detection in declaration checking. |
| Core placement argument | Implemented for `spawn(..., core_id)` / `uint64`-like placement; lists and core sets are rejected. |
| Dangerous actors | Implemented for `spawn_dangerous`, dangerous registered actors, dangerous casts, and FFI taint/error trials. |
| Registered actors | Implemented for actor and dangerous actor registration, lookup, registered calls, and registered casts. |
| Supervisors | Implemented through `spawn_registered_supervisor`, child tables, restart flows, failure reporter hooks, and guarded dangerous-child trials. |
| CPU topology | Implemented on Apple Silicon + macOS for `get_cpu_topology`, `get_core_capabilities`, `get_efficiency_cores`, and `get_performance_cores`; see the CPU topology TODO. |
| Actor memory usage | Implemented as `get_actor_memory_usage(actor_ref) -> int64`, reading mapped region size from the actor control block. |
| Pin/unpin/priority | Implemented as Apple/macOS runtime helpers using Mach/pthread policy APIs where available. |
| `migrate_actor` / `move` | Implemented as affinity retagging helpers, not as full actor relocation or region ownership migration. |

---

## Corrected Notes From Older Plan

### `get_cpu_topology()` Is No Longer Deferred

The older plan listed `get_cpu_topology()` as deferred. Current source has type-checker return types, SIR lowering, emitter labels, runtime assembly, and CPU discovery trials for both `get_cpu_topology()` and `get_core_capabilities(core_id)`.

The Apple path is still platform-scoped: it uses macOS sysctl data and does not imply that Linux, bare-metal, or other AArch64 backends are complete.

### `move()` and `migrate_actor()` Are Limited Implementations

The older plan tied `move()` and `migrate_actor()` to full region transfer. Current source accepts and lowers both calls:

- `migrate_actor(actor_ref, target_core)` emits `_silica_rt_migrate_actor`
- `move(actor_ref, from_core, to_core)` emits `_silica_rt_move_actor`

Both runtime helpers currently set thread affinity/policy toward the target core. They do not copy actor stacks, transfer heap/region ownership between schedulers, preserve NUMA locality, or implement a full migration protocol. Treat them as placement-control helpers until a later runtime migration phase exists.

### `spawn_linked` Is Internal-Only

The older plan described `spawn_linked` as behavior-only. Current type checking rejects user calls with E2091 and directs users to `spawn_registered` or `spawn_registered_supervisor`. Internal supervisor materialization still lowers to linked runtime spawn paths.

### Link / Monitor / Demonitor Are Not Complete

The type checker has behavior-only surfaces for `link`, `monitor`, and `demonitor`, and SIR/emitter labels exist. Runtime assembly currently documents `link` as a stub returning `:ok`, `monitor` as a placeholder returning the input/opaque value, and `demonitor` as `:ok`. Full idempotent link graphs, dead-target behavior, monitor references, and `DOWN` delivery still need implementation.

### Runtime Synchronization Changed

The older plan described mutex/condition synchronization. Current runtime comments and code use `os_unfair_lock` plus ulock-based condition-variable chunks to avoid libpthread-internal lifetime problems under actor teardown and supervisor stress.

---

## Remaining TODOs

| Item | Status | Notes |
|------|--------|-------|
| Full actor migration | Open | Define and implement migration beyond affinity retagging: quiescence, in-flight message handling, actor-region ownership, and scheduler handoff. |
| Region move semantics beyond spawn shadowing | Open | Current type checker marks moved initial-state region variables after spawn. Full cross-function/cross-actor region ownership remains tied to region memory safety work. |
| `link` runtime semantics | Open | Implement idempotent link graph, dead-target detection, and exit delivery semantics. |
| `monitor` / `demonitor` runtime semantics | Open | Implement monitor refs and `DOWN` messages to the standard mailbox. |
| Stack size configuration | Open | Add a public spawn configuration path if user-selectable initial/total actor region sizes remain desired. |
| Page-location API | Open | `get_actor_page_locations` remains deferred pending map-like return types or another stable representation. |
| NUMA migration | Future platform work | Apple Silicon is UMA for this runtime path; Linux/NUMA behavior belongs to a platform-specific backend plan. |
| Non-Apple actor runtime parity | Future platform work | Apple Silicon/macOS is the implemented backend for this actor runtime path. Other platforms need separate emitter/runtime work. |

---

## Verification Coverage

Existing trials cover the main implemented surfaces:

- `trials/actors_addition`: actor spawn/call/cast/send/self, registered actor behavior, migration/move call shape, and memory usage.
- `trials/supervisors_addition`: registered supervisors, child tables, restart strategies, failure reporting, nested supervisors, and lifecycle queries.
- `trials/ffi_addition`: dangerous actor and guarded FFI fault scenarios, including supervised dangerous-worker restart behavior.
- `trials/cpu_discovery_and_spawn_pinning`: Apple topology helpers, core capability records, and spawn/core placement integration.
- `trials/error_enforcement_addition`: missing `ActorMessage`, wrong message types, wrong actor helper arities, public `spawn_linked` rejection, concurrency-effect enforcement, and dangerous actor misuse.

Before marking the remaining TODOs complete, add trials that prove real link/monitor semantics, full migration behavior, and any new stack-size/page-location surface.

---

## Priority Order

| Priority | Item | Why |
|----------|------|-----|
| 1 | Complete `link` / `monitor` / `demonitor` runtime semantics | These are exposed to the type checker but runtime behavior is still stub-like. |
| 2 | Specify full actor migration semantics | Current `move`/`migrate_actor` names overpromise if read as region relocation rather than affinity retagging. |
| 3 | Tie migration to region ownership analysis | Depends on the region memory safety TODOs. |
| 4 | Decide stack-size public API | Avoid adding surface syntax until actor memory-region invariants are stable. |
| 5 | Add non-Apple backend plans | Keep platform-specific runtime promises explicit. |

---

## Revision History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-04-07 | Initial implementation plan based on spec gap audit |
| 2.0 | 2026-04-07 | Phases 1-4 substantially complete; deferred items documented |
| 3.0 | 2026-05-26 | Moved into `Phase1_TODOs`; updated against current source: topology implemented, move/migrate limited to affinity retagging, `spawn_linked` public surface removed, link/monitor runtime semantics still open |
