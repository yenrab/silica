# Silica Formal Verification Specification

## Related Documents

| Document | Purpose |
|----------|---------|
| [silica-specification.md](silica-specification.md) | Silica language specification |
| [silica-compiler&language-specification.jsonld](silica-compiler&language-specification.jsonld) | Language design source (JSON-LD) |
| [sir_design_spec.md](sir_design_spec.md) | SIR structure; verification proof terms may feed into or precede SIR |
| [sir_generator_formalization_spec.md](sir_generator_formalization_spec.md) | Compiler pipeline; verification phases integrate before sir_generator |

---

## 1. Introduction

### 1.1 Overview

This specification defines formal verification for the Silica programming language using the Curry-Howard correspondence. Verification is performed by the compiler through a layered proof calculus. Each layer corresponds to a distinct aspect of Silica's semantics; the compiler runs verification phases sequentially, producing proof terms that witness correctness.

**Key principle:** Types are propositions; terms are proofs; type checking is proof checking. The typing derivation is the proof object.

### 1.2 Design Goals

- **Compiler-native verification**: No external verification backends; the compiler performs all verification.
- **Curry-Howard integration**: Verification is type checking; proof terms are typing derivations.
- **Layered design**: Separate calculi for values, effects, regions, and actors; each layer is independently understandable and verifiable.
- **Sequential composition**: Phases run in a fixed order; each phase consumes the output of the previous phase.
- **Alignment with Silica**: Verification calculus maps directly to Silica's type system, effect system, memory model, and actor model as defined in the language specification.

### 1.3 Scope

- **In scope**: Definition of the layered proof calculi, sequential composition pipeline, mapping from Silica to proof terms, and integration points in the compiler.
- **Out of scope**: External SMT solvers or proof assistants; verification of the compiler itself; runtime verification.

---

## 2. Curry-Howard Foundation

### 2.1 Correspondence

Under Curry-Howard:

| Programming | Logic |
|-------------|-------|
| Type | Proposition |
| Term | Proof |
| Type checking | Proof checking |
| Typing derivation | Proof object |
| Function type A → B | Implication A ⇒ B |
| Product type A × B | Conjunction A ∧ B |
| Sum type A + B | Disjunction A ∨ B |
| Dependent function Πx:A. B(x) | Universal quantification ∀x:A. B(x) |
| Dependent pair Σx:A. B(x) | Existential quantification ∃x:A. B(x) |

### 2.2 Application to Silica

Silica's design aligns well with Curry-Howard:

- **Explicit types** (per silica-compiler&language-specification.jsonld §1.3): Every term has an explicit type annotation, making propositions explicit.
- **Effect types** (`proc[ε]`): Propositions about which effects a computation may perform.
- **Region types** (`ref(R, S, T)`): Propositions about memory ownership and isolation.
- **No type inference**: Typing derivations are straightforward to construct and extract.

Contracts (`pre:`, `post:`, `inv:`) are encoded as type-level propositions; a well-typed program that satisfies the contracts is a proof of those propositions.

---

## 3. Sequential Composition Pipeline

Verification is performed by the compiler in four sequential phases. Each phase consumes the output of the previous phase and produces an enriched representation. The final phase yields a proof term that witnesses the program's correctness.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Silica AST (parsed, with optional pre/post/inv contract annotations)         │
└─────────────────────────────────────────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Phase 1: Value Layer (λΠ + sums/products)                                   │
│  - Type-check pure expressions                                              │
│  - Check pattern exhaustiveness and guard consistency                       │
│  - Build proof terms for value-level contracts                              │
│  Output: Value-typed AST with proof derivations                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Phase 2: Effect Layer (Moggi monadic metalanguage)                         │
│  - Check proc types and do-sequencing                                       │
│  - Propagate and verify effect sets                                         │
│  - Build proof terms for effect safety                                      │
│  Output: Effect-annotated AST with effect proof derivations                 │
└─────────────────────────────────────────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Phase 3: Region Layer (Separation logic / indexed monad)                    │
│  - Check region lifetimes and reference dependencies                        │
│  - Verify ref/buf usage and region isolation                                │
│  - Build proof terms for memory safety                                      │
│  Output: Region-safe AST with region proof derivations                      │
└─────────────────────────────────────────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Phase 4: Actor Layer (Session types / typed channels)                      │
│  - Check spawn, send, recv usage                                             │
│  - Verify message types and actor protocols                                  │
│  - Build proof terms for concurrency safety                                  │
│  Output: Fully verified AST with complete proof term                         │
└─────────────────────────────────────────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Proof term : Proposition                                                   │
│  (The typing derivation is the proof; the proposition is the specification) │
└─────────────────────────────────────────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Downstream: SIR generation, AArch64 emission                       │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Invariant:** Each phase assumes its predecessors have succeeded. If any phase fails, verification fails and the compiler reports an error. No phase is skipped.

---

## 4. Layer 1: Value Calculus (λΠ + Sums and Products)

### 4.1 Role

The value calculus handles pure expressions: functions, tuples, records, variants, lists, pattern matching, and value-level contracts. It corresponds to Silica's expression forms that do not involve effects, regions, or actors.

### 4.2 Calculus Definition

**Types (propositions):**

```
A, B  ::=
  | Unit                    -- unit type
  | Bool                    -- booleans
  | Int8 | Int16 | Int32 | Int64
  | Float16 | Float32 | Float64
  | Char | String | Atom
  | A × B                   -- product (tuples, records)
  | A + B                   -- sum (variants)
  | A → B                   -- function
  | Πx:A. B(x)              -- dependent function (contracts)
  | Σx:A. B(x)              -- dependent pair (existential)
  | List[A]                 -- list (empty + cons)
```

**Terms (proofs):**

```
t, u  ::=
  | ()                      -- unit
  | true | false
  | n                       -- integer literal
  | (t, u)                  -- pair
  | inl t | inr u           -- sum injections
  | λx:A. t                 -- abstraction
  | t u                     -- application
  | case t of { p₁ → u₁; p₂ → u₂; ... }  -- case analysis
  | [] | t :: u             -- list constructors
```

**Typing judgment:** Γ ⊢ t : A

### 4.3 Silica Mapping

| Silica | Value Calculus |
|--------|----------------|
| `unit`, `bool`, `int64`, etc. | Unit, Bool, Int64, etc. |
| `(a, b)` | A × B |
| `{ x: T1, y: T2 }` | Record as product |
| `Variant1 \| Variant2` | A + B |
| `List[ElementType]` | List[A] |
| `fn(x: A) -> B { e }` | λx:A. t where t : B |
| `case e of { p -> e' }` | case analysis with pattern refinement |
| `pre: ⊢ P` | Precondition as Π-context or refinement |
| `post: ⊢ Q` | Postcondition as dependent return type |

### 4.4 Pattern Matching and Exhaustiveness

Per silica-compiler&language-specification.jsonld (PatternMatching, CaseExpressions):

- **Pattern exhaustiveness**: All values of the matched type must be covered.
- **Guard exhaustiveness**: For each pattern, guards must cover all cases or a catch-all must be provided.
- **Typed catch-all**: `_: T -> e` covers remaining values.

In the value calculus, case analysis uses eliminators. Each branch refines the type in that branch. Exhaustiveness is verified by ensuring the union of branch patterns covers the type.

### 4.5 Trait Polymorphism

Silica uses traits, not generics (per design principles). Traits map to **type-class evidence**:

- `impl Display for int64` provides evidence that `int64` implements `Display`.
- `fn print(x: Display)` becomes `fn print(x: A, ev: Display A)` where `ev` is the evidence.
- The proof term carries the evidence; verification checks that evidence exists for each trait constraint.

### 4.6 Output

Phase 1 produces a value-typed AST where each node is annotated with its type and a proof derivation. Contracts (pre/post) are represented as dependent types; the derivation witnesses that the term satisfies them.

---

## 5. Layer 2: Effect Calculus (Moggi Monadic Metalanguage)

### 5.1 Role

The effect calculus handles `proc[ε]` types, `do` blocks, and effect propagation. It corresponds to Silica's process monad and effect system (silica-compiler&language-specification.jsonld §9, EffectSystem).

### 5.2 Calculus Definition

**Computation types:**

```
T^ε  ::=  M^ε A

Where:
- A is a value type (from Layer 1)
- ε is an effect set: { device_io, concurrency, mem(Space), mailbox(Msg), atomic }
```

**Terms:**

```
m, n  ::=
  | return t                -- pure computation, ε = []
  | x <- m; n               -- bind (do-sequencing)
  | op(v)                   -- effectful operation
```

**Typing judgment:** Γ ⊢ m : T^ε

**Key rules:**

```
Γ ⊢ t : A
────────────────────────
Γ ⊢ return t : M^[] A

Γ ⊢ m : M^ε₁ A    Γ, x:A ⊢ n : M^ε₂ B
────────────────────────────────────────
Γ ⊢ x <- m; n : M^(ε₁ ∪ ε₂) B

Γ ⊢ m : M^ε₁ A    ε₁ ⊆ ε₂
────────────────────────────
Γ ⊢ m : M^ε₂ A    (subeffecting)
```

### 5.3 Silica Mapping

| Silica | Effect Calculus |
|--------|-----------------|
| `proc[device_io] int64` | M^{device_io} Int64 |
| `proc[mem(normal), concurrency] ref(...)` | M^{mem(normal), concurrency} Ref(...) |
| `do x <- e1; e2 end` | x <- m; n |
| `e` (pure in do block) | return t |
| Effect declaration in signature | Effect set ε in T^ε |

### 5.4 Effect Categories

Per the language specification:

- **device_io**: I/O operations
- **concurrency**: spawn, send, recv
- **mem(Space)**: Memory allocation in region/space
- **mailbox(Msg)**: Actor mailbox operations
- **atomic**: Atomic operations

Subeffecting: a computation with effects ε₁ can be used where ε₂ is required if ε₁ ⊆ ε₂.

### 5.5 Output

Phase 2 produces an effect-annotated AST. Each computation term carries its effect set and a derivation showing effect safety. The proof term extends the Layer 1 proof with effect-tracking structure.

---

## 6. Layer 3: Region Calculus (Separation Logic / Indexed Monad)

### 6.1 Role

The region calculus handles `region(R, Space)`, `ref(R, Space, T)`, `buf(R, Space, T, N)`, and region isolation. It corresponds to Silica's memory model (silica-specification.md §12, silica-compiler&language-specification.jsonld MemoryModel).

### 6.2 Calculus Definition

**Heap (separation logic):**

```
H  ::=  ∅ | R ↦ σ | H₁ ∗ H₂

Where:
- R is a region identifier
- σ is the region's contents (store)
- ∗ is separating conjunction
```

**Typing judgment:** Γ; H ⊢ e : T; H'

The judgment states that in heap H, expression e has type T and produces heap H' (possibly modified).

**Key rules:**

```
────────────────────────────────────────────
Γ; H ⊢ alloc_region(Space) : region(R, Space); H ∗ (R ↦ ∅)

Γ; H ∗ (R ↦ σ) ⊢ alloc_ref(r, v) : ref(R, Space, T); H ∗ (R ↦ σ')

Γ; H ∗ (R ↦ σ) ⊢ read_ref(ref) : T; H ∗ (R ↦ σ)

Γ; H, R:scope ⊢ e : T; H'
scope_current ≤ scope_R
────────────────────────────────────────────
Γ; H ⊢ e : T; H'   (region in scope)
```

**Lifetime environment:** L maps region identifiers to lexical scopes. References cannot outlive their region.

### 6.3 Silica Mapping

| Silica | Region Calculus |
|--------|-----------------|
| `region(R, Space)` | Region type with phantom R |
| `ref(R, Space, T)` | Reference in heap H with R ↦ σ |
| `buf(R, Space, T, N)` | Buffer in region R |
| `alloc_region(normal)` | alloc_region rule |
| `alloc_ref(r, v)` | alloc_ref rule |
| `read_ref` / `write_ref` | read_ref / write_ref rules |
| Region isolation | H₁ ∗ H₂ (no overlap) |

### 6.4 Lifetime Analysis

The existing lifetime analysis (silica-specification.md §12.1.4) uses:

- **Lifetime environment** L: R ↦ scope
- **Dependency set** D: refs and their creation scopes
- **Scope exit**: Deallocate regions, check no dangling refs

The region calculus formalizes this as separation logic. The proof term records the heap structure and scope constraints.

### 6.5 Output

Phase 3 produces a region-safe AST. Each region and reference operation carries a derivation showing heap safety and lifetime validity. The proof term extends Layer 2 with region/heap structure.

---

## 7. Layer 4: Actor Calculus (Session Types / Typed Channels)

### 7.1 Role

The actor calculus handles `spawn`, `send`, `recv`, `actor_ref`, and message protocols. It corresponds to Silica's actor model (silica-compiler&language-specification.jsonld §15, §16, ActorModel, Message Passing).

### 7.2 Calculus Definition

**Session types (simplified):**

```
P  ::=
  | !T. P      -- send T, then P
  | ?T. P      -- receive T, then P
  | end        -- protocol complete
```

**Actor type:** `actor_ref(P)` — reference to an actor following protocol P.

**Terms:**

```
spawn(s, beh)   -- create actor with initial state s and behavior beh
send(a, msg)    -- send message to actor a
recv()          -- receive message
cast(a, msg)    -- fire-and-forget send
```

**Typing:** `send` and `recv` must match the expected protocol. Dual endpoints: if one side sends T, the other receives T.

### 7.3 Silica Mapping

| Silica | Actor Calculus |
|--------|----------------|
| `actor_ref` | actor_ref(P) for some protocol P |
| `spawn(init, handler)` | spawn with state and behavior |
| `send(actor_ref, msg)` | send; msg type must match protocol |
| `recv()` | receive; type inferred from context |
| `cast(actor_ref, msg)` | fire-and-forget send |

### 7.4 Marker Traits

Per the specification, `ActorState` and `ActorMessage` are marker traits. In the calculus, these constrain the types that can be used as state and messages. Verification checks that spawn/send/recv use types implementing these traits.

### 7.5 Output

Phase 4 produces a **fully verified AST**. The proof term includes session-type structure for actor communication. The complete proof witnesses type safety, effect safety, region safety, and concurrency safety.

---

## 8. Integration with the Compiler Pipeline

### 8.1 Placement

Verification phases run **after** parsing and **before** SIR generation:

```
Lexer → Parser → [Phase 1: Value] → [Phase 2: Effect] → [Phase 3: Region] → [Phase 4: Actor] → SIR Generator → ...
```

If any verification phase fails, the compiler stops and reports errors. No SIR is generated.

### 8.2 Proof Term Output

The compiler may optionally emit proof terms:

- **Internal**: Proof terms held in memory for debugging or further analysis.
- **Export**: Optional serialization of proof terms (e.g., for auditing or tooling).

Proof terms are not required for code generation; the verified AST is sufficient. The proof term is the Curry-Howard witness that the program is correct.

### 8.3 Contract Syntax (Future)

When contracts are added to Silica, the syntax uses logical notation with `pre:`, `post:`, and `inv:` tags. This design is LLM-native (math-heavy, no inference) yet human-readable. The ⊢ symbol denotes "must hold"; all variable types come from the function signature or explicit quantification.

**Basic form:**

```silica
fn div(a: int64, b: int64) -> int64
  pre:  ⊢ b ≠ 0
  post: ⊢ result = a ÷ b
{
  a / b
}
```

The value calculus encodes `pre:` as a precondition (refinement on input) and `post:` as a dependent return type. Phase 1 verifies these.

**Symbol conventions:** `≠`, `≤`, `≥`, `∧`, `∨`, `¬`, `⇒`, `÷`, `·`; typed quantifiers `∀x:T.` and `∃x:T.` when bound variables are introduced. No type inference: parameter and return types are explicit in the signature; `result` has the return type.

**Examples — Basic arithmetic:**

```silica
fn mod(a: int64, b: int64) -> int64
  pre:  ⊢ b ≠ 0
  post: ⊢ 0 ≤ result ∧ result < |b|
{
  a % b
}

fn clamp(x: int64, lo: int64, hi: int64) -> int64
  pre:  ⊢ lo ≤ hi
  post: ⊢ result ≥ lo ∧ result ≤ hi
  post: ⊢ result = x ∨ result = lo ∨ result = hi
{
  case x < lo of {
    true  -> lo;
    false -> case x > hi of {
      true  -> hi;
      false -> x
    }
  }
}
```

**Examples — Recursive functions:**

```silica
fn factorial(n: int64) -> int64
  pre:  ⊢ n ≥ 0
  post: ⊢ result ≥ 1
{
  case n ≤ 1 of {
    true  -> 1;
    false -> n * factorial(n - 1)
  }
}

fn sum_to(n: int64, acc: int64) -> int64
  pre:  ⊢ n ≥ 0
  pre:  ⊢ acc ≥ 0
  post: ⊢ result = acc + n·(n+1)÷2
{
  case n == 0 of {
    true  -> acc;
    false -> sum_to(n - 1, acc + n)
  }
}
```

**Examples — With effects:**

```silica
fn safe_read(path: string) -> int64 proc[device_io]
  pre:  ⊢ path ≠ ""
  post: ⊢ result ≥ 0
{
  read_file_as_int(path)
}

fn main() -> atom proc[device_io]
  post: ⊢ result = :ok ∨ result = :fail
{
  x: int64 <- read_int();
  print_int(x);
  :ok
}
```

**Examples — Boolean operations:**

```silica
fn implies(p: bool, q: bool) -> bool
  post: ⊢ result = (¬p ∨ q)
{
  case p of {
    true  -> q;
    false -> true
  }
}

fn xor(a: bool, b: bool) -> bool
  post: ⊢ result = (a ∧ ¬b) ∨ (¬a ∧ b)
{
  a != b
}
```

**Examples — With quantification:**

```silica
fn max(a: int64, b: int64) -> int64
  post: ⊢ result ≥ a ∧ result ≥ b
  post: ⊢ result = a ∨ result = b
{
  case a ≥ b of {
    true  -> a;
    false -> b
  }
}

fn abs(n: int64) -> int64
  post: ⊢ result ≥ 0
  post: ⊢ result = n ∨ result = -n
{
  case n ≥ 0 of {
    true  -> n;
    false -> -n
  }
}
```

**Examples — Multiple postconditions, product return:**

```silica
fn div_rem(a: int64, b: int64) -> (int64, int64)
  pre:  ⊢ b ≠ 0
  post: ⊢ result.0 = a ÷ b
  post: ⊢ result.1 = a % b
  post: ⊢ a = result.0 * b + result.1
{
  (a / b, a % b)
}
```

### 8.4 Error Reporting

Verification errors follow the Silica compiler error format (silica-specification.md §1.6):

- Human-readable description
- Location (file, line, column)
- Error code
- Specification section reference

New error codes for verification failures should be allocated in the appropriate range (e.g., verification errors as a distinct category).

### 8.5 Constraint Inventory Sanity Check

When contracts are present, an independent **constraint inventory** provides a sanity check that verification discharged all declared obligations. This section formalizes the extraction, inventory, and completeness condition.

**Constraint extraction.** Let AST denote the parsed abstract syntax tree. Define the extraction function:

ℰ : AST → 𝒫(Φ)

where Φ is the set of contract propositions. For each declaration d ∈ AST:

- φ_pre ∈ ℰ(AST) iff d has a `pre:` clause with proposition φ_pre
- φ_post ∈ ℰ(AST) iff d has a `post:` clause with proposition φ_post
- φ_inv ∈ ℰ(AST) iff d has an `inv:` clause with proposition φ_inv

**Constraint inventory.** The inventory is the image of extraction:

𝒞 ≝ ℰ(AST) = {φ₁, φ₂, …, φₙ}

Each φᵢ is tagged with its source location ℓᵢ and kind κᵢ ∈ {pre, post, inv}.

**Discharged obligations.** Let 𝒟 denote the set of propositions witnessed in the proof term π produced by the verification pipeline:

𝒟 ≝ {ψ ∣ π ⊢ ψ}

That is, 𝒟 is the set of propositions for which the typing derivation contains a proof.

**Completeness condition (sanity check).** Verification is complete iff every extracted constraint is discharged:

𝒞 ⊆ 𝒟  ⟺  ∀φ ∈ 𝒞. φ ∈ 𝒟

**Sanity-check procedure.** After verification succeeds:

1. Compute 𝒞 = ℰ(AST).
2. Extract 𝒟 from the proof term π (or from the typing derivations).
3. Verify 𝒞 ⊆ 𝒟.
4. If 𝒞 ⊈ 𝒟, report a verification bug: some declared constraint was not discharged.

**Idempotence.** The sanity check does not re-verify; it cross-checks. The type checker remains the sole proof engine. The check ensures:

|𝒞| = |𝒞 ∩ 𝒟|  ⟹  no constraint omitted

---

## 9. Implementation Phases

### 9.1 Recommended Order

1. **Phase 1 (Value)**: Implement first. Covers the majority of Silica programs. Establishes the proof-term infrastructure.
2. **Phase 2 (Effect)**: Add effect checking with proof terms. Builds on existing effect system.
3. **Phase 3 (Region)**: Integrate with existing lifetime analysis. Formalize as separation logic.
4. **Phase 4 (Actor)**: Add last. Requires session types or simplified channel typing.

### 9.2 Incremental Adoption

- Programs without regions can skip Phase 3 (or Phase 3 trivially succeeds).
- Programs without actors can skip Phase 4.
- The pipeline is designed so that each phase can be implemented and tested independently.

---

## 10. References

- **Silica Language Specification**: [silica-specification.md](silica-specification.md)
- **Language Design Source**: [silica-compiler&language-specification.jsonld](silica-compiler&language-specification.jsonld)
- **Moggi, E.**: "Notions of computation and monads" (1989)
- **Separation Logic**: Reynolds, O'Hearn, et al.
- **Session Types**: Honda, Vasconcelos, Kubo (1998)

---

## 11. Implementation Plan: From Current Codebase to Full Specification

This section provides a detailed, step-wise plan to move from the current Silica compiler to a full implementation of the formal verification specification. Each step is incremental, testable, and preserves backward compatibility where possible.

### 11.1 Current State (Baseline)

**Pipeline** (from `src/main.silica`):

```
Lexer → Parser → Type checker → Effect checker → SIR generator → Emitter
```

**Existing components:**

| Component | Location | Status |
|-----------|----------|--------|
| Lexer | `src/lexer/` | Implemented |
| Parser | `src/parser/` | Implemented |
| Type checker | `src/type_checker/` | Implemented (types, traits, patterns) |
| Effect checker | `src/effect_checker/` | Implemented |
| SIR generator | `src/sir_generator/` | Implemented |
| Emitter | `src/emitter/` | Implemented |

**Gaps relative to verification spec:**

- No explicit verification phase numbering or pipeline documentation
- No region/lifetime analysis phase (region analysis may be minimal or deferred)
- No actor protocol verification phase
- No proof-term extraction or derivation structure
- Effect checker output is validated AST; no per-term effect annotations
- No verification-specific error code range

---

### 11.2 Step 1: Introduce Verification Pipeline Wrapper

**Goal:** Refactor the pipeline to use verification phase terminology without changing behavior.

**Tasks:**

1. **Create `verification_pipeline.silica`** (or equivalent module) that:
   - Invokes type checker as Phase 1
   - Invokes effect checker as Phase 2
   - Passes output to SIR generator
   - Returns verification result (success/failure with phase number)

2. **Update `main.silica`** to call the verification pipeline instead of type checker and effect checker directly. The call sequence remains: `type_check_program` → `check_program_effects` → `emit_program`.

3. **Document** in code comments that Phase 1 = type checker, Phase 2 = effect checker. Add a note that Phase 3 (Region) and Phase 4 (Actor) are placeholders.

**Deliverables:** Pipeline refactored; behavior unchanged; verification phase mapping documented.

**Verification:** All existing tests pass. Compilation output identical.

---

### 11.3 Step 2: Allocate Verification Error Codes

**Goal:** Reserve error codes for verification phases so errors can reference the formal verification spec.

**Tasks:**

1. **Update `error_codes.md`** and `programmer_tools/silica-error-code-scheme.jsonld`:
   - Allocate range E6000–E6999 for verification errors (or integrate into existing phase ranges)
   - Define codes: E6000 (VerificationErrorDefault), E6001 (Phase1ValueError), E6002 (Phase2EffectError), E6003 (Phase3RegionError), E6004 (Phase4ActorError)
   - Add `specificationSection` references to this document (e.g., `spec:§11`)

2. **Update `diagnostics_core.silica`** (or equivalent) to support verification error format with phase number.

3. **Wrap existing type/effect errors** so they report as Phase 1 or Phase 2 verification failures when routed through the verification pipeline.

**Deliverables:** Error code scheme extended; verification errors distinguishable.

**Verification:** Trigger a type error and an effect error; confirm they report with verification phase context.

---

### 11.4 Step 3: Enrich Effect Checker Output (Phase 2 Annotations)

**Goal:** Have the effect checker attach effect set ε to each effectful term, not just validate.

**Tasks:**

1. **Define `EffectAnnotatedAST`** (or extend existing AST) with optional `effect_set` on:
   - Function call expressions
   - `do` block expressions (when present)
   - Effectful primitives (alloc_region, alloc_ref, spawn, send, recv, etc.)

2. **Extend effect checker** to populate effect annotations during traversal. Each term that performs effects gets its ε attached.

3. **Ensure SIR generator** can consume effect-annotated AST. SIR already expects effects on calls (sir_design_spec §2); verify the generator receives them from the enriched AST or from type context.

4. **Add tests** for programs with multiple effects; verify annotations are correct.

**Deliverables:** Effect-annotated AST; Phase 2 output matches §5.5.

**Verification:** Inspect AST after effect checking; confirm effect sets on effectful terms.

---

### 11.5 Step 4: Implement Phase 3 (Region Layer)

**Goal:** Add region/lifetime verification as Phase 3, or formalize existing analysis.

**Tasks:**

1. **Assess existing region support:**
   - Check if parser/type checker handle `region(R, Space)`, `ref(R, Space, T)`, `alloc_region`, `alloc_ref`, `read_ref`, `write_ref`
   - Check if any lifetime analysis exists (e.g., in specification-analysis-state.md)
   - If minimal: implement from scratch per §6. If partial: formalize and extend.

2. **Implement region phase** (`region_checker/` or `verification_phase3_region.silica`):
   - Build lifetime environment L (region → scope)
   - Build dependency set D (ref → creation scope)
   - Traverse AST; at `alloc_region`, add R to L
   - At `alloc_ref`, add ref to D; verify region exists
   - At scope exit, check no dangling refs
   - Per silica-specification.md §12.1.4 (Static Region Lifetime Analysis)

3. **Integrate into pipeline:** Run Phase 3 after Phase 2, before SIR. Programs without regions: Phase 3 trivially succeeds (no alloc_region/ref).

4. **Add region error codes** (e.g., E6003 sub-codes): reference outlives region, invalid region, etc.

**Deliverables:** Phase 3 implemented; region-using programs verified; non-region programs unaffected.

**Verification:** Test with region-using Silica code; test with pure programs (Phase 3 no-op).

---

### 11.6 Step 5: Implement Phase 4 (Actor Layer)

**Goal:** Add actor protocol verification as Phase 4.

**Tasks:**

1. **Define actor protocol model:**
   - Start with simplified session types or typed channels
   - `spawn`, `send`, `recv`, `cast` must use types implementing `ActorState` and `ActorMessage` (marker traits)
   - Optionally: infer or annotate protocol P for `actor_ref(P)`

2. **Implement actor phase** (`actor_checker/` or `verification_phase4_actor.silica`):
   - Verify `spawn(init, handler)` — init implements ActorState, handler has correct signature
   - Verify `send(actor_ref, msg)` — msg implements ActorMessage
   - Verify `recv()` — context expects message type
   - Verify `cast(actor_ref, msg)` — msg implements ActorMessage
   - Check trait constraints (ActorState, ActorMessage) per §7.4

3. **Integrate into pipeline:** Run Phase 4 after Phase 3, before SIR. Programs without actors: Phase 4 trivially succeeds.

4. **Add actor error codes** (e.g., E6004 sub-codes): invalid spawn, message type mismatch, etc.

**Deliverables:** Phase 4 implemented; actor-using programs verified; non-actor programs unaffected.

**Verification:** Test with actor examples; test with non-actor programs (Phase 4 no-op).

---

### 11.7 Step 6: Add Proof-Term Infrastructure (Optional)

**Goal:** Enable extraction of typing derivations as proof terms, per Curry-Howard.

**Tasks:**

1. **Define proof-term data structures:**
   - `ProofTerm` (or equivalent): represents a typing derivation
   - Structure mirrors the calculi: value terms, effect terms, region terms, actor terms
   - Can be minimal (e.g., just a tree of rule applications) or full (lambda terms)

2. **Extend Phase 1 (type checker)** to optionally build derivation during type checking:
   - Each typing rule produces a derivation node
   - On success, return (typed AST, derivation)

3. **Extend Phase 2** to attach effect derivation to the proof term.

4. **Extend Phases 3 and 4** to attach region and actor derivations.

5. **Add compiler flag** (e.g., `--emit-proof-terms`) to enable proof-term construction and optional serialization.

**Deliverables:** Proof terms produced when enabled; no change to default compilation.

**Verification:** Compile with `--emit-proof-terms`; verify proof term structure for a simple program.

---

### 11.8 Step 7: Update Downstream for Enriched AST

**Goal:** Ensure SIR generator and emitter work with the fully verified, enriched AST.

**Tasks:**

1. **Review SIR generator input contract** (sir_generator_formalization_spec §3.1):
   - Input is type-checked, effect-checked AST
   - Extend to: type-checked, effect-checked, region-verified, actor-verified AST
   - Ensure SIR generator receives effect annotations (from Step 3) and any region metadata

2. **Update sir_design_spec** if needed: clarify that SIR input is verification-phase output.

3. **Update silica-compiler-creation-order.md** to show the verification pipeline:
   ```
   Lexer → Parser → [Phase 1: Value] → [Phase 2: Effect] → [Phase 3: Region] → [Phase 4: Actor] → SIR → Optimization → Code gen → ...
   ```

**Deliverables:** Documentation and contracts updated; pipeline coherent.

**Verification:** Full compile of representative programs; no regressions.

---

### 11.9 Step 8: Contract Syntax (Future)

**Goal:** Add optional `pre:`, `post:`, `inv:` contract syntax when the language design is ready.

**Tasks:**

1. **Parser:** Add grammar for `pre: ⊢ expr`, `post: ⊢ expr`, `inv: ⊢ expr` on function declarations (logical notation per §8.3).

2. **Phase 1 extension:** When contracts present, encode as refinements/dependent types; verify during type checking. Use abstract interpretation or constraint propagation (see §4.3 mapping).

3. **Error reporting:** Contract violation uses verification error codes.

**Deliverables:** Contract syntax supported; Phase 1 verifies contracts when present.

**Note:** This step is deferred until contract syntax is approved for the language. The verification pipeline works without it.

---

### 11.10 Step Summary and Ordering

| Step | Description | Depends on | Estimated effort |
|------|-------------|------------|------------------|
| 1 | Verification pipeline wrapper | — | Small |
| 2 | Verification error codes | — | Small |
| 3 | Effect annotations (Phase 2 enrichment) | 1 | Medium |
| 4 | Phase 3 (Region) | 1, 2 | Medium–Large |
| 5 | Phase 4 (Actor) | 1, 2 | Medium |
| 6 | Proof-term infrastructure | 1–5 | Medium |
| 7 | Downstream updates | 3–5 | Small |
| 8 | Contract syntax | 1–7 | Large (future) |

**Recommended sequence:** 1 → 2 → 3 → 7 (minimal viable verification), then 4 → 5 (full four-phase), then 6 (proof terms). Step 8 when contracts are approved.

---

### 11.11 Success Criteria for Full Implementation

- [ ] All four verification phases run in sequence before SIR generation.
- [ ] Phase 1 = current type checker (or equivalent); Phase 2 = current effect checker (or equivalent).
- [ ] Phase 3 verifies region lifetimes for programs using regions.
- [ ] Phase 4 verifies actor usage for programs using actors.
- [ ] Effect-annotated AST produced; SIR receives effect info.
- [ ] Verification errors use dedicated codes and reference this spec.
- [ ] Proof-term extraction available when enabled.
- [ ] Existing test suite passes; no regressions.

---

## Document History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-02-19 | Initial specification; Option A layered design; sequential composition pipeline |
| 1.1 | 2025-02-19 | Added §11 Implementation Plan: step-wise migration from current codebase |
| 1.2 | 2025-02-20 | §8.3: Math-heavy contract syntax (pre/post/inv, ⊢, no inference); §8.5: Constraint inventory sanity check; examples throughout |
| 1.3 | 2025-02-20 | Replaced LaTeX with Unicode in §8.5 (𝒞, 𝒟, ℰ, Φ, φ, ⊢, ⊆, ⟺, etc.) |
