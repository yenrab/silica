# Silica Intermediate Representation (SIR) Design Specification

## Related Documents

| Document | Purpose |
|----------|---------|
| [silica-compiler-creation-order.md](../silica-compiler-creation-order.md) | Pipeline overview; entry point for the compiler |
| [sir_optimization_spec.md](sir_optimization_spec.md) | Optimization passes and effect-aware rules |
| [sir_recursion_strategy.md](sir_recursion_strategy.md) | Recursion handling: tail calls, folds, explicit stack |

---

## 1. Introduction

### 1.1 Overview

SIR (Silica Intermediate Representation) is the intermediate representation used by the Silica compiler between the type-checked, trait-checked, and effect-checked AST and the code generation phase. SIR is designed to be:

- **Functional-design friendly**: Expression-oriented, with let-binding, case, and explicit control flow that mirrors Silica's functional semantics.
- **LLM-friendly**: Readable, consistent structure, explicit types and effects on every term, unambiguous syntax that reduces parsing and generation errors.
- **Optimization-friendly**: A normalized form suitable for constant folding, common subexpression elimination, inlining, tail-call handling, and pattern-match compilation.
- **AArch64-oriented**: SIR is designed for AArch64. It is not raw assembly; it is an optimization-friendly representation that reflects AArch64 semantics (regions, vectors, effects) and lowers to AArch64 instructions. Cross-chip support is not a goal; Silica targets AArch64 and its derivatives.

SIR is a custom IR designed for Silica's semantics and for human and LLM comprehension. LLVM is not used in the pipeline except possibly as the assembler for emitted assembly.

### 1.2 Design Principles

1. **Explicit types on every term**: Every binder and every term carries its type. No inference in the IR.
2. **Explicit effects on every call**: Every function call and every effectful primitive carries its effect set.
3. **Expression-oriented terms**: Terms are tree-shaped expressions; control flow is expressed via case and let, not basic blocks.
4. **Named identifiers**: Variables use readable names (e.g. `%x`, `%tmp_1`, `%merge_result`) rather than opaque numeric SSA IDs.
5. **Small, fixed term vocabulary**: A finite set of term constructors; no extension or implicit forms.
6. **Recursion and tail calls as terms**: Tail calls are explicit; recursion handling (jumps, folds, explicit stack) is defined in [sir_recursion_strategy.md](sir_recursion_strategy.md).
7. **Pattern matching as first-class**: Case expressions with variant, record, tuple, and literal patterns are IR terms; lowering to decision trees or jump tables is a later pass.

### 1.3 Scope

- **Input**: Type-checked, trait-checked, and effect-checked Silica AST.
- **Output**: Optimized SIR suitable for lowering to AArch64 assembly.
- **Out of scope**: Direct parsing of SIR from source (SIR is compiler-internal); binary format; debugging symbols.

---

## 2. SIR Structure

### 2.1 Hierarchy

```
SIRModule
  ├── SIRDecl* (type aliases, structs, enums, traits)
  └── SIRFunction*

SIRFunction
  ├── name: string
  ├── params: (name, type)*
  ├── return_type: SIRType
  ├── effects: SIREffectSet
  └── body: SIRTerm

SIRTerm
  └── (one of: const, var, let, case, call, tail_call, prim)
```

### 2.2 Module

A module is the top-level container. It contains:

- **Declarations**: Type aliases, struct definitions, enum definitions, trait definitions (for reference during optimization; no method bodies in IR).
- **Functions**: Each function has a name, parameter list with types, return type, effect set, and a single body term.

Module names are dot-separated identifiers (e.g. `main`, `stdlib.option`, `net.packet`).

### 2.3 Function

A function has:

- **Name**: Fully qualified (e.g. `main.main`, `stdlib.option.unwrap`).
- **Parameters**: Zero or more `(name, type)` pairs. Parameter names are unique within the function.
- **Return type**: The type of the function's result.
- **Effects**: The set of effects the function may perform (e.g. `[]`, `[mem(normal)]`, `[device_io, concurrency]`).
- **Body**: A single SIR term that is the function's body. The body must have type equal to the return type and must have effects that are a subset of the declared effects.

---

## 3. SIR Types

### 3.1 Type Grammar

SIR types mirror Silica's type system. All types are explicit.

```
SIRType ::=
  | int8
  | int16
  | int32
  | int64
  | float16
  | float32
  | float64
  | bool
  | char
  | string
  | unit
  | region(R, Space)
  | ref(R, Space, SIRType)
  | buf(R, Space, SIRType, N)
  | atomic_ref(R, Space, SIRType)
  | actor_ref
  | core_id
  | core_set
  | space
  | (SIRType, SIRType, ...)           -- tuple
  | { f1: SIRType, f2: SIRType, ... } -- record
  | VariantName                        -- enum variant (no payload or with payload)
  | Vec128Int8 | Vec128Int16 | Vec128Int32 | Vec128Int64 | Vec128Float32 | Vec128Bool
  | VecInt8 | VecInt16 | VecInt32 | VecInt64 | VecFloat16 | VecFloat32 | VecFloat64 | VecBool
  | Pred

Space ::= normal | normal_writeback | normal_writethrough | normal_noncacheable | atomic | device

R     ::= region identifier (e.g. R1, R2)

space ::= Space  -- type of space literals, used as argument to alloc_region

N     ::= integer literal (buffer size)
```

### 3.2 Variant Types

Variant types reference enum definitions. A variant has a tag and optionally a payload type:

- `OptionInt.None` — no payload
- `OptionInt.Some(int64)` — payload type int64
- `ResultIntString.Ok(int64)` — payload type int64
- `ResultIntString.Err(string)` — payload type string

### 3.3 Process Types

Process (function) types in Silica are represented as:

```
proc[effect_list] SIRType
```

In SIR, function types are stored as:

- Parameter types: `(SIRType, SIRType, ...)`
- Return type: `SIRType`
- Effect set: `SIREffectSet`

For example, a function `(int64, int64) -> int64 proc[]` is represented as params `(int64, int64)`, return `int64`, effects `[]`.

---

## 4. SIR Effects

### 4.1 Effect Set

An effect set is a list of zero or more effects:

```
SIREffectSet ::= [ Effect* ]

Effect ::=
  | mem(normal)
  | mem(normal_writeback)
  | mem(normal_writethrough)
  | mem(normal_noncacheable)
  | mem(atomic)
  | mem(device)
  | device_io
  | concurrency
  | mailbox
  | atomic
  | UserDefinedEffect
```

### 4.2 Effect Ordering

Effects are unordered within a set. Duplicates are not allowed. Subeffecting is defined as in the Silica specification:

- `[] <: E` for any effect set E
- `[e1] <: [e1, e2]` — subset relation
- `mem(normal_writeback) <: mem(normal)` (and similarly for other normal variants)
- `mem(normal) <: mem(atomic)` — atomic subsumes normal

### 4.3 Effect Annotation

Every `call` and `tail_call` term carries an effect set indicating the effects of the callee. Every effectful primitive (`alloc_region`, `alloc_ref`, `read_ref`, `write_ref`, `spawn`, etc.) carries its effect set.

---

## 5. SIR Terms

### 5.1 Term Grammar

```
SIRTerm ::=
  | const(SIRType, value)
  | var(name)
  | let(name, SIRType, SIRTerm, SIRTerm)
  | case(SIRType, SIRTerm, SIRCaseBranch+)
  | call(SIRType, callee, SIRTerm*, SIREffectSet)
  | tail_call(callee, SIRTerm*)
  | prim(SIRType, PrimOp, SIRTerm*)
```

### 5.2 Term Constructors

#### 5.2.1 const(SIRType, value)

A constant value. The type must match the value.

- `const(int64, 42)`
- `const(bool, true)`
- `const(float64, 3.14)`
- `const(unit, ())`
- `const(string, "hello")`
- `const(space, normal)` — space literal for alloc_region

#### 5.2.2 var(name)

Reference to a bound variable. The variable must be in scope at this point. Name format: `%name` where `name` is an identifier (e.g. `%x`, `%tmp_1`, `%merge`).

#### 5.2.3 let(name, SIRType, SIRTerm, SIRTerm)

Let binding: evaluate the first term, bind the result to `name` with the given type, then evaluate the second term in the extended scope. The second term is the continuation.

- `let(%x, int64, const(int64, 42), add(%x, const(int64, 1)))` — bind 42 to `%x`, then add 1.

#### 5.2.4 case(SIRType, SIRTerm, SIRCaseBranch+)

Pattern matching. Evaluate the scrutinee term, then match against the branches in order. The first matching branch is executed. The result type is the type of all branch bodies.

- `case(int64, var(%opt), ...)` — match on `%opt` with type `OptionInt`, branches produce `int64`.

#### 5.2.5 call(SIRType, callee, SIRTerm*, SIREffectSet)

Function call. Evaluate the argument terms, call the callee. Callee is a fully qualified function name (e.g. `main.add`, `stdlib.option.unwrap`). The effect set is the callee's declared effects.

- `call(int64, main.add, [var(%a), var(%b)], [])`
- `call(ref(R1, normal, int64), main.alloc_int, [var(%r)], [mem(normal)])`

#### 5.2.6 tail_call(callee, SIRTerm*)

Tail call. Same as call but the result is returned directly to the caller's caller. No continuation is needed. The callee's return type must match the current function's return type.

- `tail_call(main.factorial, [var(%n_minus_1)])`

#### 5.2.7 prim(SIRType, PrimOp, SIRTerm*)

Primitive operation. See Section 6.

---

## 6. SIR Case Branches

### 6.1 Branch Grammar

```
SIRCaseBranch ::= SIRPattern -> SIRTerm

SIRPattern ::=
  | literal_pattern(SIRType, value)
  | var_pattern(name, SIRType)
  | tuple_pattern(SIRPattern*)
  | record_pattern(SIRType, (field_name, SIRPattern)*)
  | variant_pattern(VariantName, SIRPattern?)
  | list_nil_pattern(SIRType)
  | list_cons_pattern(name, SIRType, name, SIRType)
  | wildcard_pattern(SIRType)
```

### 6.2 Pattern Kinds

- **literal_pattern**: Matches a constant (e.g. `literal_pattern(int64, 0)`).
- **var_pattern**: Binds the scrutinee to a variable (e.g. `var_pattern(%n, int64)`).
- **tuple_pattern**: Matches a tuple and patterns for each element (e.g. `tuple_pattern(var_pattern(%a, int64), var_pattern(%b, int64))`).
- **record_pattern**: Matches a record by field names and patterns for each field.
- **variant_pattern**: Matches an enum variant (e.g. `variant_pattern(OptionInt.Some, var_pattern(%x, int64))`).
- **list_nil_pattern**: Matches the empty list.
- **list_cons_pattern**: Matches a cons cell, binding head and tail (e.g. `list_cons_pattern(%h, int64, %t, List_int64)`).
- **wildcard_pattern**: Matches any value of the given type; no binding (e.g. `wildcard_pattern(int64)`).

### 6.3 Guard

Guards are optional. A branch may have a guard:

```
SIRCaseBranch ::= SIRPattern [if SIRTerm] -> SIRTerm
```

The guard term must have type `bool`. If the pattern matches, the guard is evaluated; if true, the branch body runs; if false, the next branch is tried.

---

## 7. SIR Primitives

### 7.1 Arithmetic

| PrimOp | SIRType | Args | Effect |
|--------|---------|------|--------|
| add | int8, int16, int32, int64, float32, float64 | (a, b) | [] |
| sub | int8, int16, int32, int64, float32, float64 | (a, b) | [] |
| mul | int8, int16, int32, int64, float32, float64 | (a, b) | [] |
| div | int8, int16, int32, int64, float32, float64 | (a, b) | [] |
| rem | int8, int16, int32, int64 | (a, b) | [] |
| neg | int8, int16, int32, int64, float32, float64 | (a) | [] |

### 7.2 Comparison

| PrimOp | SIRType | Args | Effect |
|--------|---------|------|--------|
| eq | int8, int16, int32, int64, float32, float64, bool, char | (a, b) | [] |
| ne | int8, int16, int32, int64, float32, float64, bool, char | (a, b) | [] |
| lt | int8, int16, int32, int64, float32, float64 | (a, b) | [] |
| le | int8, int16, int32, int64, float32, float64 | (a, b) | [] |
| gt | int8, int16, int32, int64, float32, float64 | (a, b) | [] |
| ge | int8, int16, int32, int64, float32, float64 | (a, b) | [] |

### 7.3 Logical

| PrimOp | SIRType | Args | Effect |
|--------|---------|------|--------|
| and | bool | (a, b) | [] |
| or | bool | (a, b) | [] |
| not | bool | (a) | [] |

### 7.4 Memory

| PrimOp | SIRType | Args | Effect |
|--------|---------|------|--------|
| alloc_region | region(R, Space) | (space) | [mem(Space)] |
| alloc_ref | ref(R, Space, T) | (region, value) | [mem(Space)] |
| read_ref | T | (ref) | [mem(Space)] |
| write_ref | unit | (ref, value) | [mem(Space)] |
| alloc_buf | buf(R, Space, T, N) | (region, N) | [mem(Space)] |
| buf_load | T | (buf, index) | [mem(Space)] |
| buf_store | unit | (buf, index, value) | [mem(Space)] |

### 7.5 Atomic

| PrimOp | SIRType | Args | Effect |
|--------|---------|------|--------|
| atomic_load | T | (atomic_ref) | [mem(atomic)] |
| atomic_store | unit | (atomic_ref, value) | [mem(atomic)] |
| atomic_add | T | (atomic_ref, value) | [mem(atomic)] |
| atomic_sub | T | (atomic_ref, value) | [mem(atomic)] |
| atomic_cas | (bool, T) | (atomic_ref, expected, desired) | [mem(atomic)] |

### 7.6 Tuple and Record

| PrimOp | SIRType | Args | Effect |
|--------|---------|------|--------|
| tuple_proj | T | (tuple, index) | [] |
| record_proj | T | (record, field_name) | [] |
| tuple_make | (T1, T2, ...) | (v1, v2, ...) | [] |
| record_make | { f1: T1, f2: T2, ... } | (field_name, value)* | [] |

### 7.7 Variant

| PrimOp | SIRType | Args | Effect |
|--------|---------|------|--------|
| variant_tag | int64 | (variant) | [] |
| variant_payload | T | (variant) | [] |
| variant_make | VariantName | (tag, payload?) | [] |

### 7.8 List

| PrimOp | SIRType | Args | Effect |
|--------|---------|------|--------|
| list_nil | List[T] | (T) | [] |
| list_cons | List[T] | (head, tail) | [] |
| list_is_nil | bool | (list) | [] |
| list_head | T | (list) | [] |
| list_tail | List[T] | (list) | [] |

### 7.9 Actor

| PrimOp | SIRType | Args | Effect |
|--------|---------|------|--------|
| spawn | actor_ref | (initial_state, behavior, core_hint?) | [concurrency] |
| send | unit | (actor_ref, message) | [concurrency, mailbox] |
| recv | T | (mailbox) | [concurrency, mailbox] |

### 7.10 String

| PrimOp | SIRType | Args | Effect |
|--------|---------|------|--------|
| string_concat | string | (a, b) | [] |
| string_length | int64 | (s) | [] |
| string_eq | bool | (a, b) | [] |

### 7.11 Device I/O

| PrimOp | SIRType | Args | Effect |
|--------|---------|------|--------|
| print | unit | (string) | [device_io] |
| println | unit | (string) | [device_io] |
| print_int64 | unit | (int64) | [device_io] |
| read_line | string | () | [device_io] |

### 7.12 Cast

| PrimOp | SIRType | Args | Effect |
|--------|---------|------|--------|
| cast | T | (value, source_type) | [] |

### 7.13 Vector (NEON / SVE)

| PrimOp | SIRType | Args | Effect |
|--------|---------|------|--------|
| vec_load | Vec128T / VecT | (ref, index) | [] |
| vec_store | unit | (ref, index, vec) | [] |
| vec_add | Vec128T / VecT | (a, b) | [] |
| vec_sub | Vec128T / VecT | (a, b) | [] |
| vec_mul | Vec128T / VecT | (a, b) | [] |
| vec_cmp_eq | Vec128Bool / VecBool | (a, b) | [] |
| vec_cmp_lt | Vec128Bool / VecBool | (a, b) | [] |
| vec_select | Vec128T / VecT | (mask, a, b) | [] |

---

## 8. SIR Text Format

### 8.1 Lexical Conventions

- **Identifiers**: `[a-zA-Z_][a-zA-Z0-9_]*` for names; `%` prefix for variables (e.g. `%x`, `%tmp_1`).
- **Module paths**: Dot-separated identifiers (e.g. `main`, `stdlib.option`).
- **Integer literals**: Decimal `123`, hexadecimal `0x1A`, binary `0b1010`.
- **Float literals**: `3.14`, `1e-10`.
- **String literals**: `"hello"` with escapes `\"`, `\\`, `\n`, `\t`.
- **Comments**: `--` to end of line; `{--` and `--}` for block comments.

### 8.2 Module Syntax

```
module module_name

decl type_alias_name = type
decl struct StructName { field1: type1; field2: type2; ... }
decl enum EnumName { Variant1; Variant2(T); ... }
decl trait TraitName { ... }

fn qualified_name(param1: type1, param2: type2, ...) -> return_type effects [e1, e2, ...] {
  term
}
```

### 8.3 Term Syntax

```
const type value
var %name
let %name: type = term in term
case scrutinee: type of
  pattern -> term
  pattern if guard -> term
  ...
call type callee(arg1, arg2, ...) effects [e1, e2, ...]
tail_call callee(arg1, arg2, ...)
prim type op(arg1, arg2, ...)
```

### 8.4 Pattern Syntax

```
42                    -- literal
%name: type           -- variable
_ : type              -- wildcard
(VariantName, arg)    -- variant with payload
VariantName           -- variant without payload
(f1 = p1, f2 = p2)    -- record
(p1, p2, ...)         -- tuple
[]                    -- list nil
[%head: T | %tail: ListT]  -- list cons
```

### 8.5 Full Example

```
module main

fn main.add(%a: int64, %b: int64) -> int64 effects [] {
  prim int64 add (%a, %b)
}

fn main.factorial(%n: int64) -> int64 effects [] {
  let %is_zero: bool = prim bool eq (%n, const int64 0) in
  case %is_zero: bool of
    true -> const int64 1
    false ->
      let %n_minus_1: int64 = prim int64 sub (%n, const int64 1) in
      let %rec: int64 = call int64 main.factorial (%n_minus_1) effects [] in
      prim int64 mul (%n, %rec)
}

fn main.option_demo(%opt: OptionInt) -> int64 effects [] {
  case %opt: OptionInt of
    OptionInt.None -> const int64 0
    OptionInt.Some(%x: int64) -> var %x
}
```

---

## 9. IR Construction

### 9.1 Input

- Type-checked AST: All types resolved, all names resolved, trait implementations verified.
- Effect-checked AST: All effects declared and validated.

### 9.2 Construction Rules

1. **Expression to term**: Each AST expression is transformed to a SIR term. Nested expressions become nested terms or let bindings.
2. **Statement to term**: Sequential statements (e.g. in a `do` block) become let bindings: `stmt1; stmt2` becomes `let %_ = term1 in term2`.
3. **Case to case**: AST `case` becomes SIR `case` with patterns translated to SIR patterns.
4. **Call to call**: AST function call becomes `call` or `tail_call` term. Tail-call detection: if the call is the last expression in a branch (with no effect after it), emit `tail_call`.
5. **Primitive mapping**: AST primitive operations (+, -, *, /, alloc_ref, etc.) become `prim` terms with the corresponding PrimOp.
6. **Variable names**: Generate unique names for temporaries (e.g. `%tmp_1`, `%tmp_2`). Preserve user variable names when possible (e.g. `%x`, `%result`).

### 9.3 A-Normal Form (Optional)

For optimization passes that benefit from a flat structure, SIR can be converted to A-normal form: every non-trivial subterm is let-bound. For example:

```
prim int64 add (prim int64 mul (%a, %b), const int64 1)
```

Becomes:

```
let %t1: int64 = prim int64 mul (%a, %b) in
let %t2: int64 = prim int64 add (%t1, const int64 1) in
var %t2
```

---

## 10. IR Lowering

### 10.1 Lowering to Machine Code

SIR is lowered to AArch64 assembly in the following conceptual phases:

1. **Case to decision tree**: Compile `case` terms to a sequence of tag tests, field tests, and branches. Dense integer/enum variants may use jump tables.
2. **Let to basic blocks**: Flatten let bindings into basic blocks; each block has a sequence of assignments and a terminator (branch, jump, return, tail call).
3. **Recursion handling**: Tail calls emit jumps; non-tail recursion uses fold lowering or explicit-stack lowering. See [sir_recursion_strategy.md](sir_recursion_strategy.md).
4. **Register allocation**: Assign SIR variables to physical registers or stack slots.
5. **Instruction selection**: Map `prim` operations to target instructions (e.g. AArch64 ADD, LDR, STR).
6. **Emission**: Emit AArch64 assembly. An external assembler (e.g. LLVM's assembler or GNU as) may be used to produce object files.

---

## 11. Optimization Passes

### 11.1 Pass Order

1. **Constant folding**: Evaluate `const` and `prim` when all operands are constants.
2. **Constant propagation**: Replace `var(%x)` with the constant value when `%x` is known to hold a constant.
3. **Common subexpression elimination**: When the same term appears twice with the same inputs, compute once and reuse.
4. **Dead code elimination**: Remove let bindings whose variable is never used.
5. **Inlining**: Replace `call` with the callee body when the callee is small or the call is hot.
6. **Tail-call handling**: Codegen emits jump for `tail_call`; see [sir_recursion_strategy.md](sir_recursion_strategy.md).
7. **Guard hoisting**: In case expressions, move guard evaluations that do not depend on pattern bindings earlier.
8. **Case compilation**: Compile case to decision tree or jump table.

### 11.2 Effect-Aware Optimization

- Inlining: Only inline when the caller's effect set includes the callee's effects.
- Reordering: Do not reorder effectful terms relative to each other.

---

## 12. Identifier and Scoping Rules

### 12.1 Variable Scope

- Variables are bound by `let` and by `var_pattern`/`list_cons_pattern` in case branches.
- Scope is lexical: a variable is visible from its binding to the end of the enclosing term.
- Shadowing: A nested `let` may reuse a name; the inner binding shadows the outer.

### 12.2 Function Names

- Function names are fully qualified: `module.submodule.function_name`.
- Callee in `call` and `tail_call` is a string (e.g. `"main.factorial"`).

### 12.3 Region Names

- Region identifiers (R, R1, R2) are unique within a function or module. They are assigned during IR construction from the AST's region analysis.

---

## 13. Validation Rules

### 13.1 Type Validation

- Every `var` refers to a variable whose type matches the expected type at that use.
- Every `let` binds a term whose type matches the declared type.
- Every `call` returns a value of the declared return type; argument types match parameter types.
- Every `case` branch body has the same type as the case result type.
- Every `prim` application has the correct number and types of arguments.

### 13.2 Effect Validation

- Every `call` and `tail_call` has an effect set that is a subset of the current function's declared effects.
- Every `prim` that requires effects is only used when the function's effects include those effects.

### 13.3 Exhaustiveness

- Case expressions must be exhaustive: for enum types, all variants must be covered (or a wildcard used); for integer types, a catch-all or full range must be present.

---

## 14. SIR File Extension

SIR text files use the extension `.sir` for disambiguation from Silica source (`.silica`) and other formats.

---

## 15. Revision History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-02-10 | Initial design specification |
