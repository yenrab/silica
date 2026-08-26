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

- [Build the compiler](#building-the-compiler)
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

Two tracks move in parallel — language/compiler and runtime. See the [roadmap](ROADMAP.md). Compiler-building tools (including JSON-LD agent graphs) live under [compiler-building-tools/](compiler/silica-compiler/compiler-building-tools/).

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

These steps build the self-hosted toolchain ([roadmap](ROADMAP.md) Track 1). They are not the numbered bootstrap phases in [build-plan.md](compiler/silica-compiler/design_documents/build-plan.md).

**Platform notice (temporary):** the build and link path is validated on Apple Silicon (arm64 macOS) only. Other hosts are not yet supported end-to-end. A useful early contribution is another emit backend under [src_selfhost/emitter/](compiler/silica-compiler/src_selfhost/emitter/) (see `apple_silicon_mac/` and `aarch64_debian/`), then `make TARGET=…`.

### Current build: seed host + self-hosted `silica-compiler`

The supported compiler tree is [src_selfhost/](compiler/silica-compiler/src_selfhost/). A checked-in seed binary under [binaries/](binaries/) compiles the Silica sources to assembly; Clang assembles and links the result into a new `silica-compiler`.

**1. Prerequisites**


| Requirement   | Role                                                                                                                                                                                                                                                                                                                               |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Seed compiler | Native host binary at `binaries/silica-compiler` (symlink to a versioned file such as `silica-999998-seed-macos-applesilicon`). `make` refreshes the link if needed; you can also run `binaries/update_silica_compiler_link.bash` and pick your local host platform (where the compiler runs, not an emit / cross-compile target). |
| GNU Make      | Drives the build under `src_selfhost/`.                                                                                                                                                                                                                                                                                            |
| Clang         | Assembles `.sams` → `.o` and links `silica-compiler`. On Apple Silicon Homebrew LLVM installs, the Makefile prefers `/opt/homebrew/opt/llvm/bin/clang` when present.                                                                                                                                                               |


**2. Build**

From the repository root:

```bash
cd compiler/silica-compiler/src_selfhost
make
```

Plain `make` (same as `make build`) creates or refreshes `silica.config` when needed, runs the seed compiler, assembles, and links `silica-compiler` in that directory.

**Useful options and targets**


| Command                  | What it does                                                                                  |
| ------------------------ | --------------------------------------------------------------------------------------------- |
| `make` / `make build`    | Full build: config → seed compile → objects → link.                                           |
| `make assembly`          | Seed compile only (produce / refresh `.sams`).                                                |
| `make objects`           | Assemble `.sams` → `.o` (runs assembly first if needed).                                      |
| `make executables`       | Link `silica-compiler` (runs objects first if needed).                                        |
| `make clean`             | Remove generated artifacts (`.sams`, `.o`, configs, iface caches, and the local executable).  |
| `make all`               | `clean`, then `build`.                                                                        |
| `make help`              | List targets, the active emit target, and allowable `TARGET` values.                          |
| `make TARGET=<name>`     | Bake a specific emit backend from `emitter/<name>/` into this build (writes `silica.target`). |
| `make all-targets`       | Clean/build once per allowable emit target, producing `silica-compiler-<TARGET>` for each.    |
| `make EXECUTABLE=<name>` | Override the output binary name (default: `silica-compiler`).                                 |


`TARGET` selects which `emitter/<TARGET>/` backend is compiled into the binary. It is not a runtime switch inside an already-built compiler, and it is not the `binaries/` host platform tag (for example `macos-applesilicon`). When unset, the Makefile defaults from the current host (`apple_silicon_mac` on Darwin arm64).

```bash
make                                    # host-default TARGET + full build
make TARGET=apple_silicon_mac           # explicit emit backend
make assembly TARGET=apple_silicon_mac  # seed compile only for that backend
make all-targets                        # one binary per allowable TARGET
```

Run `make help` in `src_selfhost/` for the live list of allowable emit targets and the currently active `TARGET`.

### Runtime (Track 2): no single documented build yet

[Track 2](ROADMAP.md) is foreign interoperability and, later, brokered IPC. Exact build and link steps for that path are still to be defined. Until then, the self-hosted compiler build above is the supported path.

## Building the Continuous Integration trials

CI trials live under [compiler/silica-compiler/trials/](compiler/silica-compiler/trials/). Each subdirectory (for example `atoms_addition`, `structs_addition`) holds Silica sources and golden files (`.ascomp` assembly, `.scout` expected output). The [trials Makefile](compiler/silica-compiler/trials/Makefile) compiles every listed `.silica` with the self-hosted `silica-compiler`, compares assembly to the checked-in baseline, assembles and links, runs the binaries, and compares stdout/exit code to `.scout`.

Prerequisite: build `compiler/silica-compiler/src_selfhost/silica-compiler` as above, or use the seed at `binaries/silica-compiler` (what the trials Makefile expects by default).

`make integrate` is the full pipeline and the Makefile default, so plain `make` does the same thing.

```bash
cd compiler/silica-compiler/trials
make integrate
```

Other useful targets:

- `make clean` — remove generated executables, `.sams`, `.o`, `.sout`, each trial’s `silica.config`, and each trial’s `.integrate_counts` (keeps golden `.ascomp` and `.scout`).
- `make help` — list targets and trial subdirectories.
- `make -C compiler/silica-compiler/trials/<trial_dir> silica.config` — rebuild that trial’s `silica.config` from all `*.silica` under the directory (no compile).

The harness assumes the same Apple Silicon / macOS toolchain as the compiler build. Some trial directories have their own READMEs.

Ad hoc language experiments live under [compiler/experiments/](compiler/experiments/) (separate Makefiles; not the golden-file CI pipeline).

## License

This project is licensed under the [Apache License 2.0](LICENSE). See [NOTICE](NOTICE) for copyright attribution.