# Porting Silica for OS-free (raw metal) targets

**Status:** Design plan. Not implemented. Not a substitute for [silica-specification.md](silica-specification.md). Use this document when adding a **board** or **OS-free** compiler/runtime target. Hosted ports (macOS, Linux, Windows) are out of scope except where this text contrasts them.

**Audience:** someone who already knows Silica’s region/`Space`/`sequence` model and needs a single list of what a raw-metal image must provide, what the current compiler already does, and what still has to be added.

## Related documents

| Document | Role here |
| --- | --- |
| [silica-specification.md](silica-specification.md) §9.1.1, §12.1.1.0–§12.1.1.1, §15.1.1 | Normative `mem(Space)`, `register_rwr`, `spawn_device`, OS-free vs OS-hosted |
| [silica_device_actor_specification.md](silica_device_actor_specification.md) | Who may poke: device workers only; `device_actor_ref`; exceptions for boot/panic/IRQ |
| [execution-environments-hosted-vs-bare-metal.md](execution-environments-hosted-vs-bare-metal.md) | Short hosted vs OS-free overview |
| [memory-effects-aarch64-implementation-plan.md](memory-effects-aarch64-implementation-plan.md) | How each `Space` maps to AArch64 MAIR / ESP32-S3 pools |
| [direct_machine_object_emitter_future.md](direct_machine_object_emitter_future.md) | Chip encoders and ELF / firmware objects (no `.sams` assembler) |
| [IPC_Bare.md](IPC_Bare.md) | Embedded actor runtime sketch (scheduling, isolation, on-device IPC) |
| [ROADMAP.md](../../../ROADMAP.md) | Target attention order (Linux, ESP32-S3, bare-metal AArch64, …) |

---

## 1. Purpose

A **complete OS-free port** is a Silica program that boots on a named board **without a general-purpose OS**, owns its memory map, talks to **device registers** (UART, GPIO, timers, …) in typed Silica, and can run the actor runtime with board time and interrupts.

That is more than “the compiler emits AArch64.” Today the compiler targets **Apple Silicon macOS**: Darwin syscalls, `malloc` arenas, Mach-O via assembly text. The language already *names* OS-free behavior (`device`, `register_rwr`, MAIR). Most of the hardware path is missing.

This document:

1. Defines what “ported” means (board pack + compiler target + runtime).
2. Inventories **present vs missing**.
3. Specifies the **MMIO / device-register** hole (map a bus address; volatile load/store under `register_rwr`).
4. Separates **device MMIO** from **CPU system registers**.
5. Points at **device-worker actor** rules ([silica_device_actor_specification.md](silica_device_actor_specification.md)): poke is not loose in `main`.
6. Gives a phased order so MMIO prims are not mistaken for the whole port.

---

## 2. What a port is

A port is three contracts that must name the same chip and board.

| Piece | Owns |
| --- | --- |
| **Compiler target** | ISA, ABI, object/firmware format, instruction selection for `Space` and barriers |
| **Board pack** | Linker script / memory map, boot, vectors, which physical ranges are RAM vs MMIO, capability flags (`Space` exact vs emulated) |
| **Runtime** | Region allocators per `Space`, console, clock/IRQ, actor scheduling, optional Fifi |

Two AArch64 boards are two board packs even if they share `chip/arm64`. ESP32-S3 is a different chip layer (Xtensa), not a MAIR port of AArch64.

**Hosted** ports keep a kernel: they do not promise per-`Space` hardware attributes (§12.1.1.0). **OS-free** ports do, for every `Space` the board pack marks exact.

---

## 3. Two kinds of “register”

Ports fail when these are conflated.

| Kind | What it is | Who writes it today (C/bare metal) | Silica today |
| --- | --- | --- | --- |
| **CPU GPR** | X0, SP, LR | Almost always the compiler | Compiler-owned. No user `MOV X0, X1`. |
| **Device / MMIO** | UART DR, GPIO, timer at a **fixed bus address** | Humans, via headers/HAL | Effect name + barriers only. **No map. No volatile access prim.** |
| **CPU system / privileged** | `MAIR_EL1`, DAIF, TTBR, Xtensa INTENABLE | Runtime/boot, rarely apps | Spec describes MAIR for OS-free. **No Silica prim. No EL1 runtime.** |

Raw-metal *applications* need device MMIO. Raw-metal *runtimes* also need privileged CPU registers at boot. Those are different extensions.

---

## 4. Completeness inventory

“Present” means the current seed/selfhost Apple Silicon macOS compiler and runtime. “Missing” means required for a first OS-free board that can print on a UART and allocate a real `device` region.

| Area | Present | Missing |
| --- | --- | --- |
| Language names | `mem(device)`, `region(R, device)`, `device_window`, `register_rwr`, `spawn_device`, `device_actor_ref`, `DeviceDescription`, `Register8`…`Register64` in the spec | Checker does **not** yet enforce device-worker rules |
| Device actors | Spec: [silica_device_actor_specification.md](silica_device_actor_specification.md) | No `spawn_device` runtime, third registry, device descriptions, or E2201-family checks |
| `alloc_region(device)` | Type-checks; trial exists | Still a **heap arena** (`malloc`). Space operand **unused** on emit |
| `register_rwr` | Emitter can emit `DSB SY` / `ISB` around calls if the effect string contains the name; FFI taint forbids mixing with `external_danger` data | Nothing that **maps** or **touches** MMIO; effect still usable in `main` until the checker lands |
| Volatile MMIO load/store | — | `peek` / `poke` prims lowered **device-volatile** and ordered (§5.2) |
| Bind physical/bus range | — | `map_device`: device tag + base address → `device_window` (not bump-malloc) |
| `Space` → hardware | Spec + [memory-effects plan](memory-effects-aarch64-implementation-plan.md) | `MAIR_EL1`, PTEs or ESP32-S3 pools; wire `alloc_region` space tag to runtime |
| Atomic `ref` | Plain `LDR`/`STR` | `LDAR`/`STLR` (AArch64) or Xtensa equivalents |
| Boot / vectors / linker script | Darwin crt | Board reset, exception table, stack, `.text`/`.bss` placement |
| Console / `device_io` print | Hosted stdout | UART (or semihosting) under board pack |
| Time / preemption | Hosted | Timer IRQ, tick for actors |
| Object / image | `.sams` + clang → Mach-O | `aarch64-none-elf` / Xtensa ELF or firmware image; later [direct object emitter](direct_machine_object_emitter_future.md) |
| Privileged CPU regs | — | Runtime-only access at EL1 (or ESP32 equivalent); not app MMIO |
| Actors | Hosted runtime | [IPC_Bare](IPC_Bare.md) scheduling, no Darwin threads |
| Fault notes | macOS FFI crash note | Per-board fault/IRQ document |

A port is **not complete** if only the MMIO prims land, or only the object format lands.

---

## 5. Device MMIO: required language/runtime surface

This is the hole described as: *there is no prim that maps an MMIO address or does a volatile load/store to it.*

Without it, an OS-free image cannot drive peripherals **in Silica**. Fifi into C that pokes MMIO is an escape hatch, not a port.

Who may call these prims is **not** this document: [silica_device_actor_specification.md](silica_device_actor_specification.md) requires poke only inside a `spawn_device` worker (`device_*` modules). `main` and ordinary `spawn` behaviors must not declare `register_rwr`. Reset, early panic, and IRQ enqueue are the only exceptions (§7 of that spec).

### 5.1 Map a known address

`alloc_region(device)` must **not** be the way to get UART registers. Those addresses are fixed by the SoC, not carved from a heap.

Need a **bind**, not an allocate:

```text
map_device(device: D, base: uint64)
    -> device_window(R, D)
    proc[register_rwr]
```

`D` is a device tag whose programmer-supplied device description gives the register layout, and so the window's size. The prim's language rules are in [silica_device_actor_specification.md](silica_device_actor_specification.md) §4.7 and §4.9. Constraints on the port:

- `base` and the described size must be board-legal (alignment, peripheral window). Illegal maps fail at compile time in the port's emitter when the board pack can prove it, otherwise at initialization with a hard halt.
- The window is **not** bump-allocated RAM. Loads/stores go to that physical/bus range.
- Only a **device worker** behavior (and the boot/panic/IRQ exceptions) may call this. Modules that export poke use the `device_` prefix, **not** `dangerous_` ([silica_device_actor_specification.md](silica_device_actor_specification.md) §3).
- The window is moved into that worker’s `initial_state` (or transferred by message). Clients never hold it.
- OS-hosted targets: the hosted emitter rejects `map_device` (§5.5); **do not** pretend `malloc` is MMIO. A hosted port that maps a real device through its OS may later implement it in its own emitter.

Board packs publish the legal windows (e.g. `0x40000000`–`0x400FFFFF` on a given STM32-class map, or the ESP32-S3 peripheral bus).

### 5.2 Volatile load and store: `peek` and `poke`

```text
peek(window: device_window(R, D), register: atom) -> T proc[register_rwr]
poke(window: device_window(R, D), register: atom, value: T) -> atom proc[register_rwr]
```

Registers are named, never addressed by offset, and every access carries a required width marker, `Register8`, `Register16`, `Register32`, or `Register64`, whose one type (`uint8` … `uint64`) is `T`: `peek(uart, :status) impl Register32 {}`, `poke(uart, :fifo, value impl Register32 {})`. The shared compiler checks the name, width, and access mode against the device description and resolves the name to an offset. Language rules (markers, descriptions, access modes, discarded reads, name resolution): [silica_device_actor_specification.md](silica_device_actor_specification.md) §4.7–§4.9.

The design is dedicated prims, for every port. The lexer through the SIR generator are shared by all ports, so the access design cannot differ per port; an earlier option of reusing `read_ref` / `write_ref` / `buf_load` / `buf_store` on the `device` space was dropped. Dedicated prims let a hosted emitter reject device access by prim name (§5.5), and they keep `read_ref` and friends RAM-only on every target.

What the port's emitter must produce:

- Each `peek` is exactly one load, and each `poke` exactly one store, of the marked width, at `base + offset`, where the SIR prim node carries the offset resolved from the description. None is invented, removed (including a `peek` whose result is unused or bound to `_`), merged, split, or moved past another `peek` or `poke`, and none takes a cacheable path.
- Widths: `uint32` first (typical MMIO); `uint8`, `uint16`, and `uint64` where the board pack allows. The emitter rejects a width its pack does not allow.
- Alignment and bounds need no code: the shared compiler validates them once, on the description, and every access names a validated register.

### 5.3 Ordering

Spec §9.1.1: `register_rwr` → `DSB SY` before and `ISB` after on AArch64. ESP32-S3 uses the port table (not those opcodes). Barriers on the effect are **not** a substitute for mapping and volatile access; they wrap the accesses.

### 5.4 What this is not

- Not user-facing CPU GPR moves.
- Not `device_io` (print/file/console as hosted syscalls).
- Not a vendor HAL. The register map is the programmer-supplied **device description** ([silica_device_actor_specification.md](silica_device_actor_specification.md) §4.9); a later generator may produce descriptions from vendor files (SVD, CMSIS-like headers).

### 5.5 Where the checks live

The lexer, parser, checkers, and SIR generator are the same for every port; a port's differences live only in its own `emitter/<T>/` tree.

| Stage | What it does with the poke prims |
| --- | --- |
| Shared (lexer → SIR) | Reads and validates device descriptions, types `map_device`, `peek`, and `poke`, enforces every language rule in the device spec §11, and lowers each call to a SIR prim node carrying `[register_rwr]` (as `spawn` carries `[concurrency]`) with the register's resolved offset and width. Identical on every port. |
| OS-free emitter (ESP32-S3, later bare-metal AArch64) | Emits the mapping, the volatile accesses, and the port's barriers (§5.3); rejects widths the board pack does not allow. |
| OS-hosted emitter (Apple Silicon, Linux AArch64, later hosted ports) | Rejects each poke prim in its `emit_prim_op` with a compile-time error naming the module, the enclosing function, and the prim. The rejection is explicit: the AArch64 emitters' catch-all for an unknown prim writes only a comment, which would compile the access into nothing. |

Today the emitter has no way to report a compile error: it returns assembly text, and the ESP32-S3 emitter's refusals are `.error` lines that fail only when the file is assembled, with no Silica source location. Delivering the poke prims therefore includes a shared, target-neutral diagnostic channel from `emitter_core` to the driver, which prints through the normal compiler diagnostics and exits non-zero without writing a `.sams`. The first version reports the module, function, and prim, which the emitter already knows; line and column need source locations on SIR nodes, which can follow. The ESP32-S3 refusals (foreign calls, file io) can move onto the same channel.

---

## 6. CPU system registers (runtime, not apps)

Boot must program attributes the app cannot: `MAIR_EL1`, page tables, interrupt mask, stack pointer at EL1, Xtensa window/PS, and so on.

These belong in the **runtime / board pack**, in a tightly reviewed stub, not in application Silica and not in a `register_rwr` worker. If the language later grows prims (`read_sysreg`, `write_sysreg`), they stay board-gated and out of `device_*` poke modules. They do **not** replace §5.

---

## 7. Memory spaces on the board

Follow [memory-effects-aarch64-implementation-plan.md](memory-effects-aarch64-implementation-plan.md). Gaps that block a port even after MMIO prims exist:

1. Pass `Space` from `alloc_region` into `_silica_rt_region_alloc` (today unused).
2. Per-`Space` pools: cached RAM, non-cacheable/DMA, atomic-capable, **device windows**.
3. Atomic lowering (`LDAR`/`STLR` or Xtensa).
4. Capability flags when a `Space` is emulated (e.g. no true write-through on ESP32-S3).

`device` RAM-like arenas (if any) stay distinct from **mapped peripheral windows**.

---

## 8. Boot, image, and I/O without Darwin

A first bring-up image must:

1. Reset → set SP → `.bss` → call `main` (or a tiny runtime `start`).
2. Link at the board’s load address (ELF `aarch64-none-elf`, Xtensa ELF, or a raw firmware blob).
3. Provide **one** early console for panic/bring-up (UART in the **reset stub**, or semihosting in QEMU). After the scheduler is up, application print uses a **device worker**, not `main` poke.
4. Provide a **tick** if actors run (timer IRQ). Cooperative single-thread bring-up may defer preemption.

The current `.sams` + clang Mach-O path does not produce this image. Either a hosted cross toolchain (`clang -target aarch64-none-elf`) or the [direct object emitter](direct_machine_object_emitter_future.md) ELF/firmware writer is required. Assembly text may remain during bring-up; the port must not depend on macOS `ld` or libSystem.

---

## 9. Actors

Hosted actor spawn uses OS threads and Darwin-backed stacks. OS-free must use the board runtime: one or more cores, per-actor stacks from **normal** (or specified) pools, IRQ-safe enqueue. [IPC_Bare.md](IPC_Bare.md) is the sketch; it is not wired to the current compiler.

Minimum for “actors on metal”: timer tick, mailbox in normal or atomic RAM, and MMIO only through a **device worker** (`spawn_device` / `device_actor_ref`). Ordinary `spawn` and `main` must not poke. IRQ handlers only enqueue to the owning worker.

Normative rules, registries, `device_*` vs `dangerous_*`, and the boot/panic/IRQ exceptions: [silica_device_actor_specification.md](silica_device_actor_specification.md).

---

## 10. Board pack contract

Each pack is a named directory or fragment (exact layout later) that states:

| Field | Example |
| --- | --- |
| Triple / chip family | `aarch64-none-elf`, `xtensa-esp32s3-elf` |
| Memory map | RAM, flash, peripheral windows |
| `Space` realization | exact / emulated / unsupported |
| Legal `map_device` ranges | list of `[base, size)` |
| Console | UART base + register offsets, or semihosting |
| Timer | IRQ number, programming sequence |
| IRQ → worker | Which `device_actor_ref` / registered atom owns each IRQ |
| Privileged boot | who sets MAIR/PTE or IDF cache mode |

The port's emitter refuses a constant `map_device` outside the pack’s windows (the pack is per port, so this check is not in the shared compiler). A pack is how “raw metal” stays typed instead of `uint64` everywhere.

---

## 11. Target attention order

From [ROADMAP.md](../../../ROADMAP.md), not a schedule:

1. **QEMU virt AArch64 OS-free** — MAIR, UART MMIO, ELF, no vendor IDF.
2. **ESP32-S3** — Xtensa emit + IDF-style pools + peripheral bus.
3. **Other AArch64 boards** — new packs, same `chip/arm64`.
4. Hosted Linux AArch64 can share chip lowering but **not** OS-free `Space` guarantees.

Do not generalize the object layer across chips before one OS-free AArch64 image runs.

---

## 12. Phased plan

### Phase A — Inventory freeze

This document plus board-pack fields. No silent reuse of `malloc` as `device`.

### Phase B — Space tag and pools

Wire `alloc_region(Space)` to a real allocator on one QEMU image. `device` pool is **not** UART; it is only used if the pack has a device-attributed RAM window.

### Phase C — MMIO prims

Device descriptions, `map_device`, and 32-bit `peek` / `poke`, with the hosted emitters' rejection (§5.5). First QEMU trial may use the **reset-stub** UART exception. Application poke waits for Phase F.

### Phase D — Console and panic

Early panic UART stays in the stub. After Phase F, hosted `device_io` print on that target is replaced by casts to a UART device worker.

### Phase E — Image

`aarch64-none-elf` (or direct ELF writer). Reset stub. CI: QEMU boot + UART output golden.

### Phase F — Time and device workers

Tick + `spawn_device` UART (or timer) worker. Clients `cast_device`. Privilege/sysreg stays in the stub. Checker markers from [silica_device_actor_specification.md](silica_device_actor_specification.md) §11 land with this phase or immediately after.

### Phase G — Second chip

ESP32-S3 pack and Xtensa port table. Same Silica MMIO surface; different encodings.

---

## 13. Non-goals

- User syntax for CPU GPR moves or a general assembler.
- Promising `device` hardware attributes on macOS/Linux processes.
- A full vendor HAL in the first port.
- Replacing Fifi for existing hosted C libraries.
- Claiming every MCU in [IPC_Bare.md](IPC_Bare.md) (AVR, PIC, …) is in the current ROADMAP.

---

## 14. Risks

- A device description that does not match the silicon passes every check and drives the wrong register. Descriptions must come from the vendor's register documentation (or, later, a generator over vendor files).
- A hosted emitter that lets a poke prim reach its catch-all compiles the access into nothing; the rejection must be explicit and covered by a trial on each hosted path.
- Mapping `malloc` memory as `device` will look like a port and fail on hardware.
- Page-table `device` attributes and MMIO windows must agree or the CPU will cache or fault.
- QEMU UART is not a real SoC; packs must not hard-code virt addresses as if they were ESP32-S3.
- Privileged sysreg prims in app code would bypass the boot-stub reservation.
- Loose `register_rwr` in `main` would undo device-worker isolation; enforce [silica_device_actor_specification.md](silica_device_actor_specification.md) when the checker lands.

---

## 15. Design rule

An OS-free port is complete when a **board pack** plus **compiler target** plus **runtime** can: boot, honor each supported `Space` in hardware, **map and access device registers from a device worker**, print without a hosted kernel (stub for panic; worker for apps), and run the actor loop on board time.

Until `map_device` (or equivalent) and volatile device access exist, raw-metal **drivers** are incomplete. Until boot, pools, and an ELF/firmware image exist, raw-metal **as a target** is incomplete. Those are separate missing pieces; both are required.
