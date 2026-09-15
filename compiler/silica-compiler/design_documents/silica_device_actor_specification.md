# Silica Device Actor Specification

**Status:** Normative for the language rules below. **Not implemented** in the current compiler or runtime. Until the checker enforces these rules, `register_rwr` is only an effect name plus optional barriers. The poke prims are `map_device`, `peek`, and `poke` (§4.7); every device they reach needs a programmer-supplied device description (§4.9). Bring-up inventory: [porting_for_os_free_targets.md](porting_for_os_free_targets.md).

This specification is the device-register counterpart to [silica_ffi_wrapper_specification.md](silica_ffi_wrapper_specification.md). Ordinary actors never poke MMIO. Dedicated **device worker actors** own mapped windows and execute `register_rwr` sequences. Application actors request work by `cast`.

## Related documents

| Document | Purpose |
| --- | --- |
| [silica-specification.md](silica-specification.md) | Effects, regions, actors, `register_rwr`, `mem(device)` |
| [silica_ffi_wrapper_specification.md](silica_ffi_wrapper_specification.md) | FFI workers; **disjoint** from device workers and `device_*` modules |
| [porting_for_os_free_targets.md](porting_for_os_free_targets.md) | OS-free board packs, `map_device`, boot/panic/IRQ exceptions |
| [atom_actor_registry_direct_index_design.md](atom_actor_registry_direct_index_design.md) | Atom-keyed slot tables (a **third** table for device workers) |

---

## 1. Design principles

- **Worker-scoped poke.** `map_device`, `peek`, `poke`, and `sequence proc[register_rwr]` appear only inside a **device worker** behavior installed by `spawn_device` / `spawn_device_registered`.
- **Install vs execute.** The spawn site requires `concurrency` only. It must not declare `register_rwr`. Mapping and stores run when the worker handles a message, not at spawn.
- **Typed boundary.** `device_actor_ref` is not `actor_ref` and not `dangerous_actor_ref`. No coercion.
- **Split registries.** Ordinary, FFI-dangerous, and device workers use **three** atom-keyed tables. `cast_registered` must not resolve a device worker; `cast_device_registered` must not resolve an ordinary or FFI worker.
- **Cast-only.** Device workers, and clients that initiate device work, use cast-only behaviors (`:no_reply`). `call` to a driver that also services IRQs is unsupported.
- **Exclusive window.** Each board-legal MMIO range is owned by at most one actor. The `device_window` from `map_device` is moved into that actor’s initial state.
- **Described devices.** Registers are reached by name, never by address or offset, and every name, width, and access mode is checked against the device's description (§4.9).
- **`device_*` modules.** Modules that call `map_device`, `peek`, or `poke` use the `device_` name prefix. The prefix propagates to the root when the program depends on such a module, analogous to `dangerous_*`.
- **Disjoint from Fifi.** A compilation unit must not `use` both a `device_*` poke module and a `dangerous_*` FFI module. Device-read bytes are not `external_danger`-touched; FFI results must not appear in `register_rwr` sequences (existing taint).
- **Named exceptions.** Reset, early panic, and IRQ **enqueue** may touch hardware **outside** any actor. No other path may.

---

## 2. Terminology

**Device register / MMIO.** A peripheral control or status word at a board-fixed bus address. Not a CPU GPR. Not a privileged system register (`MAIR_EL1`, DAIF, …).

**Device worker actor.** An actor spawned with `spawn_device` or `spawn_device_registered`, referenced by `device_actor_ref`, whose behavior may contain `register_rwr` sequences and own a `device_window`.

**Client actor.** An ordinary actor (`actor_ref`) that requests device work by `cast` to a device worker. It must not declare `register_rwr` or call poke prims.

**Board window.** A `[base, size)` range listed in the selected board pack as legal for `map_device`.

**Device tag.** A one-atom tagged tuple type, such as `(:esp32s3_uart)`, that names a device and its description.

**Device description.** The programmer-supplied implementation of `DeviceDescription` for a device tag: the device's registers with offsets, widths, and access modes (§4.9).

**Device window.** A `device_window(R, D)` returned by `map_device`: the mapped range of device `D`.

**Poke prim.** `map_device`, `peek`, or `poke` (§4.7). These are the only way to reach a device window: `read_ref`, `write_ref`, `buf_load`, and `buf_store` never perform device access.

---

## 3. Module naming (`device_*`)

### 3.1 Rule

A module that calls poke prims, exports a device-worker behavior that contains `register_rwr`, or `use`s a module whose name begins with `device_`, must itself use the `device_` prefix. The requirement propagates along `use` to the application root, as `dangerous_` does for FFI.

### 3.2 Disjointness

A module must not depend on both a `device_*` module and a `dangerous_*` module. A device worker must not call `dangerous_*` functions. An FFI worker must not contain `register_rwr` or poke prims.

Hosted `device_io` (stdout/files) is a different effect. It does not require a `device_*` module or a device worker.

### 3.3 Device descriptions

Device descriptions (§4.9) live in `device_*` modules. A later generator may produce them from vendor register files (SVD, CMSIS-like headers); its output is an ordinary description in a `device_*` module. Clients see protocol atoms, not register names or addresses.

---

## 4. Actor install and execute

### 4.1 Intrinsics

```
spawn_device(initial_state, behavior_fn [, core_id]) -> device_actor_ref proc[concurrency]
spawn_device_registered(initial_state, behavior_fn, name: atom [, core_id]) -> device_actor_ref proc[concurrency]
cast_device(target: device_actor_ref, message) proc[concurrency]
cast_device_registered(name: atom, message) proc[concurrency]
```

`cast` / `call` / `cast_registered` / `call` on `actor_ref` must not accept `device_actor_ref`. There is no `call_device`.

### 4.2 Who may spawn what

| Intrinsic | Behavior may contain | Behavior must not contain |
| --- | --- | --- |
| `spawn` / `spawn_registered` | ordinary effects | `register_rwr`, poke prims, `external_danger`, `dangerous_*` calls |
| `spawn_dangerous` / `spawn_dangerous_registered` | `external_danger` | `register_rwr`, poke prims |
| `spawn_device` / `spawn_device_registered` | `register_rwr`, poke prims | `external_danger`, `dangerous_*` calls |

`main` is not a device worker. `main` must not contain `register_rwr` or poke prims. Supervisors and install sites use `concurrency` only.

### 4.3 Install site

`spawn_device` / `spawn_device_registered` require `concurrency`. The enclosing `sequence` must not declare `register_rwr`. Spawn does not execute MMIO; it moves `initial_state` (including a device window, if present) to the worker.

### 4.4 Worker sequence

Poke prims appear only in the sequence portion of

```
sequence proc[register_rwr] ... produces pure ... end
```

inside a device-worker behavior. `register_rwr` authorizes **execution** of poke in that behavior. It does not authorize the spawn caller.

A completed `register_rwr` sequence produces structurally pure Silica values (status atoms, integers copied out of a register, owned buffers). It must not produce a `device_window` or a raw bus address to a client.

### 4.5 Cast-only

Device-worker behaviors and client behaviors that initiate device work must be cast-only (`:no_reply`). Results return by `cast` to a receiver named in the request (typically the client).

`call` to a device worker is a compile-time error. Rationale: a driver that also handles IRQs can deadlock if a client `call`s it on the same core.

### 4.6 Handshake

1. A client with a cast-only behavior receives work by `cast`.
2. The client `cast_device`s a request to the device worker (or `cast_device_registered`).
3. The worker runs `sequence proc[register_rwr]`, performing poke as needed.
4. If there is a result, the worker `cast`s it to the named ordinary `actor_ref`.

From the client’s scheduler view this is non-blocking, as with Fifi.

### 4.7 Poke prims

```
map_device(device: D, base: uint64) -> device_window(R, D) proc[register_rwr]
peek(window: device_window(R, D), register: atom) -> T proc[register_rwr]
poke(window: device_window(R, D), register: atom, value: T) -> atom proc[register_rwr]
```

`D` is a **device tag**: a one-atom tagged tuple type such as `(:esp32s3_uart)` that names a device description (§4.9). `map_device((:esp32s3_uart), 0x60000000)` binds the window `[base, base + size)`, where `size` is the extent of the description's registers; it does not allocate (§5.1). The window's type, `device_window(R, D)`, records which device it maps, so a window is never confused with a `region(R, device)` arena and `alloc_ref`, `read_ref`, and the other region prims do not accept it. `peek` is one volatile load and `poke` one volatile store of the named register. These are dedicated prims with one design for every port. The shared compiler (lexer through SIR generator) is the same on all ports, so there is no per-port choice of access design.

- **Named registers.** `register` is an atom literal naming a register in `D`'s description. A variable, or a name the description does not list, is a compile-time error. Offsets appear only in the description: driver code contains no addresses or offsets, and every access is checked against the description.
- **Width markers.** The programmer states the width of every access with a register marker (§4.8). A `peek` is always written `peek(window, register) impl RegisterN {}` and a `poke` value is always written `value impl RegisterN {}`, where `N` is 8, 16, 32, or 64. `T` is the marker's one type (`uint8`, `uint16`, `uint32`, or `uint64`), and `N` must equal the register's width in the description. `uint32` is required on every port that implements the prims; a port whose board pack does not allow a width rejects it in its emitter.
- **Access modes.** A `poke` to a read-only register and a `peek` of a write-only one are compile-time errors (§4.9).
- **Alignment and bounds.** Checked once, on the description (§4.9). Every access is to a validated register, so no run-time bounds check is needed and an access never touches an address outside its window.
- **No move.** Passing the window to `peek` or `poke` does not move it; the worker keeps the handle across accesses. `map_device` returns a move-only handle (§5.1).
- **Result.** `poke` returns `:ok`, as `write_ref` returns an atom.
- **Reads are never removed.** Reading a register can change the device (a FIFO pops, a status flag clears), so every `peek` is exactly one load even when its result is unused. A read done only for its effect binds the result to `_`: `_: uint32 <- peek(uart, :int_raw) impl Register32 {};`.
- **No read-modify-write prim.** Changing some bits of a register is a `peek` followed by a `poke`: two accesses. Bits the hardware changes between the two accesses are the driver's responsibility. For registers described as write-one-to-clear, the compiler warns about the most common mistake (§4.9).
- **Names are not reserved.** `map_device`, `peek`, and `poke` are not keywords. An unqualified call to one of them inside a `device_*` module is the prim. Everywhere else, and whenever it is module-qualified (`m@peek`), the name is an ordinary identifier, so an unrelated function such as the standard priority queue's `peek` is unaffected. A `device_*` module must not define a function with one of these names.

```silica
// Inside a device-worker behavior; `uart: device_window(R, (:esp32s3_uart))` is the worker's window.
// Register names come from the description in §4.9.
sequence proc[register_rwr]
    status: uint32 <- peek(uart, :status) impl Register32 {};  // 32-bit load of :status
    _: uint32 <- peek(uart, :int_raw) impl Register32 {};      // read only for its effect; still performed
    sent: atom <- poke(uart, :fifo, 784 impl Register32 {});   // 32-bit store; 784 takes type uint32
produces
    pure status
end
```

**Hosted targets** reject the poke prims in their emitters (§10).

### 4.8 Register markers

Four built-in marker traits state access widths. Each has exactly one implementation:

```
// register8.silica          // register16.silica
export trait Register8;      export trait Register16;
impl uint8;                  impl uint16;

// register32.silica         // register64.silica
export trait Register32;     export trait Register64;
impl uint32;                 impl uint64;
```

- **Closed.** No other implementation exists or may be added. Unlike `expr impl ActorMessage {}`, the postfix `expr impl RegisterN {}` never establishes an implementation; it asserts that `expr` has the marker's one type.
- **Required.** The marker is written directly on every `peek` call and directly on every `poke` value, even when the type is already known. A missing marker is a compile-time error.
- **Agreement.** The marked expression's type must be the marker's type. `word impl Register32 {}` with `word: uint64` is an error, and so is binding `peek(...) impl Register32 {}` to a `uint16`. An unsized integer literal takes the marker's type, because exactly one implementation is viable (spec §3.4.11), so `784 impl Register32 {}` is a `uint32`.

The marker makes the width the programmer's explicit statement at every access, and the checker compares it with the register's width in the device description (§4.9). The width is written twice, at the access and in the description, and the two must agree.

### 4.9 Device descriptions

Every device a program accesses must have a **device description**: code, supplied by the programmer, that lists the device's registers with their offsets, widths, and access modes. It is an implementation of the built-in `DeviceDescription` trait:

```
// devicedescription.silica (built-in)
export trait DeviceDescription;
export registers/1;

required {
    fn registers(device: DeviceDescription) -> List[{ name: atom, offset: uint64, width: uint64, access: atom }];
}
```

```silica
// device_esp32s3_uart.silica — supplied by the programmer (layout illustrative)
use devicedescription;

impl fn registers(device: (:esp32s3_uart)) -> List[{ name: atom, offset: uint64, width: uint64, access: atom }] {
    [
        { name: :fifo,    offset: 0x00, width: 32, access: :read_write },
        { name: :int_raw, offset: 0x04, width: 32, access: :write_one_to_clear },
        { name: :status,  offset: 0x1C, width: 32, access: :read_only }
    ]
}
```

The description covers the register layout only. The base address is not part of it: the same peripheral can appear at several addresses (UART0, UART1, …), so the base is given to `map_device` and checked against the board pack.

**Rules that differ from ordinary traits.** `DeviceDescription` follows spec §3.4.8–§3.4.9 except:

1. **Where it is implemented.** Implementations are written in `device_*` modules, not in `devicedescription.silica`. Such a module must `use devicedescription;`. In a `device_*` module, `impl fn registers(...)` always implements `DeviceDescription`; no other trait's `impl fn` may appear outside its own file.
2. **What it is implemented for.** The first parameter's type is a device tag, `(:tag)`. A program has at most one description per tag; a second is a compile-time error.
3. **Data only.** The body is a single list literal of record literals whose fields are literals. No calls, bindings, conditionals, or arithmetic. The compiler reads the description as data at compile time without running it; programs may also call `devicedescription@registers` at run time like any trait method.
4. **Required.** A `map_device` whose tag has no description visible from the calling module (in that module, or in a module it `use`s) is a compile-time error. There is no device access without a description.

**Validity.** The shared compiler checks each description:

- Register names are unique within the description.
- `width` is 8, 16, 32, or 64 (bits).
- `offset` is a multiple of `width / 8` (aligned).
- No two registers overlap.
- `access` is one of the four modes below.
- There is at least one register. The window size is the largest `offset + width / 8`.

**Access modes:**

| `access` | `peek` | `poke` |
| --- | --- | --- |
| `:read_write` | allowed | allowed |
| `:read_only` | allowed | compile-time error |
| `:write_only` | compile-time error | allowed |
| `:write_one_to_clear` | allowed | allowed; the compiler warns when the poked value is the unmodified result of a `peek` of the same register, a read-modify-write that clears every bit that was set |

**Where the checks run.** Descriptions and every check on them are in the shared compiler and behave the same on every port. The SIR prim node carries the resolved offset and width, so emitters never see register names.

The description is still the programmer's statement about the hardware: a description that does not match the silicon passes every check. Descriptions should be written from the vendor's register documentation. A generator that produces them from vendor files may come later (§3.3).

---

## 5. Ownership of windows and DMA

### 5.1 Exclusive map

`map_device(D, base)` succeeds only if `[base, base + size)`, with `size` from `D`'s description, is a board-pack window and no other live mapping overlaps that window. The resulting `device_window(R, D)` is move-only.

The window is moved into the device worker via `spawn_device` initial state, or via a later message that transfers ownership. After the move, the sender must not use the handle (§12.1.5 / §4.4.2).

Two workers must not own overlapping windows. Two logical devices on one I²C/SPI controller share **one** worker (or an explicit lock actor that is itself the sole window owner).

### 5.2 Client messages

Clients send protocol atoms and Silica values (`:uart_tx`, a `uint8`, a `buf` to fill). They must not send a bus address or a `device_window` unless the message is an explicit **ownership transfer** of a window to a worker (rare; normally the window is in initial state).

Register access stays inside the worker.

### 5.3 DMA buffers

`buf` / `region` in `normal_noncacheable` (or the pack’s DMA space) used for a transfer is **moved** to the device worker in the request and **moved back** in the result cast. The client must not access the buffer until it is returned. This is ordinary region-move law; checkers must apply it to device-transfer message types.

---

## 6. Taint and effect crossing

- Values from `dangerous_*` / `external_danger` must not appear in a `register_rwr` sequence (existing Fifi E2103-class rule).
- Values produced by a `register_rwr` sequence are not `external_danger`-touched. They must not be passed to an FFI worker without an explicit adapter in a `dangerous_*` module (which a `device_*` module cannot `use`). Crossing FFI and device therefore requires a third, ordinary module that only moves already-pure values — or the program is rejected.
- `register_rwr` data must not be used inside `device_io`, `network_io`, or `hot_swap` sequences without a documented copy-out to a pure value first (same structural caution as Fifi `produces pure`).

---

## 7. Exceptions (not actors)

These paths may touch hardware **without** a device worker. They are the only exceptions.

| Path | Allowed | Forbidden |
| --- | --- | --- |
| **Reset / boot stub** | Program SP, `.bss`, MAIR/PTE or vendor cache mode; optional early UART for bring-up | Application Silica in `main` |
| **Early panic** | Runtime UART or semihosting before the scheduler runs | Depending on a live device actor |
| **IRQ enqueue** | Acknowledge the IRQ if the board requires it; enqueue a message to the owning `device_actor_ref` | Full register protocols, `cast` from the handler into arbitrary actors, running a behavior function in IRQ context |

Privileged CPU system registers stay in the boot/runtime stub, not in `register_rwr` application workers.

After the actor runtime is up, **application** console print on OS-free targets goes through a device worker (UART), not through these exceptions.

---

## 8. Supervision

A supervisor may restart a device worker. Restart does **not** reset the peripheral. The replacement behavior’s first work (or `Supervisor` init for a device supervisor, if added later) must run a documented **hardware `init` / `recover`**. Otherwise MMIO after crash is undefined.

`link` / `monitor` on `device_actor_ref` follow ordinary actor rules once the type is accepted by those intrinsics; until specified, monitor a wrapper ordinary actor, not the device ref.

---

## 9. Runtime IRQ contract

The IRQ handler is not a Silica behavior. It may only:

1. Do the minimum ack the platform requires.
2. Enqueue a message to the single `device_actor_ref` that owns that IRQ’s window.

The board pack names IRQ → worker. Two workers must not share one IRQ.

---

## 10. Hosted targets

The rules in §3–§6 and §11 are checked by the shared compiler, so a program is accepted or rejected for the same language reasons on every port. Whether a target can actually reach a device is decided by that port's emitter:

- An OS-hosted emitter (Apple Silicon, Linux AArch64, and later hosted ports) rejects every `map_device`, `peek`, and `poke` with a compile-time error naming the module, the enclosing function, and the prim. The error goes through the compiler's diagnostics like any other compile error; it is not an assembler `.error` line, and the emitter must not fall through to a catch-all that emits nothing for the prim.
- The rejection applies wherever a poke prim appears, including in a function nothing calls. A program that runs on both kinds of target keeps its device code in `device_*` modules that the hosted build does not depend on.
- `spawn_device`, `cast_device`, and the other device-worker intrinsics are not rejected. Without `map_device` a device worker on a hosted target has no window to reach.
- `malloc` is not a device window. A hosted port that maps a real device through its OS (for example `/dev/mem` or UIO on Linux) may later implement the prims in its own emitter, with no change to the shared compiler.

Hosted `device_io` print does not use this specification.

---

## 11. Compile-time enforcement (when implemented)

The type checker uses the same style of markers as `__silica_tc_in_behavior` / `__silica_tc_in_main`:

| Situation | Result |
| --- | --- |
| `register_rwr` or poke prim in `main` | Error |
| `register_rwr` or poke prim in a `spawn` / `spawn_registered` behavior | Error |
| `register_rwr` or poke prim in a `spawn_dangerous` behavior | Error |
| `external_danger` or `dangerous_*` call in a `spawn_device` behavior | Error |
| `spawn_device` in a `register_rwr` sequence | Error |
| `call` / `cast` of `device_actor_ref` via ordinary primitives | Error |
| `cast_registered` name bound in the device table | Error (use `cast_device_registered`) |
| `map_device` overlapping a live map | Error |
| `device_*` module `use`s `dangerous_*` (or the reverse) | Error |
| Client message contains a raw bus address type | Error |
| Unqualified `map_device` / `peek` / `poke` call outside a `device_*` module that names no function in scope | Error that points to this specification, not a bare undefined-identifier error |
| `device_*` module defines a function named `map_device`, `peek`, or `poke` | Error |
| `peek` call or `poke` value without a `RegisterN` marker | Error |
| Marked expression's type is not the marker's type | Error |
| Any `impl` of `Register8` / `Register16` / `Register32` / `Register64` outside the built-in ones | Error |
| `map_device` with a device tag that has no description visible from the calling module | Error |
| Second description for the same device tag | Error |
| Description body that is not a list literal of literal records | Error |
| Description with a duplicate name, a width other than 8/16/32/64, a misaligned offset, overlapping registers, an unknown access mode, or no registers | Error |
| `impl fn` of any trait other than `DeviceDescription` outside its trait's file | Error |
| `peek` / `poke` register that is not an atom literal, or not in the window's description | Error |
| Marker width differs from the register's described width | Error |
| `poke` to a `:read_only` register, or `peek` of a `:write_only` register | Error |
| `poke` of the unmodified result of a `peek` of the same `:write_one_to_clear` register | Warning |

These checks are in the shared compiler and behave the same on every port. The checks below depend on the target or its board pack, so each port's emitter makes them:

| Situation | Result |
| --- | --- |
| Poke prim compiled for a target that cannot reach devices (every OS-hosted target today) | Error naming the module, function, and prim (§10) |
| `peek` / `poke` width the port's board pack does not allow | Error naming the width |
| Constant `map_device` outside the board pack's windows | Error |

Suggested codes: **E2201** onward (device-actor family), adjacent to FFI taint **E2103**; the write-one-to-clear warning takes a warning code in the same family. Exact strings are assigned when the checker lands. Error-enforcement trials belong under `trials/error_enforcement_addition/` once seed/selfhost implement the rules; the hosted rejection needs one such trial on each hosted path.

Until then, implementations **SHOULD** still follow this document in new OS-free work.

---

## 12. Relationship to other documents

- Language surface (`device_actor_ref`, `device_window`, spawn/cast names, `register_rwr` scope, the poke prims): [silica-specification.md](silica-specification.md) §4.4.6, §4.5.1, §9.1.1, §9.2.2, §15.1.1.
- Lowering of the poke prims (volatile access, barriers, board windows) and where each check lives: [porting_for_os_free_targets.md](porting_for_os_free_targets.md) §5.
- FFI isolation: [silica_ffi_wrapper_specification.md](silica_ffi_wrapper_specification.md). Do not merge device workers into `spawn_dangerous`.
