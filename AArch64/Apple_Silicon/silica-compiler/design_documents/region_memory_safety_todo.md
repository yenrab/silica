# Region Memory Safety — Implementation TODO

This document describes what remains to be done to make Silica's region-based memory model memory-safe as specified. The specification (silica-specification.md §12) defines comprehensive safety guarantees; the current implementation provides only partial support.

## Related Documents

| Document | Purpose |
|----------|---------|
| [silica-specification.md](silica-specification.md) | Core language specification, §12 Memory Model |
| [build-plan.md](build-plan.md) | Phase 6: Region Analysis Phase (Tasks 6.1–6.5) |
| [specification-analysis-state.md](../specification-analysis-state.md) | Region analyzer component requirements |

---

## 1. Current Implementation Status

### 1.1 What Is Implemented

| Component | Location | Status |
|-----------|----------|--------|
| Region type syntax validation | `type_checker/type_checker_memory_regions.silica` | ✓ Validates `ref(R, Space, T)`, `region(R, Space)`, `buf(R, Space, T, N)`, `atomic_ref(R, Space, T)` |
| Memory space validation | Same | ✓ Validates `normal`, `normal_writeback`, `normal_writethrough`, `normal_noncacheable`, `atomic`, `device` |
| Effect tracking for region ops | `effect_checker/effect_checker_memory_regions.silica` | ✓ Requires `mem(Space)` for `alloc_region`, `alloc_ref`, `read_ref`, `write_ref`, etc. |
| SIR prim generation | `sir_generator/terms/memory_region_calls.silica` | ✓ Maps region built-ins to SIR prim terms |
| Code emission | `emitter/terms/prims/prims_memory.silica` | ✓ Emits AArch64 for alloc/read/write (stack-based) |

### 1.2 Current Allocation Strategy

Regions are implemented as **stack-based allocation** (see `prims_memory.silica` comment: "Stack-based allocation for minimal implementation; region = arena on stack"):

- `alloc_region`: `SUB SP, SP, #8` — allocates 8 bytes on stack
- `alloc_ref`: `SUB SP, SP, #8` + `STR` — allocates cell on stack
- `alloc_buf`: `SUB SP, SP, X2, LSL #3` — allocates N×8 bytes on stack

This is a minimal prototype. Stack allocation means regions are implicitly "freed" when the stack frame is popped, but without lifetime analysis, the compiler cannot prevent returning or storing references that would become dangling.

---

## 2. Gaps — What Must Be Implemented

### 2.1 Static Region Lifetime Analysis (Critical)

**Specification**: §12.1.4 Static Region Lifetime Analysis

**Status**: ❌ Not implemented

**Required**: The compiler must perform static analysis to verify that:

1. References cannot outlive their containing region
2. Region deallocation occurs only after all references are no longer accessible
3. Function returns of references require the region to outlive the call
4. Cross-function and cross-module lifetime constraints are respected

**Formal framework** (from spec):

- **Lifetime environment**: `L ::= ∅ | L, R:scope` — maps region identifiers to lexical scopes
- **Dependency set**: `D ::= ∅ | D, ref(R, Space, T):scope` — tracks references and creation scopes
- **Judgment**: `Γ; L; D ⊢ e : T; L'; D'`

**Key rules to implement**:

- Region Allocation Rule: Add region to `L` at `alloc_region`
- Reference Allocation Rule: Verify region in `L`, add ref to `D` at `alloc_ref`
- Reference Usage Rule: Verify region and ref exist, scope constraints hold at `read_ref`/`write_ref`
- Scope Exit Rule: Remove regions/refs, verify no dangling references
- Function Parameter Lifetime Extension: Regions passed as parameters extend to return scope
- Function Return Lifetime Constraint: Returning `ref(R, Space, T)` requires region outlives call

**Build-plan reference**: Phase 6 (Tasks 6.1.1–6.5.2) in build-plan.md

---

### 2.2 Buffer Bounds Checking (Critical)

**Specification**: §12.3.3 Bounds Checking, §14.4.1 Bounds Checking

**Status**: ❌ Not implemented

**Current behavior**: `buf_load` and `buf_store` emit raw `LDR`/`STR` with no index validation:

```silica
// Current: prims_memory.silica
fn emit_buf_load(dest: string) -> string {
    concat("    LDR ", dest, ", [X1, X2, LSL #3]")  // No bounds check!
}
```

**Required**: Before every `buf_load` and `buf_store`:

1. Emit a bounds check: compare index against buffer size (N from `buf(R, Space, T, N)`)
2. On out-of-bounds: raise runtime error (e.g., `BoundsError`) or trap

**Implementation options**:

- **Option A**: Emit `CMP X2, N` + conditional branch to error handler before each load/store
- **Option B**: Use a runtime helper that validates and traps
- **Option C**: When MTE is available, rely on hardware (see spec §21.3); otherwise software bounds check

**Note**: The buffer size N must be available at code generation time (it is in the type `buf(R, Space, T, N)`). The SIR/emitter must propagate this to the prim emission.

---

### 2.3 Region Isolation Enforcement (High)

**Specification**: §12.4.1 Region Isolation

**Status**: ⚠️ Partial — type syntax is validated, but semantic checks are not enforced

**Required**: The type system must prevent:

1. Using a `ref(R1, Space, T)` with a `region(R2, Space)` when R1 ≠ R2
2. Creating cross-region references (e.g., `ref(R2, normal, ref(R1, normal, int))` when R1 ≠ R2)

**Current gap**: The type checker validates that `ref(R, Space, T)` is well-formed but does not:

- Verify that `alloc_ref(region, value)` receives a region whose type's R matches the result type's R
- Unify region identifiers across function boundaries (e.g., when passing regions as parameters)
- Reject mixing regions in invalid ways

**Implementation**: Extend type checker to:

- Track region identifier R from region-typed expressions
- When type-checking `alloc_ref(region_expr, value_expr)`, require `region_expr : region(R, Space)` and result `ref(R, Space, T)` with same R
- When type-checking `read_ref(ref_expr)` / `write_ref(ref_expr, value)`, ensure ref_expr's R is in scope
- Reject programs where R in ref does not match R in the corresponding region

---

### 2.4 Allocation Strategy and Use-After-Free (High)

**Specification**: §12.1.2 Region Lifetime, §12.4.2 Lifetime Safety

**Status**: ❌ Stack allocation without lifetime analysis enables use-after-free

**Problem**: With stack-based regions, when a function returns, its stack frame is popped. Any reference to memory in that frame becomes a dangling pointer. Without lifetime analysis, the compiler cannot reject:

```silica
fn leak_ref() -> ref(R, normal, int64) {
    sequence proc[mem(normal)]
        r: region(R, normal) <- alloc_region(normal);
        ref: ref(R, normal, int64) <- alloc_ref(r, 42);
    produces
        pure ref   // BUG: ref points to stack memory that is about to be invalidated
    end
}
```

**Required**:

1. **Lifetime analysis** (see §2.1) must reject returning references whose region does not outlive the function
2. **Allocation strategy**: Either:
   - Keep stack allocation but enforce (via lifetime analysis) that no ref outlives its region, or
   - Move to heap-based regions with explicit deallocation tied to scope exit (as spec implies)

The spec describes "implicit deallocation when r goes out of scope" — the implementation must ensure no references remain when that happens.

---

### 2.5 atomic_ref Support (Medium)

**Specification**: §4.4.4 Atomic Types, §17 Atomic Operations

**Status**: ⚠️ Type validation exists; emission may be incomplete

**Required**: Verify that `atomic_ref(R, Space, T)` is fully supported:

- Type checker accepts it ✓
- Effect checker handles `mem(atomic)` ✓
- SIR generation and code emission for atomic load/store/compare-exchange
- Memory ordering semantics per §17.2

---

## 3. Implementation Order

Recommended order of work:

1. **Buffer bounds checking** (§2.2) — Relatively localized; prevents buffer overflows immediately.
2. **Region isolation enforcement** (§2.3) — Extends existing type checker; prevents obvious misuse.
3. **Static region lifetime analysis** (§2.1) — Core safety; blocks use-after-free. Depends on build-plan Phase 6.
4. **Allocation strategy** (§2.4) — May follow from lifetime analysis; consider heap-based regions if stack proves insufficient.

---

## 4. Verification Checklist

When implementation is complete, the following should hold:

- [ ] No program can return a reference whose region does not outlive the call (lifetime analysis)
- [ ] No program can use a reference after its region has been deallocated (lifetime analysis)
- [ ] No `buf_load` or `buf_store` executes without a prior bounds check (bounds checking)
- [ ] No `ref(R1, ...)` is used with `region(R2, ...)` when R1 ≠ R2 (region isolation)
- [ ] Cross-module region usage is validated (cross-module lifetime tracking)
- [ ] Error messages reference specification sections (e.g., spec:§12.1.4)

---

## 5. Revision History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-03-10 | Initial document; gaps identified from implementation analysis |
