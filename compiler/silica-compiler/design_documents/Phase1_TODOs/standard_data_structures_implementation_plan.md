# Standard Data Structures Implementation Plan

This plan organizes implementation work for Silica's standard generated data structures. It is an execution plan only. The design authority remains:

- [graph_representation_design.md](../graph_representation_design.md)
- [balanced_tree_and_heap_design.md](../balanced_tree_and_heap_design.md)
- [btree_set_design.md](../btree_set_design.md)

**Design conventions (shared model):** stored payload uses language `**Collectable**` (not a separate `Storable` trait). Graphs, trees, heaps, and sets are **immutable values** with **uniform inline record types** at every boundary (graph design §2.7–§2.8). CSR/dense topology buffers remain `**int64**`; `**NodeData**` / `**EdgeData**` / keys / values live in list slots or parallel `**Collectable**` buffers (graph §2.5–§2.6). **Design/registry names** use `**List`-aligned bracket syntax** — payload type(s) then `**mem(Space)**` at **function call sites** (graph §2.11; balanced tree §2.6; btree set §4.1). **Module filenames** use representation (+ directedness for graphs) only — for example `graph_adj_directed.silica`, `btree_set_nodeid.silica` — **not** `graph_adj_directed.silica`; emitted Silica still repeats full inline record types unless a compiler-known shorthand expands to them.

This document does not introduce new representations, memory-space rules, error shapes, or source-level syntax beyond what the design documents specify. **Naming** (bracket design names and operation instantiation) is defined in those design documents (graph §2.11, §8.4; balanced tree §2.6, §8; btree set §4.1, §8). When a detail is needed, use the relevant design document section named in the step.

This document intentionally contains no Silica source code. It is written as fine-grained work items that an LLM can follow when generating Silica code from the design documents.

## Implementation Order

1. **Module filename rename (blocking).** Rename all bootstrap generated modules and trials to design-document module names (graph §2.2, §8.4; btree set §8; balanced tree §8). **No other phase work until Step 0.0 exit criteria pass.**
2. Graph representations first.
3. B-tree set representations second.
4. General balanced B-tree representations third.
5. Heap representations fourth.
6. Cross-structure validation, docs, and trial cleanup last.

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
3. Update `**use**` declarations and `**module@operation**` call sites (short operation name after `@`; payload type and `**mem(Space)**` on function brackets only).
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
2. Emit full inline structural types everywhere a Silica type is required.
3. Use the memory-space rules from the relevant design document.
4. Use `**Collectable**` payload and operand rules exactly as described by the relevant design document (graph §2.4; balanced tree §2.4; btree set §4.0).
5. Keep generated naming consistent with the relevant design document: **module names** = representation family only; **exported function names** = short operation verbs (no module-prefix duplication); **call syntax** = `module@operation[brackets](args)`; bracket registry keys carry payload type and `**mem(Space)**`. The compiler emits **module-prefixed linker symbols** (for example `btree_set_csr_contains_int64_mem_normal_`_) so short export names can coexist when multiple modules link into one executable.
6. **Single-file `use` rule (E4011):** do not `use` two generated modules in one file when both export the same `operation[brackets]`. Cross-representation call sites use re-export wrappers on one module (for example `btree_set_nodeid@validate_csr`, `btree_set_nodeid@contains_csr`) or separate trial files.
7. Add positive trials before relying on a generated helper in later phases.
8. Add negative or validation trials for every documented invariant that can be checked.
9. Do not implement a faster packed form until the clear list-oriented form has validation coverage, unless the design document specifically allows direct packed construction.
10. Do not add unplanned APIs. If a helper is needed only internally, name and scope it as a generator helper and keep it aligned with the design document's helper requirements.
11. Emit **immutable** APIs by default: mutating operations return new structure values; use `_builder_` / `_mutable_` suffixes only when the design document allows (graph §2.7).
12. Enforce **uniform inline types** for each structure value flow (graph §2.7; list spec §4.2.4 analogy).

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
5. For each family, record only design-document fields: representation name, memory space, weightedness when applicable, directedness when applicable, `**node_data_type`** / `**edge_data_type**` / `**key_type**` when applicable (concrete `**Collectable**` inline spelling or `none`), and capacity constants when applicable.

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

- Steps 3.1–3.2: `graph_dense_directed.silica` (unweighted and weighted edge-payload via bracket instantiation at call sites); trials `graph_dense_directed_unweighted.silica`, `graph_dense_directed_weighted_int64.silica` (silica-compiler integrate).
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

### Step 10.1 - Shared Collectable Payload Coverage

Authority:

- `graph_representation_design.md` section 2.4
- `balanced_tree_and_heap_design.md` section 2.4
- `btree_set_design.md` section 4.0

Actions:

1. Verify all generated APIs use concrete `**Collectable**` payload types exactly where the design documents require them.
2. Verify structural metadata and topology indices remain plain `**int64**` or buffer types, not user `**Collectable**` payload.
3. Add compile-level checks or trials for the first monomorphic `int64` families and for at least one `**uint32**` (or other unsigned) `**Collectable**` list or buffer payload shape per silica-spec §8.2.4.

Exit criteria:

- All generated public APIs follow the documented `**Collectable**` payload rule.

### Step 10.2 - Immutability And Type Invariance

Authority:

- `graph_representation_design.md` sections 2.7 and 2.8
- `balanced_tree_and_heap_design.md` section 2.5

Actions:

1. Verify mutating generated helpers return new structure values (`produces pure …`) unless the module name includes `_builder_` or `_mutable_`.
2. Verify CSR/dense public query paths do not mutate frozen buffers in place.
3. Add error-enforcement trials for mixed inline graph/tree types on the same value flow where the compiler should reject the mismatch.
4. Verify constructor return types embed payload spellings used by subsequent get/set helpers (schema pinning, graph §2.8).

Exit criteria:

- Immutability and uniform-type rules are covered by trials or documented compiler checks.

### Step 10.3 - Region Ownership Audit

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

### Step 10.4 - Naming And Emission Order Audit

Authority:

- `graph_representation_design.md` section 8.4
- `btree_set_design.md` sections 8 and 9.5
- `balanced_tree_and_heap_design.md` naming and generator requirement sections

Actions:

1. Verify **module filenames** follow design rules (representation + directedness only for graphs; no payload type or memory space in the module name).
2. Verify generated **function names** follow design naming rules.
3. Verify bracket instantiation at call sites carries payload type and `**mem(Space)**` (like `**List**`).
4. Verify helper emission order follows design requirements.
5. Verify no generated module introduces custom Silica type declarations.

Exit criteria:

- Snapshot tests cover representative generated names and emission order.

### Step 10.5 - Documentation Status Update

Authority:

- All three design documents named at the top of this plan

Actions:

1. Add implementation-status notes to this plan as families are completed.
2. Do not change the design documents unless implementation reveals an actual design/document mismatch (Collectable payload model and invariance rules in graph §2.4–§2.8 are current authority).
3. Link completed trial names from this plan.

Exit criteria:

- The plan remains a parseable implementation checklist and the design documents remain the authority.

## Completion Tracking


| Area                        | Status                            | Notes                                                                                                                                                                                                                                                                     |
| --------------------------- | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Module filename rename      | Complete                          | Phase 0.0 — graph integrate green; btree integrate green                                                                                                                                                                                                                  |
| Shared generator foundation | Complete                          | Phase 0 — `src/standard_data_structures/`; `trials/standard_data_structures_addition/`; placeholder `*_addition/`; `error_enforcement_addition/generated_data_structures/`                                                                                                |
| NodeIdAdjacencyGraph        | Complete                          | Phase 1 — `**graph_adj_directed.silica**`, `**graph_adj_undirected.silica**`; trials `graph_adj_directed_unweighted_trial`, `graph_adj_directed_int64_trial`, `graph_adj_undirected_trial`                                                                                |
| CompressedSparseRowGraph    | Complete                          | Phase 2 — `**graph_csr_directed.silica**`; trials `graph_csr_directed_unweighted_trial`, `graph_csr_directed_int64_trial`                                                                                                                                                 |
| DenseMatrixGraph            | Complete                          | Phase 3 — `**graph_dense_directed.silica**`; trials `graph_dense_directed_unweighted_trial`, `graph_dense_directed_int64_trial`                                                                                                                                           |
| DenseBitsetGraph            | Deferred with documented fallback | Phase 3 — graph design §6.4 says to generate DenseBitset only when bitwise `                                                                                                                                                                                              |
| Graph algorithms            | Complete                          | Phase 4 — trials `graph_reachability_adj_directed_trial`, `graph_reachability_csr_directed_trial`, `graph_degree_summary_csr_directed_trial`                                                                                                                              |
| NodeIDBTreeSet              | Complete                          | Phase 5 — `**btree_set_nodeid.silica`**; `btree_set_addition` integrate                                                                                                                                                                                                   |
| CsrBTreeSet                 | Complete (steps 6.1–6.4)          | Phase 6 steps 6.1–6.4 done: `empty`, `from_static_sorted`, `contains`, `validate`, `insert` all exported; 3 trials pass. Step 6.5 (NodeIDBTreeSet→CsrBTreeSet conversion) deferred; `to_csr` in `btree_set_nodeid.silica` stubbed to return `btree_set_csr@empty`         |
| NodeIDBTreeMap              | Complete                          | Phase 7 — `**btree_nodeid.silica**` (`NodeIDBTreeMap` with `replace_value`); `btree_nodeid_addition` integrate (7 trials); `NodeIDBTreeSet` duplicate rejection unchanged in same module                                                                                    |
| CsrBTreeMap                 | Complete (steps 8.1–8.2)          | Phase 8 — `**btree_csr_map.silica**`; `btree_csr_map_insert` trial: `empty`, `from_static_sorted`, `contains`, `get`, `insert` (replace_value policy), `validate` exported; step 8.3 (`btree_csr.silica` set-only form) deferred as redundant with `btree_set_csr.silica` |
| RegionBinaryHeap            | Implemented (steps 9.1–9.2)       | Phase 9 — `**heap_binary_min.silica**` exports `empty`, `len`, `is_empty`, `is_full`, `peek`, `push`, `pop`, `validate`, plus the priority/value variant; `**heap_binary_max.silica**` exports the max-heap API; `heap_addition` trials cover min, max, priority/value, and invalid validation. Full standard-data-structures build-list wiring and rebuilt-compiler verification remain pending. |
| RegionDaryHeap              | Not started                       | Phase 9.3 — registry and inline type expansion mention `RegionDaryMinHeap[int64, mem(normal)]`, but no d-ary heap module or d-ary heap trials have been generated yet.                                                                                                     |
| Cross-structure audit       | Not started                       | Phase 10 — Collectable payload, immutability, type invariance (Steps 10.1–10.2)                                                                                                                                                                                           |

