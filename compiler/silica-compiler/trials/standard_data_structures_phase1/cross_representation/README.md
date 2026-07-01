# Cross-representation graph trials

Normative contract: [`csr_dense_representation_contract.md`](../../../design_documents/Phase1_TODOs/standard_data_structures_baseline/csr_dense_representation_contract.md)

Primary clause: **CSR-D7** — WBT live, CSR snapshot, and dense matrix values are distinct concrete types that must agree through **public graph traits only**.

Planned acceptance trials for Layer 7:

| Trial stem | Clause |
|---|---|
| `graph_distinct_concrete_types` | CSR-D7 |
| `wbt_csr_dense_trait_agree` | CSR-D7 |
| `weighted_csr_conformance` | weighted trait + CSR-D4 |
| `csr_query_neighbors` | CSR §6 query behavior |

Layer 0 smoke only: `smoke_harness_ready.silica`.
