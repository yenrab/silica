# Actor Placement and Balancing: Striping and Moving Options

**Status:** Proposal for Lee's review. First draft 2026-09-19; revised 2026-09-19 after Lee's review (see the revision
note below). Nothing here is implemented, and nothing here is normative.
**Roadmap:** this design belongs to [chunk 2](../../ROADMAP.md), actor placement, pinning and cooperative multitasking,
which on hosted paths follows the BEAM pattern (one carrier thread per logical core, many actors per carrier, switching
at yield points) and may pull forward whatever part of chunk 12's scheduler it needs.
**Authority:** [silica-specification.md](silica-specification.md) wins wherever this document and the specification
differ. The specification changes this design would need are collected in [Appendix A](#appendix-a-proposed-specification-changes),
marked as proposals.
**Related:** spec §4.1.7 (atoms), §15.1.1 (spawn forms), §15.1.2 (Actor Pinning Policy), §15.1.2.2 (actor stacks),
§15.1.2.4.1 (message delivery during migration), §15.4.8, §15.4.9 and §15.4.13 (supervisors, supervision ingress,
the `Supervisor` trait), §15.5.6 (`state_machine_behavior`), §20.3.1 (the registry), §22.10 (placement and migration
API), §23.1.1 and §23.1.3 (scheduler); [actor_growable_stack_design.md](actor_growable_stack_design.md) §4.2.1, §4.3,
§5.3; [actor_spawn_core_affinity_os_semantics.md](actor_spawn_core_affinity_os_semantics.md);
[silica_ffi_wrapper_specification.md](silica_ffi_wrapper_specification.md) §4.9 (split registries);
[ROADMAP.md](../../../ROADMAP.md) chunks 2, 11 and 12.

> **Revision of 2026-09-19 (after Lee's review).** What changed from the first draft:
>
> 1. **Spawn keeps its shape.** The first draft's `placement(group, key)` spawn argument is gone, as are the idea of
>    making the last spawn argument required, and every other change to spawn forms or child specifications. An actor
>    is spawned exactly as today and then **handed off** to placement by a message (§7.7). A supervisor's children reach
>    placement through a hand-off of the supervisor (§7.8).
> 2. **One realized trait, one placement actor, per application.** Placement groups are gone. Exactly one
>    `impl … for Placement` may exist in an application, and it governs every actor handed to it. Per-actor variation
>    (a key, whether the actor may move) travels in the hand-off (§7.4, §7.7).
> 3. **Placement is a specialized actor, like a supervisor.** The placement actor has a runtime-owned behaviour on
>    hosted targets and a library-provided behaviour on raw targets; its `init/1` sets the required options when the
>    application starts it (§7.1, §7.6).
> 4. **A claimed name.** The placement actor is registered under the predefined atom `:placement`; the compiler rejects
>    that atom on every other registration path, and rejects a second placement actor (§7.5, §7.15).
> 5. **Options are fixed at startup.** Changing them while running (the first draft's `:set_cores`, `:hold_moves`,
>    `:resume_moves`) is a future enhancement (§12). The running surface is the hand-off, release, and read-only queries.
> 6. The ten named options, the no-defaults rule, the BEAM comparison, the raw library, and the benchmark and trial plans
>    are kept and updated for the above.

---

## Summary

Silica pins every actor to a core from spawn to termination, and the runtime never moves an actor on its own
(spec §15.1.2). That rule is exactly right for OS-free targets, where pinning is exclusive and a program balances
load itself with `migrate_actor`. On OS-hosted targets it leaves the programmer with nothing: a hosted program cannot
write a real core balancer, because the OS owns the cores, and the language's promise of thousands or millions of
actors per machine (§15.1.2.2 "Scalability") needs the runtime to spread and even out that load, as the BEAM does
between its scheduler threads.

This document proposes to give the programmer **named options** for that, in the way §15.1.2.2 gave five named
release algorithms for actor stacks:

- **Five striping options** decide the core a handed-off actor starts on: `:round_robin`, `:least_loaded`,
  `:with_spawner`, `:by_key`, `:fill_in_order`.
- **Five moving options** decide whether and when placed actors later move between cores: `:never`,
  `:steal_when_idle`, `:balance_periodically`, `:compact_when_quiet`, `:follow_messages`.

The options belong to a **placement actor**, a specialized actor shaped like a supervisor. The program implements a new
trait, `Placement`, whose required `init/1` returns the striping option, the moving option and the core list; `main`
starts the placement actor with `start_placement(Impl, state, stack_policy)` (or a supervisor starts it from a child
specification whose behaviour is `placement_behavior(Impl)`), and the runtime supplies its behaviour. There is exactly
one realized `Placement` per application, and the placement actor is registered under the claimed atom `:placement`.
There are **no defaults**: the flags record has no optional fields, so an `init` that does not name all three is a
compile error, as a spawn without a `stack_policy` is.

Spawning does not change. A program spawns an actor as it does today and then **hands it off** with
`cast_placement({ op: :place, actor: ref, key: k, mobility: :movable })`. The hand-off is a cast to the placement
actor. The runtime holds a freshly spawned actor's first dispatch until the placement actor has placed it (for at most
1 ms), so moving it costs almost nothing: it has no committed stack, and any messages already sent to it move with its
mailbox in order. An actor that is never handed off keeps today's semantics exactly: nothing but the program moves it.

On hosted targets the placement actor's behaviour is native runtime code working with the per-core carriers of roadmap
chunk 12; on raw targets it is a Silica library implementing a second trait, `PlacementProvider`, on exclusive pins
and `migrate_actor`, so a program written against `Placement` runs unchanged on both.

The acceptance bar is the BEAM's: each mechanism must cost no more than the BEAM's equivalent, and should cost less.
Section 6 compares every option with its BEAM counterpart, says where Silica should win (no reduction counting, no
per-process garbage collection, an empty stack at every move, compile-time knowledge of call/cast, effects and pins,
communication-aware and core-type-aware placement the BEAM does not do) and where it starts behind (no preemption
between yield points, one OS thread per actor until chunk 12, page-granular stacks, a system call per spawn, and a
hand-off message where the BEAM places at spawn for free), and Section 10 is the benchmark plan that must show parity
or better on the same machine.

---

## 1. The problem and Lee's direction

### 1.1 What the language says today

| Fact | Where |
|---|---|
| Every actor is pinned from spawn until it terminates; there is no unpinned state. | §15.1.2 Actor Pinning Policy |
| The initial core is `spawn`'s core argument, or "the core the runtime assigns at spawn time". | §15.1.2, §15.1.1, §4.6 |
| An actor changes core only through `migrate_actor()` (or a `pin_actor_to_*` helper, which is a migration). | §15.1.2, §22.10 |
| The runtime never moves an actor on its own: not for load, thermal, parking, power, NUMA or regions. | §15.1.2, §15.1.2.4, §22.10, §23.1.1 |
| A requested move takes effect at the actor's next **dispatch boundary** (a behaviour's return); nothing interrupts a dispatch. | §15.1.2 |
| At a **scheduler yield point** (a dispatch boundary, or a wait such as a `call` awaiting its reply) the per-core scheduler may run another actor; a suspended actor resumes on the same core, inside the same dispatch. Between yield points there is no preemption. | §15.1.2, §23.1.1 |
| Message order is preserved across a migration: the queue moves atomically, and messages arriving during the move are processed after it. | §15.1.2.4.1 |
| Migration requests are control messages handled by the runtime, never seen by the behaviour. | §16.2.6.6 |
| OS-free: pinning is exclusive and hard. OS-hosted: the actor is bound to the runtime's carrier thread for its core; the OS may still move that thread; `migrate_actor` moves the actor between carriers, "as the BEAM migrates processes between its scheduler threads". | §15.1.2 |
| Energy, NUMA, thermal and load considerations apply only to the initial core of an actor spawned without one. | §15.1.2.1 (Energy Efficiency Optimization), §15.1.2.4 |
| The per-core scheduler shares a core's time among its actors fairly, one dispatch at a time, with `priority_level` hints, and "never balances load by moving actors between cores". | §23.1.1, §22.10 |
| A supervisor is an actor whose behaviour the runtime supplies from the `init/1` of a `Supervisor` implementation; it has a runtime-populated supervision ingress drained before ordinary maintenance calls. A state machine's runtime-supplied behaviour can be named in a child specification with `state_machine_behavior(ImplType)`. | §15.4.8.1, §15.4.9, §15.4.13, §15.5.6 |
| Registered names are atoms fixed at build time; supervisors share the ordinary registry's namespace; FFI and device workers have registries of their own. | §20.3.1; FFI specification §4.9 |
| Hosted targets run many actors per carrier, and thousands or millions of actors per machine, because an idle actor holds no stack memory. | §15.1.2.2 (Key Properties, Advantages); ROADMAP chunk 12 |
| Each actor's stack is its own contiguous reservation; nothing is committed at spawn, and at a dispatch boundary the stack is empty. | §15.1.2.2 |

**Today's implementation** (`src/emitter/apple_silicon_mac/terms/prims/prims_actors_runtime_asm.silica` and
`prims_actors_stack_asm.silica`): the hosted runtime still runs **one pthread per actor** (`pthread_create` per spawn;
chunk 1 kept thread-per-actor and switches each thread onto the actor's reservation). There are no carrier threads yet
(they are chunk 12), so there is nothing for a placement policy to act on: the OS already balances those threads.
`_silica_rt_migrate_actor` sets `THREAD_AFFINITY_POLICY` on the actor's thread and returns; it does not yet return the
§15.1.2 result atoms. The ESP32-S3 raw path has no actor runtime yet
([ports/esp32s3_port_status.md](ports/esp32s3_port_status.md), "Actors"). Everything in Sections 8 and 9 depends on
that work and is marked as such.

### 1.2 Lee's direction

1. Keep the language's promise: on hosted targets the runtime runs many actors per carrier thread and supports
   thousands or millions of actors.
2. Give the programmer well-researched, **named** options for **striping** (initial placement across cores) and
   **moving** (rebalancing), presented like the five `stack_policy` release algorithms: each name says what it does,
   what it costs, and when to choose it. Keep every justified option, including those that trade throughput for
   energy, locality, predictability or stable placement.
3. The options live **outside ordinary actors**, in a trait realized as a **specialized actor, like a supervisor**:
   runtime-owned behaviour on hosted targets, library-provided behaviour on raw targets.
4. The **same behaviour** is what the library or libraries provide for raw (OS-free) applications, where the language
   provides only exclusive hard pinning and `migrate_actor`.
5. The mechanisms must be **as efficient as the BEAM or more efficient**, and more is preferred, compared per option
   and reported honestly.
6. **No defaults.** The program names its striping and moving choices explicitly, the way every spawn names a
   `stack_policy(reserve, release)`. The document gives guidance for choosing (Section 5), not a default.
7. **The shape of spawn does not change** (it would harm the raw board apps). Actors reach placement by a hand-off
   after they are spawned; supervisor children reach it without changing spawn or child specifications.
8. **Exactly one realized trait per application**, started once at application startup, with its options fixed there.
   Changing options while running is a later enhancement.
9. The placement actor has a **claimed, predefined registered name**, `:placement`; the compiler rejects that name
   everywhere else and rejects a second placement actor.
10. An application is intended to be **one compilation unit**, so its atoms are numbered once; the per-unit numbering
    seen today is a consequence of splitting applications into units to save compiler memory, not the language's
    model (§7.5).

### 1.3 What stays in the language, and what placement adds

| Stays exactly as it is | Added by this proposal |
|---|---|
| Every actor is pinned; there is no unpinned state. | A **placement actor**, started once per application from the one `Placement` implementation, owns the placement of the actors handed to it. |
| Every spawn form, its arguments and their positions; every child specification field. | `cast_placement` hands an already-spawned actor (or a supervisor and its subtree) to the placement actor; `call_placement` asks read-only questions. |
| A spawn with an explicit core id pins the actor there; only the program moves it. | A handed-off actor starts on the core the striping option chooses, and moves as the moving option says. |
| `migrate_actor` and its four result atoms. | `migrate_actor` on a placed actor is honoured and takes the actor out of placement (§7.12). |
| Moves take effect only at dispatch boundaries; message order is preserved. | Every placement move uses the same migration path, so the same guarantees hold. |
| The runtime never moves an actor on its own. | The runtime moves a **placed** actor, acting for the placement actor, under the moving option its `init` named, only among the cores `init` named. That is a program decision declared once, as a supervisor's restart strategy is (§15.4.13), and applied only to actors the program handed over. |
| OS-free pins are exclusive and hard. | On raw targets the placement actor's behaviour is a Silica library that moves actors with `migrate_actor`; the raw pinning rule does not change at all. |
| `stack_policy` is required at every spawn. | Striping, moving and the core list are required fields of `init`'s result; `start_placement` takes a `stack_policy` like every actor start. |

The "no silent moves" rule therefore survives in its strongest form: an actor moves only because the program asked for
it, either by calling `migrate_actor` or by handing the actor to the placement actor whose moving option it named.

---

## 2. How production runtimes place and balance lightweight tasks

This section extracts the distinct, nameable strategies. For each runtime: the mechanism, what it guarantees, what it
costs, whether it assumes preemption, and whether it fits Silica's model (moves only at dispatch boundaries, message
order preserved, pinned by default, no preemption between yield points). Sources are listed in
[Appendix B](#appendix-b-sources) and cited as [B1], [B2], and so on.

### 2.1 The BEAM (Erlang/OTP), in depth

The BEAM is the reference point for this design. What follows is from the ERTS documentation and the current
`erl_process.c` and `erl_process.h` in the OTP repository [B1–B6].

**Schedulers and run queues.** One normal scheduler thread per logical processor by default (`+S`), each with its own
run queue, split by priority (max, high, normal, low). A scheduler runs processes from its own queue [B1, B3].

**Reduction accounting and preemption.** Each process gets `CONTEXT_REDS` reductions per turn, **4000** since OTP 20
(2000 before) (`erl_vm.h`: "Swap process out after this number"). A reduction is "roughly equivalent to a function
call"; the counter is decremented at every call, and a process is switched out when it runs out, at a call or a receive
[B3, B4]. The BEAM is therefore *preemptive at function-call granularity*: a long computation is interrupted every 4000
calls. This is the single largest difference from Silica, which never interrupts a dispatch.

**Placement at spawn.** A new process goes on the **parent's run queue** (`erl_create_process`: `rq = erts_get_runq_proc(parent, ...)`),
unless the undocumented `scheduler` spawn option binds it, which the source marks "Unsupported feature" [B5]. The BEAM
has no striping choice: spawning is spawner-local, and spreading is left to balancing.

**The periodic balance check.** `check_balance()` runs on one scheduler at a time (an atomic flag) after a run queue has
executed `ERTS_RUNQ_CALL_CHECK_BALANCE_REDS` reductions, half of `ERTS_RUNQ_CHECK_BALANCE_REDS_PER_SCHED = 2000*CONTEXT_REDS`,
that is **8,000,000 reductions** per scheduler per full cycle, alternating a "halftime" pass that only samples
out-of-work flags [B5]. It locks each run queue in turn to read its maximum queue length and reductions per priority,
keeps a history of "full" reductions, and then either:

- with **scheduler compaction of load** (`+scl true`, the default) computes how many run queues should be *active*
  from the total work, keeps a memory of the last rise so it does not shrink too eagerly (90% rule), marks the rest
  inactive and gives each an **emigrate** path to an active queue; or
- with all queues active, computes an average maximum length weighted by each queue's availability, a **migration
  limit** per queue and priority, sorts queues by their excess, and pairs the longest with the shortest as
  **emigrate-to / immigrate-from** paths; or
- with **scheduler utilization balancing** (`+sub true`, off by default, implies `+scl false`) uses measured
  utilization instead of lengths, with imbalances under 0.5% (5000 ppm) ignored, over 1 s and 10 s windows [B1, B5].

**Emigration happens at enqueue.** When a process becomes runnable (a message arrives, a timer fires), it is enqueued on
its last run queue unless that queue has an emigrate path at that priority and both queues are unbalanced; then it is
enqueued on the target instead (`erts_check_emigration_need` in the enqueue path) [B5, B6]. A scheduler whose queue is
below its limit **immigrates** from its paired queue. Moves are therefore decided in bulk once per check and applied one
process at a time, for free, at the moment each process is being scheduled anyway: "Migration occurs only during
process scheduling decisions" [B6].

**Work stealing.** A scheduler whose queue empties first tries inactive run queues, then the other active ones, then
queues it skipped because they were contended. From a victim it steals **half** of the runnable processes, at least one,
**at most 100**, highest priority first ("Only steal half the tasks (to balance the load between the victim runqueue and
this one)") [B5].

**Why compaction exists.** Rickard Green (ERTS): "The runtime system tries to compact the load on as few schedulers as
possible without getting run-queues that build up. ... This compaction of load onto fewer schedulers is there in order to
reduce communication overhead when there aren't enough work to fully utilize all schedulers." Sleeping schedulers are
woken only when overload accumulates, governed by `+swt` [B7]. Busy-waiting before sleep is `+sbwt` (default `medium`)
[B1].

**Bind types.** `+sbt` binds scheduler threads to logical processors: `u` (unbound, the default), `ns`, `ts`, `ps`, `s`
(spread), `nnts`, `nnps`, `tnnps` (what `db` means). Binding "is only supported on newer Linux, Solaris, FreeBSD, and
Windows systems", and the documentation warns that binding can cost performance when other programs bind too [B1].
Binding is per scheduler thread, never per process.

**Dirty schedulers.** Native functions that run longer than about a millisecond must be marked dirty, CPU-bound or
I/O-bound, and run on separate dirty scheduler pools (`+SDcpu`, default equal to the normal scheduler count; `+SDio`,
default 10), because a long NIF would otherwise block a normal scheduler and every process queued on it [B8, B9]. The
programmer must classify each NIF by hand; a misclassification can starve normal schedulers.

**Measured costs** (the BEAM's side of the comparison in Section 6; all to be re-measured on our machines, Section 10):

| Quantity | BEAM figure | Source |
|---|---|---|
| Memory of a new process | 338 words (2,704 bytes on 64-bit), including a 233-word heap that holds the stack | [B10] (the efficiency guide says 327 words [B11]) |
| Heap growth | Fibonacci-like from 233 words, then 20% steps; per-process generational semi-space **copying** collector | [B12] |
| Spawn time | about 3 µs CPU per process for 20,000 processes (Armstrong, *Programming Erlang*, 2007 hardware) | [B13] |
| Messages | copied into the receiver, except refc binaries and literals | [B11] |
| Preemption slice | 4000 reductions | [B4] |
| Balance check | every 4–8 million reductions per scheduler; locks every run queue to read it | [B5] |
| Steal | half the victim's queue, at most 100 | [B5] |

**Fit for Silica.** Almost everything fits. Enqueue-time emigration, immigration, and stealing all move a process that is
*between* executions, which in Silica terms is an actor at a dispatch boundary. What does not carry over is preemption:
the BEAM can always get a scheduler back after 4000 reductions, so a queue never waits long behind one process. Silica
cannot, and Section 6.3 says how the design compensates.

### 2.2 Go

G (goroutine), M (thread), P (processor, the right to run Go code). Each P has a local run queue plus a one-slot
`runnext`; a global queue is checked "once in a while to ensure fairness", every 61 scheduler ticks. An idle P steals
**half** of another P's queue, trying up to four passes over a random order. An M that blocks in a system call hands its
P to another M (`handoffp`). The number of spinning Ms is limited, and new Ms are woken only if there is an idle P and no
spinning M. `sysmon` preempts goroutines that run longer than 10 ms [B14, B15].

The `runnext` slot is instructive for Silica: "A runnext goroutine shares the same time slice as the current goroutine
... To prevent a ping-pong pair of goroutines from starving all others, we depend on sysmon to preempt" [B15]. A handoff
slot without preemption can starve a core. **Fit:** per-core queues, steal-half and throttled wakeups fit; `runnext`
fits only with a bound on consecutive handoffs, because Silica has no sysmon preemption.

### 2.3 Tokio

Per-worker fixed-size single-producer multi-consumer ring buffers, overflow to a global queue, steal half, a LIFO slot so
"the receiver of the message [is] scheduled to run next", with the caveat that it can hurt fairness; at most about half
the workers search for work at once, and a worker wakes a sibling only when no searcher is active [B16]. **Fit:** the
lock-free ring and the throttled searching are the right hosted data structures for Silica's carriers. Tokio tasks are
cooperative too (a future runs until it returns `Pending`), so its fairness tools are relevant without preemption.

### 2.4 Cilk and Rayon

Randomized work stealing on per-worker deques (Chase–Lev in Rayon via crossbeam): the owner pushes and pops at one end,
thieves take from the other; expected time T1/P + O(T∞) and space within a constant factor of serial for fully strict
computations (Blumofe and Leiserson) [B17, B18, B19]. **Fit:** the theory applies to fork–join task graphs, not to
long-lived actors with mailboxes, but the result that random victim selection with steal-from-the-cold-end is
near-optimal when work exists is why every runtime above steals.

### 2.5 Seastar and ScyllaDB (shard per core)

"One application thread per core", explicit message passing between cores, no shared state and no task migration; data
is sharded so that each key has an owning core [B20]. **Fit:** perfect: it is Silica's pinning with placement by key.
It shows that *never moving* is a legitimate, high-performance choice when load is spread by key rather than by
measurement, and that it loses under skew.

### 2.6 Akka

Actors bound to a thread pool (fork-join by default), a `PinnedDispatcher` that gives one actor its own thread, a
balancing dispatcher with a shared mailbox for identical actors, and an **affinity pool** executor that "tries its best
to ensure that an actor is always scheduled to run on the same thread" using per-thread queues, assigning actors by a
map until a threshold (128 by default) and by hash afterwards [B21, B22]. **Fit:** the affinity pool is hash-based
striping with no movement; the balancing dispatcher assumes interchangeable actors sharing one mailbox, which Silica's
one-mailbox-per-actor model does not have.

### 2.7 Microsoft Orleans

Grain placement strategies, chosen per grain class [B23]:

- **random** (the default up to 9.1; relies on the law of large numbers);
- **prefer local** (the silo that received the request);
- **hash-based** (grain id mod the number of compatible silos, not stable under membership changes);
- **activation-count-based**, which samples **two** silos at random and picks the one predicted to have fewer
  activations, citing Mitzenmacher's power of two choices;
- **resource-optimized** (the default since 9.2): a weighted, normalized score over CPU, memory and activation count,
  smoothed by a Kalman-style filter, with a margin that prefers the local silo;
- **activation repartitioning** (experimental): tracks the heaviest grain-to-grain communication edges with a
  probabilistic structure and periodically migrates grains toward their partners while keeping counts balanced, with a
  recovery period and an **anchoring filter** that leaves well-placed grains alone;
- **activation rebalancing** (experimental): periodic sessions that move activations from heavy to light silos by an
  entropy measure, with a per-cycle migration limit.

**Fit:** the placement set maps almost one to one onto the striping options below. Repartitioning is the only
production example of communication-following movement; its recovery period and anchoring filter are the hysteresis
this design borrows.

### 2.8 Pony and CAF

Pony's runtime has only a work-stealing scheduler; an actor processes up to a batch of 100 messages per turn; since
0.21.1 scheduler threads suspend when there is not enough work (`--ponyminthreads`), because stealing among idle threads
"usually result[ed] in wasted CPU cycles and cache thrashing" [B24, B25]. CAF defaults to work stealing with three
polling phases (aggressive, moderate, relaxed) and offers a single-queue work-sharing policy; actors that block are
marked `detached` onto their own thread [B26]. Locality-guided scheduling in CAF steals from nearby cores first and keeps
communicating actors close [B27]. **Fit:** Pony's thread suspension is compaction; CAF's `detached` is the analogue of
BEAM dirty schedulers and of Silica's `spawn_dangerous` workers.

### 2.9 Linux: CFS/EEVDF and scheduling domains

Load balancing runs over a hierarchy of scheduling domains (SMT, cluster, package, NUMA), balancing between groups of
CPUs at each level on its own interval, and pulling tasks from the busiest runqueue of the busiest group [B28]. EEVDF
(6.6) replaced CFS's pick of the next task; the load balancer is essentially unchanged [B29]. **Automatic NUMA
balancing** scans a task's memory, takes hinting faults, migrates pages toward the task and groups tasks that share data
[B30]. **Fit:** the domain hierarchy is how victims should be ordered when stealing (nearest first). Task migration in
Linux can happen at any preemption point, which Silica does not have.

### 2.10 Energy-aware scheduling: Linux EAS and Apple QoS

Linux **EAS** chooses, at task wake-up, the CPU that minimizes energy according to an energy model of performance
domains, on asymmetric (big.LITTLE) systems only, and **only while no CPU is above the 80% tipping point**; above it the
kernel falls back to ordinary load balancing [B31]. On Apple silicon, placement on P or E cores is steered by
**quality-of-service** class, not affinity: background QoS threads are confined to E cores; higher classes prefer P cores
and spill to E cores [B32, B33]. **Fit:** EAS's "pack while under the tipping point, spread above it" is
`:fill_in_order` plus `:compact_when_quiet`; Apple's QoS is the only lever a macOS carrier has for choosing a cluster,
because macOS offers no hard binding (§15.1.2).

### 2.11 Power of two choices and communication-aware placement

Throwing n balls into n bins at random gives a maximum load of about log n / log log n; choosing the lesser of **two**
random bins gives log log n / log 2 + O(1), an exponential improvement for one extra probe [B34]. Communication-aware
actor placement for NUMA machines (Francesquini, Goldman and Méhaut: hub actors and their affinity groups placed
together, then hierarchical balancing and stealing) and locality-aware stealing for actors (Barghi and Karsten, IPDPS
2018) show that the BEAM's communication-blind balancing leaves performance on the table for chatty actors [B35, B36].

### 2.12 The strategies, distilled

| Strategy | Used by | Needs preemption? | Fits Silica? |
|---|---|---|---|
| Spawner-local placement | BEAM (always), Orleans prefer-local | no | yes: `:with_spawner` |
| Cyclic placement | Akka routers, `+sbt`-ordered schedulers | no | yes: `:round_robin` |
| Two random choices | Orleans activation-count | no | yes: `:least_loaded` |
| Hash / shard by key | Seastar, Orleans hash, Akka affinity pool | no | yes: `:by_key` |
| Pack then spill | EAS below 80%, BEAM compaction, Pony suspension | no | yes: `:fill_in_order` |
| Never move | Seastar, BEAM bound processes | no | yes: `:never` |
| Steal half when idle | BEAM, Go, Tokio, Cilk/Rayon, Pony, CAF | no, if the stolen unit is between runs | yes, stealing actors at dispatch boundaries: `:steal_when_idle` |
| Periodic threshold balancing, applied at enqueue | BEAM `check_balance`, Linux domains, Orleans rebalancer | no | yes: `:balance_periodically` |
| Compaction to fewest cores | BEAM `+scl`, Pony, EAS | no | yes, and core-type aware: `:compact_when_quiet` |
| Follow the communication graph | Orleans repartitioning, Francesquini, Barghi–Karsten, CAF locality | no | yes: `:follow_messages` |
| Time-slice fairness | BEAM reductions, Go sysmon, Linux | **yes** | **no**: Silica's fairness is between yield points (§23.1.1) |
| Handoff slot (runnext / LIFO) | Go, Tokio | Go: yes; Tokio: bounded | only with a bound (Section 8.4) |
| Shared mailbox among identical actors | Akka balancing dispatcher | no | no: one mailbox per actor |

### 2.13 What Silica's model changes

- **The unit that moves is an actor with an empty stack.** A move takes effect at a dispatch boundary, where the
  behaviour has returned and the stack pointer has reset (§15.1.2.2). No frames, no saved registers and no heap travel;
  only the control block pointer and the mailbox change carrier. That is cheaper than moving a preempted BEAM process
  or a Go goroutine, which carry live stacks.
- **An actor suspended in a wait cannot move.** A `call` awaiting its reply resumes on the same core inside the same
  dispatch (§15.1.2). Stealing and balancing may only take actors that are ready at a dispatch boundary.
- **No preemption.** A carrier running a long dispatch cannot be interrupted; the actors queued behind it wait. The
  runtime can still take those *queued* actors away (they are at boundaries). That is the compensation.
- **Message order is already guaranteed** across migration (§15.1.2.4.1). Every placement move uses that path.
- **Pins are explicit and visible in the source.** The runtime never has to guess which actors may move.


---

## 3. Striping options: where a handed-off actor starts

The `striping` option decides the core of each actor handed to the placement actor. It is applied when the placement
actor places the actor (Section 7.7), which for a freshly spawned actor is before its first dispatch. Like the release
algorithms of §15.1.2.2, each name says the rule, and the constants are part of the definition. "Load" below is a
carrier's measured busy share over the last **100 ms window** (Section 4.1); "the list" is the `cores` list `init`
returned, in its order.

| `striping` | rule | guarantees | cost per placement | choose it for |
|---|---|---|---|---|
| `:round_robin` | the next core in the list, cycling | over any run of placements, per-core counts differ by at most one (before exits and moves) | one atomic increment | many similar actors, CPU-bound worker pools, predictable spread; list order sets the spread (topology-spread orders from a stdlib helper) |
| `:least_loaded` | two cores drawn at random from the list; the one with the lower load, member count breaking ties within a 25% margin | power-of-two-choices balance: maximum overload grows as log log n, not log n / log log n | two remote reads and a random draw | churn of heterogeneous actors whose cost is unknown in advance; request handlers spawned by one acceptor |
| `:with_spawner` | the core of the actor's spawner if it is in the list; otherwise the `:round_robin` rule | parent and child share a carrier, so their messages never cross cores | none | pipelines and parent–child conversations; the BEAM's own spawn rule |
| `:by_key` | rendezvous hashing of the key over the list | the same key always lands on the same core; a supervised child returns to its core when restarted | one hash per listed core | sharded state (sessions, cache shards, per-device actors); deterministic tests |
| `:fill_in_order` | the first core in the list whose load is under 80%; if none is, the `:least_loaded` rule over the whole list | load is packed onto the head of the list and spills only when the head is busy | one read per core tried | battery and mostly-idle programs; with the list ordered efficiency cores first, the energy choice |

Details and lineage:

- **`:round_robin`.** One counter, incremented with a relaxed atomic. Under heavy concurrent hand-offs the runtime may
  keep a per-carrier counter seeded at a different offset, which weakens "differ by at most one" to "at most one per
  handing-off carrier". Lineage: cyclic routers, `+sbt spread`. *Versus the BEAM:* the BEAM puts every child on the
  parent's queue and relies on balancing to spread a burst; `:round_robin` spreads the burst at once.
- **`:least_loaded`.** Mitzenmacher's two choices, as in Orleans activation-count placement [B23, B34]. Loads are read
  from each carrier's published statistics without locks, so a concurrent burst can make stale choices; two random
  choices are robust to that. *Versus the BEAM:* no BEAM equivalent; two cache misses more than the BEAM's placement,
  fewer migrations later.
- **`:with_spawner`.** The runtime records the spawner's core in the new actor's control block at spawn (one store,
  whether or not the actor is ever handed off). For a supervisor's child the spawner is the supervisor. When the spawner
  is `main` (not an actor, §15.1.2.2) or its core is not in the list, the rule falls back to `:round_robin`; that
  fallback is part of the option, not a hidden default. Exactly the BEAM's rule [B5] and Orleans prefer-local. Alone, it
  piles an acceptor's children on the acceptor's core; with a moving option it gives the BEAM's behaviour with better
  locality.
- **`:by_key`.** The key is the `key` field of the hand-off; for a supervisor's subtree it is derived from the
  supervisor's position and the child's `id`, both stable across restarts (§15.1.4, §15.4.13.2). Rendezvous
  (highest-random-weight) hashing is chosen over `key mod n` although the core list is fixed at startup, so that the
  future ability to change the list (Section 12) moves only the keys whose core left, and never re-maps the rest. The
  cost is one hash per listed core, a few nanoseconds each. Lineage: Seastar shards, Akka affinity pool, Orleans hash
  placement.
- **`:fill_in_order`.** EAS's rule below the 80% tipping point [B31], applied at placement. A burst lands on the first
  core until the next window reports its load; that is intended (packing), and a moving option such as
  `:compact_when_quiet` or `:steal_when_idle` corrects an overshoot. *Versus the BEAM:* the BEAM compacts after the
  fact, by run-queue index and without knowing core types; this option packs at placement, in the order the program
  chose.

**Spread versus compact ordering** is the order of the `cores` list: list one core per cluster first to spread, list
cores cluster by cluster to pack. A stdlib helper builds such orders from `get_cpu_topology()` (§22.10). NUMA locality
comes from the list and from the striping rule (`:with_spawner` and `:by_key` keep related actors together); with one
placement actor per application there is no longer one group per node (see Q12).

---

## 4. Moving options: when a placed actor moves

The `moving` option decides whether and when placed actors change core. Every move, whatever the option, is a migration
in the sense of §15.1.2: it takes effect at the actor's dispatch boundary, the mailbox moves with it atomically, and
message order is preserved (§15.1.2.4.1). An actor suspended in a wait is never moved, and an actor handed off with
`mobility: :fixed` is placed by the striping option and then never moved by the moving option.

| `moving` | trigger | measures | guarantees | cost | choose it for |
|---|---|---|---|---|---|
| `:never` | none | nothing | placed actors stay where striping put them; only `migrate_actor` moves one (and takes it out of placement) | zero | shard-per-core designs, real-time and latency-critical actors, reproducible runs |
| `:steal_when_idle` | a listed carrier runs out of ready actors | ready counts per carrier | a busy carrier's queue is drained by idle ones; nothing moves while every carrier has work | zero while all are busy; one steal per idle event | throughput pools with bursty or skewed load; the Go/Tokio/Cilk model |
| `:balance_periodically` | every 100 ms window, plus stealing when idle | busy share and ready counts per carrier | carriers are kept within a 25% margin of the mean; no actor moves twice within 1 s | one pass over the cores per window; one compare per enqueue | long-lived actors whose load shifts slowly; the BEAM's model with `+scl false` or `+sub true` |
| `:compact_when_quiet` | every 100 ms window, plus stealing within the active cores | busy share per carrier | load is held on the shortest prefix of the list that keeps each active core under 80%; a core is opened after one window above, closed after 1 s below | as `:balance_periodically`; idle carriers sleep | energy, battery, big/little machines at low load; the BEAM's default (`+scl true`), made core-type aware |
| `:follow_messages` | every 1 s, plus stealing when idle | sampled sends (1 in 64) per placed actor | an actor moves to the core of the actor it messages most, if that actor takes at least half its sends and the target stays within the 25% margin; hubs stay; no actor moves twice within 1 s | one decrement per send; a bounded edge table per carrier | pipelines, conversations and hub-and-spoke patterns where cross-core sends dominate |

### 4.1 The constants

As with the 32-message window and one-second idle period of the release algorithms (§15.1.2.2), the constants are part
of the options' definitions, not knobs. That is what makes "no defaults" possible: there is nothing left unnamed.

| Constant | Value | Lineage |
|---|---|---|
| Measurement window | 100 ms | between Linux's per-domain balance intervals and the BEAM's reduction-driven check; short enough to follow a burst, long enough to average one |
| Imbalance margin | 25% of the mean | the BEAM ignores imbalance below its migration limits and `+sub` ignores 0.5%; Linux domains use an imbalance percentage; 25% keeps moves for real differences |
| Minimum residency | 1 s per actor | Orleans repartitioning's recovery period, scaled from minutes to one process |
| Steal size | half the victim's movable ready actors, at most 64 | the BEAM steals half, at most 100 [B5]; Go and Tokio steal half |
| Stuck carrier | a carrier whose current dispatch started more than 1 ms ago | the BEAM's 1 ms guideline for when native code must leave a normal scheduler [B8] |
| Placement hold | at most 1 ms (Section 7.7) | the same 1 ms bound |
| Tipping point | 80% busy share | Linux EAS [B31] |
| Close delay | 10 windows (1 s) below the tipping point | the BEAM's reluctance to shrink after a rise (its 90% rule) [B5] |
| Message sampling | 1 send in 64 | Orleans tracks edges probabilistically to bound memory [B23] |
| Dominance | the heaviest partner takes at least 50% of the samples | Orleans's anchoring filter and Francesquini's hub/affinity-group split [B23, B35] |

### 4.2 Each option in detail

**`:never`.** The placement actor chooses a core at hand-off and never again. Placed actors behave exactly like
explicitly pinned actors, except that the placement actor reports them (`:members`). Failure mode: skew. If one core's
actors become hot, nothing corrects it. That is the price of predictability, and the reason to pair `:never` with
`:by_key` (load spread by the key distribution) or `:round_robin`. *Versus the BEAM:* the BEAM cannot do this for
ordinary processes (binding is "unsupported"); it is Seastar's model [B20].

**`:steal_when_idle`.** When a listed carrier finds no ready actor, it chooses a victim among the other listed carriers
in **topology order** (same cluster or shared L2 first, then same NUMA node, then the rest; Linux domains and CAF's
locality-guided stealing [B27, B28]), and takes half of the victim's *movable ready* placed actors, at most 64, highest
`priority_level` first (as the BEAM steals highest priority first [B5]). An actor is movable when it was handed off with
`mobility: :movable`, is at a dispatch boundary with messages waiting, is not in a wait, and has not moved in the last
second. A carrier whose current dispatch has run more than 1 ms is a victim even if it is not the longest queue: every
movable actor queued behind a long dispatch may be taken. Searching carriers are limited to half of the idle listed
carriers at once (Tokio [B16]; Go's spinning-M rule [B14]). Failure modes: stealing that ping-pongs an actor between two
nearly idle carriers (prevented by the residency rule), and cache and NUMA cost on the thief (bounded by topology order).
*Versus the BEAM:* the same algorithm and steal-size rule; Silica steals only boundary actors, which carry nothing but a
control block and a mailbox.

**`:balance_periodically`.** Once per window the placement actor reads every listed carrier's published busy share and
ready count, computes the mean, and publishes a new **migration table**: for each carrier above the mean by more than
the margin, an emigrate target below it and a limit on how many actors to send. Busy share is compared while carriers
are below 100%; saturated carriers are compared by ready count, because utilization saturates. Actors then move **at
enqueue**: when an idle placed actor becomes ready because a message arrived, the enqueue checks its carrier's table
entry and, if the carrier is emigrating and the limit is not spent, enqueues the actor on the target carrier instead.
That move costs one load and one compare, happens exactly at a dispatch boundary, and needs no scan of actors. Stealing
when idle is included. Actors with the least retained stack (the `retained` field of `get_actor_memory_usage(ref)`,
§15.1.2.2) are preferred when a choice exists, because they carry nothing. Failure modes: oscillation when load changes
faster than the window (bounded by residency: at most one move per actor per second); moving an actor whose data is
cache-hot on its old core. *Versus the BEAM:* the same mechanism (bulk plan, enqueue-time emigration, immigration,
stealing); the differences are time-based windows instead of reduction counts, utilization measured in nanoseconds
instead of reductions, lock-free reads of published statistics instead of locking every run queue, and per-actor
hysteresis the BEAM does not have.

**`:compact_when_quiet`.** Each window, the placement actor computes the **active prefix**: the shortest prefix of the
core list whose cores can hold the measured load with each under 80%. Carriers outside the prefix emigrate every placed
actor that becomes ready (and are stolen from first), then sleep. When any active carrier stays above 80% for a whole
window, the next core in the list opens; the prefix shrinks by one only after 10 windows (1 s) below. Within the prefix,
the option balances as `:balance_periodically` does. Ordered efficiency cores first, the list makes this energy-aware
compaction on big/little machines; on macOS the carriers of E-core entries run at background QoS, which is the only way
to keep them on E cores (Section 8.7). Failure modes: added latency when a burst arrives on a compacted placement (one
window to open a core, plus the wake-up); on symmetric machines the energy gain comes only from idle cores sleeping
longer. *Versus the BEAM:* the BEAM's default (`+scl true`), whose purpose is to "reduce communication overhead when
there aren't enough work to fully utilize all schedulers" [B7]; this option adds the core-type order and the explicit
tipping point.

**`:follow_messages`.** Every carrier samples one in 64 sends by placed actors into a bounded table of (sender, receiver)
edges. Once per second the placement actor merges the tables: for each sampled actor, if one partner accounts for at
least half of its samples, the actor's **preferred core** becomes that partner's core, provided the target's load is
within the margin; actors with no dominant partner are hubs and stay put (Orleans's anchoring filter; Francesquini's
rule of bringing the affinity group to the hub, not the hub to the group [B23, B35]). The move is applied at the
actor's next enqueue, as in `:balance_periodically`. A `call` counts as two samples, since it is a round trip and the
caller waits on it. Stealing when idle is included, preferring actors with no dominant partner. Failure modes: herding
onto a popular hub's core (bounded by the margin); churn in communication patterns faster than a second (bounded by
residency); sampling bias for rare but expensive messages. *Versus the BEAM:* the BEAM has no communication-aware
movement; this is the option where Silica should win outright on chatty workloads.

### 4.3 One mechanism under five options

On hosted targets all moving options except `:never` reduce to three runtime mechanisms, each already proven in the
BEAM, Go or Tokio, plus the placement actor's own turns:

1. **Enqueue-time redirection.** A per-carrier migration table and a per-actor preferred core, consulted when an idle
   placed actor becomes ready. Cost: one load and one compare per enqueue.
2. **Stealing at yield points.** Per-carrier ready queues that thieves can take half from. Carriers steal directly;
   they do not wait for the placement actor.
3. **The window step,** run by the placement actor on its periodic tick: it reads published statistics and publishes a
   new table.

No mechanism scans placed actors, so every per-hand-off, per-enqueue and per-dispatch cost is constant, and the
per-window cost is proportional to cores, not actors. That is the property that keeps millions of actors cheap.

---

## 5. Choosing the options (there is no default)

The one `init` of the application must name its striping option, its moving option and its cores; there is nothing to
fall back on. Per-actor variation is limited to what the hand-off carries: the key, and `mobility: :movable | :fixed`.
Actors that need different treatment (for example FFI workers on cores of their own) are not handed off and are pinned
with explicit core ids, exactly as today.

| Workload | `striping` | `moving` | Why |
|---|---|---|---|
| Request handlers spawned by one acceptor, varied cost | `:least_loaded` | `:steal_when_idle` | spread at placement by two choices; idle cores pull queued requests; no periodic cost |
| Long-lived actors with slowly shifting load (a general server) | `:with_spawner` | `:balance_periodically` | the BEAM's model: locality at spawn, bulk correction every window |
| Pipelines, parent–child conversations, many `call`s between pairs | `:with_spawner` | `:follow_messages` | partners share a carrier; a `call` becomes a local switch, not two cross-core wake-ups |
| Hub and spoke (a registry, a coordinator) | `:with_spawner` | `:follow_messages` | spokes move to the hub; the hub stays |
| Sharded state (sessions, cache shards, per-device actors) | `:by_key` | `:never` | stable, predictable, no balancing cost; skew is the key distribution's problem |
| Homogeneous CPU-bound worker pool | `:round_robin` | `:steal_when_idle` | even spread, stealing absorbs variance |
| Mostly idle, battery powered, or big/little at low load | `:fill_in_order` (E cores first) | `:compact_when_quiet` | packs load, lets cores sleep, spills above 80% |
| Latency-critical actors inside an otherwise balanced application | any | any, with those actors handed off as `mobility: :fixed` | striping places them; no move ever perturbs them; pair with `priority_level` |
| Reproducible test runs | `:by_key` or `:round_robin` | `:never` | placement is a function of the hand-off order or the key |
| FFI workers (`spawn_dangerous`) | not handed off | — | pinned explicitly to cores of their own, the BEAM's dirty-scheduler separation without hand classification (Section 6.2, Q5) |

---

## 6. Efficiency against the BEAM

The requirement is parity or better with the BEAM for each mechanism, reported honestly per option. The figures below
are expectations from the mechanism; Section 10 is how they will be measured. Nothing here is a measurement of Silica.

### 6.1 Per-option cost against the BEAM's equivalent

Common to every option is the **hand-off**, which the BEAM does not have (it places at spawn, on the parent's queue, for
free):

| Hand-off step | Cost | Notes |
|---|---|---|
| `cast_placement` at the sender | one compare-and-swap on the actor's control block (mark it pending), one enqueue on the placement actor's hand-off queue, and a wake-up if the placement actor's carrier sleeps | synchronous checks return `:already_placed`, `:actor_not_found` or `:placement_unavailable` without a round trip |
| the placement actor's turn | the striping rule (table below) plus a control-block update, **batched**: one turn drains every queued hand-off | amortized over many hand-offs under load |
| moving an actor that has not dispatched | update its core; if messages are already queued, transfer its ready-queue entry (a one-actor steal) | no stack is committed before the first dispatch (§15.1.2.2), so nothing else moves |
| the hold | the first dispatch waits for the placement actor's turn, at most 1 ms | the latency cost of placing before the first dispatch |

For the stateless striping rules (`:round_robin` with per-carrier counters, `:with_spawner`, `:by_key`), the runtime-owned
behaviour may choose the core at the sender, inside `cast_placement`, and leave only the bookkeeping to the placement
actor; nothing observable changes, because the placement actor's behaviour is the runtime's. With that latitude the
hand-off costs roughly one compare-and-swap and one enqueue at the sender, and no hold. Whether to allow it is Q11.

| Option | BEAM equivalent | Per placement | Per enqueue | Per dispatch | Per window | Per placed actor memory | Expected |
|---|---|---|---|---|---|---|---|
| `:round_robin` | none (spawner-local, then balancing) | 1 atomic add | 0 | 0 | 0 | 48 B (all options) | parity after the hand-off; fewer later moves |
| `:least_loaded` | Orleans two-choice; no BEAM form | 2 remote reads | 0 | 0 | 0 | 48 B | dearer placement, better spread |
| `:with_spawner` | BEAM spawn rule | 0 (+1 store at every spawn) | 0 | 0 | 0 | 48 B | parity after the hand-off |
| `:by_key` | none (`scheduler` option unsupported) | one hash per core | 0 | 0 | 0 | 48 B | no BEAM form; cheap |
| `:fill_in_order` | compaction after the fact | reads until a core is under 80% | 0 | 0 | 0 | 48 B | parity; packs sooner |
| `:never` | bound process (unsupported) | — | 0 | 0 | 0 | 48 B | better: no balancing work at all |
| `:steal_when_idle` | BEAM stealing | — | 0 | 1 timestamp | 0 | 48 B | parity per steal; the stolen unit is lighter |
| `:balance_periodically` | `check_balance` (`+scl false`/`+sub true`) | — | 1 load, 1 compare | 1 timestamp | O(cores log cores), no locks | 48 B | parity or better: no run-queue locks, hysteresis |
| `:compact_when_quiet` | `+scl true` (the BEAM default) | — | 1 load, 1 compare | 1 timestamp | as above | 48 B | better on big/little (core-type order) |
| `:follow_messages` | none | — | 1 load, 1 compare | 1 timestamp | O(edge table) per second | 48 B | no BEAM form; wins on chatty workloads |

"1 timestamp" is one read of the monotonic counter at each dispatch start or end (a register read on AArch64,
`CNTVCT_EL0`); the chunk 1 runtime already records the time of the last message in the control block for
`:release_when_idle`, so the new cost is one subtraction and one add into a per-carrier accumulator. The 48 bytes per
placed actor are the placement state (pending, placed, mobility), the key, the preferred core, the time of the last
move, and the links of the placed-actor list (Section 8.4). The placement actor itself is one more actor, with its
control block and the stack its `stack_policy` retains.

### 6.2 Where Silica can beat the BEAM, and why

1. **No reduction counting.** The BEAM decrements a counter at every function call so that it can preempt [B3, B4].
   Silica compiles to native code with no scheduler check inside a dispatch; the scheduler runs only at yield points.
   Compute-heavy dispatches pay nothing for scheduling.
2. **No per-process garbage collection and no copying heap growth.** A BEAM process grows by copying collection
   [B12]. A Silica actor's stack grows in place by committing pages and is released at the message boundary
   (§15.1.2.2); regions are freed by ownership. Balancing never waits for, or triggers, a collection.
3. **An empty stack at every move.** A move takes effect at a dispatch boundary, where the actor's stack is empty
   (§15.1.2.2). What changes carrier is a control block pointer and a mailbox. An actor under `:release_on_return` holds
   no stack pages at all, and a freshly spawned actor has never committed any, so moving it leaves nothing behind.
4. **Idle actors are smaller.** A BEAM process starts at 338 words, 2,704 bytes [B10]. A Silica actor idle under
   `:release_on_return` holds its control block (512 bytes after chunk 1, plus 48 when placed) and its mailbox, and no
   stack memory. For a million idle actors that is roughly a fifth of the BEAM's memory (to be measured, B2 in Section 10).
5. **Compile-time knowledge the runtime can use.**
   - *Calling convention.* The compiler knows which references are call-only and which cast-only (§15.1.1,
     §16.2.6.4). A `call` between actors on one carrier can switch directly to the callee (a bounded handoff, Section
     8.5) and back, with no wake-up; `:follow_messages` weighs a `call` as a round trip.
   - *Effects.* An actor that may block in foreign code is a `spawn_dangerous` worker by type (§15.1.1). The program
     knows which actors can hold a core in native code without classifying each foreign function, which the BEAM
     requires for dirty NIFs and which starves normal schedulers when it is wrong [B8, B9]; it keeps them out of
     placement and pins them to cores of their own.
   - *Pins and hand-offs.* Which actors may move is explicit in the source, never inferred.
6. **Communication-aware movement.** `:follow_messages` moves actors toward their partners; the BEAM does not do this.
7. **Core-type-aware compaction.** `:fill_in_order` and `:compact_when_quiet` pack onto the cores the program listed
   first, which can be the efficiency cores; the BEAM's compaction orders run queues by index and, on macOS, cannot bind
   schedulers at all [B1].
8. **NUMA-local stacks for free.** A stack commits pages on the node of the core that faults them (design §4.2, §4.3).
   Because a handed-off actor is placed before its first dispatch, its first page is committed on the placed core's
   node. With `:release_on_return`, even a later cross-node move re-commits locally on the next message.
9. **Hysteresis.** Every moving option has a per-actor residency of 1 s; under oscillating load the BEAM can emigrate
   the same process at every check.

### 6.3 Where Silica is at a disadvantage, and how the design compensates

| Disadvantage | Effect | Compensation |
|---|---|---|
| **No preemption between yield points.** | A long dispatch holds its carrier; actors queued behind it wait, where the BEAM would have switched after 4000 reductions. It can also delay the placement actor's own turn. | Stealing treats a carrier whose dispatch has run over 1 ms as a victim for all its movable actors. Stealing and enqueue redirection run in the carriers, not in the placement actor, so a delayed placement actor delays only hand-offs and window updates, and the hold on a new actor is released after 1 ms. The program splits long work with `cast(self(), …)` (§15.1.2, §16.2.6.5). Under `:never` there is no compensation, by the program's choice. Benchmark B8 measures this honestly. |
| **A hand-off message where the BEAM places at spawn for free.** | Every placed actor costs one cast and a share of a placement-actor turn, and waits up to 1 ms before its first dispatch. | Batching; the send-site latitude for stateless rules (Q11); it is paid once per actor lifetime. Benchmark B12 measures spawn plus hand-off against BEAM spawn. |
| **One OS thread per actor today.** | No carriers, so nothing to balance; spawn and switch cost a thread. | Placement needs chunk 12. The front end, the placement actor and striping can land before it (Section 13); moving waits for carriers. |
| **Page-granular stacks.** | An actor mid-dispatch, or retaining pages under `:keep_*`, holds at least one page: 16 KB on Apple silicon and the Raspberry Pi 5, 4 KB on most other Linux (§15.1.2.2), several times a fresh BEAM process. | Idle actors under `:release_on_return` hold none. The balancer prefers moving actors with little retained stack. |
| **A system call per spawn.** | Each spawn makes an address-space reservation and each exit unmaps it; the unmap can require a TLB shootdown across threads. The BEAM allocates a process from its own allocators. | Not a placement cost, but a parity prerequisite for benchmark B1: the runtime should recycle released reservations (as glibc caches freed thread stacks, design §4.2.1) and carve them from larger mappings. |
| **Linux map-count limit.** | `vm.max_map_count` defaults to 65,530 [B37]; an actor with a committed stack is at least two mappings (§15.1.2.2), so a million active actors exceed it. | A chunk 12 prerequisite: reservations carved from shared mappings merge while inaccessible; benchmark B1 records the setting used. |
| **Suspended actors cannot move.** | An actor waiting on a `call` reply resumes on its core (§15.1.2). | Only boundary actors are stolen; a waiting actor holds no carrier time. |
| **macOS has no hard binding.** | Carriers are hints to the OS (§15.1.2); so are the BEAM's schedulers [B1]. | Same for both runtimes; QoS steers E-core carriers (Section 8.7). |

---

## 7. The placement actor

### 7.1 Shape: a specialized actor, like a supervisor

The placement actor follows the supervisor pattern of §15.4.8 and §15.4.13 point for point:

| | Supervisor | Placement actor |
|---|---|---|
| Program implements | `impl T for Supervisor` with `init/1` | `impl T for Placement` with `init/1` |
| `init` returns | restart flags and child specifications | striping, moving and cores (all required) |
| Behaviour | runtime-owned (§15.4.8.1) | runtime-owned on hosted targets; the `PlacementProvider` library's on raw targets |
| Started by | `spawn_registered_supervisor(T, state, name, stack_policy [, core])` | `start_placement(T, state, stack_policy [, core])` in `main`, or a child specification with `behavior: placement_behavior(T)` |
| Registered name | chosen by the program | the claimed atom `:placement` (§7.5) |
| How many | any number | exactly one per application (§7.4) |
| Maintenance | `call_supervisor` (call only; "there is no `cast_supervisor`") | `cast_placement` for hand-offs and release (cast, because the sender does not need the placement decision); `call_placement` only for read-only queries |
| Priority channel | supervision ingress, drained first each turn (§15.4.9) | placement ingress, drained first each turn (§7.10) |
| Handle type | `supervisor_ref` | none needed: the helpers address `:placement` |

### 7.2 Names

- **`Placement`**, the program-side trait. It names what the actor decides, where actors run, as `Supervisor` names
  supervision and `StateMachine` names its behaviour. `Balancer` was considered and rejected (a placement with
  `moving: :never` balances nothing but still places); `Scheduler` collides with the per-core scheduler of §23.1.1.
- **`:placement`**, the claimed registered name (§7.5).
- **`start_placement`**, **`placement_behavior`**, **`cast_placement`**, **`call_placement`**, following
  `spawn_registered_supervisor`, `state_machine_behavior` and `call_supervisor`.
- **`PlacementProvider`**, the engine-side trait a raw library implements (§7.14).

### 7.3 The `Placement` trait

Proposed `stdlib/Placement.silica`, following `stdlib/Supervisor.silica`:

```silica
/// Placement trait: see the design document actor_placement_and_balancing_design.md (proposal).
///
/// An application realizes this trait exactly once. Implementors supply `init`, which returns the striping option,
/// the moving option and the core list. `start_placement(T, state, stack_policy)` in `main` (or a supervisor child
/// specification whose behavior is `placement_behavior(T)`) starts the application's placement actor, registered
/// as `:placement`; the runtime calls `T.init(state)` exactly once per start. The programmer does not provide the
/// placement actor's behaviour: on hosted targets the runtime provides it, on raw targets a PlacementProvider library.
///
/// Every field is required. There is no default striping, moving option or core list.

export trait Placement;
export init/1;

required {
    fn init(initial_placement_state: ActorState) -> {
        striping: :round_robin | :least_loaded | :with_spawner | :by_key | :fill_in_order,
        moving: :never | :steal_when_idle | :balance_periodically | :compact_when_quiet | :follow_messages,
        cores: List[uint64, mem(normal)]
    };
}
```

The core list must be non-empty and name cores that exist; an empty list, a duplicate or an unknown core fails the
start with `(:explicit, :invalid_core_list)`.

### 7.4 One realized trait and one placement actor per application

The rules are stated for the one-unit model: an application, its libraries included, is one compilation unit
([APPLICATION_IS_ONE_COMPILATION_UNIT.md](APPLICATION_IS_ONE_COMPILATION_UNIT.md)), so the compiler sees the whole
application when it checks them.

- **Exactly one `impl … for Placement`** may exist in an application. A second is a compile-time error (E2301).
- **At most one start site** in the application: one `start_placement` call, or one `placement_behavior(T)` in a child
  specification, not both and not two of either (E2302). `start_placement` is accepted only in `main`'s body (E2305),
  and `main` runs once, so a `start_placement` executes at most once.
- **Temporary, while builds are split into units.** Today's split build compiles each file as its own unit (a
  work-around for compiler memory, not the language's model). Meanwhile it must keep these whole-application rules with
  a **link-time check**: each unit's interface file records whether the unit realizes `Placement` and whether it
  contains a start site, and the step that assembles the application rejects a second of either with E2301 or E2302.
  As a further backstop the realized trait emits one fixed global symbol, so a second is also a duplicate-symbol link
  error. When the application compiles as one unit, the ordinary compile-time check is all that remains.
- **What the compiler cannot see.** A child specification containing `placement_behavior(T)` is one start site, but the
  supervisor that holds it could be started twice (two `spawn_registered_supervisor` calls with the same
  implementation), or the specification could be passed to `:add_child` twice. At run time a start finds `:placement`
  held by a live placement actor, and that start fails with `(:explicit, :placement_already_running)`: for a supervisor
  that is a failed child start, handled like any restart failure. No second placement actor ever runs.
- **What one placement actor governs.** Every actor handed to it (§7.7, §7.8), and nothing else. Per-actor variation is
  carried by the hand-off: the key, and `mobility: :movable | :fixed`. There are no placement groups; an actor that
  needs different cores or no balancing is not handed off, and is pinned with an explicit core id as today.

### 7.5 The claimed name `:placement`

**The choice.** The placement actor is registered under the predefined atom `:placement`.

- It spells the role and matches the trait, as the helpers do.
- It is not one of the atoms the compiler already predefines for the runtime. Today those are 23 atoms seeded at the
  start of every atom table, `:efficiency`, `:performance`, `:ok`, `:normal`, `:language_error`, `:memory_fault`,
  `:explicit`, `:unknown`, `:noproc`, `:permanent`, `:transient`, `:temporary`, `:one_for_one`, `:one_for_all`,
  `:rest_for_one`, `:insert`, `:lookup`, `:delete`, `:found`, `:not_found`, `:unknown_op`, `:plain`, `:dangerous`
  (`seed_actor_runtime_atoms` in `src/emitter/<target>/atoms/atom_table.silica`). None of those is used as a
  registered name.
- A search of `trials/` and `stdlib/` on 2026-09-19 found no source that uses `:placement` at all, so claiming it breaks
  no program.
- It fits the registry conventions: registered names are atoms fixed at build time (§20.3.1), and prefixes already
  mark registries (`dangerous_` names belong to the dangerous registry and nowhere else, FFI specification §4.9.3). A
  single claimed name is the smallest form of the same idea.

**Which registry.** The ordinary registry, which already holds supervisors' names (§20.3.1), with `:placement` claimed:
only the runtime registers it, and only for the placement actor. (A separate fourth table, as for FFI and device workers,
is possible; Q13.)

**Every other registration path rejects it at compile time (E2303):**

| Path | Where specified |
|---|---|
| `spawn_registered(state, behavior, :placement, …)` | §15.1.1 |
| `spawn_registered_supervisor(T, state, :placement, …)` | §15.1.1, §15.4.8.1 |
| `register(:placement, ref)` and `unregister(:placement)` | §20.3.1 |
| a child specification `id: :placement` whose behaviour is not `placement_behavior(T)` (a supervised child is registered under its `id`, as `trials/modules_addition/sd3_cross_unit_atoms_registered_name.silica` relies on) | §15.4.13.2, §20.3.1 "Names across restarts" |
| `spawn_device_registered(state, behavior, :placement, …)` and `cast_device_registered(:placement, …)` | §15.1.1; device specification |
| `spawn_dangerous_registered(state, behavior, :placement, …)` and `cast_dangerous_registered(:placement, …)` (already illegal by the `dangerous_` prefix rule; E2303 is reported first because it names the real reason) | FFI specification §4.9.2, §4.9.4 |
| `cast_registered(:placement, …)`, `call_registered(:placement, …)` and `whereis(:placement)` (the placement actor is reached only through `cast_placement` and `call_placement`) | §20.3.1 |

The name is checked on atom **literals**. A name computed at run time is not possible for these paths, because names
are atoms and no atom is created at run time (§4.1.7); a variable holding `:placement` passed to `register` is caught at
run time by the claim (`register` returns `:name_taken`) and by the check that only the runtime may register it.

**A placement actor started under another name** is impossible by construction for `start_placement` (it takes no name).
For the supervised path, a child specification whose behaviour is `placement_behavior(T)` must have `id: :placement`;
any other id is E2304.

**Atom identity.** An **application is one compilation unit**
([APPLICATION_IS_ONE_COMPILATION_UNIT.md](APPLICATION_IS_ONE_COMPILATION_UNIT.md)), so its atoms are numbered once, in
one table (§4.1.7, "One table per program (normative)"). `:placement` is simply one atom of the application, whose
spelling the compiler knows to be reserved, and "one owner of the reserved name" is a whole-application rule checked
over the whole application. The runtime needs the atom's index (the registry is an atom-indexed table,
[atom_actor_registry_direct_index_design.md](atom_actor_registry_direct_index_design.md)), so the proposal adds
`:placement` to the predefined runtime atoms at a fixed index (23, directly after `:dangerous`), and runtime code uses
that index.

**Temporary, while builds are split into units.** The split build (one unit per file, to keep the compiler's memory
down) is a work-around, and the per-unit atom numbering it produces today is a defect of that work-around, not the
language's model: SD-3, shown by `trials/modules_addition/sd3_cross_unit_atoms.silica` and
`sd3_cross_unit_atoms_registered_name.silica` (a registered name minted in one unit misses the same atom minted in
another, and the cast is silently dropped). Meanwhile the split build must keep **every** atom's single identity: the
units share one application-wide atom table (for example, built from the units' interface files in dependency order and
handed to each unit), with the predefined atoms, `:placement` among them, at their fixed indices. The fixed index keeps
`:placement` correct in every unit even before SD-3 is fixed, but it is not a substitute for that fix, which every other
atom needs. The E2303 checks are likewise whole-application checks; in the split build they run per unit on literals
and need no cross-unit information, because the reserved spelling is known to every unit. When the application compiles
as one unit, the shared table and these notes go away and nothing in this design changes.

Cost of the fixed index: adding a predefined atom shifts every other atom's index by one, so goldens that print atom
indices or exit with them change once (the trial golden traps note this). Q14.

### 7.6 Starting the placement actor

```
start_placement(placement_impl_type, initial_state, stack_policy [, core_id]) -> :ok   proc[concurrency]
placement_behavior(placement_impl_type)                                                  -- compile time
```

- **`start_placement`** is a new built-in, not a spawn form, so no spawn form changes. It is accepted only in `main`,
  and should come first in `main`'s sequence, after the `FailureReporter` (§15.4.13.4) and before the actors it will
  place. It starts the placement actor, registers it as `:placement`, and calls `init` once with `initial_state`.
  There is no name argument: the name is fixed.
- It takes a **`stack_policy`**, like every actor start (§15.1.2.2): the placement actor is an actor, and nothing about
  its stack is hidden. Its dispatches are short and frequent, so `:keep_high_water` is the natural choice.
- The optional **core id** pins the placement actor itself. It is never a placed actor, stealing never takes it, and no
  moving option moves it. Guidance: give it a core that does not run long dispatches (Section 7.10).
- **Supervised start.** A supervisor may start the placement actor instead, from a child specification with
  `id: :placement` and `behavior: placement_behavior(T)`, exactly as a state machine is started from
  `state_machine_behavior(ImplType)` (§15.5.6). The child specification's shape does not change. The child's stack
  policy is its supervisor's, as for every supervised child in the chunk 1 implementation (the specification is silent
  there; Appendix A).
- **Actors spawned before the placement actor starts** are ordinary pinned actors. Nothing happens to them. They may be
  handed off once it is running; a hand-off before then returns `:placement_unavailable` and changes nothing.
- **An `init` that fails** fails the start: in `main` the failure is `main`'s (a fatal report, as for any failure in
  `main`), under a supervisor it is a failed child start.

### 7.7 The hand-off

```silica
cast_placement(PlacementMessage) -> :ok | :placement_unavailable | :already_placed | :not_placed | :actor_not_found
    proc[concurrency]
```

```
PlacementMessage =
    { op: :place, actor: actor_ref, key: uint64, mobility: :movable | :fixed }
  | { op: :place_supervisor, supervisor: supervisor_ref, mobility: :movable | :fixed }
  | { op: :release, actor: actor_ref }
```

The hand-off is a **cast** to the placement actor: the sender does not wait for, and does not need, the placement
decision. The result atom is computed at the sender from the actor's control block, without a round trip:

| Result | Meaning |
|---|---|
| `:ok` | the request is queued; for `:place` the actor is now pending (below) |
| `:placement_unavailable` | no placement actor is running (not started yet, or failed and not yet restarted); nothing changed |
| `:already_placed` | `:place` for an actor that is already placed or pending |
| `:not_placed` | `:release` for an actor that is not placed |
| `:actor_not_found` | the actor has ended |

`key` and `mobility` are required fields, like every field of a Silica record: `key` is used by `:by_key` and reported
by `:members`; `mobility: :fixed` means "place, never move". Device workers cannot be handed off: `actor` has type
`actor_ref`, and a `device_actor_ref` there is an ordinary type error. FFI workers (`dangerous_actor_ref`) cannot be
handed off in this version (Q5). A state machine's reference is an `actor_ref` and can be.

**When placement takes effect, and why it is almost free.**

1. `cast_placement({ op: :place, … })` marks the actor **pending** in its control block (one compare-and-swap) and
   queues the request on the placement actor's hand-off queue.
2. A carrier never starts the **first** dispatch of a pending actor. An actor that has never dispatched and is pending
   is **held** until the placement actor places it, or for at most 1 ms (Section 4.1).
3. The placement actor, in its next turn, applies the striping option, moves the actor to the chosen core if it differs
   (on hosted targets by re-pinning it and, if it has a queued message, transferring its ready-queue entry; on raw
   targets through `migrate_actor`), marks it placed, and releases the hold.

A freshly spawned actor has no committed stack (§15.1.2.2: nothing is committed at spawn; the first stack access of the
first message faults in the first page). Moving it before its first dispatch therefore moves a control block pointer and
a mailbox, nothing else, and its first stack page is later committed on the placed core's NUMA node.

**Messages already sent to it** wait in its mailbox. The mailbox moves with the actor atomically and in order
(§15.1.2.4.1), so they are processed on the placed core, first to last. The hold means that even if the spawner casts to
the actor immediately after the hand-off, the first dispatch runs on the placed core.

**When it is not free.** If the actor dispatched before the hand-off (a registered actor that others messaged first, a
supervised child that received messages, a state machine whose `init` runs when it starts, or a hold released after 1 ms
because the placement actor was delayed), the move takes effect at its next dispatch boundary, like any `migrate_actor`.
It then leaves behind the stack its release algorithm retains (nothing under `:release_on_return`) and its cache
warmth. A program gets the free case by handing an actor off in the same sequence that spawned it, before sharing its
reference.

**Cost** (per placed actor, once in its life): the sender's compare-and-swap and enqueue; a share of one placement-actor
turn (hand-offs are drained in batches); the striping rule; and, if the core changes, one ready-queue transfer. Section
6.1 compares this with the BEAM.

**Release.** `cast_placement({ op: :release, actor: ref })` takes an actor out of placement. It stays pinned on its
current core (there is no unpinned state) and is moved only by the program from then on.

### 7.8 A supervisor's children

`cast_placement({ op: :place_supervisor, supervisor: sup, mobility: m })` hands over a supervisor and its whole subtree.
Neither spawn nor child specifications change.

- At the sender, the runtime marks the supervisor's child table as **placed** and marks each current child pending
  (once, in proportion to the number of children). Children that have not dispatched are held from that moment, so they
  are placed before their first dispatch at almost no cost; children that have dispatched move at their next boundary.
- From then on, every child the runtime starts for that supervisor, whether declarative, added by `:add_child`, or a
  restart, is **created pending**. Because the runtime starts it, the runtime also posts the placement request itself,
  into the placement actor's ingress (§7.10), the way it posts exit notifications into a supervisor's supervision ingress
  (§15.4.9.3). Restarted and dynamic children are therefore always placed before their first dispatch.
- Child supervisors' subtrees are included, recursively. The supervisor actor itself is placed too.
- Keys are derived from the supervisor's position and the child's `id`, both stable across restarts (§15.1.4,
  §15.4.13.2), so under `:by_key` a restarted child returns to its core. `mobility` applies to the whole subtree.
- The one window that is not free: a supervisor's declarative children are started inside `spawn_registered_supervisor`,
  before the program can send the hand-off. If another actor messages one of them by name in between, that child has
  dispatched and moves at its next boundary. Handing the supervisor off in the statement right after its start makes
  the window a few instructions wide.

**An alternative that closes that window entirely** would be a fourth `init` field, `supervisors: List[atom, mem(normal)]`,
naming the supervisors whose subtrees the placement actor governs from their first child. It changes neither spawn nor
child specifications, but it is an addition to the flags, so it is asked as Q1 rather than assumed.

### 7.9 Read-only queries

The one place a caller needs an answer, so the one place a call is used:

```silica
call_placement(PlacementQuery) -> {
    tag: :members | :count | :loads | :where | :error,
    members: List[
        {
            actor: actor_ref,
            core: uint64,
            key: uint64,
            mobility: :movable | :fixed,
            moves: int64
        },
        mem(normal)
    ],
    count: int64,
    loads: List[
        {
            core: uint64,
            placed: int64,
            ready: int64,
            busy_ppm: int64
        },
        mem(normal)
    ],
    core: uint64,
    error: atom
} proc[concurrency]
```

```
PlacementQuery =
    { op: :members }
  | { op: :count_members }
  | { op: :loads }
  | { op: :where, actor: actor_ref }
```

`:loads` gives one row per listed core, busy share in parts per million over the last window. `:where` gives the core of
one actor. When no placement actor is running, the reply is `{ tag: :error, error: :placement_unavailable, ... }` rather
than an `actor_not_found` failure, so a monitoring actor survives a placement restart. There are no operations that
change options (Section 12).

### 7.10 Ingress, priority, and not being starved

The placement actor has three queues, like a supervisor's ingress and call queue (§15.4.9.2):

| Queue | Contents | Order each turn |
|---|---|---|
| Placement ingress | runtime-posted events: a supervised child started under a placed supervisor, a placed actor ended, the window tick, the one-second tick, and on raw targets idle-core notices | drained first, completely |
| Hand-off queue | `cast_placement` requests | drained next, completely (batched) |
| Query queue | `call_placement` requests | one or more after that |

The ingress is populated only by the runtime, never by user casts, as §15.4.9.3 requires of the supervision ingress.
The placement actor is kept from being starved by the actors it manages in four ways:

1. **Dispatch priority.** Its carrier starts the placement actor's dispatch ahead of every other actor on that carrier
   whenever any of its queues is non-empty (a runtime-reserved level above `:high`). Its turns are short (the cost of the
   queued requests), so the bounded-fairness rule of §23.1.1 still holds for the others.
2. **Nothing urgent waits for it.** Stealing and enqueue redirection run in the carriers from the tables it last
   published; a delayed placement actor delays only hand-offs, window updates and message-following.
3. **Bounded holds.** A new actor waits at most 1 ms for its placement, then runs where it is.
4. **Its own core.** The one thing that can delay it is a long dispatch already running on its core (there is no
   preemption). Guidance: pin it with `start_placement`'s core id to a core whose actors have short dispatches, or to
   a core outside the `cores` list.

### 7.11 Supervision, failure and restart

- **Under the top supervisor.** Yes, it may be supervised: the top supervisor lists a child specification with
  `id: :placement` and `behavior: placement_behavior(T)`. Guidance: `restart: :permanent`, and a `:one_for_one`
  strategy (or a supervisor of its own), so that a placement failure does not restart the application's other children.
- **Started from `main`.** Then it is a root actor: if it fails, it is not restarted, and placement ends for the rest of
  the program.
- **While it is down** (failed, not yet restarted, or never restarted):
  - every actor stays pinned where it is; no actor is unpinned;
  - **no moves**: carriers treat their published tables as withdrawn, so no enqueue redirection and no stealing of
    placed actors;
  - held actors are released and dispatch where they are;
  - `cast_placement` returns `:placement_unavailable`, and `call_placement` replies with that error.
- **When it comes back.** The restarted actor calls `init` again. Placement state (placed, key, mobility, preferred
  core) lives in the placed actors' control blocks, which belong to the runtime, not in the placement actor's state, so
  every actor placed before the failure is still placed and is governed again at once. `init` must return the same
  options it returned before (changing options is Section 12); if it does not, the restart fails with
  `(:explicit, :placement_options_changed)`. Hand-offs sent while it was down were refused (`:placement_unavailable`),
  so the senders know; nothing is silently lost.

### 7.12 Composition

- **Explicit pins and `migrate_actor`.** An actor spawned with a core id is never placed unless handed off. Calling
  `migrate_actor` (or a `pin_actor_to_*` helper) on a placed actor is honoured with its usual result atoms, and the
  actor leaves placement: an explicit program move always beats a policy. The alternative, `:migration_blocked`, is Q3.
- **`stack_policy`.** Independent, with one interaction: the cost of a move is the stack the actor retains.
  `:release_on_return` actors retain nothing; `:keep_high_water` actors may leave retained pages on the old core's node,
  where lazy page migration (design §4.3) or the next release deals with them. The balancer prefers actors with less
  retained stack. The `:release_when_idle` idle sweep runs on the actor's current carrier.
- **`priority_level`.** Stealing takes `:high` actors first, as the BEAM steals highest priority first. Priority never
  changes which core striping chooses.
- **Dangerous actors.** Not handed off in this version (Q5); pinned explicitly, which is the dirty-scheduler separation.
- **Device actors.** Cannot be handed off (a type error); a device worker owns its device's interrupts on raw targets.
- **State machines.** Handed off like any actor; their `init` runs when they start, so the move may come after it
  (§7.7).
- **`FailureReporter`.** Started before the placement actor, as it is started before any other actor (§15.4.13.4); it
  may be handed off like any actor, or left pinned.

### 7.13 An example

The realized trait, in its own module (the type of `init`'s result is spelled in full; Silica has no type aliases):

```silica
// app_placement.silica
use Placement;

impl AppPlacement for Placement;

fn init(initial_state: int64) -> {
    striping: :round_robin | :least_loaded | :with_spawner | :by_key | :fill_in_order,
    moving: :never | :steal_when_idle | :balance_periodically | :compact_when_quiet | :follow_messages,
    cores: List[uint64, mem(normal)]
} {
    sequence proc[mem(normal)]
        cores: List[uint64, mem(normal)] <- [1, 2, 3, 4]
    produces
        pure { striping: :least_loaded, moving: :steal_when_idle, cores: cores }
    end
}
```

`main`, which starts the placement actor first, then spawns as it does today and hands actors off:

```silica
// main.silica
use Supervisor;
use app_placement;
use app_supervisor;

fn handle_request(msg: int64, state: int64) -> (:reply, int64, int64) {
    (:reply, msg * 2, state)
}

fn main() -> int64 {
    sequence proc[concurrency, mem(normal)]
        // The placement actor, pinned to core 0, outside the cores it manages.
        _: atom <- start_placement(AppPlacement, 0, stack_policy(0, :keep_high_water), 0);

        // Spawn exactly as today, then hand off before anyone else can send to it.
        worker: actor_ref <- spawn(0, handle_request, stack_policy(0, :release_on_return));
        _: atom <- cast_placement({ op: :place, actor: worker, key: 7, mobility: :movable } impl PlacementMessage {});

        // A supervisor and its whole subtree, handed off right after it starts.
        sup: supervisor_ref <- spawn_registered_supervisor(AppSup, 0, :app_sup, stack_policy(0, :keep_last_message));
        _: atom <- cast_placement({ op: :place_supervisor, supervisor: sup, mobility: :movable } impl PlacementMessage {});

        answer: int64 <- call(worker, 21 impl ActorMessage {})
    produces
        pure answer
    end
}
```

The supervised alternative puts the placement actor in the top supervisor's `init` list instead of calling
`start_placement`:

```silica
{
    id: :placement,
    agent_type: :worker,
    initial_state: 0,
    behavior: placement_behavior(AppPlacement),
    restart: :permanent,
    shutdown: 0,
    flavor: :plain
}
```

A real program derives its core list from `get_cpu_topology()` or `get_efficiency_cores()` in `main` and passes it in
`initial_state`.

### 7.14 The `PlacementProvider` trait: the behaviour, on raw targets

On hosted targets the placement actor's behaviour is native runtime code (Section 8). On raw targets it is a Silica
library that implements this trait; the runtime calls these functions as the placement actor's behaviour, the way it
calls a supervisor's `init` and then runs the supervisor behaviour:

```silica
export trait PlacementProvider;
export start/1;
export place/2;
export on_window/2;
export on_idle/3;
export on_leave/2;

required {
    // Called once when the placement actor starts, with the options init returned.
    fn start(flags: {
        striping: :round_robin | :least_loaded | :with_spawner | :by_key | :fill_in_order,
        moving: :never | :steal_when_idle | :balance_periodically | :compact_when_quiet | :follow_messages,
        cores: List[uint64, mem(normal)]
    }) -> ActorState;

    // Called for each hand-off (and each child the runtime starts under a placed supervisor);
    // returns the core the actor is placed on.
    fn place(request: {
        actor: actor_ref,
        key: uint64,
        spawner_core: uint64,
        mobility: :movable | :fixed
    }, state: ActorState) -> (uint64, ActorState);

    // Called on each window tick with every listed core's load; returns the moves to make.
    fn on_window(loads: List[
        { core: uint64, placed: int64, ready: int64, busy_ppm: int64 },
        mem(normal)
    ], state: ActorState) -> (List[{ actor: actor_ref, target_core: uint64 }, mem(normal)], ActorState);

    // Called when a listed core runs out of ready actors; returns the moves that feed it.
    fn on_idle(core: uint64, loads: List[
        { core: uint64, placed: int64, ready: int64, busy_ppm: int64 },
        mem(normal)
    ], state: ActorState) -> (List[{ actor: actor_ref, target_core: uint64 }, mem(normal)], ActorState);

    // Called when a placed actor ends, is released, or is migrated by the program.
    fn on_leave(actor: actor_ref, state: ActorState) -> ActorState;
}
```

The raw library, `placement_raw`, implements it for all ten options, so a program written against `Placement` runs
unchanged. Like every Silica library it is `.silica` source compiled into the application, not a separately built
module. At most one `PlacementProvider` may be realized in an application (a whole-application rule, checked like
E2301). How a raw build selects it (the board
pack, or a `use` in the program) is Q6; whether a hosted program may supply its own provider is Q4.

### 7.15 Compile-time rules

| Rule | Diagnostic |
|---|---|
| A second `impl … for Placement` in the application | E2301 |
| More than one start site (`start_placement` and `placement_behavior(T)` together, or two of either) | E2302 |
| `:placement` used on any other registration or registry path (§7.5 table) | E2303 |
| A child specification with `behavior: placement_behavior(T)` whose `id` is not `:placement` | E2304 |
| `start_placement` outside `main` | E2305 |
| `start_placement`, `placement_behavior`, `cast_placement` or `call_placement` in an application with no realized `Placement` | E2306 |
| `striping` or `moving` not an atom literal from the lists, as a release atom must be a literal | E2307 |
| `start_placement(T, …)` or `placement_behavior(T)` where `T` is not the realized `Placement` implementation | E2308 |
| `init`'s record without all three fields | the existing record type error (E2005 in today's goldens) |
| A device or dangerous reference in a `:place` request | the existing type error |

The codes E2301–E2308 are proposals in an unused range (the compiler's codes today stop at E2114 in the E21xx range,
and the device-actor specification suggests E2201 onward for its family); the messages are in Appendix A.

---

## 8. Hosted implementation (runtime-owned behaviour)

**Depends on chunk 12.** Everything in this section assumes carrier threads, one per core, each running many actors
and switching only at yield points (ROADMAP chunk 12). Today's thread-per-actor runtime can run the placement actor,
accept hand-offs, apply striping by setting the placed actor's thread affinity, and honour `:never`; nothing else is
meaningful until carriers exist, because the OS already balances threads.

### 8.1 Where it lives

- **Front end** (shared by every path): `stdlib/Placement.silica` and `stdlib/PlacementProvider.silica`; interner
  kinds for `start_placement`, `placement_behavior`, `cast_placement`, `call_placement`, `PlacementMessage` and
  `PlacementQuery`; type-checker rules next to the `stack_policy` and supervisor checks
  (`type_checker_expressions_actor_spawn.silica`); the claimed-name checks on every registration path; the
  one-per-application checks at application assembly; `:placement` added to `seed_actor_runtime_atoms` in each
  emitter's `atoms/atom_table.silica`.
- **Runtime** (per hosted emitter): a new module beside the stack runtime,
  `emitter/<target>/terms/prims/prims_actors_placement_asm.silica`: the placement actor's behaviour (hand-off queue,
  ingress, window step, message-following merge, queries), the pending and hold logic, enqueue-time redirection and
  stealing. It shares the carrier structures of chunk 12's scheduler module.

### 8.2 The placement actor on hosted targets

- It is an ordinary actor to the scheduler (a control block, a stack under its `stack_policy`, a core), with the
  reserved dispatch priority of §7.10.
- Its behaviour is native: each turn drains the ingress, then the hand-off queue, then queries. Window and one-second
  ticks come from the runtime's timer (chunk 12's `send_after` machinery, posted into the ingress, not the mailbox).
- It **decides**; the carriers **act**. It publishes, per listed carrier, the migration table and the active-prefix
  flag, and per placed actor the preferred core. Carriers read those without locks when they enqueue and when they
  steal.
- For stateless striping rules the runtime may choose the core inside `cast_placement` at the sender (Section 6.1,
  Q11); the placement actor still records the actor.

### 8.3 How it uses the carriers

- One carrier per core named by the `cores` list or by an explicit pin; a core named by neither has no carrier.
- Each carrier has, per priority level, a **fixed** ready queue (actors not placed, or placed with `mobility: :fixed`,
  or placed under `:never`) and a **movable** ready queue (placed actors the moving option may move). The fixed queue
  is never stolen from. The dispatch pick alternates fairly between them so the bounded-fairness rule of §23.1.1 holds.
- The movable queue is a Tokio-style bounded single-producer multi-consumer ring with an overflow list [B16]: the owner
  pushes and pops without read-modify-write atomics; thieves take half with one compare-and-swap.
- An idle carrier searches, then spins briefly, then sleeps on its futex (Linux) or `__ulock` (macOS). At most half of
  the idle listed carriers search at once, and a sleeping carrier is woken only when no carrier is already searching
  (Tokio, Go [B14, B16]). `:compact_when_quiet` carriers outside the active prefix do not search.

### 8.4 Statistics

| Where | Field | Updated | Read by |
|---|---|---|---|
| Carrier (own cache line) | busy ns in the current window | at every dispatch end | the window step |
| Carrier | dispatches in the window | at every dispatch end | `:loads` |
| Carrier | ready count (movable, per priority) | enqueue and dequeue | striping, stealing, window step |
| Carrier | start time of the current dispatch | at dispatch start | stealing (stuck carrier) |
| Carrier | published window record (busy ppm, ready, placed) | once per window, by the owner | window step, `:loads`, `:least_loaded`, `:fill_in_order` |
| Carrier | migration table pointer | once per window, by the placement actor | enqueue |
| Carrier | sampled edge table, 1024 entries | 1 send in 64 (`:follow_messages` only) | the one-second merge |
| Actor (control block) | spawner's core | at spawn (every actor) | `:with_spawner` |
| Placed actor (control block, 48 B) | placement state (pending, placed, mobility); key; preferred core; time of last move; placed-actor list links | at hand-off, move, leave | the hold, enqueue, stealing, `:members` |

The chunk 1 control block is 512 bytes with its last field at #496; placement adds 48 bytes (and the spawner's core,
which fits in a word the control block already pads). `get_actor_memory_usage` already exposes `retained`, which the
balancer reads.

### 8.5 The mechanisms

- **Hand-off:** Section 7.7. The hold is a check at dispatch start: "pending and never dispatched" means skip for now
  and re-check after the placement actor's turn or 1 ms.
- **Enqueue:** if the actor is placed and movable, and its carrier's migration table says emigrate (or its preferred
  core differs and the residency has passed), enqueue on the target carrier instead and record the move. This is a
  migration at a dispatch boundary; the mailbox and its order come along because the actor itself is what is enqueued
  (§15.1.2.4.1).
- **Steal:** as Section 4.2. The stolen actors are re-pinned to the thief's core.
- **Window step:** in the placement actor, on the ingress tick; O(C log C) for C listed cores.
- **Handoff on `call`:** when an actor calls another actor on the same carrier, the carrier may run the callee next
  (Go's `runnext`, Tokio's LIFO slot). Go depends on preemption to stop a ping-pong pair from starving others [B15];
  Silica has none, so consecutive handoffs are bounded (for example three, as Tokio bounds its LIFO slot), after which
  the callee goes to the back of the queue. This is scheduler work for chunk 12; it is here because the co-location
  options are what make it apply.

### 8.6 Overhead budget

- Per spawn (every actor, placed or not): one store (the spawner's core).
- Per hand-off: one compare-and-swap and one enqueue at the sender, plus a batched share of a placement-actor turn.
- Per dispatch: one monotonic-counter read and two adds; a check of the pending flag at an actor's first dispatch only.
- Per enqueue of a placed actor: one load and one compare; the redirection itself when it happens.
- Per window: O(C log C) in the placement actor; O(edge table) per second for `:follow_messages`.
- Nothing proportional to the number of actors, except the `:members` reply, which the program asks for.
- **Target:** on the ping-pong and ring benchmarks (Section 10), placed actors under any moving option cost less than 1%
  of throughput against the same actors pinned by explicit core ids, when no move is needed.

### 8.7 Host specifics

- **Linux:** carriers are bound with `pthread_setaffinity_np` to one CPU each (hard eligibility, intersected with
  cpusets; actor_spawn_core_affinity_os_semantics.md). NUMA nodes come from the topology (§22.10), and stacks commit on
  the faulting core's node.
- **macOS:** no hard binding (§15.1.2); `THREAD_AFFINITY_POLICY` is a hint. The one strong lever is QoS: a carrier for
  an efficiency-core entry runs at background QoS, which macOS confines to E cores [B32, B33], so `:fill_in_order` with
  E cores first really packs onto E cores. Background QoS also lowers the carrier's I/O and CPU priority, which is the
  price.
- **Windows** (later path): CPU sets for hard restriction, as the affinity document describes.

---

## 9. Raw implementation (library-provided behaviour)

**Depends on chunk 2 and a raw actor runtime.** On raw paths the chip features of chunk 2 include "actor placement on a
specific core ... migrating it with `migrate_actor`" (ROADMAP); the ESP32-S3 path has no actor runtime yet.

### 9.1 What the platform gives

- **Exclusive, hard pins** (§15.1.2, OS-free): each core's scheduler dispatches only the actors pinned to it; several
  actors may share a core, and the core's scheduler shares its time among them (§23.1.1).
- **`migrate_actor`** with its four atoms, taking effect at the dispatch boundary with order preserved.
- `get_cpu_topology()` and the core lists (§22.10), from the board pack.

A library on top of these changes nothing in the language's raw rule: every move it makes is a `migrate_actor` call,
which is a program move.

### 9.2 The `placement_raw` library

- `start_placement` (or `placement_behavior`) starts the placement actor with the library's `PlacementProvider`
  implementation as its behaviour; `start` receives `init`'s options.
- The pending mark and the hold are **runtime** work on raw targets too (they happen in `cast_placement` and at
  dispatch start); the library's `place` returns the core, and the runtime applies it with `migrate_actor` semantics and
  releases the hold.
- Window ticks come from the raw runtime's timer (chunk 12's timers on that path), posted into the ingress.
- Idle notices: each core's scheduler posts an idle notice into the placement ingress when its queue empties (a new
  runtime hook, §9.4); the library's `on_idle` returns moves, which the runtime applies with `migrate_actor`.
- `:follow_messages` reads the runtime's sampled edge counters (a new primitive).

### 9.3 What a library can and cannot do there

| Can | Cannot |
|---|---|
| Every striping option, exactly. | Steal in nanoseconds: a steal is an idle notice, a turn of the placement actor, and a `migrate_actor`, so microseconds on a microcontroller. |
| Every moving option, at window granularity. | Redirect at enqueue: a move takes effect at the actor's next boundary through `migrate_actor`, not at the enqueue decision (unless the runtime offers the preferred-core primitive, §9.4). |
| Park cores and scale frequency through board device workers (`spawn_device`, peek and poke; porting_for_os_free_targets.md §5), for cores outside the `cores` list or after moving actors off them with `migrate_actor`. | Run while its own core is inside a long dispatch: the placement actor is an actor. On a two-core chip it cannot have a core to itself; its reserved priority bounds its delay by the longest dispatch on its core. |
| Read temperature through a device worker and move actors off a hot core with `migrate_actor` (the core list itself stays fixed until Section 12). | Preempt anything. |

### 9.4 Primitives a raw library needs that do not exist yet

| Primitive | Purpose | Also useful hosted? |
|---|---|---|
| `get_actor_core(ref) -> uint64` | where an actor is now | yes: trials cannot observe placement today |
| `get_core_load(core) -> { ready: int64, busy_ns: int64, window_ns: int64, dispatches: int64 }` | per-core statistics | yes (`:loads` is built on it) |
| `get_actor_schedule(ref) -> { core: uint64, ready: boolean, waiting: boolean, queued: int64, last_moved_ns: int64 }` | movability and residency | yes |
| an idle-notice hook from each core's scheduler into the placement ingress | stealing | no (native there) |
| `sample_sends(core) -> List[{ from: actor_ref, to: actor_ref, count: int64 }, mem(normal)]` | communication edges | no (native there) |
| `set_preferred_core(ref, core) -> :ok \| :actor_not_found \| :invalid_target` | enqueue-time moves without a round trip | no |

All are `proc[concurrency]`, like the rest of §22.10; the ones that act on other actors are accepted only inside a
`PlacementProvider` implementation.

---

## 10. Benchmark plan: parity or better against the BEAM

Benchmarks are programs run outside the trial tree (for example under `programmer_tools/benchmarks/placement/`); they
produce numbers, not goldens. They run after chunk 12 carriers exist.

**Machines.** An Apple silicon Mac (P and E clusters, unified memory), a Raspberry Pi 5 (four Cortex-A76, Linux), the
Linux x86-64 host once that path runs, and a two-socket NUMA machine when one is available.

**BEAM side.** The same Erlang/OTP release on each machine (record the version), JIT enabled, `+P` raised for a million
processes, run in four configurations: default (`+scl true`); `+scl false`; `+sub true`; and `+sbt db` where binding is
supported (Linux).

**Silica side.** Each workload under every striping and moving pair that Section 5 recommends for it, plus the same
actors pinned by explicit core ids and never handed off, as the zero-policy baseline.

| # | Workload | BEAM program | Metrics |
|---|---|---|---|
| B1 | Spawn N actors, N = 10^3…10^6, each waiting for one message | Armstrong's `processes:max/1` (*Programming Erlang*, ch. 8) | spawns per second, time until all are ready, peak RSS |
| B2 | N idle actors, N = 10^6 | the same, plus a hibernated variant | bytes per idle actor |
| B3 | Ring: N actors, M rounds | Armstrong's ring benchmark (*Programming Erlang*, ch. 8 exercise) | messages per second, cross-core sends |
| B4 | Ping-pong pairs, P pairs | Savina ping-pong [B38] | round trips per second, p99 latency |
| B5 | Fan-in, all-to-all, hub, proxy, `gen_server` stress | bencherl `bang`, `big`, `ehb`, `serialmsg`, `genstress` [B39] | throughput, p99 latency, migrations per second |
| B6 | Skewed load: an acceptor spawns and hands off 10^5 short tasks with exponential service times | an equivalent Erlang acceptor | completion time, p99 latency, spread of per-core utilization |
| B7 | Oscillating load: square waves with periods 50 ms, 500 ms and 5 s | the same program | moves per actor per second (thrashing), utilization spread, p99 |
| B8 | One 50 ms dispatch among 1000 ping actors on the same core | the same program | p99 and max latency of the pingers (the BEAM preempts; Silica relies on stealing) |
| B9 | Low load, 5% and 20% | the same program | energy in joules (`powermetrics` package and cluster power on macOS; an external meter on the Pi), wake-ups per second |
| B10 | NUMA: striping over one node's cores versus all cores | the same program with `+sbt tnnps` | remote-access counters (`perf stat`), throughput |
| B11 | Keyed shards (key–value service) | an equivalent sharded Erlang service | throughput, p99, placement stability across restarts |
| B12 | Spawn plus hand-off, N = 10^3…10^6, with and without the send-site latitude (Q11) | B1's program | added time per actor over spawn alone; first-dispatch delay (hold); held actors released by the 1 ms bound |
| B13 | Placement actor killed and restarted under load | a BEAM run with the same load (no equivalent failure) | time with no moves, recovery time, throughput dip |

**Acceptance.** For each option, parity or better than the BEAM's equivalent on the metric the option exists for, in
the BEAM configuration closest to it:

| Option | Compared with | Metric |
|---|---|---|
| `:with_spawner` + `:balance_periodically` | BEAM `+scl false` | B5, B6 throughput and p99 |
| `:with_spawner` + `:compact_when_quiet` | BEAM default | B9 energy, B6 p99 |
| any + `:steal_when_idle` | BEAM default | B6 completion time |
| `:least_loaded` | BEAM default | B6 utilization spread at placement |
| `:with_spawner` + `:follow_messages` | BEAM default | B3, B4, B5 throughput |
| `:by_key` + `:never` | BEAM default | B11 throughput and stability |
| every moving option | BEAM `+sub true` | B7 moves per actor per second (lower is better) |
| the hand-off | BEAM spawn | B12: spawn plus hand-off no slower than BEAM spawn on B1 (with the spawn prerequisites of Section 6.3) |
| mechanisms, all options | BEAM default | B1, B2; overhead against explicit pins under 1% |

Results are reported as measured, including every loss. B8 is expected to show the no-preemption cost for `:never`;
it is in the plan so the number is known.

---

## 11. Trial plan

Trials check behaviour, not speed. Placement outcomes that depend on timing are checked as invariants (counts, bounds,
order), never as exact cores; deterministic options (`:round_robin`, `:by_key`, `:with_spawner`) are checked exactly.
Goldens are hand-derived, as the trial suites require. Because only one `Placement` may be realized per application,
each runtime trial is its own application with its own realized trait. Proposed suite `trials/placement_addition`,
plus `error_enforcement_addition` entries.

**Error enforcement** (`trials/error_enforcement_addition/`):

| Trial | Demonstrates |
|---|---|
| `placement_second_realized_trait` | two `impl … for Placement` in one application: E2301 |
| `placement_second_start_site`, `placement_start_and_child_spec` | two start sites (two `start_placement`, or `start_placement` plus `placement_behavior`): E2302 |
| `placement_reserved_name_spawn_registered`, `…_supervisor`, `…_register`, `…_unregister`, `…_child_id`, `…_device`, `…_dangerous`, `…_cast_registered`, `…_whereis` | `:placement` on every other registration or registry path: E2303 |
| `placement_started_under_another_name` | a child specification with `behavior: placement_behavior(T)` and `id: :workers`: E2304 |
| `placement_start_outside_main` | `start_placement` in a behaviour: E2305 |
| `placement_without_realized_trait` | `cast_placement` in an application with no `impl … for Placement`: E2306 |
| `placement_unknown_striping`, `placement_unknown_moving`, `placement_atom_not_literal` | options are named atom literals: E2307 |
| `placement_wrong_impl_type` | `start_placement` naming a type that is not the realized implementation: E2308 |
| `placement_flags_missing_field` | no defaults: an `init` record without `moving` is a type error |
| `placement_hand_off_device_ref` | a `device_actor_ref` in `:place` is a type error |
| `placement_split_build_second_trait` (temporary, while builds are split) | a second realized trait in another unit is rejected at link time |

**Behaviour** (`trials/placement_addition/`):

| Trial | Demonstrates |
|---|---|
| `placement_round_robin_counts` | 4n hand-offs over n cores give n per core (`get_actor_core`) |
| `placement_first_dispatch_on_placed_core` | a spawned actor handed off at once has committed nothing before its first dispatch (`get_actor_memory_usage` shows 0 committed) and handles its first message on the placed core |
| `placement_messages_before_hand_off_in_order` | a registered actor receives numbered casts before and after its hand-off; it processes all of them in order on the placed core |
| `placement_hand_off_before_start` | a hand-off before `start_placement` returns `:placement_unavailable` and the actor stays where it is |
| `placement_actors_before_start_untouched` | actors spawned before `start_placement` and never handed off never move |
| `placement_already_placed` | a second `:place` of the same actor returns `:already_placed` |
| `placement_by_key_stable_restart` | a supervised child handed off through `:place_supervisor`, killed and restarted, returns to its core |
| `placement_supervisor_subtree` | declarative, `:add_child` and restarted children of a handed-off supervisor are all placed; restarted ones before their first dispatch |
| `placement_with_spawner_same_core` | parent and child share a core; hand-offs from `main` fall back to round robin |
| `placement_fill_in_order_packs` | under no load all placed actors start on the first core |
| `placement_never_no_moves` | under skew, `moves` stays 0 for every placed actor |
| `placement_fixed_mobility_never_moves` | under `:steal_when_idle`, actors handed off `:fixed` never move while `:movable` ones do |
| `placement_message_order_under_moves` | a sender numbers 10^5 casts while stealing moves the receiver repeatedly; the receiver sees them in order |
| `placement_call_reply_across_move` | a caller receives its reply after the callee moved between the call and the reply |
| `placement_no_thrash_oscillating` | under a 50 ms square wave, no actor moves more than once per second |
| `placement_steal_drains_stuck_carrier` | actors queued behind a long dispatch complete on other cores |
| `placement_compact_opens_and_closes` | the active prefix grows under load and shrinks after 1 s idle |
| `placement_follow_messages_pairs` | chatty pairs end on shared cores; a hub stays |
| `placement_explicit_pins_untouched` | explicitly pinned actors on the same cores never move |
| `placement_migrate_actor_releases` | `migrate_actor` on a placed actor succeeds and the actor is no longer in `:members` |
| `placement_release_stays` | `:release` leaves the actor on its core; it never moves again |
| `placement_supervised_restart` | the placement actor under the top supervisor is killed: no moves while it is down, hand-offs return `:placement_unavailable`, and after the restart the actors placed before are governed again |
| `placement_root_failure` | a placement actor started from `main` fails: every placed actor stays on its core for the rest of the run |
| `placement_second_start_at_run_time` | a supervisor holding the placement child specification is started twice; the second start fails with `:placement_already_running` |
| `placement_restart_options_changed` | an `init` whose options depend on a counter in its state returns different options on restart: the restart fails with `:placement_options_changed` |
| `placement_query_while_down` | `call_placement` during the gap replies `{ tag: :error, error: :placement_unavailable }` |
| `placement_raw_library_*` (ESP32-S3, later) | the same scenarios through `placement_raw` on the board |

---

## 12. Future enhancement: changing options while running

Not part of this design. The options are fixed when the placement actor starts, and a restart must reproduce them
(§7.11). Changing them while the application runs would take:

- **Changing the core list** (`:set_cores`): the use cases are parking cores, reacting to heat, and growing onto new
  cores. It needs a list epoch in the published tables; actors on removed cores are moved at their next dispatch
  boundary, placed by the striping option; `:by_key` already uses rendezvous hashing so that only the keys whose core
  left move (Section 3).
- **Switching the moving option**: a **quiesce step** first (stop the window step, withdraw the tables so no new
  redirections or steals start, let in-flight moves finish at their boundaries), then publish the new option's tables,
  resetting every actor's residency clock so the switch itself cannot cause a burst of moves.
- **Switching the striping option**: affects only later hand-offs; the easiest of the three.
- **Pausing moves** (the first draft's `:hold_moves` and `:resume_moves`): a quiesce without a new option.
- **Per-core classes** (for example a separate core set for FFI workers or one per NUMA node, which the first draft's
  placement groups offered): a larger change, since it reintroduces more than one set of options.

Each would come with its own trials (moves bounded during a switch, order preserved, no actor on a removed core after
one dispatch) and its own specification text.

---

## 13. Roadmap placement

- **Proposed new chunk 15, "Actor placement: the placement actor, striping and moving".** Spec §15.6 (new) and the
  Appendix A changes; this document. Closed by the Section 11 trials on each path and the Section 10 benchmarks
  reported. No spawn form or child specification changes.
  - **Prerequisites (hosted):** chunk 12 (carriers per core, yield-point switching, timers). The carrier ready queues of
    chunk 12 should be designed with the movable/fixed split and stealable rings of Section 8.3, and with the
    first-dispatch pending check, so this chunk adds policy rather than restructuring the scheduler.
  - **Prerequisites (raw):** chunk 2 (placement and `migrate_actor` on the chip) and the raw actor runtime; chunk 12's
    timers for the library's windows.
  - **Related:** chunk 11 (identity positions, the atom-keyed registry) for supervised children's keys and for the
    claimed name; the SD-3 fix (one atom table per application) for atom identity in split builds.
- **Early slice.** The front end (traits, the claimed name and its checks, the one-per-application checks,
  `start_placement`, `cast_placement`, `call_placement`, error trials) and the placement actor with striping and `:never`
  can land before chunk 12 on the thread-per-actor runtime, since striping there is setting a thread's affinity.
- **Why not fold it into chunk 12.** Chunk 12 already carries scheduling, fairness, priorities and timers; placement is
  a separable policy layer with its own trials and benchmark gate.
- **Later:** Section 12's option changes, as a separate enhancement request.

---

## 14. Open questions for Lee

Answered or dropped by the revision: making spawn's last argument required (dropped; spawn keeps its shape), whether a
group spawns its members (dropped; there are no groups, and placement never spawns), whether a group may be a
supervisor's child (answered: the placement actor may be, §7.11), what happens when a group ends (answered: pins stay,
no moves until it is back, §7.11), and one moving option versus combinations (answered: one; each option already
includes the stealing it needs).

1. **Q1: Supervisor subtrees.** Hand-off of the supervisor after it starts (proposed; free for children that have not
   dispatched), or an added `init` field `supervisors: List[atom, mem(normal)]` that places named supervisors' children
   before their first dispatch with no window at all? The field changes neither spawn nor child specifications, but it
   is an addition to the flags.
2. **Q2: The `mobility` field.** Keep `:movable | :fixed` as the per-actor variation (proposed), or drop it and let
   programs exclude an actor by not handing it off?
3. **Q3: `migrate_actor` on a placed actor**: honour it and take the actor out of placement (proposed), or refuse with
   `:migration_blocked`?
4. **Q4: Custom providers on hosted targets.** Allowed (with `migrate_actor`-speed moves), or raw only?
5. **Q5: FFI workers.** Leave `dangerous_actor_ref` out of hand-offs (proposed; they are pinned explicitly), or add a
   `:place_dangerous` request?
6. **Q6: Selecting the raw provider.** By the board pack, or by a `use placement_raw;` in the application?
7. **Q7: Constants.** Fixed in the definitions (proposed, like the 32-message window and one-second idle period), or
   numeric fields in the flags record? Numeric fields would each have to be required too.
8. **Q8: The key.** Always required in `:place` (proposed; unused except by `:by_key`), or a second request form without
   a key?
9. **Q9: Core-id list types.** `get_efficiency_cores()` and `get_performance_cores()` return `List[int64,normal]`
   while core ids are `uint64` (§22.10, §4.6); the flags use `uint64`. Reconcile the §22.10 signatures?
10. **Q10: `start_placement` only in `main`** (proposed), or anywhere, with the run-time `:placement_already_running`
    check only?
11. **Q11: Send-site striping.** May the runtime choose the core inside `cast_placement` for stateless striping rules
    (no hold, cheapest hand-off), as an implementation of the runtime-owned behaviour?
12. **Q12: One core set per application.** The single realized trait drops the first draft's per-group core sets (one
    group per NUMA node, dedicated FFI cores). Acceptable for now, with per-core classes as a later enhancement
    (Section 12)?
13. **Q13: The registry for `:placement`.** The ordinary registry with a claimed key (proposed), or a table of its own
    like the FFI and device registries?
14. **Q14: The fixed atom index.** Adding `:placement` to the predefined atoms shifts every other atom's index by one
    and changes goldens that print atom indices or exit with them, once. Accept that churn when the feature lands?
15. **Q15: Options on restart.** Fail a restart whose `init` returns different options (proposed), or accept the new
    options (which is option changing, Section 12)?

---

## Appendix A: Proposed specification changes

All proposals; none is applied. No spawn form and no child specification field changes.

| Section | Change |
|---|---|
| §15.1.2, Actor Pinning Policy, "No runtime movement" | Append: "The one exception is an actor the program has handed to the application's placement actor (§15.6): the runtime moves it, acting for the placement actor, under the moving option its `init` named, only among the cores `init` named, and only at the actor's dispatch boundary. Such a move is the program's, declared when it started the placement actor and handed the actor over." |
| §15.1.2, "Changing core" | Add: "or when the moving option of the placement actor moves an actor handed to it (§15.6)". |
| §15.1.2, OS-free bullet | Add: "On an OS-free target the placement actor's behaviour is a library that moves actors with `migrate_actor()`; its moves are program moves." |
| §15.1.1 | List `start_placement` and `placement_behavior` next to the other actor starts, as built-ins that are not spawn forms. |
| New §15.6, The Placement Actor | The `Placement` trait and its required flags; the striping table (Section 3) and moving table (Section 4) with their constants; one realized trait and one placement actor per application (§7.4), stated for the one-unit model; the claimed name `:placement` (§7.5); starting it (§7.6); the hand-off, the pending mark and the first-dispatch hold (§7.7); supervisor subtrees (§7.8); `call_placement` (§7.9); the placement ingress and its priority (§7.10), with the §15.4.9 comparison; supervision, failure and restart (§7.11); composition (§7.12); the `PlacementProvider` contract (§7.14); failure reasons `:invalid_core_list`, `:placement_already_running`, `:placement_options_changed`. |
| §4.1.7 | Add `:placement` to the atoms with a fixed index that the runtime relies on, and say that the atom table is one per application ([APPLICATION_IS_ONE_COMPILATION_UNIT.md](APPLICATION_IS_ONE_COMPILATION_UNIT.md)). |
| §15.1.2.1, Energy Efficiency Optimization | "Rebalancing: never done by the runtime" becomes "never done by the runtime for actors not handed to the placement actor; see §15.6". |
| §15.1.2.4 and §15.1.2.4.1 | State that placement moves, including the move of a held actor before its first dispatch, are migrations with the same queue transfer, FIFO and atomicity guarantees. |
| §15.4.8.1, §15.4.13.2 | A supervisor may start the placement actor from a child specification with `id: :placement` and `behavior: placement_behavior(T)` (as `state_machine_behavior`, §15.5.6). A handed-off supervisor's children are created pending; their keys derive from the supervisor's position and the child's `id`. State the stack policy of supervised children (the chunk 1 implementation gives them the supervisor's; the specification is silent). |
| §15.4.9 | Note the placement ingress as a second runtime-populated ingress of the same kind. |
| §16.2.6.6 | Placement moves are runtime-handled like `migrate_actor` requests. |
| §20.3.1 | `:placement` is claimed: only the runtime registers it, for the placement actor; `register`, `unregister`, `whereis`, `cast_registered`, `call_registered`, `spawn_registered`, `spawn_registered_supervisor` and child specification ids reject it at compile time. Same for the device and dangerous registries. |
| §22.10 | Add `get_actor_core`, `get_core_load`, `get_actor_schedule`; on OS-free targets the provider-only primitives of §9.4. Note that `migrate_actor` on a placed actor ends its placement. Reconcile core-list element types (Q9). |
| §23.1.1, Placement bullet | "never balances load by moving actors between cores" becomes "never moves an actor that was not handed to the placement actor; placed actors move only as its moving option says (§15.6)". Add the reserved dispatch priority of the placement actor. |
| §23.1.3 | "Program Migration" bullet: add the placement actor as the program's declared migration policy. |
| Diagnostics table | The codes below. |

**Proposed diagnostics** (numbers are proposals in an unused range; the wording follows the compiler's existing
one-sentence messages):

| Code | Message |
|---|---|
| E2301 | `Placement` is realized twice: `impl <B> for Placement` in `<file>` and `impl <A> for Placement` in `<file>`; an application realizes `Placement` at most once. |
| E2302 | the placement actor is started in two places (`<site 1>`, `<site 2>`); an application starts it at most once, with `start_placement` in `main` or one `placement_behavior` child specification. |
| E2303 | `:placement` is the registered name of the application's placement actor and cannot be used by `<operation>`. |
| E2304 | a child specification that starts the placement actor must have `id: :placement`, not `<id>`. |
| E2305 | `start_placement` may be called only in `main`. |
| E2306 | `<operation>` needs a `Placement` implementation, and this application has none. |
| E2307 | `<atom>` is not a `<striping \| moving>` option; the options are `<list>`, written as atom literals. |
| E2308 | `<T>` is not this application's `Placement` implementation (`<A>` is). |

## Appendix B: Sources

- [B1] Erlang/OTP, *erl* command reference (scheduler flags `+S`, `+sbt`, `+scl`, `+sub`, `+sbwt`, `+swt`, `+SDcpu`, `+SDio`). <https://www.erlang.org/doc/apps/erts/erl_cmd.html>
- [B2] E. Stenman, *The BEAM Book*, "Scheduling". <https://github.com/happi/theBeamBook/blob/master/chapters/scheduling.asciidoc>
- [B3] The BEAM Book and H. Soleimani, "Erlang Scheduler Details and Why It Matters" (reductions, `CONTEXT_REDS`). <https://hamidreza-s.github.io/erlang/scheduling/real-time/preemptive/migration/2016/02/09/erlang-scheduler-details.html>
- [B4] Erlang/OTP source, `erts/emulator/beam/erl_vm.h` (`CONTEXT_REDS 4000`). <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_vm.h>
- [B5] Erlang/OTP source, `erts/emulator/beam/erl_process.c` (`check_balance`, `try_steal_task`, `try_steal_task_from_victim`, `erl_create_process`, `ERTS_RUNQ_CHECK_BALANCE_REDS_PER_SCHED`, `ERTS_SCHED_UTIL_*`). <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process.c>
- [B6] Erlang/OTP source, `erts/emulator/beam/erl_process.h` (`erts_check_emigration_need`); The BEAM Book, migration. <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process.h>
- [B7] R. Green, erlang-questions, October 2012, on scheduler compaction of load. <http://erlang.org/pipermail/erlang-questions/2012-October/069585.html>
- [B8] Erlang/OTP, *erl_nif* (dirty NIFs; the one-millisecond guideline). <https://www.erlang.org/doc/apps/erts/erl_nif.html>
- [B9] J. L. Andersen, "Erlang Dirty Scheduler Overhead". <https://jlouis.github.io/posts/erlang-dirty-scheduler-overhead/>
- [B10] Erlang/OTP System Documentation, "Memory Usage" (process: 338 words including a 233-word heap). <https://www.erlang.org/doc/system/memory.html>
- [B11] Erlang/OTP Efficiency Guide, "Processes" (327 words; messages are copied). <https://www.erlang.org/doc/system/eff_guide_processes.html>
- [B12] Erlang/OTP, "Erlang Garbage Collector" (per-process generational semi-space copying collector; heap growth). <https://www.erlang.org/doc/apps/erts/garbagecollection.html>
- [B13] J. Armstrong, *Programming Erlang* (2007), ch. 8, process spawn timing, as quoted in "Erlang process spawning performance". <https://globalengineer.wordpress.com/2008/09/23/erlang-process-spawning-performance/>
- [B14] D. Vyukov, *Scalable Go Scheduler Design Doc*. <https://golang.org/s/go11sched>
- [B15] Go source, `src/runtime/proc.go` (global queue every 61 ticks, `runqsteal` steals half, `runnext` and sysmon, `forcePreemptNS = 10ms`). <https://go.dev/src/runtime/proc.go>
- [B16] C. Lerche, "Making the Tokio scheduler 10x faster" (2019). <https://tokio.rs/blog/2019-10-scheduler>
- [B17] R. D. Blumofe and C. E. Leiserson, "Scheduling Multithreaded Computations by Work Stealing", JACM 46(5), 1999. <https://dl.acm.org/doi/10.1145/324133.324234>
- [B18] D. Chase and Y. Lev, "Dynamic Circular Work-Stealing Deque", SPAA 2005. <https://www.dre.vanderbilt.edu/~schmidt/PDF/work-stealing-dequeue.pdf>
- [B19] `rayon_core` documentation. <https://docs.rs/rayon-core/latest/rayon_core/>
- [B20] Seastar, "Shared-nothing design". <https://seastar.io/shared-nothing/>
- [B21] Akka, "Dispatchers" (typed). <https://doc.akka.io/libraries/akka-core/current/typed/dispatchers.html>
- [B22] Akka, default configuration reference (affinity pool executor, `fair-work-distribution` threshold). <https://doc.akka.io/libraries/akka-core/current/general/configuration-reference.html>
- [B23] Microsoft, "Grain placement" (Orleans placement strategies, activation repartitioning and rebalancing). <https://learn.microsoft.com/en-us/dotnet/orleans/grains/grain-placement>
- [B24] Pony, "Runtime" FAQ and runtime options. <https://www.ponylang.io/faq/runtime/>
- [B25] ponyc pull request #2386, "Dynamic scheduler thread scaling based on workload". <https://github.com/ponylang/ponyc/pull/2386>
- [B26] CAF User Manual, "Scheduler". <https://actor-framework.readthedocs.io/en/stable/core/Scheduler.html>
- [B27] S. Wölke et al., "Locality-Guided Scheduling in CAF" (AGERE 2017). <http://www.inet.haw-hamburg.de/papers/whcs-lsc-17.pdf>
- [B28] Linux kernel documentation, "Scheduler Domains". <https://docs.kernel.org/scheduler/sched-domains.html>
- [B29] Linux kernel documentation, "EEVDF Scheduler"; Phoronix, "EEVDF Scheduler Merged For Linux 6.6". <https://docs.kernel.org/scheduler/sched-eevdf.html>
- [B30] R. van Riel, "Automatic NUMA Balancing" (Red Hat, 2014); SUSE tuning guide. <https://documentation.suse.com/sles/15-SP6/html/SLES-all/cha-tuning-numactl.html>
- [B31] Linux kernel documentation, "Energy Aware Scheduling" (the 80% tipping point). <https://docs.kernel.org/scheduler/sched-energy.html>
- [B32] Apple, "Optimize for Apple Silicon with performance and efficiency cores"; Energy Efficiency Guide for Mac Apps, "Prioritize Work at the Task Level". <https://developer.apple.com/news/?id=vk3m204o>
- [B33] H. Oakley, "What is Quality of Service, and how does it matter?" (background QoS confined to E cores). <https://eclecticlight.co/2025/05/09/what-is-quality-of-service-and-how-does-it-matter/>
- [B34] M. Mitzenmacher, *The Power of Two Choices in Randomized Load Balancing* (thesis, 1996); Mitzenmacher, Richa and Sitaraman, survey (2001). <https://www.eecs.harvard.edu/~michaelm/postscripts/mythesis.pdf>
- [B35] E. Francesquini, A. Goldman, J.-F. Méhaut, "Actor Scheduling for Multicore Hierarchical Memory Platforms", Erlang Workshop 2013. <https://dl.acm.org/doi/pdf/10.1145/2505305.2505313>
- [B36] S. Barghi and M. Karsten, "Work-Stealing, Locality-Aware Actor Scheduling", IPDPS 2018. <https://ieeexplore.ieee.org/document/8425202/>
- [B37] Linux kernel documentation, `/proc/sys/vm/` (`max_map_count`, default 65530). <https://docs.kernel.org/admin-guide/sysctl/vm.html>
- [B38] S. Imam and V. Sarkar, "Savina: An Actor Benchmark Suite", AGERE 2014. <https://github.com/shamsimam/savina>
- [B39] S. Aronis et al., "A scalability benchmark suite for Erlang/OTP" (bencherl), Erlang Workshop 2012. <https://github.com/softlab-ntua/bencherl>
