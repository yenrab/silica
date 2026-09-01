# Silica Device Actor Specification

**Status:** Normative for the language rules below. **Not implemented** in the current compiler or runtime. Until the checker enforces these rules, `register_rwr` is only an effect name plus optional barriers. Bring-up inventory: [porting_for_os_free_targets.md](porting_for_os_free_targets.md).

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

- **Worker-scoped poke.** `map_device`, device volatile load/store, and `sequence proc[register_rwr]` appear only inside a **device worker** behavior installed by `spawn_device` / `spawn_device_registered`.
- **Install vs execute.** The spawn site requires `concurrency` only. It must not declare `register_rwr`. Mapping and stores run when the worker handles a message, not at spawn.
- **Typed boundary.** `device_actor_ref` is not `actor_ref` and not `dangerous_actor_ref`. No coercion.
- **Split registries.** Ordinary, FFI-dangerous, and device workers use **three** atom-keyed tables. `cast_registered` must not resolve a device worker; `cast_device_registered` must not resolve an ordinary or FFI worker.
- **Cast-only.** Device workers, and clients that initiate device work, use cast-only behaviors (`:no_reply`). `call` to a driver that also services IRQs is unsupported.
- **Exclusive window.** Each board-legal MMIO range is owned by at most one actor. The `region(R, device)` from `map_device` is moved into that actor’s initial state.
- **`device_*` modules.** Modules that declare or call map/load/store poke APIs use the `device_` name prefix. The prefix propagates to the root when the program depends on such a module, analogous to `dangerous_*`.
- **Disjoint from Fifi.** A compilation unit must not `use` both a `device_*` poke module and a `dangerous_*` FFI module. Device-read bytes are not `external_danger`-touched; FFI results must not appear in `register_rwr` sequences (existing taint).
- **Named exceptions.** Reset, early panic, and IRQ **enqueue** may touch hardware **outside** any actor. No other path may.

---

## 2. Terminology

**Device register / MMIO.** A peripheral control or status word at a board-fixed bus address. Not a CPU GPR. Not a privileged system register (`MAIR_EL1`, DAIF, …).

**Device worker actor.** An actor spawned with `spawn_device` or `spawn_device_registered`, referenced by `device_actor_ref`, whose behavior may contain `register_rwr` sequences and own a mapped `region(R, device)`.

**Client actor.** An ordinary actor (`actor_ref`) that requests device work by `cast` to a device worker. It must not declare `register_rwr` or call poke prims.

**Board window.** A `[base, size)` range listed in the selected board pack as legal for `map_device`.

**Poke prim.** `map_device` and the volatile device load/store forms (or `read_ref` / `write_ref` when the static `Space` is `device`). Exact prim names follow [porting_for_os_free_targets.md](porting_for_os_free_targets.md) §5.

---

## 3. Module naming (`device_*`)

### 3.1 Rule

A module that declares poke prims, exports a device-worker behavior that contains `register_rwr`, or `use`s a module whose name begins with `device_`, must itself use the `device_` prefix. The requirement propagates along `use` to the application root, as `dangerous_` does for FFI.

### 3.2 Disjointness

A module must not depend on both a `device_*` module and a `dangerous_*` module. A device worker must not call `dangerous_*` functions. An FFI worker must not contain `register_rwr` or poke prims.

Hosted `device_io` (stdout/files) is a different effect. It does not require a `device_*` module or a device worker.

### 3.3 Generated register maps

A later generated map (CMSIS-like field names) must live in a `device_*` module and be called only from a device worker. Clients see protocol atoms, not addresses.

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

`spawn_device` / `spawn_device_registered` require `concurrency`. The enclosing `sequence` must not declare `register_rwr`. Spawn does not execute MMIO; it moves `initial_state` (including a mapped device region, if present) to the worker.

### 4.4 Worker sequence

Poke prims and device-volatile loads/stores appear only in the sequence portion of

```
sequence proc[register_rwr] ... produces pure ... end
```

inside a device-worker behavior. `register_rwr` authorizes **execution** of poke in that behavior. It does not authorize the spawn caller.

A completed `register_rwr` sequence produces structurally pure Silica values (status atoms, integers copied out of a register, owned buffers). It must not produce a `region(R, device)` or a raw bus address to a client.

### 4.5 Cast-only

Device-worker behaviors and client behaviors that initiate device work must be cast-only (`:no_reply`). Results return by `cast` to a receiver named in the request (typically the client).

`call` to a device worker is a compile-time error. Rationale: a driver that also handles IRQs can deadlock if a client `call`s it on the same core.

### 4.6 Handshake

1. A client with a cast-only behavior receives work by `cast`.
2. The client `cast_device`s a request to the device worker (or `cast_device_registered`).
3. The worker runs `sequence proc[register_rwr]`, performing poke as needed.
4. If there is a result, the worker `cast`s it to the named ordinary `actor_ref`.

From the client’s scheduler view this is non-blocking, as with Fifi.

---

## 5. Ownership of windows and DMA

### 5.1 Exclusive map

`map_device(base, size)` succeeds only if `[base, size)` is a board-pack window and no other live mapping overlaps that window. The resulting `region(R, device)` is move-only.

The region is moved into the device worker via `spawn_device` initial state, or via a later message that transfers ownership. After the move, the sender must not use the handle (§12.1.5 / §4.4.2).

Two workers must not own overlapping windows. Two logical devices on one I²C/SPI controller share **one** worker (or an explicit lock actor that is itself the sole window owner).

### 5.2 Client messages

Clients send protocol atoms and Silica values (`:uart_tx`, a `uint8`, a `buf` to fill). They must not send a bus address or a `region(R, device)` unless the message is an explicit **ownership transfer** of a window to a worker (rare; normally the window is in initial state).

Offset+width access stays inside the worker.

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

On OS-hosted processes, `map_device` is rejected or requires a platform device mmap. `malloc` is not a device window. Hosted `device_io` print does not use this specification.

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
| Overlapping `map_device` vs board pack or a live map | Error |
| `device_*` module `use`s `dangerous_*` (or the reverse) | Error |
| Client message contains a raw bus address type | Error |

Suggested codes: **E2201–E2215** (device-actor family), adjacent to FFI taint **E2103**. Exact strings are assigned when the checker lands. Error-enforcement trials belong under `trials/error_enforcement_addition/` once seed/selfhost implement the rules.

Until then, implementations **SHOULD** still follow this document in new OS-free work.

---

## 12. Relationship to other documents

- Language surface (`device_actor_ref`, spawn/cast names, `register_rwr` scope): [silica-specification.md](silica-specification.md) §4.5.1, §9.1.1, §15.1.1.
- `map_device` and volatile access: [porting_for_os_free_targets.md](porting_for_os_free_targets.md) §5.
- FFI isolation: [silica_ffi_wrapper_specification.md](silica_ffi_wrapper_specification.md). Do not merge device workers into `spawn_dangerous`.
