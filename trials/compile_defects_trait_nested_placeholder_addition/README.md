# compile_defects_trait_nested_placeholder_addition

One valid Silica program that today's trait checker rejects (see `defect_trait_nested_placeholder.silica`
and the `compile_defects_addition` README for why a compile-time defect gets a directory of its own).
`NestedBox.silica` is the trait, `NestedStore.silica` the generic reader its implementation forwards to.

- `defect_trait_nested_placeholder`: an implementation matches a required placeholder only at the top
  level of a type, so `List[{ to: KeyType, data: ValueType }, mem(normal)]` does not match the
  required `List[ValueType, mem(normal)]` (E2092). This is why the weighted directed live graph
  (`graph_weighted`, whose DirectedGraph edge payload is the generated `{to, data}` view) does not
  implement `DirectedGraph`. Expected: `7`, `x`, exit 0.

Move the trial to `ordered_data_structures/graph_collections` once the checker accepts it, and add the
`graph_weighted` DirectedGraph implementation.
