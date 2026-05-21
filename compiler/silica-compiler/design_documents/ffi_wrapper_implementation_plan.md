# Silica Compiler FFI — Implementation Plan

**Date**: May 21, 2026  
**Primary specification**: [silica_ffi_wrapper_specification.md](silica_ffi_wrapper_specification.md)  
**Code organization**: [silica-compiler-code-organization.md](silica-compiler-code-organization.md)

This plan adds outbound FFI support to `silica-compiler`. Each phase is **independently testable** before the next phase begins. Phases enforce safety and metadata rules before enabling runtime foreign calls.

---

## 0. Current State

| Area | Status |
| ---- | ------ |
| `foreign c_wrapper` syntax | Not implemented |
| `wrapper_meta` / per-binding `meta` | Not implemented |
| `external_danger` effect | Not implemented |
| `dangerous_*` module naming propagation | Not implemented |
| Cast-only behavior enforcement | Not implemented |
| FFI worker placement rules | Not implemented |
| Structural taint for `dangerous_*` returns | Not implemented |
| Sidecar `.meta` loader | Not implemented |
| Foreign call emitter / runtime | Not implemented |
| Prebuilt wrapper library link step | Not implemented |

**Existing infrastructure to reuse**: actor `spawn` / `cast`, sequence blocks, effect checking (`proc[concurrency]`, `device_io`, …), module import graph, trial harness (`trials/`, `.golden_fail`, `.scout` / `.ascomp`).

---

## 1. Target Architecture (Fixed Decisions)

These decisions are **not** revisited during implementation. See spec §16.1.

| Topic | Decision |
| ----- | -------- |
| Call transport | Cast-mediated: client actor → FFI worker actor → FFI result cast |
| Client / worker behaviors | Cast-only (no `call`-reply shape) |
| `external_danger` placement | Only inside FFI worker behavior passed directly to `spawn` |
| Dangerous call scope | Every call to any `dangerous_*` module function |
| Adapter exports | Must have a Silica body; raw `foreign c_wrapper` never exported |
| Strings | Two-layer: raw bindings use ptr+len; adapters use `string` (in and out) |
| Sidecar discovery | Explicit `wrapper_meta` / `meta` in Silica source (option 5) |
| Toolchain validation | Silica checks + sidecar load + **link-time symbol check only** (no C parsing) |
| Build | Prebuilt static libraries under `dangerous_exposure_source/lib/` |
| `blocking` metadata | Not used (architecturally non-blocking via cast model) |
| De-taint | Strict structural taint in v1 |

---

## 2. Non-Goals

- Inbound interop (C → Silica, callbacks, trampolines).
- C header / wrapper source parsing or compilation in the Silica build.
- Dynamic linking and `foreign package { … }` manifest (spec §14.3 — future).
- Validator-based de-taint.
- Deferred mechanical wrapper-side checks listed in spec §15.2 (until C parsing exists).

---

## 3. Fixture Layout (Created in Phase 0)

All FFI trials share this layout:

```text
trials/ffi_addition/
  fixtures/
    dangerous_exposure_source/
      legacy/silica_legacy_math_wrapper.meta
      text/silica_text_wrapper.meta
      lib/libsilica_legacy_math.a      # int64 add
      lib/libsilica_text.a             # echo: string in → string out
    src/                               # C sources used to build .a files (not compiled by Silica)
  README.md
  Makefile                             # builds fixture archives only
```

Trial categories:

| Subdirectory | Purpose |
| ------------ | ------- |
| `parse_addition/` | Phases 1–2: parse / AST only |
| `module_addition/` | Phase 3: naming + exports |
| `metadata_addition/` | Phase 4: sidecar + path errors |
| `placement_addition/` | Phases 5–6: cast / worker / `external_danger` |
| `taint_addition/` | Phase 7: taint + boundaries |
| `abi_addition/` | Phase 8: type matrix failures |
| `runtime_addition/` | Phases 9–10: link + execute |
| `error_enforcement_addition/` | Golden failures (may alias existing dir pattern) |

**Phase test command (convention)**:

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-N
```

Each `phase-N` target runs only the trials for phases `0…N` and must pass before starting phase `N+1`.

---

## 4. Phase Plan

### Phase 0 — Fixture Archives and Trial Harness

**Goal**: Provide linkable C wrapper fixtures and a Makefile gate independent of compiler FFI support.

**Scope**

- Add `trials/ffi_addition/fixtures/` with:
  - `silica_legacy_math_add_int64` (scalar),
  - `silica_text_echo` (string in / string out via ptr+len at C ABI),
  - matching `.meta` sidecar files (no `blocking` field).
- Add `trials/ffi_addition/Makefile` targets: `fixtures`, `phase-0`.
- Document fixture rebuild in `trials/ffi_addition/README.md`.

**Compiler touchpoints**: none.

**Independent test**

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-0
```

Verifies archives exist and `nm` / `llvm-nm` exposes expected symbols.

**Exit criteria**: `libsilica_legacy_math.a` and `libsilica_text.a` build on macOS AArch64.

---

### Phase 1 — Parse Surface

**Goal**: Lexer and parser accept/reject FFI source forms without codegen.

**Scope**

- Lexer: tokens `foreign`, `c_wrapper`, `wrapper_meta`, `meta` (if not keyword-aligned with existing lexer).
- Parser (`parser/declarations/` — new `parser_declarations_foreign.silica` or extend modules):
  - `wrapper_meta "path";`
  - `foreign c_wrapper "sym" fn …;`
  - optional `meta "path"` clause on foreign decl.
  - `external_danger` in sequence effect lists.
- AST nodes in `ast/ast_declarations.silica`:
  - `WrapperMetaDecl`, `ForeignBindingDecl` (symbol, optional meta path, fn signature).
- Reject malformed foreign forms at parse time.

**Independent test**

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-1
```

Trials in `parse_addition/`:

| Trial | Expect |
| ----- | ------ |
| `foreign_decl_valid.silica` | parse OK |
| `wrapper_meta_valid.silica` | parse OK |
| `foreign_decl_meta_override.silica` | parse OK |
| `foreign_decl_malformed.golden_fail` | parse error |

**Exit criteria**: parser tests pass; existing non-FFI trials still parse.

---

### Phase 2 — AST / SIR Plumbing

**Goal**: Foreign declarations and `wrapper_meta` survive to module and type checking.

**Scope**

- SIR / module representation carries:
  - foreign binding table per module (local name → symbol string),
  - list of referenced sidecar paths,
  - `dangerous_*` module flag.
- Debug pretty-print includes foreign nodes.
- No semantic checks yet beyond parse.

**Independent test**

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-2
```

Trials compile through parse + AST dump / scout snapshot (`.scout` lists foreign decl names).

**Exit criteria**: scout output shows foreign symbols for valid modules.

---

### Phase 3 — Module Naming, Exports, and Sidecar Path Rules

**Goal**: Enforce `dangerous_*` naming, adapter export rules, and `wrapper_meta` path constraints at compile time.

**Scope**

- Module checker (`type_checker` / module graph — extend existing module pass):
  - `DangerousModuleNameError`, `DangerousDependencyNamingError` (transitive to root),
  - raw foreign binding export forbidden,
  - exported fn from `dangerous_*` must have body (`ExportAdapterBodyError` or reuse spec name),
  - `MissingWrapperMetaError`, `WrapperMetaPathError`.
- No sidecar file loading yet — path rules only (literal, under `dangerous_exposure_source/`).

**Independent test**

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-3
```

Trials in `module_addition/`:

| Trial | Expect |
| ----- | ------ |
| `dangerous_naming_valid.silica` | compile OK |
| `dangerous_dependency_naming_fail.golden_fail` | `DangerousDependencyNamingError` |
| `export_raw_foreign_fail.golden_fail` | export error |
| `missing_wrapper_meta_fail.golden_fail` | `MissingWrapperMetaError` |
| `wrapper_meta_outside_tree_fail.golden_fail` | `WrapperMetaPathError` |

**Exit criteria**: all golden failures match; valid dangerous modules compile to type-check entry.

---

### Phase 4 — Sidecar Metadata Loader

**Goal**: Load explicitly referenced `.meta` files and validate required entries at compile time.

**Scope**

- New module `src/ffi/ffi_sidecar_loader.silica` (or `parser/…` if preferred):
  - parse minimal sidecar syntax: `link_library`, `wrapper { symbol, result, error_domain, … }`,
  - index wrappers by symbol string,
  - associate each module foreign binding with sidecar entry from `wrapper_meta` / `meta`.
- Errors:
  - missing sidecar file on disk,
  - missing `wrapper` entry for used symbol,
  - missing `link_library` in sidecar file,
  - missing required fields (`result`, `error_domain` when applicable).
- Record link libraries for driver (Phase 10).

**Independent test**

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-4
```

Trials in `metadata_addition/` using fixture sidecars:

| Trial | Expect |
| ----- | ------ |
| `sidecar_match_valid.silica` | compile OK through metadata pass |
| `sidecar_missing_entry_fail.golden_fail` | missing wrapper entry |
| `sidecar_missing_file_fail.golden_fail` | file not found |
| `sidecar_missing_link_library_fail.golden_fail` | no `link_library` |

**Exit criteria**: loader tests pass; no emitter changes required.

---

### Phase 5 — Cast-Mediated Placement and Cast-Only Behaviors

**Goal**: Enforce §4 cast/worker model at parser + effect checker.

**Scope**

- Identify FFI worker behaviors: function passed directly to `spawn` containing `external_danger` sequence.
- Identify cast-only behaviors: behavior fn must not use `call`-reply return shape (extend actor behavior validation).
- Errors:
  - `ExternalDangerClientBehaviorError` — client initiating foreign work without cast-only shape,
  - `ExternalDangerPlacementError` — `external_danger` not in FFI worker behavior,
  - `DangerousModuleCallError` — dangerous call outside worker `external_danger` sequence,
  - direct `dangerous_*` call from application actor behavior forbidden.
- Permit `cast` inside `external_danger` sequence for FFI result delivery (with `concurrency`).

**Independent test**

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-5
```

Trials in `placement_addition/`:

| Trial | Expect |
| ----- | ------ |
| `ffi_worker_valid.silica` | compile OK |
| `external_danger_in_main_fail.golden_fail` | placement error |
| `direct_dangerous_call_fail.golden_fail` | `DangerousModuleCallError` |
| `client_not_cast_only_fail.golden_fail` | client behavior error |

**Exit criteria**: positive cast/worker trial parses and type-checks; negatives match goldens.

---

### Phase 6 — Structural Taint and Message Boundaries

**Goal**: Track external-danger-touched data and enforce `produces pure` / cast boundaries.

**Scope**

- Type checker taint bit on values from `dangerous_*` module calls (structural propagation).
- Enforce:
  - `ExternalDangerSequenceResultError` — taint in `produces pure`,
  - `ExternalDangerMessageBoundaryError` — tainted regions in ordinary cast/call payloads,
  - **FFI result cast exception** — worker may send tainted payloads only on designated FFI result casts inside `external_danger` sequence,
  - `ExternalDangerRegionEscapeError`,
  - `ExternalDangerRestrictedEffectError`.
- No de-taint paths in v1.

**Independent test**

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-6
```

Trials in `taint_addition/` (compile-only negative goldens + one structurally valid worker/client pair).

**Exit criteria**: each spec §7 failure has a golden; valid worker delivers via FFI result cast only.

---

### Phase 7 — Silica-Side ABI Checker (No C Parsing)

**Goal**: Validate foreign **Silica** signatures and adapter/raw layering before codegen.

**Scope**

- New `src/ffi/ffi_abi_checker.silica`:
  - allowed scalar types and boolean as `uint8`,
  - raw foreign decls must not use Silica `string` (ptr+len instead),
  - adapter wrappers may use `string` when raw uses ptr+len,
  - adapter return `string` paired with raw return ptr+len fields (symmetric two-layer),
  - reject raw pointers / void shapes in Silica declarations,
  - max eight Silica-level arguments after lowering (string counts as one Silica arg),
  - sidecar `result` / `error_domain` consistency with Silica return shape for tagged results.
- Cross-check foreign binding against loaded sidecar entry (not against C headers).

**Independent test**

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-7
```

Trials in `abi_addition/`:

| Trial | Expect |
| ----- | ------ |
| `scalar_abi_valid.silica` | OK |
| `string_two_layer_valid.silica` | OK |
| `raw_string_param_fail.golden_fail` | raw binding used `string` |
| `too_many_args_fail.golden_fail` | >8 args |

**Exit criteria**: ABI matrix goldens pass; checker invoked from type-check pass.

---

### Phase 8 — Emitter and Runtime Foreign Calls

**Goal**: Lower foreign calls to the AArch64 runtime; marshal strings in both directions.

**Scope**

- SIR lowering for `foreign c_wrapper` calls inside adapter bodies and worker sequences.
- Emitter (`codegen/` — new `codegen_ffi_foreign.silica`):
  - direct call to symbol (AAPCS64),
  - **string in**: copy Silica string to FFI worker actor stack scratch → pass ptr+len,
  - **string out**: copy C-returned bytes into new Silica string; release wrapper-owned buffer per sidecar `returns` contract,
  - scalars by value per mapping table.
- Runtime asm / C helper in `prims_actors_runtime_asm.silica` or companion:
  - stack scratch alloc for call-duration copies,
  - Silica string allocation from byte span.
- Phase 8 trials link fixture archives **directly in trial Makefile** (not yet via compiler driver).

**Independent test**

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-8
```

Trials in `runtime_addition/`:

| Trial | Expect |
| ----- | ------ |
| `scalar_add_e2e.silica` + `.sout` | `40 + 2 → 42` via cast/worker |
| `string_echo_e2e.silica` + `.sout` | `"hello"` → `"Echo: hello"` |

**Exit criteria**: runtime trials pass with manually specified `-l` / archive path in trial Makefile.

---

### Phase 9 — Compiler Driver Link Integration

**Goal**: Compiler driver links prebuilt libraries named by loaded sidecars; report missing symbols.

**Scope**

- Driver collects `link_library` from all referenced sidecars for the compilation unit closure.
- Link against `dangerous_exposure_source/lib/lib<name>.a`.
- `MissingForeignSymbolError` when symbol missing from archives.
- Remove manual `-l` from Phase 8 trial Makefiles once driver supplies libraries.

**Independent test**

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-9
```

Same runtime trials as Phase 8, but **no** manual archive flags in Makefile.

**Exit criteria**: driver-linked binaries pass; missing-symbol golden fails at link.

---

### Phase 10 — Regression Gate and Documentation Lock

**Goal**: Stable CI lane and spec alignment.

**Scope**

- `make -C trials/ffi_addition all` runs full suite.
- Add top-level `trials/Makefile` integrate target for FFI (mirror `cpu_discovery_and_spawn_pinning`).
- Update spec §10 examples for symmetric string two-layer raw/adapters (if still inconsistent).
- Mark deferred §15.2 C-parsing checks in spec with implementation status.

**Independent test**

```bash
make -C compiler/silica-compiler/trials/ffi_addition all
make -C compiler/silica-compiler/trials integrate-ffi   # once wired
```

**Exit criteria**: full FFI suite green; no regressions in actor/effect/error_enforcement trials.

---

## 5. Compiler Module Map (Summary)

| Component | Likely files |
| --------- | ------------ |
| Parse foreign / wrapper_meta | `src/parser/declarations/parser_declarations_foreign.silica` |
| AST | `src/ast/ast_declarations.silica` |
| Module / export checks | extend module checker under `src/type_checker/` |
| Actor cast-only / worker ID | `src/type_checker/expressions/type_checker_expressions_actors.silica` |
| Taint | new `src/type_checker/ffi/type_checker_ffi_taint.silica` |
| Sidecar loader | `src/ffi/ffi_sidecar_loader.silica` |
| ABI checker | `src/ffi/ffi_abi_checker.silica` |
| Lowering / emit | `src/codegen/ffi/codegen_ffi_foreign.silica` |
| Runtime marshaling | `src/emitter/terms/prims/prims_ffi_runtime.silica` |
| Driver link | `src/compiler.silica` |

---

## 6. Completion Tracking Table

| Phase | Description | Status | Independent test | Notes |
| ----- | ----------- | ------ | ---------------- | ----- |
| 0 | Fixture `.a` + sidecar fixtures + harness | ☐ | `make … phase-0` | No compiler changes |
| 1 | Parse `foreign`, `wrapper_meta`, `meta`, `external_danger` | ☐ | `make … phase-1` | |
| 2 | AST / SIR plumbing | ☐ | `make … phase-2` | Scout snapshots |
| 3 | `dangerous_*` naming, exports, path rules | ☐ | `make … phase-3` | |
| 4 | Sidecar loader + entry validation | ☐ | `make … phase-4` | |
| 5 | Cast/worker placement + cast-only behaviors | ☐ | `make … phase-5` | |
| 6 | Structural taint + boundaries | ☐ | `make … phase-6` | |
| 7 | Silica-side ABI checker (two-layer strings) | ☐ | `make … phase-7` | No C parsing |
| 8 | Emitter + runtime marshaling | ☐ | `make … phase-8` | Manual link in trial |
| 9 | Driver link integration | ☐ | `make … phase-9` | |
| 10 | Full suite + CI + doc lock | ☐ | `make … all` | |

**Status legend**: ☐ not started · ◐ in progress · ☑ complete

---

## 7. Rollout Order

```text
0 → 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10
```

Safety and metadata phases (3–7) complete before runtime (8). Link driver (9) follows first successful runtime trials (8).

---

## 8. Exit Criteria (Feature Complete)

FFI support is complete when:

- [ ] All phases 0–10 marked ☑ in §6.
- [ ] Spec §15.1 Silica-side failures have golden trials (except deferred items).
- [ ] Spec §15.2 link/metadata failures have golden trials.
- [ ] At least one scalar and one string in/out end-to-end trial runs via cast/worker model.
- [ ] `dangerous_*` naming propagates to root module in checker.
- [ ] No regressions in existing actor, supervisor, and error_enforcement trial lanes.

---

## 9. Open Items (Non-Blocking)

Resolve during Phases 4–6; do not block Phase 0–3.

| Item | Target phase |
| ---- | ------------- |
| Exact sidecar field syntax for `returns:` ptr+len transfer | 4 |
| Standard FFI request/result cast message shapes (typed vs generic records) | 5 |
| Client actor storing pure state from tainted FFI result casts under strict taint | 6 |
| Region → actor-state conversion through FFI result casts | 6 |

---

## 10. Related Documents

- [silica_ffi_wrapper_specification.md](silica_ffi_wrapper_specification.md) — normative behavior
- [silica-compiler-code-organization.md](silica-compiler-code-organization.md) — file layout
- [actor_implementation_plan.md](actor_implementation_plan.md) — cast/spawn prerequisites
