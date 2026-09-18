---
title: Build and test the compiler
layout: default
permalink: /build-and-test/
---

# Build and test the compiler

How to rebuild the seed, build the self-hosted compiler from the seed (gen1) and from itself (gen2), and run the continuous-integration trials against either generation.

These steps build the self-hosted toolchain ([roadmap](https://github.com/yenrab/silica/blob/main/ROADMAP.md) Track 1). This page mirrors the [README](https://github.com/yenrab/silica#building-the-compiler); the repository copy is the one kept in step with the Makefiles.

**Platform notice (temporary):** the build and link path is validated on Apple Silicon (arm64 macOS) only. Other hosts are not yet supported end-to-end. A useful early contribution is another emit backend under [src_selfhost/emitter/](https://github.com/yenrab/silica/tree/main/compiler/silica-compiler/src_selfhost/emitter/) (see `apple_silicon_mac/`, `linux_aarch64/`, and `ESP32-S3_raw/`), then `make TARGET=…`.

## The three compilers

| Name | Source | Built by | Published as |
| ---- | ------ | -------- | ------------ |
| Seed | the previous fixed-point self-host, kept as a binary; `binaries/seed-compiler` points at it | itself, one generation earlier | `binaries/silica-NNNNNN-<platform>`. The original Rust bootstrap (`silica-boot`) and the seeds it produced (`silica-NNNNNN-seed-<platform>`) stay in `binaries/` for the record; their sources are in the repository's history |
| Selfhost | [compiler/silica-compiler/src_selfhost/](https://github.com/yenrab/silica/tree/main/compiler/silica-compiler/src_selfhost/) (Silica, Rust-free) | the seed, itself an earlier selfhost (**gen1**), or the selfhost that produced (**gen2**) | `binaries/silica-NNNNNN-<platform>`, reached through `binaries/silica-compiler` |
| ESP32-S3 compiler | the same tree with `TARGET=ESP32-S3_raw`: runs on the Mac, emits ESP32-S3 (Xtensa) assembly | the seed | `binaries/silica-NNNNNN-ESP32_S3_raw-<platform>`, reached through `binaries/silica-compiler-ESP32-S3_raw` |

Generation numbers in `binaries/` count **down**: the lowest `NNNNNN` is the newest build. Each kind (seed, selfhost, and each emit target such as ESP32-S3_raw) is numbered on its own, and its symlink always points at its newest build. `gen1` is the selfhost tree compiled by the seed; `gen2` is the same tree compiled by gen1, and it is the build that must pass every trial before the selfhost can replace the seed.

## 1. Prerequisites

The complete list, with versions, install commands and the ESP32-S3 board tools, is on its own page: [Required software]({{ '/required-software/' | relative_url }}).

| Requirement | Role |
| ----------- | ---- |
| Seed compiler | `binaries/seed-compiler` (symlink to the previous fixed-point self-host, a versioned `silica-NNNNNN-<platform>`). Present in the checkout; it advances only when a new fixed point is released. `binaries/update_silica_compiler_link.bash` repairs the `silica-compiler` link for your host platform (where the compiler runs, not an emit target). |
| GNU Make | Drives every build and the trial tree. |
| Clang | Assembles `.sams` → `.o` and links. On Apple Silicon with Homebrew LLVM, the Makefiles prefer `/opt/homebrew/opt/llvm/bin/clang` when present. |

## 2. The seed

There is no seed to rebuild. The seed is the previous fixed-point self-host, so a change to the compiler is made in `src_selfhost` and becomes the next seed when it reaches fixed point and is released. The Rust bootstrap and the seed source it compiled were retired once the self-host reproduced itself; their binaries stay in `binaries/` and their sources in the repository's history.

## 3. Build gen1: the selfhost from the seed

```bash
cd compiler/silica-compiler/src_selfhost
make gen1
```

`make gen1` compiles `src_selfhost` with `binaries/seed-compiler`, links `silica-compiler` in that directory, installs it as the next `binaries/silica-NNNNNN-<platform>` (moving `binaries/silica-compiler` to it), and keeps a stable copy as `binaries/silica-gen1`. Plain `make` (or `make build`) does the same build and install but does not keep the `silica-gen1` copy.

**Emit target.** With more than one backend under `emitter/`, an interactive `make` first asks which one to bake in:

```
Select the emit target (emitter/<name>/ to bake into silica-compiler):
  1) ESP32-S3_raw
  2) apple_silicon_mac  [default: host]
  3) linux_aarch64
Number or name [apple_silicon_mac]:
```

Answer with a number or a name; an empty answer takes the host default. The choice is written to `silica.target` and passed to the sub-makes, so you are asked once per build. `make TARGET=<name>` skips the question, `SILICA_TARGET_PROMPT=0` always takes the host default (for scripts), and builds without a terminal (CI, `nohup`, pipes) take the host default silently. `make help` and `make clean` never ask. `TARGET` is a code-generation backend baked into the binary, not a runtime switch and not the `binaries/` host platform tag.

## 4. Build gen2: the selfhost from the selfhost

```bash
cd compiler/silica-compiler/src_selfhost
make gen2
```

`make gen2` compiles `src_selfhost` with `binaries/silica-gen1` (it stops with a message if gen1 has not been built), installs the result as the next numbered selfhost, moves `binaries/silica-compiler` to it, and keeps the copy `binaries/silica-gen2`. Every unit is recompiled: the compiler binary is a staleness input, so there is no incremental path between generations. `INSTALL_SELFHOST=0` on either generation builds and keeps the `silica-genN` copy without touching the numbered install or the `silica-compiler` link.

## Other `src_selfhost` targets

| Command | What it does |
| ------- | ------------ |
| `make` / `make build` | Full build with `seed-compiler`: config → compile → objects → link → install. |
| `make gen1` / `make gen2` | The generation builds described above. |
| `make trials-gen1` / `make trials-gen2` / `make trials-both` | Run the whole trial tree against a generation (see [Run the trials against gen1 and gen2](#run-the-trials-against-gen1-and-gen2)). |
| `make fixpoint` | Build gen3 with gen2 and pass only if it is byte-identical to gen2 (see [Check for a fixed point](#check-for-a-fixed-point)). |
| `make assembly` | Compile only (produce / refresh `.sams`). |
| `make objects` | Assemble `.sams` → `.o` (runs assembly first if needed). |
| `make executables` | Link `silica-compiler` (runs objects first if needed). |
| `make clean` | Remove generated artifacts (`.sams`, `.o`, configs, iface caches, the local executable). |
| `make all` | `clean`, then `build`. |
| `make help` | List targets, the active emit target, and allowable `TARGET` values. |
| `make TARGET=<name>` | Bake a specific backend from `emitter/<name>/` (writes `silica.target`). A backend other than the host's is published as `binaries/silica-compiler-<name>` (for example the ESP32-S3 compiler), never as `binaries/silica-compiler`. |
| `make all-targets` | Clean/build once per allowable emit target, producing `silica-compiler-<TARGET>` for each. |
| `make EXECUTABLE=<name>` | Override the output binary name (default: `silica-compiler`). |
| `make build SILICA_COMPILER=<binary>` | Compile with any compiler binary (this is what `gen2` does with `silica-gen1`). |

## Runtime (Track 2): no single documented build yet

[Track 2](https://github.com/yenrab/silica/blob/main/ROADMAP.md) is foreign interoperability and, later, brokered IPC. Exact build and link steps for that path are still to be defined. Until then, the self-hosted compiler build above is the supported path.

## The trial tree

CI trials live under [trials/](https://github.com/yenrab/silica/tree/main/trials/). Each suite directory (for example `atoms_addition`, `case_addition`, `error_enforcement_addition`, `ordered_data_structures`) holds Silica sources and golden files: `.ascomp` (expected assembly), `.scout` (expected stdout followed by the exit code), and `.golden_fail` (expected compiler diagnostics for programs that must not compile). The [trials Makefile](https://github.com/yenrab/silica/blob/main/trials/Makefile) compiles every trial with the chosen compiler, compares the assembly to `.ascomp`, assembles and links, runs the binary, and compares its output to `.scout` (or the diagnostics to `.golden_fail`). Suites run in parallel; a counter line at the bottom of the terminal shows passes and failures as they happen.

## Run the trials against gen1 and gen2

```bash
cd compiler/silica-compiler/src_selfhost
make trials-gen1     # whole tree with binaries/silica-gen1
make trials-gen2     # whole tree with binaries/silica-gen2
make trials-both     # gen1, then gen2
```

Each run writes `trials/.integrate_report` and keeps a copy as `trials/.integrate_report.gen1` or `.gen2`, so running both does not lose the first result. The make target's exit status is the run's.

## Check for a fixed point

```bash
cd compiler/silica-compiler/src_selfhost
make gen2       # if not already done: the .sams left here must be gen1's emission
make fixpoint
```

`make fixpoint` builds gen3, `src_selfhost` compiled by `binaries/silica-gen2`, and passes only if gen3 is byte-identical to gen2. Passing the trials shows gen2 compiles programs correctly; the fixed point shows the compiler reproduces itself with nothing inherited from the seed, which is the condition for retiring the bootstrap.

It must run right after `make gen2`: the `.sams` left in `src_selfhost/` are then gen1's emission of the sources, and `make gen2` stamps them. The target refuses to run if that stamp is missing or any `.sams` was rewritten since, saves those `.sams` to `compiler/silica-compiler/.src_selfhost_fixpoint/gen1_sams/`, builds gen3 with gen2, and prints how many units gen2 emits differently from gen1 (with the first differing lines, `o<N>` node counters normalised) and both binary sizes. gen3 is linked at the same path as gen2, `src_selfhost/silica-compiler`, because Apple's linker derives the binary's UUID and signature identifier from the output path; it is not installed and is kept as `silica-compiler-gen3`. Running it again needs a fresh `make gen2`.

## Run the tree or one suite with any compiler

```bash
cd trials
make integrate                                    # default: binaries/silica-compiler (the newest selfhost build)
make integrate SILICA_COMPILER=/path/to/compiler  # any binary, e.g. ../binaries/seed-compiler
make -C case_addition integrate SILICA_COMPILER=/path/to/compiler   # one suite
```

`make integrate` is the Makefile default, so plain `make` in `trials/` does the same. A few suites are not part of the tree run and must be run directly: any suite with an `INTEGRATE_PENDING` marker is skipped (see its README).

## Run the trials on an ESP32-S3 board

The same trials can run on an ESP32-S3 board over USB: compiled by `binaries/silica-compiler-ESP32-S3_raw`, loaded into the board's RAM one at a time, and compared with the same `.scout` goldens. An interactive `make integrate` asks where to run (Enter = this Mac); set `TRIAL_TARGET` to skip the question:

```bash
cd trials
make integrate TRIAL_TARGET=ESP32-S3_raw          # the whole tree on the board (takes hours)
make integrate TRIAL_TARGET=both                  # this Mac, then the board
make -C case_addition integrate TRIAL_TARGET=ESP32-S3_raw   # one suite on the board
```

Only one trial run, host or board, can be in progress at a time; a second one stops and names the run in progress. The board run writes its report to `trials/.integrate_report.ESP32-S3_raw` and leaves the host's results untouched. How it works, what it skips and its settings: [trials/targets/README.md](https://github.com/yenrab/silica/blob/main/trials/targets/README.md). What it needs installed: [Required software]({{ '/required-software/' | relative_url }}).

## Reading a report

`trials/.integrate_report` starts with the totals and then lists each failing trial with its reason (`.sams differs from .ascomp`, `.sout differs from .scout`, `.cur_fail differs from .golden_fail`, `compilation failed`, `missing executable`, and so on).

- **Trust the totals.** If the `fail:` count is higher than the number of listed lines, a suite failed as a whole (for example its batch compile aborted on one trial) and has no per-trial lines; open that suite's `.integrate_log` for the reason.
- **`missing executable (assemble/link failed earlier)`** on hundreds of trials in one suite usually means a single `.sams` in that suite failed to assemble; the `assemble failed` line names it.
- **Assembly drift** (`.sams differs from .ascomp`) with matching runtime output is expected after an emitter change; the report prints the first differing lines. Goldens are never regenerated automatically: copy a trial's `.sams` over its `.ascomp` only after confirming the runtime result, and use `make record-golden SUB_TRIALS=<leaf>` in `ordered_data_structures/` for those leaves.
- `trials/count_trial_files.sh` prints how many golden files (`.ascomp`, `.scout`, `.golden_fail`) the tree holds.

Other useful targets in `trials/`:

- `make clean` — remove generated executables, `.sams`, `.o`, `.sout`, and per-suite counters (keeps the goldens).
- `make help` — list targets and suites.
- `make integrate-ffi` — the success-path FFI application trials only.

The harness assumes the same Apple Silicon / macOS toolchain as the compiler build (see [Required software]({{ '/required-software/' | relative_url }})). Some suites have their own READMEs.

