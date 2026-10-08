# Actor memory budget: boards and hosted environments — development plan

**Scope:** what one actor, one supervisor and `main` cost in memory, how that cost is cut, and what bounds
the number of actors that can be resident. Two targets share one plan: the `ESP32-S3_raw` board (bring-up;
not one of the numbered [ROADMAP.md](../../../ROADMAP.md) enhancement chunks) and the hosted runtimes
(`apple_silicon_mac`, `linux_aarch64`, `linux_x86_64`). Part I is common; Part II is the board only; Part III
is hosted only. Only what differs is segmented. Covers, for the board, what claims the SRAM today, `main`'s
fixed reservation, the raw path's per-actor and per-supervisor stack sizing and the trials that cannot run
for lack of memory; for hosted, the per-actor cost, what bounds the count and what stands between today and
millions.
[actor_stack_growth_plan.md](../Phase1_TODOs/actor_stack_growth_plan.md) owns the *hosted* growth mechanism
(MMU reservation, fault-driven commit); its "Still to decide" defers "what targets without demand paging
(ESP32-S3) do instead" — §11 of this plan is that answer, and it needs a specification decision (§11.1).
**Related:** [esp32s3_port_status.md](esp32s3_port_status.md), [esp32s3_xtensa_port.md](esp32s3_xtensa_port.md),
[actor_growable_stack_design.md](../actor_growable_stack_design.md).

**Status as of 2026-10-07** (board checked against `board/runtime` that day; hosted against the three hosted
runtimes under `compiler/src/emitter/`). Detail in §5 and §17.

| Item | Board (ESP32-S3) | Hosted |
| --- | --- | --- |
| A child-table row leak | not done | done (not a leak there) |
| B composites copied to the heap | appears done, gate trials unverified | not determined |
| C `run_behaviour` pad | not done | does not apply |
| D per-role stack defaults, `main` sizing | not done | does not apply (reservation + release policy instead) |
| E measure and gate | not done | mechanism done, numbers not measured |
| F stacks allocated on dispatch | not done, needs the spec decision | done in the hosted form (commit on demand, release at the message boundary) |
| G shrink the image | done 2026-10-05 | does not apply |
| H re-run the two generic-collections trials | not done, both still skipped | does not apply |
| I halve the ACB | not done (256 B) | not done (512 B) |
| J registry capacity | not done (240 slots, silent) | not done (240 slots, silent, three registries) |

# Part I: Common to the board and hosted environments

## 1. Goals and targets

**Goal, in priority order:** no corruption; as small as possible; as many actors as possible; feasible with
the runtime as it is. The work list in §5 is ordered by those four, which is why the correctness fix in §3
comes before any size is changed.

| Target | Actors and supervisors resident | Bound |
| --- | --- | --- |
| ESP32-S3 | **1,000**, with heap left for the program's own data | SRAM, 399,104 B (Part II); §11 says which steps the target needs |
| Hosted | **millions** | not memory today; the one-OS-thread-per-actor runtime and address space (Part III) |

The two targets differ in what binds, not in what is wanted: the same ACB, the same per-role stack ideas
and the same registry work serve both, and the same correctness fix (§3) gates both.

## 2. What an actor costs, and recycling

Every spawn costs, on every target: an **actor control block** (ACB, the same structure on every path, with
the supervision fields reserved whether or not they are used), a **machine stack**, an auxiliary stack on the
board, a **child-table row** in its supervisor for every child it has ever held, and allocator overhead. A
supervisor costs the same as an actor once, plus its rows. The registry actor every `main` injects at startup
is a fixed program-wide cost, not per actor. The numbers are per target: §7.1 (board), §14 (hosted).

**Recycling (board).** ACBs, stacks and message nodes go to their own free lists and are never returned to
the general heap, so a program's *peak historical* actor count, not its current one, sets a heap floor for
the rest of its life, and stacks of many different sizes strand memory in the exact-match stack list (§7.1).
Hosted ACBs, rows and message nodes are `calloc`'d and `free`'d individually (§14), so no such floor exists
there.

## 3. Correctness first: composites are passed by stack address, not copied

This is the finding that orders everything else. Verified on the board 2026-09-29; the hosted runtime has
the same structure (§14), see the note at the end of this section.

- **Spawn does not copy the initial state.** The runtime stores the spawner's `X0` (board: `a2`/`a3` via
  `silica_rt_spawn_common`) straight into the ACB's state slot. If the state is a composite, the ACB holds a
  pointer to wherever the spawner built it.
- **The reply is copied as an 8-byte value.** The behaviour-return path copies `reply` and `state` out of
  the result tuple as one 64-bit slot each into the message and the ACB. A composite is a pointer, and the
  pointee is not copied.
- **Composites are built on the stacks.** The emitter places aggregates either in the fixed frame
  (`tuple_flat_memref_near`: "allocated in the fixed frame rather than by moving SP", the machine stack) or
  by `SUB SP` (`tuple_make` "outer SUB SP bytes", the auxiliary stack on the board), depending on the path.
  `tuple_flat_memref_near` is in `term_aggregate_helpers.silica` of every emitter, hosted ones included.

So a supervisor's or worker's state can be a pointer into `main`'s frame or aux slab, and a `call` reply can
be a pointer into the callee's frame region. `supervisors_addition/supervisor_child_tuple_state` (state
`(10, 20)` built in `main`, read by the worker) passes today because `main`'s frame lives for the whole
program, which makes anything built there effectively static. That property does not survive a spawner
whose behaviour returns, and it certainly does not survive freeing a stack (§11 on the board; the release
algorithms of §17 on hosted).

**The fix (item B).** Copy composite state at spawn and composite replies/states at the end of the
behaviour-return path into the heap (board: `silica_rt_alloc`), so that no ACB and no message node ever
points into a stack. Then §11 becomes sound, and on the board the pad shrink of §8.1 (item C) depends on it.

**Hosted note.** The hosted runtime stores the spawner's `X0` and the behaviour's reply/state words the same
way (`stp x21, x20, [x19]` at spawn; the behaviour's result slab read back by the actor loop, both in
`prims_actors_runtime_asm.silica`), and runs behaviours on the actor's own reservation, which a release
algorithm can unmap at the message boundary. The aggregate-return rule in the emitter heap-promotes returned
tuples and records, but it lists actor behaviours (`use_actor_reply_sret_buffer`, reply tuple via `X20`) as
an exclusion. Whether a composite state or reply can dangle on hosted after a release is therefore **not
determined**; it needs the same two trials item B names, run on the hosted paths.

## 4. Sizing rules for a `main` that only starts a root supervisor

**Program shape assumed.** `main` declares the cores' multi-tasking algorithm, starts the application's root
supervisor through the `Supervisor` trait (`spawn_registered_supervisor`, spec §15.4.8.1), and waits. No
helper builds the supervisor: the emitter's per-supervisor start trampoline
(`module_linkage.silica` `format_one_supervisor_trampoline`) calls the module's `init` and hands its
`(flags, children)` to the runtime.

**The bounds in §9 hold only under these constraints, which are design rules, not observations:**

1. **`main` never recurses and never reaches the behaviour-return path.** That is why it is cheap. It also
   means both of `main`'s stacks have an exact compile-time bound (§4.1).
2. **A supervisor has no behaviour of its own.** One that does not drops the message in the loop
   ("a supervisor with no behaviour of its own drops the message") and never pays the board's pad; one that
   does has a floor of ~1.9 KB on the board like any actor.
3. **A supervisor's `init` is a flat child-spec list.** On the board the 816 B bound takes
   `materialize_init_children` (480) as the deeper branch; `init`'s own subtree is estimated at ~176 (its
   frame plus record/list allocation through `silica_rt_alloc`). `init` is user code — one that computes
   rather than declares needs its own measurement.
4. **The runtime sizes stacks per role.** On the board, reserve 0 gives every spawn 8 KB + 2 KB today.
   Supervisor and actor defaults differ (§10); `silica_rt_spawn_common` can tell them apart (`ACB_TRAMP` is
   non-null only for a supervisor), and that is where the per-role default belongs, with an explicit
   `stack_policy` reserve still overriding it. Item D. Hosted reserves per actor from `stack_policy` and
   inherits the spawner's policy for child-spec children (§14), so per-role sizing there is a policy choice,
   not a runtime change.

### 4.1 `main`'s stacks: exact per image, not one board-wide constant

The `recursion/` trial suite and any program that does its work in `main` need far more than the shape
above. One constant cannot serve both. Because `main` does not recurse in the shape above, and its runtime
callees are fixed constants, the compiler can compute `main`'s machine and auxiliary bound per image:
`main`'s SFRAME plus the deepest runtime callee, and the sum of its `SUB SP` reservations. Emit them as
`_silica_main_stack_size` / `_silica_main_aux_size` for the linker script and `rt_start.S` to use. A `main`
the compiler cannot bound (recursion reachable from it) keeps a documented default; trials that recurse in
`main` then either fit that default or go into `ESP32-S3_raw.skip` with a measured reason. Until that is
built, the interim constants are the ones in §10, chosen for the root-supervisor shape only. This is a
board rule: hosted `main` runs on the platform's ordinary stack (the 256 MB trampoline and link flag are
gone), and `main` is not an actor there.

### 4.2 Auxiliary stacks

Only emitted code moves the emitter's `SP`; the hand-written runtime never does. `main`'s auxiliary
consumers in this shape are its own argument slabs — the initial-state tuple (16 B for a pair) and the
`stack_policy` tuple (16 B); atoms and integers travel in registers. Estimated ~32–64 B, so 256 B (192
usable above the guard) is 3–6×; the exact figure is the per-image bound above. A supervisor's has two named
consumers: the sret buffer the trampoline reserves for `init`'s return (`SUB SP, SP, #sret_bytes`) and one
slab per child-spec record literal (eight fields, 64 B) before it is copied to the heap — so **it scales
with the width of the `init` child list**, and a root supervisor declaring many children in one literal is
the case to measure. An actor's behaviour pushes 16–32 B per level that carries a value across a call
(`board/README.md`), so the actor ratio stays at the runtime's 1:4 rather than being cut independently of
the machine stack. (The auxiliary stack is a board construct; the hosted runtime has one stack per actor.)

## 5. Work

In goal order — no corruption, then smaller, then more, then feasible — and in dependency order. Status per
target as of 2026-10-07; board status checked against `board/runtime` that day.

- **A. Fix the child-table row leak.** `silica_rt_child_table_free` (`rt_supervisors.S:172`) walks the
  `ACB_CT_HEAD` chain and calls `silica_rt_free` on each 88-byte row before clearing the anchors, mirroring
  `silica_rt_region_destroy`. Also retire the stale "the heap never frees" comments in `rt_heap.S` and
  `rt_actors.S`. First because it is a few lines and it unskews every measurement after it.
  *Board: **not done** — the function still only clears the anchors. Hosted: **done** —
  `_silica_rt_child_table_free` in `prims_actors_child_table_asm.silica` walks the node chain (`+80` next),
  `free`s each row and clears the anchors; there was no leak to fix.*
- **B. Copy composite state and replies to the heap** (§3): at spawn, for `X0` init; in
  `silica_rt_run_behaviour`, for `reply` and `state`. Add a trial in which the spawner is an actor whose
  behaviour returns before the child reads its state, and one in which a caller reads a composite reply
  after the callee has served another message. The correctness gate for C, D and F.
  *Board: **appears done** — state and message copies are on the heap and released per dispatch via
  `silica_rt_state_release`; its gate trials are unverified. Hosted: **not determined** (§3 hosted note).*
- **C. Shrink the `run_behaviour` pad** from 1,568 to 512 (§8.1). Feasible: `entry`'s immediate is 12 bits
  scaled by 8 (0–32,760), and 1,568 appears at exactly one site. Conditions: record the 192-byte derivation
  at the `entry`, or publish it as an assembler symbol computed from the loop's post-behaviour depth, so a
  later deeper call after the behaviour returns cannot silently undercut it; re-measure after any change to
  `silica_rt_actor_main`'s post-behaviour path or `actor_exit`'s chain; verify on the board with
  `supervisors_addition` and `actors_addition`, looking for wrong replies, not just clean exits — a too-small
  pad does not trip the guard.
  *Board: **not done** — `entry a1, 1568` at `rt_actors.S:1331`. Hosted: does not apply; there is no pad.*
- **D. Per-role defaults and `main`'s sizing policy** (§4, §4.1): supervisor 1,536/512 and actor 2,048/512
  at reserve 0, chosen in `silica_rt_spawn_common` by `ACB_TRAMP`; interim `main` constants 1,024/256; then
  the per-image `main` bound emitted by the compiler, with the default-or-skip rule for a `main` it cannot
  bound.
  *Board: **not done** — `DEFAULT_MACHINE_STACK` 8192 and `DEFAULT_AUX_STACK` 2048; `main` is still a 128 KB
  machine stack (`_silica_stack_size = 0x20000` in `silica_esp32s3.ld`) plus a 64 KB aux stack
  (`AUX_STACK_SIZE 0x10000` in `rt_start.S`). Hosted: does not apply in this form; stacks are reservations
  sized by `stack_policy`, and `main` runs on the platform stack.*
- **E. Measure and gate.** High-water marks for `main`, a supervisor with a wide `init` list, and the
  deepest actor in each trial suite, via `silica_rt_get_actor_memory_usage`; decide the probe store of §8.2;
  set the constants; move any trial that no longer fits into `ESP32-S3_raw.skip` with its measured shortfall.
  *Board: **not done**. Hosted: `get_actor_memory_usage` exists (reserved, committed, retained, high-water);
  no per-actor figures have been taken for this plan: **not measured**.*
- **F. Stacks allocated on dispatch** (§11) — the order-of-magnitude step on the board, last because it
  restructures the actor loop and needs the specification decision in §11.1.
  *Board: **not done** — needs the spec decision. Hosted: the equivalent is implemented (chunk 1, 2026-09-17):
  nothing committed at spawn, growth in the fault handler, release at the message boundary under one of five
  algorithms. What it does not remove is the OS thread per actor (§15).*
- **G. Complementary, any time: shrink the image.** Per-function `.text` sections, or another way to make
  `--gc-sections` effective, so a unit like `wbt_set.silica` stops bloating `.iram0.text`. Independent of
  stack sizes; attacks the ~18.5 KB large-image case that no stack change reaches.
  *Board: **done** 2026-10-05 — per-function sections plus `--gc-sections`. Hosted: does not apply.*
- **H. Re-run `two_generic_collections_record_round_trip` and `two_generic_collections_round_trip`** after D
  and G; if they still do not fit, skip them with the measured shortfall like the six in §12.
  *Board: **not done** — both trials are still skipped. Hosted: does not apply.*
- **I. Halve the ACB**, both targets. Board: to ~128 B by moving the supervision fields (offsets 96–172:
  supervisor, agent type, child row, first child, next sibling, child table, strategy, restart accounting,
  trampoline) into a side block allocated only for supervisors. Required by the board's 1,000 target (§11),
  not merely a candidate. Cost not yet assessed: emitted code reads ACB slots at fixed offsets (the state
  and behaviour pairs at 0 and 8, at least), so the layout change touches the emitter's actor primitives and
  `rt_supervisors.S`, not only `rt_actors.S`. Assess after F, since the two change the same structure.
  Hosted: the ACB is 512 B (384 runtime + 128 stack module, §14) and is `calloc`'d per spawn; at a million
  actors it is 512 MB before anything else, so the same split pays there (derived, not measured), but it is
  not what binds first (§15).
  *Board: **not done** — `ACB_SIZE` 256. Hosted: **not done** — `mov w1, #512` at both `calloc` sites in all
  three hosted runtimes.*
- **J. Registry capacity — a hard cap the memory arithmetic does not see.** Every supervised child is
  registered under its `id` atom (board: `silica_rt_supervisor_spawn_child_row_by_flavor` →
  `silica_rt_register_actor_named`), into one global table of 240 actor_refs indexed by atom index (board:
  `REGISTRY_SLOTS = 240` in `rt_actors.S`, with a second table for `:dangerous`; hosted: the PID registry
  actor's 3,840 B state, `occ[240]` at +0 and actor_refs at +1,920, in `prims_actors_runtime_asm.silica`,
  with a dangerous and a device registry made the same way). The store **silently does nothing** for an atom
  index ≥ 240 (board: `silica_rt_registry_store`; hosted: `cmp x22, #240 / b.ge L_pidreg_insert_ok`, which
  then replies `:ok`) and **silently overwrites** an occupied slot. So with the runtime's ~21 prefix atoms
  (board figure), at most ~219 distinctly named children can be registered program-wide; the rest spawn and
  are supervised through their child rows, but every name-keyed operation on them (`call_supervisor`
  `:get_child`, keyed `call`/`cast`, `kill_abnormal` by name) returns "not found" with no error. On the
  board 240 is also the discriminator between an atom and an ACB pointer in every keyed form, so it can be
  raised only while it stays below the lowest ACB address (≥ `0x3FC8_8000` there, so a few thousand is
  safe); the keyed forms test `cmp x11, #240` in the hosted runtimes too, so the same constraint is likely
  there against heap addresses and is **not checked**. Fix: size each registry per image from the atom-table
  length the emitter already emits (board: 4 B per slot, 1,024 slots × 2 registries = 8 KB), and make an
  out-of-range or duplicate registration fail loudly rather than vanish. Required by both targets' counts
  whenever children have distinct ids. Also check `INGRESS_LIMIT = 1024` (board supervision ingress depth)
  against a mass child failure at that scale.
  *Board: **not done** — `REGISTRY_SLOTS` 240, silent. Hosted: **not done** — 240 slots in three registries
  on all three hosted runtimes.*

## 6. Gate (common)

- A: a supervisor that ends frees every child-table row it held; a spawn/end loop shows no heap growth.
  (Board: to be shown. Hosted: to be shown by the same loop; the source already frees.)
- B: the two new trials in item B pass on the target, and no ACB or message node holds a stack address.
- I and J are gated with each target's own count (§13, §18).
- Each target's gate is in its own part: §13 (board), §18 (hosted).

# Part II: The board (ESP32-S3_raw)

## 7. The numbers today

`silica_esp32s3.ld` maps internal SRAM1 (one physical array, seen at the instruction bus `0x4037_8000` and
the data bus `0x3FC8_8000`) as a single `0x61700`-byte window: **399,104 B, 389.75 KiB** usable by this
bare-metal image. The ROM keeps its own stack and data above that, and SRAM0 is left free for it to use as
instruction cache. The chip has **no demand paging**: nothing is committed lazily, so every byte of every
stack is resident from the moment it is allocated.

Fixed runtime reservations, verified against source 2026-09-29:

| Reservation | Bytes | Source |
| --- | ---: | --- |
| `main`'s machine stack | 131,072 | `silica_esp32s3.ld:36` (`_silica_stack_size`), top of the SRAM window |
| `main`'s auxiliary stack | 65,536 | `rt_start.S` (`AUX_STACK_SIZE`), in `.bss` |
| Fault stack | 2,048 | `silica_esp32s3.ld` (`_silica_fault_stack_size`) |
| Canonical arena table + count + env pointer | 8,208 | `rt_ordering.S` (512 × 16 B table + two 8 B words) |
| Actor-runtime fixed state (per-core blocks, both cores' ACBs, per-core scheduler stacks, locks, free-list heads, pid-registry refs) | 9,410 | `rt_actors.S` `.bss.silica_rt_actors` (`silica_rt_cores` → `silica_rt_region_dump_limit`) |
| Per-core register blocks (×2 cores) | 256 | `rt_start.S` (`VRG_BLOCK_SIZE=128 × NCORES_MAX=2`) |
| Core 1's boot stack | 2,048 | `rt_start.S` (`CORE1_STACK_SIZE`); used only until core 1 enters its scheduler |
| Heap bookkeeping (bump pointer, free-list head, lock) | 12 | `rt_heap.S` |
| **Total** | **218,590 (213.5 KB)** | |

**55% of usable SRAM is claimed before a byte of program code, data or heap exists, and 90% of that is
`main`'s two stacks.** What remains (~176 KB nominal) holds code, rodata, data, the rest of `.bss` and the
heap. Measured 2026-09-28: **~170 KB of heap** (174,080 B is the figure used below) for a small trial image,
and as little as **~18.5 KB** for a large one (`generic_modules_addition/two_generic_collections_record_round_trip`
and `.../two_generic_collections_round_trip`: `.iram0.text` alone is 147 KB, because `wbt_set.silica` is
5,474 lines and `--gc-sections` could not drop unused functions while each compiled unit was one `.text`
section; per-function sections landed 2026-10-05, item G, and these two have not been re-measured since).

**Failure modes.** Every stack — `main`'s two, each actor's two — ends in a 64-byte guard band watched by
one of the two Xtensa data breakpoints, re-armed at every context switch for the context about to run. A
store into the band is a debug exception (cause 1006). In an actor it ends only that actor with
`(:explicit, :stack_exhausted)` (`rt_actors.S`); in `main` it is a fatal fault, exit status 70
(`board/README.md`, `esp32s3_port_status.md`). Neither corrupts memory, so a stack that turns out too small
fails loudly. The one gap is §8.2.

### 7.1 Per-actor and per-supervisor cost

An actor's ACB and both its stacks come from the general heap (`rt_actors.S`, `rt_heap.S`), so every spawn
is heap pressure. Verified 2026-09-29:

| Item | Bytes | Source |
| --- | ---: | --- |
| ACB — any actor, including a supervisor; the layout reserves the supervision fields (offsets 96–172) whether or not they are used | 256 | `rt_actors.S` `ACB_SIZE`, via `silica_rt_alloc_acb` |
| Stacks at `stack_policy` reserve 0: 8,192 machine + 2,048 auxiliary | 10,240 | `rt_actors.S`, via `silica_rt_alloc_stack` |
| Stacks at explicit reserve *N*: 3/4 *N* machine + 1/4 *N* auxiliary (at least 1 KB / 512 B) | *N* | `rt_actors.S` |
| Allocator overhead per stack block: 16 B header + up to 63 B of 64-byte alignment slack | ≤79 | `silica_rt_alloc_stack` |
| Supervisor child-table row, one per child it has *ever* held | 88 | `rt_supervisors.S` `ROW_SIZE`, via `silica_rt_alloc`, chained off the ACB |

One actor at the default policy: 256 + 10,240 + ~158 = **~10,654 B**. A supervisor costs the same once,
plus 88 B per child. One supervisor with one child, both at the default: 10,654 + 88 + 10,654 = **21,396 B**.
The registry actor every `main` injects at startup (~3.5 KB) is a fixed program-wide cost, not per actor.

**Recycling.** ACBs, stacks and message nodes go to their own free lists (`silica_rt_acb_free`,
`silica_rt_stack_free`, `silica_rt_node_free`) and are never returned to the general heap —
`silica_rt_free_acb` / `silica_rt_free_stack` only push. `silica_rt_alloc_stack` reuses a free block only on
an **exact** size match (`beq` on the header's size word). Two consequences: a program's *peak historical*
actor count, not its current one, sets a heap floor for the rest of its life; and any scheme that produces
stacks of many different sizes strands memory in that list, so sizes should be quantized to a few classes.

**Leak (item A).** `silica_rt_child_table_free` (`rt_supervisors.S:172`, run when a supervisor ends) zeroes
`ACB_CT_HEAD`/`ACB_CT_LEN` without walking the row chain and freeing each 88-byte row, unlike
`silica_rt_region_destroy`, which does exactly that for a region's extension chain. Every row a supervisor
ever held is orphaned when it ends. Independent of the 2026-09-28 `rt_heap.S` rewrite (which made `free`
real); the comment "the heap never frees" is stale in both `rt_heap.S` and `rt_actors.S`. Still open as of
2026-10-07.

## 8. Board correctness details

The mechanism of §3 on the board, and what the `run_behaviour` pad does and does not cover. Verified
2026-09-29.

- `silica_rt_spawn_common` stores the spawner's `X0` straight into `ACB_STATE` (`s32i a3, a2, ACB_STATE`).
- `silica_rt_run_behaviour` copies `reply` and `state` out of the result tuple as one 64-bit slot each into
  `MSG_REPLY` and `ACB_STATE`.

### 8.1 What the 1.5 KB pad in `silica_rt_run_behaviour` does, and does not do

`silica_rt_run_behaviour` opens `entry a1, 1568` (`rt_actors.S:1331`). Its stated job: the result may point
into the behaviour's frame region just below this frame, so the loop frames built after the behaviour returns
must fit *inside* the pad and leave that region intact. That protects fixed-frame composites on the machine
stack until the actor yields — and nothing else. `SUB SP` composites on the auxiliary stack are outside its
reach at any size, and once the actor is rescheduled and runs another behaviour, the next `run_behaviour`
descends over the region whatever the pad is. The pad is a partial mitigation of the problem in §3, not a
solution.

Measured, its job needs **192 bytes**: everything `silica_rt_actor_main` calls after
`call8 silica_rt_run_behaviour` returns, deepest first —

| Call | Depth |
| --- | ---: |
| `silica_rt_actor_exit` → `deliver_unwind_report` → `actor_cast` → `alloc_node` → `alloc` → `heap_take` | **192** |
| `silica_rt_yield_to_sched` → `ctx_switch` → `console_release_if_mine` | 96 |
| `silica_rt_wake` → `rq_push` | 64 |
| `silica_rt_free_node`, `silica_rt_rq_push`, `silica_rt_free` | 32 |

`yield_to_sched` switches to the scheduler's own stack, so nothing the scheduler does is charged here. The
pad is about 8× its requirement, and it is the single largest per-actor cost: 1,568 of an actor's 1,856 B
of fixed overhead (§9). With item B done the pad has no job left except covering its own loop's
post-behaviour frames, which is what item C then shrinks it to.

### 8.2 A guard-band gap that smaller stacks make more likely

The breakpoint fires on a store *into* the 64-byte band. `entry` only subtracts; window-overflow spills go
to the top of the callee's frame; outgoing-argument homes are at the frame's bottom but only when a
function passes more than three arguments. A frame or slab larger than 64 B whose stores all land above the
band can therefore straddle it and write below the block unseen. `silica_rt_run_behaviour`'s 1,568-byte
frame stores nothing near its bottom, so with a 2 KB stack it is the likeliest frame to do this. Shrinking
the pad (item C) reduces the exposure; a probe store at the lowest address of any frame or slab larger than
the band, emitted in the prologue or at the `SUB SP`, removes it. Recorded here so the smaller sizes below
are not adopted without it being decided (item E).

## 9. Static sizing bounds (board)

The rules these bounds rest on are in §4; this is the board's arithmetic.

**Method.** A static call-graph walk over `board/runtime/*.S`: sum `entry a1, N` along `call8` edges and
take the maximum over branches. Fallthrough labels with no `entry` of their own (`silica_rt_actor_spawn_linked`
falls into `silica_rt_actor_spawn_dangerous_linked`) resolve into their successor. The walk cannot follow
`callx8` into emitted code — the behaviour, a supervisor's `init` — so those are estimated and marked; it
does not count libgcc frames for `float64` arithmetic or 64-bit division; and the maximum it reports is
often a failure branch (the spawn paths bottom out in `spawn_reserve_failed` → `actor_trap` → …), which is
conservative. Every figure here is a bound to be confirmed on the board with the high-water scan in
`silica_rt_get_actor_memory_usage` (lowest non-zero word of the zeroed block), item E.

| Context | Decomposition | Bound |
| --- | --- | ---: |
| `main` | `_start` + `silica_rt_start_windowed` 64 + emitted `main` SFRAME ~112 (est.) + `silica_rt_registry_spawn` 416 + guard 64 | **~656 B** |
| Supervisor | `ctx_prepare` frames 144 + `silica_rt_actor_main` 80 + trampoline SFRAME 48 (fixed, `supervisor_trampoline_frame_header`) + `silica_rt_supervisor_materialize_init_children` 480 + guard 64 | **816 B** |
| Actor, fixed overhead only | `ctx_prepare` 144 + `silica_rt_actor_main` 80 + `silica_rt_run_behaviour` 1,568 + guard 64 | **1,856 B** |
| Actor, fixed overhead, pad at 512 | 144 + 80 + 512 + 64 | **800 B** |

Other things `main` does are shallower: `wait_for_exit` 240, `actor_spawn` 368, `get_cpu_topology` 176,
`set_scheduler_policy` 32. The exit path after `main` returns (`silica_rt_exit` chain, 208) is a sibling at
the same depth, not additive.

## 10. Proposed sizes, and what they buy (board)

Interim constants for the root-supervisor shape, with the pad at 512 B (item C done) and actor stacks at
the runtime's 1:4 ratio. Sizes are multiples of 512 B so the exact-match free list has few classes.

**Fixed reservations.**

| Reservation | Now | Proposed | Source |
| --- | ---: | ---: | --- |
| `main` machine stack | 131,072 | **1,024** | `silica_esp32s3.ld:36` |
| `main` auxiliary stack | 65,536 | **256** | `rt_start.S` `AUX_STACK_SIZE` |
| Everything else in §7 | 21,982 | 21,982 | |
| **Total** | **218,590 (213.5 KB)** | **23,262 (22.7 KB)** | |

**Freed: 195,328 B, all of it `main`'s.** Heap for a small image goes from ~174,080 to **~369,408 B (360.75 KB)**.

**Per entity.**

| Item | Now | Proposed |
| --- | ---: | ---: |
| `main` (machine + auxiliary) | **196,608** | **1,280** |
| Supervisor: ACB 256 + machine + auxiliary + overhead ~158 | 256 + 8,192 + 2,048 + 158 = **10,654** | 256 + 1,536 + 512 + 158 = **2,462** |
| Actor: ACB 256 + machine + auxiliary + overhead ~158 | 256 + 8,192 + 2,048 + 158 = **10,654** | 256 + 2,048 + 512 + 158 = **2,974** |
| Child-table row, per child ever held | 88 | 88 |

**Margin.**

| Stack | Bound | Proposed | Margin |
| --- | ---: | ---: | --- |
| `main` machine | ~656 | 1,024 | 1.6× |
| `main` auxiliary | ~64 (est.) | 256 | 192 usable, 3–6× |
| Supervisor machine | 816 | 1,536 | 1.9× |
| Actor machine, pad at 512 | 800 fixed | 2,048 | 1,248 B for the behaviour |

Without item C the actor's fixed overhead is 1,856 B and 2 KB leaves under 200 B for user code; the
honest figure would be 4 KB and the actor row 4,822. The pad shrink is worth 1,056 B on every actor, which
is why it precedes sizing.

**Capacity: one root supervisor and N workers, nothing else on the heap.**

| | Now | Proposed |
| --- | ---: | ---: |
| Heap for a small image | 174,080 | 369,408 |
| Root supervisor | 10,654 | 2,462 |
| Per worker, with its child-table row | 10,742 | 3,062 |
| **Workers that fit** | **15** | **119** |

Ceilings: they assume no messages, lists, strings or records on the heap, and §7.1's free lists make the
count a peak-historical one. The ratio, **~8×**, is the result to quote.

## 11. Stacks allocated on dispatch, not at spawn (item F, board)

Everything in §10 still gives every actor a stack for its whole life. The alternative: an actor is spawned
with **no stack**, gets one when dispatched, and gives it back at the dispatch boundary when its behaviour
returns. The stack never moves — no growth, no relocation, none of the hosted design's MMU tricks — it is
simply not resident while the actor is idle. This is the bare-metal answer to the question
`actor_stack_growth_plan.md` defers. (Hosted gets the same effect from demand paging, §14.)

It replaces one "actors that fit" number with two independent quantities:

- **Resident cost per actor** — ACB 256 B, plus an 88 B child-table row for a supervised one = **344 B**.
  This bounds how many actors can *exist*.
- **Stack pool** — *K* stacks, *K* being the actors holding one at the same instant.

*K* is small because the scheduler is not preemptive: an actor runs a behaviour and yields at the dispatch
boundary. **K = NCORES_RUN (2) + the actors suspended mid-behaviour** — blocked in `call` awaiting a reply,
the one suspension with live user frames (model comment at the head of `rt_actors.S`). `wait_for_exit` and
an empty mailbox suspend at a boundary with nothing live.

**Totals**, same heap and scenario as §10, stack unit 2,048 + 512 + 158 = 2,718 B (pad at 512):

| Model | Stack pool | Resident per worker | **Workers that fit** | Heap left at 1,000 |
| --- | ---: | ---: | ---: | ---: |
| Fixed stacks, §10 | — | 3,062 | **119** | — |
| On demand, K = 2, ACB 256 | 5,436 | 344 | **1,057** | 19,716 |
| On demand, K = 8, ACB 256 | 21,744 | 344 | **1,009** | 3,408 |
| On demand, K = 16, ACB 256 | 43,488 | 344 | **946** | — |
| On demand, K = 2, ACB 128 | 5,436 | 216 | **1,684** | 147,844 |
| On demand, K = 8, ACB 128 | 21,744 | 216 | **1,608** | 131,536 |
| On demand, K = 16, ACB 128 | 43,488 | 216 | **1,508** | 109,792 |

Under this model the pad and the stack sizes stop being the lever — K = 8 with the pad at 1,568 costs
~48 workers, not 1,056 B each — and **the ACB's 256 B becomes the binding cost.**

**What the 1,000 target needs, read off that table.** Fixed stacks cannot reach it at any size: 119 is the
ceiling with every stack already at its bound. On-demand stacks with the ACB at 256 B *touch* 1,000 only as
a ceiling — at K = 8 there are 3,408 B left for every message node (48 B each), mailbox, state record and
string the program owns, so it is not a working configuration. **1,000 as a working number needs F and the
ACB halving (item I) together**, plus the registry resized (item J) if the children carry distinct ids:
at K = 8 that leaves ~131 KB for the program, and holds K up to 16 with ~110 KB. The program-side condition
is K itself — a `cast`/`send` design; a supervision tree that `call`s
down through its depth pins one stack per level and spends the pool.

### 11.1 What it needs

1. **A specification decision.** `actor_stack_growth_plan.md` quotes §15.1.2.2: nothing committed at
   spawn, no per-actor maximum, a reservation that cannot be made reported as
   `(:explicit, :stack_reserve_failed)` on the spawner. A fixed pool is a program-wide cap on concurrently
   running actors, and reserving pool *slots* at spawn (the way to keep the spawner-reports-failure
   contract) is a raw-only mechanism. Item 6 of the earlier draft insisted on "the same public API, not a
   raw-only variant"; the specification wins where they differ, so §15.1.2.2 has to say what a target
   without demand paging may do before this is built.
2. **Item B done.** Releasing a stack at the boundary recycles the behaviour's frame region immediately; a
   reply or state pointing into it is corrupted, not merely at risk.
3. **The release is done by the scheduler, after the switch.** At the boundary the actor is executing
   `silica_rt_actor_main` → `yield_to_sched` → `ctx_switch` on the stack to be freed. And `actor_main`'s loop
   is a frame on that stack, so the actor stops being a resumable coroutine and is re-entered fresh through
   `silica_rt_ctx_prepare` per message. The state already lives in `ACB_STATE`, and the first-entry flags
   (`ACB_TRAMP_RAN`) already distinguish first from subsequent entries, so re-entry fits the model — but it
   restructures the loop rather than tuning it.
4. **K validated against real programs.** `cast`/`send` message passing holds K near 2; deep synchronous
   `call` chains through a supervision tree push K toward N and recover the fixed-stack numbers. The board's
   intended concurrency style decides whether this step is worth its cost.
5. **A per-dispatch cost decision.** `silica_rt_alloc_stack` zeroes every block it hands out
   (`silica_rt_zero_block`), which is how `get_actor_memory_usage` finds the high-water mark. Zeroing 2 KB
   per message is real time at 240 MHz; a pre-zeroed pool of one size class makes allocate/release O(1)
   pointer swaps, and high water then needs another measurement. `get_actor_memory_usage`'s four fields
   (spec §22.4) also need defined meanings for an idle actor with no stack.

### 11.2 Staged counts toward the target

What each stage of §5 allows, from the numbers above: one root supervisor plus N actors and supervisors
(a supervisor costs the same as a worker at rest — ACB plus its child-table row — so the tree's shape does
not change the count), K ≤ 8 throughout.

| Stage | Items | What binds | Actors + supervisors |
| --- | --- | --- | ---: |
| Today | — | 10 KB fixed stacks; `main` holding 192 KB | **15** |
| Stacks right-sized | A–E | 2.5–3 KB fixed stacks in a ~369 KB heap | **119** (~74 without C) |
| Stacks on demand | + F | the registry: ~219 distinctly named children (item J); heap ceiling 1,009 with 3 KB left | **~219 named**, or a 1,009 ceiling that is not workable |
| Registry resized | + J | the ACB at 256 B | **~900** working (~40 KB left for program data); 1,009 ceiling |
| ACB halved | + I | program data | **1,000 with ~131 KB free**; 1,608 ceiling |

The jump from 119 to ~900 is all F, which is the step that restructures the actor loop and needs §11.1's
decision; everything before it is sizing and correctness, worth doing regardless, and tops out at 119. J
is cheap and independent (8 KB of tables and a loud failure) and need not wait for F — done with D, the F
plateau is ~900 at once; it binds only when children carry distinct ids. I is what turns 1,000 from a
ceiling into a working number. The ~369 KB heap in every row is the small-image figure; a program carrying
1,000 actors' worth of behaviour code takes more of the same SRAM, which is why item G keeps the last
column honest at scale.

## 12. Currently skipped for lack of memory (`trials/targets/ESP32-S3_raw.skip`, added 2026-09-28)

Six trials are capacity-bound on this board, confirmed against the numbers above, and are skipped rather
than failing:

| Trial | Needs | Have |
| --- | --- | --- |
| `memory_region_addition/region_chain_free_walked` | three 1.6 MB buffers (200000 int64) per region | ~170 KB heap |
| `memory_region_addition/region_move_conditional` | a 1.6 MB buffer | ~170 KB heap |
| `memory_region_addition/region_move_pass_and_return` | a 1.6 MB buffer | ~170 KB heap |
| `memory_region_addition/region_move_pass_no_return` | a 1.6 MB buffer | ~170 KB heap |
| `memory_region_addition/region_scope_loop_release` | a 1.6 MB buffer | ~170 KB heap |
| `compiler_addition/substring_utf8_cursor` | ~940 KB of never-freed intermediate strings | ~170 KB heap |

All six are shortfalls against real board capacity, not defects: `rt_heap.S` became a real first-fit,
coalescing allocator on 2026-09-28 (previously `free`/`region_destroy` were permanent no-ops), which is what
let `region_local_release` (a much smaller working set) start passing. This image uses no PSRAM, so the
ceiling does not move on its own; revisit only if the board's usable memory budget changes materially, or a
trial is rewritten to fit within it. None of them is reached by any stack change in this plan. The two
generic-collections trials of item H are skipped on the same ground and are re-run after D and G.

## 13. Gate (board)

- A, B: §6.
- C: the pad's derivation is recorded at its `entry`, and `supervisors_addition` / `actors_addition` pass
  on the board with correct replies.
- D/E: `main`'s, supervisors' and actors' sizes are set from board-measured high-water marks with the
  margins in §10 or better; the §8.2 probe is decided and recorded; every trial either passes or is in
  `ESP32-S3_raw.skip` with a measured shortfall.
- F: not gated until §11.1's specification decision exists.
- F + I + J, the target: 1,000 actors and supervisors with distinct ids spawned, each registered and each
  dispatched at least once on the board, with ≥100 KB of heap still free (`get_actor_memory_usage` and the
  heap's free total), at K ≤ 8; a name-keyed lookup of the 1,000th resolves.
- H: `two_generic_collections_record_round_trip` and `two_generic_collections_round_trip` pass, or are
  skipped with a measured reason like the six in §12.

# Part III: Hosted environments (`apple_silicon_mac`, `linux_aarch64`, `linux_x86_64`)

Sources: [actor_stack_growth_plan.md](../Phase1_TODOs/actor_stack_growth_plan.md),
[actor_growable_stack_design.md](../actor_growable_stack_design.md), and
`emitter/<target>/terms/prims/prims_actors_runtime_asm.silica`, `prims_actors_stack_asm.silica` and
`prims_actors_child_table_asm.silica`, read 2026-10-07. Figures verified on `apple_silicon_mac`; where a
Linux runtime differs it is said. Anything not in the source or those documents is marked **not measured**.

## 14. The numbers today

Hosted memory is virtual and demand-paged: a reservation costs address space, not RAM, until touched. So
the per-actor cost splits into **resident** (RAM and kernel objects) and **virtual** (address space and
mappings).

| Item | Size | Source |
| --- | ---: | --- |
| ACB, `calloc`'d per spawn (at two spawn sites); 0–383 runtime, 384–511 stack module | 512 B | `prims_actors_runtime_asm.silica` `mov w1, #512`; `mov esi, 512` on x86-64 |
| ACB stack-module fields: base 384, reserve 392, committed 400, high water 408, next chunk 416, algorithm 424, retained 432, 32-entry usage ring 440, ring index 448, last message time 456, alt stack 464, policy 472, saved SP 480, lowered 488, exit reason 496 | within the 512 B | `prims_actors_stack_asm.silica` header |
| Stack reservation, virtual, one per actor | default: physical memory + swap, page-rounded (fallback 16 GiB if the sysctl read fails, Apple Silicon); `stack_policy(reserve, …)` lowers it | `_silica_rt_stack_runtime_init`; spec §15.1.2.2 |
| Stack committed at spawn | 0 | growth plan |
| First growth chunk | one platform page: 16 KB on Apple Silicon and Pi 5, 4 KB on most other Linux; then doubling per fault to 1 MB, then 1 MB per fault | design doc "Runtime constants" |
| Lowest page of every reservation | never committed (exhaustion detector) | growth plan, status |
| Release | anonymous `MAP_FIXED` remap to `PROT_NONE` at the message boundary, down to what the algorithm retains (five algorithms) | `prims_actors_stack_asm.silica` |
| Per-actor OS thread (the runtime is one thread per actor) | pthread stack 128 KB (`pthread_attr_setstacksize`, `0x20000`), carrying the mailbox loop; behaviours run on the reservation (SP switched around each `blr`) | `prims_actors_runtime_asm.silica`; growth plan |
| Per-thread signal alt stack | 64 KB `mmap` (virtual), freed with a full release | `prims_actors_stack_asm.silica` |
| Implicit heap (arena) per actor | a chain of 64 MB anonymous RW blocks mapped on first allocation; an idle actor holds none; freed together when the actor ends | `_silica_rt_region_alloc`, `_silica_rt_actor_dealloc_region` |
| Mappings per actor, stack | 2 (committed run, inaccessible remainder), plus the thread stack, the alt stack and any arena blocks | growth plan; counts beyond the 2 **not measured** |
| Child-table row, one per child held, `calloc`'d, freed on supervisor end | 88 B | `prims_actors_child_table_asm.silica` `mov w1, #88` |
| Message node | 152 B (heap) | `prims_actors_runtime_asm.silica` header comment |
| PID registry actor state, one each for the pid, dangerous and device registries | 3,840 B (`occ[240]` at +0, actor_refs at +1,920) | `_silica_rt_pid_registry_beh` |
| Resident RAM per idle actor (ACB, kernel thread, touched thread stack and TLS, first committed page) | **not measured** | |
| Kernel thread cost per actor | **not measured** | |

**Derived, not measured.** At 512 B the ACBs of a million actors are 512 MB (plus 88 B per supervised child).
A million 64 MB arena blocks, were every actor to allocate once, would be 64 TB of address space; the default
reservation at, say, 16 GB of memory plus swap would be 16 PB for a million actors, which no 47- or 48-bit
address space holds. So a program with many actors must lower every reservation, as the design document's
example does (`stack_policy(33554432, :release_on_return)`, 32 MB, which is 32 TB at a million). The address
width of the hosts is **not measured here**.

**Recycling.** ACBs, rows and message nodes are `calloc`'d and `free`'d individually, and a dying actor
unmaps its stack and its alt stack and returns its arena blocks (`_silica_rt_actor_fail_current` frees the
stack before it wakes any waiter). There is no peak-historical floor of the board's kind and no
exact-size-match list.

**Composite state.** See §3, hosted note: not determined.

**Registry.** The registry is an actor, and each operation allocates a 24 B and a 16 B reply block with
`calloc`. Insert with an atom index outside 0..239 replies `:ok` and stores nothing, lookup answers
`:not_found`, and an occupied slot is overwritten. The same 240 slots and the same silent failures as the
board's (item J), in three tables. The `cmp #240` atom-versus-pointer discrimination in the keyed forms is
present in the Linux AArch64 runtime as well.

## 15. What bounds the count

None of these is the amount of RAM taken by stacks, which demand paging and release have removed as the
first limit. In the order they would be met:

1. **One OS thread per actor.** The host's thread limit is the bound, not the stacks. The growth plan says so
   in terms: "a million idle actors cannot be reached with one OS thread per actor", and the gate trial
   (`trials/actor_stacks_addition/stack_many_idle_actors_hold_nothing`) spawns **a thousand** with lowered
   reservations and checks that every one holds zero committed stack. The thread limit of the hosts is **not
   measured**; a thousand is the highest count verified. Chunk 2 of the ROADMAP replaces this with one
   carrier thread per logical core and many actors per carrier, switching cooperatively at yield points
   (the BEAM pattern); until then the hosted gate "a million idle actors holding no stack memory" is open.
2. **Mappings.** Two per actor for the reservation alone, plus the thread's own stack and the alt stack.
   The growth plan records `vm.max_map_count` at about one million on the Pi image and 65,530 on stock
   Linux kernels, so two mappings per actor reaches the limit at half of those counts: about 32,000 actors
   on a stock Linux kernel from the reservation mappings alone, fewer with the other mappings. Whether the
   runtime documents or checks the limit at start is still undecided ("Still to decide" in the growth plan).
   macOS has no such limit that was measured here. The mappings per actor under carriers are **not measured**.
3. **Address space** of the reservations and the arena (§14, derived): bounded only by lowering the default
   reservation per actor; a program with millions of actors must pass a small `stack_policy` reserve to every
   spawn, and child-spec children inherit their supervisor's.
4. **The registry** (item J): 240 slots in each of three tables, silent. Binds at ~219 distinctly named
   children, long before millions, whenever children carry distinct ids; actors that are not registered by
   name are unaffected.
5. **The ACB** (item I): 512 B each, `calloc`'d, resident. Not the first limit, but at a million actors it is
   half a gigabyte before a single message exists.
6. **Process-level stack growth** is not a limit: the fault handler commits one chunk at a time under every
   release algorithm and the default reservation exceeds what the machine can commit.

## 16. Path to millions

Counts are only given where verified; every other cell is **not measured**.

| Stage | What changes | What binds | Actors |
| --- | --- | --- | ---: |
| Today (chunk 1, 2026-09-18) | Reservation, fault-driven growth, five release algorithms; thread per actor | the host's thread limit; mappings; 240-slot registry for named children | **1,000** verified (the gate trial); beyond that not measured |
| Registry resized (J) | per-image table size, loud failure | thread limit | no change to the count of unnamed actors; named children no longer capped at ~219 |
| Carriers (ROADMAP chunk 2, hosted part) | one carrier thread per logical core, many actors per carrier; no thread stack or alt stack per actor | mappings, address space, ACB | not measured |
| Lower reservations by default for many-actor programs | spawn names a small `stack_policy` reserve; the mapping count falls to what one actor needs | ACB and message memory | not measured |
| ACB split (I) | supervision fields moved to a side block | message and program data | not measured |

What stands between today and millions, in order: the carriers (the thread per actor), the mapping count
and address space per actor once there are no threads, the registry, and the ACB. The stack mechanism itself
is already in the form millions needs: nothing committed at spawn and release at the message boundary.

## 17. Hosted status per work item

Checked from the hosted runtime source on 2026-10-07; none of the hosted trials were re-run for this plan.

| Item | Status | Basis |
| --- | --- | --- |
| A | done | `_silica_rt_child_table_free` walks the chain and `free`s each 88 B row (`prims_actors_child_table_asm.silica`) |
| B | not determined | raw word stores at spawn and for the behaviour result; aggregate-return promotion excludes actor behaviours (§3); the item B trials have not been run on hosted |
| C | does not apply | no pad; behaviours run on the reservation |
| D | does not apply in the board form | reserve and release policy come from `stack_policy`; children inherit the spawner's policy (the child-spec record has no policy field in the specification); `main` runs on the platform stack |
| E | mechanism done, numbers not measured | `get_actor_memory_usage` returns reserved, committed, retained, high-water; measurement per suite is open |
| F | done in the hosted form | nothing committed at spawn; release at the message boundary; five algorithms; `trials/actor_stacks_addition` (13 trials) and `error_enforcement_addition/stack_policy_*` |
| G | does not apply | |
| H | does not apply | |
| I | not done | ACB 512 B |
| J | not done | 240 slots × 3 registries, silent |

## 18. Gate (hosted)

- A, B: §6. A is already satisfied by the source; B needs the two trials run on each hosted path.
- The existing chunk-1 gate stays: deep recursion past 8 MB in an actor, shrink after a message under each
  algorithm, a lowered reservation reporting `(:explicit, :stack_exhausted)`, `:stack_reserve_failed` on an
  actor and a reported exit from `main`, and a foreign call from deep inside a grown stack.
- J: a registration beyond the table fails loudly; a name-keyed lookup of the last registered child resolves.
- The target, millions: not gated until carriers exist. Then, in stages with each count measured on the host
  before it is claimed (a thousand today, then ten thousand, a hundred thousand, a million): every actor
  spawned with a lowered reservation, dispatched at least once, holding zero committed stack while idle, with
  resident memory per actor recorded (`get_actor_memory_usage` and the process's RSS), and the mapping count
  against `vm.max_map_count` recorded on Linux.
