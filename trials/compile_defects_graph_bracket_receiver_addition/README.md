# compile_defects_graph_bracket_receiver_addition

One valid Silica program that today's type checker rejects (see `defect_graph_bracket_receiver.silica`
and the `compile_defects_addition` README for why a compile-time defect gets a directory of its own).
`StubGraphDirected.silica` is a minimal construction module under a name the checker registers for
`DirectedGraph[...]` bindings; `ProbeDirected.silica` is a one-method trait over its record.

- `defect_graph_bracket_receiver`: a `DirectedGraph[int64, int64, mem(normal)]` value is rejected by a
  trait call on its own constructor's record (E2003): the bracket-to-record fit compares the record's
  `edge_target: fn(ValueType) -> KeyType` without placeholder acceptance. `WeightedGraph[...]` values
  fail the same way through `edge_weight`. Expected: `3`, exit 0.

Move the trial to `ordered_data_structures/graph_collections` once the checker accepts it.
