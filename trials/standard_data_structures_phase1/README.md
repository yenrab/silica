# Phase 1 Standard Data Structures — Trial Hierarchy

**Layer 0 §6.2** trial root for dependency-ordered implementation of the Phase 1 standard data structures.

**Design baseline:** [`design_documents/Phase1_TODOs/standard_data_structures_baseline/normative_inputs.md`](../../design_documents/Phase1_TODOs/standard_data_structures_baseline/normative_inputs.md)

**Requirements ledger:** [`design_documents/Phase1_TODOs/standard_data_structures_baseline/requirements_to_trials_ledger.md`](../../design_documents/Phase1_TODOs/standard_data_structures_baseline/requirements_to_trials_ledger.md)

**CSR/dense contract:** [`design_documents/Phase1_TODOs/standard_data_structures_baseline/csr_dense_representation_contract.md`](../../design_documents/Phase1_TODOs/standard_data_structures_baseline/csr_dense_representation_contract.md) — applies to `snapshot_graphs/` and `cross_representation/` leaves.

**Do not copy** trials or stdlib modules from `standard_data_structures_phase04_addition`, `btree_*_addition`, or other pre-reset trees.

## Directory layout

| Directory | Layer / dependency | Purpose |
|---|---|---|
| `compiler_substrate/` | Layer 1 | Canonical arenas, ordering identity, recursive tuples, trait dispatch, constructor records |
| `wbt_core/` | Layer 2A | Adams WBT `(3, 2)` core |
| `skew_ral_core/` | Layer 2B | Skew binary random-access list |
| `brodal_okasaki_core/` | Layer 2C | Brodal–Okasaki queue core |
| `binary_tree_core/` | Layer 2D | Persistent fixed-role binary-tree core (planned by 2026-07-02 amendment) |
| `binary_tree/` | Layer 3D | `tree_binary`, `BinaryTree`, and inline zipper acceptance (planned) |
| `ordered_collections/` | Layer 3 | `wbt_set`, `wbt_map`, `OrderedSet`, `OrderedMap`, `Heap` |
| `live_graphs/` | Layer 4 | Live WBT graph core and directed/undirected/weighted modules |
| `terminal_structures/` | Layer 5 | `SearchTree`, `PriorityQueue`, `Tree` (leaf traits) |
| `snapshot_graphs/` | Layer 6 | CSR freeze and dense matrix graphs |
| `error_enforcement/` | all layers | Compile-time and negative constructor/trait checks |
| `cross_representation/` | Layer 7 | WBT vs CSR vs dense conformance through public traits |

## Trial categories

Each leaf Makefile (`leaf.mk`) supports four trial kinds via filename prefix:

| Kind | Filename pattern | Artifacts | Harness |
|---|---|---|---|
| **Compile success** | ordinary `*.silica` (not excluded prefixes) | `.ascomp`, `.scout` | `make integrate` in leaf |
| **Runtime success** | same as compile success | `.scout` includes stdout + exit code | `make integrate` run step |
| **Compile failure** | `trial_compile_fail_*.silica` | `.golden_fail` | `error_enforcement/` only (isolated compile) |
| **Runtime collection error** | `trial_collection_error_*.silica` | `.scout` with stderr + exit code | added when collection runtime exists |

Excluded from positive `silica.config` automatically:

- `trial_negative_*.silica` — reserved for isolated negative compiles
- `trial_compile_fail_*.silica`
- `trial_collection_error_*.silica`

Sidecar `*.no_golden_fail` skips a source file in negative harness loops (same convention as `error_enforcement_addition`).

## Building

First-time setup after building the Phase 2 compiler (`make -C ../../src`):

```bash
make record-golden   # capture .ascomp, .scout, and compile-fail .golden_fail
```

From this directory:

```bash
make integrate    # all dependency subdirectories
make clean
```

From a leaf:

```bash
cd compiler_substrate && make integrate
```

## Smoke fixtures

Each dependency leaf (except compile-fail-only paths) includes `smoke_harness_ready.silica` — a minimal compile-and-run check that the leaf Makefile and compiler pipeline work. Replace smoke fixtures with real acceptance trials as each layer is implemented.

The BinaryTree amendment adds `binary_tree_core/` and `binary_tree/` to the required hierarchy. Create their leaf harnesses when §7.10 makes the family runnable; their absence before that branch gate does not invalidate the recorded Layer 0 smoke baseline.
