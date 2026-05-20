# Silica External Library Call Wrapper Specification

## Related Documents

| Document                             | Purpose                                                                  |
| ------------------------------------ | ------------------------------------------------------------------------ |
| `silica-specification.md`            | Core language syntax, effects, actors, regions, and compiler diagnostics |
| `silica-specification-additional.md` | Additional compile-time failure rules                                    |

---

## 1. Introduction

### 1.1 Overview

This specification defines the rules for outbound calls from Silica to external libraries through C-compatible wrapper functions.

This specification applies only to calls from Silica to C libraries and to other language libraries that expose a C-compatible interface. Underlying external libraries may be provided as dynamically linked shared libraries or as statically linked archive/object libraries. Calls from C or other languages into Silica are out of scope and are specified in a separate inbound-interop specification.

Silica does not call arbitrary external APIs directly. Every external operation callable from Silica must be exposed through a Silica-compatible C wrapper function. The wrapper function adapts the original external API into a stable, explicit ABI boundary that the Silica compiler can type-check, effect-check, and validate.

### 1.2 Design Principles

- **Wrapper-First External Calls**: External libraries are adapted through wrapper functions before Silica calls them.
- **Explicit Danger Boundary**: Every module that declares or exposes foreign functions, or that imports or otherwise uses any module whose name begins with `dangerous_`, must itself use the `dangerous_` module-name prefix. That naming requirement propagates along the module dependency graph to the root application module, so the compiled application name carries `dangerous_` whenever the program depends—directly or transitively—on any `dangerous_*` module. A module that never depends on a `dangerous_*` module is not required to use the prefix.
- **Actor-Scoped External Calls**: External calls are permitted only inside the behavior function passed directly to `spawn`.
- **Explicit Effect Tracking**: Calls to `dangerous_*` modules are required to appear only in the sequence portion of `sequence proc[external_danger] ... produces pure ... end`.
- **No Retained Dangerous Data**: A completed `external_danger` sequence produces only pure Silica values.
- **Strong Typing at the Boundary**: Wrapper code must translate C values into concrete Silica-compatible types.
- **No Raw Pointer Exposure**: Raw pointers, `void *`, and opaque C structs must not be exposed directly to Silica.
- **Non-Recursive Data Only**: Recursive C struct shapes must not be sent to Silica.
- **Outbound Only**: This specification does not define callbacks, trampolines, exported Silica functions, or external calls into Silica.

### 1.3 Scope

This specification defines:

1. C wrapper function requirements.
2. Silica `dangerous_*` module requirements, including naming rules that cascade along module dependencies to the root application module and compiled application name.
3. The `external_danger` effect.
4. Rules for strings, buffers, pointers, arrays, and de-opaqueified C structs.
5. Restrictions on dangerous data crossing actor and effect boundaries.
6. Compile-time, parser, type-check, and wrapper-validation failures.

This specification does not define:

1. C calling into Silica.
2. Callbacks from C into Silica.
3. Function-pointer trampolines.
4. Exported Silica functions callable by external libraries.
5. Inbound runtime initialization from external languages.

---

## 2. Terminology

### 2.1 External operation

An **external operation** is any operation implemented outside Silica and callable from Silica.

### 2.2 C wrapper function

A **C wrapper function** is a C ABI function written specifically to be called by Silica. It may call one or more functions from an underlying C library or from another language library that exposes a C-compatible interface.

The C wrapper function is the Silica-facing ABI adapter. It is not required to match the original library function signature.

### 2.3 Raw foreign binding

A **raw foreign binding** is a Silica declaration that names a C wrapper symbol and gives it a Silica type.

A raw foreign binding has no Silica body.

### 2.4 Dangerous module

A **dangerous module** is a Silica module whose name begins with `dangerous_`.

Any module that declares, exposes, or calls a raw foreign binding must be a `dangerous_*` module.

Any module that imports or otherwise uses any `dangerous_*` module—whether it calls adapter wrappers, re-exports symbols, or depends on types defined there—must also be a `dangerous_*` module. That rule applies transitively: if module `B` is `dangerous_*` and module `A` uses `B`, then `A` must be `dangerous_*`; any module that uses `A` must likewise be `dangerous_*`, up to and including the root application module. The root application module therefore uses the `dangerous_` prefix in its declared module name whenever its dependency closure contains any `dangerous_*` module, and the toolchain derived compiled name for the application reflects that prefix.

No module, including the root application module, is required to include `dangerous_` in its name when no module it depends on—directly or transitively—is a `dangerous_*` module.

### 2.5 External-danger-touched data

A value is **external-danger-touched** when it is returned from a `dangerous_*` module or structurally contains a value returned from a `dangerous_*` module.

The property is structural. Records, tuples, lists, sums, buffers, and nested values containing external-danger-touched data are themselves external-danger-touched.

### 2.6 De-opaqueification

**De-opaqueification** is the process by which wrapper code translates an opaque C struct or opaque external object into explicit Silica-compatible contents.

Silica does not receive the opaque object itself. Silica receives concrete data derived from the object.

---

## 3. Program Structure and Module Rules

### 3.1 Dangerous module naming rule

**Rule (foreign declarations)**: A Silica module that declares or exposes one or more foreign functions must have a name beginning with `dangerous_`.

**Rule (dangerous dependency)**: A Silica module that imports or otherwise uses any module whose name begins with `dangerous_` must itself have a name beginning with `dangerous_`. This applies regardless of whether the importing module declares foreign functions of its own. The requirement cascades along dependencies: the root application module must use the `dangerous_` prefix whenever any module in its dependency closure is `dangerous_*`, so the compiled artifact name for the application reflects that dangerous indicator.

**Rule (optional prefix)**: No module is required to use the `dangerous_` prefix when its transitive module dependencies include no `dangerous_*` module.

Valid:

```silica
module dangerous_net;
```

Invalid:

```silica
module net;
```

Invalid:

```silica
module app;

use dangerous_net;
```

**Compiler failure**:

```text
DangerousModuleNameError:
Modules that declare or expose foreign functions must use the dangerous_ prefix.
```

**Compiler failure**:

```text
DangerousDependencyNamingError:
A module that imports or uses a dangerous_* module must use the dangerous_ prefix in its own module name.
```

### 3.2 Foreign declaration syntax

This specification adds the following declaration form:

```silica
foreign c_wrapper "symbol_name"
fn local_name(arg1: Type1, arg2: Type2) -> ReturnType;
```

Example:

```silica
module dangerous_legacy_math;

foreign c_wrapper "silica_legacy_math_add_int64"
fn add_raw(left: int64, right: int64) -> int64;
```

A foreign declaration binds a C wrapper symbol to a Silica function name in the current module.

### 3.3 Export rule

**Rule**: Every exported function from a `dangerous_*` module must be a Silica adapter wrapper.

Raw foreign bindings must not be exported directly to application code.

A Silica adapter wrapper is a Silica function that calls one or more raw foreign bindings and returns a Silica-facing value that conforms to this specification.

Any exported adapter wrapper from a `dangerous_*` module is part of the danger boundary. It may be called only under the `external_danger` placement and effect rules in §4.

Example:

```silica
module dangerous_legacy_math;

export add/2;

foreign c_wrapper "silica_legacy_math_add_int64"
fn add_raw(left: int64, right: int64) -> int64;

fn add(left: int64, right: int64) -> int64 {
    add_raw(left, right)
}
```

The exported name `add/2` is callable only inside the sequence portion of a valid `sequence proc[external_danger] ... produces pure ... end` block.

The raw foreign binding `add_raw/2` is not exported.

---

## 4. The `external_danger` Effect

### 4.1 Effect declaration

This specification adds the following effect:

```silica
external_danger
```

### 4.2 Required effect rule

**Rule**: A call to any function in any `dangerous_*` module must appear in the sequence portion of an enclosing `sequence proc[external_danger] ... produces pure ... end` block.

This rule applies to:

- raw foreign bindings called by adapter wrappers inside `dangerous_*` modules;
- Silica adapter functions declared in `dangerous_*` modules;
- helper functions exposed by `dangerous_*` modules;
- functions that return only pure Silica values but reside in a `dangerous_*` module.

Valid:

```silica
sequence proc[external_danger]
    parsed: Ok(int64) | Error(int64) <- dangerous_net@parse_port(msg.text);
produces
    pure parsed
end
```

Invalid:

```silica
parsed: Ok(int64) | Error(int64) <- dangerous_net@parse_port(text);
```

**Parser failure**:

```text
DangerousModuleCallError:
Calls to dangerous_* module functions must appear in the sequence portion of sequence proc[external_danger] ... produces pure ... end.
```

### 4.3 Actor placement rule

**Rule**: A `sequence proc[external_danger] ... produces pure ... end` block is valid only when it appears directly inside the function passed to `spawn` as the actor behavior function.

The actor behavior may be written as a function literal at the `spawn` call site or as a named top-level function passed directly to `spawn`. In both cases, the function containing the `external_danger` sequence must be the actor behavior function supplied to `spawn`.

Valid:

```silica
fn main() -> int64 {
    sequence proc[concurrency]
        worker_ref: actor_ref <- spawn(
            { count: 0 },
            fn(msg: { text: string }, state: { count: int64 }) -> { count: int64 } {
                sequence proc[external_danger]
                    parsed: Ok(int64) | Error(int64) <- dangerous_net@parse_port(msg.text);
                produces
                    pure case parsed of {
                        Ok(port: int64) -> { count: state.count + port };
                        Error(code: int64) -> state;
                    }
                end
            }
        );
    produces
        pure 0
    end
}
```

Invalid:

```silica
fn parse_from_regular_function(text: string) -> Ok(int64) | Error(int64) {
    sequence proc[external_danger]
        parsed: Ok(int64) | Error(int64) <- dangerous_net@parse_port(text);
    produces
        pure parsed
    end
}
```

**Parser failure**:

```text
ExternalDangerPlacementError:
external_danger sequence blocks are only valid directly inside the function spawned for an actor.
```

### 4.4 Disallowed placements

The parser must reject an `external_danger` sequence block in any of the following positions:

- inside an ordinary top-level function body;
- inside a helper function;
- inside a function literal that is not passed directly to `spawn` as the actor behavior;
- inside a named function that is not passed directly to `spawn` as the actor behavior;
- inside a nested expression that is not the direct body of the spawned actor behavior function;
- inside any non-actor context.

### 4.5 Interaction with other effects

An actor behavior function is permitted to contain a sequence block tagged only with `external_danger` when the block performs only dangerous module calls and pure computation.

If the actor behavior function performs other effects, the sequence block is permitted to include additional effects only when those effects are valid in actor behavior functions and are not restricted by this specification.

The following effects are restricted for external-danger-touched data:

```silica
device_io
network_io
hot_swap
register_rwr
```

See §7.3.

---

## 5. C Wrapper Header Requirements

### 5.1 Header structure

A C wrapper header intended for Silica must be macro-free.

The header must not use Silica-specific C preprocessor macros, annotation macros, ABI-version macros, or generated binding macros.

Example macro-free header shape:

```c
#include <stdint.h>
#include <stddef.h>

int64_t silica_legacy_math_add_int64(int64_t left, int64_t right);
```

If a project uses ordinary C include guards or `#pragma once`, those mechanisms must not encode Silica ABI metadata and must not be required by the Silica binding generator. All Silica-specific metadata must be supplied outside the C preprocessor macro system as described in §13.

### 5.2 Symbol naming

Wrapper function symbols must use the following naming convention:

```text
silica_<module>_<function>
```

Examples:

```c
int64_t silica_legacy_math_add_int64(int64_t left, int64_t right);
silica_i64_result silica_net_parse_port(const uint8_t *text_ptr, uint64_t text_len);
```

The Silica compiler or binding generator is permitted to use this convention to infer default module and function names. Explicit binding declarations are permitted to override the local Silica name.

### 5.3 C linkage

Wrapper functions must use C linkage.

When compiled as C++, declarations must be enclosed in:

```c
#ifdef __cplusplus
extern "C" {
#endif

/* declarations */

#ifdef __cplusplus
}
#endif
```

### 5.4 Supported scalar ABI types

Silica-facing wrapper declarations must use fixed-width integer types from `<stdint.h>`.

Allowed integer types:

```c
int8_t
int16_t
int32_t
int64_t
uint8_t
uint16_t
uint32_t
uint64_t
```

Allowed floating-point types:

```c
float
double
```

`float` maps to `float32`.

`double` maps to `float64`.

### 5.5 Disallowed C ABI types

The following C types must not appear in Silica-facing wrapper declarations:

```c
char
short
int
long
long long
size_t
ptrdiff_t
_Bool
bool
void *
```

These types are permitted inside the wrapper implementation when calling the underlying external library.

Because Silica is strongly typed, wrapper code must translate any `void *` value from the underlying C-compatible library into one of the concrete Silica-facing representations allowed by this specification before the value reaches a Silica foreign binding.

### 5.6 Boolean representation

Silica `boolean` values must be represented at the wrapper ABI boundary as `uint8_t`.

Required encoding:

```text
0 = false
1 = true
```

A wrapper receiving a boolean argument is required to reject values other than `0` or `1` when the function is reachable from unchecked or external code.

---

## 6. Boundary Data Rules

### 6.1 Strings sent from Silica to a wrapper

Silica strings passed to C wrapper functions are never passed as references to the original Silica string storage.

For every Silica `string` argument passed to a C wrapper function, the runtime presents the wrapper with a copy of the string's memory.

The wrapper receives the copied string as a pointer-plus-length pair:

```c
const uint8_t *text_ptr,
uint64_t text_len
```

The copied memory resides in the expandable stack of the encompassing Silica actor. It is not allocated in the general heap.

The original Silica string is not modifiable from C. The C wrapper must treat `text_ptr` as read-only memory.

The wrapper must not assume the input is NUL-terminated.

The wrapper must not retain `text_ptr` after returning. The copied string memory is recovered when the foreign function call ends.

If the external library needs to retain the string data after the wrapper returns, the wrapper must copy the data into library-owned or wrapper-owned memory before returning.

### 6.2 Buffers

Mutable buffers must be represented explicitly as pointer-plus-length pairs.

A wrapper that writes into a buffer must document:

- whether the pointer may be null;
- the maximum number of bytes written;
- whether the output is initialized on failure;
- whether the buffer is retained after return.

### 6.3 C pointer arguments

C pointer arguments in wrapper declarations must be concrete and typed.

A C pointer argument must not be exposed to Silica as:

- an untyped pointer;
- an integer address;
- a `void *`;
- an ordinary scalar value.

The wrapper signature and Silica foreign declaration must agree on the concrete data shape being passed.

### 6.4 C pointer return values

For C function return values, C arrays and non-array C pointers have different Silica mappings.

A C return value that represents an array must be mapped to a Silica buffer. The wrapper must provide enough information for Silica to know the buffer element type and length.

Canonical array return mapping:

```text
T * returned as array data -> buf(region, T) with length
```

If the C API returns a pointer to array data without a length, the wrapper must obtain the length from the library contract, an out-parameter, a companion function, a sentinel convention, or wrapper-maintained metadata. If the wrapper cannot determine the length, it must return an explicit error result instead of exposing the array to Silica.

Any other C pointer return value must be changed from the pointer itself into the data it points to, using the pointee type.

Canonical non-array pointer return mapping:

```text
T * returned as single object -> T
const T * returned as single object -> T
```

The wrapper is responsible for reading, copying, and decoding the pointed-to value into the corresponding Silica-compatible type before returning it to Silica.

If the pointer is null, the wrapper must translate null into an explicit error result or into a documented concrete sum value such as `None`.

The pointer value itself must not be returned to Silica for non-array return values.

### 6.5 `void *` translation

`void *` must not appear in a Silica-facing wrapper declaration.

Permitted translations for `void *` are:

- a de-opaqueified Silica-compatible record, sum, buffer, string, scalar, or explicit error;
- a concrete buffer when the value is known to be byte data and has a known length;
- an explicit error result when the wrapper cannot determine or validate the pointed-to type.

The wrapper must not guess the pointee type of a `void *`. The translation must be based on the underlying library contract, an accompanying tag or type code, or wrapper-controlled metadata.

### 6.6 De-opaqueified C structs

Opaque C structs must be de-opaqueified by wrapper code before their contents reach Silica.

For this specification, an opaque C struct is any C struct whose fields are hidden from the public C header, incomplete at the wrapper boundary, version-dependent, or otherwise not directly visible to Silica source.

Silica does not accept opaque C structs as opaque handles, raw pointers, integer handles, or untyped scalar values. The wrapper must translate the opaque C struct into its actual content using Silica-compatible explicit types.

Required wrapper behavior:

- inspect or access the underlying C object through the library's documented accessor functions, public ABI contract, or wrapper-owned knowledge of the object layout;
- copy the object's meaningful contents into Silica-compatible records, sums, buffers, strings, integers, floats, booleans, atoms, or other values allowed by this specification;
- return an explicit error result when the object contents cannot be accessed, validated, or represented as Silica-compatible values;
- avoid exposing the object's pointer identity as the Silica value.

A C object that was originally opaque to ordinary C callers may be represented in Silica only after de-opaqueification into explicit content.

Example C library:

```c
typedef struct db_result db_result_t;

int db_result_status(db_result_t *result);
int64_t db_result_count(db_result_t *result);
const char *db_result_message(db_result_t *result);
```

Example Silica-facing result after de-opaqueification:

```silica
{ status: int64, count: int64, message: string }
```

If the C object represents a resource whose meaningful contents cannot be de-opaqueified, the wrapper must expose operations that return de-opaqueified snapshots or explicit error results. It must not expose the object itself as an opaque Silica value.

### 6.7 Recursive C struct prohibition

Recursive struct definitions are not allowed to be sent to Silica.

A struct is recursive if it contains, directly or indirectly, a field whose type refers back to the same struct shape. This includes recursion through pointers, arrays, records, tuples, lists, sum variants, or any chain of nested struct fields.

Invalid C shapes:

```c
struct node {
    int64_t value;
    struct node *next;
};

struct tree {
    int64_t value;
    struct tree *left;
    struct tree *right;
};
```

Such structures must be transformed by the wrapper into non-recursive Silica-compatible values, such as bounded buffers, flat records, summary values, or explicit error results.

**Compiler or wrapper-validation failure**:

```text
RecursiveExternalStructError:
Recursive C struct definitions cannot be exposed to Silica. The wrapper must return a non-recursive Silica-compatible value or an explicit error result.
```

### 6.8 Structs by value

C structs are permitted to be passed or returned by value only when the struct is explicitly declared as a Silica-compatible ABI record.

Such structs must:

- contain only wrapper-allowed scalar fields or other Silica-compatible ABI records;
- avoid bitfields;
- avoid flexible array members;
- avoid compiler-specific packing unless explicitly declared and checked;
- have layout verified by compile-time or build-time checks;
- be non-recursive.

### 6.9 Variadic functions

Silica-facing wrapper functions must not be variadic.

A C variadic API such as `printf` must be adapted through a fixed-signature wrapper function.

Example:

```c
int64_t silica_log_i64(const uint8_t *message_ptr, uint64_t message_len, int64_t value);
```

### 6.10 No inbound calls to Silica

This specification does not define callbacks, function pointers, trampolines, exported Silica functions, or any mechanism for external code to call into Silica.

Calls from C or from another language into Silica are out of scope and must be specified in a separate document.

---

## 7. External-Danger-Touched Data Rules

### 7.1 General taint rule

Any value returned from a `dangerous_*` module is external-danger-touched data.

This applies to all Silica values, not only strings, byte buffers, or memory-region references.

The taint is structural. Records, tuples, lists, and sum values containing external-danger-touched data are themselves external-danger-touched.

### 7.2 Executable content and command text

A string or byte buffer returned from a `dangerous_*` module is external-danger-touched data.

The compiler is not required to prove whether such data is executable binary content or system command text.

Instead, Silica treats such values as tainted external data. They must not be passed to APIs that execute commands, load dynamic code, write executable files, spawn processes, evaluate scripts, or cross actor `call` or `cast` message boundaries unless converted by an explicit validator.

Command execution APIs must not accept raw string commands. They must accept structured command values, such as an allowlisted program identifier plus an argument list.

### 7.3 Restricted effects

Any use of external-danger-touched data inside a `sequence ... produces pure ... end` block that declares any of the following existing Silica effects causes a type-check failure during compilation:

```silica
device_io
network_io
hot_swap
register_rwr
```

This rule applies to all external-danger-touched data, not only strings, byte buffers, or memory-region references.

The prohibition is structural. A value is not permitted to be used in one of these effect-tagged sequence blocks if it contains external-danger-touched data at any depth.

Invalid examples include:

- writing external-danger-touched data through `device_io`;
- sending external-danger-touched data through `network_io`;
- using external-danger-touched data in a `hot_swap` operation;
- using external-danger-touched data in a `register_rwr` operation;
- placing external-danger-touched data inside a record, tuple, list, or sum value and then using that containing value in any of the restricted effect blocks.

**Type-check failure**:

```text
ExternalDangerRestrictedEffectError:
external_danger-touched data cannot be used inside sequence blocks that declare device_io, network_io, hot_swap, or register_rwr.
```

### 7.4 Sequence completion rule

Silica retains no dangerous data at the completion of a `sequence proc[external_danger] ... produces pure ... end` block.

The value produced by the `produces pure` clause must contain only pure Silica values. It must not contain external-danger-touched data at any depth.

Before the sequence block completes, every external-danger-touched value must be one of the following:

- consumed entirely within the block;
- converted by an explicit validator into a non-tainted Silica value;
- copied or decoded by a wrapper or validator into a pure Silica value;
- rejected through an explicit error result.

The sequence boundary is a taint boundary. External-danger-touched data may exist only inside the dynamic and lexical extent of the `external_danger` sequence block, except for region values explicitly converted into actor-state-owned region references under §7.5.

The result of the `produces pure` expression must be checked structurally. If any field, tuple element, list element, sum payload, or nested value contains external-danger-touched data, compilation fails.

**Type-check failure**:

```text
ExternalDangerSequenceResultError:
sequence proc[external_danger] must produce only pure Silica values; external_danger-touched data cannot appear in the produced value.
```

### 7.5 Actor-local memory-region containment

Any memory region modified, created, or used within a sequence block tagged with `external_danger` is actor-local to the behavior function in which that sequence block appears.

Such a memory region must never be moved out of the actor behavior, except that it may be explicitly converted into an actor-state-owned region reference and stored in the actor state returned by that behavior invocation.

This rule applies to:

- memory-region references passed into a `dangerous_*` module function;
- memory-region references returned from a `dangerous_*` module function;
- memory regions allocated, initialized, mutated, borrowed, or consumed within an `external_danger` sequence block;
- records, tuples, lists, or sum values containing any such memory-region reference.

Actor state is the only permitted long-lived destination for such memory regions. These regions must not cross actor message boundaries.

The parser must reject any program that attempts to move an external-danger-touched memory region out of the actor behavior, except when the region is explicitly converted into an actor-state-owned region reference and placed into the actor state returned by that same behavior invocation.

**Parser failure**:

```text
ExternalDangerRegionEscapeError:
Memory regions created, modified, or used inside sequence proc[external_danger] cannot move out of the actor behavior function, except when explicitly converted into actor-state-owned region references returned as actor state.
```

### 7.6 Actor call and cast boundary rule

An external-danger-touched memory region must not be included at any depth in:

- a return value produced for an actor `call`;
- a payload sent through an actor `cast`.

This prohibition is structural. Wrapping the region inside records, tuples, lists, or sum variants does not make it valid.

**Type-check failure**:

```text
ExternalDangerMessageBoundaryError:
Memory regions created, modified, or used inside sequence proc[external_danger] cannot appear at any depth in a return value for call or in a payload for cast.
```

### 7.7 Validator rule

An explicit validator is permitted to convert external-danger-touched data into a non-tainted value only when the validator's specification permits that conversion.

Validator design is outside the scope of this version of the specification.

---

## 8. Type Mapping

Type mapping is directional. Values sent from Silica to a C wrapper follow different rules from values returned from a C wrapper to Silica.

### 8.1 Values sent from Silica to a C wrapper

When Silica calls a C wrapper function, Silica values are lowered into the wrapper ABI as follows:

| Silica source type | C wrapper receives                                                   |
| ------------------ | -------------------------------------------------------------------- |
| `int8`             | `int8_t`                                                             |
| `int16`            | `int16_t`                                                            |
| `int32`            | `int32_t`                                                            |
| `int64`            | `int64_t`                                                            |
| `uint8`            | `uint8_t`                                                            |
| `uint16`           | `uint16_t`                                                           |
| `uint32`           | `uint32_t`                                                           |
| `uint64`           | `uint64_t`                                                           |
| `float32`          | `float`                                                              |
| `float64`          | `double`                                                             |
| `boolean`          | `uint8_t`, where `0 = false` and `1 = true`                          |
| `string`           | actor-stack copy exposed as `const uint8_t *` plus `uint64_t` length |
| `buf(region, T)`   | typed pointer plus `uint64_t` length                                 |
| inline record      | C struct with matching field order and verified layout               |
| inline sum         | C struct with explicit `tag` and payload fields                      |

Rules for values sent from Silica:

- Silica strings are copied into the encompassing actor's expandable stack before the wrapper receives them.
- The original Silica string storage is never exposed to C.
- The wrapper must treat string pointers as read-only.
- Silica buffers must be passed with their length.
- Silica must not send recursive record shapes to C.

### 8.2 Values returned from a C wrapper to Silica

When a C wrapper returns values to Silica, C ABI values are raised into Silica values as follows:

| C wrapper return value           | Silica receives                                                       |
| -------------------------------- | --------------------------------------------------------------------- |
| `int8_t`                         | `int8`                                                                |
| `int16_t`                        | `int16`                                                               |
| `int32_t`                        | `int32`                                                               |
| `int64_t`                        | `int64`                                                               |
| `uint8_t`                        | `uint8`                                                               |
| `uint16_t`                       | `uint16`                                                              |
| `uint32_t`                       | `uint32`                                                              |
| `uint64_t`                       | `uint64`                                                              |
| `float`                          | `float32`                                                             |
| `double`                         | `float64`                                                             |
| `uint8_t` used as boolean        | `boolean`, only when value is `0` or `1`                              |
| C struct result                  | inline Silica record or sum with verified non-recursive shape         |
| C array return                   | Silica buffer, `buf(region, T)`, with known element type and length   |
| non-array typed C pointer return | copied or decoded pointee value of type `T`                           |
| `void *` from underlying library | de-opaqueified record, sum, buffer, string, scalar, or explicit error |
| opaque C struct                  | de-opaqueified Silica-compatible contents or explicit error           |
| null pointer                     | explicit error result or documented concrete sum value such as `None` |

Rules for values returned to Silica:

- C array returns must become Silica buffers with known element type and length.
- Non-array C pointer returns must not expose pointer identity to Silica. The wrapper must copy or decode the pointed-to value into the corresponding Silica-compatible type.
- Opaque C structs must be de-opaqueified into actual Silica-compatible contents.
- Recursive C struct definitions must not be returned to Silica.
- `void *` must never be returned to Silica as an untyped pointer.
- Returned values from `dangerous_*` modules are external-danger-touched until consumed, validated, copied, decoded, or converted according to this specification.
- A wrapper must return an explicit error when it cannot determine a returned pointer's element type, pointee type, length, layout, or de-opaqueified contents.

---

## 9. Result Conventions

### 9.1 Tag convention

Result structs are required to use an integer tag field.

Required encoding:

```text
0 = Ok
1 = Error
```

Additional tags are permitted only when documented by the specific wrapper family.

### 9.2 Integer result

```c
typedef struct silica_i64_result {
    int64_t tag;
    int64_t value;
    int64_t error_code;
} silica_i64_result;
```

Meaning:

```text
tag = 0 => Ok(value)
tag = 1 => Error(error_code)
```

### 9.3 De-opaqueified object result

Opaque C object results must be returned to Silica as de-opaqueified content.

Example Silica-facing result shape:

```silica
Ok({ status: int64, count: int64, message: string }) | Error(int64)
```

The wrapper must copy or decode the object's contents into Silica-compatible fields before returning.

If the wrapper cannot access or validate the object's actual content, it must return an explicit error result.

The pointer identity, address, private handle, or incomplete object representation must not be returned to Silica.

### 9.4 Combined result

A wrapper is permitted to use a combined result when an operation returns multiple values.

```c
typedef struct silica_counter_result {
    int64_t tag;
    int64_t value;
    int64_t error_code;
} silica_counter_result;
```

Unused fields must be initialized to deterministic values.

Required default:

```text
Unused integer field = 0
Unused object field = implementation-defined invalid value
```

### 9.5 Error codes

Wrapper error codes are required to be stable integers local to the wrapper module.

A wrapper is permitted to translate C `errno` values, library-specific error codes, or null-pointer failures into wrapper-specific error codes.

The wrapper must read `errno` immediately after the failing call when `errno` is relevant.

---

## 10. Examples

### 10.1 Numeric wrapper

Original external C API:

```c
int legacy_add(int a, int b);
```

Silica-facing C wrapper header:

```c
#include <stdint.h>

int64_t silica_legacy_math_add_int64(int64_t left, int64_t right);
```

C wrapper implementation:

```c
#include <stdint.h>
#include "legacy_math.h"

int64_t silica_legacy_math_add_int64(int64_t left, int64_t right) {
    return (int64_t)legacy_add((int)left, (int)right);
}
```

Silica dangerous module:

```silica
module dangerous_legacy_math;

export add/2;

foreign c_wrapper "silica_legacy_math_add_int64"
fn add_raw(left: int64, right: int64) -> int64;

fn add(left: int64, right: int64) -> int64 {
    add_raw(left, right)
}
```

Valid actor behavior and spawn site:

```silica
module dangerous_math_app;

use dangerous_legacy_math;

fn math_actor_behavior(
    msg: { left: int64, right: int64 },
    state: { value: int64 }
) -> { value: int64 } {
    sequence proc[external_danger]
        sum: int64 <- dangerous_legacy_math@add(msg.left, msg.right);
    produces
        pure { value: sum }
    end
}

fn main() -> int64 {
    sequence proc[concurrency]
        worker_ref: actor_ref <- spawn(
            { value: 0 },
            math_actor_behavior
        );
    produces
        pure 0
    end
}
```

This example is valid because:

- the importing compilation unit is a `dangerous_*` module (`dangerous_math_app`), satisfying the dangerous dependency naming rule in §3.1;
- the external call is exposed only through a `dangerous_*` module;
- the call appears inside the sequence portion of `sequence proc[external_danger] ... produces pure ... end`;
- the function containing the `external_danger` sequence is the function passed to `spawn` as the actor behavior;
- `main` only performs the actor spawn and does not contain the `external_danger` sequence;
- the produced value contains only a pure Silica record.

### 10.2 Fallible parser wrapper returning pure data

C wrapper header:

```c
#include <stdint.h>

typedef struct silica_i64_result {
    int64_t tag;
    int64_t value;
    int64_t error_code;
} silica_i64_result;

silica_i64_result silica_net_parse_port(
    const uint8_t *text_ptr,
    uint64_t text_len
);
```

C wrapper implementation outline:

```c
#include <stdint.h>
#include <stdlib.h>

silica_i64_result silica_net_parse_port(
    const uint8_t *text_ptr,
    uint64_t text_len
) {
    silica_i64_result result;

    if (text_ptr == 0 || text_len == 0) {
        result.tag = 1;
        result.value = 0;
        result.error_code = 1;
        return result;
    }

    /* Parse copied actor-stack string bytes without retaining text_ptr. */
    result.tag = 0;
    result.value = 8080;
    result.error_code = 0;
    return result;
}
```

Silica dangerous module:

```silica
module dangerous_net;

export parse_port/1;

foreign c_wrapper "silica_net_parse_port"
fn parse_port_raw(text: string) -> { tag: int64, value: int64, error_code: int64 };

fn parse_port(text: string) -> { tag: int64, value: int64, error_code: int64 } {
    parse_port_raw(text)
}
```

Valid actor behavior and spawn site:

```silica
module dangerous_parser_app;

use dangerous_net;

fn parser_actor_behavior(
    msg: { text: string },
    state: { last_port: int64 }
) -> { last_port: int64 } {
    sequence proc[external_danger]
        raw: { tag: int64, value: int64, error_code: int64 } <- dangerous_net@parse_port(msg.text);
    produces
        pure case raw.tag of {
            0: int64 -> { last_port: raw.value };
            _: int64 -> state;
        }
    end
}

fn main() -> int64 {
    sequence proc[concurrency]
        parser_ref: actor_ref <- spawn(
            { last_port: 0 },
            parser_actor_behavior
        );
    produces
        pure 0
    end
}
```

This example is valid because the importing compilation unit is a `dangerous_*` module (`dangerous_parser_app`), the `external_danger` sequence is in the function passed to `spawn`, not in `main`, the result of the `external_danger` sequence is a pure actor-state record, and the raw result from the dangerous module does not escape the sequence block.

### 10.3 Array return mapped to a Silica buffer

Original external C API:

```c
const int64_t *library_values(uint64_t *out_len);
```

Silica-facing C wrapper header:

```c
#include <stdint.h>

typedef struct silica_i64_buffer_result {
    int64_t tag;
    const int64_t *items_ptr;
    uint64_t items_len;
    int64_t error_code;
} silica_i64_buffer_result;

silica_i64_buffer_result silica_values_get_all(void);
```

Required wrapper behavior:

- `items_ptr` and `items_len` describe array data.
- The Silica binding maps the pair to `buf(region, int64)`.
- If the wrapper cannot determine `items_len`, it must return an error result.

Silica dangerous module:

```silica
module dangerous_values;

export get_all/0;

foreign c_wrapper "silica_values_get_all"
fn get_all_raw() -> { tag: int64, items: buf(region, int64), error_code: int64 };

fn get_all() -> { tag: int64, items: buf(region, int64), error_code: int64 } {
    get_all_raw()
}
```

Valid actor behavior and spawn site:

```silica
module dangerous_values_app;

use dangerous_values;

fn values_actor_behavior(
    msg: atom,
    state: { count: int64 }
) -> { count: int64 } {
    sequence proc[external_danger]
        raw: { tag: int64, items: buf(region, int64), error_code: int64 } <- dangerous_values@get_all();
        count_value: int64 <- buffer_length(raw.items);
    produces
        pure case raw.tag of {
            0: int64 -> { count: count_value };
            _: int64 -> state;
        }
    end
}

fn main() -> int64 {
    sequence proc[concurrency]
        values_ref: actor_ref <- spawn(
            { count: 0 },
            values_actor_behavior
        );
    produces
        pure 0
    end
}
```

This example is valid only if the importing compilation unit is a `dangerous_*` module (here `dangerous_values_app`), `values_actor_behavior` is the function passed to `spawn`, `count_value` is a pure Silica value, and the returned buffer itself does not escape the `external_danger` sequence.

### 10.4 Non-array pointer return decoded to pointee data

Original external C API:

```c
typedef struct point {
    int64_t x;
    int64_t y;
} point_t;

const point_t *library_current_point(void);
```

Silica-facing C wrapper header:

```c
#include <stdint.h>

typedef struct silica_point {
    int64_t x;
    int64_t y;
} silica_point;

typedef struct silica_point_result {
    int64_t tag;
    silica_point value;
    int64_t error_code;
} silica_point_result;

silica_point_result silica_point_current(void);
```

Required wrapper behavior:

- the original `const point_t *` is not returned to Silica;
- the wrapper copies the pointee data into `silica_point`;
- null is translated into an error result.

Silica dangerous module:

```silica
module dangerous_point;

export current/0;

foreign c_wrapper "silica_point_current"
fn current_raw() -> { tag: int64, value: { x: int64, y: int64 }, error_code: int64 };

fn current() -> { tag: int64, value: { x: int64, y: int64 }, error_code: int64 } {
    current_raw()
}
```

### 10.5 De-opaqueified C struct

Original external C API:

```c
typedef struct db_result db_result_t;

int db_result_status(db_result_t *result);
int64_t db_result_count(db_result_t *result);
const char *db_result_message(db_result_t *result);
```

Silica-facing C wrapper header:

```c
#include <stdint.h>

typedef struct silica_db_result_snapshot {
    int64_t status;
    int64_t count;
    const uint8_t *message_ptr;
    uint64_t message_len;
} silica_db_result_snapshot;

typedef struct silica_db_result_snapshot_result {
    int64_t tag;
    silica_db_result_snapshot value;
    int64_t error_code;
} silica_db_result_snapshot_result;

silica_db_result_snapshot_result silica_db_result_snapshot_current(void);
```

Required wrapper behavior:

- the opaque `db_result_t *` is not exposed to Silica;
- accessor functions are used to de-opaqueify the object;
- the returned Silica-facing value is a non-recursive snapshot;
- recursive links, private identity, handles, and raw pointers do not reach Silica.

Silica dangerous module:

```silica
module dangerous_db_result;

export current_snapshot/0;

foreign c_wrapper "silica_db_result_snapshot_current"
fn current_snapshot_raw() -> {
    tag: int64,
    value: { status: int64, count: int64, message: string },
    error_code: int64
};

fn current_snapshot() -> {
    tag: int64,
    value: { status: int64, count: int64, message: string },
    error_code: int64
} {
    current_snapshot_raw()
}
```

---

## 11. Ownership and Lifetime Rules

### 11.1 Copied Silica string memory

A Silica `string` passed to a C wrapper function is copied before the wrapper receives it.

The copy is stored in the expandable stack of the encompassing Silica actor, not in the general heap.

The C wrapper receives only the copied memory. It never receives a modifiable reference to the original Silica string.

The copied string memory is valid only for the duration of the foreign function call. When the function call ends, the copied memory is recovered.

The wrapper must not retain a pointer to the copied string memory after returning.

### 11.2 Retained data

If a C library needs to retain data passed from Silica, the wrapper must copy the data into wrapper-owned or library-owned memory.

For Silica strings, this means copying from the actor-stack string copy into memory whose lifetime is controlled by the wrapper or underlying library.

### 11.3 Allocator identity

Memory allocated by a C library must be freed by the appropriate C library or wrapper function.

Silica code must not directly free C-allocated memory unless the allocation contract explicitly permits it.

### 11.4 De-opaqueified external objects

Opaque external C objects must not be retained by Silica as opaque values.

When a C library returns an opaque object, the wrapper must either:

- decode the object's meaningful contents into Silica-compatible values;
- expose a wrapper operation that performs a complete operation and returns only Silica-compatible values;
- return an explicit error result.

The implementation may use private C pointers or wrapper-owned handle tables internally during the execution of a wrapper function, but ordinary Silica code must never observe those pointers or handles.

### 11.5 Double close

Wrapper implementations are required to detect invalid or already-closed resources when detection is possible and return an error code rather than causing undefined behavior.

---

## 12. Blocking Behavior

A wrapper function that may block must be documented as blocking.

Blocking behavior matters for actor scheduling and runtime responsiveness.

A future Silica implementation may route blocking C wrapper calls through a worker thread. Any such routing must preserve the rule that Silica source code can use `external_danger` only directly within the spawned actor behavior function.

---

## 13. Wrapper Metadata

The wrapper must be macro-free.

C wrapper headers and C wrapper implementation files must not define or require Silica-specific C preprocessor macros for ownership, lifetime, blocking behavior, result semantics, ABI versioning, or binding generation.

Wrapper metadata that cannot be derived from the C function signature must be declared outside the C preprocessor macro system.

Permitted metadata mechanisms are:

1. Silica package metadata;
2. a separate wrapper metadata file;
3. explicit Silica foreign declarations;
4. build-system metadata consumed by the Silica compiler or binding generator.

The metadata must describe any boundary property that is not mechanically derivable from the wrapper signature, including:

- borrowed inputs;
- retained external data;
- out-parameters;
- blocking behavior;
- result tag conventions;
- array element type and length source;
- de-opaqueification contract;
- non-recursive struct proof metadata;
- error-code domain.

Example metadata shape:

```text
wrapper silica_db_get_i64 {
    symbol: "silica_db_get_i64"
    blocking: true
    result: "tagged_result"
    error_domain: "dangerous_db"
    arguments: [
        { name: "key_ptr", lifetime: "borrowed", retain: false },
        { name: "key_len", role: "length", length_of: "key_ptr" }
    ]
}
```

The exact metadata file syntax is outside the scope of this document.

---

## 14. Build-System Requirements

### 14.1 Dangerous exposure source directory

All C wrapper header files are required to be located under the `dangerous_exposure_source` directory at the root of the Silica source code for the project.

Subdirectories inside `dangerous_exposure_source` are permitted.

Valid header locations:

```text
dangerous_exposure_source/silica_net_wrapper.h
dangerous_exposure_source/db/silica_db_wrapper.h
dangerous_exposure_source/vendor/sqlite/silica_sqlite_wrapper.h
```

Invalid header locations:

```text
include/silica_net_wrapper.h
src/ffi/silica_db_wrapper.h
wrappers/silica_sqlite_wrapper.h
```

The Silica compiler, build tool, or wrapper validator  rejects any wrapper header file located outside `dangerous_exposure_source`.

Required wrapper-validation failure:

```text
DangerousExposureSourceError:
C wrapper header files must be located under dangerous_exposure_source at the root of the Silica project.
```

### 14.2 Package declarations

A Silica package that uses C wrappers is required to declare:

- wrapper header files located under `dangerous_exposure_source`;
- wrapper implementation files;
- include paths;
- library search paths;
- libraries to link;
- link mode for each external library, where relevant: dynamically linked shared library or statically linked archive/object library;
- optional `pkg-config` package names;
- target-specific conditions.

Example package metadata shape:

```text
foreign package db {
    headers: ["dangerous_exposure_source/db/silica_db_wrapper.h"]
    sources: ["dangerous_exposure_source/db/silica_db_wrapper.c"]
    libraries: ["db"]
    link_mode: "dynamic"
    pkg_config: "db"
}
```

The Silica build tool must support both dynamically linked and statically linked external libraries for C wrapper packages. Package metadata must be able to distinguish the two modes whenever the target platform or package layout requires different linker flags, search paths, runtime search paths, archive ordering, or deployment behavior.

The exact package metadata syntax is outside the scope of this document.

---

## 15. Compile-Time and Parser Checks

The compile-time and parser checks are divided into two categories:

1. Silica-side checks; and
2. wrapper-side checks.

### 15.1 Silica-side checks

Silica-side checks are enforced by the Silica parser, type checker, effect checker, and module checker.

Required Silica-side hard errors:

- foreign declaration in a module whose name does not begin with `dangerous_`;
- import or other use of a `dangerous_*` module from a module whose own name does not begin with `dangerous_`;
- raw foreign binding exported directly to application code;
- exported function from a `dangerous_*` module that is not a Silica adapter wrapper;
- call to any function in a `dangerous_*` module outside the sequence portion of a `sequence proc[external_danger] ... produces pure ... end` block;
- `dangerous_*` module call outside the sequence portion of a sequence block that declares `external_danger`;
- `dangerous_*` module call inside a sequence block that does not declare `external_danger`;
- `external_danger` sequence block outside the function literal passed directly to `spawn` as the actor behavior function;
- `external_danger` sequence block inside an ordinary top-level function body;
- `external_danger` sequence block inside a helper function;
- `external_danger` sequence block inside a function literal that is not passed directly as the behavior function to `spawn`;
- `external_danger` sequence block in a nested expression position that is not the direct body of the spawned actor behavior function;
- more than eight Silica-level arguments after lowering;
- raw pointers in Silica-facing declarations;
- memory region created, modified, or used within `sequence proc[external_danger] ... produces pure ... end` moved out of the actor behavior except through explicit conversion into an actor-state-owned region reference returned as actor state;
- `sequence proc[external_danger] ... produces pure ... end` block producing a value containing unconverted external-danger-touched data at any depth;
- memory region created, modified, or used within `sequence proc[external_danger] ... produces pure ... end` included at any depth in a return value for `call` or in a payload for `cast`;
- external-danger-touched data used inside a sequence block that declares `device_io`, `network_io`, `hot_swap`, or `register_rwr`;
- recursive Silica record shape sent to a C wrapper;
- foreign declaration whose Silica type does not match the wrapper-side ABI contract.

Required parser failure:

```text
ExternalDangerPlacementError:
external_danger sequence blocks are only valid directly inside the function spawned for an actor.
```

Required parser failure:

```text
DangerousDependencyNamingError:
A module that imports or uses a dangerous_* module must use the dangerous_ prefix in its own module name.
```

Required parser failure:

```text
DangerousModuleCallError:
Calls to dangerous_* module functions must appear in the sequence portion of sequence proc[external_danger] ... produces pure ... end.
```

Required parser failure:

```text
ExternalDangerRegionEscapeError:
Memory regions created, modified, or used inside sequence proc[external_danger] cannot move out of the actor behavior function, except when explicitly converted into actor-state-owned region references returned as actor state.
```

Required type-check failure:

```text
ExternalDangerSequenceResultError:
sequence proc[external_danger] must produce only pure Silica values; external-danger-touched data cannot appear in the produced value.
```

Required type-check failure:

```text
ExternalDangerMessageBoundaryError:
Memory regions created, modified, or used inside sequence proc[external_danger] cannot appear at any depth in a return value for call or in a payload for cast.
```

Required type-check failure:

```text
ExternalDangerRestrictedEffectError:
external-danger-touched data cannot be used inside sequence blocks that declare device_io, network_io, hot_swap, or register_rwr.
```

### 15.2 Wrapper-side checks

Wrapper-side checks are enforced by the binding generator, wrapper validator, header checker, build tool, or any compiler phase that verifies the C wrapper ABI before Silica code is accepted.

Required wrapper-side hard errors:

- unsupported C type in wrapper signature;
- variadic wrapper function;
- unsupported struct layout;
- recursive C struct definition exposed to Silica instead of being flattened, summarized, bounded, or rejected;
- C array return value not mapped to a Silica buffer with known element type and length;
- non-array C pointer return value exposed as a pointer instead of being copied or decoded into its pointee type;
- `void *` value exposed directly to Silica instead of being translated by wrapper code into an approved concrete Silica-facing representation;
- opaque C object exposed to Silica as `uint64`, raw pointer, untyped scalar, handle, or opaque external type instead of being de-opaqueified into Silica-compatible contents;
- wrapper result struct missing a deterministic tag convention;
- wrapper result struct containing uninitialized fields visible to Silica;
- wrapper exposing a C string as NUL-terminated only instead of pointer-plus-length or Silica-compatible string data;
- wrapper retaining actor-stack string or buffer copies after the foreign call ends;
- wrapper failing to document blocking behavior for a blocking external call;
- wrapper failing to provide an explicit error result when it cannot determine a returned pointer's element type, pointee type, length, layout, or de-opaqueified contents;
- wrapper declaration whose C ABI type mapping disagrees with the Silica foreign declaration;
- C wrapper header file located outside `dangerous_exposure_source` at the root of the Silica project;
- Silica-specific C preprocessor macro present in a wrapper header or wrapper implementation;
- required wrapper metadata missing for ownership, lifetime, blocking behavior, result semantics, array length, de-opaqueification, or recursive-struct validation.

Required wrapper-validation failure:

```text
UnsupportedExternalAbiError:
Wrapper declaration uses a C ABI shape that is not supported by the Silica FFI wrapper specification.
```

Required wrapper-validation failure:

```text
RecursiveExternalStructError:
Recursive C struct definitions cannot be exposed to Silica. The wrapper must return a non-recursive Silica-compatible value or an explicit error result.
```

Required wrapper-validation failure:

```text
ExternalPointerReturnError:
C pointer return values must be mapped to Silica buffers, copied pointee values, de-opaqueified contents, or explicit error results.
```

Required wrapper-validation failure:

```text
ExternalVoidPointerError:
void * values must be translated by wrapper code into approved concrete Silica-facing representations before reaching Silica.
```

---

## 16. Open Design Questions

The following questions remain open:

2. Must every `dangerous_*` module function be callable only from actor behavior functions, including pure helper functions in those modules?
3. Must string lowering be built into the compiler, or must users explicitly pass `{ ptr, len }`-like records once pointer types exist?
4. What exact macro-free metadata format is required for wrapper ownership, lifetime, blocking behavior, result semantics, and validation contracts?
5. How are wrappers required to declare and verify de-opaqueification contracts for C structs whose public headers hide their fields?
6. What metadata is a wrapper required to provide to prove that a de-opaqueified C struct shape is non-recursive?
7. How are blocking C wrapper calls required to interact with actors?

Questions about external languages calling into Silica are intentionally excluded from this specification and belong in a separate inbound-interop specification.

---

## 17. Summary

Silica FFI is wrapper-first.

External libraries are required to be adapted into a small, explicit, predictable C-compatible ABI subset before Silica calls them. Wrapper functions are required to use fixed-width types, pointer-plus-length strings, explicit result structs, de-opaqueified C object contents, and clear ownership rules.

Ordinary Silica code is required to call Silica adapter functions through `dangerous_*` modules. Every compilation unit that imports or uses a `dangerous_*` module must itself be a `dangerous_*` module; that constraint propagates to the root application module and to the compiled application name whenever the program depends on FFI. Every call to a `dangerous_*` module function must occur inside the sequence portion of a `sequence proc[external_danger] ... produces pure ... end` block, and that block must be used directly within the function spawned for an actor.

Any C array return maps to a Silica buffer. Any non-array C pointer must be converted to type before being sent to Silica. Any `void *` from the underlying library must be translated by wrapper code into an approved concrete Silica-facing representation before Silica sees it. Opaque C structs must be de-opaqueified into Silica-compatible contents, not exposed as handles, raw pointers, or opaque external types.

No external-danger-touched data may be retained after an `external_danger` sequence completes unless it has been converted according to this specification. External-danger-touched data must not cross actor `call` or `cast` message boundaries and must not be used in sequence blocks declaring `device_io`, `network_io`, `hot_swap`, or `register_rwr`.
