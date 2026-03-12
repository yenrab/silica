# Silica Formal Verification Specification

**Status:** Initial document. Extends to cover recursive tuple types.

**Related:** [recursive_tuple_specification.md](recursive_tuple_specification.md)

---

## 1. Overview

This document specifies the formal verification framework for Silica, including type-theoretic foundations and proof terms. The value calculus (Layer 1) handles product types and is extended to support recursive product types.

---

## 2. Curry–Howard Foundation

Silica's type system corresponds to a logical calculus via the Curry–Howard isomorphism:

- **Types** ↔ **Propositions**
- **Terms** ↔ **Proofs**
- **Type checking** ↔ **Proof verification**

---

## 3. Layer 1: Value Calculus (λΠ + Sums and Products)

### 3.1 Product Types (Existing)

Product type A × B corresponds to conjunction A ∧ B.

**Introduction:**
```
Γ ⊢ e₁ : T₁    Γ ⊢ e₂ : T₂
────────────────────────────
Γ ⊢ (e₁, e₂) : (T₁, T₂)
```

**Elimination:**
```
Γ ⊢ e : (T₁, T₂)
─────────────────
Γ ⊢ πᵢ(e) : Tᵢ
```

---

## 4. Recursive Product Types (Extension)

### 4.1 Formation

A recursive tuple type `(T₁, rec, rec)` has `rec` referring to the enclosing tuple type. The type checker resolves `rec` via structural equality with occurs check.

### 4.2 Introduction Rule

```
Γ ⊢ e₁ : T₁
Γ ⊢ e₂ : T₂[rec ↦ (T₁, rec, rec)]
Γ ⊢ e₃ : T₃[rec ↦ (T₁, rec, rec)]
────────────────────────────────────────────────────────
Γ ⊢ (e₁, e₂, e₃) : (T₁, rec, rec)
```

Where `T₂[rec ↦ (T₁, rec, rec)]` denotes substitution of `rec` by the enclosing tuple type.

### 4.3 Elimination Rule

```
Γ ⊢ e : (T₁, rec, rec)
─────────────────────
Γ ⊢ πᵢ(e) : Tᵢ[rec ↦ (T₁, rec, rec)]
```

### 4.4 Occurs Check

When comparing recursive types, the type checker:
1. Maintains a mapping: `rec` → enclosing tuple type.
2. On encountering `rec`, substitutes and continues.
3. On cycle (rec encountered again during expansion), treats as equal when structures match.
4. Ensures decidability.

### 4.5 Well-Foundedness

Recursive structures are finite: the base case `:none` ensures termination. All recursive positions are either `:none` or a ref to a region-allocated node; no infinite unfoldings at runtime.

---

## 5. Region Lifetime Analysis (Extension)

The typing judgment is extended with a lifetime environment L and scope dependency set ScopDep for region-based memory:

**Lifetime Environment:**
```
L ::= ∅ | L, Lᵢ:scope
```

**Scope Dependency Set:**
```
ScopDep ::= ∅ | ScopDep, ref(Lᵢ, Space, T):scope
```

**Extended Typing Judgment:**
```
Γ; L; ScopDep ⊢ e : T; L'; ScopDep'
```

**Key Rules:**
- **Region Allocation**: At `alloc_region`, add Lᵢ to L with current scope.
- **Reference Allocation**: At `alloc_ref`, verify Lᵢ ∈ L, add ref(Lᵢ, Space, T) to ScopDep.
- **Reference Usage**: At `read_ref`/`write_ref`, verify Lᵢ ∈ L, ref ∈ ScopDep, scope constraints hold.
- **Scope Exit**: Remove regions/refs from current scope; verify ∀ ref(Lᵢ, ...) ∈ ScopDep'. Lᵢ ∈ L'.

See silica-specification.md §12.1.4 for the full algorithm and rules.

---

## 6. Reference

- [recursive_tuple_specification.md](recursive_tuple_specification.md) — Full design, syntax, and examples.
- [silica-specification.md](silica-specification.md) §12 — Memory Model, lifetime analysis.
