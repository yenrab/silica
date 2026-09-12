# `ESP32-S32_raw` port — native Xtensa LX7 code generation, bare metal

**Status:** In progress (started 2026-09-12). Board pack (runtime, link script, image and flash tools,
bring-up apps) exists under `src_selfhost/emitter/ESP32-S32_raw/board/`; the emitter conversion is
being done file by file. This is a design record for one port, not a specification; where it
disagrees with [silica-specification.md](../silica-specification.md), the specification wins.

**Decisions taken by Lee (2026-09-12):**

| Question | Decision |
| --- | --- |
| How is Xtensa produced | **Native code generation**: every instruction-emitting site emits Xtensa. Not a translation pass over AArch64 text. |
| Runtime | **Bare metal**: no ESP-IDF, no FreeRTOS. Image at flash 0x0, loaded into SRAM by the ROM. |
| Testing | **On the port test board** ([PORT_TEST_BOARD_byu_idaho_v4.0_feb26.md](../../src_selfhost/emitter/ESP32-S32_raw/board/PORT_TEST_BOARD_byu_idaho_v4.0_feb26.md)), with early test apps. How trials run against this target is **not decided**; nothing here touches `trials/`. |

## Related documents

| Document | Role here |
| --- | --- |
| [porting_for_os_free_targets.md](../porting_for_os_free_targets.md) | What a complete OS-free port needs; this port covers the compiler target and a first runtime |
| [linux_aarch64_port_checklist.md](linux_aarch64_port_checklist.md) | The sibling hosted port; same "Darwin-isms" inventory, different ISA problem |
| [memory-effects-aarch64-implementation-plan.md](../memory-effects-aarch64-implementation-plan.md) | `Space` on ESP32-S3 (pools, not MAIR) — later phases |
| `emitter/ESP32-S32_raw/board/README.md` | Runtime, memory map, console protocol, tools, bring-up apps |

---

## 1. The problem

The `apple_silicon_mac` emitter this tree was copied from is AArch64 through and through: register
choice (31 × 64-bit GPRs, X19–X28 callee-saved), a dozen special-case parameter shadows into those
registers, flag-setting compares, SP pushes, `[X29, #-N]` frame slots, Darwin syscalls, and ~10k lines
of hand-written runtime assembly embedded as strings (actors on pthreads, print via `SVC`, strings via
`mmap`). ESP32-S3 is Xtensa LX7: **16 visible 32-bit registers in a sliding window**, no flags, no
64-bit arithmetic, no double-precision FPU, and no OS.

The emitter's *logic* — SIR walking, type classification, aggregate promotion, region ownership,
binding bookkeeping in `outer_reg` — is ISA-independent and was debugged over dozens of defects. The
port keeps that logic and replaces the machine layer under it.

## 2. Virtual registers

The emitter keeps naming values the way it always has — `X0`–`X30`, `W<n>`, `D/S/H<n>`, `SP`, and
frame slots `[X29, #-N]` — but on this target those are **virtual registers (VRs)**, names in the
emitter's IR, not machine registers. Every instruction site asks the shared helper module
(`shared/xt_isa.silica`) for Xtensa code that operates on VRs; the helpers map each VR to a home:

| VR | Home | Why |
| --- | --- | --- |
| `X0` / `W0` | `a10:a11` (lo:hi) | CALL8 argument 0 and result position in the caller's window: calls and returns need no moves |
| `X1`, `X2` | `a12:a13`, `a14:a15` | CALL8 arguments 1 and 2 |
| `X9`, `X10` | `a4:a5`, `a6:a7` | the binary-prim operand temps; survive CALL8 (harmless superset of AArch64 caller-saved) |
| every other `X`/`W` VR | 8-byte frame slot `SVR_x<n>` | callee-saved semantics come free: each frame has its own copy |
| `X3`–`X7` | frame slots at `[sp + 0 .. 40)` | also the outgoing argument area, see §4 |
| `D0`–`D7` (`S`/`H` alias the low bits) | frame slots at `[sp + 40 .. 104)` | float arguments, see §4 |
| other `D`/`S`/`H` | frame slots `SVR_d<n>` | |
| `[X29, #-N]` | `[sp + SFP - N]` | the emitter's frame-slot area, sized exactly as before |
| `SP` pushes / slabs | auxiliary data stack (global `silica_rt_vsp`) | the machine SP may not move inside a windowed frame (§3) |

Scratch registers for expansions: `a2`, `a3`, `a8`, `a9`. Incoming arguments arrive in `a2`–`a7`; the
prologue moves them to `a10`–`a15` before anything else runs, which frees `a2`–`a7`.

**Values.** Every `X`-class value is a 64-bit pair: `int64`/`uint64` naturally, and pointers, atoms,
booleans with a zero high word. This keeps every emitter decision that says "X" valid and keeps all
memory layouts identical to `apple_silicon_mac` (8-byte fields, 24-byte list cells, string descriptor
`{tag, data@+8, length@+16}`). `W`-class values use the low register of the pair. `float32` uses the
FPU with the value homed in memory (`lsi`/`ssi` straight to and from the home). `float64` is soft
float through libgcc (`__adddf3`, …); `float16` converts through `float32`.

**Flags.** Xtensa has none. Compare-and-branch, compare-and-set and compare-and-select are emitted as
single combined helpers at the site that knows both halves (`xt_cmp_branch`, `xt_cmp_set`,
`xt_cmp_select`), using `blt/bltu/beq…` on the high words first and the low words on a tie.

## 3. Frames

Each function's layout is computed **after** its body is emitted and published as assembler symbols
at the top of the function, before the label:

```
    .set SFRAME, <total>          ; entry immediate, 16-byte aligned
    .set SFP, <x29 base>          ; [X29, #-N] == [sp + SFP - N]
    .set SVR_x19, <off> ...       ; one line per VR home the body used
    .align 4
name:
    entry   a1, SFRAME
```

GAS resolves those symbols in instruction operands and in `.if` conditions (verified with the
toolchain), so a load from a home is `l32i a8, a1, SVR_x19` when in range and a `movi`/`add` form
otherwise, chosen by the assembler from the final value. Only VR homes the body actually references are
allocated, so frames stay small for the deep recursion Silica code does.

```
 sp + SFRAME       ── caller's SP
 [SFRAME-16, SFRAME)   base save area (window overflow writes the caller's a0-a3 here)
 [SFRAME-32, SFRAME-16) this function's a4-a7 on overflow (it makes CALL8 calls)
 [SFP - area, SFP)     emitter frame slots [X29, #-N]
 [..]                  VR homes (only those used)
 [40, 104)             D0-D7 homes = outgoing float arguments (when used)
 [0, 40)               X3-X7 homes = outgoing integer arguments (when used)
 sp
```

Rules inherited from the windowed ABI (see `board/runtime/rt_vectors.S`): nothing is stored below
`sp`; the machine `sp` never moves after `entry` (so AArch64-style pushes go to the auxiliary data
stack); every function is `.align 4` (the assembler rejects an unaligned `entry`).

## 4. Calls

**Silica calling convention on Xtensa** (used for Silica functions and every runtime entry point):
windowed `CALL8`; argument *n* occupies a 64-bit pair. `X0`–`X2` travel in `a10`–`a15` (the callee's
`a2`–`a7`), which is exactly the standard ABI for three `int64` arguments. `X3`–`X7` and `D0`–`D7`
travel in the caller's outgoing area at fixed offsets; the callee copies the ones it takes as
parameters from `[sp + SFRAME + off]` into its own homes. Results: integer in `a2:a3` (caller sees
`a10:a11` = `X0`); a float result is also stored into the caller's `D0` home at `[sp + SFRAME + 40]`.

Runtime functions written in assembly follow the same convention (every argument a pair). C and libgcc
functions are called with the standard windowed ABI; libgcc's `int64`/`double` signatures coincide with
it. Because `CALL8` clobbers `a8`–`a15`, an expansion that calls libgcc in the middle of an expression
(64-bit divide, float64 arithmetic) saves and restores `X0`–`X2` around the call.

**Tail calls.** The windowed ABI has no tail call: `entry` needs the window rotation of a real `CALLn`.
A self tail call becomes a jump back to just after the prologue with the arguments reloaded, which is a
true loop. Other tail calls become `call8` + return and use stack; deep mutual tail recursion is
bounded by the stack (§6).

## 5. Runtime

Bare metal, in `board/runtime/*.S` (the IO-in-`.s` rule): reset, vectors, UART0 console, exit/fault
reporting, heap, GPIO/delay. Everything the Apple emitter inlined as Darwin syscalls or libSystem calls
becomes a call to a `silica_rt_*` entry point there. The actor runtime (pthreads, `os_unfair_lock`,
`__ulock`) has no counterpart yet; it becomes a cooperative single-core scheduler in a later phase.

## 6. Limits of the first version

- Everything runs from internal SRAM (~390 KB for code, data, heap and stack). No flash XIP, no PSRAM.
- Stack 64 KB, heap = the rest; `free` is a no-op (bump allocator).
- Single core; interrupts off; no actors yet; no FFI (the guarded-FFI runtime is setjmp/signal based).
- Non-self tail calls use stack.

## 7. Order of work

Driven by the early apps in `board/apps/silica_*` (each has the macOS reference output):

| Step | App | Emitter pieces |
| --- | --- | --- |
| 1 | `silica_00_return42` | module prelude, sections, function frame/prologue/epilogue, int64 constants, atom tables |
| 2 | `silica_01_arith`, `silica_02_calls` | lets, int64 arithmetic, calls, parameters, recursion, case on booleans |
| 3 | `silica_03_case_bool` | comparisons, and/or short circuit, nested case |
| 4 | `silica_04_print`, `silica_05_int64_wide` | print runtime, 64-bit multiply/divide |
| 5 | `silica_06_strings` | string runtime (concat, length, substring) |
| 6 | `silica_07_recursion_depth` | frame size under deep recursion |
| 7+ | new apps | narrow ints, uint64, tuples, records, lists, regions, floats, actors |
