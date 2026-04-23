# Supervisors Implementation — Development Plan

**Date started**: April 18, 2026  
**Last status update**: April 22, 2026  
**Status**: Phases A–D complete; Phase E compiler-side complete (through trampoline stub); Phase E runtime (e3a–e4a) and Phases F–H pending  
**Primary specification**: [silica-specification.md](silica-specification.md) — §15.1.3, §15.4 (Supervision and Fault Tolerance), §16.2.7  

**Related plans and docs**:
- [actor_implementation_plan.md](actor_implementation_plan.md) — baseline actor runtime, mailboxes, `spawn`, effects
- [silica-specification.md](silica-specification.md) §15.4.6 — trap / unwind / failure reporting (ingress is separate from `FailureReporter`)
- [silica_actor_capabilities_specification.md](silica_actor_capabilities_specification.md) — if present, align capability boundaries with supervision

---

## 0. Current Status Dashboard

### Phase-level progress

| Phase | Area | Status | Trials (supervisors_addition) |
|-------|------|--------|-------------------------------|
| A | Runtime metadata + `spawn_linked` | ✅ complete | `phase_a_supervisor_spawn_linked` |
| B | Supervision ingress + scheduler ordering (two-list functional queue: front/rear, reverse-on-empty) | ✅ complete | `phase_b_ingress_before_cast` |
| C | Exit notification construction + delivery; dead-supervisor stderr route | ✅ complete | `phase_c_child_exit_notification` |
| D | `Supervisor` trait surface + `impl T for Supervisor;` marker + `spawn_linked`/`link` behavior-only check + runtime metadata emission | ✅ complete | `phase_d_supervisor_module_compiles` |
| E | Declarative children + automatic restart | 🟡 **in progress** — compiler side through trampoline stub done; runtime (ACB child table, first-schedule hook, ingress-drain restart, escalation) and restart trials still pending | `phase_e_actor_state_probe`, `phase_e_probe` (compile-only) |
| F | `FailureReporter` + unwind path integration | ⬜ not started | — |
| G | `link`/`monitor`/cascading shutdown polish | ⬜ not started (partial scaffolding only) | — |
| H | Coordinated subtree shutdown (user-level) | ⬜ not started | — |

### Phase E task breakdown (detailed)

Canonical `init` signature (confirmed in `stdlib/Supervisor.silica`):

```
fn init(initial_supervisor_state: ActorState) -> (
    { strategy: :one_for_one | :one_for_all | :rest_for_one,
      allowed_restart_count: int64,
      restarts_time_frame: int64 },
    List[
      { id: atom,
        agent_type: atom,
        start: (int64, fn(msg: int64, state: int64) -> (:no_reply, int64)),
        restart: :permanent | :temporary | :transient,
        shutdown: int64 },
      normal
    ]
)
```

| ID   | Deliverable | Status | Notes |
|------|-------------|--------|-------|
| e1a  | Phase E probe trial with canonical `init` shape | ✅ | `trials/supervisors_addition/phase_e_probe.silica` exposes type-checker/emitter gaps |
| e1b  | Type-checker: atom-union as a record field type (`restart`, `strategy`) | ✅ | `type_checker_expressions_atoms.silica` |
| e1c  | Type-checker: `fn(...) -> ...` as a record field type (`child_spec.start`) | ✅ | `type_checker_expressions_identifiers.silica`, `type_checker_expressions.silica` |
| e1d  | Type-checker: nested record literals in tuple return (`supervisor_flags` + `List[child_spec, normal]`) | ✅ | `type_checker_expressions.silica`, `type_checker_record_types.silica`, `prims_record.silica` (paren/brace/bracket-depth tracking) |
| e1e  | Type-checker: `ActorState` recognised as a parameter type (alias for concrete actor state in `impl`) | ✅ | `type_checker_supervisor.silica` |
| e1f  | Upgrade Phase D trial to the canonical `init` signature; golden refreshed | ✅ | `phase_d_supervisor_module_compiles` |
| e1g  | Emitter: function-valued identifiers emit `ADRP/ADD` (not `MOV reg, %name`) | ✅ | `emitter/apple_silicon/terms/var.silica` (`is_ident_*`, `bare_name_is_valid_symbol`, `emit_fn_symbol_adrp_add`) |
| e1h  | Fix latent `MOV X9, SP` for function identifiers inside `sequence` blocks (function-typed var in nested aggregate make) | ✅ | `var.silica`: `emit_var_with_sp_check` / `emit_var_reg` now short-circuit on function type_name for both the source form `fn(...) -> R` and the SIR-stripped form `(...) -> R`; `type_name_is_function` + `find_top_arrow_from` helpers |
| e2a  | SIR: extract `init` symbol name + child-spec layout metadata | ✅ (reduced scope) | Bootstrap compiler is flat-namespace: every supervisor module's `init` is the bare symbol `init`, so the stub trampoline needs no extra SIR fields. A future multi-init-per-link-unit change must add an `init_symbol` field to `SIRSupervisorPhaseE`. |
| e2b  | Emit per-supervisor `_silica_supervisor_start_<T>` trampoline | ✅ (stub) | `module_linkage.silica::emit_supervisor_start_trampolines` wired via `emitter_core.silica::emit_module_asm_prelude`. Current body: `STP LR; MOV X0, #0; BL init; LDP LR; RET`. One stanza per `impl T for Supervisor;` row. Symbol links cleanly even when not yet called by runtime. |
| e3a  | Runtime: extend ACB with child-table pointer + len/cap + per-entry counters (child_ref, behavior_fn, initial_state, restart_policy, agent_type, restart_count, window_start) | ⬜ pending | Touches `emitter/apple_silicon/terms/prims/prims_actors_runtime_asm.silica` (~2300 LoC hand-written asm). Every existing ACB offset beyond the new insertion point shifts. Expect broad golden churn across **all** `actors_addition` and `supervisors_addition` trials. |
| e3b  | Runtime: invoke `_silica_supervisor_start_<T>` on first schedule of a supervisor actor | ⬜ pending | Modify `_actor_thread_main`: check supervisor flag slot, if set and trampoline-not-yet-called, `BL _silica_supervisor_start_<T>`. Expand trampoline body beyond the stub to walk the returned `(flags, children_list)` heap tuple: for each list node read `start.0` / `start.1`, call `_silica_rt_actor_spawn_linked`, append `(child_ref, behavior_fn, initial_state, restart_policy, agent_type)` to the ACB child table. |
| e3c  | Runtime: automatic restart on ingress drain — `:one_for_one` + `:permanent` | ⬜ pending | In `_silica_rt_actor_deliver_exit` (or the supervisor's ingress-drain path): look up child in table by `child_ref`; if `restart == :permanent`, `BL _silica_rt_actor_spawn_linked(initial_state, behavior_fn, agent_type, 0)`; replace the `child_ref` slot with the new ref. |
| e3d  | Runtime: `:one_for_all` and `:rest_for_one` strategies | ⬜ pending | Extends e3c: on any child exit under these strategies, iterate the child table and issue shutdown + respawn for the cascade set. |
| e3e  | Runtime: restart-frequency cap + escalation | ⬜ pending | Enforce `allowed_restart_count` / `restarts_time_frame` per-entry; on cap breach, terminate the supervisor itself (propagates via its own link to its parent, reusing Phase C). |
| e4a  | Trial: `:permanent` restart under `:one_for_one` | ⬜ pending | External-probe pattern (restarted child invisible to `main`'s local state): child increments a per-actor-type counter in a shared runtime region; `main` `cast`s, kills child abnormally, observes counter still progressing after restart. |
| e4b  | Trial: `:transient` no-restart-on-`:normal` + restart-on-abnormal | ⬜ pending | Two scenarios in one trial or two trials. |
| e4c  | Trial: `:temporary` never-restart | ⬜ pending | Counter stays frozen after child exits. |
| e4d  | Trial: restart-storm escalation (cap breach terminates supervisor) | ⬜ pending | Uses e3e. |
| e4e  | Trial: `:one_for_all` cascade restart | ⬜ pending | |
| e4f  | Trial: `:rest_for_one` cascade restart | ⬜ pending | |
| e5   | Run full `make integrate`, refresh all goldens, confirm no regressions | ⬜ pending | Current full-suite baseline is **1287 ✅ / 0 ❌** after e1h + e2b. Rerun after each of e3a–e3e. |

### Resumption checklist (to pick up Phase E runtime in a fresh session)

1. **Verify baseline**: `cd compiler/silica-compiler/src && make && cd ../trials && make integrate` should print `success: ✅✅ 1287` / `fail: ❌❌ 0`.
2. **Read the runtime asm** end-to-end: `compiler/silica-compiler/src/emitter/apple_silicon/terms/prims/prims_actors_runtime_asm.silica` — pay attention to the header comment block (lines 18–42) that documents every current ACB offset (#128 front / #136 rear / #144 thread / #152 alive flag / #192 supervisor / #200 agent_type / #208 core_id / #216 first-child / #224 sibling / #232 ingress-front / #240 ingress-rear / #248 ingress-depth). The ACB is a 256-byte `calloc` today — enlarge it carefully.
3. **Pick insertion point** for child-table fields (`#256` onward is the safest — append, don't insert between existing fields — but you must also bump every `MOV w1, #256` allocation size and every stride that assumes 256-byte ACB).
4. **Extend the trampoline** (currently `module_linkage.silica::format_one_supervisor_trampoline`) from stub to full walker: the stub already establishes the symbol and call-site; add body that reads the return-tuple layout documented below.
5. **Tuple-return layout for `init`** (observed in `phase_d_supervisor_module_compiles.sams`, fields at bytes 0–79 of the 80-byte heap block): flags record at `+0..+23` (strategy atom @ +0, allowed_restart_count i64 @ +8, restarts_time_frame i64 @ +16); children-list head pointer at `+24` (list cons nodes: payload_ptr @ +0, tail_ptr @ +8, length/terminator encoding per `List` runtime). Each child_spec record is 72 bytes with `start.0` (initial_state i64) @ +16 and `start.1` (behavior fn ptr) @ +24; `restart` atom @ +56; `shutdown` i64 @ +64; `id` and `agent_type` atoms @ +0, +8. **Confirm offsets before wiring** — use the `.sams` output as ground truth.
6. **Stage goldens in order** (commit between each): e3a (ACB size change only) → e3b (first-schedule hook) → e3c (:one_for_one + :permanent) → e4a trial → e3d/e3e → e4b–f.

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

**Status (cross-reference)**: See the **Phase E task breakdown** in §0 for the e1a–e5 ID grid; `e1a`–`e1h` + `e2a`/`e2b` are complete; `e3a`–`e5` are pending.

**Deliverables**:
- On supervisor start: runtime calls **`init`**, spawns each **`child_spec`** via internal **`spawn_linked`** equivalent, stores refs in **internal child table** (§15.4.13.1).
- On child exit: if **`restart`** and **`strategy`** permit, runtime performs restart protocol (§15.4.12.1)—**without** involving supervisor behavior for the mechanical respawn; supervisor behavior still receives ingress notification if spec requires visibility for policy (clarify in implementation doc: spec says runtime applies strategy; supervisor may still need notification for logging—align with §15.4.12.1 wording).
- **Escalation**: restart frequency caps; supervisor termination propagates via link (§15.4.13.2 end).
- **Dynamic children** (§15.4.13.3): no automatic restart table; ingress notifications only.

**Exit criteria**: Trial with `:permanent` vs `:transient` vs `:temporary`; `:one_for_one` minimum before `:one_for_all` / `:rest_for_one` if phased.

**Files touched so far (reference for resumption)**:
- Compiler type-checker: `src/type_checker/expressions/type_checker_expressions.silica`, `..._atoms.silica`, `..._identifiers.silica`, `..._record_types.silica`, `src/type_checker/type_checker_supervisor.silica`
- Compiler emitter: `src/emitter/apple_silicon/terms/var.silica` (function-typed var ADRP/ADD), `src/emitter/apple_silicon/terms/prims/prims_record.silica` (paren-depth fix), `src/emitter/apple_silicon/module_linkage.silica` (`emit_supervisor_start_trampolines`), `src/emitter/apple_silicon/emitter_core.silica` (wire trampolines into prelude)
- SIR generator (unchanged for e2a stub): `src/sir_generator/declarations/traits.silica`, `src/sir_generator/sir_ast.silica` already carry `SIRSupervisorPhaseE { actor_type, init_return_sir }`
- Trials: `trials/supervisors_addition/phase_e_probe.silica` (compile-only probe), `phase_e_actor_state_probe.silica` (compile-only), `phase_d_supervisor_module_compiles.silica` (upgraded to canonical `init` signature)
- Runtime (**not yet touched**; starting point for e3a): `src/emitter/apple_silicon/terms/prims/prims_actors_runtime_asm.silica`

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

- [x] Phase A landed (`phase_a_supervisor_spawn_linked` trial)  
- [x] Phase B landed (`phase_b_ingress_before_cast` trial; two-list functional-queue mailbox)  
- [x] Phase C landed (`phase_c_child_exit_notification` trial; dead-supervisor stderr route)  
- [x] Phase D landed (`phase_d_supervisor_module_compiles` trial; `Supervisor` trait + `impl T for Supervisor;` marker recognised)  
- [ ] Phase E landed — compiler side through trampoline stub done (e1a–e1h + e2a/e2b); runtime side (e3a–e3e) + restart trials (e4a–e4f) + final integrate refresh (e5) still outstanding. See §0 for the task grid.  
- [ ] Phase F landed (`FailureReporter`)  
- [ ] Phase G landed (`link` / `monitor` / cascading shutdown polish)  
- [ ] Phase H landed (coordinated subtree shutdown examples)  
- [ ] All §15.4.8–§15.4.13 acceptance bullets have a trial or unit test pointer  
- [ ] `silica-specification.md` cross-references remain valid (update if section numbers shift)  
- [x] `actor_implementation_plan.md` updated with “Supervision: see supervisors_implementation_development_plan.md” when Phase A lands  

**Current baseline** (as of last status update): full `make integrate` under `compiler/silica-compiler/trials/` reports **success: 1287 / fail: 0**. Re-run after every e3a–e3e landing and refresh goldens per stage.

---

*This document is a development roadmap only; normative behavior remains defined in `silica-specification.md`.*
