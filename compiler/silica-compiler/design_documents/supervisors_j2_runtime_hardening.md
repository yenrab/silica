# Phase J2 — Shared runtime primitives: hardening playbook

**Parent:** [supervisors_implementation_development_plan.md](supervisors_implementation_development_plan.md) §4 Phase **J**  
**Prerequisite:** [supervisors_j1_invariant_audit.md](supervisors_j1_invariant_audit.md) — ordering and single `maybe_restart` entry points  
**Date:** April 28, 2026  

Phase **J2** documents **how** the Apple-Silicon actor runtime stays on the supported lock/wait/allocator path, **what** to verify before changing assembly, and **how** to regress-test under load. It does **not** mandate a specific sanitiser build of the compiler (that stays optional per workstation).

---

## 1. Exit criteria (plan §4 J2)

| Criterion | Delivery |
|-----------|----------|
| Repeatable stress harness | **`trials/supervisors_addition/stress_j2_supervision_harness.sh`** — `make integrate` + **`stress_j1_supervision_batch.sh`** (e4e/e4f loops). Optional env: `INTEGRATE_ROUNDS`, `STRESS_ITERS`, `J2_LOG`. |
| **Makefile** entry point | `make stress-j2` in **`trials/supervisors_addition/`** runs the harness. |
| Documented regression window | §3 below — what “green” means and what to record when bisecting flakes. |
| Runtime edits justified by evidence | §4 — no barrier-only or pthread reintroduction without measurement (see plan). |

---

## 2. In-tree facts (audit April 2026)

### 2.1 `___ulock_wait` / `___ulock_wake` opcode

- **`UL_COMPARE_AND_WAIT_SHARED` = 3** (`bsd/sys/ulock.h`). The plan warns that opcode **1** (`UL_COMPARE_AND_WAIT` alone) correlated with intermittent **`EXC_BAD_ACCESS`** under stress.
- **Emitted implementation** (string chunks in `prims_actors_runtime_asm.silica`, `actor_rt_asm_cv_chunk`):  
  - `_silica_rt_cv_signal`: `mov w0, #3` before `bl ___ulock_wake`  
  - `_silica_rt_cv_wait`: `mov w0, #3` before `bl ___ulock_wait`  
- **Paired lock:** `os_unfair_lock` at the address passed as `X1` to `_silica_rt_cv_wait` (typically `ACB+16` or call-wrapper `+72`).

### 2.2 Do **not** re-embed `pthread_mutex_t` / `pthread_cond_t` in `calloc`’d ACBs

Historical **`libpthread` kwq** leakage + missing destroy, **`mfm_alloc` SIGBUS**, **`_os_unfair_lock_corruption_abort`** — summarized in `prims_actors_runtime_asm.silica` header and `actor_rt_asm_cv_chunk` comments. Any reintroduction needs **full** init/destroy pairing and a written rationale.

### 2.3 Barriers

Empirical A/B on this runtime: extra ARM barriers **increased** failure rates vs plain load/store with **`os_unfair_lock`**. Treat ordering bugs as **instrumentation + proof** problems, not “add `dmb`” (plan §4 J2).

### 2.4 Patching caveat (automation / LLM)

In **`actor_rt_asm_cv_chunk` / `_silica_rt_cv_wait`**: do **not** insert **`nop`**, stray labels, or patch points **immediately after** `bl ___ulock_wait` in ways **`llvm-as` rejects** — risk of **SIGBUS on arm64e** (signed return / resume). Debug with **lldb** on `_silica_rt_cv_wait`, single-step across `___ulock_wait` (see inline comment in `prims_actors_runtime_asm.silica`).

### 2.5 When flakes cluster — suspects

Prefer **process** before code churn:

| Layer | Symptoms / tools |
|-------|-------------------|
| **Allocator** | `calloc`/`free` imbalance, crashes in malloc zones → ** Instruments → Allocations / Leaks**, `malloc_history`, guarded malloc (dev only). |
| **TLS** | New-actor instantiation, TLV bootstrap → breakpoints on **`_tl_current_actor`** resolution, watch concurrent **spawn_linked** bursts. |
| **Scheduler load** | Nondeterministic ordering under contention → **`stress_j2_supervision_harness.sh`** with `INTEGRATE_ROUNDS` > 1 and elevated `STRESS_ITERS`; compare `.integrate_counts` and exit codes batch-to-batch. |

---

## 3. Regression window (“green” baseline)

Before merging a runtime change touching **`prims_actors_runtime_asm.silica`** or **`prims_actors_child_table_asm.silica`:**

1. **`make integrate`** in **`compiler/silica-compiler/trials/supervisors_addition/`** completes with exit status **0**; note **`ok`** / **`ko`** from **`.integrate_counts`** (`printf '%d %d\n' ok ko` — second column should stay **0**).
2. **`./stress_j2_supervision_harness.sh`** (or **`make stress-j2`**) passes with default **`STRESS_ITERS`** (25) and **`INTEGRATE_ROUNDS`** (1). For stress validation, optionally raise **`STRESS_ITERS=100`** / **`INTEGRATE_ROUNDS=5`** overnight.
3. Record in the PR/commit message or issue: **hardware model** (Apple Silicon variant), **macOS/SDK** if relevant, **git revision**, **`ok ko`** line, **`STRESS_ITERS` / `INTEGRATE_ROUNDS`**.

**Not sufficient alone:** single green **`e5`** integrate line item — Phase J warns **e5 green ≠ stress-stable**.

---

## 4. Evidence bar for behavioral changes

- Prefer **lldb backtraces**, **crash reports**, **diff of `.scout` regressions**, or **Instruments time profiles** showing lock/alloc hotspots.
- **Address Sanitizer / Malloc Scribble** on a reduced binary (single trial linked with same runtime) when investigating heap corruption — not every change needs it, but **heap** suspects should get a sanitizer run before speculative fixes.
- **Bisect** flakes by running the harness at known-good vs known-bad SHAs with fixed **`STRESS_ITERS`**.

---

## 5. Primary files

| File | Role |
|------|------|
| `src/emitter/apple_silicon/terms/prims/prims_actors_runtime_asm.silica` | ACB layout, `_actor_thread_main`, CV helpers, `deliver_exit`, `supervision_wait_and_drain_one` |
| `src/emitter/apple_silicon/terms/prims/prims_actors_child_table_asm.silica` | Heap child table, `_silica_rt_supervisor_maybe_restart` |

---

## 6. Harness usage

From **`compiler/silica-compiler/trials/supervisors_addition`:**

```bash
# Default: 1× integrate + 25 iterations of e4e+e4f stress
./stress_j2_supervision_harness.sh

# Or via Make (runs compile/link prerequisites via integrate dependency chain inside integrate)
make stress-j2
```

Environment:

| Variable | Default | Meaning |
|----------|---------|---------|
| `INTEGRATE_ROUNDS` | `1` | Repeat **`make integrate`** sequentially (full gate). |
| `STRESS_ITERS` | `25` | Passed to **`stress_j1_supervision_batch.sh`** (`phase_e4e_one_for_all` / `phase_e4f_rest_for_one` per iteration). |
| `J2_LOG` | unset | If set (path), append-only log of timestamps + commands (optional audit trail). |

---

## 7. Relation to Phase J3

[J3](supervisors_implementation_development_plan.md) (M:N scheduler, narrower shared state) applies **after** J1+J2 improvements plateau — this doc does **not** prescribe J3.
