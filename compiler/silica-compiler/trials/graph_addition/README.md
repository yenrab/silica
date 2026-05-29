# Graph generated-structure trials (Phase 1+)

Reserved for `NodeIdAdjacencyGraph`, `CompressedSparseRowGraph`, `DenseMatrixGraph`, and `DenseBitsetGraph` generated code.

Design/registry names and operation instantiation follow **`List`-aligned bracket syntax** ([graph_representation_design.md](../../design_documents/graph_representation_design.md) §2.11): payload type(s) then **`mem(normal)`** — for example `NodeIdAdjacencyGraphDirected[mem(normal)]`, `NodeIdAdjacencyGraphDirected[int64, mem(normal)]`.

Separate from `list_addition` (compiler `List[T,S]` runtime).

## Layout

- **`lib/`** — trial-local generated support modules (bracket-named registry keys). Operations use explicit instantiation, e.g. `graph_adj_directed_empty[mem(normal)]()`.
- **Trial drivers** — `graph_<repr>_<directedness>_<bracket-slug>_trial.silica` where `<bracket-slug>` is `mem_normal` (unweighted) or `int64_mem_normal` (edge payload `int64`). Drivers must not share a module basename with `lib/` (symlinked `src/standard_data_structures/` modules).

## Phase 1

`NodeIdAdjacencyGraphDirected[mem(normal)]` exercises the directed unweighted path with list-backed node records (`node_count <= 3`). Undirected and weighted adjacency trials cover the other Phase 1 families.

Success trials:

- `graph_adj_directed_mem_normal.silica` — `NodeIdAdjacencyGraphDirected[mem(normal)]`
- `graph_adj_undirected_mem_normal.silica` — `NodeIdAdjacencyGraphUndirected[mem(normal)]`
- `graph_adj_directed_int64_mem_normal.silica` — `NodeIdAdjacencyGraphDirected[int64, mem(normal)]`

Validation/runtime enforcement trial:

- `trials/error_enforcement_addition/generated_data_structures/graph/graph_adj_invalid_endpoint.silica`

## Phase 0

Phase 0 registry/type expansion trials live in `trials/standard_data_structures_addition/`.

Validation failures: `trials/error_enforcement_addition/generated_data_structures/graph/`.

## Phase 2

`CompressedSparseRowGraphDirected[mem(normal)]` and `CompressedSparseRowGraphDirected[int64, mem(normal)]` bootstrap modules live in `lib/`.

Success trials:

- `graph_csr_directed_mem_normal.silica`
- `graph_csr_directed_int64_mem_normal.silica`

## Phase 3

`DenseMatrixGraphDirected[mem(normal)]` and `DenseMatrixGraphDirected[int64, mem(normal)]` bootstrap modules live in `lib/`.

`DenseBitsetGraph` is deferred per `graph_representation_design.md` §6.4. The current path documents the fallback to `DenseMatrixGraphDirected[mem(S)]` until bitwise `|`, `&`, and shift are available.

Success trials:

- `graph_dense_directed_mem_normal.silica`
- `graph_dense_directed_int64_mem_normal.silica`

## Phase 4

Reachability and degree-summary helpers reuse `has_edge` / `out_degree` traversal APIs (bootstrap `node_count <= 3`).

Success trials:

- `graph_reachability_adj_directed_mem_normal.silica`
- `graph_reachability_csr_directed_mem_normal.silica`
- `graph_degree_summary_csr_directed_mem_normal.silica`
