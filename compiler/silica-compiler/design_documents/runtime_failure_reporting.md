# Runtime failure reporting: one report, every target

Status: implemented 2026-09-19 in all four emitter trees (`apple_silicon_mac`, `linux_aarch64`,
`linux_x86_64`, `ESP32-S3_raw`) and the ESP32-S3 board runtime. Verified on the Mac; the Linux and x86-64
runtime text is assembled but not yet run on its hosts; the board runtime is assembled and linked into
images but not yet run on the board.

The contract is normative in the specification, **§15.4.5.5 Process-Fatal Reports and Exit Status**
([silica-specification.md](silica-specification.md)). This document is non-normative: it describes how each
target meets that contract, so that a program that ends abnormally reports it the same way whether it
runs on macOS, Linux (AArch64 or x86-64) or bare metal on the ESP32-S3.

## 1. The contract

| Class | Report (one line) | Exit status |
| --- | --- | --- |
| Fatal fault: a synchronous CPU exception the program cannot survive (§15.4.4 gate fails, a fault outside any actor, including exhausting `main`'s stack, a fault in trusted runtime code) | `[silica] fault at 0x<pc>` + ` in <symbol>+0x<offset>` where available + `  addr=0x<fault address>` + `  actor=0x..  sbase=0x..  ssize=0x..` on targets with actors | **70** |
| Runtime abort: a runtime check the program cannot continue from, a resource the runtime cannot obtain (memory to commit for a growing actor stack, §15.4.10.2), a containment failure not raised by hardware | `[silica] abort: <reason> at 0x<pc>` + ` in <symbol>+0x<offset>` where available | **71** |
| Language-level failure in `main` (`case_clause`, `badarith`) | the atom name | 1 |
| Actor failure (a fault or stack exhaustion inside an actor that the runtime recovers, every exit reason of §15.4.11) | none: delivered through supervision (§15.4.4, §15.4.10) | not a process exit |

What the rules mean in practice:

- **Nothing ends a Silica process through C `abort()` (SIGABRT, status 134) or a default signal action
  (status 139).** Every call to `abort()` the runtime and the emitted code used to make is now a runtime
  abort (§4). Where the specification says the process "aborts" when containment fails, the process ends
  with a fatal fault report (status 70) when the hardware raised the failure, and with a runtime abort
  (status 71) when the runtime did (the host refusing to commit memory for a growing stack).
- **Hosted targets write the report on standard error. The OS-free target writes it on the console as
  program output**, just before the console's exit marker, which carries the status. The trial harness
  captures a hosted program's stdout and stderr in one `.sout`, so the two forms compare equal.
- **Fields a target cannot supply are left out, not made up.** See §2.
- **Every `0x` value is opaque.** Tools that compare reports fold them (§5).

## 2. What each target prints

| | `apple_silicon_mac` | `linux_aarch64` | `linux_x86_64` | `ESP32-S3_raw` (board) |
| --- | --- | --- | --- | --- |
| Where | stderr | stderr | stderr | UART0 as program output, then `\x02SILICA:EXIT:<status>\x03` |
| `0x<pc>` width | 16 digits | 16 | 16 | 8 |
| ` in <symbol>+0x<offset>` | yes: `dladdr` names the nearest preceding exported symbol | only when the address is exactly a symbol's start: the ELF symbols the emitters write have no `.size`, and glibc's `dladdr` matches a size-0 symbol only at its address | as `linux_aarch64` | never: no symbol table on the board |
| `  addr=0x..` (fault) | `si_addr` | `si_addr` | `si_addr` | `EXCVADDR`, only for the exception causes that set it (2, 3, 9, 12-18, 20, 24-26, 28, 29); a stack-guard hit (a data breakpoint, cause 1006) has none |
| `  actor=  sbase=  ssize=` (fault) | yes (zero outside an actor) | yes | yes | never: no actor runtime yet |
| After the report | exit | exit | exit | exit marker; for a fault, the board-only details after it: ` fault: exccause=<n> epc=0x.. excvaddr=0x..` and ` (stack guard: ...)` for cause 1006, which `run_on_board.py` sends to stderr |

Examples. A stack overflow in `main` (trial `actor_stacks_addition/stack_overflow_in_main`):

```
[silica] fault at 0x0000000102d41690 in m_down+0x0000000000000024  addr=0x000000014d0c3ff0  actor=0x0000000000000000  sbase=0x0000000000000000  ssize=0x0000000000000000
70
```

on the Mac, and on the board (app `silica_08_stack_guard`):

```
[silica] fault at 0x4037a1b4
\x02SILICA:EXIT:70\x03 fault: exccause=1006 epc=0x4037a1b4 excvaddr=0x00000000 (stack guard: machine or auxiliary stack overflow)
```

A list index past the end (trial `list_addition/list_at_out_of_range_aborts`, app `silica_15_list_index_abort`):

```
[silica] abort: list index out of range at 0x0000000104b2dc80 in main+0x0000000000000548
71
```

The symbol is the nearest exported one: `L_list_at_helper` is a local label emitted after `main`, so an
address inside it reads `main+0x548`. The pc of an abort is the address of the call into the abort
routine, never its return address, so a call that is the last instruction of a function is not
attributed to the function after it.

## 3. How the sameness is achieved

### 3.1 Hosted targets (macOS, Linux AArch64, Linux x86-64)

**Signals.** `silica_rt_install_sync_fault_handlers` (`emitter/<target>/terms/ffi_fault_runtime_asm.silica`)
installs `silica_rt_sync_fault_handler` with `sigaction` for SIGSEGV, SIGBUS, SIGILL and SIGFPE
(`SA_SIGINFO | SA_ONSTACK`) and gives the installing thread a 64 KB `sigaltstack`. Each actor thread gets
its own 64 KB alternate stack (ACB +464). The install runs in `silica_rt_actor_spawn`, and every
program's `main` spawns the pid registry actors in its prologue (the emitter injects
`_silica_pid_registry_init`), so the handlers exist before any user code runs, including in a program that
never spawns an actor itself. A stack overflow in `main` therefore faults on the guard page below the
main thread's stack and the handler runs on the alternate stack.

**The decision** (spec §15.4.5.3), in `silica_rt_sync_fault_handler`:

1. a fault inside the current actor's reservation is stack growth (`silica_rt_stack_grow_to`): grown,
   return; the end of a reservation the program lowered, the actor fails with
   `(:explicit, :stack_exhausted)` (supervision, not a process report); **the host refused to commit
   memory: runtime abort** `cannot commit memory for a growing actor stack`, status 71, reported at the
   faulting pc read from the `ucontext` (§15.4.10.2);
2. only now are the report fields recorded: the pc (`ucontext`), the fault address (`si_addr`), the current
   actor and its stack reservation (zero outside an actor). They are process-wide, and until 2026-09-19
   they were recorded on entry, so an actor taking growth faults on another thread could overwrite them
   while `main` was reporting its own fault (seen as a stack overflow in `main` reported with an actor's
   pc, address and stack);
3. a fault inside a guarded FFI call is recovered (`siglongjmp`) and becomes an actor failure;
4. anything else is a **fatal fault**: `silica_rt_print_fault_site` writes the report line from the
   recorded fields (`dladdr` for the symbol), and the process calls `exit(70)`.

**Runtime aborts.** One routine per target, in the same module as the fault report so that it shares the
output helpers (`silica_rt_fault_puts`, `silica_rt_fault_puthex`, the `dladdr` buffer):

- `silica_rt_abort_with(reason)` (Darwin `_silica_rt_abort_with`): the reason is a NUL-terminated string;
  the reported pc is the call instruction (AArch64 `X30 - 4`, x86-64 the return address `- 5`).
- `silica_rt_abort_with_pc(reason, pc)`: the same with an explicit address (used by the fault handler for
  the stack-commit abort).
- One entry per site, `silica_rt_abort_<site>`, which loads its reason and branches to
  `silica_rt_abort_with` with the return address untouched. Emitted code and hand-written runtime call
  these, so an abort site is one call instruction and the reasons live in one table per target.

`silica_rt_abort_with` is safe wherever the old `abort()` calls were: it allocates nothing and does not
run on the caller's stack. The first caller claims a word (`LDAXR`/`STLXR`, `xchg`) and moves onto a
static 32 KB stack, because the caller may be an actor at the very end of its reservation or the fault
handler on its alternate stack; a second caller on any thread parks (`yield` / `pause` loop) until the
process has gone. It writes `[silica] abort: <reason> at 0x<pc>`, then ` in <symbol>+0x<offset>` when
`dladdr` names the address, then a newline, with `write(2, ...)`, and ends with C `_exit(71)`
(Darwin `__exit`): async-signal-safe, no atexit handlers and no stdio flush, as the `abort()` it replaces.
Silica's own output never sits in a stdio buffer (the print helpers make the `write` system call).
The fault report keeps `exit(70)` as before.

### 3.2 The ESP32-S3 board (OS-free)

There are no signals. The runtime (`emitter/ESP32-S3_raw/board/runtime/`) installs its own vectors
(`rt_vectors.S`, VECBASE set by `rt_start.S`) and, for the two fixed stacks, data breakpoints over the
64-byte guard at the bottom of each (`DBREAKA0/1`, armed in `_start`): the 128 KB machine stack and the
64 KB auxiliary stack the emitter uses as `SP`. This is the "exception vectors + stack-guard data
breakpoints" form spec §15.4.5.1 names for OS-free targets.

**Fatal fault.** Every vector except the window overflow/underflow and alloca handlers goes to
`silica_rt_fatal_trampoline` with a cause code (EXCCAUSE for user exceptions, 1001-1008 for the kernel,
level-2..5, debug, NMI and double-exception vectors; the debug vector copies EPC6 to EPC1, so a stack-guard
hit reports the store that reached the guard). The trampoline abandons the faulting context (interrupts
off, only the current register window live, PS reset, the 2 KB fault stack) and calls
`silica_rt_fatal_report` (`rt_console.S`), which prints `[silica] fault at 0x<epc>`, adds
`  addr=0x<excvaddr>` for the causes that set EXCVADDR, ends the line, writes the exit marker with status
70, then the board-only details, and halts.

**Runtime abort.** The entries `silica_rt_abort_with(reason)` and `silica_rt_abort_<site>()` (`rt_console.S`)
are windowed functions reached with CALL8. Each names its reason in `a2` and jumps to
`silica_rt_abort_body`, which turns the return address in `a0` into the address of the CALL8 (bits 31:30
of `a0` hold the window increment, so the real top bits are taken from a code address, then 3 is
subtracted) and calls `silica_rt_abort_trampoline` (`rt_vectors.S`). That resets the context exactly as
the fatal trampoline does and calls `silica_rt_abort_report` on the fault stack, which prints
`[silica] abort: <reason> at 0x<pc>` and ends with `silica_rt_exit(71)`. Nothing is allocated, so the
heap-exhausted abort is safe. `silica_rt_abort()` without a reason remains, as `runtime check failed`,
for code compiled before the per-site entries existed.

**Console protocol.** `rt_start.S` prints `\x02SILICA:START\x03` before `main`; `silica_rt_exit` prints
`\x02SILICA:EXIT:<status>\x03` and halts. `board/tools/run_on_board.py` prints the program's bytes followed
by the status on its own line, which is exactly the `.sout` form, and sends anything after the exit marker
to stderr. The report line is before the marker, so it is part of the `.sout`, as the host's stderr line
is.

## 4. The sites

Emitted code (host emitters `terms/prims/prims_memory.silica`, `terms/prims/prims_list.silica`; the ESP32
emitter's `prims_memory.silica`):

| Site | Entry | Reason |
| --- | --- | --- |
| `L_region_overflow` (the region allocator returned nothing) | `silica_rt_abort_region_overflow` | `region allocation failed` |
| `L_buf_invalid` (`alloc_buf` count negative, or its byte size overflows) | `silica_rt_abort_buf_invalid` | `invalid buffer size` |
| `L_buf_bounds` (buffer store index at or past the length) | `silica_rt_abort_buf_bounds` | `buffer index out of bounds` |
| `L_lat_oob` in `L_list_at_helper` (host) | `silica_rt_abort_list_index` | `list index out of range` |

Hosted runtime (`emitter/<target>/terms/`):

| Site | Entry | Reason |
| --- | --- | --- |
| `canonical_arena_runtime_asm`: key 0 / 512-entry table full / `malloc` failed | `silica_rt_abort_arena_key` / `_arena_full` / `_arena_nomem` | `canonical arena key is zero` / `canonical arena table is full` / `canonical arena allocation failed` |
| `ffi_guarded_runtime_asm`: `L_gffi_enter_abort` (per-thread guarded state not allocated) | `silica_rt_abort_guarded_ffi_state` | `guarded FFI state could not be allocated` |
| `prims_actors_runtime_asm` (x86-64: `_b`): `Lsie_abort`, supervision ingress at its 1024 bound | `silica_rt_abort_ingress_full` | `supervision ingress queue is full` |
| `prims_actors_runtime_asm` (x86-64: `_c`): `Lde_abort`, exit delivery into a supervisor's ingress at its bound | `silica_rt_abort_exit_queue_full` | `supervisor ingress queue is full (exit delivery)` |
| `ffi_fault_runtime_asm`: `L_sfh_unrecoverable` (host refused to commit a growing stack) | `silica_rt_abort_with_pc` at the faulting pc | `cannot commit memory for a growing actor stack` |

Board runtime (`board/runtime/`):

| Site | Entry | Reason |
| --- | --- | --- |
| `rt_list.S` `silica_rt_list_at`, index out of range | `silica_rt_abort_list_index` | `list index out of range` |
| `rt_ordering.S` `silica_rt_canonical_arena_lookup`, key 0 / table full | `silica_rt_abort_arena_key` / `_arena_full` | as on the host |
| `rt_string.S` `silica_rt_string_bad`, `rt_console.S` `silica_rt_print_string`: the tag word is not -1 | `silica_rt_abort_not_a_string` | `not a Silica string` |
| `rt_heap.S` `silica_rt_alloc`, heap exhausted (was: status 134 and a message after the marker) | `silica_rt_abort_heap_exhausted` | `heap exhausted` |
| emitted `L_region_overflow` / `L_buf_invalid` / `L_buf_bounds` | `silica_rt_abort_region_overflow` / `_buf_invalid` / `_buf_bounds` | as on the host |

The ESP32-S3 emitter tree still carries the AArch64 runtime chunks it was copied from
(`terms/ffi_fault_runtime_asm.silica`, `canonical_arena_runtime_asm`, `ffi_guarded_runtime_asm`,
`prims_actors_runtime_asm`). They are not emitted for this target (`emit_actor_runtime_module` writes a
marker only) and are edited in step with the Mac tree.

Reachable from Silica source today, and covered by a trial: the list index, the invalid buffer size, the
zero canonical-arena key, and the fatal fault of a stack overflow in `main`. Not reachable in practice: the
allocator failures (region, arena, guarded FFI state), a full supervision ingress (1024 pending entries),
and the stack-commit refusal. **The buffer bounds check is never emitted** (an open defect): the store only
checks when its SIR node carries a `|bounds:` marker, which no trial produces, and loads have no check;
`memory_region_addition/buf_write_out_of_bounds_aborts` pins the required behaviour and fails until then.

## 5. Comparing reports

Because the addresses change from run to run, and the fields after the pc change from target to target,
every tool that compares a report folds it first with `trials/normalize_fatal_reports.awk`:

```
[silica] fault at 0x<pc>...           ->  [silica] fault at <PTR>
[silica] abort: <reason> at 0x<pc>... ->  [silica] abort: <reason> at <PTR>
```

A fault line keeps only its class, an abort line its class and reason; every other line is compared as
before (`diff -Bw`). The match is not anchored, so a report that follows program output without a newline
is folded too. Goldens are written by hand in the folded form, which the rule leaves unchanged.

Where it is applied:

- **Trial suites** that contain such trials use the per-suite hook `compare_scout_normalized.sh` (the
  convention `supervisors_addition` and `ffi_addition` already had): `actor_stacks_addition`,
  `list_addition`, `memory_region_addition`. Their Makefiles compare every trial's `.sout` with its
  `.scout` through it.
- **Board trial runs** (`trials/targets/board_suite.sh`) use the suite's `compare_scout_normalized.sh` when
  it has one, so the same `.scout` holds on the host and on the board.
- **Board apps**: `board/tools/host_reference.sh` writes `expected.sout` in the folded form, and
  `board/tools/compare_sout.sh <run.sout> <app>/expected.sout` compares a board run with it.
- **The x86-64 ladder** (`emitter/linux_x86_64/ladder/README.md`) folds both sides before its `diff`.

## 6. Limits

- **Decided 2026-09-19, not yet implemented** (spec §15.4.5.5 as updated): the fault handlers are to be
  installed by the startup code before `main` runs, and a fault raised inside the handler is to take a
  last-resort path that writes the fault line and exits 70. Today both cases still end with the default action
  (status 139): the signal is blocked while the handler runs, and a fault before `main`'s prologue finds no
  handler installed.
- **Decided 2026-09-19, not yet implemented:** process-fatal paths end with the async-signal-safe `_exit`. The
  runtime-abort routine already does. The fault handler still calls C `exit()`: in Darwin assembly `bl _exit` is
  C's `exit()`, and `bl __exit` is C's `_exit()` (Linux spells them `exit` and `_exit`).
- Two reports at once (two threads failing together) can interleave their lines; only the first abort
  reports, but a fault on another thread does not take the abort's claim word.
- On Linux the symbol is missing from most reports until the emitters write `.size` for their functions.
- The board has one core and no actor runtime, so there is no actor failure on the board yet: a fault is
  always fatal there.
- **Decided 2026-09-19, not yet implemented:** a spawn whose stack reservation fails while `main` is the spawner
  becomes a runtime abort (`stack reservation failed`, 71; spec §15.1.2.2), and a guarded FFI fault with no
  current actor becomes a fatal fault (70). Today they print `stack_reserve_failed` or `foreign_fault` and exit 1
  (`prims_actors_stack_asm`, trial `actor_stacks_addition/stack_reserve_failed_in_main`;
  `ffi_fault_runtime_asm` `L_gff_process`).
- Process-fatal paths write only the report line; unwind reports belong to actor failures (spec §15.4.5.5).

## 7. Where the code is

| Target | Fault report | Runtime abort |
| --- | --- | --- |
| `apple_silicon_mac`, `linux_aarch64`, `linux_x86_64` | `emitter/<target>/terms/ffi_fault_runtime_asm.silica`: `silica_rt_install_sync_fault_handlers`, `silica_rt_sync_fault_handler`, `silica_rt_print_fault_site` | same file, `emit_abort_runtime_asm`: `silica_rt_abort_with`, `silica_rt_abort_with_pc`, `silica_rt_abort_<site>`; the claim word and the 32 KB report stack in its `.bss` |
| `ESP32-S3_raw` | `board/runtime/rt_vectors.S` (vectors, `silica_rt_fatal_trampoline`), `rt_console.S` (`silica_rt_fatal_report`), `rt_start.S` (VECBASE, stack-guard breakpoints) | `rt_console.S` (`silica_rt_abort_with`, `silica_rt_abort_<site>`, `silica_rt_abort_body`, `silica_rt_abort_report`), `rt_vectors.S` (`silica_rt_abort_trampoline`) |

Related: [porting_for_os_free_targets.md](porting_for_os_free_targets.md),
[porting_to_linux_x86_64_hosted.md](porting_to_linux_x86_64_hosted.md), the board pack's
[README](../src_selfhost/emitter/ESP32-S3_raw/board/README.md) (console protocol),
[ports/esp32s3_port_status.md](ports/esp32s3_port_status.md), and
[macos_crash_handling_for_silica.md](macos_crash_handling_for_silica.md) (the guarded-FFI recovery that
comes before a fatal fault).
