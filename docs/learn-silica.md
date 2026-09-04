---
title: Silica for Programmers
layout: default
permalink: /learn-silica/
---

# Silica for Programmers

**A short introduction if you already write software**

Copyright © 2026 Lee Scott Barney

Silica is a functional systems language. Effects are explicit. Actors stay isolated and talk by message. Memory is regional, stacked per actor, and not garbage-collected. There are no user-level loops. This book assumes you already write software in C, Rust, Erlang or Elixir, Haskell, Go, Python, or something in that neighborhood. It maps Silica onto ideas you already have, then spends a little time on why the mapping is not one-to-one.

If you have never programmed, use [Learn to Program]({{ '/learn-programming/' | relative_url }}) first. That book is slower on purpose.

The [language specification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md) wins if this book and the compiler disagree. The hosted target today is macOS on Apple silicon.

Simple runnable programs live in `[trials/](https://github.com/yenrab/silica/tree/main/trials)`. Each subdirectory is one topic. The snippets in this book are maps of the idea. Open those files when you want a program that is meant to compile and run.

## 1. Positioning

You will recognize most of the pieces. The combination is the point. Silica puts effects, isolation, and memory into one model so the compiler can refuse programs that other languages would accept and then hope a review or a collector saves.

| You know                              | Silica’s analogue                        | Difference that matters                                                                                                                     |
| ------------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust ownership / lifetimes            | Regions + move of region handles         | No borrow-checker theatre. A lifetime is a value from `fresh_lifetime()`. Storage is the actor stack, not a heap the collector later walks. |
| Erlang/Elixir actors                  | `spawn` / `call` / `cast`, supervisors   | Native code. A behavior is `(Msg, State) -> …` and runs once per message. The runtime owns the mailbox loop.                                |
| Haskell `IO` / `ST`                   | `sequence proc[ε] … produces pure e end` | Effects live on sequences, never on `fn` signatures.                                                                                        |
| C pointers + malloc                   | `region`, `ref`, `buf`, `alloc_*`        | Space (`normal`, `atomic`, `device`, …) is part of the type. There is no implicit free.                                                     |
| Go goroutines + channels              | Actors + messages                        | Isolation is the default. There is no shared mutable heap for ordinary data.                                                                |
| ML / OCaml ADTs                       | Inline sums, tagged tuples, atoms        | No user-declared type names. You write the shape at the use site.                                                                           |
| C++ / Java generics                   | Traits over concrete inline types        | No `T` parameters in user code.                                                                                                             |
| Python names, `if`/`for`, GC, `print` | Bindings, `case`, regions, `device_io`   | Types and effects are written down. Names do not mutate. There is no heap collector.                                                        |

The design rules, in the order you will feel them:

- **Effects are explicit.** If a block prints, allocates, or sends a message, a `sequence` says so. Callers inherit that obligation.
- **Heap is an effect.** Allocating or growing region-backed storage is `mem(<space>)` on a sequence, not an invisible runtime service.
- **Memory is regional and stack-shaped per actor.** There is no garbage collector and no shared heap for long-lived application data. A `ref` or `buf` is never separated from the region that contains the memory it refers to.
- **Concurrency is message passing.** Isolation comes first. Sharing is a deliberate, typed move of a region, not a default. A well-designed application is a set of actors, not a tree of function calls.
- **Readable for people and for LLMs.** One way at the language level: recursion, not loops; traits, not generics; one operator per job. You compose those pieces; you do not pick among several primitives that do the same thing. There are no type aliases: the shape at the use site is the type, so nothing has to be unfolded through a definition set.
- **FFI is visible.** Mixed stacks wear `dangerous_` up the import graph. Pure Silica stays visibly pure.

Motto: *secure by default at compile time — fail soft, never fail silent.*

## 2. Syntax in one pass

A function has a name, typed parameters, a typed result, and a body. `main` is the entry. When the body talks to the world, it does that inside a `sequence` that names the effects.

```silica
fn add(x: int64, y: int64) -> int64 {
    x + y
}

fn main() -> int64 {
    sequence proc[device_io]
        println("hi");
        n: int64 <- add(2, 3);
    produces
        pure n
    end
}
```

`add` is pure: two `int64` values in, one out. `main` prints, then binds `n`, then produces `n`. The `device_io` on the sequence is what authorizes the print. `<-` is a bind, not a mutation. `produces pure n` is the result of the sequence; `pure` means that expression does not start any new effect.

Operators you will hit immediately:

| Token        | Role                            |
| ------------ | ------------------------------- |
| `<-`         | Bind a name to a value          |
| `=` / `==`   | Equality (not “declare a type”) |
| `->`         | Function type, or a `case` arm  |
| `:`          | Type ascription                 |
| `@`          | Module qualify: `math@add`      |
| `//` `{- -}` | Comments                        |
| `:ok`        | Atom literal                    |

There is no standalone `if`. You branch with `case`. The keyword `if` exists only as a guard on a case arm: `n: int64 if n > 0 -> …`. Python `if` / `elif` / `else` and unguarded `match` do not exist here.

Catch-alls are typed: `_: int64 -> 0`. A bare `_` is illegal. Python `match` allows `_`; Silica will not, because the type of the leftover value still has to be written down.

`and` / `or` / `not` are boolean. Comparisons are `==` `!=` `<` `>` `<=` `>=`. Integer `/` truncates toward zero. That is closer to Python `//` than to `/`, except Python `//` floors and Silica truncates toward zero.

Functions are top-level only, and they take at most eight parameters. A nested `fn name` declaration is an error. Lambdas (`fn(x: int64) -> int64 { x + 1 }`) are fine in expression position. There are no Python-style nested `def`s, no `*args`, and no default arguments. If you need more than eight inputs, group them in a tuple or a record.

The entry point is `main`. There is no `if __name__ == "__main__"`.

First program: `[trials/base/test.silica](https://github.com/yenrab/silica/blob/main/trials/base/test.silica)`. Arithmetic: `[int64_addition](https://github.com/yenrab/silica/tree/main/trials/int64_addition)`. Functions: `[functions_addition](https://github.com/yenrab/silica/tree/main/trials/functions_addition)` (`add_with_params.silica`, `fn_two_functions.silica`).

## 3. Types are structural

Silica does not let you introduce a new type name. There is no `type Foo = …`, no `struct Foo { … }`, and no user `enum` that binds a fresh identifier. You write the shape wherever a type is required — on a parameter, a return, a binding, a pattern. Python `class`, `dataclass`, `TypedDict`, and `TypeAlias` are the same kind of thing, and they are absent too.

```silica
fn area(r: { width: int64, height: int64 }) -> int64 {
    r.width * r.height
}

fn wrap(n: int64) -> (atom, int64) {
    (:ok, n)
}
```

`area` takes a record that has `width` and `height`. Any value with those fields and those field types is acceptable. Two records with the same fields and field types are the same type. The same rule holds for tuples and for inline sums such as `Some(int64) | None`. A Python `dict` with the same keys is not a type. Two dataclasses with the same fields are still different classes.

`None` in that sum is a **constructor name**, a tag with no payload. It is not Python’s `None`. The unit value — “there is no interesting result” — is `()`. Do not use `None` when you mean unit.

Everyday primitives: `int64`, `uint64`, `boolean`, `string`, `char`, `atom`, `()`, `float64`. Width-specific ints and floats exist when you need them. There is no implicit widening. Python `int` is unbounded; Silica `int64` is not.

Function types are written `(int64, int64) -> int64`. They are required, not optional annotations.

Lists are `List[int64]` in casual description, or `List[int64, mem(normal)]` when the memory space is part of the type. That space must agree along a value’s whole flow. Do not write `List[int64]` in one place and `List[int64, mem(normal)]` in another for the same list. The trials use the two-parameter form.

Atoms are interned at compile time. Equality is identity. Use them as tags (`:ok`, `:error`, `:noreply`) rather than as strings. They are not Python `Enum` members and not interned `str` constants.

Widths and literals: `[int64_addition](https://github.com/yenrab/silica/tree/main/trials/int64_addition)`, `[boolean_addition](https://github.com/yenrab/silica/tree/main/trials/boolean_addition)`, `[atoms_addition](https://github.com/yenrab/silica/tree/main/trials/atoms_addition)`, `[string_addition](https://github.com/yenrab/silica/tree/main/trials/string_addition)`. Records: `[records_addition/01_basic_struct_creation.silica](https://github.com/yenrab/silica/blob/main/trials/records_addition/01_basic_struct_creation.silica)`.

## 4. Functions, bindings, case

A function is the unit of reuse. A binding gives a value a name for the rest of the scope. `case` is how you choose.

```silica
fn abs(n: int64) -> int64 {
    case n of {
        x: int64 if x >= 0 -> x;
        x: int64 -> 0 - x
    }
}

fn labeled(n: int64) -> string {
    case n of {
        x: int64 if x > 0 -> "pos";
        x: int64 if x < 0 -> "neg";
        _: int64 -> "zero"
    }
}
```

`abs` binds the scrutinee as `x` and uses a guard for the non-negative side. The second arm covers the rest. `labeled` needs three outcomes, so the last arm is a typed catch-all.

Bindings always carry a type: `n: int64 <- 3`. Names do not mutate. If you need a different value, bind another name. `next: int64 <- n + 1` is a new binding. It is not Python `n += 1`.

`case` is exhaustive. A guard does not cover the values that fail the guard. Keep a typed catch-all, or cover the remaining constructors. Python `match` is exhaustive only if you opt in. Silica’s `case` always is. That is the feature: a forgotten arm is a compile error, not a runtime surprise.

When an arm needs more than one step, wrap it in `{ … }`:

```silica
case ready of {
    true -> {
        next: int64 <- n + 1;
        next
    };
    false -> n
}
```

Records and tuples pattern-match in the obvious way: `{ width: w, height: h }`, `(tag: atom, n: int64)`.

Working programs: `[case_addition](https://github.com/yenrab/silica/tree/main/trials/case_addition)` (`case_boolean_literal_branches.silica`, `case_int64_mirror_sign.silica`). Bindings and helpers: `[functions_addition](https://github.com/yenrab/silica/tree/main/trials/functions_addition)`.

## 5. Sequences and effects

A `sequence` is ordered steps with a marked result. Effects are declared on that block, not on the function signature. This is illegal:

```silica
fn boom() -> int64 proc[device_io] { … }   // no
```

This is legal:

```silica
fn boom() -> int64 {
    sequence proc[device_io]
        println("x");
    produces
        pure 0
    end
}
```

The signature of `boom` is only `() -> int64`. The sequence admits `device_io`. `produces pure 0` is the value that comes out; `pure` means `0` itself does not start a new effect. Nested sequences — including sequences inside lambdas — each declare what they need. Effects propagate up: a caller of `boom` must already sit in a sequence that admits `device_io`.

That is the honesty rule. A helper that prints cannot hide inside an innocent-looking `fn`. You see the effect at every layer that can reach it.

Built-in effects:

| Effect            | Means                                                 |
| ----------------- | ----------------------------------------------------- |
| `device_io`       | stdout, console, files                                |
| `network_io`      | sockets, HTTP, and other network I/O                  |
| `concurrency`     | `spawn`, `call`, `cast`, and related actor work       |
| `mem(Space)`      | allocate, read, or write in that memory space         |
| `atomic`          | atomic operations                                     |
| `hot_swap`        | dynamic code load                                     |
| `register_rwr`    | MMIO; only inside `spawn_device` behaviors            |
| `external_danger` | outbound FFI; only inside `spawn_dangerous` behaviors |

Print helpers require `device_io`. In Python, `print` is an ordinary call. Here the sequence must admit the effect, and every caller up the chain must too. The trials use `print_string`, `print_bool`, and `println` — open those files for the spelling that compiles today.

On OS-hosted targets, `mem(Space)` is still in the type system. Distinct hardware attributes per space are guaranteed on OS-free targets, not on macOS or Linux process virtual memory. The OS still chooses the page attributes; Silica still makes you name the space you meant.

A sequence that only produces `42`: `[sequence_block_addition](https://github.com/yenrab/silica/tree/main/trials/sequence_block_addition)`. Effects on sequences: `[effect_check_addition](https://github.com/yenrab/silica/tree/main/trials/effect_check_addition)` (`device_io_in_sequence.silica`). Print: `[string_addition/test_print.silica](https://github.com/yenrab/silica/blob/main/trials/string_addition/test_print.silica)`.

## 6. Data

**Tuples** are ordered groups. `(3, true)` has type `(int64, boolean)`. Unpack with `(n: int64, b: boolean) <- pair`. Position is the API, as with a Python tuple, but the types are required.

**Records** are named fields. The value `{ x: 1, y: 2 }` has type `{ x: int64, y: int64 }`. Field access is `p.x`. This is not a `dict`, not a `namedtuple`, and not a dataclass instance. The shape *is* the type.

**Lists** are immutable, head-oriented, and shared structurally. They are not a Python `list`: there is no `xs[i]`, no in-place `append` or `pop`, and no slice assignment.

```silica
xs: List[int64] <- [1, 2, 3]: List[int64];
ys: List[int64] <- prepend[int64](0, xs);   // xs unchanged
```

`prepend` returns a new list. `xs` is still `[1, 2, 3]`. Growing a list allocates, so it belongs in `sequence proc[mem(Space)]`. There is no primitive that deletes from the middle. In the trials, list types are written `List[int64, mem(normal)]`, and walking a list is usually a `case` on `[]` and `[h, t]` rather than a family of `head` / `tail` calls. Prefer those files when you write something you expect to compile.

**Tagged results** are how you represent failure. Prefer data over exceptions. There is no `try` / `except`, no `raise`, and no Python `None` meaning “missing.”

```silica
fn safe_div(x: int64, y: int64) -> (atom, int64) {
    case y == 0 of {
        true -> (:error, 0);
        false -> (:ok, x / y)
    }
}
```

The caller must look at the atom. Forget an arm and `case` is incomplete.

**Recursive tuples** replace named recursive ADTs. Self-reference is the keyword `rec` inside a tuple, allocated in a region, with `ref?` / `:none` for the empty case. A Python class with a `next` field hides allocation. Here the region is explicit. See [why no named types](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/tutorials_and_howtos/why_no_named_types.md).

**Strings** are UTF-8. Join and slice with the string operations (`concatenate` / `concat`, `substring` with character indices, `length_chars` / `length_bytes`, `starts_with` / `ends_with` / `contains`). There is no `+` for strings, no f-string, and no `str.format`. See `[string_addition](https://github.com/yenrab/silica/tree/main/trials/string_addition)` (`test_concat_literals.silica`, `test_length_chars.silica`).

Tuples: `[tuples_addition](https://github.com/yenrab/silica/tree/main/trials/tuples_addition)` (`int64_pair.silica`, `decompose_from_literal.silica`). Records: `[records_addition](https://github.com/yenrab/silica/tree/main/trials/records_addition)`. Lists: `[list_addition](https://github.com/yenrab/silica/tree/main/trials/list_addition)` (`list_int64_prepend.silica`, `list_int64_recursive_sum.silica`). Atoms: `[atoms_addition](https://github.com/yenrab/silica/tree/main/trials/atoms_addition)`.

## 7. Recursion only

There is no `for`, `while`, or `loop`. You walk data by recursion. There is no Python `for x in xs`, no `while`, and no comprehension. The mailbox “infinite loop” is inside the runtime. Your actor behavior returns.

```silica
fn sum(xs: List[int64]) -> int64 {
    case is_empty[int64](xs) of {
        true -> 0;
        false -> head[int64](xs) + sum(tail[int64](xs))
    }
}
```

The empty list is the base case. Everything else is the head plus the sum of the rest. Forget the base case and you ask the machine to work forever.

Write the stopping case first. That is the habit that keeps recursion honest. The thing that stresses an actor stack is deep work **during one message**, not the number of messages over time. Each behavior invocation returns before the next message is received.

The same idea without lists is factorial; with lists, a recursive `case` on `[h, t]`. See `[recursive_function_addition](https://github.com/yenrab/silica/tree/main/trials/recursive_function_addition)` (`recursive_factorial.silica`, `recursive_sum_tail.silica`) and `[list_addition/list_int64_recursive_sum.silica](https://github.com/yenrab/silica/blob/main/trials/list_addition/list_int64_recursive_sum.silica)`.

## 8. Modules and traits

A file is a module. The file stem is the module name. You export by name and arity; nothing is public by accident.

```silica
export add/2;

fn add(x: int64, y: int64) -> int64 {
    x + y
}
```

`export add/2` means this module offers `add` with two parameters. Importers qualify the call. That is closer to `import math` / `math.add` than to `from math import add`:

```silica
use math;

fn main() -> int64 {
    math@add(2, 3)
}
```

There is no implicit public `def` and no `__init__.py` package tree. `use` plus `@` keeps the origin of a name visible.

Traits are files, not `trait T { }` blocks and not Python ABCs or `Protocol`. `shape.silica` declares `export trait Shape;`, lists `required` and `provided` methods, and supplies `impl fn` for concrete inline types. Callers write `use shape;` and `shape@area(rect)`.

There is no `Option<T>`. Shared operations live on traits implemented for concrete sums such as `Some(int64) | None`. Marker traits (`ActorMessage`) are compile-time tags. In that sum, `None` is a constructor, not Python `None`.

If two compilation units need to recurse into each other, do not create a `use` cycle. Pass the recursive entry as a callback. See [open recursion](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/tutorials_and_howtos/open_recursion_callbacks.md).

Modules: `[modules_addition](https://github.com/yenrab/silica/tree/main/trials/modules_addition)` (`one_use_main.silica`, `lib/lib_base.silica`). Traits: `[traits_addition](https://github.com/yenrab/silica/tree/main/trials/traits_addition)` (`shape_main.silica`, `traits/Shape.silica`).

## 9. Regions

Allocation is not `malloc`, and it is not “the Python heap will get it.” You create a region, then allocate into it. Handles move. A `ref` does not outlive its region. The compiler rejects a `ref` returned without that region: at the end of a sequence the region is freed unless the result still contains the handle. There is no refcounting, no cyclic-GC pause, and no `del`.

To keep a cell after the sequence, produce the region and the `ref` together:

```silica
sequence proc[mem(normal)]
    L1: lifetime <- fresh_lifetime();
    r: region(L1, normal) <- alloc_region(normal);
    cell: ref(L1, normal, int64) <- alloc_ref(r, 42);
    _: atom <- write_ref(cell, 43);
produces
    pure (r, cell)
end
```

`fresh_lifetime()` gives you a unique `L1`. `alloc_region` creates the arena. `alloc_ref` puts a cell in that arena. `write_ref` updates the cell. The effect is `mem(normal)` because that is the space you named. `(r, cell)` keeps the arena alive in the caller; `cell` remains valid because `r` is still owned.

This is rejected and produces a compiler error — `cell` would dangle after `r` is freed at `end`:

```silica
sequence proc[mem(normal)]
    L1: lifetime <- fresh_lifetime();
    r: region(L1, normal) <- alloc_region(normal);
    cell: ref(L1, normal, int64) <- alloc_ref(r, 42);
produces
    pure cell
end
```

A value copied out with `read_ref` is an ordinary `int64`, not a `ref`. That copy can be produced without `r`; the region is then freed. That is a different case from returning the cell.

`region(L, Space)` is the arena. `ref(L, Space, T)` is a cell. `buf(L, Space, T, N)` is a fixed buffer. Do not invent two regions that pretend to share `L`. Allocation is only legal inside `sequence … end`.

When a region handle is passed to `spawn` or sent in a `call` / `cast`, ownership moves. After the send, the sender must not use the handle. A reply can move ownership back. That is the same discipline as a function argument that you are not allowed to use after a move.

Memory lives on the actor’s stack, which can grow. Lifetimes follow frames and messages, not a collector. Python objects are shared by default. Silica values are not.

Spaces (`normal`, `normal_writethrough`, `atomic`, `device`, …) are part of the type. Full hardware distinction is for OS-free targets. On a hosted OS the discipline still holds; the attributes are OS-chosen.

Tutorials: [memory region types](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/tutorials_and_howtos/memory_region_types.md), [region handles and references](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/tutorials_and_howtos/region_handles_and_references.md).

Regions: `[memory_region_addition](https://github.com/yenrab/silica/tree/main/trials/memory_region_addition)` (`alloc_region_normal.silica`, `alloc_ref_int64.silica`, `read_ref_int64.silica`, `write_ref_int64.silica`).

## 10. Actors

A well-designed Silica application is a set of actors that pass messages. Functions still do the work *inside* a turn — they are not the shape of the program. If the architecture is only `main` calling helpers calling helpers, you have not yet designed the application.

There is no `actor` keyword. A handler is an ordinary function. `spawn` is what makes it an actor. This is not a thread, not `asyncio`, and not `multiprocessing.Queue`. Isolation is the default. There is no shared object graph and no GIL to reason about.

A **call-only** behavior always replies:

```silica
fn counter(msg: int64, state: int64) -> (:reply, int64, int64) {
    total: int64 <- state + msg;
    (:reply, total, total)
}
```

A **cast-only** behavior never replies:

```silica
fn log(msg: string, state: int64) -> (:no_reply, int64) {
    sequence proc[device_io]
        println(msg);
    produces
        pure (:no_reply, state + 1)
    end
}
```

One behavior is one convention. The compiler tracks call-only versus cast-only on each `actor_ref`. `call` on a cast-only ref is a type error. You do not write a union of `(:reply, …)` and `(:no_reply, …)` and hope.

```silica
fn main() -> int64 {
    sequence proc[concurrency]
        a: actor_ref <- spawn(0, counter);
        n: int64 <- call(a, 3 impl ActorMessage {});
    produces
        pure n
    end
}
```

`spawn(0, counter)` starts an actor whose state is `0`. `call` sends `3` and waits for the reply. Messages need `ActorMessage`, usually written `expr impl ActorMessage {}` at the send site.

The runtime model is Erlang `gen_server`, not a user-level receive loop:

1. The runtime receives a message.
2. Your function runs once, with that message and the current state.
3. You return `(:reply, Reply, State)` or `(:no_reply, State)`.
4. The runtime stores the new state and waits for the next message.

`call` blocks for `Reply`. `cast` does not. A dead target raises `actor_not_found`. If the target dies with outstanding `call`s, those callers get the actor-death result. A restarted actor does not inherit the failed actor’s mailbox.

Supervisors are a different handle (`supervisor_ref`). You maintain them with `call_supervisor`, not ordinary `call` / `cast`. See the [supervisors tutorial](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/tutorials_and_howtos/supervisors_and_failure_reporter_tutorial.md).

`spawn` has variants for pinning, registration, device workers, and FFI workers. Migration strategy (`lazy`, `eager_copy`, `static_core`) is about how much stack one message uses and how often the actor moves, not about “how long the actor lives.” [Spawning tutorial](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/tutorials_and_howtos/actor_spawning_tutorial.md).

Actors: `[actors_addition](https://github.com/yenrab/silica/tree/main/trials/actors_addition)` (`actor_boolean_state_reply.silica`, `actor_cast_fire_and_forget.silica`). Supervisors: `[supervisors_addition](https://github.com/yenrab/silica/tree/main/trials/supervisors_addition)`. Pinning: `[cpu_discovery_and_spawn_pinning](https://github.com/yenrab/silica/tree/main/trials/cpu_discovery_and_spawn_pinning)`.

## 11. Fifi

Fifi is outbound FFI: the way a Silica program calls C or anything with a C ABI. Non-Silica code is outside Silica’s memory and type guarantees. This is not `ctypes`, not `cffi`, and not `subprocess` dressed up as a function.

The rules that matter in practice:

- **Wrapper-first.** You do not call C as if it were Silica. Python C-API extensions are in-process and silent. Fifi is neither.
- **Names carry the risk.** Modules that wrap foreign code, or that `use` such a module, take a `dangerous_` name.
- **The name walks up.** That obligation continues to the application root. Pure Silica stays visibly pure at the module and artifact level.
- **Foreign calls run in an FFI worker.** `spawn_dangerous` installs the worker. The worker’s sequence admits `external_danger`. Callers at the spawn site do not get that effect.
- **Device MMIO is a different door.** `spawn_device` / `register_rwr`, not Fifi.

The audit story is `grep dangerous_`. Mixed artifacts advertise themselves. You should not need a linker map or tribal knowledge to see that a release is no longer pure Silica.

Read: [designing apps with foreign functions](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/tutorials_and_howtos/designing_apps_with_foreign_functions.md), [FFI wrapper spec](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica_ffi_wrapper_specification.md), [dangerous FFI model](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/dangerous_ffi_security_model.md).

A proposed alternative to in-process FFI is [brokered IPC](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/brokered_ipc_isolation_architecture.md): keep the unsafe work out of process so the safe application does not load it at all.

FFI trials: `[ffi_addition](https://github.com/yenrab/silica/tree/main/trials/ffi_addition)`. Compile-fail goldens for the taint rules: `[error_enforcement_addition](https://github.com/yenrab/silica/tree/main/trials/error_enforcement_addition)`.

## 12. What the compiler rejects

Silica would rather stop you than “optimize away” a mistake or wait for a test to notice it. These are hard errors, not warnings you can train yourself to ignore, and not Python’s “run it and see”:

- **Dead bindings** — a name you bound and never used.
- **Duplicate work** — the same computation written twice when once would do.
- **Redundant arithmetic** — additions of zero, multiplications by one, and similar noise.
- **Loop-invariant mistakes** — the recursive equivalent of “this does not change in the loop.”
- **Missing effects** — a print, allocate, or send without the matching `proc[…]`.
- **Non-exhaustive** `case` — a value that no arm covers.
- **Untyped** `_` — a catch-all that does not name the leftover type.
- `if` **used as a statement** — use `case`.
- **Nested** `fn` **declarations** — helpers go at the top level, or use a lambda.
- **More than eight parameters** — group them.
- `call` **/** `cast` **convention mismatch** — a call-only ref used as cast-only, or the reverse.
- **Region, lifetime, or isolation violations** — a `ref` returned without its region, a ref that outlives its region, a handle used after a move.
- `dangerous_` **taint that was not declared** — a foreign dependency that did not walk up the module graph.

Diagnostics carry a code (`E2000`, …), a location, and a spec section. Read the human sentence first, then the location, then the `See specification` pointer. See spec §1.6 and [additional compiler rules](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification-additional.md).

Programs the compiler is supposed to refuse: `[error_enforcement_addition](https://github.com/yenrab/silica/tree/main/trials/error_enforcement_addition)`, `[warning_enforcement_addition](https://github.com/yenrab/silica/tree/main/trials/warning_enforcement_addition)`.

## 13. Next

Read the trials when you want a small program in hand. Read the specification when you want the rule. The tutorials are the middle ground for actors, regions, and FFI.

- [trials](https://github.com/yenrab/silica/tree/main/trials) — simple programs, grouped by topic. Run one directory with `make -C trials/<name> integrate`.
- [Language specification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md)
- [Tutorials](https://github.com/yenrab/silica/tree/main/compiler/silica-compiler/tutorials_and_howtos)
- [Build the compiler](https://github.com/yenrab/silica#building-the-compiler)
- [Participate]({{ '/participate/' | relative_url }})
- [Learn to Program]({{ '/learn-programming/' | relative_url }}) — same language, slower on-ramp

*End of Silica for Programmers.*

Copyright © 2026 Lee Scott Barney