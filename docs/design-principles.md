---
title: Silica's Design Principles
layout: default
permalink: /design-principles/
---

# Silica's Design Principles

These eight principles are the choices Silica is built around. They decide how the language looks, what the compiler will accept, and how programs run.

The [language specification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md) is the normative source (§1.2 and §1.3). This page is the reader-facing statement of the same list.

[Back to Silica]({% link index.md %})

<div class="toc" markdown="1">

On this page

1. [Explicit effects](#1-explicit-effects)
2. [Process monad](#2-process-monad)
3. [Actor-based concurrency](#3-actor-based-concurrency)
4. [Region-based memory](#4-region-based-memory)
5. [No loops](#5-no-loops)
6. [AArch64-native](#6-aarch64-native)
7. [No generics](#7-no-generics)
8. [LLM-friendly and human-readable](#8-llm-friendly-and-human-readable)
    - [Explicit type annotations](#81-explicit-type-annotations)
    - [Unambiguous syntax patterns](#82-unambiguous-syntax-patterns)
    - [Consistent naming conventions](#83-consistent-naming-conventions)
    - [Concrete types over generics](#84-concrete-types-over-generics)
    - [Structured pattern matching](#85-structured-pattern-matching)
    - [Explicit effect tracking](#86-explicit-effect-tracking)
    - [Module and import clarity](#87-module-and-import-clarity)

</div>

## 1. Explicit effects

All side effects are tracked in type signatures.

Mutation, I/O, concurrency, and other observable behavior are not inferred from names or comments. They are declared, checked, and visible at the call site’s surrounding sequence.

## 2. Process monad

Sequential computations are represented as monadic processes.

Pure functions stay ordinary functional code. Effectful work is a `proc[…]` — a sequenced computation whose effects are part of its type, not hidden control flow.

## 3. Actor-based concurrency

Message passing is the primary concurrency mechanism.

Actors are isolated. They communicate by `call` and `cast`, not by sharing mutable memory. Isolation is the default; sharing is a deliberate, typed exception.

## 4. Region-based memory

Memory is managed with regions, not a garbage collector.

Ownership and lifetimes are static. There is no shared heap for ordinary long-lived data. Storage follows actor stacks and region handles, so typical heap mistakes never become runnable code.

## 5. No loops

Source code uses recursion only. The runtime may loop internally.

You write recursive functions and folds. The compiler and runtime are allowed to lower that recursion into loop-shaped machine code. There are no user-visible `for` or `while` loops.

## 6. AArch64-native

First-class support for ARM hardware features.

Silica is designed for AArch64, not for a portable 1970s machine model. Memory spaces, vectors (NEON/SVE), Memory Tagging (MTE), and Pointer Authentication (PAC) are part of the language story, not after-the-fact backends.

## 7. No generics

Polymorphism is achieved through traits, not generic type parameters.

There is no `Option<T>` or `Result<T, E>`. Shared operations live on traits implemented for concrete, inline types. Every type occurrence in source is syntactically concrete.

## 8. LLM-friendly and human-readable

Syntax and semantics are meant to be easy for people to read and for tools — including large language models — to parse without guesswork.

That means explicit type annotations, unambiguous operators, consistent naming, and clear structural patterns. The rest of this page is the expansion of this principle.

### 8.1 Explicit type annotations

Function parameters and return types are always written out:

```silica
fn add(x: int64, y: int64) -> int64
```

Variable bindings carry their type:

```silica
value: int64 <- 42
```

Pattern matching uses typed patterns:

```silica
case x of { n: int64 -> n * 2 }
```

Catch-all patterns must declare a type: `_: int64 -> 0`, not `_ -> 0`.

**Benefit:** Types are unambiguous for tools and immediately visible to people.

### 8.2 Unambiguous syntax patterns

Different operations use different operators:

- `<-` for binding (left-to-right flow)
- `=` for equality comparison, and for bindings only where the grammar already allows it — not for declaring named types
- `->` for function types and case branches

Effect declarations sit on sequences: `sequence proc[device_io, concurrency]`.

Blocks have distinct delimiters: `sequence … produces pure … end` for sequences, `{ … }` for case expressions.

**Benefit:** Structure is obvious. Parsers and readers do not have to disambiguate overloaded punctuation.

### 8.3 Consistent naming conventions

Built-in types (`int64`, `string`, `List[…]`, and the rest) use the spellings in the specification. Composite types are structural and written inline; they are not introduced as new type names.

Functions and variables use `snake_case`: `add_numbers`, `my_value`.

Keywords are lowercase: `fn`, `struct`, `trait`, `impl`.

**Benefit:** Predictable patterns that both people and models can apply consistently.

### 8.4 Concrete types over generics

Silica has no generic type parameters. There is no `Option<T>` or `Result<T, E>` in the type system.

Optional and fallible values are ordinary sum types (tagged unions) with concrete payloads written inline wherever a type is required, for example `Some(int64) | None` or `Ok(int64) | Error(string)`.

There are no user-declared type names. The same inline sum shapes appear in application code and in library APIs.

Traits such as `OptionLike` and `ResultLike` supply shared operations by implementing them for specific inline type expressions, without introducing generics.

**Benefit:** Every type in source is concrete. There is no generic-inference puzzle for tools or for readers.

### 8.5 Structured pattern matching

Patterns carry types: `n: int64 if n > 0 -> …`.

Catch-alls require a declared type: `_: int64 -> 0`.

Guards are separated from the pattern: `pattern if condition -> expression`.

The compiler checks that every case is covered.

**Benefit:** Control flow is explicit enough to analyze, and complete enough to verify.

### 8.6 Explicit effect tracking

Side effects are declared on sequence blocks:

```silica
sequence proc[device_io, mem(normal)]
```

Function declarations do not carry effect declarations. Effects appear only on sequences, and requirements propagate through those sequences.

**Benefit:** Observable behavior is visible to people and to tools, so analysis does not depend on naming conventions.

### 8.7 Module and import clarity

Modules and imports are explicit: `use calculator;`.

Exports are declared: `export add/2;`.

Module structure matches file structure.

**Benefit:** Dependencies are named and traceable. Cross-module calls say which module they use.

---

See the [language specification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md) for the full rules these principles imply.

[Back to Silica]({% link index.md %})
