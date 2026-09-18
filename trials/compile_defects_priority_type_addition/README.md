# compile_defects_priority_type_addition

One valid Silica program that today's type checker rejects (see `defect_priority_type_composite.silica` and the
`compile_defects_addition` README for why a compile-time defect gets a directory of its own).
`StubPriorityQueue.silica` is the minimal construction module the trial needs. Expected output: `5`, exit 0.
Move the trial to `ordered_data_structures/heap_collections` once the checker accepts it.
