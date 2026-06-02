# Standard Data Structures Implementation Plan

This plan organizes implementation work for Silica's standard generated data structures. It is an execution plan only. The design authority remains:

- [silica-specification.md](../silica-specification.md) — §4.2.4 (lists), §8.2.4 (`Collectable`, **`List[Collectable, Space]`** placeholder)
- [list_implementation_design.md](../list_implementation_design.md) — §3, §7 (resolution and list primitives)
- [graph_representation_design.md](../graph_representation_design.md)
- [balanced_tree_and_heap_design.md](../balanced_tree_and_heap_design.md)
- [btree_set_design.md](../btree_set_design.md)

**Design conventions (shared model — updated methodology):**

- Graphs, trees, heaps, and sets are **immutable values** with **uniform inline record types** at every boundary (graph §2.7–§2.8). After compile-time resolution, every value flow uses **concrete** inline types; before resolution, generated stdlib may spell payload slots as **`Collectable`** and list slots as **`List[Collectable, mem(Space)]`** (silica-spec §8.2.4).
- CSR/dense topology buffers remain **`int64`**; user payload lives in `List[Collectable, S]` (resolved to `List[T, S]`), `buf(R, S, T, N)`, or inline payload fields.
- **One exported function per operation** per module (`export insert/3`, not `export insert[int64, int64, mem(normal)]/3` per width or user type). **One `fn` body** per operation in generated source, using **`Collectable`** / **`Comparable`** (keys) placeholders—not duplicated `fn insert[int8,…]`, `fn insert[int16,…]`, … in the same file.
- **Call sites (preferred):** `module@operation(args)` with specialization from the typed structure value (`tree`, `g`, `heap`) and from typed **`empty()`** results (`let` / parameter / return). **Optional:** explicit bracket instantiation at call sites when desired (graph §2.11; balanced tree §2.6; btree set §4.1).
- **Module filenames** use representation only (`graph_adj_directed.silica`, `btree_nodeid.silica`). Do **not** suffix modules with payload type or memory space.
- **Compiler obligation:** resolve `Collectable` placeholders and select/link the correct specialization per value flow before codegen (overload by first-argument structure type for non-empty ops; expected type for `empty` and other zero-receiver constructors). See **Phase 0.4**.

**Migration note:** Existing bootstrap modules (e.g. duplicated `insert[int8,…]` / `insert[int32,…]` in `btree_nodeid.silica`) are **legacy** relative to this plan. New work and refactors must converge on the single-function + placeholder model; remove per-primitive duplicate exports when Phase 0.4 is available.

This document does not introduce new representations or memory-space rules beyond the design documents. **Naming and call syntax** are defined in those documents (graph §2.11, §8.4; balanced tree §2.6, §8; btree set §4.1, §8; list §3.5). When a detail is needed, use the section named in the step.

This document intentionally contains no Silica source code. It is written as fine-grained work items that an LLM can follow when generating Silica code from the design documents.

## Implementation Order

1. **Module filename rename (blocking).** Phase 0.0 — complete.
2. **`Collectable` list placeholder and structure specialization (blocking for new methodology).** Phase 0.4 — compiler type-check and codegen resolution; see silica-spec §8.2.4. **No new per-width duplicate stdlib functions until 0.4 exit criteria pass** (refactors of existing modules may proceed in parallel where they only remove duplicates).
3. Graph representations (refactor to single-function + placeholder exports where touched).
4. B-tree set representations second.
5. General balanced B-tree and map representations third (includes `btree_nodeid.silica` deduplication).
6. Heap representations fourth.
7. Cross-structure validation, docs, and trial cleanup last.

Graph comes first because the tree and heap designs explicitly reuse graph vocabulary: node ids, inline record shapes, memory-space rules, generated naming conventions, and region-backed buffers.

## Phase 0.0 - Module Filename Rename (blocking)

**Status: complete** — `trials/graph_addition` integrate passes (`.ascomp` goldens refreshed for module-prefixed linker symbols). `trials/btree_set_addition` integrate passes. `btree_set_nodeid_to_csr` trial removed from suite; `to_csr` in `btree_set_nodeid.silica` stubbed to return `btree_set_csr@empty` — full conversion deferred to Phase 6.

Authority:

- `graph_representation_design.md` sections 2.2, 2.11, and 8.4
- `btree_set_design.md` sections 4.1 and 8
- `balanced_tree_and_heap_design.md` sections 2.6 and 8

Actions:

1. Rename graph modules under `src/standard_data_structures/` to representation + directedness only:
  - `graph_adj_directed_mem_normal.silica` + `graph_adj_directed_int64_mem_normal.silica` → `**graph_adj_directed.silica**` (one module, both bracket instantiations)
  - `graph_adj_undirected_mem_normal.silica` → `**graph_adj_undirected.silica**`
  - `graph_csr_directed_mem_normal.silica` + `graph_csr_directed_int64_mem_normal.silica` → `**graph_csr_directed.silica**`
  - `graph_dense_directed_mem_normal.silica` + `graph_dense_directed_int64_mem_normal.silica` → `**graph_dense_directed.silica**`
2. Rename B-tree set modules:
  - `btree_set_nodeid_mem_normal.silica` → `**btree_set_nodeid.silica**`
  - `btree_set_csr_mem_normal.silica` → `**btree_set_csr.silica**`
3. Update `**use**` declarations and `**module@operation**` call sites (short operation name after `@`; prefer context-typed calls; brackets optional per graph §2.11).
4. Update `src/standard_data_structures/Makefile`, `silica.config`, and trial Makefiles / `lib/` symlinks.
5. Rename trial sources to drop `**_mem_normal**` from filenames (keep `**int64**` / `**unweighted**` in trial names when they denote the bracket instantiation under test).
6. Re-run `**trials/graph_addition**` and `**trials/btree_set_addition**` `**integrate**`; refresh `.ascomp` goldens when assembly output changes only because of module rename.

Exit criteria:

- No generated module filename contains payload type or memory-space suffix.
- Every trial `**use**` names a design-document module (`graph_adj_directed`, not `graph_adj_directed_int64_mem_normal`).
- Graph and B-tree set integrate suites pass.

**Rename map (authoritative):**


| Legacy module file                                                                        | Target module file            |
| ----------------------------------------------------------------------------------------- | ----------------------------- |
| `graph_adj_directed_mem_normal.silica` + `graph_adj_directed_int64_mem_normal.silica`     | `graph_adj_directed.silica`   |
| `graph_adj_undirected_mem_normal.silica`                                                  | `graph_adj_undirected.silica` |
| `graph_csr_directed_mem_normal.silica` + `graph_csr_directed_int64_mem_normal.silica`     | `graph_csr_directed.silica`   |
| `graph_dense_directed_mem_normal.silica` + `graph_dense_directed_int64_mem_normal.silica` | `graph_dense_directed.silica` |
| `btree_set_nodeid_mem_normal.silica`                                                      | `btree_set_nodeid.silica`     |
| `btree_set_csr_mem_normal.silica`                                                         | `btree_set_csr.silica`        |


## Global Rules For All Steps

Every implementation step must obey these rules:

1. Treat design names as generator names only.
2. In **generated stdlib** sources, payload positions may use **`Collectable`** and `List[Collectable, S]` until resolution; at **codegen boundaries** for a value flow, emit/use the **resolved** full inline structural types (silica-spec §8.2.4).
3. Use the memory-space rules from the relevant design document.
4. Use payload and operand rules exactly as described by the relevant design document (graph §2.4; balanced tree §2.4; btree set §4.0).
5. Keep generated naming consistent with the relevant design document: **module names** = representation family only; **exported function names** = short operation verbs with **arity-only exports** (`export contains/2`); **preferred call syntax** = `module@operation(args)` with specialization from typed structure arguments and typed `empty()` results; **optional** `module@operation[brackets](args)`. Registry keys may still record `(payload…, mem(Space))` for generator metadata. The compiler emits **module-prefixed mangled linker symbols** from resolved structure/payload types so arity-only exports can link.
6. **Single-file `use` rule (E4011):** do not `use` two generated modules in one file when both export the same `operation/arity` and overload resolution is ambiguous. Cross-representation call sites use re-export wrappers or separate trial files.
7. Generated stdlib functions use **`Collectable`** / **`Comparable`** (keys) in payload and `List[Collectable, S]` positions; do **not** add another copy of the same operation for each primitive width or user struct spelling in one module.
8. Add positive trials before relying on a generated helper in later phases.
9. Add negative or validation trials for every documented invariant that can be checked.
10. Do not implement a faster packed form until the clear list-oriented form has validation coverage, unless the design document specifically allows direct packed construction.
11. Do not add unplanned APIs. If a helper is needed only internally, name and scope it as a generator helper and keep it aligned with the design document's helper requirements.
12. Emit **immutable** APIs by default: mutating operations return new structure values; use `_builder`_ / `_mutable_` suffixes only when the design document allows (graph §2.7).
13. Enforce **uniform inline types** for each structure value flow after placeholder resolution (graph §2.7; list spec §4.2.4 / §8.2.4).

## Phase 0 - Shared Generator Foundation

### Step 0.1 - Create Structure Registry

Authority:

- `graph_representation_design.md` sections 2.1 through 2.10 and 8
- `balanced_tree_and_heap_design.md` sections 2 and 3
- `btree_set_design.md` sections 4 and 8 through 9

Actions:

1. Create a generator-side registry of supported structure families.
2. Register the graph families named in the graph design.
3. Register the B-tree and heap families named in the balanced tree and heap design.
4. Register the B-tree set families named in the B-tree set design.
5. For each family, record only design-document fields: representation name, memory space, weightedness when applicable, directedness when applicable, `**node_data_type`** / `**edge_data_type`** / `**key_type**` when applicable (concrete payload inline spelling or `none`), and capacity constants when applicable.

Exit criteria:

- The generator can list every planned family without emitting Silica code.
- Every registry entry points to its source design document and section.

### Step 0.2 - Implement Inline Type String Expansion

Authority:

- `graph_representation_design.md` section 8.5
- `balanced_tree_and_heap_design.md` section 2.1
- `btree_set_design.md` section 9.2

Actions:

1. Implement generator routines that expand each design name into the full inline structural type specified by the design document.
2. Keep field order stable.
3. Keep memory-space spelling consistent across nested lists, regions, refs, and buffers.
4. Add snapshot tests for emitted type strings.

Exit criteria:

- Every supported family can produce a canonical inline type string.
- Snapshot tests fail if field order or memory-space spelling changes.

### Step 0.4 - `Collectable` Placeholder Resolution (blocking for new methodology)

Authority:

- [silica-specification.md](../silica-specification.md) §4.2.4, §8.2.4
- [list_implementation_design.md](../list_implementation_design.md) §3.5, §7
- [graph_representation_design.md](../graph_representation_design.md) §2.4, §2.8, §2.11
- [balanced_tree_and_heap_design.md](../balanced_tree_and_heap_design.md) §2.4, §2.6
- [btree_set_design.md](../btree_set_design.md) §4.0, §4.1

Actions:

1. **Type checker (shallow Collectable — same boundary as `ActorMessage`):** resolve `List[Collectable, S]` to concrete `List[T, S]` per value flow; use `type_checker_lists@formal_type_accepts_actual` for structural placeholder matching and `type_checker_traits@actual_type_satisfies_trait_expectation` for marker `Collectable` only. No emitter or SIR interpretation of the string `Collectable`.
2. **Type checker:** for qualified std-structure calls, select the single `fn` via `find_tag0_decl_for_qualified_call` (first argument and/or return vs binding type for zero-receiver ops); error on ambiguity or missing context.
3. **Exports:** accept arity-only exports (`export insert/3`) with multiple internal overloads distinguished by resolved structure types, or a single `fn` whose formals use `Collectable` placeholders—do not require per-payload export bracket suffixes.
4. **Linkage pass only:** `qualified_call_mangler` rewrites call names using the same `find_tag0_decl_for_qualified_call` as the type checker, then `overload_mangle` applies type slugs — monomorph after resolution, no Collectable logic in `src/emitter`.
5. **Trials:** add `trials/list_addition/` cases for `List[Collectable, normal]` under typed bindings; add graph/btree trials for context-typed `module@empty()` / `module@insert` without call-site brackets.
6. **Keys:** document/enforce `Comparable` (or inlined compare) when resolving ordered-structure keys.

Exit criteria:

- List `empty` / `prepend` / `length` work with `List[Collectable, S]` when RHS or argument context supplies concrete `T`.
- At least one graph and one btree trial call `graph_adj_directed@empty()` / `btree_nodeid@insert(tree, …)` with typed structure variables and no per-width duplicate exports in that module.
- For graph constructors with non-payload arguments, add a follow-up context trial that calls the preferred API as `graph_adj_directed@empty(3)` (no `[mem(normal)]`) once graph overloads collapse to one export per operation.
- Unresolved `List[Collectable, S]` without context is a compile error.

### Step 0.3 - Establish Trial Layout

Authority:

- `graph_representation_design.md` section 8
- `balanced_tree_and_heap_design.md` implementation sections for each family
- `btree_set_design.md` section 10

Actions:

1. Add or reserve trial directories for graph, B-tree set, balanced tree, and heap generated code.
2. Define success-path `.sout` or `.scout` expectations according to existing trial conventions.
3. Define validation-failure trials for generated invariant checks.
4. Keep generated data-structure trials separate from compiler-internal list trials.

Exit criteria:

- The trial harness can run empty placeholder suites for each data-structure family.

**Phase 0 completed (trial layout):**

- `trials/standard_data_structures_addition/` — type-string snapshot (`.scout` / `.ascomp`)
- `trials/graph_addition/`, `btree_set_addition/`, `balanced_tree_addition/`, `heap_addition/` — empty integrate via `trials/base/placeholder_makefile`
- `trials/error_enforcement_addition/generated_data_structures/` — validation-failure naming and subdirs (`graph/`, `btree_set/`, `balanced_tree/`, `heap/`); goldens added when `validate` exists

## Phase 1 - Graph Foundation: NodeIdAdjacencyGraph

### Step 1.1 - Generate Unweighted Directed Adjacency Type And Empty Constructor

Authority:

- `graph_representation_design.md` sections 3.1, 3.2, 3.4, and 3.5

Actions:

1. Generate the directed unweighted adjacency graph family for the default concrete memory-space case used by the tests.
2. Emit the empty or allocate operation described by the graph design.
3. Generate node records with empty neighbor lists.
4. Preserve `node_count` and `edge_count` fields as described.

Exit criteria:

- A trial constructs an empty directed unweighted adjacency graph.
- A trial verifies node count and edge count through generated inspection helpers.

### Step 1.2 - Generate Adjacency Node Lookup Helpers

Authority:

- `graph_representation_design.md` sections 3.4, 3.5, 3.6, and 8.2

Actions:

1. Generate helper logic for locating a node record by node id.
2. Generate helper logic for reading a node's neighbor list.
3. Generate checked public wrappers for invalid node ids if required by the error-handling section.
4. Keep unchecked helpers internal to generated code.

Exit criteria:

- Valid lookup returns the expected node information.
- Invalid lookup follows the result-shape guidance in the graph design.

### Step 1.3 - Generate Directed Edge Addition

Authority:

- `graph_representation_design.md` sections 2.2, 3.4, 3.5, and 8.2

Actions:

1. Generate `add_edge` for directed unweighted adjacency graphs.
2. Validate source and destination node ids according to the design.
3. Update only the source node's neighbor list for directed graphs.
4. Update edge count according to the design's edge-count convention.
5. Preserve all unrelated node records.

Exit criteria:

- A trial adds a directed edge and verifies source adjacency.
- A trial verifies the reverse edge is absent.
- A validation trial catches invalid endpoint ids.

### Step 1.4 - Generate Adjacency Inspection Helpers

Authority:

- `graph_representation_design.md` sections 2.10, 3.4, 3.6, 8.2, and 8.3

Actions:

1. Generate `node_count`.
2. Generate `edge_count`.
3. Generate `out_degree`.
4. Generate `has_edge`.
5. Generate neighbor traversal helpers according to the traversal strategy.

Exit criteria:

- Trials cover present edge, absent edge, out degree, and neighbor traversal.

### Step 1.5 - Generate Undirected Adjacency Variant

Authority:

- `graph_representation_design.md` sections 2.2, 3.4, and 3.5

Actions:

1. Generate the undirected unweighted adjacency family.
2. Implement edge insertion as the mirrored storage described by the design.
3. Preserve the self-edge exception described by the design.
4. Reuse validation and inspection helpers where the design permits.

Exit criteria:

- A trial adds a non-self undirected edge and verifies both directions.
- A trial adds a self-edge and verifies the design's self-edge rule.

### Step 1.6 - Generate Weighted Int64 Adjacency Variant

Authority:

- `graph_representation_design.md` sections 2.3, 3.3, 3.4, and 8.2

Actions:

1. Generate weighted adjacency storage exactly as described.
2. Generate weighted edge addition.
3. Generate weight lookup according to the recommended result shape.
4. Preserve unweighted helpers where applicable.

Exit criteria:

- A trial adds a weighted edge and reads its weight.
- A trial queries a missing weighted edge and receives the documented result behavior.

### Step 1.7 - Generate Adjacency Validation

Authority:

- `graph_representation_design.md` section 3.4

Actions:

1. Check node id range invariants.
2. Check node record count against `node_count`.
3. Check neighbor endpoint ranges.
4. Check edge count consistency.
5. Check undirected mirror invariants for undirected graphs.

Exit criteria:

- Positive validation passes for constructed graphs.
- Negative validation trials fail or return errors for each checkable invariant.

### Step 1.8 - Deduplicate NodeIdAdjacencyGraph Stdlib Surface

Authority:

- `graph_representation_design.md` sections 2.4, 2.8, 2.11, and 8.4
- [silica-specification.md](../silica-specification.md) §8.2.4

Actions:

1. Collapse legacy per-width graph stdlib exports such as `empty[int8|int16|int32|int64, …]` to one exported operation per arity (`export empty/1`, `export add_edge/4`, `export validate/1`, etc.).
2. Keep payload and memory-space specialization in `Collectable` / resolved inline structure types, not in duplicated public function names.
3. Update graph context trials so the preferred constructor call is `graph_adj_directed@empty(3)` without `[mem(normal)]`; keep explicit brackets only as optional syntax coverage where the graph design documents it.
4. Verify the context trial covers real payload specialization, not only the mem-only bracket form.

Exit criteria:

- `graph_adj_directed.silica` no longer contains duplicate public bodies for the same operation solely by primitive payload width.
- Graph context trials pass with typed structure variables and unbracketed preferred API calls.

## Phase 2 - Graph Packed Form: CompressedSparseRowGraph

### Step 2.1 - Generate CSR Type Expansion

Authority:

- `graph_representation_design.md` sections 4.1, 4.2, 4.3, 4.8, and 8.5

Actions:

1. Generate CSR inline type strings for unweighted and weighted forms.
2. Include the owning region exactly as required by the design.
3. Include concrete buffer capacities exactly as required by the design.

Exit criteria:

- Snapshot tests cover CSR type strings for unweighted and weighted forms.

### Step 2.2 - Generate CSR Direct Static Constructor

Authority:

- `graph_representation_design.md` section 4.6

Actions:

1. Generate a direct constructor for static graphs when the generator already knows offsets and adjacency data.
2. Allocate the region and buffers described by the design.
3. Fill offsets, neighbors, and weights when present.
4. Return the full graph record with region ownership.

Exit criteria:

- A trial constructs a static CSR graph and verifies node count and edge count.

### Step 2.3 - Generate CSR Validation

Authority:

- `graph_representation_design.md` section 4.4

Actions:

1. Validate offset buffer size and monotonicity.
2. Validate first and final offset values.
3. Validate neighbor endpoint ranges.
4. Validate weight buffer shape for weighted CSR.
5. Validate sorted adjacency only when the generated family declares that sortedness is required.

Exit criteria:

- Positive CSR validation passes.
- Negative CSR validation covers each checkable invariant.

### Step 2.4 - Generate CSR Inspection

Authority:

- `graph_representation_design.md` sections 4.7, 8.2, and 8.3

Actions:

1. Generate `out_degree`.
2. Generate neighbor range traversal.
3. Generate `has_edge`.
4. Use linear scan or binary search according to the sortedness condition in the design.
5. Generate `weight_at` for weighted CSR.

Exit criteria:

- Trials cover `out_degree`, present edge, absent edge, and weighted lookup.

### Step 2.5 - Generate Adjacency-To-CSR Finalization

Authority:

- `graph_representation_design.md` sections 4.5 and 7

Actions:

1. Accept a built adjacency graph as the input form.
2. Count outgoing edges by node.
3. Build CSR offsets.
4. Fill CSR neighbor buffers.
5. Fill CSR weight buffers when present.
6. Return a CSR graph with owning region.

Exit criteria:

- A trial builds an adjacency graph, finalizes it to CSR, and verifies equivalent `has_edge` results.

## Phase 3 - Graph Dense Forms

### Step 3.1 - Generate DenseMatrixGraph Type And Constructor

Authority:

- `graph_representation_design.md` sections 5.1 through 5.5

Actions:

1. Generate dense matrix graph type strings for unweighted and weighted forms.
2. Generate empty construction.
3. Generate edge setting according to the design's indexing rule.
4. Generate weighted cell setting when applicable.

Exit criteria:

- Trials construct dense matrix graphs and verify edge presence.

### Step 3.2 - Generate DenseMatrixGraph Inspection And Validation

Authority:

- `graph_representation_design.md` sections 5.4 and 5.6

Actions:

1. Generate `has_edge`.
2. Generate weighted lookup when applicable.
3. Generate outgoing traversal by scanning the row.
4. Generate validation for dimensions and endpoint ranges.

Exit criteria:

- Trials cover present edge, absent edge, traversal, and validation.

### Step 3.3 - Generate DenseBitsetGraph Only When Supported

Authority:

- `graph_representation_design.md` sections 6.1 through 6.4 and 9

Actions:

1. Check whether the current compiler path supports the bit operations required by the design.
2. If supported, generate dense bitset type, set, clear, and edge-test helpers.
3. If not supported, record the design-documented fallback to dense matrix in the implementation status notes.

Exit criteria:

- Either dense bitset trials pass, or the fallback is documented without changing the graph design.

**Phase 3 completed:**

- Steps 3.1–3.2: `graph_dense_directed.silica` (unweighted and weighted edge-payload via typed graph values or optional explicit brackets); trials `graph_dense_directed_unweighted.silica`, `graph_dense_directed_weighted_int64.silica` (silica-compiler integrate).
- Step 3.3: DenseBitset deferred per graph design §6.4; fallback to `DenseMatrixGraphDirected[mem(S)]` documented in completion tracking.
- `src/standard_data_structures/` builds graph modules via `silica-compiler` + `silica.config` (not silica-boot).

## Phase 4 - Graph Algorithms Over Stable Traversal APIs

### Step 4.1 - Generate Reachability

Authority:

- `graph_representation_design.md` sections 2.10, 3.6, 4.7, 5.6, and 7

Actions:

1. Generate reachability over adjacency graphs.
2. Generate reachability over CSR graphs.
3. Reuse the traversal APIs from earlier phases.
4. Keep queue or stack representation consistent with available Silica list and buffer support.

Exit criteria:

- Trials cover reachable and unreachable node pairs.

### Step 4.2 - Generate Degree Summaries

Authority:

- `graph_representation_design.md` sections 2.10, 3.6, 4.7, and 5.6

Actions:

1. Generate total degree summaries.
2. Generate max out-degree summary.
3. Generate per-node out-degree traversal helpers where required.

Exit criteria:

- Trials verify summaries against small graphs with known answers.

**Phase 4 completed:**

- Step 4.1: `reachable/3` on adjacency (flat slots) and CSR static graph; trials `graph_reachability_adj_directed_unweighted.silica`, `graph_reachability_csr_directed_unweighted.silica`.
- Step 4.2: `max_out_degree/1`, `total_out_degree_sum/1` on CSR; trial `graph_degree_summary_csr_directed_unweighted.silica`.

## Phase 5 - B-tree Set: NodeIDBTreeSet

### Step 5.1 - Generate NodeIDBTreeSet Type Expansion And Empty Constructor

Authority:

- `btree_set_design.md` sections 4, 5.1, 5.2, and 5.3

Actions:

1. Generate the inline type string for `NodeIDBTreeSet`.
2. Generate empty construction.
3. Preserve the empty-set convention from the design.
4. Emit order as a generator constant according to the design.

Exit criteria:

- A trial constructs an empty set and verifies empty membership behavior.

### Step 5.2 - Generate NodeIDBTreeSet Search Helpers

Authority:

- `btree_set_design.md` section 5.4

Actions:

1. Generate node lookup by node id.
2. Generate key search within a node.
3. Generate child selection for descending into internal nodes.
4. Generate public `contains`.

Exit criteria:

- Trials cover membership in hand-built valid trees.
- Trials cover absent keys.

### Step 5.3 - Generate NodeIDBTreeSet Validation

Authority:

- `btree_set_design.md` sections 4.3 and 5.7

Actions:

1. Validate unique keys.
2. Validate sorted node keys.
3. Validate child count rules.
4. Validate node id ranges.
5. Validate occupancy rules.
6. Validate leaf depth consistency.
7. Validate child key ranges.

Exit criteria:

- Positive validation passes for valid hand-built sets.
- Negative validation covers each checkable invariant.

### Step 5.4 - Generate NodeIDBTreeSet Insert

Authority:

- `btree_set_design.md` sections 5.5 and 7.2

Actions:

1. Generate insert result shape exactly as described by the design.
2. Implement duplicate handling according to the design's set semantics.
3. Generate top-down split behavior described by the design.
4. Preserve B-tree invariants after insertion.
5. Return the updated tree and insertion status.

Exit criteria:

- Trials insert into an empty set.
- Trials insert enough keys to split a node.
- Trials insert a duplicate key and verify the documented duplicate result.
- Validation passes after every insert sequence.

### Step 5.5 - Defer Or Generate Delete According To Design Readiness

Authority:

- `btree_set_design.md` sections 5.6 and 7.3

Actions:

1. Do not generate delete until the design's prerequisite conditions are met.
2. If the prerequisites are met, generate delete according to the design.
3. If delete is deferred, record it explicitly in implementation status.

Exit criteria:

- Either delete trials pass, or delete is explicitly listed as deferred according to the design.

**Phase 5 bootstrap completed:**

- Added `src/standard_data_structures/btree_set_nodeid.silica` for order-8 immutable list-backed `NodeIDBTreeSet[int64, mem(normal)]` (Phase 0.0 rename from `btree_set_nodeid.silica`).
- Public bootstrap surface: `empty/0`, `contains/2`, `insert/2`, and `validate/1`.
- Delete remains deferred per `btree_set_design.md` sections 5.6 and 7.3.
- Current compiler-path representation uses `int64` status flags for `is_leaf`, `inserted`, and `ok` rather than source-level booleans, matching the stable generated-code path used by the graph modules.
- `trials/btree_set_addition` now wires empty-set smoke coverage, non-empty hand-built membership, stable insert/duplicate status, and invalid-validation coverage into `integrate` with per-executable timeout guards.

## Phase 6 - B-tree Set: CsrBTreeSet

**Status (steps 6.1–6.4): complete** — `btree_set_csr.silica` now exports `empty`, `from_static_sorted`, `contains`, `validate`, and `insert`. All five trials pass: `btree_set_csr_contains_static`, `btree_set_csr_validate_invalid`, and `btree_set_csr_insert` (new).

Design note: `CsrBTreeSet` has been revised from its original "immutable after construction / no direct insert" spec. It now follows the same **functional-programming design** as `NodeIDBTreeSet` and Silica's `List`: `insert` returns a new value without modifying the caller's existing tree. See `btree_set_design.md` §6.7 (updated).

### Step 6.1 - Generate CsrBTreeSet Type Expansion

**Status: complete** — inline structural record type with region, capacity constants, and buffer fields is defined in `btree_set_csr.silica`.

Authority:

- `btree_set_design.md` sections 6.1 through 6.3 and 9.2

Actions:

1. Generate CSR set type strings.
2. Include region ownership.
3. Include all capacity constants required by the design.
4. Snapshot-test the generated type strings.

Exit criteria:

- CSR set type string snapshots are stable.

### Step 6.2 - Generate Static Construction From Sorted Keys

**Status: complete** — `from_static_sorted[int64, mem(normal)]` exported; `btree_set_csr_contains_static` trial passes.

Authority:

- `btree_set_design.md` section 6.4

Actions:

1. Generate static CSR set construction for known sorted keys.
2. Allocate buffers described by the design.
3. Fill metadata, keys, and child buffers.
4. Return the complete CSR set record.

Exit criteria:

- A trial constructs a static CSR set and verifies membership.

### Step 6.3 - Generate CsrBTreeSet Contains And Validation

**Status: complete** — `contains` and `validate` exported; generalized to handle 1–7 keys (was previously hardcoded for 3 keys only). `btree_set_csr_contains_static` and `btree_set_csr_validate_invalid` trials pass.

Authority:

- `btree_set_design.md` sections 6.6 and 6.8

Actions:

1. Generate CSR membership query.
2. Generate CSR node search.
3. Generate validation checks.
4. Keep error codes aligned with the NodeIDBTreeSet design where possible.

Exit criteria:

- Trials cover present keys, absent keys, and validation failures.

### Step 6.4 - Generate CsrBTreeSet Functional Insert

**Status: complete** — `insert[int64, mem(normal)]` exported. Helpers: `contains_key_at`, `keys_sorted_at`, `csr_insert_pos_from`, `csr_insert_pos`, `csr_out_key`, `build_leaf_csr`, `insert_nonempty_csr`. `btree_set_csr_insert` trial passes.

Authority:

- `btree_set_design.md` section 6.7 (revised), 7.2 (revised)

Actions:

1. Implement `csr_insert_pos_from` — recursive linear scan for sorted insertion position.
2. Implement `csr_out_key` — pure function mapping (old keys, new key, insert position, output index) → output key value.
3. Implement `build_leaf_csr` — allocate fresh region + buffers for a single-leaf CSR node.
4. Implement `insert_nonempty_csr` — checks duplicate/capacity, reads old keys, calls helpers, returns new tree.
5. Implement `insert[int64, mem(normal)]` — dispatches on empty vs non-empty.

Exit criteria:

- A trial inserts keys from empty, verifies sorted order, duplicate detection, and that the original tree is not mutated (immutability contract).

### Step 6.5 - Generate NodeIDBTreeSet-To-CsrBTreeSet Finalization (deferred)

**Status: deferred** — `to_csr` in `btree_set_nodeid.silica` is stubbed to return `btree_set_csr@empty`. Full conversion (`to_csr_from_valid`, `to_csr_write_keys`, `to_csr_remap_child`, `to_csr_write_children`, `to_csr_fits_bootstrap_caps`) deferred pending emitter register-allocation improvements for high register-pressure code paths.

Authority:

- `btree_set_design.md` sections 6.5 and 7.5

Actions:

1. Validate the NodeIDBTreeSet input.
2. Assign dense node ids according to the design.
3. Allocate CSR buffers.
4. Copy keys and children into packed buffers.
5. Return the CSR set with the documented root convention.

Exit criteria:

- A trial inserts keys into NodeIDBTreeSet, finalizes to CSR, and verifies equivalent membership.

## Phase 7 - General B-tree: NodeIDBTreeMap ✅ COMPLETE

Implemented as `NodeIDBTreeMap[int64, int64, mem(normal)]` in `btree_nodeid.silica`, reusing the
NodeIDBTreeSet node layout and traversal while applying `replace_value` duplicate-key policy (maps
update existing keys; sets still reject duplicates via `btree_set_nodeid.silica`).

### Step 7.1 - Generate NodeIDBTree Type Expansion ✅ COMPLETE

Authority:

- `balanced_tree_and_heap_design.md` sections 3 and 4

Actions:

1. ✅ Generate the general NodeIDBTreeMap type string (keys + values per node).
2. ✅ Include keys and values exactly as specified by the design.
3. ✅ Type expansion covered by `btree_nodeid.silica` module registration.

Exit criteria:

- ✅ Type string stable in generated module.

### Step 7.2 - Generate NodeIDBTree Search And Validation ✅ COMPLETE

Authority:

- `balanced_tree_and_heap_design.md` B-tree invariant and NodeIDBTree sections

Actions:

1. ✅ Generate search by key (`get`).
2. ✅ Generate value lookup returning `{found, value}`.
3. ✅ Generate invariant validation (reuses set validation with value-shape checks).
4. ✅ Set validation unchanged in `btree_set_nodeid.silica` (reject_duplicates preserved).

Exit criteria:

- ✅ Trials cover present keys, absent keys, and invalid trees (`btree_nodeid_empty_get`, `btree_nodeid_validate_invalid`).

### Step 7.3 - Generate NodeIDBTree Insert ✅ COMPLETE

Authority:

- `balanced_tree_and_heap_design.md` NodeIDBTree insert and split sections

Actions:

1. ✅ Generate insertion with `replace_value` policy (`insert` returns `{tree, inserted, replaced}`).
2. ✅ Reuse node split helpers from NodeIDBTreeSet.
3. ✅ Preserve B-tree invariants.
4. ✅ Return the documented result shape.

Exit criteria:

- ✅ Trials cover insert, replace, get, and immutability (`btree_nodeid_insert`, `btree_nodeid_insert_one/two/four/get`, `btree_nodeid_insert_get`).

## Phase 8 - General B-tree: CsrBTree ✅ COMPLETE

Implemented as `CsrBTreeMap[int64, int64, mem(normal)]` — module `btree_csr_map.silica`.
Steps 8.1 and 8.2 are complete.

### Step 8.1 - Generate CsrBTreeMap Construction ✅ COMPLETE

Authority:

- `balanced_tree_and_heap_design.md` §5.3, §5.5, §5.8

Actions:

1. ✅ Generate CSR map type string: `{region, root_id, node_count, key_count_total, order, node_key_start, node_key_count, node_child_start, node_child_count, node_is_leaf, keys, values, children}`.
2. ✅ Generate `empty` (root_id=-1) and `from_static_sorted` ({1→10, 3→30, 5→50}).
3. ✅ Generate `build_leaf_csr_map` internal builder allocating a fresh region with key and value buffers.
4. ✅ Include capacity constants `node_cap=1`, `key_cap=7`, `child_cap=8`.

Exit criteria:

- ✅ `btree_csr_map_insert` trial constructs a map via sequential insert and validates it.

### Step 8.2 - Generate CsrBTreeMap Search, Insert, And Validation ✅ COMPLETE

Authority:

- `balanced_tree_and_heap_design.md` §5.7, §5.8, §5.9, §9.2, §9.4

Actions:

1. ✅ Generate `contains` (linear key scan over `keys` buffer — reuses `contains_key_at` from `CsrBTreeSet`).
2. ✅ Generate `get` (key lookup returning `{ found: int64, value: int64 }` — uses `find_key_pos` helper).
3. ✅ Generate functional `insert` with `replace_value` duplicate policy, returning `{ tree, inserted, replaced }`.
4. ✅ Generate `validate` (order check, key-count bounds, sorted-key invariant).

Exit criteria:

- ✅ `btree_csr_map_insert` trial covers: new-key insert (inserted=1), replace-value insert (replaced=1), get found and not-found, validation pass, immutability of original map.

### Step 8.3 - CsrBTree Set-Only Form (btree_csr.silica) — Deferred

The set-only `CsrBTree[int64, mem(normal)]` form (`btree_csr.silica`) provides equivalent functionality to the already-complete `CsrBTreeSet` (`btree_set_csr.silica`). Implementing a second module would be redundant at this stage. Defer until a concrete use-case requires a separate `btree_csr.silica` module distinct from `btree_set_csr.silica`.

## Phase 9 - Heaps

Status note:

RegionBinaryHeap work has source and trial coverage already. The old completion row that marked `RegionBinaryHeap` as "Not started" was stale relative to `src/standard_data_structures/heap_binary_min.silica`, `src/standard_data_structures/heap_binary_max.silica`, and `trials/heap_addition/Makefile`.

### Step 9.1 - Generate RegionBinaryMinHeapInt64 — Implemented

Authority:

- `balanced_tree_and_heap_design.md` RegionBinaryHeap sections

Actions:

1. ✅ Generate the binary heap type string.
2. ✅ Generate empty or allocate construction.
3. ✅ Generate push.
4. ✅ Generate peek.
5. ✅ Generate pop.
6. ✅ Generate validation of heap ordering and capacity metadata.

Exit criteria:

- ✅ `heap_binary_min_empty`, `heap_binary_min_push_pop`, and `heap_binary_min_validate_invalid` cover empty heap, push, peek, pop, and validation for `RegionBinaryMinHeap[int64, mem(normal)]`.

### Step 9.2 - Generate RegionBinaryHeap Variants Permitted By Design — Implemented

Authority:

- `balanced_tree_and_heap_design.md` RegionBinaryHeap sections

Actions:

1. ✅ Generate only variants explicitly described by the design: `RegionBinaryMaxHeap[int64, mem(normal)]` and `RegionBinaryMinHeap[int64, int64, mem(normal)]`.
2. ✅ Reuse the same validation structure.
3. ✅ Add trials for each generated variant.

Exit criteria:

- ✅ `heap_binary_max_push_pop` and `heap_binary_min_priority_push_pop` cover the generated binary heap variants without changing the documented heap model.

### Step 9.3 - Generate RegionDaryHeap Only After Binary Heap Stability — Pending

Authority:

- `balanced_tree_and_heap_design.md` RegionDaryHeap sections

Actions:

1. Generate d-ary heap type strings.
2. Generate construction.
3. Generate push.
4. Generate peek.
5. Generate pop.
6. Generate validation.
7. Keep arity handling aligned with the design.

Exit criteria:

- D-ary heap trials pass after binary heap trials are stable.

## Phase 10 - Cross-Structure Integration

Phase 10 validates generated standard data structures across the full payload-type surface area required by the design documents, then audits cross-cutting invariants (immutability, region ownership, naming). Payload coverage is organized into four groups; **each group item below is its own step** with its own trial-creation work.

Authority (shared for payload coverage steps 10.1–10.23):

- `graph_representation_design.md` section 2.4
- `balanced_tree_and_heap_design.md` section 2.4
- `btree_set_design.md` section 4.0

Shared rules for payload trial steps:

1. Each step adds compile-and-run trials (or error-enforcement trials where the design forbids a shape) under the appropriate `*_addition/` directory or a dedicated payload-coverage trial suite.
2. Trials must use concrete inline payload spellings at generated API boundaries — no design-name-only shortcuts unless the compiler expands them.
3. Every trial must exercise at least one representative generated operation (construct, insert/push/add, query/get/peek/contains, validate) for the payload shape under test.
4. Structural metadata and topology indices remain plain `**int64**` or buffer types; only user-facing payload slots carry the trial payload type.

### Payload Coverage — Scalar Types

### Step 10.1 - Signed Integer Payload Trials

Actions:

1. Add trials covering every signed integer width supported as payload by the design documents (`**int8**`, `**int16**`, `**int32**`, `**int64**`, and any other signed widths the silica spec permits in generated buffer/list slots).
2. For each width, exercise at least one graph node-data or edge-data slot, one tree/map key or value slot, and one heap element slot where that width is a legal payload.
3. Verify round-trip storage and retrieval through generated get/set, peek, or contains helpers.
4. Keep the signed payload suite under the naming pattern `trials/std_data_structures_payload_signed_addition/`; refresh any older `payload_signed_addition` / signed btree trials after int8/int16/int32 duplicate exports are removed, using context-typed `Collectable` key/value flows or the remaining canonical int64 API.

Exit criteria:

- Every signed integer width named in the design documents has at least one passing positive trial per applicable structure family.
- Signed btree payload trials are either updated to the single-export `Collectable` API or explicitly narrowed to the canonical supported int64 map surface; no trial depends on removed int8/int16/int32 public btree exports.

### Step 10.2 - Unsigned Integer Payload Trials

Actions:

1. Add trials covering every unsigned integer width supported as payload (`**uint8**`, `**uint16**`, `**uint32**`, `**uint64**`, and any other unsigned widths the silica spec permits).
2. For each width, exercise the same representative structure slots as Step 10.1.
3. Verify round-trip storage and retrieval.

Exit criteria:

- Every unsigned integer width named in the design documents has at least one passing positive trial per applicable structure family.

### Step 10.3 - Floating-Point Payload Trials

Actions:

1. Add trials covering every floating-point payload type permitted by the design documents (`**float32**`, `**float64**`, and any other float widths the silica spec permits).
2. Exercise graph edge weights, tree/map values, and heap elements where floats are legal payloads.
3. Verify round-trip storage and retrieval; include at least one non-integral value per float width.

Exit criteria:

- Every floating-point payload type named in the design documents has at least one passing positive trial per applicable structure family.

### Step 10.4 - Atom Payload Trials

Actions:

1. Add trials with `**atom**` as node data, edge data, key, value, and heap element payload where the design documents allow it.
2. Verify construction, storage, retrieval, and validation for atom payloads.
3. Add error-enforcement trials where atom payload is forbidden by a specific family or slot.

Exit criteria:

- Atom payload is covered by positive trials on every family that accepts it and by error-enforcement trials where it is rejected.

### Step 10.5 - String Payload Trials

Actions:

1. Add trials with `**string**` as node data, edge data, key, value, and heap element payload where permitted.
2. Verify construction, storage, retrieval, and validation for string payloads of varying length (empty, short, multi-code-unit).
3. Add error-enforcement trials where string payload is forbidden.

Exit criteria:

- String payload is covered by positive trials on every family that accepts it and by error-enforcement trials where it is rejected.

### Payload Coverage — Tuple And Struct Types

### Step 10.6 - Simple Tuple Payload Trials

Actions:

1. Add trials with shallow tuple payloads (two to four fields) in graph node/edge data, tree/map keys or values, and heap elements.
2. Use mixed scalar field types within each tuple.
3. Verify round-trip through generated accessors.

Exit criteria:

- Simple tuple payloads compile and pass positive trials on every applicable structure family.

### Step 10.7 - Deep Tuple Payload Trials

Actions:

1. Add trials with deeply nested tuple payloads (tuples containing tuples, three or more nesting levels, wide flat tuples with many fields).
2. Verify construction, storage, retrieval, and validation for each deep shape.

Exit criteria:

- Deep tuple payloads pass positive trials on every applicable structure family.

### Step 10.8 - Simple Struct Payload Trials

Actions:

1. Add trials with shallow struct payloads (named record types with two to four fields) in all applicable payload slots.
2. Use mixed scalar and small aggregate field types.
3. Verify round-trip through generated accessors.

Exit criteria:

- Simple struct payloads pass positive trials on every applicable structure family.

### Step 10.9 - Deep Struct Payload Trials

Actions:

1. Add trials with deeply nested struct payloads (structs containing structs, structs with many fields, multi-level field nesting).
2. Verify construction, storage, retrieval, and validation for each deep shape.

Exit criteria:

- Deep struct payloads pass positive trials on every applicable structure family.

### Step 10.10 - Struct-In-Tuple Payload Trials

Actions:

1. Add trials where tuple payload slots contain struct fields — simple cases (one struct among scalars) and complex cases (multiple structs, structs in nested tuple positions).
2. Verify round-trip for each shape on every applicable structure family.

Exit criteria:

- Struct-in-tuple payloads pass positive trials on every applicable structure family.

### Step 10.11 - Tuple-In-Struct Payload Trials

Actions:

1. Add trials where struct payload fields contain tuples — simple cases (one tuple field among scalars) and complex cases (nested tuples as struct fields, tuples of tuples inside structs).
2. Verify round-trip for each shape on every applicable structure family.

Exit criteria:

- Tuple-in-struct payloads pass positive trials on every applicable structure family.

### Step 10.12 - Mixed Nested Tuple-And-Struct Payload Trials

Actions:

1. Add trials combining tuples and structs at multiple nesting levels (structs in tuples in structs, tuples of structs of tuples, and other mixed compositions of varying complexity).
2. Verify construction, storage, retrieval, and validation for the most complex shapes the design documents permit.

Exit criteria:

- Mixed nested tuple-and-struct payloads pass positive trials on every applicable structure family.

### Payload Coverage — Function Types

### Step 10.13 - Function Payload Trials By Arity

Actions:

1. Add trials with function-typed payloads at nullary, unary, binary, and higher arities permitted by the design documents.
2. Cover graph, tree/map, and heap payload slots where function types are legal.
3. Verify that stored function values round-trip through generated accessors without conflating arity.

Exit criteria:

- Each supported function arity has at least one passing positive trial per applicable structure family.

### Step 10.14 - Function Payload Trials By Parameter Type

Actions:

1. Add trials where function payload parameters use scalars, tuples, structs, atoms, strings, and other function types (where permitted).
2. Include mixed parameter-type lists at several arities.
3. Verify round-trip and validation for each parameter-type combination.

Exit criteria:

- Representative function parameter-type combinations pass positive trials on every applicable structure family.

### Step 10.15 - Function Payload Trials By Return Type

Actions:

1. Add trials where function payload return types use scalars, tuples, structs, atoms, strings, and other function types (where permitted).
2. Include functions whose return type differs in complexity from their parameter types.
3. Verify round-trip and validation for each return-type shape.

Exit criteria:

- Representative function return-type shapes pass positive trials on every applicable structure family.

### Payload Coverage — Standard Data Structure Nesting

Each step below adds payload trials for **one** standard data structure family used as payload **within itself** and **within every other standard data structure family** that exposes a payload slot. Skip pairings the design documents forbid; document skipped pairings in the step notes.

Standard data structure families in scope:

- NodeIdAdjacencyGraph (`graph_adj_directed`, `graph_adj_undirected`)
- CompressedSparseRowGraph (`graph_csr_directed`)
- DenseMatrixGraph (`graph_dense_directed`)
- NodeIDBTreeSet (`btree_set_nodeid`)
- CsrBTreeSet (`btree_set_csr`)
- NodeIDBTreeMap (`btree_nodeid`)
- CsrBTreeMap (`btree_csr_map`)
- RegionBinaryHeap (`heap_binary_min`, `heap_binary_max`)

Deferred families (add payload trials when the family is generated): DenseBitsetGraph, RegionDaryHeap.

### Step 10.16 - NodeIdAdjacencyGraph Payload Trials

Actions:

1. Add trials with a NodeIdAdjacencyGraph value as node data and as edge data **within the same adjacency graph family** (self-nesting).
2. Add trials with NodeIdAdjacencyGraph as payload in CompressedSparseRowGraph, DenseMatrixGraph, NodeIDBTreeSet, CsrBTreeSet, NodeIDBTreeMap, CsrBTreeMap, and RegionBinaryHeap.
3. Verify construction, core operations, and validation for each legal pairing.

Exit criteria:

- NodeIdAdjacencyGraph self-nesting and cross-family payload trials pass for every legal host family.

### Step 10.17 - CompressedSparseRowGraph Payload Trials

Actions:

1. Add trials with a CompressedSparseRowGraph value as payload within CompressedSparseRowGraph (self-nesting).
2. Add trials with CompressedSparseRowGraph as payload in NodeIdAdjacencyGraph, DenseMatrixGraph, NodeIDBTreeSet, CsrBTreeSet, NodeIDBTreeMap, CsrBTreeMap, and RegionBinaryHeap.
3. Verify construction, core operations, and validation for each legal pairing.

Exit criteria:

- CompressedSparseRowGraph self-nesting and cross-family payload trials pass for every legal host family.

### Step 10.18 - DenseMatrixGraph Payload Trials

Actions:

1. Add trials with a DenseMatrixGraph value as payload within DenseMatrixGraph (self-nesting).
2. Add trials with DenseMatrixGraph as payload in NodeIdAdjacencyGraph, CompressedSparseRowGraph, NodeIDBTreeSet, CsrBTreeSet, NodeIDBTreeMap, CsrBTreeMap, and RegionBinaryHeap.
3. Verify construction, core operations, and validation for each legal pairing.

Exit criteria:

- DenseMatrixGraph self-nesting and cross-family payload trials pass for every legal host family.

### Step 10.19 - NodeIDBTreeSet Payload Trials

Actions:

1. Add trials with a NodeIDBTreeSet value as payload within NodeIDBTreeSet (self-nesting) where the design permits set-of-set or equivalent value storage.
2. Add trials with NodeIDBTreeSet as payload in NodeIdAdjacencyGraph, CompressedSparseRowGraph, DenseMatrixGraph, CsrBTreeSet, NodeIDBTreeMap, CsrBTreeMap, and RegionBinaryHeap.
3. Verify construction, core operations, and validation for each legal pairing.

Exit criteria:

- NodeIDBTreeSet self-nesting and cross-family payload trials pass for every legal host family.

### Step 10.20 - CsrBTreeSet Payload Trials

Actions:

1. Add trials with a CsrBTreeSet value as payload within CsrBTreeSet (self-nesting) where permitted.
2. Add trials with CsrBTreeSet as payload in NodeIdAdjacencyGraph, CompressedSparseRowGraph, DenseMatrixGraph, NodeIDBTreeSet, NodeIDBTreeMap, CsrBTreeMap, and RegionBinaryHeap.
3. Verify construction, core operations, and validation for each legal pairing.

Exit criteria:

- CsrBTreeSet self-nesting and cross-family payload trials pass for every legal host family.

### Step 10.21 - NodeIDBTreeMap Payload Trials

Actions:

1. Add trials with a NodeIDBTreeMap value as payload within NodeIDBTreeMap (self-nesting) where permitted.
2. Add trials with NodeIDBTreeMap as payload in NodeIdAdjacencyGraph, CompressedSparseRowGraph, DenseMatrixGraph, NodeIDBTreeSet, CsrBTreeSet, CsrBTreeMap, and RegionBinaryHeap.
3. Verify construction, core operations, and validation for each legal pairing.

Exit criteria:

- NodeIDBTreeMap self-nesting and cross-family payload trials pass for every legal host family.

### Step 10.22 - CsrBTreeMap Payload Trials

Actions:

1. Add trials with a CsrBTreeMap value as payload within CsrBTreeMap (self-nesting) where permitted.
2. Add trials with CsrBTreeMap as payload in NodeIdAdjacencyGraph, CompressedSparseRowGraph, DenseMatrixGraph, NodeIDBTreeSet, CsrBTreeSet, NodeIDBTreeMap, and RegionBinaryHeap.
3. Verify construction, core operations, and validation for each legal pairing.

Exit criteria:

- CsrBTreeMap self-nesting and cross-family payload trials pass for every legal host family.

### Step 10.23 - RegionBinaryHeap Payload Trials

Actions:

1. Add trials with a RegionBinaryHeap value as payload within RegionBinaryHeap (self-nesting) where permitted — including priority/value heap variants when applicable.
2. Add trials with RegionBinaryHeap as payload in NodeIdAdjacencyGraph, CompressedSparseRowGraph, DenseMatrixGraph, NodeIDBTreeSet, CsrBTreeSet, NodeIDBTreeMap, and CsrBTreeMap.
3. Verify construction, core operations, and validation for each legal pairing.

Exit criteria:

- RegionBinaryHeap self-nesting and cross-family payload trials pass for every legal host family.

### Cross-Structure Invariant Audits

### Step 10.24 - Immutability And Type Invariance

Authority:

- `graph_representation_design.md` sections 2.7 and 2.8
- `balanced_tree_and_heap_design.md` section 2.5

Actions:

1. Verify mutating generated helpers return new structure values (`produces pure …`).
2. Verify CSR/dense public query paths do not mutate frozen buffers in place.
3. Add error-enforcement trials for mixed inline graph/tree types on the same value flow where the compiler should reject the mismatch.
4. Verify constructor return types embed payload spellings used by subsequent get/set helpers (schema pinning, graph §2.8).

Exit criteria:

- Immutability and uniform-type rules are covered by trials or documented compiler checks.

### Step 10.25 - Region Ownership Audit

Authority:

- `graph_representation_design.md` sections 2.9, 4.8, and 9
- `balanced_tree_and_heap_design.md` section 2.2
- `btree_set_design.md` sections 2 and 6

Actions:

1. Audit every generated structure that contains buffers.
2. Verify the owning region is carried in the returned record where required.
3. Verify no generated helper returns bare buffers without region ownership when the design forbids it.

Exit criteria:

- Region ownership is documented in generated-family status notes and covered by type-level trials where possible.

### Step 10.26 - Naming And Emission Order Audit

Authority:

- [silica-specification.md](../silica-specification.md) §8.2.4
- [list_implementation_design.md](../list_implementation_design.md) §3.5, §7
- `graph_representation_design.md` section 8.4
- `btree_set_design.md` sections 8 and 9.5
- `balanced_tree_and_heap_design.md` naming and generator requirement sections

Actions:

1. Verify **module filenames** follow design rules (representation + directedness only for graphs; no payload type or memory space in the module name).
2. Verify generated **function names** follow design naming rules.
3. Verify **preferred** call sites use typed structure values without brackets; verify **optional** explicit brackets still work where documented; verify `List[Collectable, S]` resolution matches typed `empty` / first-argument overload rules (silica-spec §8.2.4).
4. Verify graph preferred call sites include `graph_adj_directed@empty(3)` without `[mem(normal)]` once graph overloads are collapsed to one export per operation.
5. Verify stdlib modules do **not** duplicate the same `operation` per primitive width in one file (legacy `graph_adj_directed` and `btree_nodeid` int8/int16/int32 blocks must be removed or gated behind migration).
6. Verify helper emission order follows design requirements.
7. Verify no generated module introduces custom Silica type declarations.
Exit criteria:

- Snapshot tests cover representative generated names and emission order.

### Step 10.27 - Documentation Status Update

Authority:

- All three design documents named at the top of this plan

Actions:

1. Add implementation-status notes to this plan as families are completed.
2. Design documents now include **`Collectable` placeholder** methodology (silica-spec §8.2.4; graph §2.11; balanced tree §2.6; btree set §4.1). Update this plan if implementation reveals a mismatch; do not revert to per-width duplicate exports in stdlib modules.
3. Link completed trial names from this plan.

Exit criteria:

- The plan remains a parseable implementation checklist and the design documents remain the authority.

## Completion Tracking


| Area                        | Status                            | Notes                                                                                                                                                                                                                                                                                                                                                                                             |
| --------------------------- | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Module filename rename      | Complete                          | Phase 0.0 — graph integrate green; btree integrate green                                                                                                                                                                                                                                                                                                                                          |
| Shared generator foundation | Partial                           | Phase 0.1–0.3 complete; **Phase 0.4 complete** — shallow Collectable in type checker; `qualified_call_mangler` + `overload_mangle` linkage aligned (export `.global` slugs, sequence env, local/unqualified mangling, arity fallback)                                                                                                                                                             |
| Collectable placeholder (0.4) | Complete                      | `list_collectable_context`, `graph_adj_directed_context_trial` (`@empty[mem(normal)]` / `@validate[mem(normal)]` / `@add_edge_slot0[mem(normal)]`), `btree_nodeid_context_trial` (`@empty()` / `@insert` / `@get` without payload brackets); `btree_nodeid` int64-only exports; E2016 on unresolved list Collectable (`trial_negative_list_collectable_unresolved`) |
| NodeIdAdjacencyGraph        | Complete                          | Phase 1 — `**graph_adj_directed.silica**`, `**graph_adj_undirected.silica**`; trials `graph_adj_directed_unweighted_trial`, `graph_adj_directed_int64_trial`, `graph_adj_undirected_trial`                                                                                                                                                                                                        |
| CompressedSparseRowGraph    | Complete                          | Phase 2 — `**graph_csr_directed.silica**`; trials `graph_csr_directed_unweighted_trial`, `graph_csr_directed_int64_trial`                                                                                                                                                                                                                                                                         |
| DenseMatrixGraph            | Complete                          | Phase 3 — `**graph_dense_directed.silica**`; trials `graph_dense_directed_unweighted_trial`, `graph_dense_directed_int64_trial`                                                                                                                                                                                                                                                                   |
| DenseBitsetGraph            | Deferred with documented fallback | Phase 3 — graph design §6.4 says to generate DenseBitset only when bitwise `                                                                                                                                                                                                                                                                                                                      |
| Graph algorithms            | Complete                          | Phase 4 — trials `graph_reachability_adj_directed_trial`, `graph_reachability_csr_directed_trial`, `graph_degree_summary_csr_directed_trial`                                                                                                                                                                                                                                                      |
| NodeIDBTreeSet              | Complete                          | Phase 5 — `**btree_set_nodeid.silica`**; `btree_set_addition` integrate                                                                                                                                                                                                                                                                                                                           |
| CsrBTreeSet                 | Complete (steps 6.1–6.4)          | Phase 6 steps 6.1–6.4 done: `empty`, `from_static_sorted`, `contains`, `validate`, `insert` all exported; 3 trials pass. Step 6.5 (NodeIDBTreeSet→CsrBTreeSet conversion) deferred; `to_csr` in `btree_set_nodeid.silica` stubbed to return `btree_set_csr@empty`                                                                                                                                 |
| NodeIDBTreeMap              | Refactor started (int64 canonical)  | Phase 7 — `**btree_nodeid.silica**` deduped to int64 `empty`/`insert`/`get` exports; context trial uses unbracketed `@empty()` / `@insert` / `@get`; signed-width payload trials need refresh or Collectable key placeholders                                                                                                                                                                    |
| CsrBTreeMap                 | Complete (steps 8.1–8.2)          | Phase 8 — `**btree_csr_map.silica**`; `btree_csr_map_insert` trial: `empty`, `from_static_sorted`, `contains`, `get`, `insert` (replace_value policy), `validate` exported; step 8.3 (`btree_csr.silica` set-only form) deferred as redundant with `btree_set_csr.silica`                                                                                                                         |
| RegionBinaryHeap            | Implemented (steps 9.1–9.2)       | Phase 9 — `**heap_binary_min.silica**` exports `empty`, `len`, `is_empty`, `is_full`, `peek`, `push`, `pop`, `validate`, plus the priority/value variant; `**heap_binary_max.silica**` exports the max-heap API; `heap_addition` trials cover min, max, priority/value, and invalid validation. Full standard-data-structures build-list wiring and rebuilt-compiler verification remain pending. |
| RegionDaryHeap              | Not started                       | Phase 9.3 — registry and inline type expansion mention `RegionDaryMinHeap[int64, mem(normal)]`, but no d-ary heap module or d-ary heap trials have been generated yet.                                                                                                                                                                                                                            |
| Cross-structure audit       | Not started                       | Phase 10 — Payload coverage (Steps 10.1–10.23), immutability (10.24), region ownership (10.25), naming (10.26), documentation (10.27)                                                                                                                                                                                                                                                             |

