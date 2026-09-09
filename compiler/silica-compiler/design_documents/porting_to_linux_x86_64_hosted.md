# Porting Silica to Linux x86_64 (hosted), on one Linux machine

**Status:** Plan. Not implemented. Not normative; [silica-specification.md](silica-specification.md) governs language and ABI semantics. Use this document when adding the `linux_x86_64` emit backend, producing the first Linux seed and selfhost compilers, and deploying to DigitalOcean droplets.

**Scope of execution:** everything in this plan runs on a single Linux x86_64 machine. No macOS host, no VM, no CI runner. The machine measured while writing this: Ubuntu 25.10, glibc 2.42, 2 cores, 3 GB RAM, GNU Make 4.4.1, GNU as 2.45; no clang, LLVM, or Rust installed yet.

**Audience:** someone who knows the `src/` bootstrap build, the `src_selfhost/` batch build (`silica.config`, `emit_target.mk`), and the `binaries/` seed and selfhost naming.

## Related documents

| Document | Role here |
| --- | --- |
| [Phase1_TODOs/bootstrap_retirement_and_self_host_plan.md](Phase1_TODOs/bootstrap_retirement_and_self_host_plan.md) | Seed and selfhost conventions, batch mode, Step 1.3 stack limits. This plan keeps the bootstrap alive longer on Linux than that plan intends on macOS, see §2 |
| [direct_machine_object_emitter_future.md](direct_machine_object_emitter_future.md) | Reserves `chip/x86_64`; the backend split here is shaped to become that layer later |
| [porting_for_os_free_targets.md](porting_for_os_free_targets.md) | Board and OS-free ports; scopes hosted ports out, this document covers them |
| [actor_spawn_core_affinity_os_semantics.md](actor_spawn_core_affinity_os_semantics.md) | Linux affinity semantics for the actor runtime |
| [silica_ffi_wrapper_specification.md](silica_ffi_wrapper_specification.md) | Fifi boundary the SysV ABI work must satisfy |
| [silica-specification.md](silica-specification.md) §15.4.13.5 | x86-64 named as supported; MPK containment deferred (§9) |

---

## 1. Facts that shape the plan

1. **No committed compiler runs here.** Every binary in `binaries/` is Mach-O AArch64. On this machine the only way to obtain a first Silica compiler is to build the Rust bootstrap (`silica-bootstrap-compiler`) and let it compile the frozen `src/` tree. That is the sole entry point, so the plan is organized around it.
2. **The bootstrap chain is portable but pinned to old LLVM.** The bootstrap emits LLVM IR text, then `llvm-as` and `llc` produce `main.o`, and clang links it with the Rust static runtime. The IR uses typed pointers (`i8*`, 898 sites in `codegen.rs`), which LLVM 17 and later reject. Ubuntu 25.10 ships only LLVM 20. LLVM 15 or 16 must come from a prebuilt release tarball. The Rust side has no such problem: `edition = "2021"`, apt's `cargo` 1.85 suffices, and `runtime.rs` already has a generic fallback for platforms other than macOS and Linux AArch64.
3. **The emitter is register-bound to AArch64.** Measured on `src_selfhost/emitter/apple_silicon_mac`:

   | Measure | Count |
   | --- | --- |
   | Silica source files | 87 |
   | Lines | 27,656 |
   | Files with literal `X<n>` register names | 71 |
   | Literal `X<n>` mentions | 4,534 |
   | Files with inline asm | 43 |
   | Files with Darwin syscalls (`SVC #0x80`) | 12 |
   | Files with Mach-O section directives | 21 |
   | Files with `@PAGE` / `@PAGEOFF` | 30 |
   | Files with no asm (target neutral) | 18 |

   The x86_64 backend is a rewrite of the asm-bearing files, not an edit of the Apple tree.
4. **The backend must exist in two trees, and the first copy must be in the bootstrap dialect.** The seed is built by the bootstrap from `src/`, and whatever `src/emitter/<TARGET>/` it is built with decides what the seed emits. To get runnable Linux output from the seed, the new backend must live in `src/emitter/linux_x86_64/` and compile under the bootstrap's Silica subset (the "W" workarounds catalogued in the retirement plan). Later the selfhost compiler needs its own copy in `src_selfhost/emitter/linux_x86_64/`. The two Apple copies already differ in every file and `src_selfhost` has extra helper modules, so the copies will not be byte-identical; §4 says how to keep them close.
5. **Trials check assembly goldens, not only program output.** `trials/*/Makefile integrate` diffs each emitted `.sams` against a committed `.ascomp` and each program's `.sout` against `.scout`. The `.ascomp` files are Apple AArch64. Program output (`.scout`) is target neutral and is the real oracle for the port; assembly goldens must become per target.
6. **The Apple backend cannot be executed here.** Its emitted text can still be produced on Linux by a seed built with `TARGET=apple_silicon_mac`, which gives a text-only regression oracle (see Phase 1) but no way to run the result. This plan therefore does not refactor the Apple backend.
7. **Memory is tight.** The seed is compiled as one LLVM unit (`main.ll` holds the entire compiler). `llc` on that unit and `cargo build --release` both need headroom on 3 GB. Swap is a Phase 0 item.

---

## 2. The bootstrap chain on Linux

```
apt: cargo, clang, lld, make, gdb        prebuilt tarball: LLVM 15 (llvm-as, llc)
        │                                          │
        ▼                                          ▼
cargo build --release --no-default-features  ──► silica-boot + libsilica_compiler.a   (Phase 1)
        │
        ▼
silica-boot compiles src/ with TARGET=apple_silicon_mac ──► seed that emits Apple text   (Phase 1 smoke; text-diff oracle only)
silica-boot compiles src/ with TARGET=linux_x86_64     ──► Linux seed  silica-NNNNNN-seed-debian-x86_64   (Phase 3)
        │
        ▼
Linux seed compiles src_selfhost/ with TARGET=linux_x86_64 ──► Linux selfhost  silica-NNNNNN-debian-x86_64   (Phase 4)
        │
        ▼
selfhost recompiles src_selfhost/ ──► byte-identical .sams (fixpoint)   (Phase 5)
```

Droplets never need Rust or LLVM 15. They receive the committed selfhost binary and need only `clang` (assembler and linker driver) and `make` (Phase 6).

---

## 3. Vocabulary (decide in Phase 0, then never revisit)

| Concept | Value | Where defined |
| --- | --- | --- |
| Emit target directory | `linux_x86_64` | `src/emitter/linux_x86_64/`, `src_selfhost/emitter/linux_x86_64/` |
| Host platform id | `debian-x86_64`, already accepted by `binaries/update_silica_compiler_link.bash` and produced by `install_compiler.bash` on Ubuntu | `project_makefiles/platform/platforms.mk` |
| Host default | Linux + x86_64 → `linux_x86_64` / `debian-x86_64` | same file; `emit_target.mk` includes it and drops its own candidate list |
| Assembler syntax | GNU as with `.intel_syntax noprefix`: destination first like the ARM text the team reads; accepted by GNU as and clang | backend `platform/` |
| Object format | ELF64; every unit ends with the `.note.GNU-stack` directive | backend `platform/` |
| C ABI at boundaries | System V AMD64 | backend `platform/`, `terms/ffi_*` |
| CPU baseline | x86-64-v3 (AVX2, F16C, BMI2). Current droplet classes meet it; F16C gives cheap `float16` conversion. Runtime checks CPUID once at startup and aborts with a clear message | backend `runtime/` |
| Assembler and linker on Linux | `clang` from apt (LLVM 20) for `.sams` → `.o` and final links, `-fuse-ld=lld` | `project_makefiles/platform/linux_x86_64.mk` |
| IR tools for the bootstrap only | LLVM 15 prebuilt under `$HOME/llvm-15`, exposed as `SILICA_LLVM_PATH` (the variable `setup_silica.sh` already uses) | `deploy/linux/install_deps.sh` |
| Per-target assembly goldens | `<unit>.<target>.ascomp`, for example `add.linux_x86_64.ascomp`; bare `.ascomp` stays Apple | `trials/platform/*.mk` |

---

## 4. Directory changes

```
compiler/silica-bootstrap-compiler/          # unchanged source; built on Linux
compiler/silica-compiler/
  src/
    emitter/linux_x86_64/                    # NEW: backend in bootstrap dialect (additive; no existing src/ file edited)
      emitter_core.silica  module_linkage.silica
      platform/   regs/   runtime/   terms/  control/  recursion/  atoms/  constraints/  shared/
  src_selfhost/
    emitter/linux_x86_64/                    # NEW: same backend plus a thin adapter to selfhost-only helpers (Phase 4)
    runtime/linux_x86_64/                    # NEW: silica_rt_shim.s (SysV), deviceio_link_thunks.s
    runtime/apple_silicon_mac/               # MOVE of the three root .s files, so the link recipe is per target
  stdlib/**/build/<target>/                  # NEW: emitted .sams/.ascomp outputs per target; root Darwin copies stop being overwritten
  tools/
    ll_opaque_ptr.py                         # NEW, fallback only: rewrites typed-pointer .ll to opaque pointers if LLVM 15 cannot run here
project_makefiles/
  platform/                                  # NEW: platforms.mk (name table), apple_silicon_mac.mk, linux_x86_64.mk
trials/
  platform/                                  # NEW: per-platform assembler/linker flags, FFI fixture CFLAGS, skip list, golden suffix
  */*.linux_x86_64.ascomp                    # NEW goldens, committed as each ladder step stabilizes (Phase 3)
deploy/
  linux/                                     # NEW: install_deps.sh (dev machine), install_silica.sh (droplet)
binaries/
  silica-NNNNNN-seed-debian-x86_64           # NEW committed seed (Phase 3 exit)
  silica-NNNNNN-debian-x86_64                # NEW committed selfhost (Phase 5 exit)
```

Rules for the two backend copies: the `src/` copy is written first and is the reference; the `src_selfhost/` copy is produced by copying and then applying only the adapter changes needed for selfhost-only modules. A `make -C src_selfhost backend-drift` target runs `diff -r` between the two and prints the delta so it stays reviewable. After Phase 5 the `src/` copy is frozen like its Apple sibling.

---

## 5. Global rules

- **`src/` is additive only.** New directory `emitter/linux_x86_64/`, and `Makefile` lines that are platform includes. No existing Silica file in `src/` changes.
- **The Apple backend is not touched.** It cannot be executed here. Shared code is duplicated into the new backend (about 18 files) rather than refactored out of the Apple tree; consolidation is a later project once both targets can be run.
- **Program output is the oracle; assembly goldens are per target.** A trial passes on Linux when every `.sout` matches its `.scout`. `.linux_x86_64.ascomp` goldens are committed once a step is stable and then guard against regressions.
- **One name table** (§3) included by every makefile and both `binaries/*.bash` scripts.
- **No register or directive literal outside `platform/` and `regs/`** in the new backend. `make lint-backend` greps for `\b(r[a-z0-9]{1,3}|e[a-d]x|xmm[0-9]+)\b`, `.section`, `.L`, and `syscall` outside those directories and fails on any hit.
- **Every recipe that runs a Silica compiler is wrapped by `ulimit -s`** until Step 3.8 lands the emitted stack switch; the wrapper lives in one place, `project_makefiles/platform/linux_x86_64.mk`.

---

## 6. Phases

### Phase 0 — Machine preparation (no Silica code)

**Step 0.1 — Packages.** `deploy/linux/install_deps.sh`: `sudo apt install clang lld llvm make gdb cargo rustc`. Verify `clang --version` (20), `cargo --version` (1.85), `as --version`.

**Step 0.2 — LLVM 15 for the bootstrap.** Download the `clang+llvm-15.0.7-x86_64-linux-gnu-ubuntu-18.04.tar.xz` release tarball into `$HOME/llvm-15` and export `SILICA_LLVM_PATH=$HOME/llvm-15/bin`. Prove it with a one-line typed-pointer `.ll` through `llvm-as` then `llc -filetype=obj`. Known snag: those binaries link `libtinfo.so.5`, which Ubuntu 25.10 no longer ships; a `libtinfo5` package from an older release or a `libtinfo.so.5 → libtinfo.so.6` symlink in `$HOME/llvm-15/lib` resolves it. If LLVM 15 cannot be made to run, fall back to `tools/ll_opaque_ptr.py` (typed pointers → `ptr`, pointer-to-pointer `bitcast` removed by substitution) and use LLVM 20's `llvm-as`/`llc`; that path is a bridge, not the default.

**Step 0.3 — Memory.** Add a 4 GB swapfile. Record `free -g` before and after. `JOBS=2` everywhere.

**Step 0.4 — Name table.** Write `project_makefiles/platform/platforms.mk` (two rows, Apple and Linux x86_64). Point `emit_target.mk`, `install_compiler.bash`, and `update_silica_compiler_link.bash` at it.

**Step 0.5 — Register and frame model (the deepest design risk).** The Apple lowering assumes AAPCS64: eight integer argument registers, ten callee-saved (`X19`–`X28`) used as spill slots by `count_callee_saved_spill_pressure`, `X16`/`X17` scratch, separate float registers. System V x86_64 has six integer argument registers (`rdi rsi rdx rcx r8 r9`), five callee-saved GPRs (`rbx r12 r13 r14 r15`), `rax`/`rdx` return, `r10`/`r11` scratch, `xmm0`–`xmm7` float arguments. Write `src/emitter/linux_x86_64/regs/README.md` stating:

- Internal Silica convention is SysV-compatible for scalars, so every libc call site is ordinary and there is one convention to hold in mind.
- The callee-saved spill role is filled by `rbx r12–r15`; pressure beyond five spills to frame slots `[rbp - 8*k]`. The pressure counter returns a `(registers, slots)` pair.
- Aggregate returns follow SysV hidden-pointer rules at FFI boundaries; internal aggregate scheme is one documented choice per site.
- `float16` is stored as 16 bits, converted with `vcvtph2ps`/`vcvtps2ph`, computed in `float32`.

**Exit gate:** LLVM 15 `llvm-as` and `llc` run here; swap present; `platforms.mk` included by all consumers; `regs/README.md` written before any lowering code.

### Phase 1 — Bootstrap and seed chain on Linux (no new Silica code)

**Step 1.1 — Build the bootstrap.** `cd compiler/silica-bootstrap-compiler && cargo build --release --no-default-features`. Fix only what fails to compile on Linux, if anything; `runtime.rs` already has the generic topology fallback. Record peak memory.

**Step 1.2 — Linux link rules for the seed.** `project_makefiles/platform/linux_x86_64.mk` provides `LLVM_AS`, `LLC` (from `SILICA_LLVM_PATH`), `LINKER_CLANG`, `LDFLAGS_RUNTIME := -lpthread -ldl -lm -lgcc_s` (what a Rust `staticlib` needs on glibc), `LDFLAGS_STACK :=` (empty), and the `ulimit -s` wrapper. `src/Makefile` includes the fragment for the host platform; its Apple values move to `apple_silicon_mac.mk` unchanged.

**Step 1.3 — Smoke: build the seed with the Apple backend.** `make -C src TARGET=apple_silicon_mac INSTALL_SEED=0`. The result is an ELF x86_64 binary containing the whole frontend and the Apple emitter. It cannot produce runnable output here, but it proves cargo, LLVM 15, `llc` memory fit, and the Linux link line, with zero new Silica.

**Step 1.4 — Text-only frontend regression oracle.** With that seed as `SILICA_COMPILER`, run each trial's compile step only and diff `.sams` against the committed Apple `.ascomp`. Wrap this as `make -C trials golden-text TARGET=apple_silicon_mac`. Matching goldens prove the Linux-built frontend and the bootstrap runtime behave as on macOS. Any mismatch here is a Linux bootstrap bug to fix before writing a backend.

**Exit gate:** `golden-text` matches for every trial that has `.ascomp` files.

### Phase 2 — Linux assemble, link, run path (hand assembly, no backend)

**Step 2.1 — Trial platform fragment.** `trials/platform/linux_x86_64.mk`: `ASSEMBLER := clang`, `ASFLAGS := -c -x assembler`, `LINKER := clang`, `LDFLAGS := -fuse-ld=lld -lpthread`, FFI fixture `CFLAGS := -std=c11 -O2 -fPIC`, `GOLDEN_SUFFIX := .linux_x86_64.ascomp`, `SKIP_TRIALS := cpu_discovery_and_spawn_pinning` (until Step 3.10). Trial makefiles include the host fragment instead of their inline macOS blocks; the Apple values move to `trials/platform/apple_silicon_mac.mk` unchanged.

**Step 2.2 — Golden handling.** `integrate` compares against `<unit>$(GOLDEN_SUFFIX)` when it exists, reports "no golden yet" without failing when it does not, and always requires `.sout` to equal `.scout`.

**Step 2.3 — Hand-written smoke.** `trials/base/linux_x86_64_smoke/hello.sams`: `.intel_syntax noprefix`, `.text`, `main` that `syscall`s `write` and returns 0, GNU-stack note. `make -C trials/base/linux_x86_64_smoke integrate PLATFORM=linux_x86_64` passes using only clang and make.

**Exit gate:** the smoke trial passes; all other trials still compile-and-golden-check with the Apple text seed from Phase 1.

### Phase 3 — The `linux_x86_64` backend in `src/` (seed rebuilt each step)

Create `src/emitter/linux_x86_64/` by copying the Apple tree's structure and function signatures, not its asm bodies. Write in the bootstrap dialect (Fact 4). The loop for every step: edit backend → `make -C src TARGET=linux_x86_64 INSTALL_SEED=0` → `make -C trials/<trial> integrate SILICA_COMPILER=.../src/silica-compiler PLATFORM=linux_x86_64` → debug with `gdb` on the produced binary → commit `.linux_x86_64.ascomp` when stable.

| Step | Scope | Trials that must pass |
| --- | --- | --- |
| 3.1 | `platform/` (ELF sections, `.L` labels, `lea reg, [rip + sym]`, `syscall` wrappers, identity `c_symbol`), `emitter_core` skeleton, `module_linkage`, `main` via crt1 (no `_main` alias), `print_int64` | `base`, `int64_addition` |
| 3.2 | `regs/` and frame model from Step 0.5; `rbp` frames; spill slots; `let`/`const` int64 | `functions_addition`, shallow `recursive_function_addition` |
| 3.3 | Narrow and unsigned ints, negation, bitwise; `movsx`/`movzx` where AArch64 used `W` registers | `int8..uint64_addition`, `negation_addition`, `bitwise_addition` |
| 3.4 | Booleans, atoms, atom rodata, `case`, control flow, sequence blocks | `boolean_addition`, `atoms_addition`, `case_addition`, `sequence_block_addition` |
| 3.5 | String runtime: arena on `mmap`/`munmap` `syscall`s, length, eq, concat, substring, predicates, literal pool | `string_addition` |
| 3.6 | Records, tuples, lists, aggregate conventions from 0.5 | `records_addition`, `tuples_addition`, `list_addition` |
| 3.7 | Floats: SSE2 scalar float64/float32, F16C float16, literal pools, printing | `float16/32/64_addition` |
| 3.8 | Deep recursion and stack: emitted `main` `mmap`s a 256 MB stack and switches `rsp`, restoring before exit. Removes the `ulimit` wrapper for compiled programs and closes retirement-plan Step 1.3 for Linux | `deep_frame_spill_addition`, full `recursive_function_addition` |
| 3.9 | Modules, traits, effects, error and warning enforcement; ELF visibility (`.globl`, `.hidden`) | `modules_addition`, `traits_addition`, `effect_check_addition`, `error_enforcement_addition`, `warning_enforcement_addition` |
| 3.10 | Actor runtime: `pthread_create`/`detach`/`exit`, `pthread_key_*` TLS, `pthread_mutex_*` replacing `os_unfair_lock`, `gettid` replacing `pthread_mach_thread_np`, `sched_setaffinity`, supervisors, pid registry | `actors_addition`, `actor_registration_addition`, `supervisors_addition`, `cpu_discovery_and_spawn_pinning` (leave skip list) |
| 3.11 | FFI: SysV classification for the ABI types the trials use, `al` vector count for varargs, 16-byte call alignment, guarded and fault runtimes on glibc `sigaction`/`setjmp`; Linux `silica_rt_shim.s` | `ffi_addition/**` |
| 3.12 | Memory regions, failure reporter (no MTE tag section per spec §15.4.13.6), standard data structures, compiler trial | `memory_region_addition`, `standard_data_structures/**`, `compiler_addition` |

**Step 3.13 — Publish the seed.** `make -C src TARGET=linux_x86_64 INSTALL_SEED=1` installs `binaries/silica-NNNNNN-seed-debian-x86_64` and points `seed-compiler` at it. Commit the binary and the goldens.

**Exit gate:** `make -C trials integrate PLATFORM=linux_x86_64` green with the committed seed, skip list empty, every trial has a `.linux_x86_64.ascomp`.

### Phase 4 — Backend into `src_selfhost/`, first Linux selfhost

**Step 4.1 — Copy and adapt.** `src_selfhost/emitter/linux_x86_64/` from the `src/` copy; adapt only where the selfhost tree's `emitter_core` entry points, `compiler_maps`, or `term_*` helper split require it. `make -C src_selfhost backend-drift` shows the delta; keep it small and commented.

**Step 4.2 — Runtime and link recipe.** Move the three root `.s` files to `src_selfhost/runtime/apple_silicon_mac/` (a move, no edits), add `runtime/linux_x86_64/silica_rt_shim.s` and `deviceio_link_thunks.s` in SysV form, and make the link stage assemble `runtime/$(TARGET)/*.s`.

**Step 4.3 — Build.** `make -C src_selfhost build TARGET=linux_x86_64` with the Phase 3 seed. Then `make -C src_selfhost integrate`.

**Exit gate:** selfhost integrate suite green on Linux.

### Phase 5 — Fixpoint and publication

> **Note (2026-09-08):** no fixpoint has been demonstrated on macOS either. On macOS the seed-built selfhost runs all trials, but the selfhost compiling all of `src_selfhost/`, a generation-2 build, a differential and a fixpoint are still open (see the status snapshot in `Phase1_TODOs/bootstrap_retirement_and_self_host_plan.md`). Step 5.1 here is therefore the first fixpoint run for *any* target unless macOS gets there first, and the macOS procedure should be reused verbatim when it exists.

**Step 5.1 — Generation 2.** Rebuild `src_selfhost` with the Phase 4 selfhost as the seed. The `.sams` set must be byte-identical to generation 1's. A difference means host-dependent output (uninitialized memory, address-dependent ordering); fix it before continuing.

**Step 5.2 — Resource fit.** Record peak RSS on the largest unit. If above 2 GB, file it as a compiler memory issue, separate from the port.

**Step 5.3 — Publish.** `install_compiler.bash selfhost` → `binaries/silica-NNNNNN-debian-x86_64`; `update_silica_compiler_link.bash` picks it on a Linux host. Commit.

**Exit gate:** fresh clone on this machine → `binaries/update_silica_compiler_link.bash` → `make -C trials integrate` green with no bootstrap, LLVM 15, or Rust involved.

### Phase 6 — Droplets and project builds

**Step 6.1 — `deploy/linux/install_silica.sh`.** `apt install clang lld make`, clone or pull, run the symlink updater, compile a hello project with `project_makefiles/Makefile`. Idempotent; `sudo` only for apt.

**Step 6.2 — `project_makefiles/` on Linux.** Include the platform fragment (Step 1.2); make `silica_link.sh` return ELF archives.

**Step 6.3 — Stdlib outputs per target.** Emit to `stdlib/**/build/<target>/`; `trials/stdlib_prereq.mk` reads from there; stop overwriting the committed Darwin files at the stdlib root.

**Step 6.4 — Docs.** Update `design_documents/README.md`, `tutorials_and_howtos/ffi_wrappers_and_makefiles.md` (SysV notes), the top-level `README.md` platform list, and mark this document Implemented.

**Exit gate:** a new droplet goes from image to compiling a Silica project with one script and never installs Rust or LLVM 15.

---

## 7. Order and dependencies

```
Phase 0 ──► Phase 1 ──► Phase 3 ──► Phase 4 ──► Phase 5 ──► Phase 6
              │            ▲
              └─► Phase 2 ─┘   (Phase 2 needs only Phase 0; run alongside Phase 1)
```

Phase 3 is the long pole. Phases 1 and 2 are short and remove every toolchain unknown before backend code is written.

---

## 8. Risks and what retires each

| Risk | Retired by |
| --- | --- |
| LLVM 15 prebuilt does not run on Ubuntu 25.10 (`libtinfo5`) | Step 0.2 symlink or package; `ll_opaque_ptr.py` fallback |
| `llc` on the single-unit `main.ll` exceeds 3 GB | Step 0.3 swap; `llc -O1` if needed; Step 1.3 measures before any backend work |
| Backend constrained to the bootstrap dialect | Accepted: the Apple backend was written the same way; the `src_selfhost` copy may evolve after Phase 5 |
| Two backend copies drift | `backend-drift` target; the `src/` copy freezes at Phase 5 |
| Register-pressure mismatch (10 callee-saved → 5) corrupts callers silently | Step 0.5 decided before code; `deep_frame_spill_addition` at 3.8; `lint-backend` |
| Two calling conventions drift | Step 0.5: internal is SysV-compatible for scalars |
| Apple `.ascomp` goldens fail on Linux by construction | Per-target golden suffix (Step 2.2); `.scout` is the oracle |
| Frontend behaves differently when built on Linux | Step 1.4 text oracle against Apple goldens |
| Host-dependent codegen breaks fixpoint | Step 5.1 |
| `float16` on a droplet without F16C | x86-64-v3 baseline; runtime CPUID abort |
| Main-thread stack too small | `ulimit` wrapper now; Step 3.8 stack switch permanently |
| Signal and `setjmp` layouts | Call glibc wrappers by name; never hand-lay the structs |

---

## 9. Out of scope

- **MPK containment** (spec §15.4.13.5). The first hosted port has no hardware memory protection; hosted targets may omit it. Follow-on after Phase 5.
- **Direct object emission** (`chip/x86_64` encoders). Text assembly stays; `platform/` and `regs/` are shaped to be replaced by the encoder layer later.
- **Any Apple backend refactor**, including the `emitter_common/` consolidation of the 18 neutral files. It cannot be validated by execution on this machine.
- **`linux_aarch64`.** No hardware to run it. With `platform/` and `regs/` in place it becomes a small follow-on if hardware appears.
- **Retiring the Rust bootstrap on Linux.** It is needed exactly once, to produce the first seed. After Phase 5 it is no longer on any Linux path, matching the retirement plan's end state.

---

## Appendix A — OS surface inventory of the Apple backend and Linux replacements

| Apple symbol or syscall | Where emitted | Linux x86_64 replacement |
| --- | --- | --- |
| `_malloc`, `_free`, `_abort` | many | `malloc`, `free`, `abort` (glibc) |
| `_pthread_create/_detach/_exit/_key_create/_getspecific/_setspecific` | `prims_actors_runtime_asm`, `ffi_guarded_runtime_asm` | same names without underscore (glibc) |
| `_os_unfair_lock_lock/_unlock` | `prims_actors_runtime_asm` | `pthread_mutex_lock/_unlock` |
| `_pthread_mach_thread_np` | `prims_actors_runtime_asm` | `gettid` |
| `_signal`, `_sigaction`, `_setjmp` | `ffi_fault_runtime_asm`, `ffi_guarded_runtime_asm`, actors | same names (glibc); `sigsetjmp`/`siglongjmp` if the signal mask must be restored |
| `SVC #0x80` write, exit | `print_*_inline` | `syscall` 1, 60 (`exit_group` 231 for the process) |
| `SVC #0x80` mmap, munmap (`0x2000000+197`, `+73`) | `string_*_mmap_nomte` | `syscall` 9, 11; `MAP_PRIVATE\|MAP_ANONYMOUS` = `0x22` |
| `-Wl,-stack_size` | makefiles | `ulimit -s` wrapper, then Step 3.8 stack switch |
| `_main` alias | `main_entry_alias.s` | none; crt1 calls `main` |
| `-Wl,-macos_version_min`, `-arch arm64`, SDK sysroot | trials, fixtures | none; `-fuse-ld=lld -lpthread` |
