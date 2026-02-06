# Proposal: Crypto-Safe Language Primitives and Compiler Guarantees for Silica
**Audience:** Security professionals and cryptographic engineers familiar with lib-crypto ecosystems
**Goal:** Enable the construction of high-assurance, quantum-prepared cryptographic libraries in Silica by making entire classes of side-channel and misuse vulnerabilities unrepresentable at compile time.

---

## Executive Summary

Silica is a functional, strongly typed language that compiles to AArch64 (soon to be ported to x86 64) assembly and exposes only memory-safe hardware behaviors, with first-class vector operations. This proposal defines a set of **compiler-recognized primitive traits, types, and semantic restrictions** that enable developers to implement cryptographic libraries (including quantum-prepared constructions) with **constant-time guarantees by construction**, strong secret-handling semantics, and minimal risk of accidental misuse.

The core idea: move the burden of side-channel discipline and API safety from “coding guidelines” into **the language and compiler**. The result is a development model where entire classes of crypto vulnerabilities (secret-dependent branching, secret-derived addressing, non-constant-time equality, accidental declassification, secret lifetime leakage) are prevented at compile time.

---

## Design Principles

1. **Make the safe thing the easy thing**
   Crypto-safe patterns should be the default. Unsafe patterns should be unrepresentable.

2. **Compiler-enforced constant-time discipline**
   No secret-dependent control flow or memory addressing is permitted.

3. **Secret-taint propagation via primitive traits**
   Secrecy is part of the type system and flows through computation.

4. **High-performance by construction**
   Vector ops and AArch64 assembly emission enable constant-time, high-throughput primitives without relying on optimizer behavior.

5. **Quantum-prepared by default**
   The language supports safe composition of hybrid classical + PQ cryptography (e.g., ML-KEM + X25519) without allowing downgrade or timing leaks.

---

## Proposed Compiler Primitives (Traits and Types)

### 1. Secrecy Lattice (Primitive Traits)

- `Secret` — marks values as secret-tainted (compiler-recognized)
- `Public` — marks values as public
- **Compiler rule:** any operation involving `Secret` yields `Secret`
- **Compiler rule:** no implicit `Secret -> Public` conversion

Optional gated declassification (for audited boundaries only):
- `CanDeclassify` — capability trait only attachable in explicitly audited modules

---

### 2. Constant-Time Mask Primitive

- `CtMask` — represents constant-time truth values (all-ones / all-zeros masks)
- **Secret comparisons must return `CtMask`, not booleans**
- `CtMask` supports branchless selection

Example pattern (concrete type; Silica has no generics—each element type has its own `select` or trait-based overload):
```
mask: CtMask <- ct_eq(secret_a, secret_b);
out: SecretByte <- select(mask, x, y);
```

---

### 3. Control-Flow Gate Primitive

- `Condition` — primitive trait required by `case`, guards, and short-circuit operators (Silica has no `if` or loops)
- **Only public booleans implement `Condition`**
- Secret values cannot implement `Condition`

**Compiler rule:**
Any conditional expression must have `Condition`; secret-derived conditions are rejected at compile time.

---

### 4. Address Formation Gate Primitives

- `AddressIndex` — required for indexing into buffers (and list index operations, if supported)
- `AddressRange` — required for slicing (if supported)
- `AddressKey` — required for map/dictionary lookup (if applicable)

**Compiler rule:**
Any operation that forms a memory address requires these traits; secret-derived indices/ranges/keys are rejected.

Provide constant-time alternatives (Silica has no generics—concrete variants per element type and size):
- `ct_lookup(buffer: buf(R, normal, SecretByte, 32), secret_index: SecretIndex) -> SecretByte` — full scan + mask-select for small tables. `SecretIndex` denotes a secret index value used only with `ct_lookup`; other sizes/element types have separate function variants.

---

### 5. Secret Buffer Primitives

Silica uses the existing `buf(R, Space, T, N)` type constructor with secrecy traits:

- `buf(R, Space, T, N)` where `T: Public` — fixed-size public buffer
- `buf(R, Space, T, N)` where `T: Secret` — fixed-size secret buffer
- The buffer type itself uses Silica's built-in `buf(R, Space, T, N)` syntax where:
  - `R` is a region identifier
  - `Space` is a memory space (e.g., `normal`, `normal_noncacheable`)
  - `T` is the element type (must implement `Public` or `Secret` trait)
  - `N` is the fixed capacity (compile-time constant)

Compiler guarantees for buffers with `Secret` element types:
- Non-copyable (or uniqueness enforced)
- Not printable/loggable
- **Overwritten on scope exit:** When a buffer holding secret data goes out of scope, it is overwritten (zeroized) by default. The compiler enforces this behind the scenes—no explicit call is required, and the compiler ensures the overwrite is not optimized away. This reduces the window where secret material remains in memory and prevents accidental retention.
- No non-constant-time equality
- Length is public and fixed

Example:
```
key: buf(R, normal, SecretByte, 32)
nonce: buf(R, normal, PublicByte, 12)
```

Where `SecretByte` and `PublicByte` are type aliases or concrete types implementing the `Secret` and `Public` traits respectively.

---

### 6. Secret Context Primitive (Optional but Recommended)

- `proc[secret]` effect in function signatures
- Compiler enforces additional restrictions:
  - No panics/exceptions
  - No I/O or logging (no `device_io` effect allowed)
  - No secret-dependent allocation sizes
  - No secret-dependent control flow or addressing
  - No dynamic-length buffers derived from secrets

This provides a “constant-time verified region” for crypto cores.

Example (concrete size; Silica has no generics—each size is a separate function or fixed at compile time):
```
fn decrypt_secret(key: buf(R, normal, SecretByte, 32), ct: buf(R, normal, PublicByte, 64)) 
    -> proc[secret, mem(normal)] buf(R, normal, SecretByte, 64) {
    -- constant-time decryption implementation
}
```

---

## Compiler Semantic Restrictions

### Control Flow
- `case` expressions, guards, and short-circuit operators require `Condition`
- Secret comparisons produce `CtMask`, not `Condition`

### Memory Access
- Indexing and slicing require `AddressIndex` / `AddressRange`
- Vector gather/scatter must require public indices

### Work-Factor Discipline
- Recursion depth, folds, and traversal sizes must be public
- Secret-derived structure sizes are rejected in `proc[secret]` contexts

### Equality and Verification
- No `==` for secret buffers (secret-typed values cannot use equality operator)
- Provide `ct_eq(a: Secret, b: Secret) -> CtMask` where both arguments must be the same concrete type implementing `Secret`
- Verification APIs should expose masked results for constant-time composition

---

## Constant-Time Crypto Construction Model

Silica’s functional + vectorized model encourages **data-independent traversal with masked selection**:

- Fixed-size buffers (`buf(R, Space, T, N)`) as the primary container for secret-bearing data
- Branchless updates using `select(mask: CtMask, a: Secret, b: Secret)` — return type is the same concrete type as the arguments (separate overloads or trait-based dispatch per type; Silica has no generics)
- Deterministic traversal of buffers using `read_buf` and `write_buf` operations

Example pattern for constant-time validation (concrete size 64; each size would have its own `open_masked` variant):
```
result: (CtMask, buf(R, normal, SecretByte, 64)) <- open_masked(key, nonce, aad, ct);
valid_mask: CtMask <- result.0;
pt_candidate: buf(R, normal, SecretByte, 64) <- result.1;
pt: buf(R, normal, SecretByte, 64) <- select(valid_mask, pt_candidate, zero_buffer);
```

---

## Quantum-Prepared Architecture

Silica’s primitives enable safe construction of hybrid cryptographic protocols:

- **Hybrid KEM composition** (e.g., ML-KEM + X25519)
- **HKDF-based secret derivation** from combined shared secrets
- **AEAD sealing** using constant-time primitives
- **Downgrade-resistant negotiation** via public-only condition checks

Example hybrid derivation (`concat_bufs` is a proposed stdlib function with concrete signatures per buffer size):
```
ss: buf(R, normal, SecretByte, 32) <- hkdf("silica-hybrid-kem", concat_bufs(ss_classical, ss_pq, context));
```

The language enforces:
- No secret-dependent branching during decapsulation
- No early-exit verification logic
- No secret-derived addressing during PQC polynomial or vector operations

---

## AArch64 and Vectorization Implications

Because Silica emits AArch64 assembly directly:

- Constant-time instruction selection is guaranteed
- Vector ops (NEON) can be safely exposed for:
  - ChaCha20-Poly1305
  - AES-GCM (with crypto extensions)
  - SHA-2 / SHA-3 / SHAKE
- Zeroization can be guaranteed (no dead-store elimination)

The compiler should provide:
- Constant-time vector select idioms
- Zeroization intrinsics that cannot be optimized out
- Optional clearing of secret-holding registers at function boundaries

---

## Security Outcomes

This design makes the following classes of vulnerabilities **unrepresentable**:

- Secret-dependent branching
- Secret-dependent memory addressing
- Non-constant-time equality on secrets
- Accidental logging or formatting of secrets
- Implicit declassification
- Secret-dependent work-factor leaks
- Optimizer-induced constant-time violations

The result is a language environment suitable for implementing:
- lib-crypto–grade primitives
- Hybrid post-quantum protocols
- Constant-time AEAD, KEM, and signature schemes
- High-assurance cryptographic protocols with strong misuse resistance

---

## Conclusion

By elevating secrecy, constant-time discipline, and address-formation constraints into **primitive compiler concepts**, Silica can provide security guarantees that conventional lib-crypto libraries must enforce through convention, audits, and brittle coding patterns.

This positions Silica as a strong candidate for next-generation cryptographic engineering—especially for **quantum-prepared** systems—where correctness, side-channel resistance, and misuse resistance are enforced by construction rather than policy.
