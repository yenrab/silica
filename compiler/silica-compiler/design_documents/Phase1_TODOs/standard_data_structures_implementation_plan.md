# Standard Data Structures Implementation Plan

This plan organizes implementation work for Silica's standard generated data structures. It is an execution plan only. The design authority remains:

- [data_structures_as_traits.md](data_structures_as_traits.md) — Phase 1 standard-structure trait model, constructor function records, comparator result contract
- [silica-specification.md](../silica-specification.md) — §4.2.4 (lists), §8.2.4 (`Collectable`, **`List[Collectable, Space]`** placeholder)
- [list_implementation_design.md](../list_implementation_design.md) — §3, §7 (resolution and list primitives)
- [graph_representation_design.md](../graph_representation_design.md)
- [balanced_tree_and_heap_design.md](../balanced_tree_and_heap_design.md)
- [btree_set_design.md](../btree_set_design.md)

**Design conventions (shared model — trait-oriented methodology):**

- Graphs, trees, heaps, maps, and sets expose behavior through **standard traits** (`DirectedGraph`, `OrderedSet`, `OrderedMap`, `Heap`, etc.). Generated concrete representations implement those traits automatically.
- Generated constructors take **inline typed function records**. These records contain the comparators, extractors, and hash functions required by the representation. The compiler uses the function-field signatures as compile-time type witnesses.
- Collection variables still declare their collection type explicitly, including memory effect/type parameters such as `mem(normal)` or `mem(writethrough)`. Constructor function records check against that declaration; they do not hide or infer the collection type from return context alone.
- Comparator functions return **`atom`** with valid results `:less`, `:equal`, and `:greater`; standard-library behavior treats any other atom as invalid comparator behavior. Generated modules and trait signatures currently spell this as the sum type `:less | :equal | :greater` (stricter than bare `atom`); treat that as the accepted generated spelling until invalid-atom validation is implemented.
- Node ids, keys, values, priorities, edge payloads, and weights are **typed collection parameters**. They must not be assumed to be `int64`. Counts, capacities, dense physical slots, and generated buffer indices may remain `int64`.
- **One exported function per operation** per module remains the naming rule (`export insert/2`, not `export insert[int64, mem(normal)]/2` per width or user type), but specialization comes from concrete collection types and constructor function records, not from `Collectable` placeholders in standard-structure APIs.
- **Call sites (preferred):** constructors are called with function records (`btree_set_nodeid@empty({ compare_item: compare_string })`), and operations dispatch from typed structure values (`OrderedSet@contains(set, key)` for trait behavior or `btree_set_nodeid@insert(set, key)` for generated representation updates).
- **Migration dual API:** during Phase 0.5–Phase 10, bootstrap modules may continue to export width-specialized operations alongside trait dispatch. New acceptance trials and Phase 1+ steps use trait-oriented constructor records; old `*_addition` trials remain reference material until retargeted.
- **Module filenames** use representation only (`graph_adj_directed.silica`, `btree_nodeid.silica`). Do **not** suffix modules with payload type or memory space.
- **Compiler obligation:** check constructor record field signatures against the declared collection type, preserve captured functions in the generated representation value, and resolve trait operations from the concrete receiver type. See **Phase 0.4**.

**Clean rewrite note:** Existing bootstrap modules and their `.bak` copies are algorithm references only. They preserve useful insertion, validation, traversal, CSR, and heap algorithms, but new `.silica` work should be written directly against standard behavior traits plus typed constructor function records. The list-specific `List[Collectable, S]` placeholder support remains useful for lists; it is not the standard data-structure construction model.

This document does not introduce new representations or memory-space rules beyond the design documents. **Naming and call syntax** are defined in those documents (graph §2.11, §8.4; balanced tree §2.6, §8; btree set §4.1, §8; list §3.5). When a detail is needed, use the section named in the step.

This document intentionally contains no Silica source code. It is written as fine-grained work items that an LLM can follow when generating Silica code from the design documents.

## Implementation Order

1. **Trait-oriented constructor records and trait dispatch (compiler).** Phase 0.4 — **complete** for set/map/heap bracket types (see status below).
2. **Stdlib design reconciliation (pre-Phase 1 gate).** Phase 0.5 — fix known violations of captured-comparator and trait-semantics rules in existing trait modules and partially migrated generated families.
3. Graph representations first (Phase 1).
4. B-tree set representations second (Phase 3 in this plan).
5. General balanced B-tree and map representations third (Phase 4).
6. Heap representations fourth (Phase 5).
7. Cross-structure validation, docs, and trial cleanup last (Phase 10).

Graph comes first because the tree and heap designs explicitly reuse graph vocabulary: node ids, inline record shapes, memory-space rules, generated naming conventions, and region-backed buffers.

## Migration Staging Decisions

These decisions align execution with current code while `data_structures_as_traits.md` remains the long-term authority. Update this subsection if implementation reveals a mismatch.

| Topic | Staging choice (current) | Final design target (authority) |
| ----- | ------------------------ | ------------------------------- |
| Trait module shape | `required` blocks plus per-representation `impl fn` smoke impls in stdlib trait modules | `provided` blocks with `fold`-based default algorithms where the design specifies them (`OrderedSet`, `OrderedMap`) |
| `fold` / `find_value` / map `size` | Not exported on trait modules yet | Export and implement per `data_structures_as_traits.md` after first representation supplies `fold` |
| Comparator result type | Sum type `:less \| :equal \| :greater` in generated signatures | Design prose uses `atom`; invalid atoms deferred (open question in traits doc) |
| `get` / `peek` result shapes | `{ found: int64, value: T }` and bare `int64` peek in smoke impls | `{ found: boolean, value: T }` per traits doc — migrate when retargeting trials |
| Unweighted graph edge payload | **Option A — resolved (Step 1.1 action 6):** `EdgePayloadType = NodeIdType`; neighbor lists store destination ids; `add_edge(g, from, to)`; `edge_target` is identity on the payload | Weighted/attributed graphs use explicit neighbor records (`{ to, weight }` / `{ to, data }`) per `graph_representation_design.md` §3.6 |
| Module vs trait API | Both coexist during migration | Trait acceptance is the exit criterion; module exports retired per family as trials retarget |
| Phase 0.4 graph/heaps | `graph_phase04`, `heap_phase04`, `btree_set_phase04` toy modules plus minimal trait impls | Replace with real generated families wired through standard traits |

## Global Rules For All Steps

Every implementation step must obey these rules:

1. Treat design names as generator names only.
2. In **generated standard data-structure** sources, collection element/key/value/node/edge/priority types are witnessed by constructor function records and then stored as concrete inline structural types in the generated value. `Collectable` placeholders are list/compiler machinery, not the public construction rule for standard structures.
3. Use the memory-space rules from the relevant design document.
4. Use payload and operand rules exactly as described by the relevant design document (graph §2.4; balanced tree §2.4; btree set §4.0).
5. Keep generated naming consistent with the relevant design document: **module names** = representation family only; **exported function names** = short operation verbs with **arity-only exports** (`export contains/2`); constructors receive inline function records; behavior operations are available through standard traits and dispatch from the receiver's concrete generated representation.
6. **Single-file `use` rule (E4011):** do not `use` two generated modules in one file when both export the same `operation/arity` and overload resolution is ambiguous. Cross-representation call sites use re-export wrappers or separate trial files.
7. Do **not** add another copy of the same public operation for each primitive width or user struct spelling in one module. Use constructor function records and generated impl specialization instead.
8. Add positive trials before relying on a generated helper in later phases.
9. Add negative or validation trials for every documented invariant that can be checked.
10. Do not implement a faster packed form until the clear list-oriented form has validation coverage, unless the design document specifically allows direct packed construction.
11. Do not add unplanned APIs. If a helper is needed only internally, name and scope it as a generator helper and keep it aligned with the design document's helper requirements.
12. Emit **immutable** APIs by default: mutating operations return new structure values; use `_builder`_ / `_mutable_` suffixes only when the design document allows (graph §2.7).
13. Enforce **uniform inline types** for each structure value flow after constructor-record checking (graph §2.7; list spec §4.2.4 / §8.2.4).

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

### Step 0.4 - Trait-Oriented Constructor Records And Trait Dispatch (blocking for final methodology)

Authority:

- [data_structures_as_traits.md](data_structures_as_traits.md)
- [silica-specification.md](../silica-specification.md) §4.2.4, §8.2.4
- [list_implementation_design.md](../list_implementation_design.md) §3.5, §7
- [graph_representation_design.md](../graph_representation_design.md) §2.4, §2.8, §2.11
- [balanced_tree_and_heap_design.md](../balanced_tree_and_heap_design.md) §2.4, §2.6
- [btree_set_design.md](../btree_set_design.md) §4.0, §4.1

Actions:

1. Define standard behavior traits for graph, set, map, tree, heap, and priority-queue operations named in `data_structures_as_traits.md`.
2. Add type-checking for inline constructor function records. Each required field must exist and have the exact function type required by the declared collection type, for example `compare_item: fn(ItemType, ItemType) -> atom`.
3. Treat constructor function records as structural inline values only. Do not introduce named or aliased struct types for these records.
4. Preserve captured comparator, extractor, and hashing functions in generated representation values so update operations (`insert`, `push`, `add_edge`, etc.) reuse the construction-time functions.
5. Add generated impls that connect concrete representations to standard behavior traits. Trait calls dispatch from the receiver-like first argument (`OrderedSet@contains(set, key)`, `DirectedGraph@neighbors(g, node)`).
6. Validate comparator result handling for the atom contract (`:less`, `:equal`, `:greater`); add negative or runtime-validation trials for invalid atoms once the standard-library error path is selected.
7. Retain the already-completed list `Collectable` placeholder trials as compiler/list coverage, but stop using them as the acceptance criterion for standard data-structure constructors.

Exit criteria:

- Set, map, graph, and heap constructors accept inline typed function records and reject records whose function-field signatures disagree with the declared collection type.
- Generated representation values preserve construction-time functions across update operations.
- At least one generated set/map, one graph, and one heap representation implements and passes calls through its corresponding standard behavior trait.
- Comparator functions returning atoms outside `:less`, `:equal`, and `:greater` are covered by validation behavior or by a documented deferred stricter check.
- Existing list `Collectable` placeholder tests remain green as list-specific coverage.

**Phase 0.4 status — compiler (complete for set/map/heap):**

| # | Work item | Status | Notes |
| - | --------- | ------ | ----- |
| 1 | First-argument trait dispatch | Done | Smoke trials in `trials/standard_data_structures_phase04_addition/` |
| 2 | Placeholder matching for trait required/provided signatures (E2092) | Done | `type_checker_trait_placeholders.silica`; module-level assoc placeholders still permissive |
| 3 | Constructor function-record resolution and witnesses (E2017) | Done | `type_checker_collections.silica` |
| 4 | Generic assoc-type witnesses (`ItemType`, `KeyType`, `ValueType`) | Done | Bracket types `OrderedSet[T, mem(S)]`, `OrderedMap[K, V, mem(S)]`, `Heap[T, mem(S)]` |
| 5 | Link-name mangling from resolved concrete trait impl types | Done | `recv_type_for_trait_link_mangle` in SIR mangler paths |

**Key compiler modules:** `type_checker/type_checker_collections.silica`, `type_checker/traits/type_checker_trait_placeholders.silica`, wiring in `type_checker/expressions/type_checker_expressions.silica`, `trait_checker/trait_checker_core.silica`, `sir_generator/declarations/overload_mangle.silica`, `sir_generator/declarations/qualified_call_mangler.silica`.

**Phase 0.4 status — stdlib smoke (partial):**

| Area | Status | Notes |
| ---- | ------ | ----- |
| Trait module definitions | Partial | All nine standard trait names exist under `src/standard_data_structures/`; shape uses staging `required` + `impl fn`, not final `provided` + `fold` |
| `OrderedSet` / `OrderedMap` trait impls | Partial | Wired through adapter modules to `btree_set_nodeid`, `btree_set_csr`, `btree_nodeid`, `btree_csr_map`; Phase 0.5 comparator/size/CSR fixes applied |
| `DirectedGraph` / undirected / weighted | Partial | `graph_adj_directed@empty/2` + trait impls on adjacency record (Step 1.1); `graph_phase04` toy still in `silica.config.phase04_traits` until Step 1.4 retires it |
| `Heap` | Smoke only | `heap_phase04` toy record; not `heap_binary_min` |
| Stdlib batch build | Done | `src/standard_data_structures/silica.config.phase04_traits` includes btree backends, adapters, and trait modules; `make` in that directory succeeds |
| Phase 0.4 trials | Done | `trials/standard_data_structures_phase04_addition/` integrate; `trials/error_enforcement_addition/standard_data_structures_phase04/` negative trials |
| Graph bracket-type witnesses | Done | `DirectedGraph[Node, Edge, mem(S)]` in `type_checker_collections.silica`; compile witness: `directed_graph_witness_int64.silica` (Step 1.1) |
| Provided-block bodies / default impl checking | Not started | Deferred until `fold` migration |
| Invalid comparator atom validation | Deferred | Documented in Migration Staging Decisions |

**Remaining Phase 0.4 exit criteria (not yet met):**

- Graph **generated** family accepts function records and passes trait calls on empty graph (Step 1.1 — **complete** for directed unweighted adjacency empty path; update/edge paths remain Phase 1.3+).
- Heap **generated** families (not phase04 toys) accept function records and pass trait calls.
- CSR set/map generated values **preserve** captured comparators in the value (Phase 0.5 — **complete**).
- Comparator invalid-atom path selected and trialed.

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

- The trial harness can run reserved suites for each data-structure family.

**Phase 0 completed (trial layout):**

- `trials/standard_data_structures_addition/` — type-string snapshot (`.scout` / `.ascomp`)
- `trials/standard_data_structures_phase04_addition/` — trait dispatch, constructor-record witnesses, link-name mangling (integrate target for new-design smoke)
- `trials/error_enforcement_addition/standard_data_structures_phase04/` — E2017, E2092, E2003 negative trials
- `trials/graph_addition/`, `btree_set_addition/`, `balanced_tree_addition/`, `heap_addition/` — old-design reference; integrate via placeholder or legacy suites until retargeted
- `trials/error_enforcement_addition/generated_data_structures/` — validation-failure naming and subdirs (`graph/`, `btree_set/`, `balanced_tree/`, `heap/`); goldens added when `validate` exists

## Phase 0.5 - Stdlib Design Reconciliation (pre-Phase 1 gate)

Authority:

- [data_structures_as_traits.md](data_structures_as_traits.md) — constructor function record rule, captured comparators, trait semantics
- Migration Staging Decisions (above)

Purpose:

Fix **known violations** in existing trait modules and partially migrated generated families before Phase 1 graph migration copies the wrong patterns. This phase does **not** migrate traits to `provided` + `fold`; that remains a later step after the first representation implements `fold`.

### Step 0.5.1 - Fix Captured Comparator Usage In Trait Impls

Actions:

1. `OrderedMap` nodeid shape: `compare_value` must delegate to `map.compare_value`, not hardcoded numeric comparison.
2. `OrderedSet` CSR shape: `compare_item` must delegate to a captured comparator; CSR record type in trait impl must include `compare_item` field consistent with generated value.
3. Add or extend phase04 trials where a non-default comparator would fail under the buggy impl (map value compare, CSR set compare).

Exit criteria:

- Trait-layer comparator calls use captured functions for all wired set/map shapes.
- Trials compile and would fail on the pre-fix impl behavior.

### Step 0.5.2 - Fix Trait Semantics Bugs

Actions:

1. `OrderedSet` nodeid `size`: return item/key count, not B-tree `node_count`.
2. Confirm CSR set `size` uses `key_count_total` (already correct in current impl).
3. Document `{ found: int64 }` vs `{ found: boolean }` as staging debt; do not block Phase 1 on boolean migration unless a trial requires it.

Exit criteria:

- `OrderedSet@size` on nodeid btree matches item count after insert trial.
- Size semantics documented in Migration Staging Decisions if boolean/`found` shapes remain deferred.

### Step 0.5.3 - Preserve Comparators In CSR Generated Values

Actions:

1. `btree_set_csr@empty`: store `compare_item` in returned set record; thread through insert/update paths.
2. `btree_csr_map@empty`: store `compare_key` and `compare_value` in returned map record; thread through update paths.
3. Update inline type expansion / registry entries if CSR record shapes change.
4. Align `OrderedSet` / `OrderedMap` CSR trait impl record types with updated generated shapes.

Exit criteria:

- CSR empty constructors preserve function fields per design rule #4.
- Witness trials for non-`int64` item/key types can reach CSR representations without silent fallback to built-in ordering.

### Step 0.5.4 - Document Staging Vs Authority

Actions:

1. Keep this plan's Migration Staging Decisions table current.
2. When Phase 0.5 completes, update Completion Tracking rows for trait modules and CSR families.

Exit criteria:

- No open Phase 0.5 item remains without either a fix or an explicit deferral note in Completion Tracking.

## Phase 1 - Graph Foundation: NodeIdAdjacencyGraph

### Step 1.1 - Generate Unweighted Directed Adjacency Type And Empty Constructor

Authority:

- `graph_representation_design.md` sections 3.1, 3.2, 3.4, and 3.5
- [data_structures_as_traits.md](data_structures_as_traits.md) — graph constructor function record (`compare_node`, `compare_edge`, `edge_target`)

Actions:

1. Generate the directed unweighted adjacency graph family for the default concrete memory-space case used by the tests.
2. Emit `empty/2` taking an inline graph function record plus `node_count`, per traits doc (replace bootstrap `empty(initial_node_count)` without function record).
3. Preserve `compare_node`, `compare_edge`, and `edge_target` in the graph value across updates.
4. Generate node records with empty neighbor lists.
5. Preserve `node_count` and `edge_count` fields as described.
6. Resolve unweighted edge payload shape (bare `NodeId` vs `{ to: NodeIdType }`) per Migration Staging Decisions before edge-addition trials depend on it.
7. Add compiler bracket-type witness checking for `DirectedGraph[NodeIdType, EdgePayloadType, mem(SpaceType)]` (extend collection machinery beyond set/map/heap).
8. Add `DirectedGraph` trait `impl fn` for the adjacency record shape (`node_count`, `edge_count`, `neighbors`, `edge_target` as required ops per traits doc — minimum smoke: `node_count`, `edge_count`, `neighbors` on empty graph).

Exit criteria:

- A trial constructs an empty directed unweighted adjacency graph via function record constructor.
- A trial verifies node count and edge count through generated inspection helpers or `DirectedGraph@node_count` / `DirectedGraph@edge_count`.
- Declared type `DirectedGraph[T, Edge, mem(normal)]` witnesses against constructor function-record field types.

**Step 1.1 coverage — unique vs elsewhere in this plan:**

| Item | Also covered elsewhere? | Notes |
| ---- | ----------------------- | ----- |
| Action 1 — directed unweighted adjacency family (trait-oriented record shape) | Partially | Step 0.1–0.2 registry/type expansion name the family but omit captured function fields; Step 1.5 adds undirected variant; Step 10.26 audits width-export retirement. **Only 1.1** introduces the trait-oriented directed unweighted empty family. |
| Action 2 — `empty/2` with graph function record | Partially | Phase 0.4 defines the general constructor-record rule. **Only 1.1** emits graph `empty/2`. |
| Action 3 — preserve comparators **across updates** | Yes | Phase 0.4 action 4 (general rule); Step 1.3+ threads through `add_edge` and later ops. At **empty** construction, 1.1 only. |
| Action 4 — node records with empty neighbor lists | No | Initial empty construction only. |
| Action 5 — `node_count` / `edge_count` fields in the value | Partially | Step 1.4 generates inspection helpers and trait wiring for the same fields on non-empty graphs. |
| Action 6 — resolve unweighted edge payload shape | No | **Resolved:** Option A (`EdgePayloadType = NodeIdType`). Recorded in Migration Staging Decisions. |
| Action 7 — `DirectedGraph[…]` bracket-type witnesses | Yes | Phase 0.4 exit criteria and `type_checker_collections.silica`; 1.1 adds the graph compile witness trial. |
| Action 8 — minimum `DirectedGraph` trait impl (empty smoke) | Partially | Step 1.4 extends trait wiring (`has_edge`, `out_degree`, `edge_target` usage, retire `graph_phase04`). |
| Exit — empty constructor + count trials | Partially | Step 1.4 adds post-edge inspection trials; empty-only acceptance is **1.1**. |
| Exit — bracket-type witness | Yes | Phase 0.4 (same machinery as set/map/heap witnesses). |

**Step 1.1 status — not covered elsewhere (still open):**

- **Action 1 (remainder)** — align `inline_type_expansion.silica` with the runtime record (`compare_node`, `compare_edge`, `edge_target` fields); no dedicated later step names this for graphs (CSR set/map analogue: Step 0.5.3 action 3).
- **Bootstrap coexistence** — old width-specialized exports remain beside the new API until Step 10.26 (naming/duplication audit) or explicit retirement in Step 1.4 action 7 (`graph_phase04`).

**Step 1.1 status — complete:**

| # | Work item | Status | Notes |
| - | --------- | ------ | ----- |
| 1 | Action 2 — `graph_adj_directed@empty/2` with function record | **Complete** | `build_empty_adj_unweighted_graph` in `graph_adj_directed.silica` |
| 2 | Action 4 — empty node records | **Complete** | via `graph_adj_list_helpers@graph_adj_build_empty_nodes_unweighted` |
| 3 | Action 5 — `node_count` / `edge_count` initialized | **Complete** | empty graph value |
| 4 | Action 3 (empty) — captured `compare_node`, `compare_edge`, `edge_target` stored | **Complete** | preserved in empty graph record |
| 5 | Action 7 — `DirectedGraph[T, Edge, mem(S)]` bracket witnesses | **Complete** | `type_checker_collections.silica`; trial `directed_graph_witness_int64.silica` (compile witness) |
| 6 | Action 8 — minimum trait impl on adjacency record | **Complete** | `DirectedGraph.silica`: `@node_count`, `@edge_count`, `@neighbors` on empty graph |
| 7 | Exit — empty constructor trial | **Complete** | `directed_graph_adj_empty_trait.silica` |
| 8 | Exit — count verification via trait | **Complete** | same trial; also `directed_graph_trait.silica` |
| 9 | Exit — declared-type witness | **Complete** | `directed_graph_witness_int64.silica` |
| 10 | Action 6 — unweighted edge payload shape (Option A) | **Complete** | `EdgePayloadType = NodeIdType`; Migration Staging Decisions updated |

**Step 1.1 exit criterion: met** for the empty directed unweighted adjacency path. Remaining 1.1-open items above do not block later Phase 1 steps.

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
- [data_structures_as_traits.md](data_structures_as_traits.md) — `DirectedGraph` required and provided operations

Actions:

1. Generate `node_count`.
2. Generate `edge_count`.
3. Generate `out_degree`.
4. Generate `has_edge`.
5. Generate neighbor traversal helpers according to the traversal strategy.
6. Wire `DirectedGraph` trait impls for adjacency record (required: `neighbors`, `edge_target`; provided-style ops such as `has_edge`, `out_degree` may live as trait impls or generated helpers called from trait).
7. Add phase04-style trait trial: `graph_adj_directed@empty({...}, n)` then `DirectedGraph@neighbors` / `DirectedGraph@has_edge` (retire dependence on `graph_phase04` for acceptance).

Exit criteria:

- Trials cover present edge, absent edge, out degree, and neighbor traversal.
- At least one trial uses trait dispatch on the real adjacency representation, not `graph_phase04`.

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

**Status: complete** — `inline_type_expansion.silica` exports `expand_csr_directed_unweighted/1` and `expand_csr_directed_weighted_int64/1`; unweighted and weighted shapes include `region(R, S)`, `offsets: buf(R, S, int64, N_PLUS_ONE)`, `neighbors: buf(R, S, int64, M)`, and weighted adds `weights: buf(R, S, int64, M)` per graph_representation_design.md §4.2–§4.3. Snapshot trial: `graph_csr_type_expansion_snapshot.silica` (also covered in `type_expansion_snapshot.silica` indices 4–7).

Authority:

- `graph_representation_design.md` sections 4.1, 4.2, 4.3, 4.8, and 8.5

Actions:

1. Generate CSR inline type strings for unweighted and weighted forms.
2. Include the owning region exactly as required by the design.
3. Include concrete buffer capacities exactly as required by the design.

Exit criteria:

- Snapshot tests cover CSR type strings for unweighted and weighted forms.

### Step 2.2 - Generate CSR Direct Static Constructor

**Status: complete** — `graph_csr_directed@from_static_edges[mem(normal)]` and `from_static_edges[int64, mem(normal)]` allocate region/buffers, fill offsets/neighbors/(weights), return full graph record with region ownership per §4.6. Trial: `graph_csr_static_constructor_trial.silica`.

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

**Status: complete** — `validate[mem(normal)]` and `validate[int64, mem(normal)]` check offsets (first/final endpoints, monotonicity), neighbor endpoint ranges, and weighted buffer shape per §4.4. `validate_checked` exports error codes for negative trials. Trials: `graph_csr_validate_valid.silica`, `graph_csr_validate_invalid.silica` (non-monotonic offsets → error 4).

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

**Status: complete** — `out_degree`, `neighbor_at` (offset-range traversal), `has_edge` (linear scan per §4.7 unsorted default), and `weight_at`/`weight_at_checked` for weighted CSR. Trial: `graph_csr_inspection_trial.silica`.

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

**Status: complete** — `freeze[mem(normal)]` and `freeze[int64, mem(normal)]` build CSR offsets and neighbor/weight buffers from flat bootstrap adjacency graphs (§4.5). Trial: `graph_csr_adj_finalize_trial.silica`.

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

**Status: complete** — `graph_dense_directed.silica` provides unweighted and weighted inline shapes, `empty/1`, and `set_edge` with `index = from * node_count + to` (§5.5). Trials: `graph_dense_type_expansion_snapshot.silica`, `graph_dense_constructor_trial.silica`.

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

**Status: complete** — `has_edge`, `neighbor_at`, `out_degree`, `weight_at`, and `validate` in `graph_dense_directed.silica` (§5.4, §5.6). Trials: `graph_dense_inspection_trial.silica`, `graph_dense_validate_invalid.silica`, plus existing unweighted/int64 trials.

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

**Status: complete for Phase 1 directed unweighted graphs** — required bitwise operations (`bor`, `band`, `bnot`, `shl`, `shr`) are available for `uint64`, and `graph_dense_bitset_directed.silica` now provides the generated-capacity `DenseBitsetGraphDirected[mem(normal)]` path. Trials: `graph_dense_bitset_type_expansion_snapshot.silica`, `graph_dense_bitset_constructor_trial.silica`, `graph_dense_bitset_inspection_trial.silica`, `graph_dense_bitset_validate_invalid.silica`, plus negative enforcement under `error_enforcement_addition/generated_data_structures/graph/`.

Authority:

- `graph_representation_design.md` sections 6.1 through 6.4 and 9
- `bitwise_operators_implementation_plan.md` for the required `bor`, `band`, `bnot`, `shl`, and `shr` compiler support.

Actions:

1. Check whether the current compiler path supports the bit operations required by the design. Complete: Phase 1 supports `uint64` bit operations.
2. Generate dense bitset type, set, clear, and edge-test helpers. Complete for `DenseBitsetGraphDirected[mem(normal)]`.
3. Record fallback behavior for unsupported variants. Complete: weighted DenseBitsetGraph construction remains rejected; use dense matrix for weighted dense graphs.

Exit criteria:

- Dense bitset trials pass for supported unweighted directed graphs, and unsupported variants have negative enforcement coverage.

**Phase 3 completed:**

- Steps 3.1–3.2: `graph_dense_directed.silica` (bootstrap unweighted and weighted edge-payload via typed graph values); trials `graph_dense_directed_unweighted.silica`, `graph_dense_directed_weighted_int64.silica` (silica-compiler integrate).
- Step 3.3: `graph_dense_bitset_directed.silica` implements directed unweighted dense bitsets using `uint64` words, generated capacity `WORD_COUNT = 4`, and operations for set, clear, edge-test, degree, neighbor lookup, and validation.
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

- Step 4.1: `reachable/3` on adjacency flat slots and CSR generated-capacity graphs. Coverage includes reflexive, direct, unreachable, and multi-hop pairs, including a non-canonical hand-built CSR graph. Trials: `graph_reachability_adj_directed_trial.silica`, `graph_reachability_csr_directed_trial.silica`.
- Step 4.2: `max_out_degree/1`, `total_out_degree_sum/1` on CSR and adjacency graphs. Trials: `graph_degree_summary_csr_directed_trial.silica`, `graph_degree_summary_adj_directed_trial.silica`.

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

**Algorithm reference available:**

- The `.bak` copy of `btree_set_nodeid.silica` preserves the order-8 immutable list-backed insertion, membership, and validation algorithms.
- Delete remains deferred per `btree_set_design.md` sections 5.6 and 7.3.
- The clean `.silica` rewrite should expose constructor-record and trait-oriented APIs rather than the bootstrap `empty/0`, `contains/2`, `insert/2`, and `validate/1` surface.

**Phase 5 completed:**

- Steps 5.1-5.3: `btree_set_nodeid.silica` provides constructor-record empty construction, generated shape membership, size, and validation coverage for empty, valid hand-built, and invalid trees.
- Step 5.4: `insert[int64, mem(normal)]` accumulates keys functionally, preserves captured comparators, reports duplicate insertion status, and splits from one leaf into a two-leaf root shape at order-8 capacity. Trials: `btree_set_nodeid_insert.silica`, `btree_set_nodeid_insert_split.silica`.
- Step 5.5: delete remains explicitly deferred per `btree_set_design.md` sections 5.6 and 7.3.

## Phase 6 - B-tree Set: CsrBTreeSet

**Status (steps 6.1–6.4 plus NodeID finalization bridge): complete** — `btree_set_csr.silica` now exports `empty`, `from_static_sorted`, `contains`, `validate`, and `insert` for the generated order-8 leaf form and first split-root form (`NODE_CAP=3`, `KEY_CAP=8`, `CHILD_CAP=2`). `btree_set_nodeid@to_csr` finalizes generated one-leaf and first-split NodeID trees into CSR. Trials pass: `btree_set_csr_contains_static`, `btree_set_csr_validate_invalid`, `btree_set_csr_insert`, and `btree_set_nodeid_to_csr`.

Design note: `CsrBTreeSet` has been revised from its original "immutable after construction / no direct insert" spec. It now follows the same **functional-programming design** as `NodeIDBTreeSet` and Silica's `List`: `insert` returns a new value without modifying the caller's existing tree. See `btree_set_design.md` §6.7 (updated).

### Step 6.1 - Generate CsrBTreeSet Type Expansion

**Status: complete** — inline structural record type with region, capacity constants, and buffer fields is defined in `btree_set_csr.silica`; the generated Phase 1 capacity now covers the leaf and first split-root CSR shapes.

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

**Status: complete** — `contains` and `validate` exported; generalized to handle leaf keys and the first split-root shape. `btree_set_csr_contains_static`, `btree_set_csr_validate_invalid`, and `btree_set_nodeid_to_csr` trials pass.

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

**Status: complete** — `insert[int64, mem(normal)]` exported. Helpers include generated leaf insertion plus first-split construction when inserting the eighth distinct key. `btree_set_csr_insert` passes for direct functional insert/duplicate/immutability coverage; `btree_set_nodeid_to_csr` covers split CSR finalization, validation, and membership.

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
- A finalization trial converts a generated split NodeIDBTreeSet into split CSR and verifies metadata, validation, present-key lookup, and absent-key lookup.

## Phase 7 - General B-tree: NodeIDBTreeMap

Algorithm reference exists in `btree_nodeid.silica.bak`, reusing the NodeIDBTreeSet node layout and traversal while applying `replace_value` duplicate-key policy. The clean rewrite must expose the map through constructor records and `OrderedMap` behavior.

**Status (steps 7.1–7.3 plus trait exports): complete** — `btree_nodeid@empty({ compare_key, compare_value })` stores both comparators; `get`, `insert`, and `validate` cover generated leaf and first-split shapes; `to_csr` finalizes to CSR (Phase 8.3). `OrderedMap` trait exports `size`, `find_value`, and `fold` delegate through nodeid and CSR adapters. Trials: `ordered_map_nodeid_size_trait`, `ordered_map_nodeid_find_value_trait`, `ordered_map_nodeid_fold_trait`, plus existing `ordered_map_compare_value_trait` and `btree_nodeid_map_to_csr`.

### Step 7.1 - Generate NodeIDBTree Type Expansion

Authority:

- `balanced_tree_and_heap_design.md` sections 3 and 4

Actions:

1. Generate the general NodeIDBTreeMap type string (keys + values per node).
2. Include keys and values exactly as specified by the design.
3. Preserve constructor-record fields needed for `compare_key` and `compare_value`.

Exit criteria:

- Type string stable in generated module.

### Step 7.2 - Generate NodeIDBTree Search And Validation

Authority:

- `balanced_tree_and_heap_design.md` B-tree invariant and NodeIDBTree sections

Actions:

1. Generate search by key (`get`).
2. Generate value lookup returning `{found, value}`.
3. Generate invariant validation (reuses set validation with value-shape checks).
4. Preserve `replace_value` map policy while keeping set `reject_duplicates` separate.

Exit criteria:

- Trials cover present keys, absent keys, and invalid trees.

### Step 7.3 - Generate NodeIDBTree Insert

Authority:

- `balanced_tree_and_heap_design.md` NodeIDBTree insert and split sections

Actions:

1. Generate insertion with `replace_value` policy (`insert` returns `{tree, inserted, replaced}`).
2. Reuse node split helpers from NodeIDBTreeSet.
3. Preserve B-tree invariants.
4. Return the documented result shape.

Exit criteria:

- Trials cover insert, replace, get, and immutability.

## Phase 8 - General B-tree: CsrBTree

Algorithm reference exists in `btree_csr_map.silica.bak` for `CsrBTreeMap[int64, int64, mem(normal)]`.

**Status (steps 8.1–8.5): complete** — `btree_csr_map.silica` exports `empty`, `empty/1` (constructor function record), `from_static_sorted`, `contains`, `contains_key`, `get`, functional `insert` with `replace_value` policy, and `validate` for the generated order-8 leaf and first split-root shapes (`NODE_CAP=3`, `KEY_CAP=8`, `CHILD_CAP=2`). Captured `compare_key` and `compare_value` are preserved across insert/replace paths. `btree_nodeid@to_csr` finalizes generated NodeID leaf/split maps into CSR. Trials pass: `btree_csr_map_contains_static`, `btree_csr_map_validate_invalid`, `btree_csr_map_insert`, `btree_csr_map_insert_split`, `btree_nodeid_map_to_csr`, and `ordered_map_csr_compare_value_trait`.

### Step 8.1 - Generate CsrBTreeMap Construction

**Status: complete** — type string matches design §5.3; `empty` (`root_id=-1`), `from_static_sorted`, and `build_leaf_csr_map_skeleton` / insert/replace builders allocate fresh regions with key and value buffers.

Authority:

- `balanced_tree_and_heap_design.md` §5.3, §5.5, §5.8

Actions:

1. Generate CSR map type string: `{region, root_id, node_count, key_count_total, order, node_key_start, node_key_count, node_child_start, node_child_count, node_is_leaf, keys, values, children}`.
2. Generate `empty` (root_id=-1) and `from_static_sorted`.
3. Generate `build_leaf_csr_map` internal builder allocating a fresh region with key and value buffers.
4. Include capacity constants `node_cap`, `key_cap`, and `child_cap`.

Exit criteria:

- A trial constructs a map via sequential insert and validates it.

### Step 8.2 - Generate CsrBTreeMap Search, Insert, And Validation

**Status: complete** — `contains` reuses linear `contains_key_at`; `get` uses `find_key_pos`; `insert` returns `{ tree, inserted, replaced }` with replace-value semantics; `validate` checks order, key-count bounds, and sorted-key invariant (error code 5 for unsorted keys).

Authority:

- `balanced_tree_and_heap_design.md` §5.7, §5.8, §5.9, §9.2, §9.4

Actions:

1. Generate `contains` (linear key scan over `keys` buffer — reuses `contains_key_at` from `CsrBTreeSet`).
2. Generate `get` (key lookup returning `{ found: int64, value: ValueType }` — uses `find_key_pos` helper).
3. Generate functional `insert` with `replace_value` duplicate policy, returning `{ tree, inserted, replaced }`.
4. Generate `validate` (order check, key-count bounds, sorted-key invariant).

Exit criteria:

- Trial covers: new-key insert, replace-value insert, get found and not-found, validation pass, and immutability of original map.

### Step 8.3 - Generate NodeIDBTreeMap To CsrBTreeMap Finalization

**Status: complete** — `btree_nodeid@to_csr[int64, int64, mem(normal)]` finalizes generated one-leaf and first-split NodeID maps into CSR via structural leaf copy or sequential CSR insert with value lookup. Helpers `validate_csr_error`, `contains_csr`, and `get_csr` delegate to `btree_csr_map`. Trial: `btree_nodeid_map_to_csr`.

Authority:

- `balanced_tree_and_heap_design.md` §5.6

Actions:

1. Generate `btree_nodeid_map_to_csr[Key, Value, mem(Space)]` (exported as `to_csr`).
2. Validate source NodeIDBTreeMap shape (node_count 1 or 3).
3. Allocate CSR map buffers and copy keys/values or rebuild via functional insert.
4. Preserve captured comparators in the CSR result.

Exit criteria:

- A trial builds a split NodeIDBTreeMap, finalizes to CSR, and verifies metadata, validation, membership, and value lookup.

### Step 8.4 - Generate CsrBTreeMap First Split-Root Growth

**Status: complete** — direct CSR map `insert` grows a full leaf into the first split-root shape when the eighth distinct key is inserted (mirrors Phase 6 CsrBTreeSet). Split-root `contains`, `get`, and `validate` paths handle three-node topology. Trial: `btree_csr_map_insert_split`.

Authority:

- `balanced_tree_and_heap_design.md` §5.8
- Phase 6 CsrBTreeSet split-root reference (`btree_set_csr.silica`)

Actions:

1. Upgrade capacities to `NODE_CAP=3`, `KEY_CAP=8`, `CHILD_CAP=2`.
2. Generate `build_split_csr_map_from_insert` with parallel key and value buffer shifts.
3. Extend `insert_nonempty_csr_map` to split at leaf capacity and handle split-root replace-value updates.
4. Extend search and validation for three-node split-root topology.

Exit criteria:

- Trial inserts eight distinct key-value pairs directly into CSR map and verifies split-root metadata and validation.

### Step 8.5 - CsrBTreeMap Trait Dispatch Acceptance

**Status: complete** — `OrderedMap@compare_value` on CSR map shape delegates to captured `compare_value` in the generated record (not hardcoded ordering). Trial: `ordered_map_csr_compare_value_trait` in `trials/standard_data_structures_phase04_addition/`.

Authority:

- [data_structures_as_traits.md](data_structures_as_traits.md)
- Phase 0.4 trait dispatch and Phase 0.5 comparator delegation rules

Actions:

1. Confirm `OrderedMap` CSR impl record includes `compare_key` and `compare_value` fields.
2. Add phase04 acceptance trial using constructor function record `empty/1` and non-default `compare_value`.
3. Verify trait call returns comparator result from captured function.

Exit criteria:

- Trial passes with reversed value comparator on CSR map shape.

## Phase 9 - Heaps

Algorithm reference available:

The `.bak` copies of `heap_binary_min.silica` and `heap_binary_max.silica` preserve binary min/max heap and priority/value heap algorithms. The clean rewrite must expose heap behavior through constructor records and `Heap` / `PriorityQueue` traits.

### Step 9.1 - Generate RegionBinaryMinHeap — Done

Authority:

- `balanced_tree_and_heap_design.md` RegionBinaryHeap sections

Actions:

1. Generate the binary heap type string. ✅ `stdlib/data_structures/heap_binary_min.silica`.
2. Generate empty or allocate construction with a `compare_item` constructor function record. ✅ `empty({ compare_item })`.
3. Generate push. ✅ `min_heap_push` with binary sift-up.
4. Generate peek. ✅ `min_heap_peek` plus `Heap@peek`.
5. Generate pop. ✅ `min_heap_pop` with binary sift-down.
6. Generate validation of heap ordering and capacity metadata. ✅ `min_heap_validate`.

Exit criteria:

- Trials cover empty heap, push, peek, pop, and validation for `RegionBinaryMinHeap[ItemType, mem(normal)]`. ✅ `heap_binary_min_compare_item_trait` passes in the phase04 suite (`make integrate` in `trials/standard_data_structures_phase04_addition/` reports 34/0).

### Step 9.2 - Generate RegionBinaryHeap Variants Permitted By Design — Done

Authority:

- `balanced_tree_and_heap_design.md` RegionBinaryHeap sections

Actions:

1. Generate only variants explicitly described by the design: max heaps and priority/value heaps. ✅ `heap_binary_max.silica` (max heap) and the priority-queue wiring through `PriorityQueue`.
2. Reuse the same validation structure. ✅ Variants share the binary sift/validate shape.
3. Add trials for each generated variant. ✅ `heap_binary_max_constructor_record_trait` and `heap_priority_queue_constructor_record_trait`.

Exit criteria:

- Trials cover generated binary heap variants without changing the documented heap model. ✅ Both variant trials pass in the phase04 suite (34/0).

### Step 9.3 - Generate RegionDaryHeap Only After Binary Heap Stability — Done

Authority:

- `balanced_tree_and_heap_design.md` RegionDaryHeap sections

Actions:

1. Generate d-ary heap type strings. ✅ `stdlib/data_structures/heap_dary_min.silica` builds the
   six-field constructor-record shape (`{ compare_item, region, len, capacity, arity, values }`);
   `inline_type_expansion.silica` expands `expand_region_dary_min_heap_int64` to the fn-first order.
2. Generate construction. ✅ `empty({ compare_item })` captures the comparator into the record.
3. Generate push. ✅ `dary_min_heap_push` with d-ary (`D=4`) sift-up.
4. Generate peek. ✅ `dary_min_heap_peek` plus `Heap@peek`.
5. Generate pop. ✅ `dary_min_heap_pop` with d-ary sift-down (`dary_smallest_child_*` fold).
6. Generate validation. ✅ `dary_min_heap_validate`.
7. Keep arity handling aligned with the design. ✅ `arity()`/`child_index`/`parent_index` use `D=4`.

Exit criteria:

- D-ary heap trials pass after binary heap trials are stable. ✅ `heap_dary_min_compare_item_trait`
  passes (`make integrate` in `trials/standard_data_structures_phase04_addition/` reports 34/0):
  it builds via `empty`, pushes `{9,2,7,1,5}`, and verifies the captured comparator drives ordering
  through the generated ops and the standard `Heap` trait (`peek` / `len` / `compare_priority`),
  including a fully sorted drain.

Implementation note (resolved): earlier d-ary sift-down helpers bound every parameter to a local
before any `case` whose scrutinee performs a function call (e.g. `slot < arity()`) to dodge a backend
register-clobber bug — leaving parameters in their incoming caller-saved registers across such a call
let the branch bodies read clobbered values. This is now fixed in codegen: the Apple-silicon emitter
detects this hazard (a `case` scrutinee containing a user call) and, for functions whose parameters
all fit in callee-saved registers X19–X28, shadows every parameter into callee-saved registers at
function entry (`use_full_param_shadow` in `emitter_core.silica`). The source-level hoist workarounds
have been removed from `heap_dary_min.silica` and the `heap_dary_min_compare_item_trait` trial, which
still pass (34/0).

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
4. Keep the signed payload suite under the naming pattern `trials/std_data_structures_payload_signed_addition/`; use constructor function records to witness key/value/element types.

Exit criteria:

- Every signed integer width named in the design documents has at least one passing positive trial per applicable structure family.
- Signed btree payload trials use the trait/constructor-record API and do not rely on per-width public btree exports.

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

Deferred families (add payload trials when the family is generated): none for heaps — RegionDaryHeap is now generated (Step 9.3 complete, min/D=4) and its payload-trial deferral is lifted. DenseBitsetGraph payload coverage is pending beyond the Phase 1 directed unweighted path.

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

- [data_structures_as_traits.md](data_structures_as_traits.md)
- `graph_representation_design.md` section 8.4
- `btree_set_design.md` sections 8 and 9.5
- `balanced_tree_and_heap_design.md` naming and generator requirement sections

Actions:

1. Verify **module filenames** follow design rules (representation + directedness only for graphs; no payload type or memory space in the module name).
2. Verify generated **function names** follow design naming rules.
3. Verify constructors use inline function records whose field signatures agree with the declared collection type.
4. Verify behavior trait calls dispatch from concrete generated receiver values.
5. Verify stdlib modules do **not** duplicate the same `operation` per primitive width in one file.
6. Verify helper emission order follows design requirements.
7. Verify no generated module introduces custom Silica type declarations or named constructor-record aliases.
Exit criteria:

- Snapshot tests cover representative generated names and emission order.

### Step 10.27 - Documentation Status Update

Authority:

- All three design documents named at the top of this plan

Actions:

1. Add implementation-status notes to this plan as families are completed.
2. Design documents now include the **trait-oriented constructor-record** methodology (`data_structures_as_traits.md`; graph §2.11; balanced tree §2.6; btree set §4.1). Update this plan if implementation reveals a mismatch; do not revert to per-width duplicate exports in stdlib modules.
3. Link completed trial names from this plan.

Exit criteria:

- The plan remains a parseable implementation checklist and the design documents remain the authority.

## Completion Tracking

Last updated to reflect Step 1.1 Action 6 decision (unweighted edge payload Option A).


| Area                        | Status          | Notes                                                                                                                                                                                                 |
| --------------------------- | --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Shared generator foundation | Partial         | Phase 0.1–0.3 foundation exists in `src/standard_data_structures/structure_registry.silica`, `inline_type_expansion.silica`, and `trials/standard_data_structures_addition/`. `make integrate` passed for `trials/standard_data_structures_addition/`. |
| Phase 0.4 compiler          | Complete (set/map/heap/graph witnesses) | Items 1–5 done plus `DirectedGraph[…]` bracket witnesses (Step 1.1). Provided-block checking not started. See Phase 0.4 status table. |
| Phase 0.4 stdlib smoke      | Partial         | Nine trait modules; adapters for set/map; `silica.config.phase04_traits` batch green; phase04 trials integrate. `graph_adj_directed@empty/2` wired through `DirectedGraph` (Step 1.1); `graph_phase04` toy retained until Step 1.4. `heap_phase04` toy is retired from the stdlib build (removed from the `data_structures` Makefile and `silica.config*`); its source file is retained only because the `heap_witness_int64` witness trial still symlinks it. `btree_set_phase04` toy remains. Trait shape is staging `required`+`impl`, not final `provided`+`fold`. |
| Phase 0.5 stdlib reconciliation | Complete | Comparator delegation fixed in trait impls; nodeid `OrderedSet@size` uses `item_count` (leaf key totals); CSR set/map empty+insert preserve captured comparators; phase04 acceptance trials: `ordered_set_nodeid_size_trait`, `ordered_map_compare_value_trait`, `ordered_set_csr_compare_item_trait`. `{ found: int64 }` vs boolean remains staging debt. |
| Trait constructor records   | Partial         | Witness checking works for `OrderedSet`, `OrderedMap`, `Heap`, and `DirectedGraph` bracket types. Negative trials: E2017, E2092, E2003 in error_enforcement phase04 suite (graph negative witnesses not yet added). Invalid-atom validation deferred. |
| NodeIdAdjacencyGraph        | Step 1.1 complete | Step 1.1: `graph_adj_directed@empty/2`, captured comparators in empty value, `DirectedGraph` trait impls, trials `directed_graph_adj_empty_trait`, `directed_graph_trait`, `directed_graph_witness_int64`. Bootstrap width exports coexist (Step 10.26); Steps 1.2–1.7 and 1.4 `graph_phase04` retirement remain. |
| CompressedSparseRowGraph    | Steps 2.1–2.5 complete | Type expansion, static constructor, validation, inspection, and adjacency `freeze`. |
| DenseMatrixGraph            | Steps 3.1–3.2 complete | Type expansion, constructor, inspection (`has_edge`, `neighbor_at`, `out_degree`, `weight_at`), validation trials green. |
| DenseBitsetGraph            | Step 3.3 complete (directed unweighted) | `DenseBitsetGraphDirected[mem(normal)]` uses `uint64` word storage (`WORD_COUNT = 4`) with set/clear/has/out-degree/neighbor/validate trials green. Weighted variants remain unsupported and covered by negative enforcement; dense matrix remains the fallback for weighted dense graphs. |
| Graph algorithms            | Phase 4 complete (module API) | `reachable/3`, `max_out_degree/1`, and `total_out_degree_sum/1` are covered for adjacency and CSR generated-capacity graphs. Trait-level reattachment remains part of the later standard graph trait migration. |
| NodeIDBTreeSet              | Phase 5 complete (module API) | `btree_set_nodeid@empty({ compare_item })` preserves comparator; `contains`, `validate`, `insert`, and `OrderedSet@size` are covered by Phase 5 trials. Insert handles generated-capacity leaf accumulation, duplicate status, and the first root split; delete remains deferred by design. |
| CsrBTreeSet                 | Phase 6 complete (module API) | `btree_set_csr@empty/1` stores `compare_item`; leaf and first split-root shapes support `contains`, `validate`, and functional insert. `btree_set_nodeid@to_csr` finalizes generated NodeID leaf/split trees into CSR; delete and deeper growth remain deferred by design. |
| NodeIDBTreeMap              | Phase 7 complete (module API) | `btree_nodeid@empty({ compare_key, compare_value })` stores both functions; leaf and first split-root shapes support `get`, functional `insert`, and `validate`. `OrderedMap@size`, `@find_value`, and `@fold` delegate via adapters; nodeid `compare_value` uses captured fn. Trials: `ordered_map_nodeid_size_trait`, `ordered_map_nodeid_find_value_trait`, `ordered_map_nodeid_fold_trait`, `ordered_map_compare_value_trait`, `btree_nodeid_map_to_csr`. Deeper growth beyond first split-root remains deferred by design. |
| CsrBTreeMap                 | Phase 8 complete (module API) | `btree_csr_map@empty/1` stores `compare_key` and `compare_value`; leaf and first split-root shapes support `from_static_sorted`, `contains`, `get`, functional `insert`, and `validate`. `btree_nodeid@to_csr` finalizes NodeID leaf/split maps. Trials: `btree_csr_map_contains_static`, `btree_csr_map_validate_invalid`, `btree_csr_map_insert`, `btree_csr_map_insert_split`, `btree_nodeid_map_to_csr`, `ordered_map_csr_compare_value_trait`. Deeper growth beyond first split-root remains deferred by design. |
| RegionBinaryHeap            | Steps 9.1–9.2 complete (min, max, priority/value) | `heap_binary_min.silica` / `heap_binary_max.silica` expose `empty({ compare_item })` constructor function records; generated `push`/`peek`/`pop`/`validate` dispatch through the standard `Heap` / `PriorityQueue` traits. Trials `heap_binary_min_compare_item_trait`, `heap_binary_max_constructor_record_trait`, and `heap_priority_queue_constructor_record_trait` are green in the phase04 suite (34/0). The `heap_phase04` toy module is retired from the stdlib build (its source survives only for the `heap_witness_int64` witness trial). |
| RegionDaryHeap              | Step 9.3 complete (min, D=4) | `heap_dary_min@empty({ compare_item })` captures the comparator into the six-field fn-first record; generated `push`/`peek`/`pop`/`validate` use `D=4` sift logic, and `Heap@peek`/`@len`/`@compare_priority` dispatch over the record. Trial `heap_dary_min_compare_item_trait` (sorted drain) green in the phase04 suite. The earlier source-level register-hoist workaround has been removed now that the backend register-clobber bug is fixed (`use_full_param_shadow`). Max/d-ary-max and priority-queue variants remain future work. |
| Cross-structure audit       | Not started     | Phase 10 — Payload coverage (Steps 10.1–10.23), immutability (10.24), region ownership (10.25), naming (10.26), documentation (10.27). |
