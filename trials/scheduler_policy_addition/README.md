# scheduler_policy_addition

Trials for `set_scheduler_policy(policy: atom) -> :ok | :invalid_policy` (`proc[concurrency]`): the
programmer-selectable order in which the ready actors *sharing one core* are dispatched by the
`ESP32-S3_raw` runtime (`compiler/src/emitter/ESP32-S3_raw/board/runtime/rt_actors.S`, design in
[esp32s3_xtensa_port.md §8.10](../../compiler/design_documents/ports/esp32s3_xtensa_port.md)). The
policy never preempts a running actor (switches happen only at the yield points of spec §15.1.2) and
never moves an actor between cores; it chooses which ready actor of a core runs next.

| Policy | What it does |
| --- | --- |
| `:priority_fifo` | the default: the head of the highest-priority non-empty queue (`set_actor_priority`, spec §22.10) |
| `:round_robin` | one flat queue per core, priorities ignored: strict arrival order |
| `:weighted_fair` | deficit round robin over the priority tiers, quanta high 4 / normal 2 / low 1, FIFO within a tier |
| `:lottery` | a weighted random draw with the same weights as tickets (xorshift32 seeded from the cycle counter) |

| Trial | Shows |
| --- | --- |
| `scheduler_policy_names_accepted` | the four names are accepted, another atom is rejected with `:invalid_policy` and leaves the policy unchanged, the policy can be changed at any time, the program keeps running |
| `scheduler_policy_priority_fifo_order` | twelve messages queued to three actors (low / normal / high) on one core before any runs: `H H H H N N N N L L L L` |
| `scheduler_policy_round_robin_order` | the same program under `:round_robin`: `L N H L N H L N H L N H` |
| `scheduler_policy_weighted_fair_order` | the same program under `:weighted_fair`: `H H H H N N L N N L L L` |
| `scheduler_policy_weighted_fair_counts` | completion and per-actor counts under `:weighted_fair` (order-free, multiset on every target) |
| `scheduler_policy_lottery_all_run` | under `:lottery` the program neither crashes nor hangs and every ready actor runs to the end of its mail (no order asserted) |

## Goldens and targets

- The dispatch order is defined only where the runtime owns the ready queues (the board). On a hosted
  target the actors are threads the OS schedules, so the three `*_order` trials print the same lines in
  an order nobody defines. They carry `<stem>.scout.multiset`, and this suite's
  `compare_scout_multiset.sh` reads a golden in two ways: the board's own `<stem>.ESP32-S3_raw.scout`
  in order, line for line; the shared `<stem>.scout` as a multiset of lines (same lines, any order).
  The two files hold the same lines; the per-target one is the reviewed strict expectation.
- `set_scheduler_policy` on a hosted target is a no-op that answers `:ok` for every atom (there is
  nothing to select; as `pin_actor_to_efficiency_core` is a no-op on a board whose cores are alike),
  so `scheduler_policy_names_accepted` has a board golden (`:invalid_policy` / `rejected`) and a host
  golden (`:ok` / `accepted`).
- The order trials move their actors to core 0 (`migrate_actor(ref, 0)`), where `main` runs, because
  `main` never yields while it casts and core 0 dispatches only when `main` suspends in
  `wait_for_exit`; every message is therefore queued before the first dispatch, and the observed order
  is the policy's alone. Actors left on core 1 would start running while `main` was still casting.
- `set_actor_priority`'s level is the atom type `priority_level` (`:low | :normal | :high`, spec §22.10);
  the board emitter maps the program's atoms to the runtime's levels at the call site (2026-09-28;
  before that the atom index itself reached the runtime and every level above 2 was clamped to normal).
- Host `.ascomp` goldens: the host compilers must be rebuilt with the front end that knows
  `set_scheduler_policy` before this suite compiles on a hosted target; the assembly goldens are
  recorded from that reviewed emission, not written by hand. Until then the suite fails to compile on
  the host, which is why it has a directory of its own (a rejected unit aborts a suite's batch compile).

Board run: `make integrate TRIAL_TARGET=ESP32-S3_raw TRIAL_SUITES=scheduler_policy_addition` from
`trials/`.
