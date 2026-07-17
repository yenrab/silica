# Inline struct trial applications

This directory contains trial applications demonstrating **inline record types** for Silica, as described in `silica-compiler&language-specification.jsonld` and `design_documents/silica-specification.md` (§3.4.2: no user-defined `type Name = …` or `struct Name { … }` for naming types).

## Design principles

In these trials, structs are:

- **Literal shapes only**: Types are written inline as `{ field: Type, … }` wherever a type is needed.
- **No type aliases**: Convenience names for whole types are expressed by **factory functions** (e.g. `create_item`, `point`) that return a fully written inline record type, not by `type … = …`.

## Trial apps

### 01_basic_struct_creation.silica

**Demonstrates:** Creating values with struct literals, field access, and inline types on locals and function signatures.

**Key concepts:** Inline struct creation; `.field` access; repeating the same inline shape in annotations.

---

### 02_struct_as_parameter.silica

**Demonstrates:** Parameters and return types as inline records; multiple fields; building new structs from old ones.

**Key concepts:** Struct-typed parameters; struct return types.

---

### 03_struct_in_list.silica

**Demonstrates:** `List[..., mem(normal)]` whose element type is an inline struct; folds over lists with struct patterns.

**Key concepts:** Lists of struct shapes; recursive walks; `empty` / `prepend` with explicit type arguments matching the element record.

---

### 04_nested_structs.silica

**Demonstrates:** Fields whose types are other inline record types; chained field access.

**Key concepts:** Nested `{ … }` types; `outer.inner.field`.

---

### 05_struct_pattern_matching.silica

**Demonstrates:** `case` on struct values with struct patterns and guards.

**Key concepts:** Struct patterns; wildcards; guards.

---

### 06_struct_in_sequence.silica

**Demonstrates:** Structs inside `sequence` blocks and list processing with effects.

**Key concepts:** Sequences with struct values; functional “updates” via new literals.

---

### 07_struct_with_list_fields.silica

**Demonstrates:** An inline struct whose **fields** are `List[..., mem(normal)]` values (not only “structs inside lists”).

**Key concepts:** Record types with multiple list-typed fields; building lists in `sequence`, then storing them in a struct literal; aggregating via `row.field`.

---

### 08_complex_struct_operations.silica

**Demonstrates:** Richer compositions—segments whose endpoints are points, merging range structs, summing a numeric property over a list of nested structs—**without** recursive named variant types (no `type Tree = …`).

**Key concepts:** Deeply nested inline records; list + fold; helpers that take and return only inline types.

---

### 09_struct_as_actor_message.silica

**Demonstrates:** Command/state/response shapes as inline structs for message-style workflows.

**Key concepts:** Structs as state and payloads; transitions driven by struct fields.

---

### 10_comprehensive_example.silica

**Demonstrates:** A small “order processing” flow using **only** inline record and list types (no `type Item = …` / `type Order = …`).

**Key concepts:** Repeating the same inline `List[{ … }]` element shape; helpers; `case` on structs and lists.

---

## Common patterns

### Inline record type (repeated at each use site)

```silica
fn factory(x: int64, y: int64) -> { x: int64, y: int64 } {
    p: { x: int64, y: int64 } <- { x: x, y: y };
    p
}
```

### Struct usage in functions

```silica
fn distance_squared(p: { x: int64, y: int64 }) -> int64 {
    p.x * p.x + p.y * p.y
}
```

### Pattern matching

```silica
case point of {
    { x: 0, y: 0 } -> true
    _ -> false
}
```

## Running the trials

Each file is a complete Silica program. Compile or run them individually with your Silica toolchain, for example:

```bash
silica-compiler 01_basic_struct_creation.silica
silica-compiler 02_struct_as_parameter.silica
```

## Notes

- Trials follow the rule that **record types are structural and inline**; they do not introduce `type` aliases for struct or variant shapes.
- When the same shape is used often, prefer a **named function** that returns that shape (and shared helpers) rather than a type alias.
