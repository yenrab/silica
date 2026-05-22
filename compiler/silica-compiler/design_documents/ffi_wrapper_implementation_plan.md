# Silica Compiler FFI — Implementation Plan

**Date**: May 22, 2026  
**Primary specification**: [silica_ffi_wrapper_specification.md](silica_ffi_wrapper_specification.md)  
**Code organization**: [silica-compiler-code-organization.md](silica-compiler-code-organization.md)

This plan adds outbound FFI support to `silica-compiler`. Each phase is **independently testable** before the next phase begins. Phases enforce safety and metadata rules before enabling runtime foreign calls.

---

## 0. Current State

| Area | Status |
| ---- | ------ |
| `foreign c_wrapper` syntax | Implemented (Phases 1–2) |
| `wrapper_meta` / per-binding `meta` | Implemented (Phases 1–4) |
| `external_danger` effect | Implemented (Phases 1, 5) |
| `dangerous_*` module naming propagation | Implemented (Phase 3) |
| Cast-only behavior enforcement | Implemented (Phase 5) |
| FFI worker placement rules | Implemented (Phase 5) |
| Structural taint for `dangerous_*` returns | Implemented (Phase 6) |
| Sidecar `.meta` loader | Implemented (Phase 4) |
| Silica-side ABI checker | Implemented (Phase 7) |
| Foreign call emitter / runtime | Implemented (Phase 8) |
| Link manifest + Makefile link integration | Implemented (Phase 9) |

**Existing infrastructure to reuse**: actor `spawn` / `cast`, sequence blocks, effect checking (`proc[concurrency]`, `device_io`, …), module import graph, trial harness (`trials/`, `.golden_fail`, `.scout` / `.ascomp`), and the standard three-stage build in `trials/base/makefile` (`silica-compiler` → `.sams` → `clang` assemble → `rust-lld`/`clang` link).

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
| Assembler / linker | External via Makefiles: `clang` assembles `.sams` → `.o`; `rust-lld` or `clang` links executables. `silica-compiler` writes `.sams` and a link manifest; it does **not** invoke the linker. |
| Build | Prebuilt static libraries under `dangerous_exposure_source/lib/` |
| `blocking` metadata | Not used (architecturally non-blocking via cast model) |
| De-taint | Strict structural taint in v1 |

---

## 2. Non-Goals

- Inbound interop (C → Silica, callbacks, trampolines).
- C header / wrapper source parsing or compilation in the Silica build.
- Dynamic linking and `foreign package { … }` manifest (spec §14.3 — future).
- Validator-based de-taint.
- Deferred mechanical wrapper-side checks listed in spec §15.2 — **skipped**; author guidelines only, no C parsing in the Silica toolchain.

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
| `module_addition/` | Phase 3: naming + exports (compile-only) |
| `app_sidecar_legacy_math_add/` | Phase 4: sidecar metadata + runnable legacy math add |
| `app_cast_worker_legacy_add/` | Phase 5: cast/worker + `external_danger` runnable app |
| `app_ffi_result_cast_add/` | Phase 6: FFI result cast runnable app |
| `app_foreign_abi_valid/` | Phase 7: Silica-side ABI runnable apps |
| `app_e2e_scalar_string_echo/` | Phase 8–9: e2e scalar/string foreign calls |
| `error_enforcement_addition/ffi_addition/error_app_*` | Golden compile/link failures |

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
- Record link libraries on `SIRModule` for the Phase 9 link manifest.

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
- Phase 8 trials link fixture archives via `silica.link` manifest consumption in trial Makefiles (Phase 9).

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

### Phase 9 — Toolchain Link Integration (Link Manifest + Makefile)

**Goal**: Automatically link prebuilt wrapper libraries named by loaded sidecars for the whole `silica.config` closure; surface `MissingForeignSymbolError` at link time.

**Build pipeline context**

Silica uses a **three-stage build** (see `trials/base/makefile`):

1. **`silica-compiler`** — compile to `.sams` (+ `__silica_runtime.sams` when needed)
2. **`clang`** — assemble `.sams` → `.o`
3. **`rust-lld` or `clang`** — link `.o` → executable

Phase 9 does **not** embed the linker in `silica-compiler`. The compiler emits link metadata after a successful compile; project/trial Makefiles consume that metadata during Stage 3. This matches the normative spec split: the compiler loads sidecar metadata at compile time; the **linker** resolves wrapper symbols when producing the binary (spec §14.2, §1582, §1833).

**Scope — compiler (Stage 1 output)**

- After successful compile of **all units** listed in `silica.config`, aggregate:
  - `link_library` names (union across the program closure; dedupe),
  - every `foreign c_wrapper "symbol"` string referenced by emitted code.
- **Step 3 (archive validation)**: before writing `silica.link`, verify each resolved archive path exists on disk; emit `E4034` / `MissingArchiveError` if a referenced prebuilt library is absent (spec §14.2, §15.2).
- Write one link manifest in the build directory, **`silica.link`**, containing at minimum:
  - one `link_library: "<name>"` line per required archive (deduped),
  - resolved archive paths: `archive: "dangerous_exposure_source/lib/lib<name>.a"`,
  - one `symbol: "<c_wrapper_symbol>"` line per required foreign binding.
- Extend `src/build_output/build_output.silica` to write `silica.link`.
- Extend `src/main.silica` to aggregate link libraries and foreign symbols program-wide (single-unit and multi-unit `silica.config` paths).
- Reuse / extend `src/ffi/ffi_sidecar_loader.silica` for program-wide collection if helpful (`collect_program_link_libraries` already exists per unit; Phase 9 needs closure-wide merge).

**Suggested `silica.link` shape** (macro-free, Makefile-friendly):

```text
link_library: "silica_legacy_math"
archive: "dangerous_exposure_source/lib/libsilica_legacy_math.a"
link_library: "silica_text"
archive: "dangerous_exposure_source/lib/libsilica_text.a"
symbol: "silica_legacy_math_add_int64"
symbol: "silica_text_echo"
```

Makefiles may translate `archive:` lines to direct archive arguments or derive `-L dangerous_exposure_source/lib -l<name>` from `link_library:` lines. Direct archive paths are preferred for deterministic trial builds.

**Scope — Makefiles (Stage 3 link)**

- Extend `trials/base/makefile` to read `silica.link` after Stage 1 and append derived flags to `LDFLAGS_rust-lld` / `LDFLAGS_clang` during Stage 3.
- Update `trials/ffi_addition/app_e2e_scalar_string_echo/Makefile`: **remove** hardcoded `EXTRA_LIBS`; use the shared manifest-reading pattern from `trials/base/makefile`.
- Add `phase-9` target to `trials/ffi_addition/Makefile` (same runtime trials as Phase 8, manifest-driven link).

**`MissingForeignSymbolError`**

- **Primary (required)**: linker undefined-symbol failure when a declared `foreign c_wrapper "symbol"` is absent from the archives named in `silica.link`. A link-stage golden trial in `error_enforcement_addition/ffi_addition/` documents this failure.
- **Optional enhancement**: compile-time `nm` / `llvm-nm` preflight in `silica-compiler` for a structured `MissingForeignSymbolError` diagnostic before Stage 3. Not required for Phase 9 exit (spec §1.1: symbol validation at link time only).

**Non-goals**

- `silica-compiler` invoking `rust-lld`, `clang`, or any linker directly.
- Compiling C wrapper sources or changing assembler/linker tool choice.

**Independent test**

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-9
```

Same runtime trials as Phase 8, but **no** manual `-L` / `-l` archive flags in the trial Makefile.

| Trial | Expect |
| ----- | ------ |
| `dangerous_scalar_add_e2e` + `.sout` | `40 + 2 → 42` via manifest-driven link |
| `dangerous_string_echo_e2e` + `.sout` | `"hello"` → `"Echo: hello"` via manifest-driven link |
| missing foreign symbol link golden | link fails with undefined symbol / `MissingForeignSymbolError` |
| missing prebuilt archive compile golden | compile fails with `E4034` before `silica.link` is written |

**Exit criteria**: `silica.link` is emitted for FFI programs; `trials/base/makefile` and `app_e2e_scalar_string_echo/` link using manifest flags only; e2e app trials pass without hardcoded `EXTRA_LIBS`; missing-symbol link golden fails at Stage 3; missing-archive golden fails at link-manifest validation (step 3).

---

### Phase 10 — Regression Gate and Documentation Lock

**Goal**: Stable CI lane and spec alignment.

**Scope** (complete)

- ☑ `make -C trials/ffi_addition all` runs the success-path suite (Phase 9 manifest-driven link via app integrates).
- ☑ `make -C trials integrate-ffi` — same as `make -C ffi_addition integrate` (success-path app goldens only); wired in CI.
- ☑ Compile/link failure goldens (`link-fail`, `archive-missing`) run via `error_enforcement_addition/ffi_addition integrate` (included in top-level `integrate` through `error_enforcement_addition`).
- ☑ Top-level `trials/Makefile` `integrate` runs `ffi_addition` via auto-discovered trial subdir.
- ☑ `silica.link` documented in `trials/ffi_addition/README.md`; Makefile consumption in `trials/base/makefile`.
- **Skipped:** spec §10 symmetry pass (examples already consistent); tutorial does not document `integrate-ffi` (by choice).
- **Skipped (out of scope):** mechanical enforcement of §15.2 wrapper-side C checks — author guidelines only (see spec §15.2).

**Exit criteria**: ☑ FFI success-path suite green via `integrate-ffi`; ☑ failure goldens green via error-enforcement integrate; ☑ CI; no regressions in actor/effect/error_enforcement trials.

---

### Phase 11 — Guarded FFI Runtime Boundary Model

**Goal**: Define and implement the runtime boundary that makes same-process guarded FFI recoverable only at prepared call sites.

This phase extends the already-working Phase 8 runtime FFI path. It does not change the Phase 0–8 compiler acceptance rules.

**Scope**

- Add runtime state for guarded FFI entry/exit:
  - current actor/fiber/task identity,
  - current OS thread,
  - active guarded-call metadata,
  - prearranged recovery point,
  - preallocated fault record.
- Enter guarded FFI only through a small runtime wrapper around the emitted foreign call.
- Ensure the runtime does not hold scheduler, mailbox, supervisor, actor-system, allocator, or logging locks while inside guarded FFI.
- Define a `ForeignFault`/foreign-failure reason carried by actor failure/exit metadata.
- Mark the guarded boundary as best-effort:
  - if the runtime cannot prove the fault happened inside a prepared guarded call, abort,
  - if runtime invariants are suspect after the fault, abort,
  - never resume the original C frame or continue the actor in place.

**Independent test**

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-11
```

Trials (`app_legacy_math_add_guarded`, `app_legacy_math_add_twice`):

| Trial | Expect |
| ----- | ------ |
| `dangerous_legacy_math_add_once` | `silica_legacy_math_add_int64` e2e success through guarded enter/exit |
| `dangerous_legacy_math_add_twice` | two sequential guarded calls in one worker; per-thread depth returns to 0 |

**Exit criteria**: guarded wrapper is on the runtime call path, has no observable behavior change for successful FFI (same `.scout` outputs), and leaves no stale guarded-call state after success (second call in `app_legacy_math_add_twice`).

---

### Phase 12 — Per-Actor FFI Arena and Pointer Relocation

**Goal**: Move all C-facing pointers used by guarded FFI from ordinary heap/actor storage into a per-actor FFI arena before adding crash recovery.

This phase intentionally comes before the macOS signal bridge. Crash handling must recover from faults against arena-contained C-visible memory, not from a design that still passes heap pointers and would need to be rewritten later.

**Scope**

- Add a per-actor FFI arena descriptor to actor runtime state:
  - base pointer,
  - capacity,
  - bump/reset cursor or equivalent allocator state,
  - guard/bounds metadata,
  - ownership of any wrapper-owned output buffers that need release.
- Define the FFI arena as runtime-managed scratch containment memory:
  - copied inputs,
  - output buffers,
  - temporary C-facing memory,
  - metadata needed for recovery.
- Clarify implementation relationship to Silica memory regions:
  - arena may be backed by a Silica memory region,
  - arena is not automatically identical to user-visible region ownership,
  - ordinary region ownership rules still apply at the Silica boundary.
- Relocate Phase 8 marshaling so raw C pointer arguments point into the per-actor FFI arena:
  - Silica strings copied to arena bytes before call,
  - C output byte spans copied back into Silica-owned values after success,
  - temporary pointer-plus-length records stored in arena or runtime scratch, not user heap.
- Ensure raw Silica heap pointers, actor state pointers, scheduler pointers, mailbox pointers, and arbitrary region aliases are not passed to C in guarded mode.
- Reset or discard arena memory after successful guarded calls.
- On actor termination, free or reclaim the per-actor FFI arena with normal actor cleanup.
- Add diagnostics or runtime assertions when a guarded FFI wrapper attempts to expose unsupported pointers.
- Keep untrusted or plugin-like C out of same-process guarded mode; process isolation remains the future robust containment strategy.

**Independent test**

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-12
```

Trials:

| Trial | Expect |
| ----- | ------ |
| `guarded_arena_string_in_success.silica` | C receives arena pointer for copied Silica string |
| `guarded_arena_string_out_success.silica` | C output span is copied into Silica-owned value after success |
| `guarded_arena_reset_after_success.silica` | repeated successful calls do not retain stale arena data |
| `guarded_no_raw_vm_pointer_fail.golden_fail` | guarded wrapper cannot expose raw Silica heap pointer |
| `guarded_region_boundary_valid.silica` | copied/handle-based region transfer remains valid |

**Exit criteria**: guarded FFI no longer passes ordinary Silica heap or actor-state pointers to C; all guarded C-visible pointers are arena/copy/handle based before fault handling is added.

---

### Phase 13 — macOS `sigaltstack` / `sigaction` Fault Bridge

**Goal**: Add macOS-specific same-process fault detection for guarded FFI using the approach in [macos_crash_handling_for_silica.md](macos_crash_handling_for_silica.md).

**Scope**

- Install macOS signal handlers early in runtime initialization with `sigaction`.
- Configure `sigaltstack` and use `SA_ONSTACK` for handled synchronous fault signals.
- Handle only the minimal relevant signals initially:
  - `SIGSEGV`,
  - `SIGBUS`,
  - `SIGILL`,
  - `SIGFPE`,
  - optionally `SIGABRT` and `SIGTRAP` only if policy decides those belong to guarded FFI.
- Handler responsibilities are strictly limited:
  - inspect preinitialized per-thread guarded-FFI state,
  - record signal number, fault address, code, and current guarded metadata into preallocated storage,
  - record whether the fault address was inside the active per-actor FFI arena when applicable,
  - jump to the prepared recovery point or restore default handling / abort.
- Handler must not:
  - allocate,
  - lock,
  - format strings,
  - call Objective-C/Swift/runtime code,
  - send actor/supervisor messages,
  - run Silica cleanup,
  - restart actors.
- Preserve debugger/crash-reporter behavior for faults outside guarded FFI by forwarding or restoring default disposition as appropriate.

**Independent test**

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-13
```

Trials:

| Trial | Expect |
| ----- | ------ |
| `guarded_null_deref_fault.silica` | C null dereference becomes `ForeignFault`, process survives |
| `guarded_arena_bounds_fault.silica` | arena-bounds fault is recognized as guarded FFI fault |
| `guarded_sigbus_fault.silica` | recognized bus fault becomes `ForeignFault`, process survives |
| `unguarded_null_deref_abort.silica` | same C fault outside guarded FFI aborts / fails the process as expected |
| `handler_altstack_stack_overflow_smoke.silica` | handler is installed with alternate stack and reaches recovery boundary |

**Exit criteria**: guarded synchronous faults on macOS are detected without doing rich work inside the handler; unguarded faults are not silently swallowed.

---

### Phase 14 — Actor Failure, Arena Reset, and Supervisor Restart Integration

**Goal**: Convert recognized guarded-FFI faults into ordinary Silica actor failure after control returns to safe runtime code.

**Scope**

- Recovery boundary maps preallocated fault record to a runtime `ForeignFault` reason.
- Runtime marks the current actor/fiber/task failed.
- Runtime tears down the failed actor's current execution without running arbitrary user cleanup inside the signal handler.
- Runtime resets or discards the actor's FFI arena as part of actor failure handling.
- Supervisor and monitor notifications use the existing actor failure/exit plumbing.
- Supervisor restart policy may restart the actor exactly as it would for ordinary actor failure.
- If the current execution is not inside an actor, map to task failure or process abort according to runtime policy.
- Explicitly reject in-place continuation of the actor after a recognized guarded-FFI fault.

**Independent test**

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-14
```

Trials:

| Trial | Expect |
| ----- | ------ |
| `guarded_fault_actor_dies.silica` | actor running guarded FFI terminates with `ForeignFault` |
| `guarded_fault_supervisor_restarts.silica` | supervisor observes failure and restarts child |
| `guarded_fault_monitor_down.silica` | monitor receives failure/down notification |
| `guarded_arena_reset_after_fault.silica` | arena state is discarded after `ForeignFault` |
| `guarded_fault_no_in_place_continue.silica` | actor state after fault comes from restart, not from continued corrupted frame |

**Exit criteria**: recognized guarded-FFI faults become actor/task death through normal runtime paths, FFI arena state is discarded on failure, and supervisor restart behavior is observable in trials.

---

### Phase 15 — macOS Guarded FFI Regression Gate and Platform Expansion Notes

**Goal**: Make macOS guarded-FFI crash handling part of the regression suite and document that equivalent support is platform-specific.

**Scope**

- Add `trials/ffi_addition/guarded_macos_addition/` for macOS-only guarded fault trials.
- Gate macOS-only tests so non-macOS hosts skip them cleanly with a clear message.
- Ensure successful output-golden comparisons use the same `✅✅` and `.integrate_counts` conventions as other trial subdirectories.
- Add docs for current macOS status:
  - same-process guarded FFI is best-effort,
  - actor restart is allowed only after safe recovery to runtime code,
  - arbitrary C corruption remains outside the guarantee.
- Add placeholder sections or tracking items for future Linux, Windows, and bare-metal fault-handling documents.

**Independent test**

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-15
make -C compiler/silica-compiler/trials integrate
```

Trials:

| Trial | Expect |
| ----- | ------ |
| macOS host | guarded macOS fault suite runs and counts golden comparisons |
| non-macOS host | guarded macOS fault suite skips with `0 0` or documented skip count |
| top-level integrate | includes guarded macOS suite without disrupting other FFI/runtime trials |

**Exit criteria**: macOS guarded FFI is covered by CI-compatible trials, and platform-specific caveats are linked from the FFI spec and implementation plan.

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
| Lowering / emit | `src/emitter/apple_silicon/terms/ffi_foreign.silica` |
| Runtime marshaling | `src/emitter/apple_silicon/terms/ffi_foreign_runtime_asm.silica` |
| Link manifest writer | `src/build_output/build_output.silica`, `src/main.silica` |
| Program-wide link/symbol aggregation | `src/ffi/ffi_sidecar_loader.silica` (extend if needed) |
| Makefile link step (Stage 3) | `trials/base/makefile`, trial-specific Makefiles |

---

## 6. Completion Tracking Table

| Phase | Description | Status | Independent test | Notes |
| ----- | ----------- | ------ | ---------------- | ----- |
| 0 | Fixture `.a` + sidecar fixtures + harness | ☑ | `make … phase-0` | Fixture archives and `phase-0` target exist |
| 1 | Parse `foreign`, `wrapper_meta`, `meta`, `external_danger` | ☑ | `make … phase-1` | Implemented; old parse-only trial lane removed from current harness |
| 2 | AST / SIR plumbing | ☑ | `make … phase-2` | Implemented; old SIR-scout lane removed from current harness |
| 3 | `dangerous_*` naming, exports, path rules | ☑ | `make … phase-3` | Valid path in `ffi_addition`; failures moved under `error_enforcement_addition/ffi_addition` |
| 4 | Sidecar loader + entry validation | ☑ | `make … phase-4` | Valid path in `ffi_addition`; failures moved under `error_enforcement_addition/ffi_addition` |
| 5 | Cast/worker placement + cast-only behaviors | ☑ | `make … phase-5` | Valid path in `ffi_addition`; failures moved under `error_enforcement_addition/ffi_addition` |
| 6 | Structural taint + boundaries | ☑ | `make … phase-6` | Valid path in `ffi_addition`; failures moved under `error_enforcement_addition/ffi_addition` |
| 7 | Silica-side ABI checker (two-layer strings) | ☑ | `make … phase-7` | No C parsing; failures moved under `error_enforcement_addition/ffi_addition` |
| 8 | Emitter + runtime marshaling | ☑ | `make … phase-8` | Runnable apps under `app_e2e_scalar_string_echo/` |
| 9 | Link manifest + Makefile link integration | ☑ | `make … phase-9` | Steps 1–3: emit `silica.link`, Makefile `archive:` consumption, compile-time archive existence (`E4034`); link-time missing-symbol golden. |
| 10 | Full suite + CI + doc lock | ☑ | `make … integrate-ffi` | CI wired; `integrate-ffi` = success-path; failures via `error_enforcement_addition/ffi_addition integrate` |
| 11 | Guarded FFI runtime boundary model | ☑ | `make … phase-11` | Enter/exit wrapper + per-thread TLS; `app_legacy_math_add_guarded`, `app_legacy_math_add_twice` |
| 12 | Per-actor FFI arena and pointer relocation | ☑ | `make … phase-12` | Move C-facing pointers out of ordinary heap/actor storage before fault handling |
| 13 | macOS `sigaltstack` / `sigaction` fault bridge | ☑ | `make … phase-13` | macOS-specific |
| 14 | Actor failure, arena reset, and supervisor restart integration | ☐ | `make … phase-14` | No actor lifecycle work in handler |
| 15 | macOS guarded FFI regression gate and platform expansion notes | ☐ | `make … phase-15` | Other platforms documented as support is added |

**Status legend**: ☐ not started · ◐ in progress · ☑ complete

---

## 7. Rollout Order

```text
0 → 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10
```

Safety and metadata phases (3–7) complete before runtime (8). Toolchain link integration (9) follows first successful runtime trials (8) and wires manifest output into the existing Makefile link stage — it does not move linking into `silica-compiler`.

Guarded macOS FFI crash handling is an extension after the Phase 8 runtime path exists:

```text
8 → 11 → 12 → 13 → 14 → 15
```

Phase 12 deliberately precedes the macOS fault bridge so signal recovery is built around per-actor arena pointers from the start, avoiding a later rewrite from heap-backed C pointers to arena-backed C pointers. Toolchain link integration and the original regression gate (9–10) may proceed independently of guarded crash handling, but Phase 15 should be folded into the full regression gate once the macOS guarded path is enabled.

---

## 8. Exit Criteria (Feature Complete)

FFI support is complete when:

- [x] All phases 0–10 marked ☑ in §6.
- [x] Spec §15.1 Silica-side failures have golden trials.
- [x] Spec §15.2 link/metadata failures have golden trials (wrapper-side C authoring guidelines in §15.2 are not compiler-enforced).
- [x] At least one scalar and one string in/out end-to-end trial runs via cast/worker model with manifest-driven link (no manual `EXTRA_LIBS`).
- [x] `dangerous_*` naming propagates to root module in checker.
- [x] No regressions in existing actor, supervisor, and error_enforcement trial lanes.

---

## 9. Open Items (Deferred)

The following design items remain open after the currently implemented FFI phases. They do not block the completed Phase 0–8 runtime path, but they must be resolved before the corresponding behavior is treated as feature-complete.

| Item | Target phase |
| ---- | ------------- |
| Exact sidecar field syntax for `returns:` ptr+len transfer | 4 |
| Standard FFI request/result cast message shapes (typed vs generic records) | 5 |
| Client actor storing pure state from tainted FFI result casts under strict taint | 6 |
| Region → actor-state conversion through FFI result casts | 6 |

---

## 10. Related Documents

- [silica_ffi_wrapper_specification.md](silica_ffi_wrapper_specification.md) — normative behavior
- [macos_crash_handling_for_silica.md](macos_crash_handling_for_silica.md) — macOS guarded FFI fault handling, actor failure, and recovery caveats
- [silica-compiler-code-organization.md](silica-compiler-code-organization.md) — file layout
- [actor_implementation_plan.md](actor_implementation_plan.md) — cast/spawn prerequisites
