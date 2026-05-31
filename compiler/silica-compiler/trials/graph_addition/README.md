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

- `graph_csr_directed_unweighted_trial.silica`
- `graph_csr_directed_int64_trial.silica`

## Phase 3

- `graph_dense_directed_unweighted_trial.silica`
- `graph_dense_directed_int64_trial.silica`

`DenseBitsetGraph` is deferred per `graph_representation_design.md` §6.4.

## Phase 4

- `graph_reachability_adj_directed_trial.silica`
- `graph_reachability_csr_directed_trial.silica`
- `graph_degree_summary_csr_directed_trial.silica`
