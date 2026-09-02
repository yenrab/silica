---
title: Silica
layout: default
---

<div class="masthead"><img src="./silica_icon_small.png" alt="Silica" width="256" height="256"><span class="masthead-tagline">Secure by default at compile time — fail soft, never fail silent</span></div>

[View the project on GitHub](https://github.com/yenrab/silica) · [Build the compiler](https://github.com/yenrab/silica#building-the-compiler) · [Learn to program]({{ '/learn-programming/' | relative_url }}) · [Silica for programmers]({{ '/learn-silica/' | relative_url }}) · [Silica's Design Principles]({{ '/design-principles/' | relative_url }}) · [Language specification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md) · [Participate]({{ '/participate/' | relative_url }})

On this page

- [Learn](#learn)
- [Silica's Design Principles]({{ '/design-principles/' | relative_url }})
- [Why Silica is worth your attention](#why-silica-is-worth-your-attention)
- [Why participate](#why-participate-in-silicas-development)
- [Get the source](#get-the-source)

Silica's target is to be the world’s most intellectually honest, secure language and runtime—without asking you to fight the tools or the language to get there.

With Silica, security is not a bolt-on checklist; it is woven through the language model, the compiler, and the runtime so that ordinary code reads clearly and dangerous patterns fail during compile-time, with explanations you can act on.

Silica is a highly concurrent, functional, systems language. This means you get explicit effects, actor-based message passing, and region-based memory with no garbage collection. Those choices are spelled out as [Silica's Design Principles]({{ '/design-principles/' | relative_url }}).

Many other development stacks treat memory, concurrency, and observable behavior as separate concerns—each papered over with conventions, code reviews, and runtime luck.

Silica weaves them into one model: regions and lifetimes give memory a coherent story without a garbage collector; actors default to isolation and messages instead of ad hoc sharing; effects on types make mutation and all possible kinds of I/O something the compiler checks, not something teams infer from names and docs.

That integration dramatically narrows where bugs can hide: whole classes of memory and concurrency mistakes fail at compile time with explanations tied to the spec, and ordinary modules stay easier to audit because intent is explicit.

You keep predictable performance and a small conceptual surface area. With Silica, you stay in control without being overwhelmed.

Silica is built for today’s silicon and what real cores actually expose to software today—so you are not stuck translating ideas through obsolete machine models or carry-over abstractions from another era.

Real machines reward locality, parallelism without accidental sharing, and contained failure; the language and runtime are shaped so those ideas stay first-class, not bolted on after the fact or 'optimized in'.

What you write maps to performance aligned with the hardware you can actually buy and use, not to a dangerously nostalgic picture of how chips and computers used to work.

Silica stays honest about FFI through Fifi—the compiler's outbound foreign-function layer.

Think of Fifi as a cute, cuddly-looking poodle that will bite you when you try to pet it: non-Silica code loaded and run by Silica applications looks approachable, but it lives outside Silica's guarantees and can fail in ways pure Silica cannot.

Fifi enforces a wrapper-first FFI contract and the `dangerous_` indicator that refuses to hide C-shaped boundaries behind ordinary Silica code: modules that are foreign wrappers—or use any module whose name carries `dangerous_`—must wear the same indicator themselves, and that naming obligation walks up the dependency graph all the way to your application's root module.

Silica does not hide language boundary crossings behind anonymous imports or reviewers memorizing transitive deps; pure Silica stays visibly pure at the module and artifact level, while mixed stacks advertise themselves without digging through linker maps.

That turns security audits, dependency reviews, release gates, and hand-offs between creators and maintainers into grep-friendly, architectural signals: external calls stay bracketed by `dangerous_*` modules, `external_danger` effect typing, and wrapper-first boundaries the compiler highlights risk, instead of burying it in tribal knowledge.

See the [FFI wrapper specification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica_ffi_wrapper_specification.md), the [dangerous FFI security model](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/dangerous_ffi_security_model.md), and the tutorial on [designing apps with foreign functions](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/tutorials_and_howtos/designing_apps_with_foreign_functions.md).

![](./silica_icon_emoji.png)

Motto: Secure by default at compile time — fail soft, never fail silent

## Learn

Two books on this site:

- [Learn to Program]({{ '/learn-programming/' | relative_url }}) — an introduction to programming for readers who have never written a program. Uses Silica as the teaching language.
- [Silica for Programmers]({{ '/learn-silica/' | relative_url }}) — a short introduction to Silica if you already write software.

[Silica's Design Principles]({{ '/design-principles/' | relative_url }}) states the choices the language is built around. The [language specification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md) and [tutorials](https://github.com/yenrab/silica/tree/main/compiler/silica-compiler/tutorials_and_howtos) are the next step after either book.

## Why Silica is worth your attention

### Security and correctness by design

#### Memory and effects are first-class

Side effects are tracked in types; memory is organized through regions and references with static lifetime reasoning—so many whole classes of bugs never become runnable code. A reference is never separated from the region that holds its memory. Allocating or growing that storage is a `mem(…)` effect, not a silent heap.

See the [language specification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md) (memory model, effects, actors). Related design docs: [actor capabilities and message ordering](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica_actor_capabilities_specification.md) (draft extension) and [memory effects on AArch64 / OS-free targets](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/memory-effects-aarch64-implementation-plan.md) (implementation plan).

#### Memory is allocated in actor stacks, not heaps

Memory allocation happens in each actor’s stack—storage is stack-shaped and flexibly sized, with no per-actor or shared heap for long-lived data.

Lifetimes follow calls and frames, which keeps memory easy to reason about and wipes out typical heap-style mistakes (use-after-free, double-free, leaks, etc.) without a garbage collector.

Sharing stays message-shaped and execution stays predictable.

See [§15.1.2.2 — Actor stack architecture](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md#spec-actor-stack-architecture) (stack allocation, growable stacks, handler-local memory); [§12.1.5 — Region handles and actor spawn](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md#spec-region-handles-actor-spawn) (regions move in at `spawn`); and [§12.1.6 — Region handles in actor messages](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md#spec-region-handles-actor-messages) (regions and related payloads move in `call` and `cast`, including reply ownership on `call`).

#### The compiler rejects “almost right” code

Patterns that optimizers usually patch up—dead bindings, duplicate work, redundant arithmetic, loop-invariant mistakes—are compile-time errors so behavior stays intentional and predictable.

See [additional compiler rules](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification-additional.md).

#### Cryptography gets language-level compiler guardrails

Proposed: secret vs. public labels, constant-time comparisons, no secret-driven control flow, and protected buffers—shifting many crypto mistakes from “hope someone catches it” to “the compiler says no.”

See [crypto proposal](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/crypto-proposal-introduction.md).

#### Formal methods meet engineering

The type system is aligned with a proof-oriented view of programs (Curry–Howard), with a path to richer verification as the toolchain matures.

See [formal verification specification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-formal-verification-specification.md).

### A runtime built for isolation and recovery

#### Unsafe worlds stay outside your safe core

Proposed as a choice instead of FFI. When you must touch C or other unsafe libraries, a brokered IPC design keeps the safe application free of in-process FFI to untrusted code: separate channels, validated messages, no shared memory with the worker, centralized policy—so isolation and recovery are architectural, not aspirational.

When you choose to use the brokered IPC no dangerous indicators are needed in your code.

See [brokered IPC architecture](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/brokered_ipc_isolation_architecture.md).

#### BEAM-inspired fault containment, native speed

The runtime direction is lightweight actors running concurrenlty with independent stacks and no heap, message passing, and “let it crash” semantics at the process level—paired with hardware-assisted safety (e.g. MTE on AArch64) so faults become controlled events, not silent corruption.

See [crash containment design](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/beam_like_crash_containment_design_notes.md).

### Still easy to read, write, and tool

#### Explicit types and syntax

Explicit types and syntax reduce ambiguity for humans and for tools—including structured, spec-linked diagnostics.

The language is intentionally readable and LLM-friendly without sacrificing rigor: clear bindings, pattern matching, and module boundaries.

See [Silica's Design Principles]({{ '/design-principles/' | relative_url }}) (principle 8) and §1.3 of the [language specification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md).

#### No generics maze

Polymorphism through traits and concrete types keeps programs straightforward to navigate compared with heavy type-level programming.

## Why participate in Silica’s development

This is a rare moment: a language whose security story and runtime architecture are being shaped in the open, with deep design docs and a bootstrap path toward a self-hosted compiler on many chips and cross compilers for many others.

Contributing here means influencing:

- how memory safety, concurrency, and effects meet real systems code;
- how isolation and crypto defaults look in practice;
- and how compiler errors and specifications stay aligned so security is teachable, not tribal.

If you care about secure-by-construction systems, native performance, and clarity of intent, Silica is built to reward that investment.

[Where the project is headed]({{ '/participate/' | relative_url }}) organizes in-flight work into parallel language and runtime tracks. The [code organization](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-compiler-code-organization.md) document helps you navigate the tree. How to open issues and PRs is in [CONTRIBUTING.md](https://github.com/yenrab/silica/blob/main/CONTRIBUTING.md).

## Get the source

The compiler, specification, tutorials, and build instructions live in the [GitHub repository](https://github.com/yenrab/silica).

- [Build the compiler](https://github.com/yenrab/silica#building-the-compiler)
- [Silica's Design Principles]({{ '/design-principles/' | relative_url }})
- [Language specification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md)
- [Learn to Program]({{ '/learn-programming/' | relative_url }})
- [Silica for Programmers]({{ '/learn-silica/' | relative_url }})
- [Tutorials](https://github.com/yenrab/silica/tree/main/compiler/silica-compiler/tutorials_and_howtos)
- [Roadmap](https://github.com/yenrab/silica/blob/main/ROADMAP.md)
- [Participate]({{ '/participate/' | relative_url }})
- [Contributing](https://github.com/yenrab/silica/blob/main/CONTRIBUTING.md)
- [Apache License 2.0](https://github.com/yenrab/silica/blob/main/LICENSE)

*Silica: systems programming where security and clarity are part of the language—not an afterthought.*