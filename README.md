<div align="center">
  <img src="silica_icon.png" alt="Silica logo: stylized SiO₂ molecule" width="220" />
</div>

# Silica

**Silica's target to be the world’s most secure language and runtime—without asking you to fight the tools to get there.** Security is not a bolt-on checklist; it is woven through the language model, the compiler, and the runtime so that ordinary code reads clearly and dangerous patterns fail early, with explanations you can act on.

Silica is a **functional systems language**: explicit effects, **actor-based** message passing, and **region-based memory** with **no garbage collector**. You keep predictable performance and a small conceptual surface area, while the type system and compiler shoulder much of the burden that other ecosystems leave to discipline, reviews, and production incidents.

**Bad security practices should always have been compiler errors!**

---

## Why Silica is worth your attention

### Security and correctness by design

- **Memory and effects are first-class.** Side effects are tracked in types; memory is organized through regions and references with static lifetime reasoning—so many whole classes of bugs never become runnable code. See the [language specification](compiler/silica-compiler/design_documents/silica-specification.md) (memory model, effects, actors).
- **The compiler rejects “almost right” code.** Patterns that optimizers usually patch up—dead bindings, duplicate work, redundant arithmetic, loop-invariant mistakes—are **compile-time errors** so behavior stays intentional and predictable. See [additional compiler rules](compiler/silica-compiler/design_documents/silica-specification-additional.md).
- **Cryptography gets language-level guardrails** (proposed): secret vs. public labels, constant-time comparisons, no secret-driven control flow, and protected buffers—shifting many crypto mistakes from “hope someone catches it” to “the compiler says no.” See [crypto proposal](compiler/silica-compiler/design_documents/crypto-proposal-introduction.md).
- **Formal methods meet engineering.** The type system is aligned with a proof-oriented view of programs (Curry–Howard), with a path to richer verification as the toolchain matures. See [formal verification specification](compiler/silica-compiler/design_documents/silica-formal-verification-specification.md).

### A runtime built for isolation and recovery

- **Unsafe worlds stay outside your safe core.** When you must touch C or other unsafe libraries, a **brokered IPC** design keeps the safe application free of in-process FFI to untrusted code: separate channels, validated messages, no shared memory with the worker, centralized policy—so isolation and recovery are architectural, not aspirational. See [brokered IPC architecture](compiler/silica-compiler/design_documents/brokered_ipc_isolation_architecture.md).
- **BEAM-inspired fault containment, native speed.** The runtime direction is **lightweight processes** with independent stacks/heaps, message passing, and “let it crash” semantics at the process level—paired with hardware-assisted safety (e.g. **MTE** on AArch64) so faults become controlled events where possible, not silent corruption. See [crash containment design](compiler/silica-compiler/design_documents/beam_like_crash_containment_design_notes.md).

### Still easy to read, write, and tool

- **Explicit types and syntax** reduce ambiguity for humans and for tools—including structured, spec-linked diagnostics. The language is intentionally **readable and LLM-friendly** without sacrificing rigor: clear bindings, pattern matching, and module boundaries. See §1.3 of the [language specification](compiler/silica-compiler/design_documents/silica-specification.md).
- **No generics maze:** polymorphism through **traits** and concrete types keeps programs straightforward to navigate compared with heavy type-level programming.

---

## Why participate in Silica’s development

This is a rare moment: a language whose **security story and runtime architecture are being shaped in the open**, with deep design docs and a **bootstrap path toward a self-hosted compiler on many chips and cross compilers for many others**. Contributing here means influencing:

- how **memory safety**, **concurrency**, and **effects** meet real systems code;
- how **isolation** and **crypto** defaults look in practice;
- and how **compiler errors** and **specifications** stay aligned so security is teachable, not tribal.

If you care about **secure-by-construction systems**, **native performance**, and **clarity of intent**, Silica is built to reward that investment. The [compiler build plan](compiler/silica-compiler/design_documents/build-plan.md) outlines the toolchain roadmap; the [code organization](compiler/silica-compiler/design_documents/silica-compiler-code-organization.md) document helps you navigate the tree.

### Where the project is headed (roadmap phases)

The numbering here is the **language and platform roadmap** the bootstrap compiler was Phase 1.

**Phase 2 — current development focus**

- **Foreign interoperability:** call into **existing C libraries** and into **any library that exposes a C-compatible ABI** (a stable C calling convention and linkable symbols) via **FFI bindings**, instead of rewriting the ecosystem in pure Silica. That lets Silica programs use mature native code where appropriate while the longer-term isolation story (below) stays on the table for untrusted or high-risk components.

**Phase 3 — under conceptualization**

- **Runtime safety:** implement the **brokered IPC** architecture so unsafe language work can be isolated and mediated as designed. See [brokered IPC architecture](compiler/silica-compiler/design_documents/brokered_ipc_isolation_architecture.md).
- **Compiler architecture:** **re-engineer `silica-compiler`** so it is structured around **Silica actors**—dogfooding the concurrency model in the toolchain itself while simplifying the compiler's source code.
- **Numeric tower:** **automatic big integers** (`Auto BigInt`), **automatic big floats** (`Auto BigFloat`), **rationals** plus **automatic big rationals** (`Auto BigRational`) as first-class directions for precise and overflow-safe numerics.
- **Formal methods:** deepen **Curry–Howard**–aligned reasoning and proof tooling on top of the type system. See [formal verification specification](compiler/silica-compiler/design_documents/silica-formal-verification-specification.md).
- **Cryptography:** realize the **language-level cryptographic guardrails** (secret/public labels, constant-time discipline, protected buffers, and related rules). See [crypto proposal](compiler/silica-compiler/design_documents/crypto-proposal-introduction.md).

### Compiler-building tools

The directory [`compiler/silica-compiler/compiler-building-tools/`](compiler/silica-compiler/compiler-building-tools/) holds **JSON-LD agent specifications** (GAB / AALang–style graphs). In compatible AI-assisted workflows—typically by opening a given `.jsonld` file as the task context—the assistant follows that graph as a **specialized “tool agent”** for compiler work: structured prompts, modes, and guardrails rather than ad hoc chat.

You do not need every file for day-to-day hacking; pick the graph that matches what you are doing. At a high level:

| Area | Examples (file names) |
|------|------------------------|
| **Compiler pipeline scaffolding** | Code generators / builders for the major phases—`silica-lexer-code-generator`, `silica-parser-code-generator`, `silica-typechecker-code-generator`, `silica-effect-code-generator`, `silica-sir_generator_builder`, `silica-codegen-code-generator`, `silica-emitter_builder`, plus `main_generator` for wiring a `main` entry. |
| **Planning and integration** | `silica-compiler-phase-planning-tool` (phase design and coordination), `silica-CI` (driving CI-style checks), `golden-fail-generator` (golden / failure test workflows around trial outputs). |
| **Documentation** | `silica_doc_generator` — guided doc generation aligned with project conventions. |
| **Focused design discussions** | `tuple_recursion_discussion`, `memory_regions_discussion`, `device-io-sequence-block-tool` — structured exploration of specific language and runtime topics. |

Individual graphs contain their own execution instructions; treat them as **executable playbooks** for the assistant, not plain documentation.

---

## Building the compiler

Instructions below follow the **roadmap phases** described earlier (Phase 2 = current FFI-and-toolchain work; Phase 3 = projected runtime and compiler architecture). They do **not** refer to the numbered **bootstrap pipeline phases** inside [build-plan.md](compiler/silica-compiler/design_documents/build-plan.md).

**Platform notice (temporary):** The build and link path is **validated on Apple Silicon (arm64 macOS)** only. Other chips are not supported end-to-end yet. **Early contribution opportunity:** help bring additional targets online by adding or completing an **emitter backend** under [`compiler/silica-compiler/src/emitter/`](compiler/silica-compiler/src/emitter/) (see existing `apple_silicon/`), wiring `TARGET=…` in the [`Makefile`](compiler/silica-compiler/src/Makefile), and extending toolchain/triple notes in the [build plan](compiler/silica-compiler/design_documents/build-plan.md) as needed. That work is a concrete way to support new CPUs and boards before the runtime roadmap in Phase 3 lands.

### Phase 2 (current): bootstrap compiler + self-hosted `silica-compiler`

**1. Install tooling**

| Requirement | Role |
|-------------|------|
| **Rust** (`rustc`, `cargo`) | **1.70+** — builds the bootstrap compiler ([`compiler/silica-bootstrap-compiler`](compiler/silica-bootstrap-compiler)). |
| **GNU Make** | Drives the Silica-in-Silica compiler build under [`compiler/silica-compiler/src`](compiler/silica-compiler/src). |
| **LLVM tools** | `llvm-as` and `llc` assemble and lower the generated LLVM IR; a **C linker** (typically **Clang**) links `main.o` with the bootstrap static runtime. The Phase 2 `Makefile` looks for tools on `PATH` and, on Apple Silicon Homebrew installs, under `/opt/homebrew/opt/llvm/bin/`. |
| **Optional: LLVM 15** | Enables the bootstrap compiler’s **LLVM bitcode backend** (`llvm_backend` feature). Set `LLVM_SYS_150_PREFIX` to your LLVM 15 prefix if you build with that feature. See [`compiler/silica-bootstrap-compiler/README.md`](compiler/silica-bootstrap-compiler/README.md). |

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

**3. Build the Phase 2 self-hosted compiler (Silica sources + link)**

By default, `make` builds for **Apple Silicon** (`TARGET=apple_silicon`, the `emitter/apple_silicon` backend). That is currently the **only** emitter tree shipped for a full build; see the platform notice above for supporting more chips.

```bash
cd compiler/silica-compiler/src
make clean   # optional
make
```

When additional backends exist under `emitter/<target>/`, you will select them with `make TARGET=<target>` (the Makefile lists discovered directories in `make help`).

This produces the `silica-compiler` executable in `compiler/silica-compiler/src/`. For Makefile details and targets, run `make help` in that directory.

### Phase 3 (projected): not yet documented as a single build

Phase 3 brings in the **brokered IPC runtime**, an **actor-structured `silica-compiler`**, extended numerics, and related platform pieces. The **exact build and link steps are still to be defined** (multiple OS processes, IPC libraries, and runtime services will join the current “compiler + static runtime” flow). When those pieces land, this section will be replaced with concrete prerequisites (e.g. broker daemon, isolation test harness) and end-to-end commands. Until then, treat **Phase 2** as the supported path above.

## Building the Continuous Integration trials

The **CI trials** live under [`compiler/silica-compiler/trials/`](compiler/silica-compiler/trials/). Each subdirectory (e.g. `atoms_addition`, `structs_addition`) holds Silica sources and golden files (`.ascomp` assembly, `.scout` expected output). The top-level [`Makefile`](compiler/silica-compiler/trials/Makefile) drives the same checks automation would: compile every listed `.silica` with the **self-hosted** `silica-compiler`, compare generated assembly to the checked-in baseline, assemble and link, run the binaries, and compare stdout/exit code to `.scout`.

**Prerequisite:** build `compiler/silica-compiler/src/silica-compiler` as in **Phase 2** above. The trials invoke `../src/silica-compiler` by path.

Run the full CI pipeline with **`make integrate`** (compile every trial, diff assembly against `.ascomp`, link, run, diff output against `.scout`). That target is also the Makefile’s default, so plain `make` runs the same steps.

```bash
cd compiler/silica-compiler/trials
make integrate
```

Other useful targets:

- `make clean` — remove generated executables, `.sams`, `.o`, `.sout` (keeps golden `.ascomp` and `.scout`).
- `make help` — list targets and trial subdirectories.
- [`rebuild-silica-configs.sh`](compiler/silica-compiler/trials/rebuild-silica-configs.sh) — refresh each subdir’s `silica.config` from `*.silica` without compiling.

The trial harness assumes the same **Apple Silicon / macOS** toolchain as the Phase 2 build (assembly and linking use the host SDK and arm64). For details per trial, see READMEs under specific trial directories where present.

Ad hoc language experiments also exist under [`compiler/experiments/`](compiler/experiments/) (separate Makefiles; not the same integrated golden-file pipeline as the CI trials).

---

*Silica: systems programming where security and clarity are part of the language—not an afterthought.*
