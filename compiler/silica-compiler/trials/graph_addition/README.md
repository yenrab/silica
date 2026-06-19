# Graph generated-structure trials (Phase 1+)

Reserved for `NodeIdAdjacencyGraph`, `CompressedSparseRowGraph`, `DenseMatrixGraph`, and `DenseBitsetGraph` generated code.

Design/registry names and operation instantiation follow **`List`-aligned bracket syntax** ([graph_representation_design.md](../../design_documents/graph_representation_design.md) §2.11): payload type(s) then **`mem(normal)`** — for example `NodeIdAdjacencyGraphDirected[mem(normal)]`, `NodeIdAdjacencyGraphDirected[int64, mem(normal)]`.

Call sites use **`module@operation[brackets](args)`** with short operation names (no module-prefix duplication), for example `graph_adj_directed@empty[mem(normal)]()`.

Separate from `list_addition` (compiler `List[T,S]` runtime).

## Layout

- **`lib/`** — symlinks to `src/standard_data_structures/` modules.
- **Trial drivers** — `graph_<repr>_<directedness>_<variant>_trial.silica` (`unweighted`, `int64`, etc.).

## Phase 1

`NodeIdAdjacencyGraphDirected[mem(normal)]` exercises the directed unweighted path with list-backed node records (`node_count <= 3`).

Success trials:

- `graph_adj_directed_unweighted_trial.silica` — `NodeIdAdjacencyGraphDirected[mem(normal)]`
- `graph_adj_undirected_trial.silica` — `NodeIdAdjacencyGraphUndirected[mem(normal)]`
- `graph_adj_directed_int64_trial.silica` — `NodeIdAdjacencyGraphDirected[int64, mem(normal)]`

## Phase 2

Step 2.1 — CSR inline type expansion (unweighted and weighted):

- `graph_csr_type_expansion_snapshot.silica` — registry + `inline_type_expansion` golden for all four `CompressedSparseRowGraph*` families

Steps 2.2–2.4:

- `graph_csr_static_constructor_trial.silica` — step 2.2: `from_static_edges` for unweighted and weighted; verifies `node_count` and `edge_count` only
- `graph_csr_validate_valid.silica` — step 2.3: positive validation on static graphs
- `graph_csr_validate_invalid.silica` — step 2.3: non-monotonic offsets rejected (error code 4)
- `graph_csr_inspection_trial.silica` — step 2.4: `out_degree`, `neighbor_at`, `has_edge`, `weight_at`
- `graph_csr_adj_finalize_trial.silica` — step 2.5: `freeze` from adjacency graph; verifies equivalent `has_edge` and weighted `weight_at`
- `graph_csr_directed_unweighted_trial.silica`
- `graph_csr_directed_int64_trial.silica`

## Phase 3

- `graph_dense_directed_unweighted_trial.silica`
- `graph_dense_directed_int64_trial.silica`
- `graph_dense_bitset_type_expansion_snapshot.silica` — registry + inline type expansion for dense bitset graph families
- `graph_dense_bitset_constructor_trial.silica` — `DenseBitsetGraphDirected[mem(normal)]` construction, duplicate insert, checked invalid endpoint, validation
- `graph_dense_bitset_inspection_trial.silica` — cross-word set/clear, `has_edge`, `out_degree`, `neighbor_at`, validation
- `graph_dense_bitset_validate_invalid.silica` — validation rejects stored bits outside `node_count * node_count`

`DenseBitsetGraphDirected[mem(normal)]` is implemented for the Phase 1 unweighted `uint64` path. Weighted dense graphs continue to use the dense matrix representation.

## Phase 4

- `graph_reachability_adj_directed_trial.silica`
- `graph_reachability_csr_directed_trial.silica`
- `graph_degree_summary_csr_directed_trial.silica`
