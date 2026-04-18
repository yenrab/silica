# Supervisors Implementation — Development Plan

**Date**: April 18, 2026  
**Status**: Planning (not started as a tracked epic)  
**Primary specification**: [silica-specification.md](silica-specification.md) — §15.1.3, §15.4 (Supervision and Fault Tolerance), §16.2.7  

**Related plans and docs**:
- [actor_implementation_plan.md](actor_implementation_plan.md) — baseline actor runtime, mailboxes, `spawn`, effects
- [silica-specification.md](silica-specification.md) §15.4.6 — trap / unwind / failure reporting (ingress is separate from `FailureReporter`)
- [silica_actor_capabilities_specification.md](silica_actor_capabilities_specification.md) — if present, align capability boundaries with supervision

---

## 1. Goal

Implement **OTP-style supervision** as specified: supervisor actors implementing the **`Supervisor` trait**, **`spawn_linked`** as the parent–child binding, a **high-priority supervision ingress** for exit notifications, structured **failure payloads**, **declarative restart policies** for `child_spec` children, independent **unwind reports** to **`FailureReporter`**, and correct interaction with **`link`**, **`monitor`**, and cascading shutdown.

Out of scope for an initial milestone unless explicitly pulled in: full **`:one_for_all` / `:rest_for_one`** semantics if deferred; **FFI fault containment** (§15.4.13.5) beyond hooks needed for ordinary actor death; **migrate_actor** and topology features that depend on unimplemented `move()` semantics.

---

## 2. Specification Map (authoritative sections)

| Topic | Spec anchor |
|-------|-------------|
| Supervisor as actor + `Supervisor` trait | §15.4.8.1, §15.4.13 |
| Single supervisor per child; root actor | §15.4.8.2 |
| `spawn_linked` intrinsic; `agent_type`; behavior-only call site | §15.4.8.3 |
| Exit propagation; `link`; non-supervisor exit | §15.4.8.4–§15.4.8.5 |
| `monitor` / `demonitor`; `DOWN` to **standard** mailbox | §15.4.8.6–§15.4.8.7 |
| Supervision ingress motivation, structure, population, ordering | §15.4.9 |
| Unified exit notification delivery | §15.4.10 |
| Failure payload fields; `failure_reason` sum type | §15.4.11 |
| Restart protocol; coordinated shutdown; restart strategies | §15.4.12, §15.4.13.2 |
| `child_spec`, `supervisor_flags`, dynamic children | §15.4.13.1–§15.4.13.3 |
| `FailureReporter` trait; ordering vs restart | §15.4.13.4 |
| Two-channel mailbox model for supervisors | §16.2.2 extension, §16.2.7 |

---

## 3. Prerequisites (inventory before coding)

1. **Runtime audit**: Document current actor struct layout, spawn path, any partial `spawn_linked` / link metadata, and where child death is handled today (`handle_actor_crash` or equivalent per §15.4.10.3).
2. **Compiler audit**: Whether `spawn_linked`, `link`, `monitor`, `Supervisor`, or ingress types appear in lexer/parser/SIR/emitter stubs.
3. **Test harness**: Decide trial layout under `compiler/silica-compiler/trials/` (e.g. new `supervisors_addition` or extend `actors_addition`) for golden / integration checks of exit ordering and restart counts.

---

## 4. Phased Implementation

### Phase A — Runtime metadata and `spawn_linked` (foundation)

**Spec**: §15.4.8.2–§15.4.8.3  

**Deliverables**:
- Child metadata: **supervisor ref** (or none for root), **`agent_type` atom**, bidirectional **link set** or equivalent structure.
- **`spawn_linked(initial_state, behavior_fn, agent_type [, core_id])`**: intrinsic available **only** from behavior context (compile-time enforcement in Phase D; runtime guard acceptable for early bring-up).
- Link established **atomically** before child processes first message.
- **Root actors**: on death with no supervisor, unwind report path to **stderr** (or existing behavior) until `FailureReporter` exists (Phase F).

**Exit criteria**: Unit-level or trial: parent and child both alive; killing child does not yet require full ingress if notifications are stubbed—optional stub callback into runtime test hook.

---

### Phase B — Supervision ingress + scheduler ordering

**Spec**: §15.4.9, §16.2.7  

**Deliverables**:
- Per-actor **second queue** for actors marked as supervisors (or lazily allocated on first `spawn_linked` from a supervisor module—design choice: spec assumes supervisor implements `Supervisor` trait; ingress may be allocated when supervisor actor starts after `init`).
- Scheduler / run loop: **drain ingress completely (or bounded batch with fairness policy)** before dequeuing standard mailbox, each scheduling turn.
- **Bounded** ingress: document max depth; policy on overflow (spec implies trusted runtime only—overflow should be treated as runtime invariant failure or documented drop with escalation).

**Exit criteria**: Synthetic trial: flood standard mailbox; inject synthetic exit notification; supervisor behavior observes exit **before** backlog `cast` (ordering test).

---

### Phase C — Exit notification construction and delivery

**Spec**: §15.4.10, §15.4.11, §15.4.10.4  

**Deliverables**:
- On **any** linked child death (normal, error, explicit, memory fault per §15.4.10.1): build payload with **`child_ref`**, **`failure_reason`**, **`agent_type`** (§15.4.11.1).
- Enqueue exactly **one** notification to supervisor ingress; **no user code** in child participates.
- **Dead supervisor** (§15.4.10.4): drop notification; ensure unwind still reaches stderr / `FailureReporter` per §15.4.11.4.
- **Stack growth** never generates supervision events (§15.4.10.2).

**Exit criteria**: Trials diff structured output or assert on discriminated `failure_reason` variants where stable.

---

### Phase D — Language surface: `Supervisor` trait and checking

**Spec**: §15.4.13, §15.4.8.1  

**Deliverables**:
- Parse and type-check **`trait Supervisor { fn init(self) -> (supervisor_flags, [child_spec]); }`** (exact surface per spec §15.4.13.1–§15.4.13.2).
- Types: **`supervisor_flags`**, **`restart_strategy`**, **`child_spec`** with documented fields (`shutdown` ms semantics, `restart` enum).
- **Compile-time rule**: `spawn_linked` only inside behavior functions (§15.4.8.3); same for **`link`** (§15.4.8.5).
- Mark supervisor actors for runtime (metadata flag) from trait implementation.

**Exit criteria**: Programs that misuse `spawn_linked` / `link` fail type check with stable error codes; minimal supervisor module compiles.

---

### Phase E — Declarative children and automatic restart

**Spec**: §15.4.12.1, §15.4.12.3, §15.4.13.2  

**Deliverables**:
- On supervisor start: runtime calls **`init`**, spawns each **`child_spec`** via internal **`spawn_linked`** equivalent, stores refs in **internal child table** (§15.4.13.1).
- On child exit: if **`restart`** and **`strategy`** permit, runtime performs restart protocol (§15.4.12.1)—**without** involving supervisor behavior for the mechanical respawn; supervisor behavior still receives ingress notification if spec requires visibility for policy (clarify in implementation doc: spec says runtime applies strategy; supervisor may still need notification for logging—align with §15.4.12.1 wording).
- **Escalation**: restart frequency caps; supervisor termination propagates via link (§15.4.13.2 end).
- **Dynamic children** (§15.4.13.3): no automatic restart table; ingress notifications only.

**Exit criteria**: Trial with `:permanent` vs `:transient` vs `:temporary`; `:one_for_one` minimum before `:one_for_all` / `:rest_for_one` if phased.

---

### Phase F — `FailureReporter` and unwind path integration

**Spec**: §15.4.11.4, §15.4.13.4–§15.4.13.6  

**Deliverables**:
- Root **`FailureReporter`** actor started before other system actors (bootstrap ordering).
- **`handle_report`**: string report + region dumps per spec; **never** block restart logic.
- All actor deaths generate unwind report to **`FailureReporter`** or stderr fallback; **independent** of supervisor ingress payload.

**Exit criteria**: Supervised child dies: ingress gets structured fields only; `FailureReporter` receives full string report (trial asserts both channels).

---

### Phase G — `link`, `monitor`, cascading shutdown polish

**Spec**: §15.4.8.4–§15.4.8.7, §15.4.8.5 (supervisor vs non-supervisor exit)  

**Deliverables**:
- **`link`**: idempotent; immediate exit if target dead; supervisor receives via **ingress**; non-supervisor exits with linked failure reason.
- **`monitor` / `demonitor`**: `DOWN` to **standard** mailbox only.
- Supervisor death: exit signals to linked children; non-supervisor children terminate; supervisor children route via ingress (§15.4.8.4, §15.4.8.5).

**Exit criteria**: Matrix trial: link vs monitor vs spawn_linked combinations.

---

### Phase H — Coordinated subtree shutdown (user-level protocol)

**Spec**: §15.4.12.2  

**Deliverables**:
- Documentation + examples: supervisor tears down children with **`cast`/`call`** shutdown messages; `actor_not_found` treated as already stopped.
- No conflation with §15.4.6 trap recovery path.

**Exit criteria**: Example supervisor in trials performs clean restart cycle.

---

## 5. Risk register (short)

| Risk | Mitigation |
|------|------------|
| Ingress vs mailbox deadlock or starvation | Bounded ingress + fairness tests; document batching if not full drain |
| Restart storm / livelock | Enforce `allowed_restart_count` / `restarts_time_frame` early; metrics in trials |
| Spec ambiguity: whether supervisor behavior always runs before runtime restart | Spec clarification note in implementation doc; match §15.4.12.1 literally |
| `FailureReporter` blocking | Non-blocking handoff or bounded queue with stderr fallback |

---

## 6. Suggested order of work

1. **A → B → C** (runtime spine: metadata, ingress, notifications)  
2. **F** in parallel once C produces unwind hooks, or immediately after A if reports are currently stderr-only  
3. **D** (language) when runtime needs stable types for `child_spec` / flags  
4. **E** (automatic restart)  
5. **G**, **H** (polish and documentation-heavy)

---

## 7. Completion checklist (project-wide)

- [ ] All §15.4.8–§15.4.13 acceptance bullets have a trial or unit test pointer  
- [ ] `silica-specification.md` cross-references remain valid (update if section numbers shift)  
- [ ] `actor_implementation_plan.md` updated with “Supervision: see supervisors_implementation_development_plan.md” when Phase A lands  

---

*This document is a development roadmap only; normative behavior remains defined in `silica-specification.md`.*
