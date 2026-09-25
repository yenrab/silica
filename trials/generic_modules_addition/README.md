# generic_modules_addition

Trials for generic modules used across units (`ItemType` / `KeyType` / `ValueType` placeholders with a
witness record), where the caller and the generic module are separate units. Every unit without a
`main` is a helper linked into every main, so a trial is `<name>.silica` plus the module(s) it uses
(`gbox.silica`, `gen_rec.silica`, `gpass.silica`).

A generic unit keeps every `ItemType` value in one word: a scalar, or a pointer to a record or tuple.
Since 2026-09-18 every unit uses the same uniform boxed layout (header of
`compiler/silica-compiler/src_selfhost/emitter/*/terms/prims/prims_record.silica`): each record field
is one 8-byte slot and a record-, tuple-, list-, string- or function-typed field holds a pointer; a
list cell of a record or tuple element holds a pointer too. These trials check that values built on
one side of the generic boundary read back correctly on the other.

- `record_item_through_generic_result` (+ `gbox`): a record item stored by a generic writer and read
  back by a caller that knows it is a record. Expected `7`, `8`.
- `record_tuple_field_then_list_read` (+ `gen_rec`): a generic `{status, value: ItemType, path}` record
  read with int64 and then tuple items; `path` comes after the tuple field. Expected `7 3`, `7 70`, `3`.
- `record_item_through_generic_argument` (+ `gpass`): records built here (from a variable and from a
  nested literal) passed into generic readers of the ItemType field and the field after it.
  Expected `7`, `8`, `5`, `11`, `12`, `6`.
- `nested_record_item_through_generic` (+ `gbox`, `gpass`): an item that itself holds a nested record,
  round-tripped through two generic writers, read by projection and by a nested record pattern.
  Expected `1`, `2`, `3`, `4`, `3`, `9`, `4321`.
- `record_list_through_generic` (+ `gpass`): lists of records built here and walked, indexed and
  extended by the generic unit, and a list built by the generic unit read here with `.head`, `.tail`
  and `[h | t]`. Expected `3`, `2 20`, `0 5`, `4`, `65`, `7 70 8 80`.
- `tuple_item_list_through_generic` (+ `gpass`): the same for tuple items, plus a record field after a
  tuple-typed ItemType field. Expected `4 40 9`, `3`, `2 20`, `0 5`, `65`, `7 70 8 80`.

- `impl_item_type_return` (+ `StubTree.silica`, `stub_tree_core.silica`): trait impls whose ItemType-valued
  result (bare, record-wrapped, tuple-wrapped, or from a local helper) comes from another generic call, and a
  caller destructuring a generic `(ItemType, int64)` result directly. Formerly the open compile defect
  `compile_defects_impl_item_type_addition`. Expected `7`, `7`, `8`, `8`, `7`.

- `record_param_fields_around_nested_copy`: a record built from a record parameter's fields around a
  field that is copied to the region (a record variable, a nested record literal, a tuple variable), in
  functions with no other call. The copy's region_alloc clobbered the scratch base the parameter's later
  fields were read through (the pq_* faults in brodal_okasaki_priority@rebuild). Expected `1 6 7 4 5`,
  `1 4 5 4 5`, `1 8 9 4`, one number per line.

- `tuple_param_returned_as_is`: a tuple-returning function whose body is a parameter (bare, after a let
  spine, and called as a function value the way the generic graph modules call edge_target). The
  epilogue popped a return slab the body never pushed and returned to a garbage address (the identity
  edge_target of `ordered_data_structures/graph_live_core/gl_tuple_node_ids`). Expected `1 2`, `1 2`,
  `5 6`, one number per line.

- `two_generic_collections_round_trip` (+ `pair_sets`, `wbt_set`; formerly the open defect D13,
  found while writing docs/learn-silica.md §10, fixed 2026-09-19): a tuple holding two generic
  collections (`pair_sets@make_pair`, two `wbt_set` instances over different ItemTypes), returned
  across the `pair_sets`/`main` module boundary and passed back into `pair_sets@waiting`. Each
  `wbt_set@empty` result is rebuilt at the call site (the ordering-bundle merge) and so lives in
  `make_pair`'s frame region; the return copied the outer pair to the region but left both elements
  pointing into the popped frame, and `wbt_set@size` faulted. A let-bound record or tuple packed by
  name into a tuple is now copied to the region when its pointer is in the current frame
  (`term_aggregate_helpers@tuple_elem_frame_var_promote`). Expected `0 at start`, exit 0.

- `two_generic_collections_record_round_trip`: the record form of the same round trip
  (`pair_sets@make_record`/`waiting_record`, the `{ queue, seen }` shape of the original D13 report).
  A let-bound aggregate stored in a record field was always copied to the region at the store, so
  this form passed while the tuple form faulted; it stays as the regression guard. Expected `0 at
  start`, exit 0.

Every trial exits 0.
