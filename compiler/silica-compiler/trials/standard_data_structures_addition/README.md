# Standard data structures — Phase 0 trials

Foundation trials for the generator in `src/standard_data_structures/`:

| File | Role |
|------|------|
| `lib/structure_registry.silica` | Symlinks to registry (24 families + design-doc metadata) |
| `lib/inline_type_expansion.silica` | Symlinks to canonical inline type strings |
| `type_expansion_snapshot.silica` | Prints every family name + expanded type; exit code = validation failure count |
| `type_expansion_snapshot.scout` | Golden stdout + exit code |
| `type_expansion_snapshot.ascomp` | Golden assembly |

## Integrate

```bash
make -C trials/standard_data_structures_addition integrate
```

## Related trial directories

- `graph_addition`, `btree_set_addition`, `balanced_tree_addition`, `heap_addition` — empty placeholders until Phases 1–9
- `error_enforcement_addition/generated_data_structures/` — future validation-failure goldens
