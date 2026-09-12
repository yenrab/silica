---
title: Participate
layout: default
permalink: /participate/
---

The bootstrap compiler is complete and the self-hosted compiler runs the trials. Work is organised by **emitter path** and delivered in **chunks between fixed points**; the authoritative checklist is the [roadmap](https://github.com/yenrab/silica/blob/main/ROADMAP.md). How to open issues and PRs is in [CONTRIBUTING.md](https://github.com/yenrab/silica/blob/main/CONTRIBUTING.md). The [code organization](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-compiler-code-organization.md) document helps you navigate the tree, and [Build and test the compiler]({{ '/build-and-test/' | relative_url }}) explains generations, trials, and the fixed-point check.

## Fixed points and chunks

A **fixed point** is a self-hosted compiler that passes the whole trial tree when built by itself and then reproduces itself byte for byte (`make fixpoint`). A **chunk** is a slice of the not-yet-implemented specification; it is closed by the next fixed point on that path, never by a build that merely compiles. Every chunk ships with trials.

## The three paths

- **Apple Silicon (macOS, AArch64)** leads. The code as it stands today is the content of its first fixed point (FP1).
- **Linux AArch64** is in lock-step: fixed point *n* on Linux has exactly the Silica behaviours of fixed point *n* on Apple Silicon. Apple leads inside a chunk; the chunk is not closed until Linux has caught up. Port notes: [linux_aarch64_port_checklist.md](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/ports/linux_aarch64_port_checklist.md).
- **ESP32-S3 (Xtensa LX7, OS-free)** is not in lock-step. Its FP1 has all the behaviours of Apple Silicon FP1 **plus peek and poke** — `map_device` binding a board-legal MMIO window, volatile device loads and stores, callable only from a `spawn_device` worker — after which it works through the same chunks at its own pace, skipping hosted-only items. Port notes: [esp32s3_xtensa_port.md](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/ports/esp32s3_xtensa_port.md), [porting_for_os_free_targets.md](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/porting_for_os_free_targets.md), [device actor specification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica_device_actor_specification.md).

## Raw paths: chip features in chunk 2

On every raw (OS-free) path the chip's own capabilities are **chunk 2**, right after the actor stacks and data
structures, wherever the chip has them; a feature the chip lacks is recorded in the port notes as not applicable rather
than left open. Hosted paths are different by specification: on an OS the memory spaces are a discipline without
guaranteed attribute differentiation (§12.1.1.0) and placement goes through the OS affinity interface, which Apple Silicon
already has ([cpu_topology_implementation_plan.md](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/Phase1_TODOs/cpu_topology_implementation_plan.md)).

| Feature (spec wording) | Where | What the raw path delivers |
| --- | --- | --- |
| **Actor placement on a specific core.** `spawn(initial_state, behavior, n)` with a `uint64` logical core index or `core_id(n)` (§4.6); `get_cpu_topology()` and the `cpu_topology` / `core_info` records (§22.10); scheduling and affinity (§23.1.3). | §4.6, §22.10, §23.1.3 | On a multi-core chip the runtime pins that actor to the named core and reports the real topology. ESP32-S3 has two LX7 cores, so this applies to it. |
| **Memory spaces.** `region(L, Space)` with `normal`, `normal_writeback`, `normal_writethrough`, `normal_noncacheable`, `atomic`, and `device` (§4.4); OS-free runtimes give each space its real attributes, on AArch64 through `MAIR_EL1` (§12.1.1.0, §12.1.1.1). | §4.4, §12.1.1 | Every space the chip can distinguish maps to the matching cache policy, shareability, or device attribute; `device` is the peek-and-poke window. Spaces the chip cannot distinguish are documented as collapsed. |
| **Chip-specific behaviours.** Feature detection and capability queries (§21.0), then whatever the chip has: on AArch64 SVE (§21.1), NEON (§21.2), MTE (§21.3), PAC (§21.4) and the system-register access of §21.0.2; on ESP32-S3 the Xtensa LX7 equivalents (its vector and cache-control instructions) and none of the AArch64-only items. | §21 | Each raw path carries the §21 items its chip supports, with trials, and lists the rest as not applicable. |

## Where you can help, by chunk

Chunk 1 is fixed and identical on every path; the rest are the planned order and may shift when a dependency demands it.

1. **Growable and shrinkable actor stacks, and the remaining standard data structures** — first after FP1 on every path. Stacks (spec §15.1.2.2): segmented, runtime-managed, with a probe under each frame, a guard page under each segment, growth in the fault handler and shrink on return, so a single actor's stack is bounded only by machine memory; runtime, emitter prologue, and trials for deep recursion inside actors. Plan: [actor_stack_growth_plan.md](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/Phase1_TODOs/actor_stack_growth_plan.md). Data structures: the remaining public traits and graph query backends, then using them inside the compiler. [Designs](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/Phase1_TODOs/data_structure_designs/README.md).
2. **Raw paths: chip features** — actor placement on a specific core, the memory spaces with their real attributes, and the chip's §21 behaviours, as listed above. A no-op on hosted paths; the number is kept so fixed points line up across paths.
3. **Diagnostics and open defects**: the remaining silent miscompilations and the mistakes the compiler still accepts, listed in [HIGH_PRIORITY defects](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/HIGH_PRIORITY_compiler_defects_and_diagnostic_gaps_2026-09-08.md); each fix lands with an `error_enforcement_addition` or positive trial. Also the standing bar for every later feature: human-readable, machine-friendly, spec-linked errors (spec §1.6).
4. **Immutable lists — map, filter, reduce** over region-backed, vector-sized chunks so the functional pipeline stays expressive while the emitter can target SIMD-friendly layouts. [List implementation design](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/list_implementation_design.md), [TODO M1–M3](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/Phase1_TODOs/list_map_filter_reduce_and_hardening_todo.md).
5. **Region memory safety**: static lifetime analysis, buffer bounds, isolation, an ownership-based release strategy, atomic references, lifetime polymorphism. [Region TODO](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/Phase1_TODOs/region_memory_safety_todo.md).
6. **Variants and advanced control**: variant types and patterns, behaviour switching, advanced effects (spec §4.2.5, §6).
7. **Atomic operations and synchronization guarantees** audited and completed (spec §17, §18).
8. **Extended numerics**: big integers, big floats, rationals, big rationals, and 128-bit integers as distinct explicit types with no implicit widening.
9. **Beyond the process**: Fifi inbound calls and dynamic linking (spec §26.3.1) and [brokered IPC](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/brokered_ipc_isolation_architecture.md), hosted paths only. Today Fifi calls C and C-ABI libraries; see [designing apps with foreign functions](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/tutorials_and_howtos/designing_apps_with_foreign_functions.md).
10. **Tooling and proof**: [formal verification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-formal-verification-specification.md), [cryptographic guardrails](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/crypto-proposal-introduction.md), IDE and developer experience (spec §29), and tighter AArch64 emission without weakening the trials' contract with checked-in baselines.

CI trial edge-case additions are welcome at any time: grow [trials/](https://github.com/yenrab/silica/tree/main/trials) with scenarios that stress parsing, types, effects, and codegen so the trial tree keeps catching regressions.

## Later paths

A new emitter's first fixed point has the same behaviours as the **analogous pre-existing fixed point**, not Apple Silicon's: a hosted Linux path starts from Linux AArch64 (Linux on AMD or Intel x86-64 takes the current Linux AArch64 fixed point), and a bare-metal path starts from the nearest bare-metal one (bare-metal AArch64 takes the current ESP32-S3 fixed point, peek and poke included). From there it works through the chunks. Hosted and bare metal stay parallel; smaller numbers mean sooner planned attention, not a guarantee. Among bare-metal rows in the same band, ESP32-S3 is listed first because a volunteer is driving it. If you enjoy ABIs, triples, link steps, CI on new hosts, or bringing up a small runtime on a board with no OS, pick a row and open a discussion or PR. See [porting for OS-free targets](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/porting_for_os_free_targets.md).

| Planned focus | Strand | Target | Why it helps |
| --- | --- | --- | --- |
| 1 | Hosted (chip + OS) | Linux on AArch64 | Same ISA as today’s primary machine, different syscall/link story; ARM cloud and desktop Linux for contributors and trials. |
| 1 | Bare metal (OS-free, by chip) | ESP32-S3 (Xtensa LX7) | Volunteer in flight—this is the first bare-metal bring-up planned for this chip (ROM/startup, linker, and runtime on a widely used dev-board line). |
| 1 | Bare metal (OS-free, by chip) | AArch64 | Real cores without a full OS; see [memory effects on AArch64 / OS-free targets](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/memory-effects-aarch64-implementation-plan.md); ABI/runtime on a minimal environment. |
| 2 | Hosted (chip + OS) | Linux on x86_64 | Broad server and desktop footprint; strong payoff for CI and for developers not on Apple hardware. |
| 2 | Bare metal (OS-free, by chip) | RISC-V (application-profile cores) | Broad embedded/accelerator footprint; calling convention, linker/platform story, trials or hardware-in-the-loop. |
| 3 | Hosted (chip + OS) | Windows (x86_64; AArch64 when there is demand) | Lowers the barrier for contributors and teams on Windows workstations. |
| 3 | Bare metal (OS-free, by chip) | Common MCU classes (e.g. 32-bit embedded) | Longer tail of boards/ISAs; linker scripts, platform packages, minimal-runtime contract per profile. |

## Compiler-building tools

Tools for generating compiler code and coordinating work live under [compiler/silica-compiler/compiler-building-tools/](https://github.com/yenrab/silica/tree/main/compiler/silica-compiler/compiler-building-tools/).

For JSON-LD agent graphs, which files to use, and how they fit AI-assisted workflows, see [compiler-building-tools/README.md](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/compiler-building-tools/README.md).
