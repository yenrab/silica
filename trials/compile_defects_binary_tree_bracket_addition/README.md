# compile_defects_binary_tree_bracket_addition

One valid Silica program that today's parser/type checker rejects (see
`defect_binary_tree_bracket.silica` and the `compile_defects_addition` README for why a
compile-time defect gets a directory of its own). References the real stdlib
`tree_binary.silica`/`BinaryTree.silica` by relative path rather than copying them.

- `defect_binary_tree_bracket`: `BinaryTree[string, mem(normal)]` is rejected (E1040) everywhere
  it is used, because `tree_binary`/`BinaryTree` are missing from
  `type_checker_collections.silica`'s `representation_id_from_module` (and
  `required_constructor_fields_for_module`) table -- every other bracket collection type
  (`wbt_set`, `wbt_map`, the graph modules, `brodal_okasaki_*`, `tree_rose`) has an entry there,
  `tree_binary` does not. Expected: a one-node tree; prints `root`; exit 1 (`node_count`).

Move the trial to `ordered_data_structures` (or `list_addition`'s tree sibling, wherever the fix
lands) once `representation_id_from_module` recognizes `tree_binary`/`BinaryTree`.
