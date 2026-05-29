# B-tree set generated-structure trials (Phase 5+)

Design/registry names and operation instantiation follow **`List`-aligned bracket syntax** ([btree_set_design.md](../../design_documents/btree_set_design.md) §4.1):

- `NodeIDBTreeSet[int64, mem(normal)]` — module `btree_set_nodeid_mem_normal`
- `CsrBTreeSet[int64, mem(normal)]` — module `btree_set_csr_mem_normal`

Operations use explicit instantiation at call sites, e.g. `btree_set_nodeid_empty[int64, mem(normal)]()`.

## Layout

- **`lib/`** — trial-local generated support modules (bracket-named registry keys).
- **Trial drivers** — `btree_set_<repr>_mem_normal_<scenario>.silica`.

## Phase 5 (`NodeIDBTreeSet[int64, mem(normal)]`)

- `btree_set_nodeid_mem_normal_empty_contains.silica` — empty construction, validation, absent key
- `btree_set_nodeid_mem_normal_contains_handbuilt.silica` — membership on a hand-built tree
- `btree_set_nodeid_mem_normal_insert.silica` — stable insert and duplicate status
- `btree_set_nodeid_mem_normal_validate_invalid.silica` — invalid tree rejected by validation

## Phase 6 (`CsrBTreeSet[int64, mem(normal)]`)

- `btree_set_csr_mem_normal_contains_static.silica` — static sorted keys `{1,3,5}`
- `btree_set_csr_mem_normal_validate_invalid.silica` — invalid CSR rejected by validation
- `btree_set_nodeid_mem_normal_to_csr.silica` — insert on NodeID form, finalize to CSR, verify membership

The `integrate` target runs all trials through the local compiler with a per-executable timeout so recursive runtime regressions fail quickly.
