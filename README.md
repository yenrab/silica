<p align="center">
  <img src="./silica_icon_small.png" alt="Silica" width="192" />
</p>

# Silica

**Silica's target is to be the world’s most secure language and runtime—without asking you to fight the tools or the language to get there.** With Silica, security is not a bolt-on checklist; it is woven through the language model, the compiler, and the runtime so that ordinary code reads clearly and dangerous patterns fail during compile-time, with explanations you can act on.

Silica is a **highly concurrent**, **functional**, **systems language**. This means you get explicit effects, **actor-based** message passing, and **region-based memory** with **no garbage collection**. Many other development stacks treat **memory**, **concurrency**, and **observable behavior** as separate concerns—each papered over with conventions, code reviews, and runtime luck. Silica **weaves them into one model**: regions and lifetimes give memory a coherent story **without** a collector; actors default to **isolation and messages** instead of ad hoc sharing; effects on types make mutation and all possible kinds of I/O something the compiler **checks**, not something teams infer from names and docs. That integration dramatically narrows where bugs can hide: whole classes of memory and concurrency mistakes fail **at compile time** with explanations tied to the spec, and ordinary modules stay easier to audit because intent is explicit. You keep **predictable performance** and a **small conceptual surface area**. **With Silica, you stay in control without being overwhelmed.**

**Silica** is built for **today’s silicon** and **what real cores actually expose to software today**—so you are not stuck translating ideas through **obsolete machine models** or carry-over abstractions from another era. Real machines reward **locality**, **parallelism without accidental sharing**, and **contained failure**; the language and runtime are shaped so those ideas stay first-class, not bolted on after the fact or 'optimized in'. What you write maps to **performance aligned with the hardware** you can actually buy and use, not to a dangerously nostalgic picture of how chips and computers used to work.

---

<img
  src="./silica_icon_emoji.png"
  alt=""
  style="
    height: 1.5em;
    width: auto;
    object-fit: contain;
    display: inline-block;
    vertical-align: -0.5em;
    margin-right: 0.2em;
  "
/><strong>Motto: Secure by default at compile time — fail soft, never fail silent</strong>

---

## Why Silica is worth your attention

### Security and correctness by design

- **Memory and effects are first-class.** Side effects are tracked in types; memory is organized through regions and references with static lifetime reasoning—so many whole classes of bugs never become runnable code. See the [language specification](compiler/silica-compiler/design_documents/silica-specification.md) (memory model, effects, actors). Related design docs: [actor capabilities and message ordering](compiler/silica-compiler/design_documents/silica_actor_capabilities_specification.md) (draft extension) and [memory effects on AArch64 / OS-free targets](compiler/silica-compiler/design_documents/memory-effects-aarch64-implementation-plan.md) (implementation plan).
- **Memory is allocated in actor stacks, not heaps.** **Memory allocation happens in each actor’s stack**—storage is **stack-shaped and flexibly sized**, with **no per-actor or shared heap** for long-lived data. Lifetimes follow **calls and frames**, which keeps memory easy to reason about and **wipes out** typical **heap-style** mistakes (use-after-free, double-free, leaks, etc.) **without** a garbage collector. **Sharing stays message-shaped** and execution stays **predictable**. See [§15.1.2.2 — *Actor stack architecture*](compiler/silica-compiler/design_documents/silica-specification.md#spec-actor-stack-architecture) (stack allocation, growable stacks, handler-local memory); [§12.1.5 — *Region handles and actor spawn*](compiler/silica-compiler/design_documents/silica-specification.md#spec-region-handles-actor-spawn) (regions **move** in at `spawn`); and [§12.1.6 — *Region handles in actor messages*](compiler/silica-compiler/design_documents/silica-specification.md#spec-region-handles-actor-messages) (regions and related payloads **move** in `call` and `cast`, including **reply** ownership on `call`).
- **The compiler rejects “almost right” code.** Patterns that optimizers usually patch up—dead bindings, duplicate work, redundant arithmetic, loop-invariant mistakes—are **compile-time errors** so behavior stays intentional and predictable. See [additional compiler rules](compiler/silica-compiler/design_documents/silica-specification-additional.md).
- **Cryptography gets language-level guardrails** (proposed): secret vs. public labels, constant-time comparisons, no secret-driven control flow, and protected buffers—shifting many crypto mistakes from “hope someone catches it” to “the compiler says no.” See [crypto proposal](compiler/silica-compiler/design_documents/crypto-proposal-introduction.md).
- **Formal methods meet engineering.** The type system is aligned with a proof-oriented view of programs (Curry–Howard), with a path to richer verification as the toolchain matures. See [formal verification specification](compiler/silica-compiler/design_documents/silica-formal-verification-specification.md).

### A runtime built for isolation and recovery

- **Unsafe worlds stay outside your safe core** (proposed). When you must touch C or other unsafe libraries, a **brokered IPC** design keeps the safe application free of in-process FFI to untrusted code: separate channels, validated messages, no shared memory with the worker, centralized policy—so isolation and recovery are architectural, not aspirational. See [brokered IPC architecture](compiler/silica-compiler/design_documents/brokered_ipc_isolation_architecture.md).
- **BEAM-inspired fault containment, native speed.** The runtime direction is **lightweight actors** running concurrenlty with independent stacks and no heap, message passing, and “let it crash” semantics at the process level—paired with hardware-assisted safety (e.g. **MTE** on AArch64) so faults become controlled events, not silent corruption. See [crash containment design](compiler/silica-compiler/design_documents/beam_like_crash_containment_design_notes.md).

### Still easy to read, write, and tool

- **Explicit types and syntax** reduce ambiguity for humans and for tools—including structured, spec-linked diagnostics. The language is intentionally **readable and LLM-friendly** without sacrificing rigor: clear bindings, pattern matching, and module boundaries. See §1.3 of the [language specification](compiler/silica-compiler/design_documents/silica-specification.md).
- **No generics maze:** polymorphism through **traits** and concrete types keeps programs straightforward to navigate compared with heavy type-level programming.

---

## Why participate in Silica’s development

This is a rare moment: a language whose **security story and runtime architecture are being shaped in the open**, with deep design docs and a **bootstrap path toward a self-hosted compiler on many chips and cross compilers for many others**. Contributing here means influencing:

- how **memory safety**, **concurrency**, and **effects** meet real systems code;
- how **isolation** and **crypto** defaults look in practice;
- and how **compiler errors** and **specifications** stay aligned so security is teachable, not tribal.

If you care about **secure-by-construction systems**, **native performance**, and **clarity of intent**, Silica is built to reward that investment. [Where the project is headed](#where-the-project-is-headed-roadmap-tracks) organizes in-flight work into parallel **language** and **runtime** tracks; the [code organization](compiler/silica-compiler/design_documents/silica-compiler-code-organization.md) document helps you navigate the tree.

### Where the project is headed (roadmap tracks)

The **bootstrap compiler** is complete; ongoing work below is grouped by whether it primarily advances the **language and toolchain** or the **runtime platform**. Items can move forward on either track as design and implementation allow.

**Track 1 — language and compiler**

**Phase 2 (in progress)**

**Here’s where you could help**

**Immutable lists and lowering**

- **Fast Map, Filter, and Reduce:** make **immutable list** traversals use **vector-sized chunks** in **region-backed** storage—so the usual functional pipeline stays expressive in source while the emitter can target **SIMD-friendly** layouts and avoid redundant allocations. See [list implementation design](compiler/silica-compiler/design_documents/list_implementation_design.md) (kernel ops and lowering).

**Compiler rules and diagnostics**

- **Compiler errors for anti-patterns:** the self-hosted compiler reports **hard errors** for **dead bindings**, **duplicate work**, **redundant arithmetic**, **loop-invariant mistakes**, and other patterns spelled out in [additional compiler rules](compiler/silica-compiler/design_documents/silica-specification-additional.md)—so inefficient or ambiguous code is fixed at the source, not silently “optimized away.”
- **Fine-tuning compiler errors:** refine **diagnostics** for the current pipeline—clearer messages, stable **error codes**, accurate locations, and **spec-linked** references (see §1.6 of the [language specification](compiler/silica-compiler/design_documents/silica-specification.md))—so fixing mistakes stays fast while the self-hosted compiler matures. Carry **diagnostic quality** forward as further language features land—new rules for **crypto**, **numerics**, **IPC**, and **verification**-oriented feedback—with the same bar: **human-readable** and **machine-friendly** errors that stay aligned with the specification.

**CI and golden trials**

- **CI trial edge-case additions:** grow `[compiler/silica-compiler/trials/](compiler/silica-compiler/trials/)` with scenarios that stress the self-hosted pipeline—corner cases for parsing, types, effects, and codegen—so `make integrate` stays the gate for regressions on golden assembly (`.ascomp`) and output (`.scout`).

**Chip, OS, and bare-metal support**

The self-hosted toolchain is **validated end-to-end on macOS with Apple Silicon (AArch64)**. The table is **not a timeline** and **not a rigid pecking order**: **Planned focus** is **where work is intended to go next**—counting **within each strand** only (hosted vs bare metal stay **parallel**). Smaller numbers mean **sooner planned attention**, not a guarantee every row advances in lockstep or that cross-strand rows compete. Rows are **sorted by that band**; when two rows share the same band, **Hosted** is listed before **bare metal** for readability, not as ranking one strand above the other. Among bare-metal rows in the same band, **ESP32-S3** is listed first—there is **already a volunteer** driving that bring-up. If you enjoy **ABIs, triples, link steps, CI on new hosts, or bringing up a small runtime on a board with no OS**, pick a row and open a discussion or PR—see the [build plan](compiler/silica-compiler/design_documents/build-plan.md), `TARGET=…` and emitter layout under `[compiler/silica-compiler/src/emitter/](compiler/silica-compiler/src/emitter/)`, and [hosted vs bare-metal execution](compiler/silica-compiler/design_documents/execution-environments-hosted-vs-bare-metal.md).

| Planned focus | Strand | Target | Why it helps |
| ------------: | ------ | ------ | ------------ |
| 1 | Hosted (chip + OS) | Linux on AArch64 | Same ISA as today’s primary machine, different syscall/link story; ARM cloud and desktop Linux for contributors and trials. |
| 1 | Bare metal (OS-free, by chip) | ESP32-S3 (Xtensa LX7) | **Volunteer in flight**—this is the **first bare-metal bring-up planned** for this chip (ROM/startup, linker, and runtime on a widely used dev-board line). |
| 1 | Bare metal (OS-free, by chip) | AArch64 | **Real cores without a full OS**; see [memory effects on AArch64 / OS-free targets](compiler/silica-compiler/design_documents/memory-effects-aarch64-implementation-plan.md); ABI/runtime on a minimal environment. |
| 2 | Hosted (chip + OS) | Linux on x86_64 | Broad server and desktop footprint; strong payoff for CI and for developers not on Apple hardware. |
| 2 | Bare metal (OS-free, by chip) | RISC-V (application-profile cores) | Broad embedded/accelerator footprint; calling convention, linker/platform story, trials or hardware-in-the-loop. |
| 3 | Hosted (chip + OS) | Windows (x86_64; AArch64 when there is demand) | Lowers the barrier for contributors and teams on Windows workstations. |
| 3 | Bare metal (OS-free, by chip) | Common MCU classes (e.g. 32-bit embedded) | Longer tail of boards/ISAs; linker scripts, platform packages, minimal-runtime contract per profile. |

**Phase 3 (next up)**

- **Assembly optimization:** tighten and tune **AArch64** emission (instruction choice, scheduling, and related emitter paths) for better performance and smaller binaries without weakening the trials’ contract with checked-in baselines.
- **Compiler architecture:** **re-engineer `silica-compiler`** so it is structured around **Silica actors**—dogfooding the concurrency model in the toolchain itself while simplifying the compiler's source code.
- **Extended numerics:** first-class **big integers**, **big floats**, **rationals**, and **big rationals** as distinct explicit types (no implicit widening or automatic promotion between numeric kinds).
- **Formal methods:** deepen **Curry–Howard**–aligned reasoning and proof tooling on top of the type system. See [formal verification specification](compiler/silica-compiler/design_documents/silica-formal-verification-specification.md).
- **Cryptography:** realize the **language-level cryptographic guardrails** (secret/public labels, constant-time discipline, protected buffers, and related rules). See [crypto proposal](compiler/silica-compiler/design_documents/crypto-proposal-introduction.md).

**Track 2 — runtime**

**Phase 2 (in progress)**

- **Foreign interoperability:** call into **existing C libraries** and into **any library that exposes a C-compatible ABI** (a stable C calling convention and linkable symbols) via **FFI bindings**, instead of rewriting the ecosystem in pure Silica. That lets Silica programs use mature code in unsafe languages to get us  started.

**Phase 3 (next up)**

- **Runtime safety:** implement the **brokered IPC** architecture so unsafe language work can be isolated and mediated as designed. See [brokered IPC architecture](compiler/silica-compiler/design_documents/brokered_ipc_isolation_architecture.md).

### Compiler-building tools

Tools for generating compiler code and coordinating Phase 2 work live under [`compiler/silica-compiler/compiler-building-tools/`](compiler/silica-compiler/compiler-building-tools/). For JSON-LD agent graphs, which files to use, and how they fit AI-assisted workflows, see **[compiler/silica-compiler/compiler-building-tools/README.md](compiler/silica-compiler/compiler-building-tools/README.md)**.

---

## Documentation and tutorials

### Hosted vs OS-free execution

On a mainstream OS, kernel policy bounds what you can rely on for **memory spaces** and **core pinning** compared with bare-metal or OS-free targets; see [execution environments: hosted vs bare metal](compiler/silica-compiler/design_documents/execution-environments-hosted-vs-bare-metal.md) for a short overview and links to the specification.

### Design documents

Indexed specifications, plans, and design notes in [`compiler/silica-compiler/design_documents/`](compiler/silica-compiler/design_documents/); start with the [language specification](compiler/silica-compiler/design_documents/silica-specification.md) and [additional compiler rules](compiler/silica-compiler/design_documents/silica-specification-additional.md). Also see [actor capabilities and message ordering](compiler/silica-compiler/design_documents/silica_actor_capabilities_specification.md) and [memory effects (AArch64 / OS-free)](compiler/silica-compiler/design_documents/memory-effects-aarch64-implementation-plan.md). These are **working documents** and change with the implementation.

### Tutorials and how-tos

Hands-on guides in [`compiler/silica-compiler/tutorials_and_howtos/`](compiler/silica-compiler/tutorials_and_howtos/) (actors, regions, lists, blocks, and related topics).

---

## Building the compiler

Instructions below follow the **roadmap tracks** described earlier (**Track 1** = language and compiler work in flight and the self-hosted toolchain; **Track 2** = runtime platform, including **foreign interoperability** and **brokered IPC**). They do **not** refer to the numbered **bootstrap pipeline phases** inside [build-plan.md](compiler/silica-compiler/design_documents/build-plan.md).

**Platform notice (temporary):** The build and link path is **validated on Apple Silicon (arm64 macOS)** only. Other chips are not supported end-to-end yet. **Early contribution opportunity:** help bring additional targets online by adding or completing an **emitter backend** under `[compiler/silica-compiler/src/emitter/](compiler/silica-compiler/src/emitter/)` (see existing `apple_silicon/`), wiring `TARGET=…` in the `[Makefile](compiler/silica-compiler/src/Makefile)`, and extending toolchain/triple notes in the [build plan](compiler/silica-compiler/design_documents/build-plan.md) as needed. That work is a concrete way to support new CPUs and boards before **Track 2** runtime pieces land.

### Current build: bootstrap compiler + self-hosted `silica-compiler` (Track 1 toolchain)

**1. Install tooling**


| Requirement                 | Role                                                                                                                                                                                                                                                                                |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Rust** (`rustc`, `cargo`) | **1.70+** — builds the bootstrap compiler (`[compiler/silica-bootstrap-compiler](compiler/silica-bootstrap-compiler)`).                                                                                                                                                             |
| **GNU Make**                | Drives the Silica-in-Silica compiler build under `[compiler/silica-compiler/src](compiler/silica-compiler/src)`.                                                                                                                                                                    |
| **LLVM tools**              | `llvm-as` and `llc` assemble and lower the generated LLVM IR; a **C linker** (typically **Clang**) links `main.o` with the bootstrap static runtime. The self-hosted compiler `Makefile` looks for tools on `PATH` and, on Apple Silicon Homebrew installs, under `/opt/homebrew/opt/llvm/bin/`. |
| **Optional: LLVM 15**       | Enables the bootstrap compiler’s **LLVM bitcode backend** (`llvm_backend` feature). Set `LLVM_SYS_150_PREFIX` to your LLVM 15 prefix if you build with that feature. See `[compiler/silica-bootstrap-compiler/README.md](compiler/silica-bootstrap-compiler/README.md)`.            |


The self-hosted compiler build uses the bootstrap compiler **without** the LLVM backend feature (`cargo build --release --no-default-features`) so the pipeline stays consistent with text IR generation and the LLVM tools above.

**2. Build the bootstrap compiler (Rust)**

From the repository root:

```bash
cd compiler/silica-bootstrap-compiler
./build_bootstrap.sh
```

`build_bootstrap.sh` detects LLVM 15 when present and prints status; you can also build manually:

```bash
# Default: text LLVM IR (no LLVM 15 required for this mode)
cargo build --release --no-default-features

# Optional: LLVM bitcode backend (requires LLVM 15; see README in this directory)
export LLVM_SYS_150_PREFIX=/path/to/llvm-15
cargo build --release --features llvm_backend
```

Artifacts used by the next step include `target/release/silica-boot` and `target/release/libsilica_compiler.a`.

**3. Build the self-hosted compiler (Silica sources + link)**

By default, `make` builds for **Apple Silicon** (`TARGET=apple_silicon`, the `emitter/apple_silicon` backend). That is currently the **only** emitter tree shipped for a full build; see the platform notice above for supporting more chips.

```bash
cd compiler/silica-compiler/src
make clean   # optional
make
```

When additional backends exist under `emitter/<target>/`, you will select them with `make TARGET=<target>` (the Makefile lists discovered directories in `make help`).

This produces the `silica-compiler` executable in `compiler/silica-compiler/src/`. For Makefile details and targets, run `make help` in that directory.

### Track 2 runtime (projected): not yet documented as a single build

**Track 2** covers **Phase 2** foreign interoperability and **Phase 3** **brokered IPC** and related platform services. Ongoing **Track 1** work (for example an **actor-structured `silica-compiler`**, extended numerics, and other language features) continues to use the self-hosted build path above until its own requirements change. The **exact Track 2 build and link steps are still to be defined** (multiple OS processes, IPC libraries, and runtime services will join the current “compiler + static runtime” flow). When those pieces land, this section will be replaced with concrete prerequisites (e.g. broker daemon, isolation test harness) and end-to-end commands. Until then, treat the **self-hosted compiler build** above as the supported path.

## Building the Continuous Integration trials

The **CI trials** live under `[compiler/silica-compiler/trials/](compiler/silica-compiler/trials/)`. Each subdirectory (e.g. `atoms_addition`, `structs_addition`) holds Silica sources and golden files (`.ascomp` assembly, `.scout` expected output). The top-level `[Makefile](compiler/silica-compiler/trials/Makefile)` drives the same checks automation would: compile every listed `.silica` with the **self-hosted** `silica-compiler`, compare generated assembly to the checked-in baseline, assemble and link, run the binaries, and compare stdout/exit code to `.scout`.

**Prerequisite:** build `compiler/silica-compiler/src/silica-compiler` as in the **self-hosted compiler** steps above. The trials invoke `../src/silica-compiler` by path.

Run the full CI pipeline with `**make integrate`** (compile every trial, diff assembly against `.ascomp`, link, run, diff output against `.scout`). That target is also the Makefile’s default, so plain `make` runs the same steps.

```bash
cd compiler/silica-compiler/trials
make integrate
```

Other useful targets:

- `make clean` — remove generated executables, `.sams`, `.o`, `.sout`, each trial’s `silica.config`, and each trial’s `.integrate_counts` (keeps golden `.ascomp` and `.scout`).
- `make help` — list targets and trial subdirectories.
- `make -C compiler/silica-compiler/trials/<trial_dir> silica.config` — rebuild that trial’s `silica.config` from all `*.silica` under the directory (no compile).

The trial harness assumes the same **Apple Silicon / macOS** toolchain as the self-hosted compiler build (assembly and linking use the host SDK and arm64). For details per trial, see READMEs under specific trial directories where present.

Ad hoc language experiments also exist under `[compiler/experiments/](compiler/experiments/)` (separate Makefiles; not the same integrated golden-file pipeline as the CI trials).

## License

This project is licensed under the [Apache License 2.0](LICENSE). See [NOTICE](NOTICE) for copyright attribution.

---

*Silica: systems programming where security and clarity are part of the language—not an afterthought.*