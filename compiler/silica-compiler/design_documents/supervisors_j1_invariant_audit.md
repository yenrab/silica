# Phase J1 — “One logical writer” invariant audit

**Parent:** [supervisors_implementation_development_plan.md](supervisors_implementation_development_plan.md) §4 Phase **J**  
**Spec anchors:** [silica-specification.md](silica-specification.md) §15.4 (supervision), §16.2.7 (scheduler / mailbox ordering)  
**Date:** April 28, 2026  

This note records the **exit criteria for J1**: a written invariant checklist, code-path audit, and pointers for stress repros. It does **not** replace **J2** (runtime primitive hardening) or **J3** (architectural options).

---

## 1. Invariant checklist (enforceable statements)

| ID | Invariant | Verified in tree (Apr 2026) | Notes |
|----|-----------|------------------------------|--------|
| **J1.1** | **Supervision ingress is drained before the standard mailbox** on each `_actor_thread_main` scheduling turn | **Yes** | `prims_actors_runtime_asm.silica`: dequeue **#232** (ingress) first, then **#128** (standard) in `LBB1_wait` / `LBB1_try_std`. Header comment at file top documents the pair layout. |
| **J1.2** | **`_silica_rt_actor_supervision_wait_and_drain_one` does not hold `ACB+16` across `_silica_rt_supervisor_maybe_restart`** | **Yes** | Same file: `Lswd_try` dequeues under lock, then `bl _os_unfair_lock_unlock` **before** `bl _silica_rt_supervisor_maybe_restart` (`Lswd` path, ~2361–2370). Comment: “mutex must not be held — maybe_restart may spawn / take table locks”. |
| **J1.3** | **`_actor_thread_main` does not hold the current actor’s `ACB+16` across `maybe_restart`** (ingress path `LBB1_free`, `w21==1`) | **Yes** | `LBB1_process` releases `ACB+16` **before** running the nested behaviour (`bl _silica_rt_actor_call` chain). **`LBB1_free`**/`maybe_restart` therefore run **without** holding the dequeuing actor’s mutex. Separate from **`supervision_wait_and_drain_one`** (J1.2), which unlocks supervisor `+16` explicitly after dequeue. |
| **J1.4** | **`_silica_rt_supervisor_maybe_restart` is not duplicated**: the **only** linkable entry points are the two `bl _silica_rt_supervisor_maybe_restart` sites in emitted runtime ASM | **Yes** | `rg '_silica_rt_supervisor_maybe_restart' compiler/silica-compiler/src/**/*.silica` → **only** `prims_actors_runtime_asm.silica` defines call sites (**not** `prims_actors_child_table_asm.silica`, which **defines** the symbol body). Embedded copies in `.sams` / `.ascomp` artifacts are linker bundles, not second implementations. |
| **J1.5** | **Cross-thread mutation of heap child-table *policy*** (restart, row respawn, strategy-driven cascade kill) runs **inside** `_silica_rt_supervisor_maybe_restart` or **serialized** trampolines `_silica_rt_supervisor_materialize_init_children` / `_silica_rt_supervisor_start_child` on the **supervisor actor’s thread** (`_tl_current_actor` = supervisor) | **Partial / by design** | **Writers:** `maybe_restart` expects `X1` = supervisor `ACB`; `materialize_*` / `start_child` take supervisor from TLS before table mutation. **Readers:** accessors `child_table_first_ref` / `child_table_row_ref` run from supervisor behaviours (type-checked). **Other threads** deliver **enqueue-only** exits via `_silica_rt_actor_deliver_exit` (locks supervisor long enough to append to ingress + signal `ACB+80`). **Exception:** cascade paths in `maybe_restart` lock **sibling child `ACB+16`** to set `alive=0` — intentional contention called out in plan Phase J motivation. |
| **J1.6** | **No second “shadow” restart engine** in the compiler pipeline (lexer/SIR) that mutates the table | **Yes** | Restart policy is **runtime-only**; compiler exposes `start_child`, `child_table_*`, `supervision_*` as calls into the prims above. |
| **J1.7** | **Stress repros for contention** are **`phase_e4e_one_for_all`** and **`phase_e4f_rest_for_one`**, not strategy renames | **Documented** | Heavier sibling walks + kill/respawn order; use batch script [../trials/supervisors_addition/stress_j1_supervision_batch.sh](../trials/supervisors_addition/stress_j1_supervision_batch.sh) after building binaries. |

---

## 2. Code-path audit — who touches what

### 2.1 `_silica_rt_supervisor_maybe_restart` (single definition)

- **Definition:** `prims_actors_child_table_asm.silica` → emitted `child_table_rt_asm_chunk_3` → global `_silica_rt_supervisor_maybe_restart`.
- **Call sites (runtime source, exactly two):**

| # | Location | Caller context | `X0` (child ref) | `X1` (supervisor) | `X2` / reason tag |
|---|----------|----------------|------------------|-------------------|-------------------|
| 1 | `LBB1_free` path in `_actor_thread_main` | Actor processing **ingress** message (`w21==1`) | From payload | Current actor `x19` | From payload `+8` (e4b) |
| 2 | `Lswd_free_wrap` after `_silica_rt_actor_supervision_wait_and_drain_one` | **Supervisor** behaviour (TLS) | From payload | `x19` from TLS | From payload `+8` |

**Policy:** Do **not** add a third call site without updating this audit and proving ingress ordering is preserved.

### 2.2 Supervision event **enqueue** (non–supervisor threads)

- **`_silica_rt_actor_deliver_exit`**: builds payload, locks **supervisor** `+16`, checks supervisor `+152` (alive), enqueues to supervisor ingress `+232`, bumps depth `+248`, signals `+80`, unlocks. **Does not** edit child table rows.
- **`_silica_rt_actor_supervision_ingress_enqueue`**: Phase B / synthetic trial hook; same bounded queue discipline.

### 2.3 Child table structure **writers** (supervisor thread)

| API | File (chunk) | When |
|-----|--------------|------|
| `_silica_rt_supervisor_materialize_init_children` | `child_table_rt_asm_chunk_1` | First-schedule supervisor trampoline; sequential `spawn_linked` per init list row |
| `_silica_rt_supervisor_start_child` | `child_table_rt_asm_chunk_2` | `start_child(spec)` from supervisor behaviour |
| `_silica_rt_child_table_ensure_min_cap` | `child_table_rt_asm_chunk_0` | Grow table buffer |
| `_silica_rt_supervisor_maybe_restart` | `child_table_rt_asm_chunk_3` | After child death notification |
| `_silica_rt_child_table_free` | `child_table_rt_asm_chunk_0` | Actor teardown (`LBB1_exit` on dying actor) |

### 2.4 Child table **read-only** accessors

- `_silica_rt_child_table_first_ref`, `_silica_rt_child_table_row_ref` — scan rows; must be called from supervisor behaviour (enforced in `type_checker_expressions.silica`).

---

## 3. Ordering contract (must remain true after edits)

1. **`_actor_thread_main`:** ingress queue **#232/#240** before standard **#128/#136** each iteration (`LBB1_wait` / `LBB1_try_std`).
2. **`_silica_rt_actor_supervision_wait_and_drain_one`:** lock → dequeue one → **unlock** → `maybe_restart` → free node.
3. **`_silica_rt_cv_wait` / `_silica_rt_cv_signal`:** paired with `os_unfair_lock` on `ACB+16`; futex word `ACB+80` — see `actor_rt_asm_cv_chunk` and [**Phase J2**](supervisors_j2_runtime_hardening.md) (do not “fix” flakes with random barriers).

---

## 4. Future work (out of J1 scope)

- **J2:** see **[`supervisors_j2_runtime_hardening.md`](supervisors_j2_runtime_hardening.md)** — ulock opcode verification, barrier/pthread cautions, **`stress_j2_supervision_harness.sh`** / **`make stress-j2`**, regression-window checklist.
- **J3:** if pthread-per-actor contention plateaus, consider M:N scheduling or narrower shared surfaces (plan §4).

---

## 5. References

- `compiler/silica-compiler/src/emitter/apple_silicon/terms/prims/prims_actors_runtime_asm.silica` — ACB layout, `_actor_thread_main`, `_silica_rt_actor_supervision_wait_and_drain_one`, `deliver_exit`
- `compiler/silica-compiler/src/emitter/apple_silicon/terms/prims/prims_actors_child_table_asm.silica` — child table, `maybe_restart`, materialize, `start_child`
