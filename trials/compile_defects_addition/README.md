# compile_defects_addition

Valid Silica programs that today's compiler rejects. Each one fails the directory's compile step, so
there is one trial per directory (a sibling suite for the next one). Move a trial to the suite that
matches its topic once the compiler accepts it and its behaviour matches the golden.

- `defect_tuple_pattern_named_binding`: `(:ok, n: int64) -> ...` in a tuple case pattern is E2002
  "undefined identifier: n". Expected: `cannot divide by zero`, `20`, exit 0.

Sibling directories built the same way: `compile_defects_priority_type_addition` (PriorityType never
substituted in composite types, E2003) and `compile_defects_item_type_addition` (ItemType resolves to the
first bracket parameter for PriorityQueue receivers, E2001), both from the Heap/PriorityQueue work.
