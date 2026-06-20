# B-tree set generated-structure trials (Phase 5+)

Design/registry names and operation instantiation follow **`List`-aligned bracket syntax** ([btree_set_design.md](../../design_documents/btree_set_design.md) §4.1):

- `NodeIDBTreeSet[int64, mem(normal)]` — module `btree_set_nodeid`
- `CsrBTreeSet[int64, mem(normal)]` — module `btree_set_csr`

Call sites use **`module@operation[brackets](args)`** with short operation names (no module-prefix duplication), for example `btree_set_csr@contains[int64, mem(normal)](tree, key)`.

## Layout

- **`lib/`** — symlinks to `src/standard_data_structures/` modules.
- **Trial drivers** — `btree_set_<repr>_<scenario>.silica`.

## Phase 5 (`NodeIDBTreeSet[int64, mem(normal)]`)

- `btree_set_nodeid_empty_contains.silica` — empty construction, validation, absent key
- `btree_set_nodeid_contains_handbuilt.silica` — membership on a hand-built tree
- `btree_set_nodeid_insert.silica` — stable insert and duplicate status
- `btree_set_nodeid_insert_split.silica` — root split insertion, membership, and size
- `btree_set_nodeid_validate_invalid.silica` — invalid tree rejected by validation

## Phase 6 (`CsrBTreeSet[int64, mem(normal)]`)

- `btree_set_csr_contains_static.silica` — static sorted keys `{1,3,5}`
- `btree_set_csr_validate_invalid.silica` — invalid CSR rejected by validation
- `btree_set_csr_insert.silica` — direct functional CSR insert, duplicate status, membership, and immutability
- `btree_set_nodeid_to_csr.silica` — insert to the split NodeID shape, finalize to split CSR, validate, and verify membership

The `integrate` target runs all trials through the local compiler with a per-executable timeout so recursive runtime regressions fail quickly.
