# Phase F5 — unwind path coverage (audit)

**Purpose:** Record how §4 Phase **F5** (“path coverage + stderr fallback”) is evidenced in-tree. Normative semantics remain in **`silica-specification.md`** (§15.4.6, §15.4.10–§15.4.11.4).

## Central sequencing (`prims_actors_runtime_asm.silica`)

Actor thread shutdown is single-exit **`LBB1_exit`** (`actor_thread_main`), after **`alive`** at **ACB+#152** is cleared:

1. **`_os_unfair_lock_unlock`** (ACB+16).
2. **`_silica_rt_actor_deliver_exit(X0=self)`** — builds structured ingress notification when a supervisor exists and is alive; returns **X0∈{0,1}** (**1** = dropped: no supervisor, dead supervisor §15.4.10.4, or OOM before enqueue).
3. **`_silica_rt_actor_deliver_unwind_report(X0=self)`** — **always** runs unwind text (**independent** of step 2’s return value except legacy root note below).
4. If step 2 returned **≠0**, **`_write(2, L_silica_root_actor_exit, …)`** legacy stderr line (distinct from Phase F block).
5. Teardown (`_silica_rt_child_table_free`, region free, **`_free(ACB)`**).

So ingress (Phase C/E) and unwind (Phase F **F4**) share the same teardown site; **ingress can be suppressed** while unwind still proceeds.

## Unwind formatting + routing (`actor_rt_asm_phase_f_chunk`)

- **`_silica_rt_actor_deliver_unwind_report`**: `malloc(2048)` + `_snprintf` with **`L_phase_f_unwind_fmt`**, then either **`_silica_rt_actor_cast`** to **`_silica_rt_root_failure_reporter`** when set and alive, or **`_write` stderr**.
- **Malloc failure**: **`Ldup_oom_msg`** one-line **`_write`** to stderr (**F5** hardening) so teardown never skips all reporting silently.
- **FailureReporter is the dying actor**: clear **`_silica_rt_root_failure_reporter`**, route text to stderr.

## Proof trials

| Trial | What it proves |
|-------|----------------|
| **`phase_f5_unwind_stderr_fallback`** | **`register_failure_reporter`** **not** called; child exit merges Phase C ingress lines plus **`=== Silica Actor Failure ===`** via **`2>&1`** (stderr → integrate capture). |
| **`phase_f_failure_reporter_cast_alignment`** | FR registered first; ingress path unchanged; unwind intended for reporter mailbox (**F3**/F4 bring-up — see trial header). |
| **`phase_f7_bootstrap_ordering`** | **F7**: normative **`main`** ordering evidenced (`F7_bootstrap_ok` after **`register_failure_reporter`**, supervisor spawned after); see **`supervisors_f7_bootstrap_ordering.md`**. |

Further **F5** work (explicitly **out-of-band** audits, not gated by compiler changes): escalate/kill/matrix that forces **supervisor-first** teardown order; dead-sup **+** concurrently dropped ingress—document results when hardened.
