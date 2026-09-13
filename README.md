![Silica](./silica_icon_small.png)

# Silica

Silica is a memory-safe, functional systems language. The compiler rejects unsafe or accidental behavior at compile time, with errors you can act on. Effects are explicit, actors pass messages, and memory is region-based with no garbage collector.

Silica is designed for bare metal systems development, but also supports applications hosted by operating systems. The initial toolchain works, today, on Apple Silicon macOS.

[Why Silica](https://yenrab.github.io/silica/) — language goals and how to get involved.

```silica
module main;

fn add(a: int64, b: int64) -> int64 {
    a + b
}

fn main() -> atom {
    case add(20, 1) * 2 of {
        42: int64 -> :ok;
        _: int64 -> :error
    }
}
```

- [Build the compiler](#building-the-compiler) (seed, gen1, gen2) and [run the trials](#running-the-continuous-integration-trials)
- [Language specification](compiler/silica-compiler/design_documents/silica-specification.md)
- [Tutorials](compiler/silica-compiler/tutorials_and_howtos/)
- [Roadmap](ROADMAP.md)
- [Contributing](CONTRIBUTING.md)

---

![](./silica_icon_emoji.png)**Motto: Secure by default at compile time — fail soft, never fail silent**

---



## Language and runtime

- Effects live in the type system. Memory is organized with regions and per-actor stacks: no shared heap, no GC. See the [language specification](compiler/silica-compiler/design_documents/silica-specification.md), [actor stack architecture](compiler/silica-compiler/design_documents/silica-specification.md#spec-actor-stack-architecture), and [region handles](compiler/silica-compiler/design_documents/silica-specification.md#spec-region-handles-actor-spawn).
- Dead bindings, duplicate work, redundant arithmetic, and similar “the optimizer will fix it” patterns are compile-time errors. See [additional compiler rules](compiler/silica-compiler/design_documents/silica-specification-additional.md).
- FFI goes through Fifi, the compiler’s outbound foreign-function layer. Think of a cute poodle that bites: non-Silica code looks approachable and lives outside Silica’s guarantees. Wrappers and anything that depends on them must be named `dangerous_*` all the way to the app root. See the [FFI wrapper specification](compiler/silica-compiler/design_documents/silica_ffi_wrapper_specification.md), the [dangerous FFI security model](compiler/silica-compiler/design_documents/dangerous_ffi_security_model.md), and [designing apps with foreign functions](compiler/silica-compiler/tutorials_and_howtos/designing_apps_with_foreign_functions.md).
- The runtime is lightweight actors, message passing, and let-it-crash isolation (BEAM-like, native), with hardware help such as MTE on AArch64. See [crash containment](compiler/silica-compiler/design_documents/beam_like_crash_containment_design_notes.md). Brokered IPC for untrusted C is proposed as an alternative to in-process FFI; that path would not need `dangerous_*` names. See [brokered IPC](compiler/silica-compiler/design_documents/brokered_ipc_isolation_architecture.md).
- Types and syntax stay explicit. There are no generics; polymorphism is traits plus concrete types. Language-level crypto labels and richer proof tooling are proposed: [crypto proposal](compiler/silica-compiler/design_documents/crypto-proposal-introduction.md), [formal verification](compiler/silica-compiler/design_documents/silica-formal-verification-specification.md).

Related notes: [actor capabilities](compiler/silica-compiler/design_documents/silica_actor_capabilities_specification.md) (draft), [memory effects on AArch64 / OS-free targets](compiler/silica-compiler/design_documents/memory-effects-aarch64-implementation-plan.md).

## Contributing and roadmap

Working on a self-hosted toolchain plus the runtime around it, in the open, with the spec and the errors meant to stay aligned. How to open issues and PRs is in [CONTRIBUTING.md](CONTRIBUTING.md). [Code organization](compiler/silica-compiler/design_documents/silica-compiler-code-organization.md) is a map of the tree.

Development is organised by emitter path — Apple Silicon leads, Linux AArch64 is in lock-step, ESP32-S3 follows at its own pace — and delivered in chunks between fixed points. See the [roadmap](ROADMAP.md). Compiler-building tools (including JSON-LD agent graphs) live under [compiler-building-tools/](compiler/silica-compiler/compiler-building-tools/).

## Documentation

Design docs are working documents and change with the implementation. Start here:

- [Language specification](compiler/silica-compiler/design_documents/silica-specification.md) and [additional compiler rules](compiler/silica-compiler/design_documents/silica-specification-additional.md)
- Fifi: [§26.3](compiler/silica-compiler/design_documents/silica-specification.md#spec-fifi), [FFI wrapper specification](compiler/silica-compiler/design_documents/silica_ffi_wrapper_specification.md), [dangerous FFI security model](compiler/silica-compiler/design_documents/dangerous_ffi_security_model.md), [macOS guarded FFI crash handling](compiler/silica-compiler/design_documents/macos_crash_handling_for_silica.md)
- [Actor capabilities](compiler/silica-compiler/design_documents/silica_actor_capabilities_specification.md), [memory effects (AArch64 / OS-free)](compiler/silica-compiler/design_documents/memory-effects-aarch64-implementation-plan.md)
- Full index: [design_documents/](compiler/silica-compiler/design_documents/)
- Hosted vs OS-free: on a mainstream OS, kernel policy limits what you can assume about memory spaces and core pinning. Short overview: [execution environments](compiler/silica-compiler/design_documents/execution-environments-hosted-vs-bare-metal.md).

Tutorials (actors, regions, lists, blocks, and related topics) are in [tutorials_and_howtos/](compiler/silica-compiler/tutorials_and_howtos/). Useful starting points:

- Foreign functions: [designing apps with foreign functions](compiler/silica-compiler/tutorials_and_howtos/designing_apps_with_foreign_functions.md), then [FFI wrappers and Makefiles](compiler/silica-compiler/tutorials_and_howtos/ffi_wrappers_and_makefiles.md)
- App builds: [building apps with project Makefiles](compiler/silica-compiler/tutorials_and_howtos/building_apps_with_project_makefiles.md) (drop-in files in `[project_makefiles/](project_makefiles/)`)
- Large apps: [compiling with less RAM](compiler/silica-compiler/tutorials_and_howtos/compiling_with_less_ram.md)



## Building the compiler

These steps build the self-hosted toolchain ([roadmap](ROADMAP.md) Track 1). The same instructions, with more context, are on the project site: [Build and test the compiler](https://yenrab.github.io/silica/build-and-test/).

**Platform notice (temporary):** the build and link path is validated on Apple Silicon (arm64 macOS) only. Other hosts are not yet supported end-to-end. A useful early contribution is another emit backend under [src_selfhost/emitter/](compiler/silica-compiler/src_selfhost/emitter/) (see `apple_silicon_mac/`, `linux_aarch64/`, and `ESP32-S32_raw/`), then `make TARGET=…`.

### The three compilers

| Name | Source | Built by | Published as |
| ---- | ------ | -------- | ------------ |
| Bootstrap | [compiler/silica-bootstrap-compiler/](compiler/silica-bootstrap-compiler/) (Rust) | `cargo` | `target/release/silica-boot` (not in `binaries/`) |
| Seed | [compiler/silica-compiler/src/](compiler/silica-compiler/src/) (Silica) | the bootstrap | `binaries/silica-NNNNNN-seed-<platform>`, reached through `binaries/seed-compiler` |
| Selfhost | [compiler/silica-compiler/src_selfhost/](compiler/silica-compiler/src_selfhost/) (Silica, Rust-free) | the seed (**gen1**) or a previous selfhost (**gen2**) | `binaries/silica-NNNNNN-<platform>`, reached through `binaries/silica-compiler` |

Generation numbers in `binaries/` count **down**: the lowest `NNNNNN` is the newest build. The two symlinks always point at the newest seed and the newest selfhost. `gen1` is the selfhost tree compiled by the seed; `gen2` is the same tree compiled by gen1, and it is the build that must pass every trial before the selfhost can replace the seed.

### 1. Prerequisites

| Requirement | Role |
| ----------- | ---- |
| Seed compiler | `binaries/seed-compiler` (symlink to a versioned `silica-NNNNNN-seed-<platform>`). Present in the checkout; rebuilt by step 2 when the seed sources change. `binaries/update_silica_compiler_link.bash` repairs the links for your host platform (where the compiler runs, not an emit target). |
| GNU Make | Drives every build and the trial tree. |
| Clang | Assembles `.sams` → `.o` and links. On Apple Silicon with Homebrew LLVM, the Makefiles prefer `/opt/homebrew/opt/llvm/bin/clang` when present. |
| Rust toolchain (`cargo`) and LLVM (`llvm-as`, `llc`) | Only for step 2, rebuilding the seed: the bootstrap compiler is a Cargo project and the seed build goes through LLVM IR. |

### 2. Rebuild the seed (only after editing `src/`)

```bash
cd compiler/silica-compiler/src
make
```

This builds the Rust bootstrap if needed (`cargo build --release --no-default-features`), compiles the seed sources with it, links `silica-compiler` in `src/`, and installs it as the next `binaries/silica-NNNNNN-seed-<platform>`, moving `seed-compiler` to it. Without that install a seed fix stays invisible to the selfhost tree. `make INSTALL_SEED=0` builds without publishing.

### 3. Build the selfhost from the seed (gen1)

```bash
cd compiler/silica-compiler/src_selfhost
make gen1
```

`make gen1` compiles `src_selfhost` with `binaries/seed-compiler`, links `silica-compiler` in that directory, installs it as the next `binaries/silica-NNNNNN-<platform>` (moving `binaries/silica-compiler` to it), and keeps a stable copy as `binaries/silica-gen1`. Plain `make` (or `make build`) does the same build and install but does not keep the `silica-gen1` copy.

**Emit target.** With more than one backend under `emitter/`, an interactive `make` first asks which one to bake in:

```
Select the emit target (emitter/<name>/ to bake into silica-compiler):
  1) ESP32-S32_raw
  2) apple_silicon_mac  [default: host]
  3) linux_aarch64
Number or name [apple_silicon_mac]:
```

Answer with a number or a name; an empty answer takes the host default. The choice is written to `silica.target` and passed to the sub-makes, so you are asked once per build. `make TARGET=<name>` skips the question, `SILICA_TARGET_PROMPT=0` always takes the host default (for scripts), and builds without a terminal (CI, `nohup`, pipes) take the host default silently. `make help` and `make clean` never ask. `TARGET` is a code-generation backend baked into the binary, not a runtime switch and not the `binaries/` host platform tag.

### 4. Build the selfhost from the selfhost (gen2)

```bash
cd compiler/silica-compiler/src_selfhost
make gen2
```

`make gen2` compiles `src_selfhost` with `binaries/silica-gen1` (it stops with a message if gen1 has not been built), installs the result as the next numbered selfhost, moves `binaries/silica-compiler` to it, and keeps the copy `binaries/silica-gen2`. Every unit is recompiled: the compiler binary is a staleness input, so there is no incremental path between generations. `INSTALL_SELFHOST=0` on either generation builds and keeps the `silica-genN` copy without touching the numbered install or the `silica-compiler` link.

### Other `src_selfhost` targets

| Command | What it does |
| ------- | ------------ |
| `make` / `make build` | Full build with `seed-compiler`: config → compile → objects → link → install. |
| `make gen1` / `make gen2` | The generation builds described above. |
| `make trials-gen1` / `make trials-gen2` / `make trials-both` | Run the whole trial tree against a generation (next section). |
| `make fixpoint` | Build gen3 with gen2 and pass only if it is byte-identical to gen2 (see the trials section). |
| `make assembly` | Compile only (produce / refresh `.sams`). |
| `make objects` | Assemble `.sams` → `.o` (runs assembly first if needed). |
| `make executables` | Link `silica-compiler` (runs objects first if needed). |
| `make clean` | Remove generated artifacts (`.sams`, `.o`, configs, iface caches, the local executable). |
| `make all` | `clean`, then `build`. |
| `make help` | List targets, the active emit target, and allowable `TARGET` values. |
| `make TARGET=<name>` | Bake a specific backend from `emitter/<name>/` (writes `silica.target`). |
| `make all-targets` | Clean/build once per allowable emit target, producing `silica-compiler-<TARGET>` for each. |
| `make EXECUTABLE=<name>` | Override the output binary name (default: `silica-compiler`). |
| `make build SILICA_COMPILER=<binary>` | Compile with any compiler binary (this is what `gen2` does with `silica-gen1`). |

### Runtime (Track 2): no single documented build yet

[Track 2](ROADMAP.md) is foreign interoperability and, later, brokered IPC. Exact build and link steps for that path are still to be defined. Until then, the self-hosted compiler build above is the supported path.

## Running the Continuous Integration trials

CI trials live under [trials/](trials/). Each suite directory (for example `atoms_addition`, `case_addition`, `error_enforcement_addition`, `ordered_data_structures`) holds Silica sources and golden files: `.ascomp` (expected assembly), `.scout` (expected stdout followed by the exit code), and `.golden_fail` (expected compiler diagnostics for programs that must not compile). The [trials Makefile](trials/Makefile) compiles every trial with the chosen compiler, compares the assembly to `.ascomp`, assembles and links, runs the binary, and compares its output to `.scout` (or the diagnostics to `.golden_fail`). Suites run in parallel; a counter line at the bottom of the terminal shows passes and failures as they happen.

### Run the tree against gen1 and gen2

```bash
cd compiler/silica-compiler/src_selfhost
make trials-gen1     # whole tree with binaries/silica-gen1
make trials-gen2     # whole tree with binaries/silica-gen2
make trials-both     # gen1, then gen2
```

Each run writes `trials/.integrate_report` and keeps a copy as `trials/.integrate_report.gen1` or `.gen2`, so running both does not lose the first result. The make target's exit status is the run's.

### Check for a fixed point

```bash
cd compiler/silica-compiler/src_selfhost
make gen2       # if not already done: the .sams left here must be gen1's emission
make fixpoint
```

`make fixpoint` builds gen3, `src_selfhost` compiled by `binaries/silica-gen2`, and passes only if gen3 is byte-identical to gen2. Passing the trials shows gen2 compiles programs correctly; the fixed point shows the compiler reproduces itself with nothing inherited from the seed, which is the condition for retiring the bootstrap.

It must run right after `make gen2`: the `.sams` left in `src_selfhost/` are then gen1's emission of the sources, and `make gen2` stamps them. The target refuses to run if that stamp is missing or any `.sams` was rewritten since, saves those `.sams` to `compiler/silica-compiler/.src_selfhost_fixpoint/gen1_sams/`, builds gen3 with gen2, and prints how many units gen2 emits differently from gen1 (with the first differing lines, `o<N>` node counters normalised) and both binary sizes. gen3 is linked at the same path as gen2, `src_selfhost/silica-compiler`, because Apple's linker derives the binary's UUID and signature identifier from the output path; it is not installed and is kept as `silica-compiler-gen3`. Running it again needs a fresh `make gen2`.

### Run the tree or one suite with any compiler

```bash
cd trials
make integrate                                    # default: binaries/silica-compiler (the newest selfhost build)
make integrate SILICA_COMPILER=/path/to/compiler  # any binary, e.g. ../binaries/seed-compiler
make -C case_addition integrate SILICA_COMPILER=/path/to/compiler   # one suite
```

`make integrate` is the Makefile default, so plain `make` in `trials/` does the same. A few suites are not part of the tree run and must be run directly: any suite with an `INTEGRATE_PENDING` marker is skipped (see its README).

### Reading a report

`trials/.integrate_report` starts with the totals and then lists each failing trial with its reason (`.sams differs from .ascomp`, `.sout differs from .scout`, `.cur_fail differs from .golden_fail`, `compilation failed`, `missing executable`, and so on).

- **Trust the totals.** If the `fail:` count is higher than the number of listed lines, a suite failed as a whole (for example its batch compile aborted on one trial) and has no per-trial lines; open that suite's `.integrate_log` for the reason.
- **`missing executable (assemble/link failed earlier)`** on hundreds of trials in one suite usually means a single `.sams` in that suite failed to assemble; the `assemble failed` line names it.
- **Assembly drift** (`.sams differs from .ascomp`) with matching runtime output is expected after an emitter change; the report prints the first differing lines. Goldens are never regenerated automatically: copy a trial's `.sams` over its `.ascomp` only after confirming the runtime result, and use `make record-golden SUB_TRIALS=<leaf>` in `ordered_data_structures/` for those leaves.
- `trials/count_trial_files.sh` prints how many golden files (`.ascomp`, `.scout`, `.golden_fail`) the tree holds.

Other useful targets in `trials/`:

- `make clean` — remove generated executables, `.sams`, `.o`, `.sout`, and per-suite counters (keeps the goldens).
- `make help` — list targets and suites.
- `make integrate-ffi` — the success-path FFI application trials only.

The harness assumes the same Apple Silicon / macOS toolchain as the compiler build. Some suites have their own READMEs.

## License

This project is licensed under the [Apache License 2.0](LICENSE). See [NOTICE](NOTICE) for copyright attribution.