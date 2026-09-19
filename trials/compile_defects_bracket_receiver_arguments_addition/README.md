# compile_defects_bracket_receiver_arguments_addition

One valid Silica program that today's type checker rejects (see `defect_bracket_receiver_arguments.silica`
and the `compile_defects_addition` README for why a compile-time defect gets a directory of its own).
`StubGraphUndirected.silica` is a minimal construction module under a name the checker registers for
`UndirectedGraph[...]` bindings; `PlusGraph.silica` is a trait with two implementations whose records
both carry the bracket's witnesses.

- `defect_bracket_receiver_arguments`: with two fitting implementations, an `UndirectedGraph[...]`
  value is rejected (E2003 "type mismatch") by a trait method whose next parameter is a placeholder
  (`id: KeyType`, like `UndirectedGraph@degree`). Expected: `5`, exit 0.

Move the trial to `ordered_data_structures/graph_collections` once the checker accepts it.
