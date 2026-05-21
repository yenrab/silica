# Silica External Library Call Wrapper Specification

**Last updated**: May 21, 2026

## Related Documents

| Document                             | Purpose                                                                  |
| ------------------------------------ | ------------------------------------------------------------------------ |
| `silica-specification.md`            | Core language syntax, effects, actors, regions, and compiler diagnostics |
| `silica-specification-additional.md` | Additional compile-time failure rules                                    |
| `ffi_wrapper_implementation_plan.md` | Compiler implementation phases for this specification                    |

---

## 1. Introduction

### 1.1 Overview

This specification defines the rules for outbound calls from Silica to external libraries through C-compatible wrapper functions.

This specification applies only to calls from Silica to C libraries and to other language libraries that expose a C-compatible interface. Underlying external libraries may be provided as dynamically linked shared libraries or as statically linked archive/object libraries. Calls from C or other languages into Silica are out of scope and are specified in a separate inbound-interop specification.

Silica does not call arbitrary external APIs directly. Every external operation callable from Silica must be exposed through a Silica-compatible C wrapper function. The wrapper function adapts the original external API into a stable, explicit ABI boundary that the Silica compiler can type-check and effect-check. In the initial implementation, the toolchain validates wrapper symbols at **link time only**; it does not parse C headers.

### 1.2 Design Principles

- **Wrapper-First External Calls**: External libraries are adapted through wrapper functions before Silica calls them.
- **Explicit Danger Boundary**: Every module that declares or exposes foreign functions, or that imports or otherwise uses any module whose name begins with `dangerous_`, must itself use the `dangerous_` module-name prefix. That naming requirement propagates along the module dependency graph to the root application module, so the compiled application name carries `dangerous_` whenever the program depends—directly or transitively—on any `dangerous_*` module. A module that never depends on a `dangerous_*` module is not required to use the prefix.
- **Cast-Mediated Foreign Calls**: Application actors never call `dangerous_*` module functions directly. Every outbound foreign operation is requested by `cast` to a dedicated **FFI worker actor**; the worker executes the C wrapper call and delivers the outcome by `cast` to a designated receiver actor.
- **Cast-Only Client Behaviors**: An actor whose behavior initiates foreign work must be a **cast-only behavior** (it handles incoming messages through `cast`, not `call`).
- **Worker-Scoped `external_danger`**: Calls to `dangerous_*` modules are required to appear only in the sequence portion of `sequence proc[external_danger] ... produces pure ... end` inside an FFI worker actor behavior.
- **No Retained Dangerous Data in `produces pure`**: A completed `external_danger` sequence produces only structurally pure Silica values. Foreign results leave the worker through designated FFI result casts, not through a tainted `produces pure` value.
- **Strict Structural Taint**: Values returned from `dangerous_*` modules remain external-danger-touched at every depth. This specification version does not define validator-based de-taint.
- **Strong Typing at the Boundary**: Silica foreign declarations, explicit `wrapper_meta` references, and sidecar metadata define the Silica-facing ABI; link time verifies symbol presence only.
- **Two-Layer String Declarations**: Raw foreign bindings use pointer-plus-length arguments; exported adapter wrappers accept Silica `string` and perform the copy before calling the raw binding.
- **No Raw Pointer Exposure**: Raw pointers, `void *`, and opaque C structs must not be exposed directly to Silica source types.
- **Non-Recursive Data Only**: Recursive C struct shapes must not be sent to Silica.
- **Outbound Only**: This specification does not define callbacks, trampolines, exported Silica functions, or external calls into Silica.
- **Prebuilt Wrapper Libraries (initial toolchain)**: C wrapper object code is supplied as prebuilt static libraries; the Silica build tool links them but does not compile C wrapper sources in the initial implementation.

### 1.3 Scope

This specification defines:

1. C wrapper function requirements.
2. Silica `dangerous_*` module requirements, including naming rules that cascade along module dependencies to the root application module and compiled application name.
3. Cast-mediated foreign calls, cast-only client behaviors, and the `external_danger` effect.
4. Rules for strings, buffers, pointers, arrays, and de-opaqueified C structs.
5. Restrictions on dangerous data crossing actor and effect boundaries.
6. Sidecar wrapper metadata and link-time symbol validation.
7. Compile-time, parser, type-check, and link-time failures.

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

### 2.7 FFI worker actor

An **FFI worker actor** is a spawned actor whose behavior executes outbound foreign calls. It receives foreign-call request messages by `cast`, invokes `dangerous_*` module functions inside `sequence proc[external_danger] ... produces pure ... end`, and delivers outcomes by `cast` to a designated receiver actor named in the request.

### 2.8 Cast-only behavior

A **cast-only behavior** is an actor behavior function that handles incoming actor messages exclusively through `cast`. It must not be written as a `call`-reply behavior. Application actors that initiate foreign work, and FFI worker actors that execute foreign calls, must use cast-only behaviors.

### 2.9 Sidecar metadata file

A **sidecar metadata file** is a macro-free metadata file stored under `dangerous_exposure_source/`. It declares link libraries and per-symbol wrapper facts that cannot be derived from Silica foreign declarations alone.

Sidecar files are **not** discovered automatically from header names or directory layout. A `dangerous_*` module references sidecar files explicitly through `wrapper_meta` declarations in Silica source (§3.2).

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

This specification adds the following declaration forms.

**Sidecar reference (module level)** — required in any module that declares one or more raw foreign bindings:

```silica
wrapper_meta "dangerous_exposure_source/db/silica_db_wrapper.meta";
```

A module may declare multiple `wrapper_meta` paths when its foreign bindings draw metadata from more than one sidecar file.

**Raw foreign binding**:

```silica
foreign c_wrapper "symbol_name"
fn local_name(arg1: Type1, arg2: Type2) -> ReturnType;
```

**Optional per-binding sidecar override** — when one foreign binding uses a different sidecar file from the module defaults:

```silica
foreign c_wrapper "symbol_name"
    meta "dangerous_exposure_source/other/extra.meta"
fn local_name(arg1: Type1, arg2: Type2) -> ReturnType;
```

Example:

```silica
module dangerous_legacy_math;

wrapper_meta "dangerous_exposure_source/legacy/silica_legacy_math_wrapper.meta";

foreign c_wrapper "silica_legacy_math_add_int64"
fn add_raw(left: int64, right: int64) -> int64;
```

**Sidecar reference rules**:

- Every module containing a `foreign c_wrapper` declaration must declare at least one `wrapper_meta` path, unless every such declaration carries its own `meta` clause.
- Each `wrapper_meta` and `meta` path must be a string literal.
- Each path must be rooted under the project `dangerous_exposure_source/` directory.
- The compiler loads **only** sidecar files referenced by `wrapper_meta` or `meta` declarations in the module being compiled. It does not infer paths from header basenames, directory walks, or symbol naming conventions.
- Each `foreign c_wrapper "symbol"` declaration must have exactly one matching `wrapper symbol { ... }` entry in a sidecar file referenced by that module.

**Compiler failures**:

```text
MissingWrapperMetaError:
A module that declares foreign c_wrapper bindings must declare wrapper_meta paths or a meta path on each foreign binding.
```

```text
WrapperMetaPathError:
wrapper_meta and meta paths must be located under dangerous_exposure_source at the root of the Silica project.
```

A foreign declaration binds a C wrapper symbol to a Silica function name in the current module.

### 3.3 Export rule

**Rule**: Every exported function from a `dangerous_*` module must be a Silica adapter wrapper.

Raw foreign bindings must not be exported directly to application code.

**Adapter detection rule**: An exported function from a `dangerous_*` module is a valid Silica adapter wrapper if and only if it is an ordinary Silica function declaration with a body. A `foreign c_wrapper` declaration is never a valid export.

A Silica adapter wrapper is a Silica function with a body that calls one or more raw foreign bindings and returns a Silica-facing value that conforms to this specification. Exported adapter wrappers are callable only from FFI worker actors inside `external_danger` sequences (§4). Application actors must not call them directly; they request foreign work by `cast` to an FFI worker actor (§4.2).

Example:

```silica
module dangerous_legacy_math;

export add/2;

wrapper_meta "dangerous_exposure_source/legacy/silica_legacy_math_wrapper.meta";

foreign c_wrapper "silica_legacy_math_add_int64"
fn add_raw(left: int64, right: int64) -> int64;

fn add(left: int64, right: int64) -> int64 {
    add_raw(left, right)
}
```

The exported name `add/2` is not callable from application actor behaviors. An FFI worker actor calls it inside the sequence portion of a valid `sequence proc[external_danger] ... produces pure ... end` block.

The raw foreign binding `add_raw/2` is not exported.

---

## 4. Cast-Mediated Foreign Calls and the `external_danger` Effect

### 4.1 Effect declaration

This specification adds the following effect:

```silica
external_danger
```

### 4.2 Cast-mediated call model

**Rule**: Every outbound foreign operation uses a cast-mediated handshake. Application actors must not call `dangerous_*` module functions directly.

Required flow:

1. A **client actor** with a cast-only behavior receives work by `cast`.
2. The client actor sends a foreign-call request by `cast` to an **FFI worker actor**.
3. The FFI worker actor executes the requested operation inside `sequence proc[external_danger] ... produces pure ... end`, calling one or more `dangerous_*` module functions as needed.
4. If the operation has a result, the FFI worker actor delivers it by `cast` to the receiver actor named in the request (typically the client actor).

This cast-mediated model applies to all outbound foreign calls. From the perspective of Silica actor scheduling, every foreign call is **non-blocking**: the client actor sends a request cast and resumes; the FFI worker actor performs the call and delivers the outcome by a separate result cast (§12).

### 4.3 Cast-only client behavior rule

**Rule**: An actor whose behavior initiates foreign work must be spawned with a cast-only behavior.

Such a behavior:

- handles incoming messages exclusively through `cast`;
- must not use the `call`-reply behavior shape;
- requests foreign operations only by casting to an FFI worker actor;
- receives foreign outcomes only by handling FFI result casts.

**Parser failure**:

```text
ExternalDangerClientBehaviorError:
Actors that initiate foreign work must use cast-only behaviors.
```

### 4.4 FFI worker actor rule

**Rule**: Outbound C wrapper execution occurs only inside an FFI worker actor behavior spawned by `spawn`.

An FFI worker actor:

- uses a cast-only behavior;
- receives foreign-call request casts;
- executes `dangerous_*` module calls only inside `sequence proc[external_danger] ... produces pure ... end`;
- delivers foreign outcomes by `cast` to the receiver named in the request;
- must not deliver external-danger-touched data through the `produces pure` clause (§7.4).

A program may use one shared FFI worker actor or multiple specialized worker actors. The worker behavior must be the function passed directly to `spawn`.

### 4.5 Required effect rule

**Rule**: A call to any function in any `dangerous_*` module must appear in the sequence portion of an enclosing `sequence proc[external_danger] ... produces pure ... end` block inside an FFI worker actor behavior.

This rule applies to:

- raw foreign bindings called by adapter wrappers inside `dangerous_*` modules;
- Silica adapter functions declared in `dangerous_*` modules;
- helper functions defined in `dangerous_*` modules;
- functions that return only pure Silica values but reside in a `dangerous_*` module.

Calls to `dangerous_*` module functions from application actor behaviors, ordinary top-level functions, or non-worker actor behaviors are invalid.

Valid (inside FFI worker behavior):

```silica
sequence proc[external_danger]
    sum: int64 <- dangerous_legacy_math@add(req.left, req.right);
    cast(req.reply_to, { tag: :foreign_ok, value: sum });
produces
    pure state
end
```

Invalid (direct call from application behavior):

```silica
sum: int64 <- dangerous_legacy_math@add(left, right);
```

**Parser failure**:

```text
DangerousModuleCallError:
Calls to dangerous_* module functions must appear in the sequence portion of sequence proc[external_danger] ... produces pure ... end inside an FFI worker actor behavior.
```

### 4.6 Worker placement rule

**Rule**: A `sequence proc[external_danger] ... produces pure ... end` block is valid only when it appears directly inside the cast-only behavior function passed to `spawn` for an FFI worker actor.

The worker behavior may be written as a function literal at the `spawn` call site or as a named top-level function passed directly to `spawn`. In both cases, the function containing the `external_danger` sequence must be the FFI worker behavior function supplied to `spawn`.

**Parser failure**:

```text
ExternalDangerPlacementError:
external_danger sequence blocks are only valid directly inside the cast-only behavior function spawned for an FFI worker actor.
```

### 4.7 Disallowed placements

The parser must reject an `external_danger` sequence block in any of the following positions:

- inside an ordinary top-level function body;
- inside a helper function that is not the FFI worker behavior passed directly to `spawn`;
- inside an application actor behavior;
- inside a function literal that is not passed directly to `spawn` as an FFI worker behavior;
- inside a named function that is not passed directly to `spawn` as an FFI worker behavior;
- inside a nested expression that is not the direct body of the spawned FFI worker behavior function;
- inside any non-actor context.

The parser must reject a direct call to any `dangerous_*` module function outside an FFI worker actor `external_danger` sequence.

### 4.8 Interaction with other effects

An FFI worker behavior is permitted to contain a sequence block tagged only with `external_danger` when the block performs only dangerous module calls, pure computation, concurrency casts required for FFI result delivery, and no other effects.

If the worker behavior performs other effects, the sequence block is permitted to include additional effects only when those effects are valid in actor behavior functions and are not restricted by this specification.

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

This specification uses a **two-layer declaration model**:

1. **Raw foreign binding**: declares the C ABI shape using pointer-plus-length arguments.
2. **Exported adapter wrapper**: accepts a Silica `string`, copies it to actor-stack scratch memory, and calls the raw foreign binding.

Raw foreign binding example (the enclosing module must also declare `wrapper_meta`; see §3.2):

```silica
wrapper_meta "dangerous_exposure_source/net/silica_net_wrapper.meta";

foreign c_wrapper "silica_net_parse_port"
fn parse_port_raw(text_ptr: uint8, text_len: uint64) -> { tag: int64, value: int64, error_code: int64 };
```

Adapter wrapper example:

```silica
export parse_port/1;

fn parse_port(text: string) -> { tag: int64, value: int64, error_code: int64 } {
    parse_port_raw(text)
}
```

The compiler lowers a Silica `string` argument at a raw foreign binding call site into an actor-stack copy exposed to C as:

```c
const uint8_t *text_ptr,
uint64_t text_len
```

When the call site is an adapter wrapper with a `string` parameter, the adapter body calls the raw binding; the compiler performs the same copy lowering for that call.

The copied memory resides in the expandable stack of the encompassing Silica actor (the FFI worker actor executing the foreign call). It is not allocated in the general heap.

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
- have layout declared consistently in Silica foreign declarations and sidecar metadata;
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

Instead, Silica treats such values as tainted external data. They must not be passed to APIs that execute commands, load dynamic code, write executable files, spawn processes, evaluate scripts, or cross ordinary actor `call` or `cast` message boundaries. The sole exception is the **FFI result cast** path defined in §4.2 and §7.6.

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

Silica retains no external-danger-touched data in the `produces pure` value of a `sequence proc[external_danger] ... produces pure ... end` block.

The value produced by the `produces pure` clause must contain only structurally pure Silica values. It must not contain external-danger-touched data at any depth.

**Strict structural taint**: In this specification version, external-danger-touched data is not cleared by destructuring, tag matching, field projection, copying into fresh records, or user-defined helper functions. A value returned from a `dangerous_*` module remains external-danger-touched at every depth until it is consumed entirely inside the `external_danger` sequence block without appearing in `produces pure`.

Foreign outcomes leave an FFI worker actor through the designated **FFI result cast** to the receiver named in the request (§4.2). The cast payload may contain external-danger-touched data. The worker's `produces pure` value must contain only structurally pure actor state.

Before the sequence block completes, every external-danger-touched value inside the sequence must be one of the following:

- consumed entirely within the block without appearing in `produces pure`;
- delivered to the designated receiver by an FFI result cast executed inside the block;
- rejected through explicit control flow that does not place tainted data in `produces pure`.

The sequence boundary is a taint boundary for `produces pure`. External-danger-touched data may exist inside the dynamic and lexical extent of the `external_danger` sequence block, and may appear only in FFI result casts executed from that block, except for region values explicitly converted into actor-state-owned region references under §7.5.

The result of the `produces pure` expression must be checked structurally. If any field, tuple element, list element, sum payload, or nested value contains external-danger-touched data, compilation fails.

**Type-check failure**:

```text
ExternalDangerSequenceResultError:
sequence proc[external_danger] must produce only pure Silica values; external_danger-touched data cannot appear in the produced value.
```

### 7.5 Actor-local memory-region containment

Any memory region modified, created, or used within a sequence block tagged with `external_danger` is actor-local to the FFI worker behavior function in which that sequence block appears.

Such a memory region must never be moved out of the FFI worker behavior, except that it may be explicitly converted into an actor-state-owned region reference and stored in the actor state returned by that behavior invocation, or delivered through an FFI result cast under §7.6.

This rule applies to:

- memory-region references passed into a `dangerous_*` module function;
- memory-region references returned from a `dangerous_*` module function;
- memory regions allocated, initialized, mutated, borrowed, or consumed within an `external_danger` sequence block;
- records, tuples, lists, or sum values containing any such memory-region reference.

Actor state is the only permitted long-lived destination for such memory regions. These regions must not cross actor message boundaries.

The parser must reject any program that attempts to move an external-danger-touched memory region out of the FFI worker behavior, except when the region is explicitly converted into an actor-state-owned region reference and placed into the actor state returned by that same behavior invocation, or delivered through an FFI result cast under §7.6.

**Parser failure**:

```text
ExternalDangerRegionEscapeError:
Memory regions created, modified, or used inside sequence proc[external_danger] cannot move out of the FFI worker behavior function, except when explicitly converted into actor-state-owned region references returned as actor state, or delivered through an FFI result cast under §7.6.
```

### 7.6 Actor call and cast boundary rule

An external-danger-touched memory region must not be included at any depth in:

- a return value produced for an ordinary actor `call`;
- an ordinary actor `cast` payload.

This prohibition is structural. Wrapping the region inside records, tuples, lists, or sum variants does not make it valid.

**FFI result cast exception**: An FFI worker actor may include external-danger-touched data, including memory-region references, in a cast payload sent to the receiver named in a foreign-call request, provided the cast is executed inside the requesting worker's `external_danger` sequence block. No other cast or call path may carry external-danger-touched memory regions.

A client actor that receives an FFI result cast must consume or discard any external-danger-touched payload within that cast handler. It must not place external-danger-touched memory regions into actor state, ordinary outbound casts, or `call` replies.

**Type-check failure**:

```text
ExternalDangerMessageBoundaryError:
Memory regions created, modified, or used inside sequence proc[external_danger] cannot appear at any depth in an ordinary call reply or cast payload. Only FFI result casts from an FFI worker actor to the designated receiver are permitted.
```

### 7.7 Validator rule

Validator-based de-taint is **out of scope** for this specification version. External-danger-touched data remains tainted structurally until consumed inside an permitted `external_danger` sequence block or delivered through an FFI result cast as defined in §4.2 and §7.6.

A future specification may define explicit validators that convert external-danger-touched data into non-tainted values.

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
| `string` (adapter) | adapter parameter only; lowered at raw call to actor-stack copy as `const uint8_t *` plus `uint64_t` length |
| `string` (raw foreign binding) | not permitted; raw bindings use pointer-plus-length arguments |
| `buf(region, T)`   | typed pointer plus `uint64_t` length                                 |
| inline record      | C struct with matching field order and verified layout               |
| inline sum         | C struct with explicit `tag` and payload fields                      |

Rules for values sent from Silica:

- Raw foreign bindings declare pointer-plus-length arguments for string data; exported adapter wrappers accept Silica `string`.
- Silica strings are copied into the FFI worker actor's expandable stack before the raw foreign binding call reaches C.
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
- Returned values from `dangerous_*` modules are external-danger-touched until consumed inside an FFI worker `external_danger` sequence or delivered through an FFI result cast.
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

wrapper_meta "dangerous_exposure_source/legacy/silica_legacy_math_wrapper.meta";

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

fn ffi_worker_behavior(
    msg: {
        op: :foreign_call,
        reply_to: actor_ref,
        left: int64,
        right: int64
    },
    state: { pending: int64 }
) -> { pending: int64 } {
    sequence proc[external_danger, concurrency]
        sum: int64 <- dangerous_legacy_math@add(msg.left, msg.right);
        cast(msg.reply_to, { tag: :foreign_ok, value: sum });
    produces
        pure state
    end
}

fn client_behavior(
    msg: {
        op: :compute,
        left: int64,
        right: int64
    } | {
        tag: :foreign_ok,
        value: int64
    },
    state: { value: int64, worker: actor_ref, self_ref: actor_ref }
) -> { value: int64, worker: actor_ref, self_ref: actor_ref } {
    case msg of {
        { op: :compute, left: left, right: right } -> {
            cast(state.worker, {
                op: :foreign_call,
                reply_to: state.self_ref,
                left: left,
                right: right
            });
            state
        };
        { tag: :foreign_ok, value: value } -> {
            { value: value, worker: state.worker, self_ref: state.self_ref }
        };
    }
}

fn main() -> int64 {
    sequence proc[concurrency]
        worker_ref: actor_ref <- spawn({ pending: 0 }, ffi_worker_behavior);
        client_ref: actor_ref <- spawn(
            { value: 0, worker: worker_ref, self_ref: client_ref },
            client_behavior
        );
    produces
        pure 0
    end
}
```

This example is valid because:

- the importing compilation unit is a `dangerous_*` module (`dangerous_math_app`), satisfying the dangerous dependency naming rule in §3.1;
- the external call is exposed only through a `dangerous_*` module;
- the call appears inside the sequence portion of `sequence proc[external_danger] ... produces pure ... end` in an FFI worker cast-only behavior;
- the client actor requests foreign work by `cast` and receives the outcome by FFI result cast;
- the worker's `produces pure` value contains only structurally pure actor state; the foreign result leaves through an FFI result cast;
- `main` only performs actor spawns and does not contain an `external_danger` sequence.

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

wrapper_meta "dangerous_exposure_source/net/silica_net_wrapper.meta";

foreign c_wrapper "silica_net_parse_port"
fn parse_port_raw(text_ptr: uint8, text_len: uint64) -> { tag: int64, value: int64, error_code: int64 };

fn parse_port(text: string) -> { tag: int64, value: int64, error_code: int64 } {
    parse_port_raw(text)
}
```

Sidecar metadata referenced by `wrapper_meta` (`dangerous_exposure_source/net/silica_net_wrapper.meta`):

```text
link_library: "silica_net"

wrapper silica_net_parse_port {
    symbol: "silica_net_parse_port"
    result: "tagged_result"
    error_domain: "dangerous_net"
}
```

Valid actor behavior and spawn site:

```silica
module dangerous_parser_app;

use dangerous_net;

fn ffi_worker_behavior(
    msg: {
        op: :foreign_call,
        reply_to: actor_ref,
        text: string
    },
    state: { pending: int64 }
) -> { pending: int64 } {
    sequence proc[external_danger, concurrency]
        raw: { tag: int64, value: int64, error_code: int64 } <- dangerous_net@parse_port(msg.text);
        cast(msg.reply_to, { tag: :foreign_ok, raw: raw });
    produces
        pure state
    end
}

fn client_behavior(
    msg: { op: :parse, text: string } | { tag: :foreign_ok, raw: { tag: int64, value: int64, error_code: int64 } },
    state: { last_port: int64, worker: actor_ref, self_ref: actor_ref }
) -> { last_port: int64, worker: actor_ref, self_ref: actor_ref } {
    case msg of {
        { op: :parse, text: text } -> {
            cast(state.worker, { op: :foreign_call, reply_to: state.self_ref, text: text });
            state
        };
        { tag: :foreign_ok, raw: raw } -> {
            case raw.tag of {
                0: int64 -> { last_port: raw.value, worker: state.worker, self_ref: state.self_ref };
                _: int64 -> state;
            }
        };
    }
}

fn main() -> int64 {
    sequence proc[concurrency]
        worker_ref: actor_ref <- spawn({ pending: 0 }, ffi_worker_behavior);
        parser_ref: actor_ref <- spawn(
            { last_port: 0, worker: worker_ref, self_ref: parser_ref },
            client_behavior
        );
    produces
        pure 0
    end
}
```

This example is valid because the importing compilation unit is a `dangerous_*` module (`dangerous_parser_app`), the client actor uses a cast-only behavior, the `external_danger` sequence is in the FFI worker behavior rather than in `main`, the raw foreign binding uses pointer-plus-length while the adapter accepts `string`, and the worker delivers the foreign result through an FFI result cast rather than through `produces pure`.

### 10.3 Array return mapped to a Silica buffer

Examples §10.3–§10.5 show `dangerous_*` module and C wrapper shapes only. Application integration uses the cast-mediated FFI worker model from §4 and §10.1–§10.2.

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

wrapper_meta "dangerous_exposure_source/values/silica_values_wrapper.meta";

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

wrapper_meta "dangerous_exposure_source/point/silica_point_wrapper.meta";

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

wrapper_meta "dangerous_exposure_source/db/silica_db_result_wrapper.meta";

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

## 12. Non-Blocking Foreign Calls

All outbound foreign calls use the cast-mediated model in §4.2. Client actors and FFI worker actors both use cast-only behaviors. A client never waits synchronously inside an `external_danger` sequence for a C library call to finish.

**Architectural rule**: Every foreign call is non-blocking at the Silica actor level. The client sends a foreign-call request cast and continues until it handles the FFI result cast. Any synchronous wait inside the underlying C wrapper executes on the FFI worker actor's thread only and does not block other actors' scheduling.

Sidecar metadata does **not** declare `blocking`. That field is redundant because the cast-only FFI worker model already isolates potentially blocking C library calls to the worker actor thread.

Wrapper authors remain responsible for not performing unbounded blocking work that would stall an FFI worker actor, but that is an operational constraint on worker design—not a separate metadata dimension tracked by the compiler.

---

## 13. Wrapper Metadata

The wrapper must be macro-free.

C wrapper headers and C wrapper implementation files must not define or require Silica-specific C preprocessor macros for ownership, lifetime, blocking behavior, result semantics, ABI versioning, or binding generation.

Wrapper metadata that cannot be derived from Silica foreign declarations must be supplied in **sidecar metadata files** stored under `dangerous_exposure_source/`.

### 13.1 Sidecar reference and discovery

Sidecar metadata files live under `dangerous_exposure_source/` but are located **only** through explicit Silica source references:

```silica
wrapper_meta "dangerous_exposure_source/db/silica_db_wrapper.meta";
```

or, for a single binding override:

```silica
foreign c_wrapper "silica_db_get_i64"
    meta "dangerous_exposure_source/db/silica_db_wrapper.meta"
fn get_i64_raw(key_ptr: uint8, key_len: uint64) -> { tag: int64, value: int64, error_code: int64 };
```

Example layout:

```text
dangerous_exposure_source/db/silica_db_wrapper.h      # C header (authoring reference; not parsed by toolchain)
dangerous_exposure_source/db/silica_db_wrapper.meta    # sidecar metadata
dangerous_exposure_source/lib/libsilica_db.a           # prebuilt archive
```

```silica
module dangerous_db;

wrapper_meta "dangerous_exposure_source/db/silica_db_wrapper.meta";

foreign c_wrapper "silica_db_get_i64"
fn get_i64_raw(key_ptr: uint8, key_len: uint64) -> { tag: int64, value: int64, error_code: int64 };
```

Rules:

- The compiler loads a sidecar file if and only if a `wrapper_meta` or `meta` declaration in the compiling module names that path.
- The compiler does **not** infer sidecar paths from header basenames, directory walks, `silica_<module>_<function>` symbol names, or build-system defaults.
- A module with foreign bindings must declare at least one reachable sidecar path before compilation succeeds.
- Each `foreign c_wrapper "symbol"` declaration must have exactly one matching `wrapper symbol { ... }` entry in a referenced sidecar file.
- A sidecar file may declare one `link_library` name used at link time for all wrappers listed in that file.

The Silica compiler reads explicitly referenced sidecar metadata during compilation. The linker verifies that each referenced wrapper symbol exists in the prebuilt library named by `link_library`.

### 13.2 Required sidecar fields (initial implementation)

Each `wrapper` entry must include at minimum:

```text
wrapper silica_db_get_i64 {
    symbol: "silica_db_get_i64"
    result: "tagged_result"
    error_domain: "dangerous_db"
}
```

Optional argument metadata may describe properties not visible in Silica foreign declarations:

```text
    arguments: [
        { name: "key_ptr", lifetime: "borrowed", retain: false },
        { name: "key_len", role: "length", length_of: "key_ptr" }
    ]
```

Required documented properties include, when applicable:

- borrowed inputs;
- retained external data;
- out-parameters;
- result tag conventions;
- array element type and length source;
- de-opaqueification contract;
- non-recursive struct declarations;
- error-code domain.

`blocking` is not a sidecar field. The cast-only FFI worker architecture makes blocking metadata redundant (§12).

### 13.3 What the toolchain does not validate initially

The initial Silica toolchain does **not** parse C headers or C source. It does not mechanically verify C struct layout, C type spelling, variadic signatures, or wrapper implementation behavior.

Silica-side foreign declarations and sidecar metadata are the source of truth for the Silica-facing ABI. Link time verifies symbol presence in the named prebuilt library only.

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

The Silica compiler rejects any `wrapper_meta` or `meta` path, or any sidecar metadata file referenced by those declarations, located outside `dangerous_exposure_source`.

Required failure:

```text
WrapperMetaPathError:
wrapper_meta and meta paths must be located under dangerous_exposure_source at the root of the Silica project.
```

### 14.2 Prebuilt wrapper libraries (initial implementation)

In the initial implementation, C wrapper object code is supplied as **prebuilt static libraries**. The Silica build tool links these libraries but does not compile C wrapper sources.

Conventions:

- A sidecar file names the library to link: `link_library: "silica_db"`.
- Prebuilt archives live under `dangerous_exposure_source/lib/`, for example `dangerous_exposure_source/lib/libsilica_db.a`.
- The linker must resolve every `foreign c_wrapper "symbol"` used by the program against the libraries named by the referenced sidecar files.

**Link-time failure**:

```text
MissingForeignSymbolError:
Required C wrapper symbol is not defined in the linked prebuilt libraries.
```

### 14.3 Package declarations (future)

A future Silica package format may additionally declare:

- wrapper header files located under `dangerous_exposure_source`;
- wrapper implementation files;
- include paths;
- library search paths;
- libraries to link;
- link mode for each external library, where relevant: dynamically linked shared library or statically linked archive/object library;
- optional `pkg-config` package names;
- target-specific conditions.

Example future package metadata shape:

```text
foreign package db {
    headers: ["dangerous_exposure_source/db/silica_db_wrapper.h"]
    sidecars: ["dangerous_exposure_source/db/silica_db_wrapper.meta"]
    libraries: ["dangerous_exposure_source/lib/libsilica_db.a"]
    link_mode: "static"
}
```

Compilation of C wrapper sources and dynamic linking support are out of scope for the initial implementation described by this specification version.

---

## 15. Compile-Time and Parser Checks

The compile-time and parser checks are divided into two categories:

1. Silica-side checks; and
2. link-time and metadata checks.

### 15.1 Silica-side checks

Silica-side checks are enforced by the Silica parser, type checker, effect checker, and module checker.

Required Silica-side hard errors:

- foreign declaration in a module whose name does not begin with `dangerous_`;
- import or other use of a `dangerous_*` module from a module whose own name does not begin with `dangerous_`;
- raw foreign binding exported directly to application code;
- exported function from a `dangerous_*` module that is not a Silica adapter wrapper with a body;
- module containing `foreign c_wrapper` declarations with no `wrapper_meta` declaration and no per-binding `meta` clause;
- `wrapper_meta` or `meta` path not located under `dangerous_exposure_source`;
- sidecar file referenced by `wrapper_meta` or `meta` that does not exist on disk;
- direct call to any function in a `dangerous_*` module from an application actor behavior or outside an FFI worker actor;
- call to any function in a `dangerous_*` module outside the sequence portion of a `sequence proc[external_danger] ... produces pure ... end` block;
- `dangerous_*` module call outside the sequence portion of a sequence block that declares `external_danger`;
- `dangerous_*` module call inside a sequence block that does not declare `external_danger`;
- actor that initiates foreign work spawned with a behavior that is not cast-only;
- `external_danger` sequence block outside the cast-only behavior function passed directly to `spawn` for an FFI worker actor;
- `external_danger` sequence block inside an ordinary top-level function body;
- `external_danger` sequence block inside a helper function that is not the FFI worker behavior passed directly to `spawn`;
- `external_danger` sequence block inside an application actor behavior;
- `external_danger` sequence block inside a function literal that is not passed directly as an FFI worker behavior to `spawn`;
- `external_danger` sequence block in a nested expression position that is not the direct body of the spawned FFI worker behavior function;
- raw foreign binding declaration using Silica `string` instead of pointer-plus-length arguments;
- more than eight Silica-level arguments after lowering;
- raw pointers in Silica-facing declarations;
- missing or mismatched sidecar metadata entry for a used `foreign c_wrapper` symbol;
- memory region created, modified, or used within `sequence proc[external_danger] ... produces pure ... end` moved out of the FFI worker behavior except through explicit conversion into an actor-state-owned region reference returned as actor state;
- `sequence proc[external_danger] ... produces pure ... end` block producing a value containing external-danger-touched data at any depth;
- memory region created, modified, or used within `sequence proc[external_danger] ... produces pure ... end` included at any depth in an ordinary `call` reply or cast payload;
- external-danger-touched data used inside a sequence block that declares `device_io`, `network_io`, `hot_swap`, or `register_rwr`;
- recursive Silica record shape sent to a C wrapper;
- foreign declaration whose Silica type disagrees with the declared sidecar metadata contract.

Required parser failure:

```text
MissingWrapperMetaError:
A module that declares foreign c_wrapper bindings must declare wrapper_meta paths or a meta path on each foreign binding.
```

Required parser failure:

```text
WrapperMetaPathError:
wrapper_meta and meta paths must be located under dangerous_exposure_source at the root of the Silica project.
```

Required parser failure:

```text
ExternalDangerClientBehaviorError:
Actors that initiate foreign work must use cast-only behaviors.
```

Required parser failure:

```text
ExternalDangerPlacementError:
external_danger sequence blocks are only valid directly inside the cast-only behavior function spawned for an FFI worker actor.
```

Required parser failure:

```text
DangerousDependencyNamingError:
A module that imports or uses a dangerous_* module must use the dangerous_ prefix in its own module name.
```

Required parser failure:

```text
DangerousModuleCallError:
Calls to dangerous_* module functions must appear in the sequence portion of sequence proc[external_danger] ... produces pure ... end inside an FFI worker actor behavior.
```

Required parser failure:

```text
ExternalDangerRegionEscapeError:
Memory regions created, modified, or used inside sequence proc[external_danger] cannot move out of the FFI worker behavior function, except when explicitly converted into actor-state-owned region references returned as actor state, or delivered through an FFI result cast under §7.6.
```

Required type-check failure:

```text
ExternalDangerSequenceResultError:
sequence proc[external_danger] must produce only pure Silica values; external-danger-touched data cannot appear in the produced value.
```

Required type-check failure:

```text
ExternalDangerMessageBoundaryError:
Memory regions created, modified, or used inside sequence proc[external_danger] cannot appear at any depth in an ordinary call reply or cast payload. Only FFI result casts from an FFI worker actor to the designated receiver are permitted.
```

Required type-check failure:

```text
ExternalDangerRestrictedEffectError:
external-danger-touched data cannot be used inside sequence blocks that declare device_io, network_io, hot_swap, or register_rwr.
```

### 15.2 Link-time and metadata checks

Link-time and metadata checks are enforced by the Silica compiler when loading sidecar metadata and by the linker when producing the final binary.

Required hard errors in the initial implementation:

- sidecar file path referenced by `wrapper_meta` or `meta` located outside `dangerous_exposure_source`;
- module with `foreign c_wrapper` declarations and no reachable sidecar reference;
- `foreign c_wrapper "symbol"` declaration with no matching sidecar `wrapper` entry in a referenced sidecar file;
- sidecar entry with no `link_library` declaration for its file;
- required sidecar field missing for a used wrapper (`result` or `error_domain` when applicable);
- referenced prebuilt library archive missing from the project layout;
- wrapper symbol referenced by Silica not defined in the linked prebuilt libraries.

Required link-time failure:

```text
MissingForeignSymbolError:
Required C wrapper symbol is not defined in the linked prebuilt libraries.
```

Required metadata failure:

```text
MissingWrapperMetaError:
A module that declares foreign c_wrapper bindings must declare wrapper_meta paths or a meta path on each foreign binding.
```

```text
WrapperMetaPathError:
wrapper_meta and meta paths must be located under dangerous_exposure_source at the root of the Silica project.
```

The following wrapper-side checks from earlier specification drafts are **deferred** until the toolchain gains C header parsing or wrapper source analysis:

- unsupported C type in wrapper signature;
- variadic wrapper function;
- unsupported C struct layout;
- recursive C struct definition exposed to Silica instead of being flattened, summarized, bounded, or rejected;
- C array return value not mapped to a Silica buffer with known element type and length;
- non-array C pointer return value exposed as a pointer instead of being copied or decoded into its pointee type;
- `void *` value exposed directly to Silica instead of being translated by wrapper code into an approved concrete Silica-facing representation;
- opaque C object exposed to Silica as `uint64`, raw pointer, untyped scalar, handle, or opaque external type instead of being de-opaqueified into Silica-compatible contents;
- wrapper result struct missing a deterministic tag convention;
- wrapper result struct containing uninitialized fields visible to Silica;
- wrapper exposing a C string as NUL-terminated only instead of pointer-plus-length or Silica-compatible string data;
- wrapper retaining actor-stack string or buffer copies after the foreign call ends;
- wrapper failing to provide an explicit error result when it cannot determine a returned pointer's element type, pointee type, length, layout, or de-opaqueified contents;
- Silica-specific C preprocessor macro present in a wrapper header or wrapper implementation.

The deferred checks remain design requirements for wrapper authors even when not mechanically enforced in the initial toolchain.

---

## 16. Open Design Questions

The following questions remain open:

1. Exact sidecar `.meta` field syntax details beyond the initial required fields in §13.2.
2. Standard foreign-call request and FFI result cast message shapes.
3. How many FFI worker actors a program should spawn by convention (shared vs per-domain workers).
4. How client actors derive structurally pure actor state from external-danger-touched FFI result casts under strict structural taint.
5. How region values returned through FFI result casts may be converted into actor-state-owned region references.
6. When and how the toolchain should add C header parsing and deferred wrapper-side checks from §15.2.
7. When the build tool should compile C wrapper sources and support dynamic linking (§14.3).

### 16.1 Resolved design decisions

The following decisions are fixed by this specification version:

| Topic | Decision |
| ----- | -------- |
| Dangerous-module call scope | Every call to any function in any `dangerous_*` module must appear inside an FFI worker `external_danger` sequence. |
| Foreign call transport | All outbound foreign calls use cast to an FFI worker actor; results return by FFI result cast. |
| Client actor shape | Actors that initiate foreign work use cast-only behaviors. |
| Adapter-wrapper detection | Exported `dangerous_*` functions must have a Silica body; raw `foreign c_wrapper` declarations are never exported. |
| String declarations | Raw foreign bindings use pointer-plus-length; adapter wrappers accept Silica `string`. |
| Metadata | Sidecar `.meta` files under `dangerous_exposure_source/`, referenced explicitly by `wrapper_meta` or per-binding `meta` in Silica source. |
| Toolchain validation | No C parsing initially; Silica declarations + sidecar metadata + link-time symbol resolution. |
| Build integration | Prebuilt static wrapper libraries under `dangerous_exposure_source/lib/`. |
| De-taint | Strict structural taint; no validator-based clearing in this version. |
| Foreign call scheduling | Architecturally non-blocking via cast-only client and FFI worker actors; no `blocking` sidecar field. |

Questions about external languages calling into Silica are intentionally excluded from this specification and belong in a separate inbound-interop specification.

---

## 17. Summary

Silica FFI is wrapper-first and cast-mediated.

External libraries are required to be adapted into a small, explicit, predictable C-compatible ABI subset before Silica calls them. Wrapper functions are required to use fixed-width types, pointer-plus-length strings, explicit result structs, de-opaqueified C object contents, and clear ownership rules. Wrapper object code is supplied initially as prebuilt static libraries; sidecar `.meta` files under `dangerous_exposure_source/` declare link libraries and wrapper facts and are loaded only when named by `wrapper_meta` or `meta` declarations in Silica source.

Application actors never call `dangerous_*` module functions directly. They use cast-only behaviors to send foreign-call requests to FFI worker actors. FFI worker actors execute `sequence proc[external_danger] ... produces pure ... end` blocks and deliver outcomes by FFI result cast. Every compilation unit that imports or uses a `dangerous_*` module must itself be a `dangerous_*` module; that constraint propagates to the root application module and to the compiled application name whenever the program depends on FFI.

Raw foreign bindings declare pointer-plus-length arguments for string data; exported adapter wrappers accept Silica `string`. The initial toolchain validates Silica declarations, explicit sidecar references, sidecar metadata contents, and link-time symbol presence. It does not parse C headers.

Any C array return maps to a Silica buffer. Any non-array C pointer must be converted to type before being sent to Silica. Any `void *` from the underlying library must be translated by wrapper code into an approved concrete Silica-facing representation before Silica sees it. Opaque C structs must be de-opaqueified into Silica-compatible contents, not exposed as handles, raw pointers, or opaque external types.

External-danger-touched data must not appear in the `produces pure` value of an `external_danger` sequence. It may appear only in FFI result casts executed from that sequence. Strict structural taint applies in this specification version. External-danger-touched data must not cross ordinary actor `call` or `cast` boundaries and must not be used in sequence blocks declaring `device_io`, `network_io`, `hot_swap`, or `register_rwr`.
