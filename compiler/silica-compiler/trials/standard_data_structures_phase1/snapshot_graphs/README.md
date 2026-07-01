# Snapshot graph trials — CSR/dense contract

Normative contract: [`csr_dense_representation_contract.md`](../../../design_documents/Phase1_TODOs/standard_data_structures_baseline/csr_dense_representation_contract.md)

Ledger clause IDs: `CSR-D2` … `CSR-D6` (this leaf); `CSR-D7` also involves `cross_representation/`.

Planned acceptance trials for Layer 6:

| Trial stem | Clause |
|---|---|
| `csr_node_id_not_slot` | CSR-D2 |
| `dense_node_id_not_slot` | CSR-D2 |
| `csr_runtime_extents` | CSR-D3 |
| `dense_v_squared_overflow` | CSR-D3 |
| `csr_weighted_parallel_buffers` | CSR-D4 |
| `dense_unweighted_boolean_cells` | CSR-D5 |
| `dense_weighted_tagged_cells` | CSR-D6 |
| `csr_freeze_from_live` | CSR §4 freeze pipeline |
| `csr_freeze_overflow_fail` | CSR §9 failure behavior |

Layer 0 smoke only: `smoke_harness_ready.silica`.
