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

The modules compile direct static constructors, validation helpers, out-degree, edge lookup, and weighted lookup. The graph integration trials currently use no-op runtime mains because the current emitter hangs when region-owned CSR buffer records are constructed, returned, or passed through graph trial executables. Assembly and executable goldens still verify that the generated modules compile, assemble, link, and remain stable.

Success trials:

- `graph_csr_directed_unweighted.silica`
- `graph_csr_directed_weighted_int64.silica`

## Phase 3

`DenseMatrixGraph` has bootstrap generated modules for directed unweighted and directed weighted int64 dense matrix graphs in `src/standard_data_structures/`.

The modules compile fixed 3-node capacity matrix constructors, checked edge setters, edge lookup, out-degree, weighted lookup, and validation helpers. Like the CSR trials, runtime mains are currently no-op because region-owned buffer records are emitter-sensitive in graph trial executables. Assembly and executable goldens verify compile, assemble, link, and output stability.

`DenseBitsetGraph` is deferred per `graph_representation_design.md` §6.4. The current bootstrap path documents the fallback to `DenseMatrixGraphDirectedUnweighted` until bitwise `|`, `&`, and shift are available in this compiler path.

Success trials:

- `graph_dense_directed_unweighted.silica`
- `graph_dense_directed_weighted_int64.silica`
