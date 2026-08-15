# Bootstrap Retirement and Self-Host Migration Plan

**Purpose:** Identify every change needed so `silica-compiler` can build and maintain itself without `silica-bootstrap-compiler`, replacing bootstrap-era internal data structures and workarounds with the standard generated families specified in [data_structures_as_traits.md](data_structures_as_traits.md) and [data_structure_to_algorithms.md](data_structure_to_algorithms.md).

**Scope:** Parallel compiler tree `compiler/silica-compiler/src_selfhost/` (self-host edits), additive build targets, and compiler-facing trials. Production `compiler/silica-compiler/src/` and `silica-bootstrap-compiler` stay untouched until Phase 6 cutover. Stdlib implementation of WBT / Brodal–Okasaki / BinaryTree modules is tracked in [standard_data_structures_implementation_plan.md](standard_data_structures_implementation_plan.md); this plan covers **when and how the compiler adopts** those modules.

**Authority:**

- [data_structure_to_algorithms.md](data_structure_to_algorithms.md) — **locked algorithms** (Adams WBT for ordered collections; Brodal–Okasaki for heaps; WBT graphs + CSR freeze; no dense bitset)
- [data_structures_as_traits.md](data_structures_as_traits.md) — trait API specification, constructor function records, trait → module mapping (`wbt_set`, `wbt_map`, …)
- [standard_data_structures_implementation_plan.md](standard_data_structures_implementation_plan.md) — stdlib build progress and acceptance trials
- [data_structure_designs/persistent_binary_tree.md](data_structure_designs/persistent_binary_tree.md) and [data_structure_designs/binary_tree_trait.md](data_structure_designs/binary_tree_trait.md) — accepted representation/API contract required before compiler AST adoption
- [list_implementation_design.md](../list_implementation_design.md) — when `List[T, S]` remains correct (AST chains, token streams)

**Current state (audit):**

| Layer | Bootstrap (`silica-boot`) | Self-hosted (`silica-compiler`) |
| ----- | ------------------------- | -------------------------------- |
| Compiler executable | `src/Makefile` builds `main.silica` → `main.ll` via bootstrap; links `libsilica_compiler.a` from Rust | Used by all `trials/*/`, already builds `standard_data_structures/` |
| Subdir Makefiles | `lexer/`, `parser/`, `type_checker/` (partial), `sir_generator/`, `emitter/`, `effect_checker/` | — |
| Internal ADTs | `data_structures/bst.silica` (naive unbalanced BST); ~20 custom `List*` cons-cell structs | Bracket-type witnesses for `OrderedSet`/`OrderedMap`/`Heap`/`DirectedGraph` in type checker (trait dispatch not fully wired to WBT backends) |
| Parser expression AST | Recursive `Expr` struct with direct `inner` / `right_expr` and cyclic `kind=-1` dummy | Standard `BinaryTree` adoption deferred to Phase 7 |
| Type aliases | `type TokenKind = int64` in `lexer_token_kind.silica` — **must be removed before self-host flip**; updated `src/` may have zero aliases | — |

This document intentionally contains no Silica source code. It is written as fine-grained work items that an LLM can follow when migrating compiler internals.

## Global Rules

1. **Do not remove bootstrap until a self-hosted stage produces a passing `make integrate` on the full compiler tree.**
2. **Lists stay lists** where order and immutability are the model (tokens, AST declaration lists, SIR function lists). **Maps/sets** replace linear-scan or naive-BST lookups keyed by `string` or ordered scalars.
3. **Use trait-oriented WBT APIs** once stdlib modules exist: `wbt_map@empty({ compare_key, compare_value })`, `OrderedMap@get`, etc.—not legacy `btree_*` / `graph_adj_*` / `heap_binary_*` bootstrap modules and not width-specialized exports like `empty[int64, mem(normal)]/0`.
4. **No new named constructor-record struct types** (per traits doc Constructor Function Record Rule).
5. **Each migration step** adds or extends a trial under `trials/` before deleting the old path.
6. **Retire duplicated logic** only after self-hosted cross-module ABI is verified (see Phase 2).
7. **BinaryTree acceptance and compiler adoption are separate gates.** Phase 7 may consume only an already accepted `tree_binary`; its completion cannot be used to excuse a missing standard-structure trial. §12 may use an index-arena encoding for `Expr`/`SIRTerm` so self-host does not wait on `tree_binary`.
8. **Safe dual-path freeze until cutover.** Through Phases 0–5 and until Phase 6 cutover: leave **`silica-bootstrap-compiler` untouched** and leave production **`compiler/silica-compiler/src/` untouched** for self-host migration. All alias/BST/WBT/ABI/dialect self-host edits happen only under **`compiler/silica-compiler/src_selfhost/`**. Default build remains bootstrap → frozen `src/`.
9. **No `type` aliases and no named `struct` declarations in the parallel compiler source.** After Phase 5.1 / §12 dialect waves: `rg '^\s*type\s+\w+\s*='` and `rg '^\s*struct\s+\w+'` under `compiler/silica-compiler/src_selfhost` must stay at zero. Types follow [silica-specification.md](../silica-specification.md) §3.4.2 (inline records, `List[T]`, seed-legal tree encodings). Do not introduce temporary aliases or named wrapper structs.
10. **Self-host compile requires self-hostable parallel source first.** Do not claim a successful `build-selfhost` while `bst`, any `type` alias, or any named `struct` declaration remains in `src_selfhost/`. Host is the seed `silica-compiler` from frozen `src/` with **E1047 on** — do not disable E1047 to pass the gate.

## Implementation Order

Critical path for full self-hosting without the bootstrap compiler (matches [standard_data_structures_implementation_plan.md](standard_data_structures_implementation_plan.md) §§11–13):

1. **Phase 0** — Audit, workaround inventory, stdlib smoke; stand up `src_selfhost/` as a copy of frozen `src/`.
2. **Phase 5.1** — Remove every `type` alias from **`src_selfhost/`** only (Global Rule 9).
3. **Phase 3** — In **`src_selfhost/`**, replace emitter `bst` with WBT maps; drop `bst` from the parallel graph only (keep `src/data_structures/bst.silica`).
4. **Phase 2 (class-A) + §12 dialect rewrite** — Rewrite **`src_selfhost/`** to seed-legal Silica (no named structs / no aliases; List + inline records + Wave C arena for trees) and fix remaining W-ids so the seed compiles the full parallel graph. Prefer edits in the parallel tree; shared host bugfixes only when they do not alter frozen `src/` contracts. **Do not** expand the seed to re-accept boot-era named structs.
5. **Phase 1** — Add dual-build targets: default stays bootstrap → `src/`; `build-selfhost` / `assembly-selfhost` build only from `src_selfhost/` using the seed host.
6. **Phase 2 (remainder) + Phase 5.2** — Finish workaround cleanup and legacy `use` greps inside `src_selfhost/`.
7. **Phase 4** — In `src_selfhost/`, replace linear-scan association lists with WBT maps (still no aliases / no named structs).
8. **Phase 6** — Self-host integrate + fixed-point on the parallel tree; **then** cut over (promote `src_selfhost/` → production `src/`) and retire bootstrap from the default path.
9. **Phase 7** — Optionally replace the §12 `Expr`/`SIRTerm` index-arena with standard `BinaryTree` (downstream modernization; not required for initial bootstrap retirement once the arena path is green).

Phase 4 may start once Phase 1 stage **`build-selfhost`** exists; it must not block the first self-host binary if emitter BST, aliases, and named structs are already gone from `src_selfhost/`. Phase 2 class-A + dialect rewrite that block compile precede a green Phase 1 exit; deleting duplicated lookups (Step 2.2) waits until after Phase 1 ABI is verified and stays inside the parallel tree until cutover.

Phase 7 begins only after standard `BinaryTree` acceptance and a stable self-host compiler. It is not required to accept BinaryTree and is not required to retire bootstrap unless separately promoted to a release gate.

**Phases 3–4 require** accepted `wbt_map` / `wbt_set` (Adams WBT [Ada93]) per the algorithm map — now available via standard-plan §§8A–§10. Do not adopt legacy `btree_nodeid` as the long-term compiler backend.

## Compiler-internal collection targets

Per [data_structure_to_algorithms.md](data_structure_to_algorithms.md) and [data_structures_as_traits.md](data_structures_as_traits.md):

| Compiler need | Trait | Target module | Algorithm | Needed for bootstrap retirement? |
| ------------- | ----- | ------------- | --------- | -------------------------------- |
| String-keyed symbol / literal tables | `OrderedMap` | `wbt_map` | Adams WBT [Ada93] | **Yes** (Phases 3–4) |
| Ordered scalar pools (optional) | `OrderedMap` / `OrderedSet` | `wbt_map` / `wbt_set` | Adams WBT [Ada93] | **Yes** (emitter pools) |
| Priority worklists | `Heap` | `brodal_okasaki_min` | Brodal–Okasaki [BO96] | **No** (not used in compiler today) |
| Graph structures | `DirectedGraph` | `graph_wbt_*` | WBT + WBT neighbors | **No** (symbol tables are maps, not graphs) |
| Parser expression AST | `BinaryTree` | `tree_binary` | Fixed left/right recursive tree + zipper | **No** for initial retirement; **Yes** for Phase 7 modernization |

**Not in scope for compiler or stdlib:** dense bitset graphs, Patricia tries, region binary/d-ary heaps, NodeIDBTree / CsrBTree bootstrap families.

## Phase 0 — Prerequisites (blocking self-hostable source and the build flip)

These are compiler/runtime fixes or stdlib gaps that block compiling the full `src/` tree with `silica-compiler` instead of `silica-boot`. Alias removal (Phase 5.1) and BST → WBT (Phase 3) are also on that critical path; this phase inventories them.

### Step 0.1 — Inventory bootstrap-only build assumptions + stand up `src_selfhost/`

**Actions:**

1. Document that `src/Makefile` uses **two build models**:
   - Per-file `.silica` → `.ll` via `silica-boot` (compiler executable)
   - `silica.config` batch → `.sams` via `silica-compiler` (trials, `standard_data_structures/`)
2. List all Makefiles still pointing at bootstrap (7 found):
   - `src/Makefile`, `src/lexer/Makefile`, `src/effect_checker/Makefile`, `src/sir_generator/Makefile`, `src/emitter/Makefile` (+ parent rules for `parser/`, `type_checker/`, `module_checker/`, `ffi/`, `trait_checker/` via `src/Makefile`)
3. Identify bootstrap runtime dependency: `libsilica_compiler.a` linked into `silica-compiler` executable (`src/Makefile` lines 13–14, 143–148).
4. Note stray duplicates not in the build graph for later cleanup (do **not** delete from frozen `src/` in this step unless already proven unused by the default path):
   - `src/btree_set_nodeid.silica` (orphan bootstrap copy; superseded by target `stdlib/data_structures/wbt_set.silica`)
   - `stdlib/data_structures/` duplicate or `.bak` files if present
5. Create `compiler/silica-compiler/src_selfhost/` as a copy of frozen `src/`; add a short README: self-host migration edits only this tree; bootstrap and `src/` stay default until Phase 6 cutover.

**Exit criteria:**

- Written inventory checked into this plan's Completion Tracking table; `src_selfhost/` present as the sole self-host edit target.

### Step 0.2 — Self-host compile feasibility matrix (read-only audit)

**Actions:** For each bootstrap workaround comment in `src/`, classify as:

- **A — Compiler bug** (must fix in `silica-compiler` before self-host)
- **B — Source workaround** (can delete once self-host compiles correctly)
- **C — Intentional** (unrelated to bootstrap, e.g. FailureReporter "bootstrap" in supervisor sense)

**Known bootstrap workaround sites (27 comments across 20 files):**

| ID | File | Issue | Class |
| -- | ---- | ----- | ----- |
| W01 | `build_output.silica` | `concat` drops second arg in inline `case` | A/B |
| W02 | `effect_checker_core.silica`, `effect_serializer.silica` | `length_bytes("")` returns non-zero | A |
| W03 | `lexer_runner.silica` | `skip_whitespace` sometimes no-ops | A |
| W04 | `atom_rodata.silica`, `int_rodata.silica` | `.asciz` / escaped-quote bugs | B (emitter style) |
| W05 | `type_checker_expressions_string_calls.silica` | string pattern-match issues | A/B |
| W06 | `type_checker_tuple_decompose_helpers.silica` | tuple binding decomposition bug; cross-module string return segfault | A |
| W07 | `sir_generator/terms/terms.silica` | cross-`.ll` string return segfault → duplicated lookup fns | A |
| W08 | `type_checker_core.silica` | sret/stack on deep module TC; `(bool,string)` tuple return ABI | A |
| W09 | `sir_generator/declarations/qualified_call_mangler.silica`, `trait_specialization.silica` | structural-vs-Named inference for `List`-typed fields | A/B |
| W10 | `parser_tuples.silica` | token.kind false-matches grouping kinds | A/B |
| W11 | `type_checker_expressions.silica` | `call_name_is_module_qualified` misclassification; `tc_` prefix collision | A/B |
| W12 | `parser_ast.silica` | nominal rebuild helpers for structural records; legacy recursive Expr representation | B / Phase 7 |
| W13 | `parser/constraint_extract.silica` | tuple order for bootstrap codegen; stack overflow guard on case branches | A/B |
| W14 | `sir_generator/terms/identifiers.silica` | nested tuple destructuring codegen | A/B |
| W15 | `emitter/.../term_emitter.silica` | 6-param `emit_const` issue | B |
| W16 | `type_checker_core.silica` | `lookup_symbol_found` exists because `""` string compare unreliable | A |

**Exit criteria:**

- Every W-id has an owner phase (2, 3, or 7) and a trial or integrate check.

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

**Prerequisite:** Phase 0 inventory + `src_selfhost/` tree; Phase 5.1 (zero `type` aliases in `src_selfhost/`); Phase 3 (zero `bst` / WBT emitter pools in `src_selfhost/`); Phase 2 class-A items **and** the §12 dialect rewrite (zero named `struct` declarations; List/inline/arena shapes) so the **seed with E1047 on** can compile that parallel tree. Claiming `build-selfhost` success before those source gates is invalid.

**Safe-transition note:** Phase 1 **adds** self-host targets; it does **not** change the default bootstrap → `src/` path and does **not** modify `silica-bootstrap-compiler`.

### Step 1.1 — Introduce dual-build Makefile switch

**Actions:**

1. Add `BOOTSTRAP_COMPILER` and `HOST_COMPILER` variables (root or `src/` Makefile) without changing default behavior.
2. Add target `build-selfhost`: requires existing `silica-compiler` (seed from last bootstrap build of frozen `src/`), compiles **`src_selfhost/`** with the self-hosted compiler.
3. Keep `build-bootstrap` (frozen `src/` via `silica-boot`) as default until Phase 6 cutover.
4. Prefer consolidating the self-host path on batch config (Step 1.2); leave existing per-file `.ll` bootstrap recipes for `src/` alone.

**Exit criteria:**

- `make build-selfhost` produces a binary from `src_selfhost/` without invoking `silica-boot` on that path (runtime link may still use bootstrap `.a` temporarily).
- Default `make` still builds via bootstrap → frozen `src/`.

### Step 1.2 — Unify self-host build on `silica.config` batch mode

**Rationale:** `main.silica` already implements the batch pipeline (`silica.config`, dependency sort, `.sams` emission). Trials use this successfully; subdir `.ll` per-module builds are redundant for the linked executable (`src/Makefile` comment: "main.o contains everything").

**Actions:**

1. Create `src_selfhost/silica.config.compiler` listing all parallel-tree `.silica` units in dependency order (lexer → parser → … → `main.silica`).
2. Add `make assembly-selfhost`: run `silica-compiler` against `src_selfhost/` with that config → `.sams` → `.o` → link.
3. Prefer self-emitted `__silica_runtime.sams` on the self-host link path (align with trial Makefiles); bootstrap `.a` may remain temporarily.
4. Do not retire bootstrap subdir `.o` production for frozen `src/` in this phase.

**Exit criteria:**

- Self-host binary from `src_selfhost/` built through the `.sams` pipeline; no `main.ll` on the **self-host** critical path.

### Step 1.3 — Fix stack / resource limits for self-host input size

**Actions:**

1. Preserve/enlarge main-thread stack flags (`-Wl,-stack_size,0x10000000` in `src/Makefile`) in the new link recipe.
2. Add integrate trial compiling the full `src/silica.config.compiler` graph (compile-only or run `--version` smoke if available).

**Exit criteria:**

- Full compiler source batch compiles without stack overflow (constraint_extract case-depth guard W13 may become unnecessary—track in Phase 2).

### Step 1.4 — Bootstrap retirement gate (deferred to Phase 6)

**Actions (document only in Phase 1; execute in Phase 6 after fixed-point):**

1. Document **fixed-point procedure:** bootstrap-built seed → host₁ (from `src_selfhost/`) → host₂; compare artifacts or run full `make integrate`.
2. Do **not** remove `silica-bootstrap-compiler` or change the default Makefile here.
3. Note the cutover checklist for Phase 6 (promote parallel tree, then retire bootstrap from default path).

**Exit criteria (Phase 1):**

- Fixed-point and cutover procedure written; bootstrap and frozen `src/` still the default.

## Phase 2 — Remove bootstrap workarounds in compiler source

Work items map to W-ids from Step 0.2. Prefer fixing and simplifying **inside `src_selfhost/`**. Shared host/runtime bugfixes are allowed only when they do not change frozen `src/` contracts. Do not strip workarounds from frozen `src/` until Phase 6 cutover.

### Step 2.1 — String and empty-string reliability (W02, W16)

**Files:** `effect_checker_core.silica`, `effect_serializer.silica`, `type_checker_core.silica` (`lookup_symbol_found`).

**Actions:**

1. Add error-enforcement or unit trials for `length_bytes("") == 0` and reliable `""` equality.
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

## Phase 3 — Replace `bst` in the parallel tree (`src_selfhost/`)

**Prerequisite:** Step 0.3 — `wbt_map` / `wbt_set` acceptance trials green; Phase 5.1 complete (no `type` aliases in `src_selfhost/`). **This phase is on the critical path before Phase 1** — self-host compile of the parallel tree must not depend on `bst`.

**Freeze:** Do not edit emitter modules or delete `bst` under frozen `src/`. Production continues to use `src/data_structures/bst.silica`.

**Current module (frozen `src/`):** Unbalanced BST with `BstNode { value, index, next_left, next_right }`, numeric/string compare via `string_parse@string_to_int64` hack for lexicographic order.

**Target (in `src_selfhost/`):** `OrderedMap[string, int64, mem(normal)]` (and other key/value instantiations) backed by **`wbt_map`** — Adams weight-balanced tree with path copying [Ada93]. All lookups via **`OrderedMap@get`** / trait dispatch, not direct legacy module calls.

### Step 3.1 — Shared compile-time map helper module

**Actions:**

1. Add `compiler_maps.silica` under `src_selfhost/` (or a self-host-only helper path):
   - `compare_string(a, b) -> :less | :equal | :greater` (atom contract per traits doc)
   - `compare_int64(a, b) -> atom` for numeric-key pools
   - `empty_string_int64_map()` → `wbt_map@empty({ compare_key: compare_string, compare_value: compare_int64 })`
   - `map_insert_or_get_index(map, key, value) -> { map, index, inserted }` using `wbt_map@insert`
   - `map_index_of(map, key) -> int64` using `OrderedMap@get` ( `{ found: boolean, value: int64 }` shape)
2. No type aliases of any kind; explicit `OrderedMap[Key, Value, mem(normal)]` at bindings.
3. Preserve functional persistence: every insert returns a new map value.

**Exit criteria:**

- Trial `compiler_string_index_map.silica` in `trials/` (self-host leaf) covers insert, lookup, duplicate key via WBT-backed map.

### Step 3.2 — Migrate emitter literal pools in `src_selfhost/` (5 modules)

| Module (under `src_selfhost/`) | Frozen `src/` current | Migration in parallel tree |
| ------ | ------- | ----------- |
| `emitter/.../atom_table.silica` | `bst@bst_insert_string` + `ListAtomLexeme` | `OrderedMap[string, int64, mem(normal)]` via `wbt_map`; keep `ListAtomLexeme` for rodata order |
| `emitter/.../int64_literal_pool.silica` | `bst@bst_insert` (int64 keys) | `OrderedMap[int64, int64, mem(normal)]` with `compare_int64` |
| `emitter/.../float32_literal_pool.silica` | BST on stringified float | `wbt_map` keyed by `string` canonical form |
| `emitter/.../float64_literal_pool.silica` | same | same |
| `emitter/.../int_rodata.silica` | BST for int8–int64 print tables | Same WBT map pattern per width |

**Actions:**

1. In `src_selfhost/` only: replace `use bst` with `use wbt_map`, `use OrderedMap`, `use compiler_maps`.
2. Remove `BstNode` fields from table structs in the parallel tree; store `OrderedMap[…]` in table record.
3. Add or extend self-host / emitter trials that compile against `src_selfhost/` (do not break frozen `src/` goldens).

**Exit criteria:**

- Parallel emitter path green; zero `use bst` under `src_selfhost/`; frozen `src/` still uses `bst` unchanged.

### Step 3.3 — Migrate `string_literal_pool.silica` in `src_selfhost/` (list scan → map)

**Current (frozen):** O(n) list dedup for UTF-8 correctness.

**Actions:**

1. In `src_selfhost/`: use `wbt_map`-backed `OrderedMap[string, int64, mem(normal)]` with **raw string ordering** via `compare_string` on full UTF-8 lexeme (not BST int64 hack).
2. Retain immutability contract and rodata emission order via auxiliary list (same as atom table).

**Exit criteria:**

- Parallel string literal / UTF-8 path green; frozen `src/` unchanged.

### Step 3.4 — Drop `bst` from the parallel graph only

**Exit criteria:**

- `src_selfhost/` search paths no longer need `bst`; `src/data_structures/bst.silica` **retained** until Phase 6 cutover.

## Phase 4 — Replace linear-scan association lists (parallel tree hot paths)

**Prerequisite:** Step 0.3 — `wbt_map` acceptance trials green (same gate as Phase 3). Edits only under `src_selfhost/` until Phase 6 cutover.

**Principle:** Keep cons-cell lists for **ordered sequences**; replace **name → type** maps with **`wbt_map`**-backed `OrderedMap` and trait dispatch.

### Step 4.1 — Symbol table (`ListSymbolEntry`)

**Files (under `src_selfhost/`):** `type_checker_core.silica` (+ all consumers: TC, SIR, module_checker paths).

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

## Phase 5 — Type alias ban and API cleanup (parallel tree)

**Schedule:** Step 5.1 runs **before Phase 3 and Phase 1** on `src_selfhost/` (standard-plan §11). Step 5.2 may complete after the first self-host binary. Frozen `src/` keeps its aliases until Phase 6 cutover.

### Step 5.1 — Remove every `type` alias from `src_selfhost/` (including `TokenKind`)

**File (parallel):** `src_selfhost/lexer/lexer_token_kind.silica` (known in frozen tree: `type TokenKind = int64`); re-grep all of `src_selfhost/` before exit.

**Actions:**

1. In `src_selfhost/` only: replace `TokenKind` with bare `int64` in signatures; `token_kinds()` returns an **inline** constant record (not a named `TokenKinds` struct type — that removal is part of the §12 dialect waves).
2. Remove any other `type` alias under `src_selfhost/` — none may remain there.
3. Do not add replacement aliases or replacement named structs; do not edit frozen `src/` for this step (Global Rules 8–9).

**Exit criteria:**

- `rg '^\s*type\s+\w+\s*=' compiler/silica-compiler/src_selfhost` returns zero matches; frozen `src/` alias state unchanged; production lexer trials still pass via bootstrap → `src/`.
- Named-struct removal is tracked under standard-plan §12 waves A–C (Global Rule 9), not solely this step.

### Step 5.2 — Stop depending on bootstrap stdlib exports in compiler `use` graph

**Actions:**

1. Ensure compiler sources never `use` legacy bootstrap modules (`btree_*`, `graph_adj_*`, `heap_binary_*`, `heap_dary_*`) or width-specialized `empty[int64, mem(normal)]` exports.
2. Compiler-internal stdlib usage goes only through **`wbt_map` / `wbt_set`** and trait dispatch (`OrderedMap@…`, `OrderedSet@…`) per Phase 0.3 config.

**Exit criteria:**

- Grep for `btree_`, `graph_adj_`, `heap_binary_`, and width-specialized bootstrap exports under compiler `use` paths returns zero.

## Phase 6 — Validation, cutover, bootstrap removal

**Until Steps 6.1–6.2 pass, the §11 freeze still holds:** bootstrap and frozen `src/` remain the production default.

### Step 6.1 — Self-host integrate suite

**Actions:**

1. Add `trials/self_host_addition/` with Makefile running full `src_selfhost/silica.config.compiler` through host compiler.
2. Add fixed-point script (documented): `host_n` (built from `src_selfhost/`) compiles `host_{n+1}` from the same tree; fail on byte mismatch or trial regression.

**Exit criteria:**

- CI/local `make integrate` includes the self-host trial **in addition to** the existing bootstrap → `src/` path (do not drop the frozen path yet).

### Step 6.2 — Update design docs

**Actions:**

1. Cross-link this plan from `standard_data_structures_implementation_plan.md` (section: "Compiler internal consumption").
2. When the parallel tree adopts WBT maps, add a note to `data_structures_as_traits.md` §Related documents or a short "Compiler adoption" subsection (implementation status remains driven by acceptance trials).
3. Prepare (but do not yet finalize) bootstrap-analysis “historical / retired” wording for Step 6.4.

### Step 6.3 — Cutover: promote `src_selfhost/` → production `src/`

**Actions:**

1. Only after Step 6.1 fixed-point is green: replace production `src/` with the parallel tree contents (or atomic directory swap), preserving history per project policy.
2. Point default Makefile targets at the promoted tree / self-host build.
3. Remove `bst` from the promoted production tree if it was only retained for the frozen path.
4. Confirm production `src/` now has zero `type` aliases.

**Exit criteria:**

- Default build no longer depends on the pre-cutover frozen tree; alias/`bst` constraints hold on production `src/`.

### Step 6.4 — Remove `silica-bootstrap-compiler` from repo workflow

**Actions:**

1. Delete bootstrap references from default Makefiles (archive dual-path notes if useful).
2. Archive or remove `silica-bootstrap-compiler` crate per project policy (outside this plan's file scope).
3. Mark bootstrap-analysis doc historical / retired.

**Exit criteria:**

- No `silica-boot` in default build path.

## Phase 7 — Compiler-wide parser AST migration to BinaryTree

**Nature:** downstream compiler adoption after standard-structure acceptance.
**Standard-library prerequisite:** implementation-plan §9D `tree_binary` / `BinaryTree` exit gate.
**Non-gating rule:** this phase is not a prerequisite for accepting standard BinaryTree and is not part of the BinaryTree requirements-to-trials ledger.

This phase migrates the parser `Expr` representation and every compiler phase that consumes or rewrites it. It does not silently include `SIRTerm`; the SIR tree remains a separate representation and requires a separate design decision if migration is later desired.

### Phase 7 entry gate

- `BinaryTree[ItemType, mem(SpaceType)]`, `tree_binary`, and inline zipper operations pass their complete standard-data-structure acceptance suite.
- Phase 1 self-host staging has passed; the migration is tested with a self-host compiler, not only the bootstrap compiler.
- The current `Expr.kind` contract and the child role of every kind are recorded from `parser_ast.silica`.
- The old compiler remains available as an equivalence oracle until Phase 7 exits.
- No change to BinaryTree's standard API is justified solely by an AST convenience without first updating its normative detailed design.

### Step 7.1 — Freeze the AST-to-BinaryTree schema

Record one table for every parser `Expr.kind`:

- payload fields used (`kind`, `value`, `name`, source location, tuple-decomposition bindings, sequence effects);
- whether `inner` is absent or occupied;
- whether `right_expr` is absent or occupied;
- whether either child is a semantic operand, body, continuation, branch-list spine, tuple/list/record spine, error recovery subtree, or opaque holder;
- whether child visitation changes lexical scope or expected type; and
- whether the kind may legally occur only during parsing/lowering.

The BinaryTree item is one exact inline payload record/tuple. No `AstNode`, `AstPayload`, `AstPath`, `AstCursor`, or `AstZipper` alias/struct/enum is introduced. Existing payload components may be carried while their independent bootstrap cleanup remains pending, but the tree surface itself repeats its complete structural type.

Mapping rules:

- old `kind = -1` cyclic dummy is represented by `:none`, never by a BinaryTree node;
- old `inner` maps only to the fixed left role;
- old `right_expr` maps only to the fixed right role;
- right-spined call, tuple, list, record, and case-branch encodings initially retain their exact shape;
- no migration step flattens or reorders those spines;
- `Program`, declaration lists, parameter lists, effect lists, and named-program lists remain lists/records unless separately authorized.

**Exit criteria:**

- Every current kind, including parse-error markers and lambda/function-type holders, has one unambiguous payload/child schema.
- The schema identifies all scope-sensitive traversal edges.

### Step 7.2 — Add a bidirectional compatibility bridge

Add temporary compiler-internal functions:

- legacy `Expr` → `BinaryTree[<complete inline Expr payload record>, mem(normal)]`;
- BinaryTree → legacy `Expr`;
- legacy dummy ↔ absent child conversion;
- structural equality/reporting used only by migration trials.

Extend `src/silica.config.compiler_internal` for this phase with `tree_binary`, `BinaryTree`, and their accepted dependencies. Do not add them to the compiler build graph before the Phase 7 entry gate.

The bridge:

- constructs through `tree_binary`, never by forging private node fields;
- preserves exact source locations and side lists;
- preserves child roles and right-spine order;
- checks count overflow and canonical arena behavior;
- handles every parser error marker; and
- never becomes a permanent public stdlib adapter.

Trials under `trials/self_host_addition/`:

- `ast_binary_tree_roundtrip_all_kinds`;
- `ast_binary_tree_roundtrip_nested_calls`;
- `ast_binary_tree_roundtrip_case_branches`;
- `ast_binary_tree_roundtrip_sequence_and_let`;
- `ast_binary_tree_roundtrip_parse_errors`;
- `ast_binary_tree_no_cyclic_dummy`.

**Exit criteria:**

- Parsing representative valid and invalid sources, converting old → new → old, yields structurally identical legacy ASTs.
- New → old → new yields identical BinaryTree payload/fold sequences and validates.

### Step 7.3 — Introduce the compiler AST access/rewrite façade

Before migrating consumers, add compiler-internal helpers over `BinaryTree`:

- payload/kind/value/name/location access;
- optional left/right child access;
- kind-checked child-role accessors where scope or semantics differ;
- preorder/postorder traversal;
- path replacement;
- zipper open/down/up/close;
- shallow rebuild from a payload and two optional child subtrees; and
- AST-specific validation of kind/arity rules layered on `tree_binary@validate`.

All façade signatures use `BinaryTree[...]` and inline result/path/zipper structures. No new named tree wrapper is introduced.

AST validation checks properties outside generic BinaryTree:

- allowed child occupancy for each kind;
- required continuation/branch-list spine kinds;
- opaque holder restrictions;
- parse-error marker shape;
- no unexpected dummy payload;
- side-list presence where required.

**Exit criteria:**

- A consumer can inspect, traverse, and rebuild every current expression shape without direct access to BinaryTree private fields.
- AST-specific validation rejects one malformed fixture for every kind-family rule.

### Step 7.4 — Migrate parser producers

Migrate `parser/constraint_extract.silica` and parser helpers:

1. leaf expressions use `with_root`;
2. unary expressions construct exactly the left child;
3. binary/continuation expressions construct fixed left and right children;
4. old `parser_ast@dummy_expr()` arguments become empty subtrees/absent children;
5. right-spined argument, tuple, list, record, and branch builders preserve their current order;
6. lambda lifting performed inside the parser uses the façade/zipper rather than raw node construction; and
7. parse-error recovery preserves diagnostic locations and recovered subtrees.

Keep the compatibility bridge so parser output can still feed unmigrated consumers during this step.

**Exit criteria:**

- Parser output is natively BinaryTree.
- Parser golden diagnostics and AST shape trials are unchanged through the bridge.
- No new legacy `Expr { ... }` construction remains in parser production code.

### Step 7.5 — Migrate read-only compiler consumers

Migrate consumers in bounded groups, running their local integrate suites after each group:

1. module checker and `main.silica` dependency/debug walks;
2. FFI placement, taint, and ABI checkers;
3. effect checker;
4. type checker and recursive/type-specific helpers; and
5. read-only SIR-generation queries.

Replace:

- `.kind`, `.value`, `.name`, and `.location` access with payload façade calls;
- `.inner` / `.right_expr` recursion with fixed-role child or zipper traversal;
- `kind < 0` dummy checks with explicit child absence;
- manual branch-list and argument-spine walks with kind-checked façade iterators.

**Exit criteria:**

- Each migrated group consumes native BinaryTree without round-tripping to legacy `Expr`.
- Type errors, effect errors, FFI diagnostics, module resolution, and source locations remain unchanged.

### Step 7.6 — Migrate rewriting and lowering passes

Migrate all passes that currently rebuild `Expr` values:

- lambda lifting and higher-order-function wrapping;
- compile-only field cleanup;
- tuple-decomposition and sequence rewriting;
- argument replacement helpers;
- actor/supervisor expression rewrites;
- collection-constructor preprocessing; and
- AST-to-SIR lowering.

Use:

- path copying for targeted child replacement;
- zipper reconstruction for focus-oriented rewrites;
- postorder mapping for whole-tree payload cleanup;
- explicit scope-sensitive recursion where callbacks require environments.

Do not use repeated root-relative path lookup inside a full traversal when a zipper/cursor provides linear traversal. Do not mutate or inspect `tree_binary` private fields.

Equivalence trials compare:

- diagnostics;
- emitted SIR text;
- emitted assembly for a bounded representative corpus;
- lifted declaration order and names;
- closure capture lists;
- source-location-derived labels; and
- failure behavior on malformed source.

**Exit criteria:**

- No compiler pass converts BinaryTree back to legacy `Expr` for ordinary operation.
- Targeted rewrites allocate only the changed path; full rewrites remain linear in logical AST nodes.

### Step 7.7 — Remove the legacy recursive Expr representation

After every producer and consumer is native:

1. remove recursive `inner: Expr` and `right_expr: Expr` storage;
2. remove `dummy_expr()` and all `kind == -1` / `kind < 0` absence checks that referred to it;
3. remove nominal AST rebuild helpers made obsolete by BinaryTree operations;
4. remove the temporary bidirectional bridge;
5. remove bridge-only trials while retaining native equivalence regressions; and
6. verify there is no named AST node/path/cursor/zipper type introduced as a replacement.

Declaration, parameter, effect, and program-list cleanup is not implied unless those structures independently violate the active self-host rules.

**Exit criteria:**

- `rg` finds no legacy recursive Expr construction or cyclic dummy use.
- Every compiler phase accepts the same native BinaryTree AST value.

### Step 7.8 — Performance, persistence, and fixed-point gate

Add operation-counter and stress trials:

- deeply nested unary and binary expressions;
- long call/tuple/list/record/case right spines;
- large sequence/let continuations;
- targeted deep argument replacement;
- whole-tree cleanup/lambda-lift passes;
- retained old AST roots across rewrites;
- legal shared subtrees and cycle rejection.

Gate on:

- no repeated root traversal causing accidental `O(nh)` full passes;
- `O(h)` targeted path/zipper reconstruction;
- `O(n)` whole-tree folds/maps;
- unchanged compiler diagnostics and emitted output;
- full self-host `make integrate`; and
- hostₙ → hostₙ₊₁ fixed-point equivalence under the migrated AST.

### Phase 7 exit gate

- Standard BinaryTree remains unchanged unless its own design/acceptance process approved a required revision.
- Parser `Expr` production, all checkers, cleanup/rewriters, and AST-to-SIR lowering use native BinaryTree.
- The cyclic dummy representation and all legacy bridge code are gone.
- AST kind/arity validation, persistence, allocation, and complexity suites pass.
- SIRTerm remains explicitly outside this migration unless a separate accepted plan adds it.
- Compiler-wide self-host and fixed-point gates pass.

## Suggested First PR (minimal vertical slice)

1. Step 0.1 inventory (read-only against frozen `src/` / bootstrap); optionally note orphan `src/btree_set_nodeid.silica` for later cleanup — **do not delete from frozen `src/` in this PR unless already unused by the default build**
2. Copy `src/` → `src_selfhost/` (or scripted sync) with README stating: edit only `src_selfhost/` for self-host work
3. Step 5.1 in `src_selfhost/` only: remove `type TokenKind = int64` (and any other aliases)
4. Step 3.1 `src_selfhost/compiler_maps.silica` + self-host trial (explicit `OrderedMap[…]`; no aliases)
5. Step 3.2 migrate **only** `src_selfhost/.../atom_table.silica` off BST onto `wbt_map`

**Then continue the critical path (still without touching bootstrap or frozen `src/`):**

6. Finish Phase 3 in `src_selfhost/` (remaining emitter pools); drop `bst` from the parallel graph only
7. Phase 2 class-A fixes needed to compile `src_selfhost/` with `silica-compiler`
8. Step 1.1+ `build-selfhost` / `assembly-selfhost` targets (default remains bootstrap → `src/`)
9. Phase 4 / Phase 6 fixed-point, then cutover, then bootstrap retirement

That sequences **freeze production → parallel alias ban → parallel WBT emitter adoption → compileability → additive build flip → fixed-point → cutover**.

## Risks

1. **Chicken-and-egg:** Last bootstrap build of frozen `src/` seeds first self-host; document seed binary policy. Bootstrap stays available until Phase 6.4.
2. **Source-before-flip:** Self-host compile fails if aliases, `bst`, or named `struct` declarations remain in `src_selfhost/`; do not reorder a green Phase 1 exit ahead of those gates. Disabling E1047 in a staging seed is not a valid substitute for the dialect rewrite.
3. **Drift between trees:** `src/` and `src_selfhost/` can diverge; document refresh/cherry-pick rules and prefer landing unrelated bugfixes in both trees only when required for production.
4. **WBT stdlib gate:** Phases 3–4 require accepted `wbt_map` / `wbt_set` (standard-plan §§8A–§10); do not permanently adopt legacy `btree_*` as a shortcut.
5. **Compile time / memory:** Full compiler batch may stress host compiler; may need staged `silica.config` shards before monolithic config.
6. **Scope creep:** Phase 4 parser constraint migration—default **keep lists** unless profiling shows need.
7. **Trait compiler gaps:** `provided` blocks, graph bracket witnesses, and boolean `found` shapes from traits doc may block clean `OrderedMap@get` adoption until compiler obligations are met.
8. **AST migration breadth:** Parser `Expr` is consumed across parsing, checking, cleanup, and SIR lowering. Phase 7 uses a bidirectional bridge and bounded consumer groups so representation conversion never becomes a flag-day rewrite.
9. **Traversal regression:** Replacing direct binary fields with repeated root-relative path lookup can create `O(nh)` passes. Zipper/cursor operation counters are a Phase 7 exit gate.

## Completion Tracking

| Area | Status | Notes |
| ---- | ------ | ----- |
| Phase 0 audit | Partial | Workaround sites catalogued in Step 0.2; full W-id ownership still open for §12 |
| `src_selfhost/` parallel tree | Stood up | Copy of `src/` (2026-07-16); sole edit target until cutover |
| Frozen `src/` + bootstrap | Retained | Untouched for self-host migration until Phase 6.3–6.4 |
| Build system dual-mode | Partial (§12) | Default: bootstrap → `src/`; `assembly-selfhost`/`build-selfhost` on `src_selfhost/`; batch blocked on dialect rewrite |
| DeviceIO file intrinsics in seed | Staging binary | Implemented in `src_selfhost/` + `src_staging_deviceio/` overlay; frozen `src/` sources untouched; binary artifact may be refreshed |
| Named-struct dialect rewrite | In progress (§12 A–C) | Lexer slice + tree-wide `SourceLocation` done; ~88 named structs remain; Expr/SIRTerm index-arena still open |
| Runtime link | Bootstrap `.a` | Self-host path moves to self-emitted runtime; bootstrap path unchanged until cutover |
| `data_structures/bst.silica` | Gone from `src_selfhost/` | Still in frozen `src/`; emitter pools use `compiler_maps` + WBT |
| `ListSymbolEntry` | In use | Migrate with Wave A → `List[…]`; Phase 4 still owns WBT map upgrade for keyed lookups |
| `type TokenKind = int64` | Cleared in `src_selfhost/` | Still present in frozen `src/`; Global Rule 9 |
| `wbt_map` / `wbt_set` stdlib | Accepted (§§8A–§10) | Unblocks Phase 3; see Step 0.3 |
| `compiler_maps` + emitter WBT | Done in parallel tree | Smoke: `self_host_maps/compiler_string_index_map` |
| Compiler trait obligations | Not complete | See traits doc §Compiler obligations |
| `tree_binary` / `BinaryTree` stdlib | Planned | Standard-plan §§7.10, 8D, 9D; optional Phase 7 upgrade from §12 arena |
| Parser `Expr` / `SIRTerm` seed-legal form | Open (§12 Wave C) | Index-arena for §12 exit; BinaryTree optional Phase 7 |
| Stray `src/btree_set_nodeid.silica` | Orphan | Do not adopt; remove from frozen `src/` only when safe / at cutover |
| Bootstrap retirement | Not started | Gate: parallel Phases 5.1+3+dialect+1+6.1, then cutover 6.3–6.4 |
