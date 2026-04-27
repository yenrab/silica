# Supervisors Implementation — Development Plan

**Date started**: April 18, 2026  
**Last status update**: April 27, 2026  
**Status**: Phases A–D complete; Phase E runtime **e3a** (heap child table) + **e3b** (first-schedule hook + materialize) + **e3b2** (`start_child`) + **e3c** (automatic restart `:one_for_one`/`:permanent`) + **e3d** (`:one_for_all` / `:rest_for_one` strategies) + **e3e** (restart-frequency cap + escalation) + **e3f** (tombstones/reuse) complete; trial **e4a** (`:permanent` + `:one_for_one`) + **e4b** (`:transient` no-restart-on-`:normal` / restart-on-abnormal) complete; **e4c–e5** and Phases F–H pending **— child table: unified heap design for declarative + `start_child` (see §0 Phase E child-table model, silica-specification §15.4.13.3)**  
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
| E | Declarative + **dynamic** supervised children, automatic restart (heap child table) | 🟡 **in progress** — runtime **e3a–e3f** complete; restart behavior trials **e4a–e4g** and final integrate **e5** still pending | `phase_e_actor_state_probe`, `phase_e_probe` (compile-only) |
| F | `FailureReporter` + unwind path integration | ⬜ not started | — |
| G | `link`/`monitor`/cascading shutdown polish | ⬜ not started (partial scaffolding only) | — |
| H | Coordinated subtree shutdown (user-level) | ⬜ not started | — |

### Phase E — unified heap child table (design snapshot)

Normative detail is in **silica-specification.md §15.4.13.3**. Summary for implementers:

- **One** internal child table per supervisor: **heap-allocated**, **growable** (`realloc` or equivalent); the **ACB** stores a **pointer** (and len/cap metadata), not the full table inline in a fixed-size block.
- **Declarative** children: returned from **`init`**; trampoline walks the list and **appends** one row per `child_spec`.
- **Dynamic** supervised children (OTP-style): **`start_child(spec)`** appends a row and spawns; **same** `supervisor_flags`, restart, and escalation rules as declarative rows.
- **Bare `spawn_linked`**: still creates a **link** and **ingress** notifications; **no** table row ⇒ **no** automatic restarts (behavior may implement policy manually).

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
| e3a  | Runtime: ACB **fields for heap child table** (pointer + len + cap, or equivalent) + **out-of-line** buffer on the heap; per-row fields (see below). **Realloc** when cap exhausted. | ✅ | **Do not** require the full table to live in the fixed 256 B block — only **anchor** in ACB (and bump allocation size for new slots only, per header comment in `prims_actors_runtime_asm.silica`). Implemented: 280 B ACB, anchors **#256/#264/#272**, `prims_actors_child_table_asm.silica` (`_silica_rt_child_table_free`, `_silica_rt_child_table_ensure_min_cap`), 80 B row layout; spawn path init + thread exit free. Table rows: `child_ref`, `child_spec` data needed for restart (`start` pair, `restart`, `agent_type`, `id`, `shutdown`, …), escalation **counters** per spec. Goldens refreshed (`actors_addition`, `supervisors_addition`, `cpu_discovery_and_spawn_pinning`). |
| e3b  | Runtime: invoke `_silica_supervisor_start_<T>` on first schedule of a supervisor actor | ✅ | ACB 296 B (+16: **#280** trampoline ptr, **#288** ran flag). `_actor_thread_main` hook: `LDR [#280]; CBZ skip; LDR [#288]; CBNZ skip; BLR; STR 1`. SIR value tag `[concurrency]\|e3b:<T>` (sir_generator `terms.silica`); emitter `prims_actors.silica` `ADRP/ADD + STR [dest, #280]` on spawn/spawn_linked of supervisor state types. Trampoline (`module_linkage.silica`): `LDR X0, [ACB]; BL init; BL _silica_rt_supervisor_materialize_init_children(ACB, tuple_ptr)`. Materialize (`prims_actors_child_table_asm.silica`): walks cons list at tuple+24, `spawn_linked` each child, appends 80 B row via `ensure_min_cap`. TypeContext carries supervisor CSV via magic symbol `__silica_e3b_sup_csv`. Goldens refreshed. Baseline **1289** ✅ / 0 ❌ (1 skip: cpu_discovery). |
| e3b2 | Compiler/runtime: **`start_child(spec) -> actor_ref`** for supervisors | ✅ | Type checker: `start_child` in `is_actor_concurrency_builtin`, `check_start_child_call` (1-arg, behavior-only, returns `actor_ref`). SIR: `build_start_child_prim` (unary, `[concurrency]`). Emitter: `is_actor_prim`, `runtime_label` → `_silica_rt_supervisor_start_child`, dispatched via `emit_runtime_unary`. Runtime (`prims_actors_child_table_asm.silica`): reads TLS for supervisor ACB, `spawn_linked(start.0, start.1, agent_type, 0)`, appends 80 B row via `ensure_min_cap`, returns child `actor_ref` in X0. `stdlib/Supervisor.silica` exports `start_child/1`. Goldens refreshed. Baseline **1289** ✅ / 0 ❌ (1 skip). |
| e3c  | Runtime: automatic restart on ingress drain — `:one_for_one` + `:permanent` | ✅ | `_actor_thread_main` `LBB1_free`: when `w21==1` (ingress), loads payload child_ref `[x20+8]+0`, calls `_silica_rt_supervisor_maybe_restart(child_ref, ACB)`. New routine (`prims_actors_child_table_asm.silica`): scans heap table rows (child_ref@+0), on match loads restart atom @+40, `strcmp` vs embedded `L_atom_permanent` (`:permanent`); if equal, `spawn_linked(start.0, start.1, agent_type, 0)` and stores new child_ref into row. `:one_for_one` (single-child restart). Goldens refreshed. Baseline **1289** ✅ / 0 ❌ (1 skip). |
| e3d  | Runtime: `:one_for_all` and `:rest_for_one` strategies | ✅ | ACB extended 296→**304 B**; new field **#296** = strategy atom ptr (set by materialize from `init` tuple+0, zeroed on spawn). `_silica_rt_supervisor_maybe_restart` rewritten: after finding dead child + confirming `:permanent` restart, loads strategy from `[ACB, #296]` and `strcmp`-dispatches: **`:one_for_one`** (default) — respawn single child; **`:one_for_all`** — kill all OTHER children (lock mutex, alive=0, signal cond, unlock on each sibling ACB), then respawn ALL in table order; **`:rest_for_one`** — kill children AFTER dead child index, then respawn dead + rest. Embedded atom strings `L_atom_one_for_all` / `L_atom_rest_for_one` via `.byte`. Frame expanded to 96 B (x19–x28, x29/x30) for kill/respawn loop registers. Goldens refreshed (28 actor + supervisor files). Baseline **1289** ✅ / 0 ❌ (1 skip). |
| e3e  | Runtime: restart-frequency cap + escalation | ✅ | ACB extended 304→**336 B**; new fields **#304** `allowed_restart_count` (i64), **#312** `restarts_time_frame` (i64, seconds), **#320** `restart_count` (current window), **#328** `window_start_time` (seconds from `time(NULL)`). Materialize stores #304/#312 from `init` tuple flags (+8, +16). In `_silica_rt_supervisor_maybe_restart`: after `:permanent` check, if both #304 and #312 are 0 → unlimited (skip cap). Otherwise calls `_time(NULL)` for current seconds; if window_start is 0 or elapsed > time_frame → reset window (start=now, count=1); else increment count. If `count > allowed_restart_count` → **escalate**: lock supervisor mutex, set alive=0, signal cond, unlock → supervisor exits loop → Phase C link propagation. Goldens refreshed (28 files). Baseline **1289** ✅ / 0 ❌ (1 skip). |
| e3f  | Runtime: **remove** or repurpose **supervised** child row when child is removed (optional API later) / document tombstones | ✅ | Tombstone model implemented in `prims_actors_child_table_asm.silica`: `child_ref == 0` marks inactive rows; `start_child` reuses the first tombstone before growing; cascade restart loops skip tombstones. Bare `spawn_linked` children **without** a row need no change (§15.4.13.3). |
| e4a  | Trial: `:permanent` restart under `:one_for_one` | ✅ | `trials/supervisors_addition/phase_e4a_permanent_one_for_one.silica`: supervisor prints probe; `main` holds only supervisor ref; `sup_beh` uses `child_table_first_ref` + `call`, `remove_actor`, ingress drain, third `call` returns **msg+0** (300) after restart. Runtime fixes: value-flat list payload loads `restart`/`shutdown` at **+32/+40** in materialize; `_silica_rt_supervisor_maybe_restart` compares **atom indices** (`lexeme_to_index_first_added_order`) instead of `strcmp`; no `cbz` on restart atom (index 0 is valid). |
| e4b  | Trial: `:transient` no-restart-on-`:normal` + restart-on-abnormal | ✅ | `trials/supervisors_addition/phase_e4b_transient.silica`: one declarative `:transient` child + one dynamic via `start_child`. **Normal exit** (`remove_actor`, `reason_tag=0`) ⇒ no restart, row tombstoned, `child_table_first_ref` skips it. **Abnormal exit** (`kill_abnormal`, `reason_tag=1`) ⇒ restart in place; third `call` returns msg+0 (1300) on the fresh state slot. Runtime: ACB extended **336→344 B** (`#336` `pending_exit_reason_tag`); new helper `_silica_rt_actor_kill_abnormal` flips `#336=1` then alive=0 + cond_signal; `_silica_rt_actor_deliver_exit` reads `#336` into payload `+8`; `_silica_rt_supervisor_maybe_restart` takes `reason_tag` in **X2/x28** with `:permanent` (always) / `:transient` (skip iff `x28==0`) dispatch, fall-through to tombstone for unmatched policy. `_silica_rt_child_table_first_ref` now scans rows and returns the first non-tombstone `child_ref`. `_silica_rt_supervisor_start_child` ABI corrected to value-flat 48 B spec (`restart@+32, shutdown@+40`) — earlier 72 B comment was never matched by the emitter's `record_make`. Compiler: `kill_abnormal/1` wired through type checker / SIR / effect / emitter as `[concurrency]` builtin. Emitter dealloc bug fix: actor-reply behavior fns set `_tuple_sret=X20`, which was suppressing the `ADD SP` that balances an intermediate `record_make` let-binding (`record_make` always SUBs SP — see record-emit comment). `term_emitter.silica` now special-cases `term.inner.kind==6 && name=="record_make"` so the dealloc bytes are computed regardless of `_tuple_sret`; without this, behavior fns building a literal record (e.g. `start_child(spec)`) returned with `SP` 48 B too low and crashed the LDP epilogue with SIGBUS. Goldens refreshed (actors_addition: 25 .ascomp; supervisors_addition: 6 .ascomp + new `phase_e4b_transient.{ascomp,scout}`). Baseline **1295** ✅ / 0 ❌. |
| e4c  | Trial: `:temporary` never-restart | ⬜ pending | Counter stays frozen after child exits. |
| e4d  | Trial: restart-storm escalation (cap breach terminates supervisor) | ⬜ pending | Uses e3e. |
| e4e  | Trial: `:one_for_all` cascade restart | ⬜ pending | |
| e4f  | Trial: `:rest_for_one` cascade restart | ⬜ pending | |
| e4g  | Trial: **`start_child` dynamic row** + `:permanent` restart under `:one_for_one` (heap table row *not* from `init`) | ⬜ pending | Assert same restart machinery as declarative; optional second scenario with mixed `init` + `start_child` children. |
| e5   | Run full `make integrate`, refresh all goldens, confirm no regressions | ⬜ pending | Current full-suite baseline is **1289 ✅ / 0 ❌** after e3a golden refresh. Rerun after each of e3b–e3e, e3b2, and trials. |

### Resumption checklist (to pick up Phase E runtime in a fresh session)

1. **Verify baseline**: `cd compiler/silica-compiler/src && make && cd ../trials && make integrate` should print `success: ✅✅ 1289` / `fail: ❌❌ 1` (1 = cpu_discovery skip, 0 real failures).
2. **Read the runtime asm** end-to-end: `compiler/silica-compiler/src/emitter/apple_silicon/terms/prims/prims_actors_runtime_asm.silica` — ACB offsets: #128 front / #136 rear / #144 thread / #152 alive / #192 supervisor / … / #248 ingress-depth / **#256–#272 heap child table anchors** / **#280 trampoline ptr** / **#288 ran flag** / **#296 strategy atom ptr** (e3d) / **#304 allowed_restart_count** / **#312 restarts_time_frame** / **#320 restart_count** / **#328 window_start_time** (e3e). The ACB is a **336**-byte `calloc` (e3e).
3. **ACB vs heap child buffer**: **e3a** adds anchor fields **#256** ptr, **#264** len, **#272** cap; the **row array** is `calloc`/`realloc` in `prims_actors_child_table_asm.silica` **out of line**. **e3b** adds #280/#288 for first-schedule trampoline. **e3d** adds #296 for strategy atom. **e3e** adds #304/#312/#320/#328 for restart frequency cap.
4. **Trampoline** (`module_linkage.silica::format_one_supervisor_trampoline`) now calls `init` with `LDR X0, [ACB]` (initial state) then `BL _silica_rt_supervisor_materialize_init_children(ACB, tuple_ptr)`. Materialize walks cons list at tuple+24, `spawn_linked` each child, appends 80 B row.
5. **Tuple-return layout for `init`** (observed in `phase_d_supervisor_module_compiles.sams`, fields at bytes 0–79 of the 80-byte heap block): flags record at `+0..+23` (strategy atom @ +0, allowed_restart_count i64 @ +8, restarts_time_frame i64 @ +16); children-list head pointer at `+24` (list cons nodes: payload 72 B inline, tail at +72). Each child_spec: `id` @ +0, `agent_type` @ +8, `start.0` @ +16, `start.1` @ +24, `restart` @ +40, `shutdown` @ +48. **`start_child`** must append a row with the same logical fields (spec §15.4.13.3).
6. **Stage goldens in order** (commit between each): e3b2 (`start_child` + one dynamic trial pointer) → e3c → e4a / e4g → e3d / e3e → e4b–e4f.

---

## 1. Goal

Implement **OTP-style supervision** as specified: supervisor actors implementing the **`Supervisor` trait**, **`spawn_linked`** and **`start_child(spec)`** (§15.4.13.3) as the two parent–child bindings for **ad-hoc** vs **supervised (table)** children, a **heap-allocated, growable internal child table**, a **high-priority supervision ingress** for exit notifications, structured **failure payloads**, **declarative and dynamic** `child_spec` restart behaviour under **`supervisor_flags`**, independent **unwind reports** to **`FailureReporter`**, and correct interaction with **`link`**, **`monitor`**, and cascading shutdown.

The **unified** model is: **all** children that participate in the **runtime** restart protocol (§15.4.12.1) have a **row** in the same internal table, whether the row was created from **`init`** or from **`start_child`**. **Bare `spawn_linked`** without a row remains valid for **ingress only**; no automatic restarts (§15.4.13.3). Out of scope unless explicitly pulled in: full **`:one_for_all` / `:rest_for_one`** in the first tranche of Phase E (may still be deferred per milestone), **FFI fault containment** (§15.4.13.5) beyond hooks for ordinary actor death, **`delete_child` / row removal** (see e3f) until a stable API is needed, **migrate_actor** and topology that depend on unimplemented `move()` semantics.

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
| `child_spec`, `supervisor_flags`, `start_child`, internal heap child table | §15.4.13.1–§15.4.13.3 |
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

### Phase E — Declarative + dynamic supervised children and automatic restart

**Spec**: §15.4.12.1, §15.4.12.3, §15.4.13.2, **§15.4.13.3 (heap child table, `start_child`)**  

**Status (cross-reference)**: See the **Phase E task breakdown** in §0 for the e1a–e5 ID grid; `e1a`–`e1h` + `e2a`/`e2b` + **`e3a`**–**`e3f`** (including **e3b2**) are complete; `e4a`–`e5` (including **e4g**) are pending.

**Deliverables**:
- **Internal child table** (§15.4.13.3): **heap-allocated**, growable; ACB holds **pointer + len + cap** (or equivalent), not a large inline fixed array. Same restart machinery for every **table** row.
- On supervisor start: runtime calls **`init`**, spawns each **declarative** `child_spec` via internal **`spawn_linked`**, **appends** a **row** to the child table. **`supervisor_flags`** from `init` apply to **all** table rows.
- **Dynamic** supervised children: **`start_child(spec)`** appends a row and spawns; behaviour matches **`supervisor:start_child/2`** in OTP (one operation). Implement **e3b2** and wire `stdlib` / compiler as in §0.
- On child exit: if the **row**’s **`restart`** and **`strategy`** permit, runtime performs the restart protocol (§15.4.12.1) **for any row** (declarative or `start_child`)—mechanical respawn without supervisor **behavior** participation; **ingress** still delivers the structured notification. **Escalation** and **bare `spawn_linked`** (no row) per §15.4.13.3.
- **Optional**: `delete_child` / row removal deferred to **e3f** or later.

**Exit criteria**: Trials e4a–e4c for restart modes; **e4g** for dynamic `start_child` + restart; **`:one_for_one` minimum** before full **`:one_for_all` / `:rest_for_one`** (e3d / e4e / e4f) if still phased.

**Files touched so far (reference for resumption)**:
- Compiler type-checker: `src/type_checker/expressions/type_checker_expressions.silica`, `..._atoms.silica`, `..._identifiers.silica`, `..._record_types.silica`, `src/type_checker/type_checker_supervisor.silica`
- Compiler emitter: `src/emitter/apple_silicon/terms/var.silica` (function-typed var ADRP/ADD), `src/emitter/apple_silicon/terms/prims/prims_record.silica` (paren-depth fix), `src/emitter/apple_silicon/module_linkage.silica` (`emit_supervisor_start_trampolines`), `src/emitter/apple_silicon/emitter_core.silica` (wire trampolines into prelude)
- SIR generator (unchanged for e2a stub): `src/sir_generator/declarations/traits.silica`, `src/sir_generator/sir_ast.silica` already carry `SIRSupervisorPhaseE { actor_type, init_return_sir }` — may need extension for `start_child` symbol / supervisor-only intrinsics
- `stdlib/Supervisor.silica` — add **`start_child`** per §15.4.13.3 (e3b2)
- Trials: `trials/supervisors_addition/phase_e_probe.silica` (compile-only probe), `phase_e_actor_state_probe.silica` (compile-only), `phase_d_supervisor_module_compiles.silica` (upgraded to canonical `init` signature); add **e4g** (and others per §0)
- Runtime — **e3a** child table: `src/emitter/apple_silicon/terms/prims/prims_actors_child_table_asm.silica`, `src/emitter/apple_silicon/terms/prims/prims_actors_runtime_asm.silica` (ACB size, init, exit `free`); next: **e3b** trampoline + first schedule.

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
| **Heap child table: realloc fail, ACB/heap out of sync** | Validate cap growth path; on OOM, documented containment/abort per §15.4.4; do not leave dangling pointer in ACB |
| **`init` order vs `start_child` order for strategies** | Spec §15.4.13.3: stable combined ordering; document in e3d implementation note |
| `FailureReporter` blocking | Non-blocking handoff or bounded queue with stderr fallback |

---

## 6. Suggested order of work

1. **A → B → C** (runtime spine: metadata, ingress, notifications)  
2. **F** in parallel once C produces unwind hooks, or immediately after A if reports are currently stderr-only  
3. **D** (language) when runtime needs stable types for `child_spec` / flags  
4. **E** (heap child table, **`init` + `start_child`**, automatic restart — §15.4.13.3)  
5. **G**, **H** (polish and documentation-heavy)

---

## 7. Completion checklist (project-wide)

- [x] Phase A landed (`phase_a_supervisor_spawn_linked` trial)  
- [x] Phase B landed (`phase_b_ingress_before_cast` trial; two-list functional-queue mailbox)  
- [x] Phase C landed (`phase_c_child_exit_notification` trial; dead-supervisor stderr route)  
- [x] Phase D landed (`phase_d_supervisor_module_compiles` trial; `Supervisor` trait + `impl T for Supervisor;` marker recognised)  
- [ ] Phase E landed — compiler side through trampoline stub done (e1a–e1h + e2a/e2b); runtime **e3a–e3f** + **`start_child` (e3b2)** done; restart + dynamic trials (e4a–e4g) + final integrate (e5) still outstanding. See §0 for the task grid.  
- [ ] Phase F landed (`FailureReporter`)  
- [ ] Phase G landed (`link` / `monitor` / cascading shutdown polish)  
- [ ] Phase H landed (coordinated subtree shutdown examples)  
- [ ] All §15.4.8–§15.4.13 acceptance bullets have a trial or unit test pointer  
- [ ] `silica-specification.md` cross-references remain valid (update if section numbers shift)  
- [x] `actor_implementation_plan.md` updated with “Supervision: see supervisors_implementation_development_plan.md” when Phase A lands  

**Current baseline** (as of last status update): full `make integrate` under `compiler/silica-compiler/trials/` reports **success: 1289 / fail: 0** (post-e3a). Re-run after every e3b–e3e landing and refresh goldens per stage.

---

*This document is a development roadmap only; normative behavior remains defined in `silica-specification.md`.*
