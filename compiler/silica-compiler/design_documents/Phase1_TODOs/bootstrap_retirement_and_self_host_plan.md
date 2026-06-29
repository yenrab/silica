# Bootstrap Retirement and Self-Host Migration Plan

**Purpose:** Identify every change needed so `silica-compiler` can build and maintain itself without `silica-bootstrap-compiler`, replacing bootstrap-era internal data structures and workarounds with the standard generated families specified in [data_structures_as_traits.md](data_structures_as_traits.md) and [data_structure_to_algorithms.md](data_structure_to_algorithms.md).

**Scope:** `compiler/silica-compiler/src/` (compiler internals), build system, and compiler-facing trials. Stdlib implementation of WBT / Brodal–Okasaki modules is tracked in [standard_data_structures_implementation_plan.md](standard_data_structures_implementation_plan.md); this plan covers **when and how the compiler adopts** those modules.

**Authority:**

- [data_structure_to_algorithms.md](data_structure_to_algorithms.md) — **locked algorithms** (Adams WBT for ordered collections; Brodal–Okasaki for heaps; WBT graphs + CSR freeze; no dense bitset)
- [data_structures_as_traits.md](data_structures_as_traits.md) — trait API specification, constructor function records, trait → module mapping (`wbt_set`, `wbt_map`, …)
- [standard_data_structures_implementation_plan.md](standard_data_structures_implementation_plan.md) — stdlib build progress and acceptance trials
- [list_implementation_design.md](../list_implementation_design.md) — when `List[T, S]` remains correct (AST chains, token streams)

**Current state (audit):**

| Layer | Bootstrap (`silica-boot`) | Self-hosted (`silica-compiler`) |
| ----- | ------------------------- | -------------------------------- |
| Compiler executable | `src/Makefile` builds `main.silica` → `main.ll` via bootstrap; links `libsilica_compiler.a` from Rust | Used by all `trials/*/`, already builds `standard_data_structures/` |
| Subdir Makefiles | `lexer/`, `parser/`, `type_checker/` (partial), `sir_generator/`, `emitter/`, `effect_checker/` | — |
| Internal ADTs | `data_structures/bst.silica` (naive unbalanced BST); ~20 custom `List*` cons-cell structs | Bracket-type witnesses for `OrderedSet`/`OrderedMap`/`Heap`/`DirectedGraph` in type checker (trait dispatch not fully wired to WBT backends) |
| Type aliases | `type TokenKind = int64` in `lexer_token_kind.silica` (only `type` alias in `src/`) | — |

This document intentionally contains no Silica source code. It is written as fine-grained work items that an LLM can follow when migrating compiler internals.

## Global Rules

1. **Do not remove bootstrap until a self-hosted stage produces a passing `make integrate` on the full compiler tree.**
2. **Lists stay lists** where order and immutability are the model (tokens, AST declaration lists, SIR function lists). **Maps/sets** replace linear-scan or naive-BST lookups keyed by `string` or ordered scalars.
3. **Use trait-oriented WBT APIs** once stdlib modules exist: `wbt_map@empty({ compare_key, compare_value })`, `OrderedMap@get`, etc.—not legacy `btree_*` / `graph_adj_*` / `heap_binary_*` bootstrap modules and not width-specialized exports like `empty[int64, mem(normal)]/0`.
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

**Phases 3–4 additionally require** `wbt_map` / `wbt_set` stdlib modules (Adams WBT [Ada93]) per the algorithm map. Until those modules pass acceptance trials, compiler map migration stays on `data_structures/bst.silica` or other interim structures—do not adopt legacy `btree_nodeid` as the long-term compiler backend.

## Compiler-internal collection targets

Per [data_structure_to_algorithms.md](data_structure_to_algorithms.md) and [data_structures_as_traits.md](data_structures_as_traits.md):

| Compiler need | Trait | Target module | Algorithm | Needed for bootstrap retirement? |
| ------------- | ----- | ------------- | --------- | -------------------------------- |
| String-keyed symbol / literal tables | `OrderedMap` | `wbt_map` | Adams WBT [Ada93] | **Yes** (Phases 3–4) |
| Ordered scalar pools (optional) | `OrderedMap` / `OrderedSet` | `wbt_map` / `wbt_set` | Adams WBT [Ada93] | **Yes** (emitter pools) |
| Priority worklists | `Heap` | `brodal_okasaki_min` | Brodal–Okasaki [BO96] | **No** (not used in compiler today) |
| Graph structures | `DirectedGraph` | `graph_wbt_*` | WBT + WBT neighbors | **No** (symbol tables are maps, not graphs) |

**Not in scope for compiler or stdlib:** dense bitset graphs, Patricia tries, region binary/d-ary heaps, NodeIDBTree / CsrBTree bootstrap families.

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
   - `src/btree_set_nodeid.silica` (orphan bootstrap copy; superseded by target `stdlib/data_structures/wbt_set.silica`)
   - `stdlib/data_structures/` duplicate or `.bak` files if present

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

**Target (not yet implemented):** Adams WBT modules `wbt_map` and `wbt_set` with trait adapters per [data_structures_as_traits.md](data_structures_as_traits.md). Progress tracked in [standard_data_structures_implementation_plan.md](standard_data_structures_implementation_plan.md).

**Actions:**

1. **OrderedMap / OrderedSet:** Block compiler adoption on **`wbt_map` / `wbt_set` acceptance trials** (insert, delete, get, generic `string` keys). Do **not** wire compiler internals to legacy `btree_nodeid`, `btree_set_nodeid`, or CSR B-tree modules—they are obsolete relative to the algorithm map.
2. **Heap / PriorityQueue:** Not required for compiler symbol tables. If adopted later, use **`brodal_okasaki_min`** only—not `heap_binary_*` or `heap_dary_*`.
3. **Graph:** Not required for compiler internals (maps suffice).
4. Add **`src/silica.config.compiler_internal`** listing only modules the compiler will `use` once WBT lands: `wbt_map`, `wbt_set`, `OrderedMap`, `OrderedSet`, `ordered_map_wbt_adapter`, `ordered_set_wbt_adapter`, plus shared compare helpers (e.g. `compare_string`).
5. Add compiler obligation checklist from traits doc §Compiler obligations: trait dispatch, function-record witnesses (E2017), `provided` block checking, `{ found: boolean, … }` get shapes.

**Exit criteria:**

- `wbt_map` / `wbt_set` pass stdlib acceptance trials for at least `string` and `int64` payload shapes.
- Minimal stdlib batch config compiles with smoke trial: `OrderedMap[string, int64, mem(normal)]` constructed via `wbt_map@empty({ compare_key, compare_value })` and queried via `OrderedMap@get`.

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

**Prerequisite:** Step 0.3 — `wbt_map` / `wbt_set` acceptance trials green.

**Current module:** Unbalanced BST with `BstNode { value, index, next_left, next_right }`, numeric/string compare via `string_parse@string_to_int64` hack for lexicographic order.

**Target:** `OrderedMap[string, int64, mem(normal)]` (and other key/value instantiations) backed by **`wbt_map`** — Adams weight-balanced tree with path copying [Ada93]. All lookups via **`OrderedMap@get`** / trait dispatch, not direct legacy module calls.

### Step 3.1 — Shared compile-time map helper module

**Actions:**

1. Add `compiler_maps.silica` under `src/` (or shared trial helper):
   - `compare_string(a, b) -> :less | :equal | :greater` (atom contract per traits doc)
   - `compare_int64(a, b) -> atom` for numeric-key pools
   - `empty_string_int64_map()` → `wbt_map@empty({ compare_key: compare_string, compare_value: compare_int64 })`
   - `map_insert_or_get_index(map, key, value) -> { map, index, inserted }` using `wbt_map@insert`
   - `map_index_of(map, key) -> int64` using `OrderedMap@get` ( `{ found: boolean, value: int64 }` shape)
2. No new type aliases; explicit `OrderedMap[Key, Value, mem(normal)]` at bindings.
3. Preserve functional persistence: every insert returns a new map value.

**Exit criteria:**

- Trial `compiler_string_index_map.silica` in `trials/` covers insert, lookup, duplicate key via WBT-backed map.

### Step 3.2 — Migrate emitter literal pools (5 modules)

| Module | Current | Migration |
| ------ | ------- | ----------- |
| `emitter/.../atom_table.silica` | `bst@bst_insert_string` + `ListAtomLexeme` | `OrderedMap[string, int64, mem(normal)]` via `wbt_map`; keep `ListAtomLexeme` for rodata order |
| `emitter/.../int64_literal_pool.silica` | `bst@bst_insert` (int64 keys) | `OrderedMap[int64, int64, mem(normal)]` with `compare_int64` |
| `emitter/.../float32_literal_pool.silica` | BST on stringified float | `wbt_map` keyed by `string` canonical form |
| `emitter/.../float64_literal_pool.silica` | same | same |
| `emitter/.../int_rodata.silica` | BST for int8–int64 print tables | Same WBT map pattern per width |

**Actions:**

1. Replace `use bst` with `use wbt_map`, `use OrderedMap`, `use compiler_maps` (and adapters if trait dispatch requires them in batch config).
2. Remove `BstNode` fields from table structs; store `OrderedMap[…]` in table record.
3. Update atom/literal pool trials under `trials/`.

**Exit criteria:**

- Emitter integrate trials pass; `data_structures/bst.silica` has zero `use` references; no `use btree_*` in compiler `src/`.

### Step 3.3 — Migrate `string_literal_pool.silica` (list scan → map)

**Current:** O(n) list dedup for UTF-8 correctness.

**Actions:**

1. Use `wbt_map`-backed `OrderedMap[string, int64, mem(normal)]` with **raw string ordering** via `compare_string` on full UTF-8 lexeme (not BST int64 hack).
2. Retain immutability contract and rodata emission order via auxiliary list (same as atom table).

**Exit criteria:**

- String literal / UTF-8 trials pass.

### Step 3.4 — Delete `data_structures/bst.silica`

**Exit criteria:**

- Module removed; `SEARCH_PATHS` / Makefiles drop `-I data_structures` if only BST lived there (`string_parse.silica`, `string_escapes.silica` remain).

## Phase 4 — Replace linear-scan association lists (compiler hot paths)

**Prerequisite:** Step 0.3 — `wbt_map` acceptance trials green (same gate as Phase 3).

**Principle:** Keep cons-cell lists for **ordered sequences**; replace **name → type** maps with **`wbt_map`**-backed `OrderedMap` and trait dispatch.

### Step 4.1 — Symbol table (`ListSymbolEntry`)

**Files:** `type_checker_core.silica` (+ all consumers: TC, SIR, module_checker paths).

**Current:** O(n) linked list; `add_symbol` prepends; shadowing by linear scan.

**Target shape:** `symbols: OrderedMap[string, SymbolEntry, mem(normal)]` with key = symbol name and value = inline `{ type_name, declared_effects, is_effect_alias, source_module }`.

**Actions:**

1. Add `SymbolTable` wrapper module using constructor function record + `wbt_map@insert` / `OrderedMap@get`.
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

1. Ensure compiler sources never `use` legacy bootstrap modules (`btree_*`, `graph_adj_*`, `heap_binary_*`, `heap_dary_*`) or width-specialized `empty[int64, mem(normal)]` exports.
2. Compiler-internal stdlib usage goes only through **`wbt_map` / `wbt_set`** and trait dispatch (`OrderedMap@…`, `OrderedSet@…`) per Phase 0.3 config.

**Exit criteria:**

- Grep for `btree_`, `graph_adj_`, `heap_binary_`, and width-specialized bootstrap exports under compiler `use` paths returns zero.

## Phase 6 — Validation, documentation, bootstrap removal

### Step 6.1 — Self-host integrate suite

**Actions:**

1. Add `trials/self_host_addition/` with Makefile running full `src/silica.config.compiler` through host compiler.
2. Add fixed-point script (documented): `host_n` compiles `host_{n+1}`; fail on byte mismatch or trial regression.

**Exit criteria:**

- CI/local `make integrate` includes self-host trial.

### Step 6.2 — Update design docs

**Actions:**

1. Cross-link this plan from `standard_data_structures_implementation_plan.md` (section: "Compiler internal consumption").
2. When compiler adopts WBT maps, add a note to `data_structures_as_traits.md` §Related documents or a short "Compiler adoption" subsection (implementation status remains driven by acceptance trials).
3. Mark bootstrap-analysis doc historical.

### Step 6.3 — Remove `silica-bootstrap-compiler` from repo workflow

**Actions:**

1. Delete bootstrap references from all Makefiles.
2. Archive or remove `silica-bootstrap-compiler` crate per project policy (outside this plan's file scope).

**Exit criteria:**

- No `silica-boot` in default build path.

## Suggested First PR (minimal vertical slice)

1. Step 0.1 inventory + delete stray `src/btree_set_nodeid.silica`
2. Step 1.1 `build-selfhost` Makefile target (still link bootstrap runtime)
3. Step 2.1–2.2 one ABI/string fix with trial (unblocks self-host before map migration)

**After `wbt_map` acceptance trials pass:**

4. Step 3.1 `compiler_maps.silica` + trial
5. Step 3.2 migrate **only** `atom_table.silica` off BST onto `wbt_map`

That sequences build-system flip and workaround fixes before WBT adoption inside the compiler.

## Risks

1. **Chicken-and-egg:** Last bootstrap build required to seed first self-host; document seed binary policy.
2. **WBT stdlib gate:** Phases 3–4 blocked until `wbt_map` / `wbt_set` exist and pass acceptance trials; do not permanently adopt legacy `btree_*` as a shortcut.
3. **Compile time / memory:** Full compiler batch may stress host compiler; may need staged `silica.config` shards before monolithic config.
4. **Scope creep:** Phase 4 parser constraint migration—default **keep lists** unless profiling shows need.
5. **Trait compiler gaps:** `provided` blocks, graph bracket witnesses, and boolean `found` shapes from traits doc may block clean `OrderedMap@get` adoption until compiler obligations are met.

## Completion Tracking

| Area | Status | Notes |
| ---- | ------ | ----- |
| Phase 0 audit | Not started | 27 bootstrap workaround comments catalogued in Step 0.2 |
| Build system dual-mode | Not started | 7 Makefiles use `silica-boot` |
| Runtime link | Bootstrap `.a` | Must move to self-emitted runtime |
| `data_structures/bst.silica` | In use | 5 emitter modules |
| `ListSymbolEntry` | In use | TC + SIR core; 4× duplicated lookups |
| `type TokenKind = int64` | In use | Only type alias in `src/` |
| `wbt_map` / `wbt_set` stdlib | Not implemented | Blocks Phases 3–4; see Step 0.3 |
| Compiler trait obligations | Not complete | See traits doc §Compiler obligations |
| Stray `src/btree_set_nodeid.silica` | Orphan | Remove in Step 0.1; do not adopt as compiler backend |
| Bootstrap retirement | Not started | Gate: Phase 1.4 + Phase 6.1 |
