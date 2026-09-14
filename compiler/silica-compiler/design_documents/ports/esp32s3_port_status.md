# `ESP32-S3_raw` port status — behaviours verified, differences from Apple Silicon, and gaps

State as of 2026-09-13, for `binaries/silica-compiler-ESP32-S3_raw` = `silica-999998-ESP32_S3_raw-macos-applesilicon`
and the board pack in `src_selfhost/emitter/ESP32-S3_raw/board/`. The reference behaviour is the Apple
Silicon path's first fixed point (the current `trials/` tree, [ROADMAP.md](../../../../ROADMAP.md)); this
document says how much of it the ESP32-S3 path has, how each part was verified, where an app author sees a
different behaviour on the board, and what is missing — ordered so the next addition to port is easy to pick.
It is updated by hand when a suite is run on the board or a gap closes.

## Related documents

| Document | Role here |
| --- | --- |
| [esp32s3_xtensa_port.md](esp32s3_xtensa_port.md) | The design: virtual registers, frames, calls, runtime, first-version limits |
| `emitter/ESP32-S3_raw/board/README.md` | The board pack: memory map, console protocol, tools, the fifteen bring-up apps |
| [trials/targets/README.md](../../../../trials/targets/README.md) | Running the trial tree on the board; the skip list `ESP32-S3_raw.skip` |
| [porting_for_os_free_targets.md](../porting_for_os_free_targets.md) | What a complete OS-free port needs (MMIO, device registers) |

## 1. Verified on the board

Everything below was compiled by the ESP32-S3 compiler, loaded into the board's RAM and compared with the
same `.scout` goldens the host uses (behaviour only: there are no Xtensa `.ascomp` goldens).

| What | Result | When |
| --- | --- | --- |
| Bring-up apps `silica_00`..`silica_14` (return, arithmetic, calls, case, print, wide int64, strings, 2000-deep recursion, stack guard, lists, records and tuples, regions and buffers, float64/32/16, checked int64, the stdlib `wbt_map`) | 15 / 15 match the host's output byte for byte | 2026-09-13 |
| `tuples_addition` | 630 / 630 | 2026-09-13 |
| `float64_addition` | 322 / 322 | 2026-09-13 |
| `float32_addition` | 316 / 316 (after the nan/inf printing fix, §2) | 2026-09-13 |
| `float16_addition` | 303 / 303 (after `silica_rt_f16_to_i32`, §2) | 2026-09-13 |
| `modules_addition` | 253 / 253 (7 m 51 s, about 1.9 s per trial) | 2026-09-13 |
| `traits_addition` | 20 / 20 programs (the other 18 units are trait modules, compiled with them) | 2026-09-13 |
| `deep_frame_spill_addition` | 8 / 8 | 2026-09-13 |
| `error_enforcement_addition` | a 40-trial sample of `.golden_fail` diagnostics, 40 / 40 | 2026-09-13 |

Not yet run on the board (§4.3) is the larger part of the tree; nothing there is known to fail, but nothing
there is known to pass either.

## 2. Behaviours that differ from Apple Silicon

What a program sees on the board that it would not see on the Mac. "Same" means the trials could not tell
the two apart.

| Area | Apple Silicon | ESP32-S3_raw | Consequence |
| --- | --- | --- | --- |
| Console | `write(2)` to stdout / stderr | UART0, one stream; program output between `\x02SILICA:START\x03` and the exit marker | `.sout` comparisons are the same because the host harness merges stderr into `.sout` too. There is no separate stderr on the board: `print_string_stderr` goes to the same UART. |
| Exit status | process exit code | low byte of `main`'s result in the exit marker | Same for 0–255. |
| Division by zero, no case arm | `badarith` / `case_clause` on stderr, status 1 | the same text on the UART, status 1 | Same. |
| `abort` | SIGABRT, status 134 | status 134 | Same. |
| A bad memory access | the sync fault handler prints `[silica] fault at 0x… in <fn>+0x… addr=0x…`, status 70 | the fatal vector prints `exccause=<n> epc=… excvaddr=…` (`(stack guard …)` for cause 1006), status **139** | Different status and message. A trial whose `.scout` expects a host fault would need a `<trial>.ESP32-S3_raw.scout`. None of the suites run so far expects one. |
| Stack overflow | 8 MB main stack (`-stack_size 0x10000000` for the compiler itself), SIGSEGV status 139 | 128 KB machine stack + 64 KB auxiliary (`SP`) stack, each with a 64-byte guard watched by an Xtensa data breakpoint; status 139 | ~2000 nested frames verified (`silica_07`). Deeper recursion that the host survives faults on the board. |
| Heap and regions | region arenas with release; `free` returns memory | one bump allocator over the rest of SRAM; `free`, `region_free`, `region_destroy` are no-ops; `region_grow` / `region_contains` work | A program that allocates and frees in a long loop exhausts ~300 KB on the board where the host runs indefinitely. |
| Program + data size | effectively unlimited | ≈ 390 KB of SRAM for code, data, heap and both stacks (`0x3FC88000`–`0x3FCE9700`); the linker refuses an image with no heap left | Very large programs do not fit. The ~40 KB deep_frame_spill programs fit with room to spare. |
| int64 / uint64 | native | pairs of 32-bit registers; multiply, divide and remainder through libgcc (`__muldi3`, `__divdi3`, …) | Same results; slower. |
| float64 | FPU (`D` registers) | libgcc soft float, IEEE double | Same digits (verified by every float64 trial); slower. |
| float32 | FPU | the LX7 FPU | Same. |
| float16 | FPU (`H` registers, FEAT_FP16) | widened to float32 by `silica_rt_h2f`, narrowed by `silica_rt_f2h` (round to nearest even) | Same results in `float16_addition`. |
| Printing non-finite float32 | `FCVTZS` saturates: nan prints `-0.0000000`, ±inf `±2147483647.0000000` | reproduced deliberately (`rt_float.S`) | Same. float64 / float16 print `nan`, `inf`, `-inf` on both. |
| A `main() -> float16 / float32 / float64` exit code | `FCVTZS` (nan → 0, saturated) | `silica_rt_f16_to_i32`; float32 via `trunc.s`; float64 via libgcc | Same for the values the trials use; saturation of huge float64 values is libgcc's, not `FCVTZS`'s (untested). |
| Strings | inline `L_*` helpers per module | `rt_string.S` (`concat`, lengths, `eq`/`cmp`, `starts_with`/`ends_with`/`contains`, `substring`, `substring_until_char`), same 24-byte header | Same where tested (`silica_06`, modules, tuples). `string_addition` not yet run. |
| Lists | inline chunked lists | `rt_list.S` (`length`, `tail`, `at`, `prepend`), same layout | Same where tested (`silica_09`, `silica_14`). `list_addition` not yet run. |
| Tail calls | self tail calls are jumps | self tail calls are jumps; a tail call to another function still grows the stack | Mutual recursion depth is bounded by the 128 KB stack. |
| Timing | — | `silica_rt_delay_us`, `silica_rt_cycles` (board only) | Board-only surface, used by the `asm_*` apps; not reachable from Silica yet (§4.2). |
| Compile time | — | the ESP32 emitter is about 9× slower on very large functions (deep_frame_spill: 88.7 s vs 9.6 s per trial); small trials compile in the same time | A whole-tree board run is dominated by board time, not compile time, except for that suite. |

## 3. Behaviours the emitter refuses

A module that uses one of these does not assemble: the emitter writes one `.error` line naming the feature
instead of Xtensa text, so the failure is immediate and legible.

| Construct | `.error` text | Why |
| --- | --- | --- |
| Actors: `spawn`, `send`, `recv`, `call`, `cast`, `link`, `monitor`, registration, supervisors | `ESP32-S3: actors are not supported yet (<prim> -> <reg>)` | No actor runtime on the board (`rt_actors_stub.S` only initialises the registries). The host runtime is pthreads, `os_unfair_lock`, `__ulock`; the board needs a cooperative single-core scheduler (design pending). |
| Foreign calls (`ffi`) | `ESP32-S3: foreign (C) calls are not supported on this target` | No C runtime; the host's guarded FFI is setjmp / signal based. |
| File io | `ESP32-S3: file io is not supported on this target (<prim> -> <reg>)` | No filesystem. |
| A frame ENTRY cannot allocate | `frame larger than ENTRY can allocate (32760 bytes)` | Xtensa windowed-ABI limit; not hit by any trial so far. |

## 4. Gaps, in the order that makes the next addition easy

### 4.1 Language behaviours of Apple Silicon FP1 that the board does not have

| Gap | What it blocks in `trials/` | What closing it needs |
| --- | --- | --- |
| Actor runtime (spawn / send / recv / call / cast / link / monitor, mailboxes, the pid registry, `remove_actor`, exit reporting) | `actors_addition` (294), `actor_registration_addition` (3), `supervisors_addition` (106) | A cooperative scheduler on one core: per-actor stacks in SRAM, mailboxes, `recv` as a yield point, the failure / unwind reports the host prints, then the supervisor trampolines in `module_linkage.silica` and `prims_actors.silica` re-targeted from `.error` to `silica_rt_actor_*` routines (the AArch64 dispatcher is kept as `emit_prim_op_aarch64` for reference). Chunk 1's growable actor stacks apply here too. |
| CPU topology and core placement | `cpu_discovery_and_spawn_pinning` (4) | Follows the actor runtime; the ESP32-S3 has two LX7 cores (roadmap chunk 2). Today the port is single core with interrupts off. |
| Foreign calls | `ffi_addition` (16 apps), `warning_enforcement_addition` (its fixtures build C archives) | Not applicable on bare metal unless a C runtime is brought in; record as not applicable per [porting_for_os_free_targets.md §4](../porting_for_os_free_targets.md). |
| File io | no dedicated suite; used inside some trials | Not applicable without a filesystem. |
| Host fault semantics (status 70 and the `[silica] fault at …` line) | any trial whose golden encodes a host fault | Either teach `rt_console.S`'s fatal report the host's line and status, or keep 139 and record per-target `.scout` files as such trials appear. |
| Memory release | none directly; long-running allocation loops | A free list or region release in `rt_heap.S` (the host's release entry points are the model). |

### 4.2 ESP32-S3 FP1 items beyond Apple Silicon FP1

The roadmap defines ESP32-S3 FP1 as Apple Silicon FP1 **plus peek and poke** (`map_device`, volatile
`device_load*` / `device_store*`, `spawn_device`, `register_rwr` ordering; [ROADMAP.md](../../../../ROADMAP.md),
[porting_for_os_free_targets.md §5](../porting_for_os_free_targets.md)). Nothing of it exists yet in the
emitter or the runtime: GPIO, delays and the cycle counter are reachable only from hand-written assembly
(`rt_board.S`, the `asm_*` apps). This is the first board-specific addition after the trial tree is green.

### 4.3 Trial suites not yet run on the board

Every suite below is expected to work — the emitter converts every non-actor primitive and the bring-up apps
cover each area once — but has not been run. Times are estimates at ~1.9 s per program trial.

| Suite | Programs | Estimated board time | Note |
| --- | --- | --- | --- |
| `case_addition` | 747 | 24 min | |
| `string_addition` | 669 | 21 min | the largest untested runtime surface (`rt_string.S`) |
| `functions_addition` | 645 | 20 min | |
| `list_addition` | 529 | 17 min | `rt_list.S` |
| `records_addition` | 457 | 15 min | |
| `compiler_addition` | 425 | 14 min | |
| `int64_addition`, `int32_addition`, `int16_addition`, `int8_addition` | 414, 402, 404, 373 | 51 min | libgcc 64-bit division on the board |
| `uint64_addition`, `uint32_addition`, `uint16_addition`, `uint8_addition` | 409, 405, 412, 411 | 52 min | |
| `memory_region_addition` | 362 | 12 min | bump heap; `free` is a no-op |
| `effect_check_addition` | 342 | 11 min | |
| `negation_addition`, `recursive_function_addition`, `sequence_block_addition` | 324, 323, 276 | 29 min | |
| `bitwise_addition`, `boolean_addition`, `atoms_addition`, `base` | 294, 208, 212, 84 | 25 min | |
| `error_enforcement_addition` | 10,829 compile-failure trials (compile only, 4 at a time) | ~15 min | 40 sampled so far; its 7 sub-suites with their own Makefiles are skipped by the driver (§4.4) |

About 5–6 hours in one run (`make integrate TRIAL_TARGET=ESP32-S3_raw`), which holds the trial lock for that
long; per suite with `TRIAL_SUITES=`. The Mac is kept awake by the driver.

### 4.4 Trial driver gaps (`trials/targets/`)

| Gap | Effect | What closing it needs |
| --- | --- | --- |
| `ordered_data_structures` | skipped (`driver:`) | Its 10 sub-suites share one stdlib build and put programs in leaf directories; `board_suite.sh` handles one flat suite. |
| Sub-suites with their own Makefile (the 7 under `error_enforcement_addition`) | skipped automatically, reported | Recurse into them the way the host `Makefile`s do. |
| No Xtensa `.ascomp` | assembly is never compared on the board target | By design for now; an Xtensa golden set would need its own hand-derivation policy. |
| One board | suites compile in parallel but run one trial at a time on the single board | More boards, `BOARD_PORT` per worker. |

### 4.5 Emitter performance

Very large functions compile ~9× slower than on the host (the `xt_*` helper layer builds every instruction
from its AArch64 operand text). Only `deep_frame_spill_addition` shows it; a profile of `xt_isa` / `xt_mem`
on one of those units is the starting point.

### 4.6 Host defects the board apps still avoid

`silica_04_print` binds every `print_int64` result because a bare `print_int64(<literal>);` statement drops
its value on the host too (`trials/sequence_block_addition/sequence_print_int64_literal_statement`); the port
reproduces the host exactly. Three others found by the apps were fixed on 2026-09-13 in all three emitter
trees (tuple literal as a record field value; a `case` yielding a float; float parameters clobbered by
float-prim staging) and are covered by trials; the linux_aarch64 tree does not have those fixes yet.

## 5. How to update this document

- A suite run clean on the board moves from §4.3 to §1 with its count and date.
- A behaviour difference discovered by a trial goes in §2, with the `<trial>.ESP32-S3_raw.scout` it needed.
- A gap closed in §4 is removed there and, if it changed what a program sees, noted in §2.
