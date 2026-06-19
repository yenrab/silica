# Bitwise Operators Implementation Plan

**Date**: June 19, 2026  
**Status**: Not started — design and execution plan only  
**Audience**: Compiler implementers and LLM agents generating Silica compiler changes  

**References**:

- [silica-specification.md](../silica-specification.md) — §2.2.4 (operators), §4 (integral types)
- [silica-specification-additional.md](../silica-specification-additional.md) — §3.3 (redundant algebraic ops; already names `&` and `|` in examples)
- [sir_design_spec.md](../../sir_design_spec.md) — §7 (SIR primitives)
- [sir_optimization_spec.md](../../sir_optimization_spec.md) — strength reduction (multiply by power of two)
- [graph_representation_design.md](../graph_representation_design.md) — §6 (`DenseBitsetGraph`; §6.4 gate)
- [standard_data_structures_implementation_plan.md](standard_data_structures_implementation_plan.md) — Phase 3 Step 3.3 (DenseBitset deferred pending bit ops)
- [silica-compiler-code-organization.md](../silica-compiler-code-organization.md) — compiler module layout
- [cpu_topology_implementation_plan.md](cpu_topology_implementation_plan.md) — phased compiler pipeline template

---

## 1. Objective

Add **keyword bitwise operators** to the Silica surface language and compiler pipeline so generated code can manipulate packed bit fields using ordinary expressions — without reusing sum-type `|` or atom-delimiter `&`.

**Chosen operator set (Option 1 — keyword family):**

| Role | Keyword | Example |
|------|---------|---------|
| Bitwise OR | **`bor`** | `new_word <- word bor mask` |
| Bitwise AND | **`band`** | `(word band mask) != 0` |
| Bitwise NOT | **`bnot`** | `clear_mask <- bnot mask` |
| Left shift | **`shl`** | `mask <- one shl bit_offset` |
| Right shift | **`shr`** | `tag <- value shr 56` |

**Primary consumer**: `DenseBitsetGraph` in [graph_representation_design.md](../graph_representation_design.md) §6. Until these operators exist end-to-end, Step 3.3 of [standard_data_structures_implementation_plan.md](standard_data_structures_implementation_plan.md) remains deferred and `DenseMatrixGraphDirected[mem(S)]` stays the fallback.

**Non-goals (this plan)**:

- Surface syntax using `|`, `&`, `<<`, or `>>` as expression operators
- Alternative unary spelling **`compl`**
- SVE2 vector bitwise intrinsics (spec appendix only today)
- Changing sum-type spelling (`A | B`) or logical keywords (`and`, `or`, `not`)

---

## 2. Design rationale

### 2.1 Why not `|`, `&`, `<<`, `>>`?

| Character | Existing Silica role |
|-----------|---------------------|
| `\|` | Sum / variant types (`Ok(int64) \| Error(string)`), pattern alternatives, atom delimiter |
| `&` | Atom-name delimiter (not an expression operator) |
| `<<` / `>>` | Not surface operators today; [silica-specification-additional.md](../silica-specification-additional.md) §3.2 notes shifts are internal strength-reduction only |

Reusing `\|` or `&` for bitwise ops would collide with type-forming syntax and atom lexing.

### 2.2 Why keywords (`bor`, `band`, `bnot`, `shl`, `shr`)?

- **LLM-friendly**: No confusion with sum `\|` or logical `or` / `and`.
- **Easiest lexing**: Same mechanism as `and` / `or` — reserved words, no multi-character maximal-munch rules.
- **Consistent with Silica**: Logical operators are already keywords, not C-style symbols.
- **Distinct names**: `bor` vs `or`, `band` vs `and` — readable in generated graph code.

### 2.3 Relationship to logical operators

| Operator | Operands | Result | Meaning |
|----------|----------|--------|---------|
| `and` | `bool`, `bool` | `bool` | Logical conjunction |
| `or` | `bool`, `bool` | `bool` | Logical disjunction |
| `not` | `bool` | `bool` | Logical negation |
| **`band`** | integral `T`, integral `T` | `T` | Bitwise AND on two's-complement bit pattern |
| **`bor`** | integral `T`, integral `T` | `T` | Bitwise OR |
| **`bnot`** | integral `T` | `T` | Bitwise complement |
| **`shl`** | integral `T`, shift amount | `T` | Left shift |
| **`shr`** | integral `T`, shift amount | `T` | Right shift (see §2.5) |

**Compile-time error** if `bor` / `band` / `bnot` / `shl` / `shr` are applied to `bool`, `float*`, or mismatched integral widths.

**Important correction**: Today `int64_binary_ops.silica` maps lexeme `"and"` / `"or"` to SIR prims that the **int64 emitter lowers to boolean** logic (`logical.silica` + `CSET`). That path must **not** be reused for `bor` / `band`. Bitwise emission must produce **integral** results in the destination register (`ORR`, `AND`, `LSL`, `LSR` / `ASR` on AArch64).

### 2.4 Operand and result types

**Phase 1 scope (minimum for DenseBitsetGraph):**

- **`uint64`** only for `bor`, `band`, `bnot`, `shl`, `shr`.
- Dense bitset graph word storage must use `uint64` once these operators are available.

**Phase 2 extension (optional, same rules):**

- `int64`, `int8`, `int16`, `int32`, `uint8`, `uint16`, `uint32` — mirror existing per-width arithmetic modules and narrow emitters (`prims_narrow.silica`).

**Shift amount (right operand of `shl` / `shr`):**

- Type **`int64`**, same as `%` remainder RHS convention.
- **Compile-time error** if the shift amount is a compile-time constant and is negative or ≥ bit width of the left operand (64 for `uint64`, 32 for `uint32`, etc.).
- **Runtime behavior** for dynamic out-of-range amounts is undefined; Phase 1 does not add runtime shift bounds checks.

**`bor` / `band`:**

- Left and right operands must have the **same** integral type.
- Result type equals operand type.

### 2.5 Shift semantics

| Left type | `shl` | `shr` |
|-----------|-------|-------|
| `uint64`, `uint32`, `uint16`, `uint8` | Logical left shift | **Logical** right shift (zero-fill high bits) |
| `int64`, `int32`, `int16`, `int8` | Logical left shift on bit pattern | **Logical** right shift on bit pattern when signed widths are added |

**Rationale**: `DenseBitsetGraph` stores **`uint64` words as unsigned bit bags** (see graph design §6.2). Logical shifts match `read_buf` / `write_buf` word manipulation and match AArch64 `LSL` / `LSR` on `X` registers.

**Future**: If signed arithmetic shift (`ASR`) is needed, add a separate keyword (e.g. `sar`) in a follow-on plan — do not overload `shr` silently.

### 2.6 Precedence and associativity

New keywords fit into the existing binary-operator precedence ladder:

```text
(highest)
  unary: not, negate_*, bnot
  *, /, %
  +, -
  shl, shr          ← new; bind tighter than band/bor
  band              ← new; binds tighter than bor
  bor               ← new
  <, >, <=, >=, ==, !=
  and, or
(lowest)
```

**Associativity**: `shl`, `shr`, `bor`, and `band` are **left-associative** (same as `+` and `*`). `bnot` is unary prefix and binds with other unary operators.

**Parentheses** always override, as today.

### 2.7 Effects

`bor`, `band`, `bnot`, `shl`, and `shr` are **pure** — empty effect `[]`, same as `add` / `sub`.

### 2.8 Atom delimiter note

`bor`, `band`, `bnot`, `shl`, and `shr` are **keywords**, not identifiers. They terminate atom names the same way `and` and `or` do. Update the atom delimiter list so `:bor` tokenizes as the atom delimiter `:` followed by keyword `bor`, not as a single atom literal. `bor` without `:` remains the bitwise operator keyword in expression context.

---

## 3. Surface language specification (normative for this plan)

Add to [silica-specification.md](../silica-specification.md) §2.2.4:

##### Bitwise Operators

```text
"bor"  "band"  "bnot"  "shl"  "shr"
```

**Informative examples** (DenseBitsetGraph-style):

```silica
bit_index: int64 <- from * node_count + to;
word_index: int64 <- bit_index / 64;
bit_offset: int64 <- bit_index % 64;
one: uint64 <- 1;
zero: uint64 <- 0;
mask: uint64 <- one shl bit_offset;
word: uint64 <- read_buf(g.words, word_index);
new_word: uint64 <- word bor mask;
_: atom <- write_buf(g.words, word_index, new_word);
present: int64 <- case (word band mask) != zero of { true -> 1; false -> 0 };
cleared: uint64 <- word band (bnot mask);
```

Update [graph_representation_design.md](../graph_representation_design.md) §6.3–6.4 pseudocode to use `bor`, `band`, `bnot`, and `shl` instead of `|`, `&`, `~`, and `<<` when this plan completes.

---

## 4. SIR primitive design

Add to [sir_design_spec.md](../../sir_design_spec.md) §7 (new subsection **7.x Bitwise**):

| PrimOp | SIRType | Args | Effect |
|--------|---------|------|--------|
| `bor` | uint64, … | (a, b) | [] |
| `band` | uint64, … | (a, b) | [] |
| `bnot` | uint64, … | (a) | [] |
| `shl` | uint64, … | (a, amount) | [] |
| `shr` | uint64, … | (a, amount) | [] |

SIR term shape: same as existing binary arithmetic prims (`kind: 6`, `name: "bor"`, etc.).

**Do not** alias `bor` → existing `or` prim or `band` → existing `and` prim.

---

## 5. Current compiler snapshot

| Area | Status |
|------|--------|
| Lexer | No `bor` / `band` / `bnot` / `shl` / `shr` keywords or token kinds |
| Parser | `capability_expr_int64_ops.silica` recognizes `and` / `or` only as extra ops |
| Type checker | Logical op inference for `"and"` / `"or"` only; no bitwise rules |
| SIR generator | `uint64_binary_ops.silica` — no `bor` / `band` / `bnot` / `shl` / `shr` |
| Emitter | `logical.silica` lowers `and` / `or` to **boolean**; no integral `ORR`/`AND`/shift helpers for user prims |
| Trials | No `bitwise_addition` trial directory |
| Stdlib graphs | `DenseBitsetGraph` type expansion exists; operations not generated |

---

## 6. Implementation phases

Follow the same pipeline order as [cpu_topology_implementation_plan.md](cpu_topology_implementation_plan.md): **spec freeze → lexer → parser → type checker → effect checker → SIR → emitter → trials**.

### Phase A — Specification and SIR freeze

**Goal**: Lock operator spellings, semantics, precedence, and SIR prim names before code changes. Phase A is explicitly responsible for converting the defaults in §9 into final decisions in the specification patches.

**Tasks**

1. Patch [silica-specification.md](../silica-specification.md) §2.2.4 (bitwise keywords) and precedence appendix if one exists.
2. Patch [sir_design_spec.md](../../sir_design_spec.md) §7 with `bor`, `band`, `bnot`, `shl`, `shr` rows.
3. Patch [graph_representation_design.md](../graph_representation_design.md) §6.3–6.4 to reference `bor` / `band` / `bnot` / `shl` and remove the “only when `\|`, `&`, shift available” wording in favor of “when `bor` / `band` / `bnot` / `shl` / `shr` are available”.
4. Add cross-reference from [standard_data_structures_implementation_plan.md](standard_data_structures_implementation_plan.md) Step 3.3 to this plan.

**Exit criteria**

- Spec text is internally consistent.
- No open semantic questions for Phase 1 (`uint64`, logical `shr`, prefix `bnot`).

**Primary artifacts**: Spec sections above; this document §2–§4.

---

### Phase B — Lexer

**Goal**: Recognize five new reserved words.

**Tasks**

1. Add token kinds to `src/lexer/lexer_token_kind.silica`:
   - `bor_keyword`, `band_keyword`, `bnot_keyword`, `shl_keyword`, `shr_keyword`
   - Assign stable numeric IDs (append after existing keyword IDs; document in a one-line comment).
2. Map spellings in `src/lexer/lexer_keywords.silica`:
   - `"bor"`, `"band"`, `"bnot"`, `"shl"`, `"shr"` → corresponding kinds.
3. Add keywords to the reserved-word predicate (same list as `and` / `or`) so they cannot bind as identifiers.
4. Rebuild lexer; confirm `:bor` lexes as atom delimiter `:` plus `bor_keyword`, while expression `bor` lexes as keyword.

**Files**

- `src/lexer/lexer_token_kind.silica`
- `src/lexer/lexer_keywords.silica`

**Tooling**: `compiler-building-tools/silica-lexer-code-generator.jsonld`

**Exit criteria**

- Token stream for `one shl 2 bor three` contains bitwise operator tokens, not identifiers.

---

### Phase C — Parser (capabilities)

**Goal**: Allow bitwise keywords in binary expression positions for integral trials.

**Tasks**

1. Extend the unsigned integral expression capability used for `uint64` operations:
   - `is_bitwise_op/2` for `bor`, `band`, `shl`, and `shr`.
   - unary recognition for prefix `bnot`.
   - Include in `is_op_token/2` and `initial_roles` for `role_op`.
2. If there is no dedicated `capability_expr_uint64_ops.silica`, add or extend the current integral capability path so Phase 1 supports `uint64` first.
3. Extend other width capabilities (`int32`, `uint32`, …) in Phase 2 only.
4. Verify constraint runner does not treat `bor` as a call name or module prefix.

**Files**

- `src/parser/capabilities/capability_expr_int64_ops.silica` (only if this remains the shared integral capability path)
- `src/parser/capabilities/capability_expr_uint64_ops.silica` (new or existing)
- `src/parser/constraint_extract.silica` (if binary-op lexeme lists are duplicated)

**Tooling**: `compiler-building-tools/silica-parser-code-generator.jsonld`

**Exit criteria**

- Parser accepts parenthesized and unparenthesized chains such as `one shl 2 bor three band four` with precedence from §2.6.

---

### Phase D — Type checker

**Goal**: Sound integral typing for bitwise operators.

**Tasks**

1. Add `is_bitwise_op_lexeme/1` in `src/type_checker/expressions/type_checker_expressions.silica` (`"bor"`, `"band"`, `"shl"`, `"shr"`) and unary `bnot` recognition.
2. **Typing rules**:
   - `bor` / `band`: both operands same integral type `T` → result `T`.
   - `bnot`: operand integral `T` → result `T`.
   - `shl` / `shr`: left operand integral `T`, right operand `int64` → result `T`.
   - Reject `bool` and float operands with **E2003** (or dedicated error code if preferred).
3. Extend `type_checker_tuple_decompose_helpers.silica` inference (`is_bitwise_op_infer`) for kind-3 binary expr decomposition.
4. **Constant shift validation**: when RHS is a compile-time constant, error if `< 0` or `≥ width(T)`.
5. Ensure `and` / `or` on integrals remain errors (or stay on the legacy bool path) — do not widen logical ops to integers.

**Files**

- `src/type_checker/expressions/type_checker_expressions.silica`
- `src/type_checker/expressions/type_checker_tuple_decompose_helpers.silica`

**Tooling**: `compiler-building-tools/silica-typechecker-code-generator.jsonld`

**Exit criteria**

- `word bor mask` type-checks when both are `uint64`.
- `(word band mask) != 0` type-checks.
- `true bor false` fails at compile time.

---

### Phase E — Effect checker

**Goal**: Bitwise prims are pure.

**Tasks**

1. Confirm no effect annotation is required for expressions using `bor` / `band` / `bnot` / `shl` / `shr`.
2. Add a short note to effect checker docs if a central prim→effect table exists.

**Files**

- `src/effect_checker/effect_checker_core.silica` (audit only unless a whitelist exists)

**Exit criteria**

- Bitwise expressions compile inside `produces pure` blocks without extra effects.

---

### Phase F — SIR generator

**Goal**: Lower source operators to SIR bitwise prims.

**Tasks**

1. Extend `src/sir_generator/terms/uint64_binary_ops.silica`:

   ```text
   "bor" -> ("bor", "uint64")
   "band" -> ("band", "uint64")
   "shl" -> ("shl", "uint64")
   "shr" -> ("shr", "uint64")
   ```

2. Extend the unary SIR path for `bnot` on `uint64`.
3. Confirm `terms.silica` dispatch reaches the correct `*_binary_ops` module (already routes by operand type).
4. **Do not** map `"and"` / `"or"` on integral values to bitwise prims.

**Files**

- `src/sir_generator/terms/uint64_binary_ops.silica`
- Unary term lowering module used for prefix operators
- `src/sir_generator/terms/terms.silica` (audit)

**Tooling**: `compiler-building-tools/silica-sir_generator_builder.jsonld`

**Exit criteria**

- SIR dump (`.scout`) for a trial shows `prim(bor, uint64, …)` not `prim(or, …)`.

---

### Phase G — Emitter (Apple Silicon)

**Goal**: Lower SIR bitwise prims to correct AArch64 integral instructions.

**Tasks**

1. Add `src/emitter/apple_silicon/terms/prims/bitwise.silica`:
   - `emit_bor_op(dest, reg_prefix)` → `ORR`
   - `emit_band_op(dest, reg_prefix)` → `AND`
   - `emit_bnot_op(dest, reg_prefix)` → `MVN` alias or `ORN dest, XZR, src`
   - `emit_shl_op(dest, reg_prefix)` → `LSL` (register form; constant shifts via existing arithmetic helpers when RHS is const)
   - `emit_shr_op(dest, reg_prefix)` → `LSR` for `uint64` Phase 1
2. Wire into `prims_uint64.silica`:

   ```text
   "bor"  -> bitwise@emit_bor_op(dest, "X")
   "band" -> bitwise@emit_band_op(dest, "X")
   "shl"  -> bitwise@emit_shl_op(dest, "X")
   "bnot" -> bitwise@emit_bnot_op(dest, "X")
   "shr"  -> bitwise@emit_shr_op(dest, "X")
   ```

3. **Leave** `prims_int64.silica` `"and"` / `"or"` on `logical@emit_*` (boolean path) unchanged.
4. Register `bitwise.silica` in the emitter `silica.config` / Makefile chain used by `prims_uint64`.
5. For constant shift amounts fitting AArch64 immediate encodings, optional peephole in emitter or SIR opt — not required for Phase 1 if register shifts are always emitted.

**Files**

- `src/emitter/apple_silicon/terms/prims/bitwise.silica` (new)
- `src/emitter/apple_silicon/terms/prims/prims_uint64.silica`
- Emitter module Makefile / `silica.config` entries

**Tooling**: `compiler-building-tools/silica-emitter_builder.jsonld`

**Exit criteria**

- Trial binary uses `ORR` / `AND` / `LSL` / `LSR` on `X` registers for bitset-style mask code.
- No erroneous `CSET` after `ORR` on integral `bor` (contrast with `logical.silica`).

---

### Phase H — Trials and integration

**Goal**: Lock behavior with integrate-ready trials before stdlib `DenseBitsetGraph` codegen.

**Tasks**

1. Create `trials/bitwise_addition/`:
   - `silica.config` — minimal compiler slice + new trials
   - `Makefile` — `integrate` target consistent with `trials/int64_addition/`
2. **Positive trials** (suggested names):
   - `bitwise_bor_band.silica` — `three bor five`, `seven band three`, print or bind results; bind operands as `uint64` using the repo's current literal spelling or typed literal convention
   - `bitwise_bnot.silica` — `bnot zero`, `seven band (bnot two)` with `uint64` operands
   - `bitwise_shl_shr.silica` — `one shl 4`, `sixteen shr 2` with `uint64` left operands
   - `bitwise_precedence.silica` — `one shl 2 bor three band one` vs `(one shl 2) bor (three band one)` with `uint64` operands
   - `bitwise_dense_bitset_mask.silica` — reproduces graph §6 mask / set / test without full graph module
3. **Negative trials** (error enforcement subdirectory or local expects-fail):
   - `true bor false` — type error
   - `one shl 64` — shift out of range (constant), with `one` bound as `uint64`
4. Run `make integrate` from `trials/bitwise_addition/`; store `.sout` goldens.
5. Document trial README with mapping to graph §6 operations.

**Exit criteria**

- All positive trials green on Apple Silicon bootstrap path.
- Negative trials fail with expected error codes.
- `.scout` / `.sams` artifacts reviewed for correct prim names and assembly opcodes.

---

### Phase I — Stdlib follow-on (separate execution; blocked on H)

**Goal**: Unblock [standard_data_structures_implementation_plan.md](standard_data_structures_implementation_plan.md) Step 3.3.

**Tasks**

1. Add `graph_dense_bitset_directed.silica` (and undirected variant if required) using `bor` / `band` / `bnot` / `shl` / `shr`.
2. Add `trials/graph_addition/graph_dense_bitset_*_trial.silica`.
3. Update completion table: `DenseBitsetGraph` → Partial / Complete when trials pass.
4. Retarget `graph_representation_design.md` §6 pseudocode in generated sources only (design doc already updated in Phase A).

**Exit criteria**

- Dense bitset trials pass; matrix fallback remains documented but no longer the only dense unweighted option.

---

## 7. Dependency order

```text
A (spec + SIR freeze)
  → B (lexer)
  → C (parser)
  → D (type checker)
  → E (effect checker, parallel with D)
  → F (SIR generator)
  → G (emitter)
  → H (trials)
  → I (DenseBitsetGraph stdlib — optional separate PR)
```

Phases **B–G** must land together for a coherent compiler; partial merges will produce parse-without-codegen or codegen-without-typecheck failures.

---

## 8. Verification checklist

| Check | Phase |
|-------|-------|
| `:bor` tokenizes as `:` plus `bor_keyword` | B |
| `bor` keyword not usable as identifier | B |
| Precedence: `bnot` > `shl` > `band` > `bor` > `and` | C, H |
| `uint64 bor uint64` → `uint64` | D |
| `bool bor bool` → compile error | D, H |
| SIR prim names `bor` not `or` | F |
| Assembly `ORR` without bool `CSET` for `bor` | G |
| Mask idiom `one shl k` with `one: uint64` | H |
| Graph mask/set/test trial | H |

---

## 9. Defaults To Finalize In Phase A

| # | Question | Proposed default |
|---|----------|------------------|
| 1 | Phase 1 widths | **`uint64` only** |
| 2 | `shr` semantics | **Logical** (`LSR`) for Phase 1 `uint64` |
| 3 | Dedicated error code for bad shift amount? | Reuse **E2003** with message suffix unless a bit-shift code is added |
| 4 | Unary `bnot` for `clear_edge`? | **Include `bnot` in Phase 1** |
| 5 | SIR optimization: strength-reduce `x * (one shl k)` to `x shl k`? | **Optional** follow-on in [sir_optimization_spec.md](../../sir_optimization_spec.md); not blocking |

---

## 10. Related tooling index

| Compiler area | Tool JSON-LD |
|---------------|----------------|
| Lexer | `compiler-building-tools/silica-lexer-code-generator.jsonld` |
| Parser | `compiler-building-tools/silica-parser-code-generator.jsonld` |
| Type checker | `compiler-building-tools/silica-typechecker-code-generator.jsonld` |
| Effect checker | `compiler-building-tools/silica-effect-code-generator.jsonld` |
| SIR generator | `compiler-building-tools/silica-sir_generator_builder.jsonld` |
| Emitter | `compiler-building-tools/silica-emitter_builder.jsonld` |
| Cross-phase planning | `compiler-building-tools/silica-compiler-phase-planning-tool.jsonld` |

---

## 11. Completion tracking

| Phase | Description | Status |
|-------|-------------|--------|
| A | Spec + SIR freeze | Not started |
| B | Lexer keywords | Not started |
| C | Parser capabilities | Not started |
| D | Type checker | Not started |
| E | Effect checker | Not started |
| F | SIR generator | Not started |
| G | Emitter (AArch64) | Not started |
| H | Trials | Not started |
| I | DenseBitsetGraph stdlib | Blocked on H |

**Last updated**: June 19, 2026
