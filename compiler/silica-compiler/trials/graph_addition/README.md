# Graph generated-structure trials (Phase 1+)

Reserved for `NodeIdAdjacencyGraph`, `CompressedSparseRowGraph`, `DenseMatrixGraph`, and `DenseBitsetGraph` generated code.

Separate from `list_addition` (compiler `List[T,S]` runtime).

## Phase 1

`NodeIdAdjacencyGraph` now exercises the directed unweighted normal-memory path with list-backed node records (`node_count <= 3`). The undirected and weighted adjacency trials remain bootstrap coverage for the generated Phase 1 families.

Success trials:

- `graph_adj_directed_unweighted.silica`
- `graph_adj_undirected_unweighted.silica`
- `graph_adj_directed_weighted_int64.silica`

Validation/runtime enforcement trial:

- `trials/error_enforcement_addition/generated_data_structures/graph/graph_adj_invalid_endpoint.silica`

## Phase 0

Phase 0 registry/type expansion trials live in `trials/standard_data_structures_addition/`.

Validation failures: `trials/error_enforcement_addition/generated_data_structures/graph/`.

## Phase 2

`CompressedSparseRowGraph` has bootstrap generated modules for directed unweighted and directed weighted int64 CSR graphs in `src/standard_data_structures/`.

The modules compile direct static constructors, validation helpers, buffer-backed out-degree, edge lookup, and weighted lookup. The graph integration trials now run runtime mains that construct region-owned CSR buffer records, pass them through inspection helpers, and verify node count, edge count, validation, present edges, absent edges, out-degree, and weighted lookup.

Success trials:

- `graph_csr_directed_unweighted.silica`
- `graph_csr_directed_weighted_int64.silica`

## Phase 3

`DenseMatrixGraph` has generated modules for directed unweighted and directed weighted int64 dense matrix graphs in `src/standard_data_structures/`.

The modules provide fixed 3-node capacity matrix constructors, checked edge setters, direct-buffer edge lookup, out-degree, weighted lookup, and validation helpers using flat int64 guards (same emitter-safe style as CSR trials). Integration runs under `silica-compiler` via `make integrate` in this directory.

`DenseBitsetGraph` is deferred per `graph_representation_design.md` §6.4. The current path documents the fallback to `DenseMatrixGraphDirectedUnweighted` until bitwise `|`, `&`, and shift are available.

Success trials:

- `graph_dense_directed_unweighted.silica`
- `graph_dense_directed_weighted_int64.silica`

## Phase 4

Reachability and degree-summary helpers reuse `has_edge` / `out_degree` traversal APIs (bootstrap `node_count <= 3`).

Success trials:

- `graph_reachability_adj_directed_unweighted.silica`
- `graph_reachability_csr_directed_unweighted.silica`
- `graph_degree_summary_csr_directed_unweighted.silica`
