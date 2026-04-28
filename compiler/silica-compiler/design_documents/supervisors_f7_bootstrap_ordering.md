# Phase F7 — root `FailureReporter` bootstrap ordering

**Purpose:** Normative **program structure** for **`register_failure_reporter/1`** relative to other root actors, so unwind delivery (§15.4.11.4, §15.4.13.4) can reach the intended `FailureReporter` before dependents run work that may terminate and emit unwind reports. Spec surface: **`silica-specification.md` §15.4.13.4** (root `FailureReporter`); runtime slot: **`_silica_rt_root_failure_reporter`** in `prims_actors_runtime_asm.silica`.

## Requirement

1. **Spawn** the root `FailureReporter` implementation actor ( **`spawn(…, fr_beh, …)`** ).
2. **Register** it with **`register_failure_reporter(fr_actor_ref)`** — this stores the ACB pointer in **`_silica_rt_root_failure_reporter`** (first successful registration wins; see F2).
3. **Then** spawn root supervisor(s), workers, or other system actors whose **tear down** may produce unwind text while the program still expects those reports to **cast** to the `FailureReporter`.

There is **no** automatic runtime or linker pass that creates or registers a `FailureReporter`; **`main/0`** (or the program entry you own) must establish ordering explicitly.

## Why order matters

`_silica_rt_actor_deliver_unwind_report` runs during **every** actor exit (`LBB1_exit` after `_silica_rt_actor_deliver_exit`). If **`_silica_rt_root_failure_reporter`** is still **zero** at that moment, the runtime **writes the formatted block to stderr** (F5) instead of enqueueing to FR. If a child is spawned and can exit **before** `register_failure_reporter` runs, that exit’s unwind path may see an **unregistered** slot.

Conversely, **`register_failure_reporter` before** those actors makes the global slot valid for their teardown (subject to FR liveness — if the FR actor is dead or not started, the runtime falls back per F4/F6).

## Canonical `main` skeleton

```silica
sequence proc[concurrency, device_io]
    fr: actor_ref <- spawn(0, fr_beh);
    _: atom <- register_failure_reporter(fr);
    sup: actor_ref <- spawn(0, sup_beh);
    …
end
```

Do **not** rely on **`spawn`** scheduling order alone without registration: the root slot is only set by **`register_failure_reporter`**.

## Relationship to Phase I (`wait_for_exit`)

When **`wait_for_exit/0`** and a standard long-lived **`main`** loop exist (Phase **I** — not required for F7 completion), the same ordering applies at startup: register the root `FailureReporter` **before** starting the root supervisor tree that will run until `wait_for_exit` returns. No change to F7’s **documented contract** is expected when Phase I lands—only the surrounding **control flow** around the skeleton above.

## Proof / reference in-tree

| Artifact | Role |
|----------|------|
| **`trials/supervisors_addition/phase_f7_bootstrap_ordering.silica`** | Executable reference: prints **`F7_bootstrap_ok`** immediately after **`register_failure_reporter`**, before spawning the supervisor. **`handle_report`** is intentionally silent on **stdout** so integrate goldens stay deterministic (FR thread **`device_io`** races supervisor thread **`device_io`**). Typed **`handle_report`** is proved under **`phase_f_failure_reporter_cast_alignment`** (F6). |
| **`trials/supervisors_addition/phase_f_failure_reporter_cast_alignment.silica`** | Same **`main`** ordering (FR → register → supervisor); F6 **`F6_handle_report`** unwind probe. |

## Anti-patterns

- Spawning a linked child or supervisor **before** **`register_failure_reporter`** when those actors may exit early and you require FR delivery (not stderr-only).
- Assuming **TLS** or **linker** initializes the root `FailureReporter`; only **`register_failure_reporter`** sets the global.

---

*Cross-links: [supervisors_implementation_development_plan.md](supervisors_implementation_development_plan.md) §4 Phase F **F7**; [supervisors_f5_exit_path_audit.md](supervisors_f5_exit_path_audit.md) (teardown vs ingress).*
