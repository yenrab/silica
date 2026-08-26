---
title: Participate
layout: default
permalink: /participate/
---

The bootstrap compiler is complete. Ongoing work is grouped by whether it primarily advances the language and toolchain or the runtime platform. Items can move forward on either track as design and implementation allow.

A shorter checklist lives in the [roadmap](https://github.com/yenrab/silica/blob/main/ROADMAP.md). How to open issues and PRs is in [CONTRIBUTING.md](https://github.com/yenrab/silica/blob/main/CONTRIBUTING.md). The [code organization](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-compiler-code-organization.md) document helps you navigate the tree.

## Track 1 — language and compiler

### Phase 2 (in progress)

Here’s where you could help.

#### Immutable lists and lowering

Fast Map, Filter, and Reduce: make immutable list traversals use vector-sized chunks in region-backed storage—so the usual functional pipeline stays expressive in source while the emitter can target SIMD-friendly layouts and avoid redundant allocations.

See [list implementation design](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/list_implementation_design.md) (kernel ops and lowering).

#### Compiler rules and diagnostics

Compiler errors for anti-patterns: the self-hosted compiler reports hard errors for dead bindings, duplicate work, redundant arithmetic, loop-invariant mistakes, and other patterns spelled out in [additional compiler rules](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification-additional.md)—so inefficient or ambiguous code is fixed at the source, not silently “optimized away.”

Fine-tuning compiler errors: refine diagnostics for the current pipeline—clearer messages, stable error codes, accurate locations, and spec-linked references (see §1.6 of the [language specification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md))—so fixing mistakes stays fast while the self-hosted compiler matures.

Carry diagnostic quality forward as further language features land—new rules for crypto, numerics, IPC, and verification-oriented feedback—with the same bar: human-readable and machine-friendly errors that stay aligned with the specification.

#### CI and golden trials

CI trial edge-case additions: grow [compiler/silica-compiler/trials/](https://github.com/yenrab/silica/tree/main/compiler/silica-compiler/trials) with scenarios that stress the self-hosted pipeline—corner cases for parsing, types, effects, and codegen—so `make integrate` stays the gate for regressions on golden assembly (`.ascomp`) and output (`.scout`).

#### Chip, OS, and bare-metal support

The self-hosted toolchain is validated end-to-end on macOS with Apple Silicon (AArch64).

The table is not a timeline and not a rigid pecking order. Planned focus is where work is intended to go next—counting within each strand only (hosted vs bare metal stay parallel). Smaller numbers mean sooner planned attention, not a guarantee every row advances in lockstep or that cross-strand rows compete.

Rows are sorted by that band. When two rows share the same band, Hosted is listed before bare metal for readability, not as ranking one strand above the other. Among bare-metal rows in the same band, ESP32-S3 is listed first—there is already a volunteer driving that bring-up.

If you enjoy ABIs, triples, link steps, CI on new hosts, or bringing up a small runtime on a board with no OS, pick a row and open a discussion or PR. See the [build plan](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/build-plan.md), `TARGET=…` and emitter layout under [compiler/silica-compiler/src_selfhost/emitter/](https://github.com/yenrab/silica/tree/main/compiler/silica-compiler/src_selfhost/emitter), and [hosted vs bare-metal execution](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/execution-environments-hosted-vs-bare-metal.md).

| Planned focus | Strand | Target | Why it helps |
| --- | --- | --- | --- |
| 1 | Hosted (chip + OS) | Linux on AArch64 | Same ISA as today’s primary machine, different syscall/link story; ARM cloud and desktop Linux for contributors and trials. |
| 1 | Bare metal (OS-free, by chip) | ESP32-S3 (Xtensa LX7) | Volunteer in flight—this is the first bare-metal bring-up planned for this chip (ROM/startup, linker, and runtime on a widely used dev-board line). |
| 1 | Bare metal (OS-free, by chip) | AArch64 | Real cores without a full OS; see [memory effects on AArch64 / OS-free targets](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/memory-effects-aarch64-implementation-plan.md); ABI/runtime on a minimal environment. |
| 2 | Hosted (chip + OS) | Linux on x86_64 | Broad server and desktop footprint; strong payoff for CI and for developers not on Apple hardware. |
| 2 | Bare metal (OS-free, by chip) | RISC-V (application-profile cores) | Broad embedded/accelerator footprint; calling convention, linker/platform story, trials or hardware-in-the-loop. |
| 3 | Hosted (chip + OS) | Windows (x86_64; AArch64 when there is demand) | Lowers the barrier for contributors and teams on Windows workstations. |
| 3 | Bare metal (OS-free, by chip) | Common MCU classes (e.g. 32-bit embedded) | Longer tail of boards/ISAs; linker scripts, platform packages, minimal-runtime contract per profile. |

### Phase 3 (next up)

Assembly optimization: tighten and tune AArch64 emission (instruction choice, scheduling, and related emitter paths) for better performance and smaller binaries without weakening the trials’ contract with checked-in baselines.

Compiler architecture: re-engineer `silica-compiler` so it is structured around Silica actors—dogfooding the concurrency model in the toolchain itself while simplifying the compiler's source code.

Extended numerics: first-class big integers, big floats, rationals, and big rationals as distinct explicit types (no implicit widening or automatic promotion between numeric kinds).

Formal methods: deepen Curry–Howard–aligned reasoning and proof tooling on top of the type system. See [formal verification specification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-formal-verification-specification.md).

Cryptography: realize the language-level cryptographic guardrails (secret/public labels, constant-time discipline, protected buffers, and related rules). See [crypto proposal](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/crypto-proposal-introduction.md).

## Track 2 — runtime

### Phase 2 (in progress)

Foreign interoperability (Fifi): call into existing C libraries and into any library that exposes a C-compatible ABI (a stable C calling convention and linkable symbols) via Fifi—the compiler's outbound FFI layer—instead of rewriting the ecosystem in pure Silica.

Non-Silica code loaded and run this way is the poodle that bites: approachable at the boundary, but outside Silica's memory and type guarantees. That lets Silica programs use mature code in unsafe languages to get us started.

For app structure, start with [designing apps with foreign functions](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/tutorials_and_howtos/designing_apps_with_foreign_functions.md). For security-review framing of the `dangerous_*` boundary, see the [dangerous FFI security model](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/dangerous_ffi_security_model.md).

### Phase 3 (next up)

Runtime safety: implement the brokered IPC architecture so unsafe language work can be isolated and mediated as designed. See [brokered IPC architecture](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/brokered_ipc_isolation_architecture.md).

## Compiler-building tools

Tools for generating compiler code and coordinating Phase 2 work live under [compiler/silica-compiler/compiler-building-tools/](https://github.com/yenrab/silica/tree/main/compiler/silica-compiler/compiler-building-tools).

For JSON-LD agent graphs, which files to use, and how they fit AI-assisted workflows, see [compiler-building-tools/README.md](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/compiler-building-tools/README.md).

---

[Back to Silica]({% link index.md %}) · [View the project on GitHub](https://github.com/yenrab/silica)
