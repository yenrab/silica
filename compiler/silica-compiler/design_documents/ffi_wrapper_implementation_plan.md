# Silica FFI Wrapper Behavior — Stepwise Implementation Plan

**Date**: May 15, 2026  
**Primary specification**: [silica_ffi_wrapper_specification.md](silica_ffi_wrapper_specification.md)

This plan breaks implementation into incremental phases that keep the compiler runnable while adding the full wrapper-first FFI boundary (`dangerous_*` modules + `external_danger` effect + wrapper ABI validation).

---

## 0. Goals and Non-Goals

### Goals

- Implement the source-level FFI surface from the specification:
  - `foreign c_wrapper "symbol"` declarations in `dangerous_*` modules.
  - `external_danger` effect placement and usage rules.
  - dangerous module naming and transitive dependency naming checks.
- Enforce compile-time and parser diagnostics listed in spec section 15.1.
- Add wrapper-side ABI validation flow (section 15.2), including header placement under `dangerous_exposure_source`.
- Establish metadata-driven checks for ownership/lifetime/blocking/result/array/de-opaqueification requirements.

### Non-Goals (for this execution cycle)

- Inbound interop (callbacks, trampolines, C -> Silica calls).
- Finalizing every metadata syntax detail in one step (the spec leaves exact syntax open).
- Full optimizer-level treatment of taint beyond required structural safety checks.

---

## 1. Implementation Strategy

- Land parser + AST support first (feature-gated if needed).
- Add module/effect/type checks next, with exact required diagnostics.
- Integrate lowering/emitter/runtime ABI behavior for supported signatures.
- Add wrapper validator + build-system integration for headers/metadata.
- Backfill trials and failure tests at each phase to prevent regressions.

---

## 2. Phase-by-Phase Plan

## Phase 1 — Syntax, AST, and IR Plumbing

### Scope

- Add parser support for:
  - `foreign c_wrapper "symbol_name" fn local_name(...) -> ReturnType;`
  - `external_danger` as a recognized sequence effect.
- Add AST/SIR representation for raw foreign bindings:
  - local function name + arity
  - wrapper symbol string
  - argument/return types
  - module ownership metadata (`dangerous_*` module flag)

### Deliverables

- Parser accepts valid declarations and rejects malformed forms.
- AST printing/debug output includes foreign declaration nodes.
- SIR generator carries foreign declarations into downstream passes.

### Acceptance checks

- Positive parse tests for valid declarations and effect-tag usage.
- Negative parse tests for malformed foreign declarations.

---

## Phase 2 — Module Rules and Export Constraints

### Scope

- Implement dangerous naming checks in module checker:
  - module with foreign declarations must be `dangerous_*`.
  - any module importing/using `dangerous_*` must also be `dangerous_*` (transitive to root).
- Implement export rule checks:
  - raw foreign bindings cannot be exported.
  - exported functions from `dangerous_*` module must be Silica adapter wrappers (not raw bindings).

### Deliverables

- Dependency graph propagation for dangerous naming.
- Required errors implemented:
  - `DangerousModuleNameError`
  - `DangerousDependencyNamingError`
- Export diagnostics aligned with section 3.3 and 15.1.

### Acceptance checks

- Trials that verify valid transitive dangerous naming.
- Trials that intentionally violate naming/export rules and assert exact failures.

---

## Phase 3 — `external_danger` Placement and Effect Enforcement

### Scope

- Enforce call placement:
  - calls to `dangerous_*` functions must occur in sequence portion of `sequence proc[external_danger] ... produces pure ... end`.
- Enforce actor placement:
  - `external_danger` sequence is only valid directly in behavior function passed to `spawn`.
- Disallow placements from spec section 4.4.

### Deliverables

- Parser/effect checker integration for:
  - `ExternalDangerPlacementError`
  - `DangerousModuleCallError`
- Effect checker ensures sequence effect tag includes `external_danger` when dangerous calls exist.

### Acceptance checks

- Positive tests in spawned actor behavior (literal and named function passed directly to `spawn`).
- Negative tests for top-level/helper/nested non-behavior placements.

---

## Phase 4 — External-Danger-Touched Data and Region Boundary Checks

### Scope

- Introduce taint marker in type checker for values returned from `dangerous_*` modules.
- Structural taint propagation through records/tuples/lists/sums.
- Enforce:
  - pure completion of `external_danger` sequence (no tainted values at any depth),
  - region escape restrictions,
  - no call/cast boundary crossing for forbidden region data,
  - restricted-effects prohibition (`device_io`, `network_io`, `hot_swap`, `register_rwr`).

### Deliverables

- Type-check/parser failures:
  - `ExternalDangerSequenceResultError`
  - `ExternalDangerRegionEscapeError`
  - `ExternalDangerMessageBoundaryError`
  - `ExternalDangerRestrictedEffectError`

### Acceptance checks

- Focused negative tests per error.
- Structural nested-value tests to ensure depth-aware enforcement.

---

## Phase 5 — ABI Type Mapping and Foreign Signature Validation

### Scope

- Implement ABI mapping checker for supported scalar types and bool encoding conventions.
- Reject disallowed C ABI shapes in Silica-facing declarations.
- Add argument-count/shape validation (including "more than eight Silica-level args after lowering" requirement).
- Validate directional type mapping (Silica -> C and C -> Silica).

### Deliverables

- Deterministic signature compatibility checker used by type checker and wrapper validator.
- Compiler diagnostic path for declaration/signature mismatches.

### Acceptance checks

- Matrix tests for allowed/disallowed scalar and composite signatures.
- Boolean representation tests (`uint8_t` mapping with 0/1 semantics).

---

## Phase 6 — Lowering, Emitter, and Runtime Boundary Semantics

### Scope

- Implement foreign call lowering in SIR/emitter for supported signatures.
- String boundary behavior:
  - copy string to actor stack scratch for call duration.
  - pass pointer + length.
  - ensure no direct mutation of original string storage.
- Buffer and pointer boundary handling:
  - typed pointer+length conventions,
  - explicit handling for pointer returns according to array vs non-array rules.

### Deliverables

- Working outbound wrapper call path at runtime.
- Calling convention tests proving argument marshaling and return raising behavior.

### Acceptance checks

- End-to-end sample wrappers (numeric, parser result, buffer return).
- Runtime assertions or sanitizer-backed checks for temporary call-lifetime storage.

---

## Phase 7 — Wrapper Metadata + Header Validation + Build Integration

### Scope

- Add wrapper validator input model:
  - signature-derived facts
  - metadata-derived facts (ownership/lifetime/blocking/result conventions/array lengths/de-opaqueification/non-recursive proof/error domain).
- Enforce build-system rule:
  - wrapper headers must live under project-root `dangerous_exposure_source`.
- Reject Silica-specific preprocessor macro requirements in wrappers.
- Add package/build declarations support for wrapper headers/sources/includes/libs.

### Deliverables

- Wrapper-side errors:
  - `UnsupportedExternalAbiError`
  - `RecursiveExternalStructError`
  - `ExternalPointerReturnError`
  - `ExternalVoidPointerError`
  - `DangerousExposureSourceError`
- Initial metadata schema + loader (minimal but extensible).

### Acceptance checks

- Validator tests for each required wrapper-side hard error category.
- Build tests that validate allowed/forbidden header paths.

---

## Phase 8 — Result Conventions and De-Opaqueification Compliance

### Scope

- Implement checks for deterministic result tag conventions (`0=Ok`, `1=Error`) where wrappers use tagged results.
- Validate object/result initialization requirements.
- Validate de-opaqueification contracts for opaque objects and recursive-struct prohibition.
- Ensure explicit-error requirements when pointer shape/type/length cannot be determined.

### Deliverables

- Validator rules for sections 6, 8, 9, 11, and 13 that are not purely syntactic.
- Example wrapper metadata/templates for common patterns (tagged int result, object decode, array return).

### Acceptance checks

- Golden wrapper fixtures: valid + intentionally invalid metadata/header combinations.

---

## Phase 9 — Trials, Regression Suite, and Documentation Lock

### Scope

- Add dedicated trial set (similar to supervisors trials organization) for:
  - parser/module/effect/type failures,
  - valid outbound calls,
  - wrapper-validator failures,
  - dangerous dependency propagation to root module naming.
- Add release criteria and checklist for feature completion.
- Document user-facing compiler diagnostics and migration guidance.

### Deliverables

- Repeatable test command(s) for FFI feature gate.
- Updated design docs and examples mapped to implemented behavior.

### Acceptance checks

- CI lane for FFI behavior and wrapper validation.
- Regression pass over existing actor/effect tests to ensure no collateral breakage.

---

## 3. Recommended Rollout Order

1. Phase 1-2 (syntax/module safety baseline).
2. Phase 3-4 (effect + taint safety hard guarantees).
3. Phase 5-6 (ABI lowering/runtime functionality).
4. Phase 7-8 (wrapper-side contract enforcement).
5. Phase 9 (full trials + stabilization).

This order minimizes risk by enforcing safety constraints before broad runtime exposure.

---

## 4. Tracking Checklist (Copy Into Sprint Board)

- [ ] Parser: `foreign c_wrapper` declaration support.
- [ ] Parser/effect: `external_danger` effect token + placement checks.
- [ ] Module checker: dangerous naming (local + transitive).
- [ ] Export checker: no raw foreign export.
- [ ] Type checker: taint propagation and sequence purity checks.
- [ ] Type checker/parser: region/message boundary restrictions.
- [ ] ABI checker: supported/disallowed type matrix.
- [ ] Lowering/emitter/runtime: outbound call marshaling.
- [ ] String/buffer boundary semantics implemented and tested.
- [ ] Wrapper metadata schema + parser.
- [ ] Wrapper validator: required hard-error categories.
- [ ] Build integration: `dangerous_exposure_source` enforcement.
- [ ] End-to-end wrapper fixtures + failure suites.
- [ ] CI lane + docs/examples update.

---

## 5. Open Decisions to Resolve Early

The specification explicitly leaves several design points open. Resolve these by the end of Phase 3 to avoid rework:

- Exact metadata file syntax and discovery rules.
- Exact adapter-wrapper detection rule for exports (AST shape vs semantic call graph).
- Whether all `dangerous_*` module functions (including pure helpers) are actor-behavior-only callable, or only calls made from outside the module.
- How to represent validator-backed de-taint conversions in the type system.
- Blocking-call runtime strategy (inline vs worker-thread handoff) and observability hooks.

---

## 6. Exit Criteria (Feature Complete)

The FFI wrapper behavior is complete when:

- all required section 15.1 Silica-side failures are implemented and tested;
- all required section 15.2 wrapper-side failures are implemented and tested;
- at least one fully valid wrapper package builds and runs end-to-end;
- dangerous naming propagation to root module is enforced;
- `external_danger` placement/effect restrictions are enforced with exact diagnostics;
- no unresolved safety regressions remain in actor/effect/type-check suites.

