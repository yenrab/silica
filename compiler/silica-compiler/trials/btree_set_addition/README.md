# B-tree set generated-structure trials (Phase 5+)

Phase 5 adds the bootstrap `NodeIDBTreeSetInt64Normal` source module and runtime trials for empty construction, non-empty membership on a hand-built tree, stable insert/duplicate status, and invalid validation.

The `integrate` target wires all Phase 5 B-tree set trial sources through the local compiler and uses a per-executable timeout so recursive runtime regressions fail quickly instead of hanging the suite.
