# ESP32-S3 memory budget — development plan

**Scope:** `ESP32-S3_raw` board bring-up only; not one of the numbered [ROADMAP.md](../../../ROADMAP.md)
enhancement chunks. Tracks the board's real memory ceiling: what claims it today, what a program has left
for its heap, and the trials that currently cannot run because of it.
**Related:** [esp32s3_port_status.md](esp32s3_port_status.md), [esp32s3_xtensa_port.md](esp32s3_xtensa_port.md),
[actor_stack_growth_plan.md](../Phase1_TODOs/actor_stack_growth_plan.md) (its "Still to decide" already flags
"what targets without demand paging (ESP32-S3) do instead" for *actor* stacks; this plan is the parallel
question for `main`'s own fixed reservation, which `stack_policy` does not touch).

## The numbers today

`silica_esp32s3.ld` maps internal SRAM1 (one physical array, seen at the instruction bus `0x4037_8000` and
the data bus `0x3FC8_8000`) as a single `0x61700`-byte window — about **390.75 KiB** usable by this
bare-metal image; the ROM keeps its own stack and data above that, and SRAM0 is left free for it to use as
instruction cache.

Before a single byte of a compiled program's code, `.rodata`, `.data` or `.bss` is placed, fixed runtime
reservations already claim more than half that window. Itemized (verified against source 2026-09-29):

| Reservation | Size | Source |
| --- | --- | --- |
| `main`'s machine stack | 128 KB | `silica_esp32s3.ld:36` (`_silica_stack_size`), top of the SRAM window |
| `main`'s auxiliary stack | 64 KB | `rt_start.S` (`AUX_STACK_SIZE`), in `.bss` |
| Fault stack | 2 KB | `silica_esp32s3.ld` (`_silica_fault_stack_size`) |
| Canonical arena table + count + env pointer | 8.02 KB | `rt_ordering.S` (512 × 16 B table + two 8 B words) |
| Actor-runtime fixed state (per-core blocks, both cores' ACBs, per-core scheduler stacks, locks, free-list heads, pid-registry refs, etc.) | 9.19 KB | `rt_actors.S` `.bss.silica_rt_actors` (`silica_rt_cores` → `silica_rt_region_dump_limit`) |
| Per-core register blocks (×2 cores) | 256 B | `rt_start.S` (`VRG_BLOCK_SIZE=128 × NCORES_MAX=2`) |
| Core 1's boot stack | 2 KB | `rt_start.S` (`CORE1_STACK_SIZE`) — used only until core 1 enters its own scheduler |
| Heap bookkeeping (bump pointer, free-list head, lock) | 12 B | `rt_heap.S` |
| **Total** | **~213.5 KB** | |

That is **~213.5 KB of ~390.75 KB — about 55% of the whole chip's usable SRAM — claimed before a single
byte of a user program's own code, data, or heap exists.** (Earlier notes here rounded this to "192 KB",
counting only `main`'s two stacks; the fuller itemization above is the number to use going forward.)

Whatever is left (~177 KB nominal, shared by code + rodata + data + the rest of `.bss` + the heap) is what a
compiled program actually has to work with. Measured 2026-09-28: **~166–171 KB** of heap remained for a
small trial image (code/data eating the difference from the ~177 KB nominal), but as low as **~18.5 KB**
for a larger one (`generic_modules_addition/two_generic_collections_record_round_trip` and
`.../two_generic_collections_round_trip`: their `.iram0.text` alone is 147 KB, because `wbt_set.silica` is
5,474 lines and `--gc-sections` cannot drop unused functions — each compiled unit is one `.text` section).

Both of `main`'s stacks are guarded by the same two Xtensa hardware data breakpoints as actor stacks,
re-armed at every context switch, so an overflow fails cleanly as `(:explicit, :stack_exhausted)` rather
than corrupting memory. Shrinking either is therefore a safe-to-attempt experiment, not a guess with a
silent-corruption downside — but it should be sized from a measured high-water mark, not cut blind: the
`recursion/` trial suite exists specifically to exercise deep call chains, and 128 KB may already be close
to what the worst case there needs.

## Per-actor and per-supervisor cost

Both an actor's ACB and its stacks come out of the same general heap the rest of the program shares
(`rt_actors.S`, `rt_heap.S`), so every spawn is heap pressure. Verified against source 2026-09-29:

| Item | Size | Source |
| --- | --- | --- |
| ACB (any actor, including a supervisor — the layout reserves the supervision fields, offsets 96–172, whether or not they're used) | 256 B | `rt_actors.S` `ACB_SIZE`, via `silica_rt_alloc_acb` |
| Stacks, default `stack_policy` (reserve 0) | 8 KB machine + 2 KB auxiliary = 10 KB | `rt_actors.S` comment, via `silica_rt_alloc_stack` |
| Stacks, explicit reserve *N* | 3/4 *N* machine + 1/4 *N* auxiliary | `rt_actors.S` |
| Supervisor's child-table row, one per child it has ever held (including ones later removed/terminated) | 88 B | `rt_supervisors.S` `ROW_SIZE`, via `silica_rt_alloc`, chained off the ACB |

So one actor at the default policy costs **~10.25 KB** (256 B ACB + 10 KB stacks); a supervisor costs the
same **per child it has held**, plus 88 B. Worked example, one supervisor spawning one child, both default
policy: 10,496 B (supervisor) + 88 B (its one child-table row) + 10,496 B (the child) = **21,080 B ≈ 20.6
KB** — consistent with (and now more precise than) the ~24 KB figure used earlier for this scenario in
conversation, which also folded in the ~3.5 KB registry actor every `main` injects at startup (a fixed,
program-wide cost, not a per-actor one).

**Recycling caveat.** ACBs, stacks and message nodes are recycled through their own free lists
(`silica_rt_acb_free`, `silica_rt_stack_free`, `silica_rt_node_free`; `rt_actors.S`), never through the
general heap's free list — `silica_rt_free_acb` / `silica_rt_free_stack` only push onto these lists, they
never call `silica_rt_free`. So once carved out of the general heap for an actor, that memory is only ever
reused for another actor of a matching category/size; it is never returned to the general heap even after
every actor has terminated. A program's **peak historical actor count**, not its currently-running count,
sets a floor on heap consumed for the rest of the program's life.

**Found while researching this: a live leak, separate from the one already fixed.**
`silica_rt_child_table_free(acb)` (`rt_supervisors.S:172`, called when a supervisor itself ends) only
zeroes the ACB's `ACB_CT_HEAD`/`ACB_CT_LEN` fields — it never walks the row chain and calls `silica_rt_free`
on each row first, unlike `silica_rt_region_destroy`, which does exactly that for a region's extension
chain. So every child-table row a supervisor ever allocated (88 B each) is orphaned, not freed, when that
supervisor ends. This predates and is independent of the 2026-09-28 `rt_heap.S` rewrite (the comment there
still reads "the heap never frees," left over from before that fix). Tracked as Work item 5 below.

## Work

1. **Instrument.** Measure `main`'s real machine- and auxiliary-stack high-water mark across the trial
   suite (recursion-heavy trials in particular) before changing either fixed size.
2. **Right-size.** Pick a smaller `_silica_stack_size` (`silica_esp32s3.ld:36`) and/or auxiliary stack size
   (`rt_start.S`) from that measurement plus margin, freeing heap for every program image on the board.
3. **Complementary: shrink the image instead of the stacks.** Per-function `.text` sections (or another way
   to make `--gc-sections` effective) would stop an unused-function-heavy unit like `wbt_set.silica` from
   bloating `.iram0.text`, attacking the same budget from the code side without touching stack safety
   margins.
4. **Re-run and settle.** Re-run `two_generic_collections_record_round_trip` and
   `two_generic_collections_round_trip` once the budget opens up. If they still do not fit, add them to
   `trials/targets/ESP32-S3_raw.skip` with the measured shortfall, the same way the trials below were.
5. **Fix the child-table row leak.** `silica_rt_child_table_free` (`rt_supervisors.S:172`) needs to walk the
   `ACB_CT_HEAD` chain and call `silica_rt_free` on each 88-byte row before clearing the anchors, mirroring
   what `silica_rt_region_destroy` already does for a region's extension chain. Until fixed, any program
   that spawns and ends supervisors repeatedly leaks 88 B per child they ever held, independent of and not
   covered by the 2026-09-28 `rt_heap.S` allocator rewrite.
6. **Reduce each supervisor's and actor's memory allocation.** The fixed 8 KB + 2 KB default (`stack_policy`
   reserve 0) is chosen once at spawn and never shrinks (see "Per-actor and per-supervisor cost" above) —
   on a board with ~150–170 KB of heap total, a handful of actors at the default reserve is already a
   meaningful bite out of it. Bring the raw path's actor stacks to parity with the hosted paths' **already
   implemented** design (chunk 1, [actor_stack_growth_plan.md](../Phase1_TODOs/actor_stack_growth_plan.md)),
   through the same public API (`stack_policy(reserve, release)`, the five release algorithms, and
   `get_actor_memory_usage`), not a raw-only variant of it:
   - **Smaller initial allocation.** Start an actor with far less than the current fixed reserve and let it
     grow from there, instead of committing the whole reserve up front.
   - **Growing stack.** Extend a stack in place as it needs more, the way the hosted fault handler grows a
     reservation in chunks, rather than picking one fixed size forever at spawn.
   - **Release.** Give stack memory back (per the actor's `:release_on_return` / `:keep_last_message` /
     `:track_recent_peak` / `:release_when_idle` / `:keep_high_water` policy) instead of only ever returning
     a whole stack, unchanged in size, to the free list when the actor ends.
   - **Open problem, not yet solved here:** the hosted mechanism leans on the MMU — reserve a large
     `PROT_NONE` region up front so growth is a permission change at a fixed address, never a copy. The
     ESP32-S3 raw path has no MMU and no demand paging, so growth can't reuse that trick as-is; whoever
     picks this up needs a bare-metal way to extend a stack without relocating it (addresses embedded in
     saved frames and return addresses can't move). `actor_stack_growth_plan.md`'s own "Still to decide"
     already flags this exact question ("what targets without demand paging (ESP32-S3) do instead") as
     awaiting discussion — this item is that discussion's placeholder so it isn't lost.

## Currently skipped for lack of memory (`trials/targets/ESP32-S3_raw.skip`, added 2026-09-28)

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
trial is rewritten to fit within it.

## Gate

- `main`'s machine- and auxiliary-stack sizes are set from a measured high-water mark, not a guess.
- `two_generic_collections_record_round_trip` and `two_generic_collections_round_trip` either pass on the
  board, or are moved into `ESP32-S3_raw.skip` with a measured reason like the six above.
