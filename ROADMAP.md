# Roadmap

Development is organised by **emitter path**. Each path is a backend under
[src_selfhost/emitter/](compiler/silica-compiler/src_selfhost/emitter/), and each path advances through
numbered **fixed points**: a fixed point is a build of the self-hosted compiler that passes the whole
[trial tree](trials/) when built by itself (gen2) and reproduces itself byte for byte (gen3 = gen2; see
[Build and test](README.md#building-the-compiler)). Between fixed points the not-yet-implemented parts of the
[specification](compiler/silica-compiler/design_documents/silica-specification.md) are delivered in **chunks**;
a chunk is closed by a new fixed point, never by a build that merely compiles. Dates are deliberately absent.

## The three paths

| Path | Emitter | Rule |
| --- | --- | --- |
| Apple Silicon (macOS, AArch64) | `emitter/apple_silicon_mac/` | Leads. The code as it is today is the content of its first fixed point (FP1). |
| Linux AArch64 | `emitter/linux_aarch64/` | **Lock-step** with Apple Silicon: fixed point *n* on Linux contains exactly the Silica behaviours of fixed point *n* on Apple Silicon. Apple leads inside a chunk; the chunk is not closed until Linux has caught up and both fixed points are re-established. Port notes: [linux_aarch64_port_checklist.md](compiler/silica-compiler/design_documents/ports/linux_aarch64_port_checklist.md). |
| ESP32-S3 (Xtensa LX7, OS-free) | `emitter/ESP32-S3_raw/` | **Not** in lock-step. Its first fixed point has all the behaviours of Apple Silicon FP1 **plus peek and poke** (device memory access, below). It then works through the same chunks at its own pace, skipping hosted-only items. Port notes: [esp32s3_xtensa_port.md](compiler/silica-compiler/design_documents/ports/esp32s3_xtensa_port.md), [porting_for_os_free_targets.md](compiler/silica-compiler/design_documents/porting_for_os_free_targets.md). |

**Peek and poke (first delivered in ESP32-S3 FP1).** Every device has a programmer-supplied **device description**, an
implementation of the built-in `DeviceDescription` trait that lists its registers with offsets, widths, and access modes.
`map_device(device, base) -> device_window(R, D)` binds that device's registers at a board-legal address (a bind, not an
allocation); `peek(window, :register)` and `poke(window, :register, value)` are the volatile device load and store, dedicated
prims that name registers rather than offsets, with the width stated at every access by a required `Register8`…`Register64`
marker and checked against the description. Only a `spawn_device` worker in a `device_*` module may call them, and
`register_rwr` ordering follows the port table. The lexer through the SIR generator are the same on every path, so
the prims are part of every path's compiler: ESP32-S3 implements them in its emitter, and each hosted emitter rejects them with
a compile error naming the module, function, and prim. That rejection needs a diagnostic channel from the emitter, which it
does not have today. When ESP32-S3 reaches FP1, Apple Silicon and Linux AArch64 take the same shared change and the rejection
and re-establish their fixed points; nothing they already run changes.
Details: [silica_device_actor_specification.md §4.7–§4.9, §10](compiler/silica-compiler/design_documents/silica_device_actor_specification.md) and
[porting_for_os_free_targets.md §5](compiler/silica-compiler/design_documents/porting_for_os_free_targets.md).

## Raw paths: chip features in chunk 2

On every raw (OS-free) path the chip's own capabilities are **chunk 2**, right after the actor stacks and data
structures, wherever the chip has them; a feature the chip lacks is recorded in the port notes as not applicable rather
than left open. Hosted paths are different by specification: on an OS the memory spaces are a discipline without
guaranteed attribute differentiation (§12.1.1.0) and placement goes through the OS affinity interface, which Apple Silicon
already has ([cpu_topology_implementation_plan.md](compiler/silica-compiler/design_documents/Phase1_TODOs/cpu_topology_implementation_plan.md)).

| Feature (spec wording) | Where | What the raw path delivers |
| --- | --- | --- |
| **Actor placement on a specific core.** `spawn(initial_state, behavior, n)` with a `uint64` logical core index or `core_id(n)` (§4.6); migrating it with `migrate_actor` (§15.1.2, §22.10); `get_cpu_topology()` and the `cpu_topology` / `core_info` records (§22.10); scheduling and affinity (§23.1.3). | §4.6, §15.1.2, §22.10, §23.1.3 | On a multi-core chip the runtime pins that actor to the named core, exclusively and for its whole life unless the program moves it or it terminates (Actor Pinning Policy, §15.1.2), and reports the real topology. ESP32-S3 has two LX7 cores, so this applies to it. |
| **Memory spaces.** `region(L, Space)` with `normal`, `normal_writeback`, `normal_writethrough`, `normal_noncacheable`, `atomic`, and `device` (§4.4); OS-free runtimes give each space its real attributes, on AArch64 through `MAIR_EL1` (§12.1.1.0, §12.1.1.1). | §4.4, §12.1.1 | Every space the chip can distinguish maps to the matching cache policy, shareability, or device attribute; `device` is the peek-and-poke window. Spaces the chip cannot distinguish are documented as collapsed. |
| **Chip-specific behaviours.** Feature detection and capability queries (§21.0), then whatever the chip has: on AArch64 SVE (§21.1), NEON (§21.2), MTE (§21.3), PAC (§21.4) and the system-register access of §21.0.2; on ESP32-S3 the Xtensa LX7 equivalents (its vector and cache-control instructions) and none of the AArch64-only items. | §21 | Each raw path carries the §21 items its chip supports, with trials, and lists the rest as not applicable. |

## Fixed point 1 (each path)

- **Apple Silicon FP1** — the current behaviour set: every trial suite green under the selfhost built by the selfhost, and `make fixpoint` passing. Progress and open items: [self-host plan](compiler/silica-compiler/design_documents/Phase1_TODOs/bootstrap_retirement_and_self_host_plan.md), [open defects](compiler/silica-compiler/design_documents/HIGH_PRIORITY_compiler_defects_and_diagnostic_gaps_2026-09-08.md). Reaching it retires the Rust bootstrap for this path.
- **Linux AArch64 FP1** — the same behaviours, same trials, on Linux.
- **ESP32-S3 FP1** — the same behaviours plus peek and poke (and, on Apple Silicon and Linux AArch64, a new fixed point that rejects them; see above), with the hosted-only pieces (console and file `device_io` syscalls, sysctl CPU topology, Fifi against OS libraries) replaced by their board-pack equivalents.

## The chunks after FP1: enhancement requests

The chunks are **enhancement requests, not an ordered list**. Their numbers are identifiers, not a sequence or a
priority: any of them can be taken up when a contributor, a dependency, or demand calls for it, and several can be in
progress at once. Two placements were decided explicitly and still hold: chunk 1 comes immediately after FP1 on every
path, and chunk 2 (the raw-path chip features) follows it on the raw paths.

However a chunk is picked up, it is done on Apple Silicon first, then Linux (lock-step), and picked up by ESP32-S3
when that path gets to it; each ends in a new fixed point for the path. Fixed points are numbered in the order they are
actually reached, so a chunk has no fixed-point number until it lands.

| # | Chunk | Spec / design | What closes it |
| --- | --- | --- | --- |
| 1 | **Growable and shrinkable actor stacks, and the remaining standard data structures.** Runtime-managed segmented stacks: a prologue probe under each frame, a guard page under each segment, growth in the fault handler, shrink when the handler returns; no OS-thread stack size and no fixed reserve as a hidden maximum. Alongside them, the remaining public data-structure traits and query backends (CSR and dense graph indexes, `BinaryTree`), then the compiler's own use of them (keyed lookups instead of association lists, `BinaryTree` AST). | §15.1.2.2; [actor_stack_growth_plan.md](compiler/silica-compiler/design_documents/Phase1_TODOs/actor_stack_growth_plan.md); [data_structure_designs](compiler/silica-compiler/design_documents/Phase1_TODOs/data_structure_designs/README.md); self-host plan Phases 4 and 7 | Deep recursion inside actors bounded only by machine memory and stacks that shrink after each message, with trials; all ten traits with goldens under `ordered_data_structures`. |
| 2 | **Raw paths: chip features.** Actor placement on a specific core, the memory spaces with their real attributes, and the chip's §21 behaviours, as listed under "Raw paths: chip features in chunk 2" above. On hosted paths this chunk is a no-op and the numbering is kept so fixed points line up across paths. | §4.4, §4.6, §12.1.1, §21, §22.10, §23.1.3 | Each raw path's port notes list every item as delivered with trials or as not applicable. |
| 3 | **Immutable lists: map, filter, reduce** with region-backed chunked storage and Collectable elements. | [list_implementation_design.md](compiler/silica-compiler/design_documents/list_implementation_design.md), [TODO M1–M3](compiler/silica-compiler/design_documents/Phase1_TODOs/list_map_filter_reduce_and_hardening_todo.md) | M1–M3 delivered with trials. |
| 4 | **Region memory safety.** Static region-lifetime analysis, buffer bounds checking, region isolation, an ownership-based release strategy that ends use-after-free, atomic references, lifetime polymorphism. | [region_memory_safety_todo.md](compiler/silica-compiler/design_documents/Phase1_TODOs/region_memory_safety_todo.md); spec §12, §30 | The §12 safety properties enforced at compile time or halted at run time, never silent. |
| 5 | **Variants and advanced control.** Variant types and variant patterns, behaviour switching by returning a different behaviour, advanced effects. | spec §4.2.5, §6.1.2, §6.3, §15 | Trials per construct. |
| 6 | **Atomic operations and synchronization guarantees** audited against the spec and completed. | spec §17, §18 | Every listed operation has a trial and a documented ordering. |
| 7 | **Extended numerics.** Big integers, big floats, rationals, big rationals, and 128-bit integers, all as distinct explicit types with no implicit widening. | spec §30; Phase 3 in the previous roadmap | Each numeric type has literals, arithmetic, comparison, conversion rules, and printing, with trials; no implicit widening anywhere. |
| 8 | **Beyond the process.** Fifi inbound calls and dynamic linking; brokered IPC so unsafe work stays out of process (hosted paths only). | spec §26.3.1; [brokered_ipc_isolation_architecture.md](compiler/silica-compiler/design_documents/brokered_ipc_isolation_architecture.md) | Inbound calls and dynamic linking work with trials on the hosted paths; unsafe work can run in a brokered process. |
| 9 | **Tooling and proof.** Formal-verification tooling, language-level cryptographic guardrails, IDE and developer-experience surface, tighter emission. | spec §29; [formal verification](compiler/silica-compiler/design_documents/silica-formal-verification-specification.md), [crypto proposal](compiler/silica-compiler/design_documents/crypto-proposal-introduction.md) | Each tool or guardrail lands with its own trials or diagnostics. |
| 10 | **TCP/IP as a first-class part of the language.** The specification makes networking part of the core language (§20.4, built-ins in §22.17); this chunk implements it in the compiler and runtime: the `network_io` effect, IP addresses, sockets, TCP and UDP as built-in types checked by the compiler, and a runtime in which a socket that is not ready suspends only its own actor. Hosted paths use the OS socket API underneath, with readiness notification (kqueue on macOS, epoll on Linux) delivered to actor mailboxes so no carrier thread ever blocks. Raw paths get a TCP/IP stack written in Silica as supervised actors (ARP, IPv4, ICMP, UDP, TCP, DHCP) over a `spawn_device` network driver; on ESP32-S3, which has no on-chip Ethernet MAC, that driver is an SPI Ethernet controller or the Wi-Fi radio. **Prerequisites:** chunk 1 (one actor per connection needs growable stacks), the buffer bounds checking from chunk 4 (every received byte is untrusted input), and on raw paths chunk 2 (the `device` and non-cacheable memory spaces for DMA rings). | spec §20.4, §22.17, §9 (effects), §15; [device actors](compiler/silica-compiler/design_documents/silica_device_actor_specification.md) | Loopback echo-server, client and many-connection trials on the hosted paths; the same trials between two boards, or a board and a host, on the raw paths; malformed-packet trials that fail safely. |


## Later paths

A new emitter's first fixed point has the same behaviours as the **analogous pre-existing fixed point**, not
Apple Silicon's: a hosted Linux path starts from Linux AArch64 (Linux on AMD or Intel x86-64 takes the current
Linux AArch64 fixed point), and a bare-metal path starts from the nearest bare-metal one (bare-metal AArch64
takes the current ESP32-S3 fixed point, peek and poke included). From there it works through the chunks.
Planned attention, hosted and bare metal in parallel: Linux x86-64 ([execution plan](compiler/silica-compiler/design_documents/porting_to_linux_x86_64_hosted.md))
and bare-metal AArch64 next; RISC-V after; Windows and other MCU classes later.

## Runtime (Track 2)

Fifi, calling C and C-ABI libraries, is in production on the hosted paths
([designing apps with foreign functions](compiler/silica-compiler/tutorials_and_howtos/designing_apps_with_foreign_functions.md)).
Inbound calls, dynamic linking, and brokered IPC are chunk 8 above.
