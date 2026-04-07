# Actor Implementation Plan: Spec Gap Closure

**Date**: April 7, 2026
**Status**: Phases 1–4 substantially complete
**References**:
- [silica-specification.md](silica-specification.md) §15, §16, §22
- [silica-specification-additional.md](silica-specification-additional.md) §4
- [actor_growable_stack_design.md](actor_growable_stack_design.md)

---

## Current State Summary

The core actor pipeline — `spawn`, `call`, `cast`, `send`, `self`, and the gen_server-style runtime loop — is functional end-to-end (lexer → parser → type checker → SIR → emitter → bundled AArch64 runtime). The runtime uses `pthread_create`, mutex/cond synchronization, and a linked-list mailbox. Effect checking for `proc[concurrency]` is in place. Behavior function shape validation works.

---

## Phase 1: Type-Checker Hardening (Correctness) ✅ COMPLETE

### 1a. ActorMessage Enforcement ✅ DONE

**Spec**: §16.3.2 — `ActorMessage` is a marker trait. A type may be used as the payload of `send()` or `cast()` **only** with the in-place postfix annotation: `expr impl ActorMessage {}`.

**Implementation** (completed):

1. **Parser** (`constraint_extract.silica`): Changed `skip_impl_annotation` to return `(ListTokenSlot, bool)` where the bool indicates whether `impl ActorMessage {}` was found. In `extract_call_rest`, when found on an argument, the pair node's `value` field is set to `"AM"`.

2. **Type checker** (`type_checker_expressions.silica`): Added `check_actor_message_satisfies_am` which checks for the `"AM"` marker on the call expression's right_expr (the second argument pair node). If absent, emits:
   ```
   E2005: message must satisfy ActorMessage (§16.3.2); use `expr impl ActorMessage {}`
   ```
   This check is applied in `check_actor_call_builtin`, `check_actor_cast_builtin`, and `check_actor_send_builtin`.

3. **Type checker helpers** (`type_checker_expressions_actors.silica`): Added `call_has_impl_am_marker` to inspect the parser-set marker.

**Files modified**: `src/parser/constraint_extract.silica`, `src/type_checker/expressions/type_checker_expressions.silica`, `src/type_checker/expressions/type_checker_expressions_actors.silica`

---

### 1b. Message Type Matching + `call()` Reply Type Extraction ✅ DONE

**Spec**: §16 — The message passed to `call`/`cast`/`send` must match the message parameter type of the spawned behavior function. §16.1.1 — `call(actor, message) -> Reply proc[concurrency]` where `Reply` is extracted from the behavior's `(:reply, Reply, State)` return type.

**Implementation** (completed — Approach B):

1. **Behavior type tracking via symbol table**: When a let-binding creates an `actor_ref` from `spawn(init, behavior_fn)`, the type checker now also stores `__actor_beh__<var_name>` → `behavior_function_type` in the symbol environment.

2. **Type checker helpers** (`type_checker_expressions_actors.silica`): `parse_behavior_msg_param_type`, `lookup_actor_behavior_type`, `parse_behavior_reply_type`

3. **Type checker** (`type_checker_expressions.silica`): Message validation in `check_actor_call_builtin`, `check_actor_cast_builtin`, `check_actor_send_builtin`. Reply type validation for `call()`.

**Files modified**: `src/type_checker/expressions/type_checker_expressions.silica`, `src/type_checker/expressions/type_checker_expressions_actors.silica`

---

### 1c. Behavior Non-Recursion Check ✅ DONE

**Spec**: §15.1.2 — Behavior functions must not be recursive.

**Implementation** (completed): `body_has_self_call` AST walker + `check_behavior_no_self_recursion` in declaration checker.

**Files modified**: `src/type_checker/expressions/type_checker_expressions_actors.silica`, `src/type_checker/declarations/type_checker_declarations_functions.silica`

---

### 1d. Core Affinity Argument Typing ✅ DONE

**Spec**: §4.6, §15.1.1 — spawn's optional 3rd argument validated as `int64` or `List[int64,normal]`.

**Files modified**: `src/type_checker/expressions/type_checker_expressions.silica`

---

## Phase 2: Migration & Topology Runtime ✅ COMPLETE (core items)

### 2a. `migrate_actor()` — Deferred

Requires `move()` semantics (region transfer at runtime). Depends on 2b and 4d.

### 2b. `move(processid, from, to)` — Deferred

Requires region move semantics at runtime level.

### 2c. `get_cpu_topology()` — Deferred

Requires `cpu_topology` struct layout definition in the emitter.

### 2d. `get_efficiency_cores()` / `get_performance_cores()` ✅ DONE

Real `sysctlbyname` implementations in AArch64 assembly. `_silica_rt_build_int_range_list` helper builds cons-cell lists.

**Files modified**: `src/emitter/terms/prims/prims_actors_runtime_asm.silica`

### 2e. `pin_actor_to_core()` and Variants ✅ DONE

Real implementations using `pthread_mach_thread_np` + `thread_policy_set(THREAD_AFFINITY_POLICY)`:
- `pin_actor_to_core(actor_ref, core_id)` — sets affinity tag = core_id
- `pin_actor_to_efficiency_core(actor_ref)` — affinity tag = 0x45 ('E')
- `pin_actor_to_performance_core(actor_ref)` — affinity tag = 0x50 ('P')
- `pin_actor_realtime` — stub (requires `THREAD_TIME_CONSTRAINT_POLICY`)

**Files modified**: `src/emitter/terms/prims/prims_actors_runtime_asm.silica`

### 2f. `remove_actor()`, `unpin_actor()`, `set_actor_priority()` ✅ DONE

- `remove_actor(actor_ref)` — locks mutex, sets alive=0, signals cond, unlocks
- `unpin_actor(actor_ref)` — `thread_policy_set` with affinity_tag=0
- `set_actor_priority(actor_ref, priority)` — maps priority (0–4) to QoS class via `pthread_set_qos_class_np`

**Files modified**: `src/emitter/terms/prims/prims_actors_runtime_asm.silica`

---

## Phase 3: Growable Stack Architecture ✅ COMPLETE

### 3a. Per-Actor Virtual Memory (1GB sparse) + Initial 8MB Mapping ✅ DONE

**Implementation**:
- Actor struct extended to 192 bytes: `[160:168]` = region_base, `[168:176]` = mapped_size, `[176:184]` = alloc_ptr, `[184:192]` = total_size
- `_silica_rt_actor_init_region`: reserves 1GB VA via `mmap(PROT_NONE, MAP_PRIVATE|MAP_ANON)`, maps first 8MB via `mprotect(PROT_READ|PROT_WRITE)`
- Called from `_silica_rt_actor_spawn` after pthread_detach
- SIGSEGV handler installed on first spawn call (once-guard via `_silica_rt_fault_handler_installed`)

**Files modified**: `src/emitter/terms/prims/prims_actors_runtime_asm.silica`

### 3b. SIGSEGV Handler for Stack Growth ✅ DONE

**Implementation**:
- `_silica_rt_install_fault_handler`: installs `_silica_rt_stack_fault_handler` via `sigaction(SIGSEGV, SA_SIGINFO)`
- `_silica_rt_stack_fault_handler`: reads `si_addr` from `siginfo_t`, gets current actor from `_tl_current_actor` TLS, validates fault is within actor's region, calls `mprotect(page, 4096, PROT_READ|PROT_WRITE)`, updates `mapped_size`
- Out-of-region faults (stack overflow): gracefully terminates actor (sets alive=0, signals cond)
- Non-actor faults: re-raises SIGSEGV via `signal(SIGSEGV, SIG_DFL); raise(SIGSEGV)`

**Files modified**: `src/emitter/terms/prims/prims_actors_runtime_asm.silica`

### 3c. Actor Termination: Full Cleanup ✅ DONE

**Implementation**:
- Thread main exit path (`LBB1_14`): calls `_silica_rt_actor_dealloc_region` (munmap), then `pthread_mutex_destroy`, `pthread_cond_destroy`, `free(actor_struct)`
- `_silica_rt_actor_dealloc_region`: reads `region_base` and `total_size` from actor struct, calls `munmap`

**Files modified**: `src/emitter/terms/prims/prims_actors_runtime_asm.silica`

### 3d. Emitter: Region-Based Allocation ✅ DONE

**Implementation**:
- `_silica_rt_region_alloc(size)`: bump-allocates from current actor's mmap region via TLS lookup; 8-byte aligned; falls back to `_malloc` when not in actor context
- `prims_list.silica`: cons cell allocation changed from `BL _malloc` to `BL _silica_rt_region_alloc`
- `prims_memory.silica`: region allocation changed from `BL _malloc` to `BL _silica_rt_region_alloc`

**Files modified**: `src/emitter/terms/prims/prims_list.silica`, `src/emitter/terms/prims/prims_memory.silica`, `src/emitter/terms/prims/prims_actors_runtime_asm.silica`

### 3e. Stack Size Configuration at Spawn — Deferred

Requires extending `spawn()` with a 4th argument for stack size. Default 1GB total / 8MB initial is used.

### 3f. Lazy NUMA Page Migration — Deferred

Apple Silicon is UMA. NUMA migration is only relevant for future Linux support.

---

## Phase 4: Advanced Features ✅ COMPLETE (core items)

### 4a. Region Move Semantics at Spawn ✅ DONE

**Spec**: §12.1.5 — When `initial_state` contains a region handle, ownership transfers to the actor.

**Implementation** (completed):
- `enrich_env_for_actor_spawn` extended: when spawn's initial_state is a variable with a memory region type (`ref(...)`, `region(...)`, `buf(...)`, `atomic_ref(...)`), a shadow binding with type `"moved_region"` is added to the environment
- Any subsequent use of the moved variable will fail type checking (type mismatch against `"moved_region"`)

**Files modified**: `src/type_checker/expressions/type_checker_expressions.silica`

### 4b. Memory Monitoring APIs ✅ DONE

**Spec**: actor_growable_stack_design.md §7.2

**Implementation** (completed — full pipeline):
- `get_actor_memory_usage(actor_ref) -> int64`: full pipeline (type checker, SIR generator, emitter, runtime)
- Runtime: `_silica_rt_get_actor_memory_usage` reads `mapped_size` from actor struct offset 168
- `get_actor_page_locations` — deferred (requires `MapInt64ToInt64` type)

**Files modified**: `src/type_checker/expressions/type_checker_expressions.silica`, `src/sir_generator/terms/actor_calls.silica`, `src/emitter/terms/prims/prims_actors.silica`, `src/emitter/terms/prims/prims_actors_runtime_asm.silica`

### 4c. Error Handling: Stack Overflow Detection ✅ DONE

**Implementation** (completed):
- SIGSEGV handler detects out-of-region faults (address ≥ region_base + total_size)
- On overflow: sets actor's alive flag to 0, signals cond variable for graceful termination
- Actor thread exits cleanly on next message loop iteration
- Non-actor SIGSEGV: re-raises signal with default handler

**Files modified**: `src/emitter/terms/prims/prims_actors_runtime_asm.silica`

### 4d. Migration Strategy Selection — Deferred

Requires `move()` and `migrate_actor()` runtime implementations. Strategies: `lazy`, `eager_copy`, `static_core`.

---

## Deferred Items Summary

| Item | Dependency | Notes |
|------|-----------|-------|
| 2a. `migrate_actor()` | Region move runtime | Needs runtime move semantics |
| 2b. `move(processid, from, to)` | Region move runtime | Needs runtime region transfer |
| 2c. `get_cpu_topology()` | `cpu_topology` struct | Needs struct layout definition |
| 3e. Stack size configuration | 4th spawn argument | Default 1GB/8MB works |
| 3f. NUMA migration | Linux support | Apple Silicon is UMA |
| 4d. Migration strategy | 2a, 2b | Depends on move/migrate |

---

## Priority Order

| Priority | Item | Effort | Impact | Status |
|----------|------|--------|--------|--------|
| 1 | 1a. ActorMessage enforcement | Medium | High | ✅ Done |
| 2 | 1b. call() reply type | Medium | High | ✅ Done |
| 3 | 1c. Behavior non-recursion | Low | Medium | ✅ Done |
| 4 | 1d. Core affinity typing | Low | Medium | ✅ Done |
| 5 | 2d. Topology real impl | Medium | Medium | ✅ Done |
| 6 | 2e-f. Pinning/remove/priority | Medium | Medium | ✅ Done |
| 7 | 3a-b. Virtual memory + SIGSEGV | High | High | ✅ Done |
| 8 | 3c. Termination cleanup | Medium | High | ✅ Done |
| 9 | 3d. Region-based allocation | High | High | ✅ Done |
| 10 | 4a. Region move semantics | High | High | ✅ Done |
| 11 | 4b. Memory monitoring | Medium | Medium | ✅ Done |
| 12 | 4c. Stack overflow handling | Medium | High | ✅ Done |
| 13 | 2a. migrate_actor | Medium | Medium | Deferred |
| 14 | 2b. move() | Low | Low | Deferred |
| 15 | 2c. get_cpu_topology | Medium | Medium | Deferred |
| 16 | 3e-f. Stack config + NUMA | Medium | Medium | Deferred |
| 17 | 4d. Migration strategy | Medium | Medium | Deferred |

---

## Revision History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-04-07 | Initial implementation plan based on spec gap audit |
| 2.0 | 2026-04-07 | Phases 1–4 substantially complete; deferred items documented |
