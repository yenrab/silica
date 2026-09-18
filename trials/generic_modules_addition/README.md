# generic_modules_addition

Trials for generic modules used across units (`ItemType` / `KeyType` / `ValueType` placeholders with a
witness record), where the caller and the generic module are separate units. Every unit without a
`main` is a helper linked into every main, so a trial is `<name>.silica` plus the module(s) it uses.

- `emitter_defect_record_item_through_generic_result` (+ `gbox.silica`): documented open emitter defect.
  A generic writer stores a record-typed ItemType field as one pointer word; a caller that knows the
  field is a record reads it with the inline nested-record layout. Expected `7`, `8`, exit 0; today it
  prints a pointer-like value and a stray word. No `.ascomp` until fixed.
- `emitter_defect_record_tuple_field_then_list_read` (+ `gen_rec.silica`): documented open emitter defect.
  A generic `{status, value: ItemType, path: List[int64]}` record read in a concrete unit with a tuple
  ItemType reads `path` at the inline-pair offset and faults. Expected `7 3`, `7 70`, `3`, exit 0.
