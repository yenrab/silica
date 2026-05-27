# Standard Data Structures Implementation Plan

This plan organizes implementation work for Silica's standard generated data structures. It is an execution plan only. The design authority remains:

- [graph_representation_design.md](graph_representation_design.md)
- [balanced_tree_and_heap_design.md](balanced_tree_and_heap_design.md)
- [btree_set_design.md](btree_set_design.md)

This document does not introduce new representations, naming rules, memory-space rules, error shapes, generated API shapes, or source-level syntax. When a detail is needed, use the relevant design document section named in the step.

This document intentionally contains no Silica source code. It is written as fine-grained work items that an LLM can follow when generating Silica code from the design documents.

## Implementation Order

1. Graph representations first.
2. B-tree set representations second.
3. General balanced B-tree representations third.
4. Heap representations fourth.
5. Cross-structure validation, docs, and trial cleanup last.

Graph comes first because the tree and heap designs explicitly reuse graph vocabulary: node ids, inline record shapes, memory-space rules, generated naming conventions, and region-backed buffers.

## Global Rules For All Steps

Every implementation step must obey these rules:

1. Treat design names as generator names only.
2. Emit full inline structural types everywhere a Silica type is required.
3. Use the memory-space rules from the relevant design document.
4. Use `Storable` exactly as described by the relevant design document.
5. Keep generated naming consistent with the relevant design document.
6. Add positive trials before relying on a generated helper in later phases.
7. Add negative or validation trials for every documented invariant that can be checked.
8. Do not implement a faster packed form until the clear list-oriented form has validation coverage, unless the design document specifically allows direct packed construction.
9. Do not add unplanned APIs. If a helper is needed only internally, name and scope it as a generator helper and keep it aligned with the design document's helper requirements.

## Phase 0 - Shared Generator Foundation

### Step 0.1 - Create Structure Registry

Authority:

- `graph_representation_design.md` sections 2.1 through 2.6 and 8
- `balanced_tree_and_heap_design.md` sections 2 and 3
- `btree_set_design.md` sections 4 and 8 through 9

Actions:

1. Create a generator-side registry of supported structure families.
2. Register the graph families named in the graph design.
3. Register the B-tree and heap families named in the balanced tree and heap design.
4. Register the B-tree set families named in the B-tree set design.
5. For each family, record only design-document fields: representation name, memory space, weightedness when applicable, directedness when applicable, key type when applicable, and capacity constants when applicable.

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

- `graph_representation_design.md` sections 2.6, 3.4, 3.6, 8.2, and 8.3

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

## Phase 4 - Graph Algorithms Over Stable Traversal APIs

### Step 4.1 - Generate Reachability

Authority:

- `graph_representation_design.md` sections 2.6, 3.6, 4.7, 5.6, and 7

Actions:

1. Generate reachability over adjacency graphs.
2. Generate reachability over CSR graphs.
3. Reuse the traversal APIs from earlier phases.
4. Keep queue or stack representation consistent with available Silica list and buffer support.

Exit criteria:

- Trials cover reachable and unreachable node pairs.

### Step 4.2 - Generate Degree Summaries

Authority:

- `graph_representation_design.md` sections 2.6, 3.6, 4.7, and 5.6

Actions:

1. Generate total degree summaries.
2. Generate max out-degree summary.
3. Generate per-node out-degree traversal helpers where required.

Exit criteria:

- Trials verify summaries against small graphs with known answers.

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

## Phase 6 - B-tree Set: CsrBTreeSet

### Step 6.1 - Generate CsrBTreeSet Type Expansion

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

Authority:

- `btree_set_design.md` sections 6.6 and 6.8

Actions:

1. Generate CSR membership query.
2. Generate CSR node search.
3. Generate validation checks.
4. Keep error codes aligned with the NodeIDBTreeSet design where possible.

Exit criteria:

- Trials cover present keys, absent keys, and validation failures.

### Step 6.4 - Generate NodeIDBTreeSet-To-CsrBTreeSet Finalization

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

## Phase 7 - General B-tree: NodeIDBTree

### Step 7.1 - Generate NodeIDBTree Type Expansion

Authority:

- `balanced_tree_and_heap_design.md` sections 3 and 4

Actions:

1. Generate the general NodeIDBTree type string.
2. Include keys and values exactly as specified by the design.
3. Snapshot-test the type expansion.

Exit criteria:

- Type string snapshots are stable.

### Step 7.2 - Generate NodeIDBTree Search And Validation

Authority:

- `balanced_tree_and_heap_design.md` B-tree invariant and NodeIDBTree sections

Actions:

1. Generate search by key.
2. Generate value lookup when values are part of the selected family.
3. Generate invariant validation.
4. Reuse set validation concepts only where the balanced-tree design permits.

Exit criteria:

- Trials cover present keys, absent keys, and invalid trees.

### Step 7.3 - Generate NodeIDBTree Insert

Authority:

- `balanced_tree_and_heap_design.md` NodeIDBTree insert and split sections

Actions:

1. Generate insertion using the duplicate-key policy specified by the design.
2. Generate node split helpers.
3. Preserve B-tree invariants.
4. Return the documented result shape.

Exit criteria:

- Trials cover insert, replacement or duplicate policy behavior, and split.

## Phase 8 - General B-tree: CsrBTree

### Step 8.1 - Generate CsrBTree Static Construction

Authority:

- `balanced_tree_and_heap_design.md` CsrBTree sections

Actions:

1. Generate CSR B-tree type strings.
2. Generate construction from known data when the design permits.
3. Include region ownership and capacity constants.

Exit criteria:

- A trial constructs a CSR B-tree and validates it.

### Step 8.2 - Generate CsrBTree Search And Validation

Authority:

- `balanced_tree_and_heap_design.md` CsrBTree search and validation sections

Actions:

1. Generate search.
2. Generate value lookup when values are present.
3. Generate validation.
4. Keep mutation policy aligned with the design.

Exit criteria:

- Trials cover search, lookup, and validation.

## Phase 9 - Heaps

### Step 9.1 - Generate RegionBinaryMinHeapInt64

Authority:

- `balanced_tree_and_heap_design.md` RegionBinaryHeap sections

Actions:

1. Generate the binary heap type string.
2. Generate empty or allocate construction.
3. Generate push.
4. Generate peek.
5. Generate pop.
6. Generate validation of heap ordering and capacity metadata.

Exit criteria:

- Trials cover empty heap, push, peek, pop, and validation.

### Step 9.2 - Generate RegionBinaryHeap Variants Permitted By Design

Authority:

- `balanced_tree_and_heap_design.md` RegionBinaryHeap sections

Actions:

1. Generate only variants explicitly described by the design.
2. Reuse the same validation structure.
3. Add trials for each generated variant.

Exit criteria:

- Variant trials pass without changing the documented heap model.

### Step 9.3 - Generate RegionDaryHeap Only After Binary Heap Stability

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

### Step 10.1 - Shared Storable Coverage

Authority:

- `graph_representation_design.md` section 2.4
- `balanced_tree_and_heap_design.md` section 2.4
- `btree_set_design.md` section 4.0

Actions:

1. Verify all generated APIs use `Storable` exactly where the design documents require it.
2. Verify structural metadata remains plain structural data, not `Storable`.
3. Add compile-level checks or trials for the first monomorphic `int64` families.

Exit criteria:

- All generated public APIs follow the documented `Storable` rule.

### Step 10.2 - Region Ownership Audit

Authority:

- `graph_representation_design.md` sections 2.5, 4.8, and 9
- `balanced_tree_and_heap_design.md` section 2.2
- `btree_set_design.md` sections 2 and 6

Actions:

1. Audit every generated structure that contains buffers.
2. Verify the owning region is carried in the returned record where required.
3. Verify no generated helper returns bare buffers without region ownership when the design forbids it.

Exit criteria:

- Region ownership is documented in generated-family status notes and covered by type-level trials where possible.

### Step 10.3 - Naming And Emission Order Audit

Authority:

- `graph_representation_design.md` section 8.4
- `btree_set_design.md` sections 8 and 9.5
- `balanced_tree_and_heap_design.md` naming and generator requirement sections

Actions:

1. Verify generated function names follow design naming rules.
2. Verify helper emission order follows design requirements.
3. Verify no generated module introduces custom Silica type declarations.

Exit criteria:

- Snapshot tests cover representative generated names and emission order.

### Step 10.4 - Documentation Status Update

Authority:

- All three design documents named at the top of this plan

Actions:

1. Add implementation-status notes to this plan as families are completed.
2. Do not change the design documents unless implementation reveals an actual design/document mismatch.
3. Link completed trial names from this plan.

Exit criteria:

- The plan remains a parseable implementation checklist and the design documents remain the authority.

## Completion Tracking

| Area | Status | Notes |
|------|--------|-------|
| Shared generator foundation | Not started | Phase 0 |
| NodeIdAdjacencyGraph | Not started | Phase 1 |
| CompressedSparseRowGraph | Not started | Phase 2 |
| DenseMatrixGraph | Not started | Phase 3 |
| DenseBitsetGraph | Not started | Phase 3, conditional on bit operations per graph design |
| Graph algorithms | Not started | Phase 4 |
| NodeIDBTreeSet | Not started | Phase 5 |
| CsrBTreeSet | Not started | Phase 6 |
| NodeIDBTree | Not started | Phase 7 |
| CsrBTree | Not started | Phase 8 |
| RegionBinaryHeap | Not started | Phase 9 |
| RegionDaryHeap | Not started | Phase 9 |
| Cross-structure audit | Not started | Phase 10 |
