# `emitter/ESP32-S3_raw/board/` — bare-metal ESP32-S3 board pack

Everything the `ESP32-S3_raw` emit target needs besides the emitter itself: the runtime a
compiled program links against, the memory map, the image and flash tools, and the early test
apps used to bring the port up. None of it is compiled by the Silica build (`topo_silica_config`
only lists `.silica` units); it is assembled with Espressif's Xtensa GCC toolchain.

What the port has of the host's behaviours, where a program sees a difference on the board, and what is
still missing is kept in
[design_documents/ports/esp32s3_port_status.md](../../../../design_documents/ports/esp32s3_port_status.md).

The port was tested on the board described in
[PORT_TEST_BOARD_byu_idaho_v4.0_feb26.md](PORT_TEST_BOARD_byu_idaho_v4.0_feb26.md) (a BYU-I eBadge
v4.0). Pin numbers in the apps follow that file's J4 table, which comes from the board's design
netlist; the supplied pin sheet has the RGB red pin wrong (1 instead of 2).

## Layout

| Path | What |
| --- | --- |
| `runtime/rt_vectors.S` | Exception vectors: windowed-ABI overflow/underflow handlers (from ESP-IDF), alloca, fatal-exception report |
| `runtime/rt_start.S` | `_start`: interrupts off, VECBASE, fresh window, own stack, watchdogs off, zero `.bss` + heap, `main`, exit marker |
| `runtime/rt_console.S` | UART0 output (raw FIFO writes), the `print_*` routines, `exit` / `abort` / `badarith` / `case_clause` / fault report |
| `runtime/rt_string.S` | String routines (`silica_rt_string_*`): concat, lengths, eq/cmp, predicates, substring |
| `runtime/rt_list.S` | List routines (`silica_rt_list_*`): length, tail, at, prepend over the emitter's chunked lists |
| `runtime/rt_float.S` | Float printing (host digit algorithm, including the host's `FCVTZS` behaviour on nan/inf; float64 via libgcc, float32 on the FPU), half conversions, truncation, float16 → int32 for a float16 exit code |
| `runtime/rt_ordering.S` | Ordering identity tokens, canonical arenas (stdlib data structures), checked int64 add/mul |
| `runtime/rt_heap.S` | Bump allocator (free is a no-op; exhaustion is a hard stop) |
| `runtime/rt_board.S` | GPIO output/input/read/write, `delay_us`, cycle counter |
| `runtime/silica_esp32s3.ld` | Linker script: code from IRAM 0x40378000, data after it on the D-bus, heap, 128 KB machine stack below 0x3FCE9700 (the 64 KB auxiliary stack is in `.bss`) |
| `tools/build_image.sh` | Assemble sources + runtime, link, `esptool elf2image` → `.elf`, `.map`, `.bin` (`-r <dir>` caches the assembled runtime) |
| `tools/run_on_board.py` | Load `.bin` into RAM and run it (or `--flash` it at 0x0), capture UART0 between markers, print `.sout`-style text; `--probe` checks the board |
| `tools/host_reference.sh` | Writes an app's `expected.sout` by building and running it with the macOS compiler |
| `apps/asm_*` | Bring-up apps in hand-written Xtensa assembly (runtime and board checks, no compiler involved) |
| `apps/silica_*` | Early Silica apps for the port's emitter, each with the `expected.sout` from the macOS compiler |
| `tests/xt_helpers_selftest/` | 408-case hardware self-test of the emitter's Xtensa instruction helpers (`shared/xt_*.silica`) |

## Image model

The image is built to be flashed at offset **0x0**, where the second-stage bootloader normally lives,
or loaded straight into RAM through the ROM's download mode (what `run_on_board.py` and the trials
do by default: nothing is written to flash). Either way the ESP32-S3 ROM loads its RAM segments and
calls `_start`; there is no ESP-IDF bootloader, no FreeRTOS and no
flash cache mapping, so the whole program (code + data + heap + stack) must fit in about 390 KB of
internal SRAM. Larger programs need a flash-XIP layout (future work).

Rules the emitter must follow (they come from the Xtensa windowed ABI and this memory map):

- Every function starts with `entry a1, N` and must be 4-byte aligned (`.align 4` before each label;
  the assembler rejects an unaligned `entry`).
- Nothing is ever stored below SP: the 16 bytes under SP receive the caller's `a0`–`a3` on a window
  overflow. The top 16 bytes of a frame are the base save area and, in a function that makes CALL8
  calls, the next 16 receive its own `a4`–`a7`. Locals live below `sp + N - 32`.
- Outgoing stack arguments go at `[sp + 0 ...]`; a callee reads them at `[sp + N + 0 ...]`.
- All data (strings, tables, `.rodata`) goes in data-bus sections. Only code and `L32R` literal pools
  go on the instruction bus, which faults on byte and halfword loads.
- Symbols are undecorated (`main`, `silica_rt_alloc`), ELF sections (`.text`, `.rodata`, `.data`, `.bss`).

## Console protocol

```
\x02SILICA:START\x03<program output>\x02SILICA:EXIT:<status>\x03[ fault: exccause=.. epc=.. excvaddr=..]
```

`status` is the low byte of `main`'s result, like a host exit code. `abort` reports 134, a CPU
exception 139 (what SIGABRT / SIGSEGV give on the host). `run_on_board.py` turns this into
`<program output><status>\n`, the same text the trial harness writes to a `.sout`.

## Usage

```sh
B=src_selfhost/emitter/ESP32-S3_raw/board
PY=~/.espressif/python_env/idf6.2_py3.14_env/bin/python
$B/tools/build_image.sh -o /tmp/esp/asm_00_hello $B/apps/asm_00_hello/main.S
$PY $B/tools/run_on_board.py /tmp/esp/asm_00_hello.bin          # from RAM (default)
$PY $B/tools/run_on_board.py --flash /tmp/esp/asm_00_hello.bin  # write flash, then run
$PY $B/tools/run_on_board.py --probe                            # port, chip and MAC of the board
```

A Silica app is compiled with `binaries/silica-compiler-ESP32-S3_raw` (built and published by
`make TARGET=ESP32-S3_raw` in `src_selfhost`), then its `.sams` files go to `build_image.sh`.
RAM loading uploads at 921600 baud and switches the chip back to 230400 before starting the program:
at 921600 the console of images above about 40 KB arrives corrupted (cause not established).

Required software (the Xtensa toolchain, esptool and pyserial from ESP-IDF 6.2): see
[docs/required-software.md](../../../../../../docs/required-software.md). Override the tool paths with
`XTENSA_BIN` / `ESPTOOL`.

The trial tree can run on the board too: `make integrate TRIAL_TARGET=ESP32-S3_raw` in `trials/`
(see [trials/targets/README.md](../../../../../../trials/targets/README.md)).

## Not supported on this target

Actors (the runtime is not written yet), foreign C calls (no C runtime on bare metal) and file io
(no filesystem) make a module fail to assemble with one `.error` line naming the feature, e.g.
`.error "ESP32-S3: actors are not supported yet (spawn -> X0)"`, instead of emitting AArch64 text.

## Stack budget

The machine stack is 128 KB and the auxiliary stack (the emitter's `SP`: pushes and argument slabs)
64 KB; both end in a 64-byte guard watched by an Xtensa data breakpoint, so an overflow reports
`exccause=1006` with status 139 instead of corrupting memory. A plain recursive function costs one
windowed frame (48-64 bytes) per level, so about 2000 levels fit; a level that also pushes to the
auxiliary stack (a binary operator around the call, or an argument tuple such as `write_buf`'s) uses
16-32 bytes more there. The host's 8 MB stack has no such limit, so apps keep their recursion depth
within this budget (`silica_11_regions` fills its 16 KB buffer at a stride of 20).

## Early test apps

These are bring-up checks for this port, not trials; how the trial suites will run against this
target has not been decided.

| App | Checks | Result on the test unit |
| --- | --- | --- |
| `asm_00_hello` | ROM load, `_start`, UART0, exit status | ✓ `hello from silica on esp32s3`, status 42 |
| `asm_01_blink` | GPIO output, delay | ✓ LED (GPIO6) blinks 10x -- faint, it runs at ~0.3 mA |
| `asm_02_rgb` | RGB LED red GPIO2 / green GPIO4 / blue GPIO5 | ✓ red, green, blue, white, off |
| `asm_03_buttons` | Inputs; prints raw levels for 30 s | ✓ all six directional/A/B buttons, active high |
| `asm_04_buzzer` | Buzzer (GPIO48) at 440 / 880 / 2730 Hz, CCOUNT per tone | ✓ once the RV1 trimmer is turned up (counter-clockwise); CPU at ~20 MHz |
| `asm_05_recursion` | Window overflow/underflow, int64 add-with-carry | ✓ `sum 500500`, `sum64 hi 999 lo 4294966296`, `windows ok` |
| `asm_06_heap` | Allocator alignment/zeroing | ✓ `heap used 48000`, `list sum 1999000` |
| `asm_07_fault` | Fault vector -> status 139 + details | ✓ status 139, `exccause=28` |
| `asm_08_gpio_diag` | Register read-back of the GPIO setup | diagnostic |
| `asm_09_pin_readback` | Each output pin's real pad level at 1 and 0 | ✓ all listed pins toggle |
| `asm_10_pin_hunt` | Square wave on every pin in `pins.inc` | diagnostic (used to rule out pin-map errors) |
| `asm_11_addressable_leds` | 24 x WS2813B on GPIO7, bit-banged | ✓ red, green, blue, running white dot |
| `silica_00_return42` | Prologue/epilogue, int64 constant, exit status | ✓ 42 |
| `silica_01_arith` | let bindings, int64 mul/div/rem (libgcc `__divdi3`) | ✓ 100 |
| `silica_02_calls` | User calls, arguments in `a10`–`a15`, results | ✓ 85 |
| `silica_03_case_bool` | case on booleans, compare-and-branch | ✓ 215 |
| `silica_04_print` | `print_int64`, `print_string`, `println` through `silica_rt_*` | ✓ matches the host |
| `silica_05_int64_wide` | Values above 2^32: carry, 64-bit compare, div/rem | ✓ matches the host |
| `silica_06_strings` | `concat`, `length_bytes`, `substring` (`rt_string.S`) | ✓ matches the host |
| `silica_07_recursion_depth` | 2000 nested frames: window spills, aux-stack pushes | ✓ 2001000 (needed the 128 KB + 64 KB stacks) |
| `silica_08_stack_guard` | Unbounded recursion hits the stack-guard watchpoint | ✓ status 139, `exccause=1006 ... (stack guard` |
| `silica_09_lists` | Literal/empty lists, length, prepend, patterns, remove_head, list_at, a 100-element fold (`rt_list.S`) | ✓ matches the host |
| `silica_10_records_tuples` | Records (fields, parameters, results, nesting), tuple decomposition, a tuple literal in a record read back out, a list of records | ✓ matches the host |
| `silica_11_regions` | alloc_region, scalar/tuple refs, int64 buffers, a 16 KB buffer that grows the region (`rt_heap.S`) | ✓ matches the host |
| `silica_12_floats` | float64 (soft float), float32 (FPU) and float16 arithmetic, compares, cases yielding floats, printing (`rt_float.S`) | ✓ matches the host |
| `silica_13_checked` | checked_int64_add / _mul / _add1 with overflow (`rt_ordering.S`) | ✓ matches the host |
| `silica_14_wbt_map` | The stdlib `wbt_map` (weight-balanced tree) compiled for the board with the app: insert, get, size, contains_key | ✓ matches the host |

Every `silica_*` app is compiled by the ESP32-S3_raw emitter and compared byte for byte with the
`expected.sout` the macOS compiler produced for the same source (`tools/host_reference.sh <app dir>`
writes that file: it compiles, links and runs the app on the Mac).
An app that needs other modules (the stdlib) lists them in its `extra_sources.txt`, one path per
line relative to `silica-compiler/`, in dependency order; both the host script and the board build
compile those files first, in one `silica.config` with the app.

`silica_04_print` binds every `print_int64` result: a bare `print_int64(<literal>)` statement is an
open host-compiler defect (`trials/sequence_block_addition/sequence_print_int64_literal_statement`).
The port reproduces that host behaviour exactly; the app avoids it so the rest of the app still
tests. Three host defects the apps found are fixed in both emitters (2026-09-13) and covered by
trials, and the apps exercise them again: a tuple literal as a record field value read back out
(`silica_10_records_tuples`; `trials/tuples_addition/decompose_record_tuple_field`,
`record_tuple_field_*`), a `case` whose arms yield a float not assembling (`MOV D0, #0`;
`trials/float64_addition/case_float_result`), and a `case` on a comparison of two float parameters
returning the wrong one, the compare's operand staging having overwritten the second parameter's
register (`silica_12_floats`; `trials/float64_addition/case_float_compare_params` and its float32 /
float16 twins).
