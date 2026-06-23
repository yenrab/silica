# Bootstrap Retirement and Self-Host Migration Plan

**Purpose:** Identify every change needed so `silica-compiler` can build and maintain itself without `silica-bootstrap-compiler`, replacing bootstrap-era internal data structures and workarounds with the standard generated families described in [standard_data_structures_implementation_plan.md](standard_data_structures_implementation_plan.md) and [data_structures_as_traits.md](data_structures_as_traits.md).

**Scope:** `compiler/silica-compiler/src/` (compiler internals), build system, and compiler-facing trials. Does **not** duplicate stdlib family work already tracked in the standard-data-structures plan (Phases 1–10 there).

**Authority:**

- [data_structures_as_traits.md](data_structures_as_traits.md) — constructor function records, trait dispatch, no named constructor-record aliases
- [standard_data_structures_implementation_plan.md](standard_data_structures_implementation_plan.md) — generated family status and Phase 0.4/0.5 obligations
- [list_implementation_design.md](../list_implementation_design.md) — when `List[T, S]` remains correct (AST chains, token streams)

**Current state (audit):**

| Layer | Bootstrap (`silica-boot`) | Self-hosted (`silica-compiler`) |
| ----- | ------------------------- | -------------------------------- |
| Compiler executable | `src/Makefile` builds `main.silica` → `main.ll` via bootstrap; links `libsilica_compiler.a` from Rust | Used by all `trials/*/`, already builds `standard_data_structures/` |
| Subdir Makefiles | `lexer/`, `parser/`, `type_checker/` (partial), `sir_generator/`, `emitter/`, `effect_checker/` | — |
| Internal ADTs | `data_structures/bst.silica` (naive unbalanced BST); ~20 custom `List*` cons-cell structs | `type_checker_collections.silica` already understands `OrderedSet`/`OrderedMap`/`Heap`/`DirectedGraph` bracket types |
| Type aliases | `type TokenKind = int64` in `lexer_token_kind.silica` (only `type` alias in `src/`) | — |

This document intentionally contains no Silica source code. It is written as fine-grained work items that an LLM can follow when migrating compiler internals.

## Global Rules

1. **Do not remove bootstrap until a self-hosted stage produces a passing `make integrate` on the full compiler tree.**
2. **Lists stay lists** where order and immutability are the model (tokens, AST declaration lists, SIR function lists). **Maps/sets** replace linear-scan or naive-BST lookups keyed by `string` or ordered scalars.
3. **Use standard generated APIs:** `btree_nodeid@empty({ compare_key, compare_value })`, `OrderedMap@get`, etc.—not width-specialized bootstrap exports like `empty[int64, mem(normal)]/0` inside compiler code.
4. **No new named constructor-record struct types** (per traits doc Constructor Function Record Rule).
5. **Each migration step** adds or extends a trial under `trials/` before deleting the old path.
6. **Retire duplicated logic** only after self-hosted cross-module ABI is verified (see Phase 2).

## Implementation Order

1. **Phase 0** — Audit, workaround inventory, stdlib smoke for compiler-internal `use`.
2. **Phase 1** — Build system flip (`silica.config.compiler`, self-host link).
3. **Phase 2** — Remove bootstrap workarounds (ABI, inference, lexer/rodata).
4. **Phase 3** — Replace `data_structures/bst.silica` in emitter literal pools.
5. **Phase 4** — Replace linear-scan association lists (symbol/effect tables).
6. **Phase 5** — Type alias and bootstrap API cleanup.
7. **Phase 6** — Self-host integrate suite and bootstrap removal.

Phases 3 and 4 can proceed in parallel once Phase 1 stage **`build-selfhost`** exists. Phase 2 should precede deleting duplicated lookup functions (Step 2.2).

## Phase 0 — Prerequisites (blocking self-host build flip)

These are compiler/runtime fixes or stdlib gaps that block compiling the full `src/` tree with `silica-compiler` instead of `silica-boot`.

### Step 0.1 — Inventory bootstrap-only build assumptions

**Actions:**

1. Document that `src/Makefile` uses **two build models**:
   - Per-file `.silica` → `.ll` via `silica-boot` (compiler executable)
   - `silica.config` batch → `.sams` via `silica-compiler` (trials, `standard_data_structures/`)
2. List all Makefiles still pointing at bootstrap (7 found):
   - `src/Makefile`, `src/lexer/Makefile`, `src/effect_checker/Makefile`, `src/sir_generator/Makefile`, `src/emitter/Makefile` (+ parent rules for `parser/`, `type_checker/`, `module_checker/`, `ffi/`, `trait_checker/` via `src/Makefile`)
3. Identify bootstrap runtime dependency: `libsilica_compiler.a` linked into `silica-compiler` executable (`src/Makefile` lines 13–14, 143–148).
4. Remove or relocate stray duplicates not in the build graph:
   - `src/btree_set_nodeid.silica` (old bootstrap copy; canonical: `standard_data_structures/btree_set_nodeid.silica`)
   - `standard_data_structures/heap_dary_min copy.silica`, `inline_type_expansion copy.silica` if present

**Exit criteria:**

- Written inventory checked into this plan's Completion Tracking table.

### Step 0.2 — Self-host compile feasibility matrix (read-only audit)

**Actions:** For each bootstrap workaround comment in `src/`, classify as:

- **A — Compiler bug** (must fix in `silica-compiler` before self-host)
- **B — Source workaround** (can delete once self-host compiles correctly)
- **C — Intentional** (unrelated to bootstrap, e.g. FailureReporter "bootstrap" in supervisor sense)

**Known bootstrap workaround sites (27 comments across 20 files):**

| ID | File | Issue | Class |
| -- | ---- | ----- | ----- |
| W01 | `build_output.silica` | `concat` drops second arg in inline `case` | A/B |
| W02 | `effect_checker_core.silica`, `effect_serializer.silica` | `len("")` returns non-zero | A |
| W03 | `lexer_runner.silica` | `skip_whitespace` sometimes no-ops | A |
| W04 | `atom_rodata.silica`, `int_rodata.silica` | `.asciz` / escaped-quote bugs | B (emitter style) |
| W05 | `type_checker_expressions_string_calls.silica` | string pattern-match issues | A/B |
| W06 | `type_checker_tuple_decompose_helpers.silica` | tuple binding decomposition bug; cross-module string return segfault | A |
| W07 | `sir_generator/terms/terms.silica` | cross-`.ll` string return segfault → duplicated lookup fns | A |
| W08 | `type_checker_core.silica` | sret/stack on deep module TC; `(bool,string)` tuple return ABI | A |
| W09 | `sir_generator/declarations/qualified_call_mangler.silica`, `trait_specialization.silica` | structural-vs-Named inference for `List`-typed fields | A/B |
| W10 | `parser_tuples.silica` | token.kind false-matches grouping kinds | A/B |
| W11 | `type_checker_expressions.silica` | `call_name_is_module_qualified` misclassification; `tc_` prefix collision | A/B |
| W12 | `parser_ast.silica` | nominal rebuild helpers for structural records | B |
| W13 | `parser/constraint_extract.silica` | tuple order for bootstrap codegen; stack overflow guard on case branches | A/B |
| W14 | `sir_generator/terms/identifiers.silica` | nested tuple destructuring codegen | A/B |
| W15 | `emitter/.../term_emitter.silica` | 6-param `emit_const` issue | B |
| W16 | `type_checker_core.silica` | `lookup_symbol_found` exists because `""` string compare unreliable | A |

**Exit criteria:**

- Every W-id has an owner phase (2 or 3) and a trial or integrate check.

### Step 0.3 — Stdlib readiness for compiler-internal use

**Actions (defer to [standard_data_structures_implementation_plan.md](standard_data_structures_implementation_plan.md) where noted):**

1. **OrderedMap / OrderedSet (nodeid):** Phase 5–8 module API complete — **ready** for `string` keys and `int64` values with constructor records.
2. **Heap:** Phase 9 still partial (`heap_binary_min` has width-specialized bootstrap exports, no constructor-record `empty/1`) — **blocker** only if compiler adopts heaps internally (optional; not required for Phase 4).
3. **Graph:** Step 1.1 complete — not needed for compiler symbol tables.
4. Add **`src/silica.config.compiler_internal`** (new) listing only the stdlib modules compiler code will `use`: `btree_nodeid`, `OrderedMap`, adapters, `compare_string` helper module if needed.

**Exit criteria:**

- Minimal stdlib batch config compiles under existing `silica-compiler` with one smoke trial importing `OrderedMap[string, int64, mem(normal)]`.

## Phase 1 — Build system: bootstrap → self-host staging

### Step 1.1 — Introduce dual-build Makefile switch

**Actions:**

1. Add `BOOTSTRAP_COMPILER` and `HOST_COMPILER` variables to `src/Makefile`.
2. Add target `build-selfhost`: requires existing `silica-compiler` (from last bootstrap build), compiles `main.silica` with **self-hosted** compiler.
3. Keep `build-bootstrap` as default until Step 1.4 passes.
4. Mirror switch in subdir Makefiles OR **prefer consolidation**: stop per-file `.ll` subdir builds for the executable path (see Step 1.2).

**Exit criteria:**

- `make build-selfhost` produces `silica-compiler` without invoking `silica-boot` on the critical path (runtime link may still use bootstrap `.a` temporarily).

### Step 1.2 — Unify compiler build on `silica.config` batch mode

**Rationale:** `main.silica` already implements the batch pipeline (`silica.config`, dependency sort, `.sams` emission). Trials use this successfully; subdir `.ll` per-module builds are redundant for the linked executable (`src/Makefile` comment: "main.o contains everything").

**Actions:**

1. Create `src/silica.config.compiler` listing all compiler `.silica` units in dependency order (lexer → parser → … → `main.silica`), mirroring `SEARCH_PATHS` from `src/Makefile`.
2. Add `make assembly-selfhost`: run `silica-compiler` in `src/` with that config → `.sams` → `.o` → link.
3. Replace `libsilica_compiler.a` with self-emitted `__silica_runtime.sams` from the host compiler (align with trial Makefiles).
4. Retire subdir `.o` production from the **executable** critical path (keep optional for incremental dev if useful).

**Exit criteria:**

- Self-hosted `silica-compiler` binary built entirely through `.sams` pipeline; no `main.ll` on critical path.

### Step 1.3 — Fix stack / resource limits for self-host input size

**Actions:**

1. Preserve/enlarge main-thread stack flags (`-Wl,-stack_size,0x10000000` in `src/Makefile`) in the new link recipe.
2. Add integrate trial compiling the full `src/silica.config.compiler` graph (compile-only or run `--version` smoke if available).

**Exit criteria:**

- Full compiler source batch compiles without stack overflow (constraint_extract case-depth guard W13 may become unnecessary—track in Phase 2).

### Step 1.4 — Bootstrap retirement gate

**Actions:**

1. Document **fixed-point procedure:** bootstrap → host₁ → host₂; compare artifacts or run full `make integrate`.
2. Remove `silica-bootstrap-compiler` from default `src/Makefile` once gate passes.
3. Update `design_documents/README.md` bootstrap-analysis entry to "historical / retired."

**Exit criteria:**

- Clean clone can build `silica-compiler` with only LLVM toolchain + prior release binary (or documented one-time seed binary).

## Phase 2 — Remove bootstrap workarounds in compiler source

Work items map to W-ids from Step 0.2. Fix in `silica-compiler` first where class A; then simplify source.

### Step 2.1 — String and empty-string reliability (W02, W16)

**Files:** `effect_checker_core.silica`, `effect_serializer.silica`, `type_checker_core.silica` (`lookup_symbol_found`).

**Actions:**

1. Add error-enforcement or unit trials for `len("") == 0` and reliable `""` equality.
2. Once green under self-host, collapse `(bool, string)` lookup pairs to single string return where safe (W08).

**Exit criteria:**

- W02/W16 trials pass; `lookup_symbol_found` documented as temporary or removed.

### Step 2.2 — Cross-module string / tuple ABI (W06, W07, W08, W14)

**Files:** `type_checker_core.silica`, `type_checker_tuple_decompose_helpers.silica`, `sir_generator/terms/terms.silica`, `sir_generator/terms/identifiers.silica`.

**Actions:**

1. Fix emitter/ABI for returning `string` across module boundaries on Apple Silicon.
2. Delete **four duplicated** symbol lookup implementations:
   - `type_checker_core@lookup_fn_type_by_name_and_module`
   - `type_checker_expressions@lookup_fn_type_by_name_and_module_local`
   - `type_checker_tuple_decompose_helpers@lookup_fn_type_by_name_and_module_infer`
   - `sir_generator/terms/terms@lookup_fn_type_by_name_and_module_sir`
3. Route all through `type_checker_core` exports.

**Exit criteria:**

- Single lookup implementation; `make integrate` green for module-checker and SIR trials.

### Step 2.3 — Parser / type-checker inference quirks (W09–W11, W13)

**Files:** `qualified_call_mangler.silica`, `trait_specialization.silica`, `parser_tuples.silica`, `type_checker_expressions.silica`, `constraint_extract.silica`.

**Actions:**

1. Fix structural-vs-Named inference for `List`-typed fields so manual `Declaration { … }` rebuilds in `clone_decl_with_first_param_type` are unnecessary.
2. Revisit `branch_depth` stack guard in `constraint_extract.silica` after self-host stack sizing.
3. Remove `tc_` prefix duplication if parser/type_checker symbol collision is resolved.

**Exit criteria:**

- Trait specialization and qualified-call mangler trials unchanged; reduced manual AST cloning.

### Step 2.4 — Lexer and build_output (W01, W03, W04)

**Actions:**

1. Fix root causes or confirm self-host already correct; simplify `sams_path_from_source`, rodata `.byte` emission, whitespace skipping.

**Exit criteria:**

- Workaround comments removed or marked "historical."

## Phase 3 — Replace `data_structures/bst.silica`

**Current module:** Unbalanced BST with `BstNode { value, index, next_left, next_right }`, numeric/string compare via `string_parse@string_to_int64` hack for lexicographic order.

**Target:** `OrderedMap[string, int64, mem(normal)]` backed by `btree_nodeid` (or CSR after insert paths needed for compile-time table growth—prefer nodeid for incremental insert).

### Step 3.1 — Shared compile-time map helper module

**Actions:**

1. Add `compiler_maps.silica` under `src/` (or `standard_data_structures/` if shared with trials):
   - `compare_string(a, b) -> :less | :equal | :greater` (atom contract)
   - `empty_string_int64_map()` → `btree_nodeid@empty({ compare_key: compare_string, compare_value: compare_int64 })`
   - `map_insert_or_get_index(map, key) -> { map, index }`
   - `map_index_of(map, key) -> int64` (uses `OrderedMap@get` / `btree_nodeid@get`)
2. No new type aliases; use explicit `OrderedMap[string, int64, mem(normal)]` at bindings.

**Exit criteria:**

- Trial `compiler_string_index_map.silica` in `trials/` covers insert, lookup, duplicate key.

### Step 3.2 — Migrate emitter literal pools (5 modules)

| Module | Current | Migration |
| ------ | ------- | ----------- |
| `emitter/.../atom_table.silica` | `bst@bst_insert_string` + `ListAtomLexeme` | OrderedMap for lexeme→index; keep `ListAtomLexeme` for rodata order |
| `emitter/.../int64_literal_pool.silica` | `bst@bst_insert` (int64 keys) | `OrderedMap[int64, int64, mem(normal)]` with numeric comparator |
| `emitter/.../float32_literal_pool.silica` | BST on stringified float | Map keyed by `string` canonical form or dedicated float compare |
| `emitter/.../float64_literal_pool.silica` | same | same |
| `emitter/.../int_rodata.silica` | BST for int8–int64 print tables | Same pattern per width |

**Actions:**

1. Replace `use bst` with `use btree_nodeid`, `use OrderedMap`, `use compiler_maps`.
2. Remove `BstNode` fields from table structs; store `OrderedMap[…]` in table record.
3. Update atom/literal pool trials under `trials/`.

**Exit criteria:**

- Emitter integrate trials pass; `data_structures/bst.silica` has zero `use` references.

### Step 3.3 — Migrate `string_literal_pool.silica` (list scan → map)

**Current:** O(n) list dedup for UTF-8 correctness.

**Actions:**

1. Use `OrderedMap[string, int64, mem(normal)]` with **raw string equality** via `compare_string` on full UTF-8 lexeme (not BST int64 hack).
2. Retain immutability contract and rodata emission order via auxiliary list (same as atom table).

**Exit criteria:**

- String literal / UTF-8 trials pass.

### Step 3.4 — Delete `data_structures/bst.silica`

**Exit criteria:**

- Module removed; `SEARCH_PATHS` / Makefiles drop `-I data_structures` if only BST lived there (`string_parse.silica`, `string_escapes.silica` remain).

## Phase 4 — Replace linear-scan association lists (compiler hot paths)

**Principle:** Keep cons-cell lists for **ordered sequences**; replace **name → type** maps.

### Step 4.1 — Symbol table (`ListSymbolEntry`)

**Files:** `type_checker_core.silica` (+ all consumers: TC, SIR, module_checker paths).

**Current:** O(n) linked list; `add_symbol` prepends; shadowing by linear scan.

**Target shape:** `symbols: OrderedMap[string, SymbolEntry, mem(normal)]` with key = symbol name and value = inline `{ type_name, declared_effects, is_effect_alias, source_module }`.

**Actions:**

1. Add `SymbolTable` wrapper module using constructor record + `btree_nodeid@insert` / `@get`.
2. Migrate `add_symbol`, `lookup_symbol*`, `lookup_fn_*` to map operations.
3. Preserve shadowing semantics: **prepend-scoped** lists may need **nested map stack** (list of maps) rather than single global map—design choice:
   - **Option A (recommended):** `List[OrderedMap[string, SymbolEntry, mem(normal)], mem(normal)]` scope stack
   - **Option B:** Keep list for lexical scopes, map only for export/program-level tables

**Exit criteria:**

- All `type_checker/` and `sir_generator/` trials pass; no `ListSymbolEntry` in `type_checker_core.silica`.

### Step 4.2 — Effect and type environments

**Files:** `effect_checker_core.silica` (`ListEffectEntry`, `ListTypeEntry`).

**Actions:** Same pattern as 4.1 for effect name → effects and type name → type string.

**Exit criteria:**

- Effect checker integrate suite green.

### Step 4.3 — Module and FFI lookup tables

**Files:** `module_checker_core.silica` (`ListParsedFile` stays list; path→program index may become map), `ffi_sidecar_loader.silica` (linear `ListSirString` scan for wrapper lookup).

**Actions:** Map keyed by module path / symbol name where lookup is by key.

**Exit criteria:**

- Module-check and FFI trials pass.

### Step 4.4 — Parser constraint helpers (lower priority)

**Files:** `constraint_core.silica` (`ListInt`, `ListConstraint`), `constraint_extract.silica` (`lookup_outer_binding_type` on `ListTupleDecomposeBinding`).

**Actions:** Evaluate case-by-case; many are **ordered** constraint stacks—not all should become maps. Only migrate true keyed lookups.

**Exit criteria:**

- Documented per-site keep/migrate decisions in Completion Tracking.

## Phase 5 — Type alias and API cleanup

### Step 5.1 — Remove `type TokenKind = int64`

**File:** `lexer/lexer_token_kind.silica`

**Actions:**

1. Replace `TokenKind` with bare `int64` in signatures OR use `struct TokenKinds` fields directly (already exists).
2. Confirm `silica-compiler` does not require `type` aliases for this path (only alias in all of `src/`).

**Exit criteria:**

- No `type` declarations in compiler `src/`; lexer trials pass.

### Step 5.2 — Stop depending on bootstrap stdlib exports in compiler `use` graph

**Actions:**

1. Ensure compiler sources never `use` width-specialized `empty[int64, mem(normal)]` from heap/graph modules.
2. Compiler-internal stdlib usage goes only through trait-oriented nodeid map/set APIs (Phase 0.3 config).

**Exit criteria:**

- Grep for width-specialized bootstrap exports under compiler `use` paths returns zero.

## Phase 6 — Validation, documentation, bootstrap removal

### Step 6.1 — Self-host integrate suite

**Actions:**

1. Add `trials/self_host_addition/` with Makefile running full `src/silica.config.compiler` through host compiler.
2. Add fixed-point script (documented): `host_n` compiles `host_{n+1}`; fail on byte mismatch or trial regression.

**Exit criteria:**

- CI/local `make integrate` includes self-host trial.

### Step 6.2 — Update design docs

**Actions:**

1. Cross-link this plan from `standard_data_structures_implementation_plan.md` (new section: "Compiler internal consumption").
2. Update `data_structures_as_traits.md` Generated-family snapshot for compiler adoption.
3. Mark bootstrap-analysis doc historical.

### Step 6.3 — Remove `silica-bootstrap-compiler` from repo workflow

**Actions:**

1. Delete bootstrap references from all Makefiles.
2. Archive or remove `silica-bootstrap-compiler` crate per project policy (outside this plan's file scope).

**Exit criteria:**

- No `silica-boot` in default build path.

## Suggested First PR (minimal vertical slice)

1. Step 0.1 inventory + delete stray `src/btree_set_nodeid.silica`
2. Step 3.1 `compiler_maps.silica` + trial
3. Step 3.2 migrate **only** `atom_table.silica` off BST
4. Step 1.1 `build-selfhost` Makefile target (still link bootstrap runtime)

That proves stdlib maps inside the compiler and begins bootstrap retirement without boiling the ocean.

## Risks

1. **Chicken-and-egg:** Last bootstrap build required to seed first self-host; document seed binary policy.
2. **Compile time / memory:** Full compiler batch may stress host compiler; may need staged `silica.config` shards before monolithic config.
3. **Scope creep:** Phase 4 parser constraint migration—default **keep lists** unless profiling shows need.
4. **CSR vs nodeid:** Emitter tables grow incrementally; nodeid B-tree insert is sufficient for Phase 3; CSR only if compile-time memory becomes an issue.

## Completion Tracking

| Area | Status | Notes |
| ---- | ------ | ----- |
| Phase 0 audit | Not started | 27 bootstrap workaround comments catalogued in Step 0.2 |
| Build system dual-mode | Not started | 7 Makefiles use `silica-boot` |
| Runtime link | Bootstrap `.a` | Must move to self-emitted runtime |
| `data_structures/bst.silica` | In use | 5 emitter modules |
| `ListSymbolEntry` | In use | TC + SIR core; 4× duplicated lookups |
| `type TokenKind = int64` | In use | Only type alias in `src/` |
| Stdlib for compiler internals | Partial | nodeid map/set ready; heap not needed yet |
| Stray `src/btree_set_nodeid.silica` | Orphan | Remove in Step 0.1 |
| Bootstrap retirement | Not started | Gate: Phase 1.4 + Phase 6.1 |
