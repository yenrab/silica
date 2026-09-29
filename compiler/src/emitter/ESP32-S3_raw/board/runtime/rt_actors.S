/*
   Copyright 2026 Lee Scott Barney

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

// Actor runtime for the bare-metal ESP32-S3: the cooperative per-core scheduler of
// silica-specification.md §15.1.2 / §23.1.1, with the entry points the emitter's
// terms/prims/prims_actors.silica calls (the same `silica_rt_*` names as the Apple Silicon runtime
// in apple_silicon_mac/terms/prims/prims_actors_runtime_asm.silica, which is the reference for
// every observable behaviour here; the design is in design_documents/ports/esp32s3_xtensa_port.md
// §7 and the status in design_documents/ports/esp32s3_port_status.md).
//
// Model. There is no OS. Every context -- `main`, each actor, and one scheduler per core -- is a
// windowed-ABI machine stack plus an auxiliary (emitter `SP`) stack, and switches happen only where
// the specification allows: at a dispatch boundary (the behaviour returned) or where the running
// context suspends to wait (`call` awaiting its reply, `wait_for_exit`, an actor with an empty
// mailbox). Nothing is preempted. A core's scheduler dispatches only the actors pinned to it; an
// actor changes core only through migrate_actor (applied at its next dispatch boundary), so pinning
// is exclusive and hard (§15.1.2, OS-free). Fairness: an actor yields to its core's scheduler after
// every dispatch, so actors on one core take turns; ready queues per priority level
// (set_actor_priority, §22.10) pick the next actor, in the order the program's dispatch-order policy
// says (set_scheduler_policy, "Dispatch-order policies" below). Cross-core state (queues, status words,
// mailboxes) is guarded by one S32C1I spinlock, silica_rt_sched_lock, held only for a few
// instructions and never across a switch or a user call. The free lists (ACBs, message nodes,
// stacks) have their own lock, silica_rt_free_lock, taken inside the six list routines: some
// callers hold the scheduler lock and some do not, and the list routines call nothing that locks.
//
// Two cores. Core 0 (the PRO cpu) boots the program; core 1 (the APP cpu) is started by
// silica_rt_actors_init through rt_start.S silica_rt_start_app_cpu and enters its scheduler from
// silica_rt_core1_main. The emitted code's caller-saved virtual registers and its auxiliary stack
// pointer live in a per-core block reached through THREADPTR (rt_start.S silica_rt_core_regs), so
// the two cores run emitted code at the same time without sharing a register home. The heap bump
// is a compare-and-swap (rt_heap.S) and the console has a lock (rt_console.S). NCORES_RUN below is
// how many cores the runtime uses: 2, or 1 to diagnose a two-core problem on one core.
//
// Context switch. silica_rt_ctx_switch spills every live register window above its own to the
// stacks they belong to (the SPILL_ALL_WINDOWS trick: touch a high register in each rotation and
// let the overflow exception spill it), then saves a0 (return address + window increment), a1 and
// the emitter SP and loads the other context's. A context is first entered through a fabricated
// pair of frames at the top of its stack that makes RETW land in a trampoline with the ACB in a2.
// Emitted code keeps its caller-saved virtual registers (X3-X8, D0-D7) in this core's register
// block (THREADPTR); a switch happens only inside a runtime call, which the emitter treats as
// clobbering them, so nothing else needs saving.
//
// Stacks. Actor stacks are fixed-size heap blocks chosen at spawn (spec §15.1.2.2 leaves the rule
// for targets without demand paging to the port): stack_policy reserve 0 = 8 KB machine + 2 KB
// auxiliary; an explicit reserve is split 3/4 machine, 1/4 auxiliary. Both are guarded by the two
// Xtensa data breakpoints, re-armed at every switch for the context that is about to run (main's
// two stacks included), so an overflow is a debug exception that ends only that actor (failure
// reason (:explicit, :stack_exhausted), tag 4) instead of corrupting memory. Release algorithms have
// nothing to release without paging; get_actor_memory_usage reports the block sizes and a measured
// high-water mark (the blocks are zeroed when handed out). Freed stacks, ACBs and message nodes go
// on free lists, because the heap never frees.
//
// Layouts (32-bit words; the two 8-byte slots the emitted code reads keep a zero high word):
//   ACB (256 B)   0 state(8)  8 behaviour(8)  16 alive  20 core  24 target_core (-1 none)
//                 28 status  32/36 mailbox head/tail  40/44/48 ingress head/tail/depth
//                 52 ready link  56/60 machine stack base/size  64/68 aux stack base/size
//                 72/76/80 saved a0/a1/vsp  84 kind (0 actor, 1 main, 2 scheduler)  88 priority
//                 92 spawn waiter  96 supervisor  100 agent_type atom  104 child row  108 first child
//                 112 next sibling  116/120 child table head/len  124 sync_failure_reported
//                 128 trampoline  132 trampoline ran  136 strategy  140 allowed restarts
//                 144 time frame  148 restart count  152 window start  156 exit reason tag
//                 160 active call node  164 exit reason atom  168 policy word  172 wait node
//                 176 registered name+1 (0 none)  180 requested reserve  184 free-list link
//   message node (48 B)  0 next  4 kind (0 cast/send, 1 call, 2 exit notice)  8 payload(8)
//                 16 reply set  20 caller ACB  24 reply(8)  32 ingress kind (0 payload ptr, 1 inline)
//   core block (64 B)  0/4/8 ready heads (high, normal, low)  12/16/20 tails  24 current context
//                 28 scheduler pseudo-ACB  32 busy  36 id  40/44/48 queued actors per ready queue
//                 52/56 weighted-fair tier being served and its remaining credit
//   failure payload (32 B, heap)  0 child  8 reason tag  16 reason atom  24 agent type (8-byte slots)
//
// Result and status conventions follow the reference: spawn -> ACB (the actor_ref), cast -> 1,
// send -> 0, call -> the reply or 0 when the target is dead or dies, remove_actor / kill_abnormal /
// link / demonitor / register -> 0, monitor -> its argument, migrate_actor -> 0 :ok, 1
// :invalid_target, 2 :actor_not_found, 3 :migration_blocked (the emitter maps the code to the
// program's atom indices), wait_for_exit -> 0 once every actor is idle. Atoms the runtime names
// itself are the fixed prefix every executable's atom table starts with (atoms/atom_table.silica
// seed_actor_runtime_atoms): :efficiency 0, :performance 1, :ok 2, :normal 3, :language_error 4,
// :memory_fault 5, :explicit 6, :unknown 7, :insert 15, :lookup 16, :delete 17, :found 18,
// :not_found 19, :unknown_op 20.

#define ACB_STATE        0
#define ACB_BEH          8
#define ACB_ALIVE        16
#define ACB_CORE         20
#define ACB_TARGET       24
#define ACB_STATUS       28
#define ACB_MB_HEAD      32
#define ACB_MB_TAIL      36
#define ACB_IN_HEAD      40
#define ACB_IN_TAIL      44
#define ACB_IN_DEPTH     48
#define ACB_NEXT_READY   52
#define ACB_STK_BASE     56
#define ACB_STK_SIZE     60
#define ACB_AUX_BASE     64
#define ACB_AUX_SIZE     68
#define ACB_CTX_A0       72
#define ACB_CTX_A1       76
#define ACB_CTX_VSP      80
#define ACB_KIND         84
#define ACB_PRIO         88
#define ACB_SPAWN_WAITER 92
#define ACB_SUP          96
#define ACB_AGENT        100
#define ACB_ROW          104
#define ACB_FIRST_CHILD  108
#define ACB_NEXT_SIB     112
#define ACB_CT_HEAD      116
#define ACB_CT_LEN       120
#define ACB_SYNC_REPORTED 124
#define ACB_TRAMP        128
#define ACB_TRAMP_RAN    132
#define ACB_STRATEGY     136
#define ACB_ALLOWED      140
#define ACB_TIMEFRAME    144
#define ACB_RESTARTS     148
#define ACB_WINDOW       152
#define ACB_REASON_TAG   156
#define ACB_ACTIVE_CALL  160
#define ACB_REASON_ATOM  164
#define ACB_POLICY       168
#define ACB_WAIT_NODE    172
#define ACB_REG_NAME     176
#define ACB_RESERVE      180
#define ACB_FREE_NEXT    184
#define ACB_SIZE         256

#define ST_IDLE          0
#define ST_READY         1
#define ST_RUNNING       2
#define ST_SUSPENDED     3
#define ST_DEAD          4
#define ST_WAITQ         5
#define ST_FAULTED       6
#define ST_WAIT_INGRESS  7

#define KIND_ACTOR       0
#define KIND_MAIN        1
#define KIND_SCHED       2

#define MSG_NEXT         0
#define MSG_KIND         4
#define MSG_PAYLOAD      8
#define MSG_REPLY_SET    16
#define MSG_CALLER       20
#define MSG_REPLY        24
#define MSG_INGRESS_KIND 32
#define MSG_SIZE         48

#define CORE_RQ_HEAD     0
#define CORE_RQ_TAIL     12
#define CORE_CURRENT     24
#define CORE_SCHED       28
#define CORE_BUSY        32
#define CORE_ID          36
#define CORE_RQ_LEN      40
#define CORE_WF_TIER     52
#define CORE_WF_CREDIT   56
#define CORE_SIZE        64
#define NCORES_MAX       2

// Dispatch-order policies (set_scheduler_policy; design_documents/ports/esp32s3_xtensa_port.md §8.10).
// One policy is in force for the whole program and every core applies it to its own ready queues;
// it decides only which READY actor of a core runs next -- never when a running one stops (nothing
// preempts between the yield points of §15.1.2) and never which core an actor is on.
//   PRIORITY_FIFO   the default: the head of the highest-priority non-empty queue.
//   ROUND_ROBIN     one flat queue: every push lands in the normal queue, whatever the actor's
//                   priority, so ready actors run in strict arrival order.
//   WEIGHTED_FAIR   deficit round robin over the three priority tiers with a unit cost per dispatch:
//                   the tier being served runs its head while it has credit and actors, then the
//                   next tier starts with a fresh quantum; an empty tier forfeits its quantum (DRR
//                   zeroes the deficit of an idle queue). Quanta: high 4, normal 2, low 1 -- the
//                   weight classes; the shares are per tier, FIFO within a tier.
//   LOTTERY         Waldspurger-Weihl lottery: a tier holds weight x (actors queued in it) tickets
//                   with the same weights, a xorshift32 draw picks the tier in proportion to its
//                   tickets and the tier's head runs. Choosing the tier and then its head keeps a
//                   pick O(1); FIFO within the tier gives every actor of a tier the same turns, so
//                   an actor's expected share is its ticket share.
// A policy change takes effect at the next push or pop on every core and leaves queued actors where
// they are: a switch to ROUND_ROBIN drains what already waits in the high and low queues in the old
// order first, a switch away from it treats what waits in the flat queue as normal until each
// actor's next push. Only the queue chosen at push and at pop depends on the policy.
#define POLICY_PRIORITY_FIFO 0
#define POLICY_ROUND_ROBIN   1
#define POLICY_WEIGHTED_FAIR 2
#define POLICY_LOTTERY       3
#define POLICY_COUNT         4
#define WEIGHT_HIGH          4
#define WEIGHT_NORMAL        2
#define WEIGHT_LOW           1

#define DEFAULT_MACHINE_STACK 8192
#define DEFAULT_AUX_STACK     2048
#define MIN_MACHINE_STACK     1024
#define MIN_AUX_STACK         512
#define SCHED_STACK_SIZE      4096
#define REGISTRY_SLOTS        240
#define REGISTRY_RESERVE      2048
#define INGRESS_LIMIT         1024

#define ATOM_EFFICIENCY  0
#define ATOM_PERFORMANCE 1
#define ATOM_OK          2
#define ATOM_NORMAL      3
#define ATOM_LANGUAGE_ERROR 4
#define ATOM_MEMORY_FAULT 5
#define ATOM_EXPLICIT    6
#define ATOM_UNKNOWN     7
#define ATOM_INSERT      15
#define ATOM_LOOKUP      16
#define ATOM_DELETE      17
#define ATOM_FOUND       18
#define ATOM_NOT_FOUND   19
#define ATOM_UNKNOWN_OP  20

#define DBREAKC_STORE_64 0x80000000

// Cores the runtime runs on (1 or NCORES_MAX). With 1, core 1 is never started and every actor is
// pinned to core 0, as before core 1 ran.
#define NCORES_RUN       2

// The auxiliary stack pointer's offset in the per-core register block (rt_start.S VRG_VSP,
// shared/xt_vr.silica vr_vsp_off).
#define VRG_VSP          112

// ---------------------------------------------------------------------------------------------
// Macros

// Spill every live register window except the current one to the stacks they belong to (Zephyr /
// ESP-IDF xt_asm_utils.h SPILL_ALL_WINDOWS for 64 address registers): writing a high register of a
// rotated window that another frame owns raises the overflow exception that spills it. Sixteen
// rotations bring WINDOWBASE back to where it started; a0-a3 of this window are untouched.
.macro SPILL_ALL_WINDOWS
    and     a12, a12, a12
    rotw    3
    and     a12, a12, a12
    rotw    3
    and     a12, a12, a12
    rotw    3
    and     a12, a12, a12
    rotw    3
    and     a12, a12, a12
    rotw    4
.endm

// The scheduler lock: S32C1I compare-and-swap on one word. \ra and \rb are scratch.
.macro SCHED_LOCK ra, rb
    movi    \ra, silica_rt_sched_lock
9:  movi    \rb, 0
    wsr     \rb, scompare1
    movi    \rb, 1
    s32c1i  \rb, \ra, 0
    bnez    \rb, 9b
.endm

.macro SCHED_UNLOCK ra, rb
    memw
    movi    \ra, silica_rt_sched_lock
    movi    \rb, 0
    s32i    \rb, \ra, 0
    memw
.endm

// The free-list lock: the same shape. Never taken while the scheduler lock is wanted afterwards
// (the list routines are leaves), so holding the scheduler lock around a list call is fine.
.macro FREE_LOCK ra, rb
    movi    \ra, silica_rt_free_lock
9:  movi    \rb, 0
    wsr     \rb, scompare1
    movi    \rb, 1
    s32c1i  \rb, \ra, 0
    bnez    \rb, 9b
.endm

.macro FREE_UNLOCK ra, rb
    memw
    movi    \ra, silica_rt_free_lock
    movi    \rb, 0
    s32i    \rb, \ra, 0
    memw
.endm

// \r = this core's id (0 or 1) from PRID (0xCDCD on the PRO cpu, 0xABAB on the APP cpu: bit 13).
.macro THIS_CORE r
    rsr     \r, prid
    extui   \r, \r, 13, 1
.endm

// \r = this core's core block. \t scratch.
.macro THIS_CORE_BLOCK r, t
    THIS_CORE \r
    slli    \r, \r, 6
    movi    \t, silica_rt_cores
    add     \r, \r, \t
.endm

// ---------------------------------------------------------------------------------------------
// Data

    .section .bss.silica_rt_actors, "aw", @nobits
    .balign 16
    .global silica_rt_cores
silica_rt_cores:
    .space  CORE_SIZE * NCORES_MAX
    .global silica_rt_main_acb
silica_rt_main_acb:
    .space  ACB_SIZE
silica_rt_sched_acbs:
    .space  ACB_SIZE * NCORES_MAX
    .balign 64
silica_rt_sched_stacks:
    .space  SCHED_STACK_SIZE * NCORES_MAX
    .global silica_rt_sched_lock
silica_rt_sched_lock:
    .space  4
silica_rt_free_lock:
    .space  4
silica_rt_core1_boot_acb:               // core 1's boot context: saved by its first switch, never resumed
    .space  ACB_SIZE
silica_rt_actors_ready:
    .space  4
    .global silica_rt_ncores
silica_rt_ncores:
    .space  4
    .global silica_rt_sched_policy
silica_rt_sched_policy:                 // POLICY_* in force; zero = PRIORITY_FIFO
    .space  4
silica_rt_lottery_state:                // xorshift32 state, seeded from CCOUNT at the first draw
    .space  4
silica_rt_next_core:
    .space  4
silica_rt_acb_free:
    .space  4
silica_rt_node_free:
    .space  4
silica_rt_stack_free:
    .space  4
    .global silica_rt_wait_root_acb
silica_rt_wait_root_acb:
    .space  4
    .global silica_pid_registry_ref
silica_pid_registry_ref:
    .space  8
    .global silica_dangerous_pid_registry_ref
silica_dangerous_pid_registry_ref:
    .space  8
    .global silica_rt_root_failure_reporter
silica_rt_root_failure_reporter:
    .space  4
    .global silica_rt_region_dump_limit
silica_rt_region_dump_limit:
    .space  8

    .section .rodata.silica_rt_actors, "a"
silica_rt_root_exit_msg:
    .asciz  "[silica] root actor exited (no supervisor)\n"
silica_rt_banner_head:
    .asciz  "\n=== Silica Actor Failure ===\nactor_id:        "
silica_rt_banner_agent:
    .asciz  "\nagent_type_atom: "
silica_rt_banner_reason:
    .asciz  "\nreason_tag:      "
silica_rt_banner_sup:
    .asciz  "\nsupervisor_acb:  "
silica_rt_banner_stack:
    .asciz  "\ncall_stack:\n  #0  "
silica_rt_banner_unknown:
    .asciz  "<unknown behavior>"
silica_rt_banner_tail:
    .asciz  "\n  #1  _actor_thread_main\n=== End Silica Actor Failure ===\n"

    .section .text.silica_rt_actors, "ax"
    .literal_position

// ---------------------------------------------------------------------------------------------
// Initialisation. Idempotent; the first actor entry point every program runs is
// silica_pid_registry_init, injected at the top of main, so the runtime sets itself up there.
// main becomes a context: a pseudo-ACB pinned to core 0 whose stacks are the boot stacks, so the
// guard re-arming at a switch back to main restores the two watchpoints _start armed. Core 1 is
// started last, once its core block and scheduler context exist; a spawn pinned to core 1 before
// its scheduler runs simply waits on that core's ready queue.

    .align  4
    .global silica_rt_actors_init
    .type   silica_rt_actors_init, @function
silica_rt_actors_init:
    entry   a1, 32
    movi    a2, silica_rt_actors_ready
    l32i    a3, a2, 0
    bnez    a3, 9f
    movi    a3, 1
    s32i    a3, a2, 0
    movi    a2, silica_rt_ncores
    movi    a3, NCORES_RUN
    s32i    a3, a2, 0
    // core blocks: id and scheduler pseudo-ACB
    movi    a4, silica_rt_cores
    movi    a5, silica_rt_sched_acbs
    movi    a6, 0
1:  s32i    a6, a4, CORE_ID
    s32i    a5, a4, CORE_SCHED
    s32i    a5, a4, CORE_CURRENT    // the scheduler, until something runs (core 0: main, below)
    movi    a7, WEIGHT_HIGH         // weighted fair starts a round at the high tier
    s32i    a7, a4, CORE_WF_CREDIT
    movi    a7, KIND_SCHED
    s32i    a7, a5, ACB_KIND
    s32i    a6, a5, ACB_CORE
    movi    a7, 1
    s32i    a7, a5, ACB_ALIVE
    // the scheduler's stack: no auxiliary stack (it runs no emitted code)
    movi    a7, silica_rt_sched_stacks
    movi    a8, SCHED_STACK_SIZE
    mull    a9, a6, a8
    add     a7, a7, a9
    s32i    a7, a5, ACB_STK_BASE
    s32i    a8, a5, ACB_STK_SIZE
    mov     a10, a5
    movi    a11, silica_rt_sched_landing
    call8   silica_rt_ctx_prepare
    addi    a4, a4, CORE_SIZE
    addi    a5, a5, ACB_SIZE
    addi    a6, a6, 1
    movi    a7, NCORES_MAX
    blt     a6, a7, 1b
    // main's pseudo-ACB
    movi    a4, silica_rt_main_acb
    movi    a5, KIND_MAIN
    s32i    a5, a4, ACB_KIND
    movi    a5, 1
    s32i    a5, a4, ACB_ALIVE
    movi    a5, ST_RUNNING
    s32i    a5, a4, ACB_STATUS
    movi    a5, 0
    s32i    a5, a4, ACB_CORE
    movi    a5, -1
    s32i    a5, a4, ACB_TARGET
    movi    a5, _silica_heap_end          // machine stack bottom (the guard _start armed)
    s32i    a5, a4, ACB_STK_BASE
    movi    a5, _silica_stack_size
    s32i    a5, a4, ACB_STK_SIZE
    movi    a5, silica_rt_aux_stack_bottom
    s32i    a5, a4, ACB_AUX_BASE
    movi    a5, 0x10000                   // AUX_STACK_SIZE (rt_start.S)
    s32i    a5, a4, ACB_AUX_SIZE
    movi    a5, 1
    s32i    a5, a4, ACB_PRIO
    movi    a5, silica_rt_cores
    s32i    a4, a5, CORE_CURRENT
#if NCORES_RUN > 1
    call8   silica_rt_start_app_cpu     // rt_start.S: core 1 boots into silica_rt_core1_main
#endif
9:  retw
    .size   silica_rt_actors_init, . - silica_rt_actors_init

// silica_rt_core1_main(): core 1's first and only call, on its boot stack (rt_start.S
// silica_rt_start_core1). The boot context is saved into a pseudo-ACB nothing ever loads, and the
// core's scheduler is entered through its prepared frames. Never returns.
    .align  4
    .global silica_rt_core1_main
    .type   silica_rt_core1_main, @function
silica_rt_core1_main:
    entry   a1, 32
    movi    a10, silica_rt_core1_boot_acb
    THIS_CORE_BLOCK a3, a4
    l32i    a11, a3, CORE_SCHED
    call8   silica_rt_ctx_switch
1:  j       1b
    .size   silica_rt_core1_main, . - silica_rt_core1_main

// ---------------------------------------------------------------------------------------------
// Free lists and allocation

// silica_rt_alloc_acb() -> zeroed ACB
    .align  4
silica_rt_alloc_acb:
    entry   a1, 32
    FREE_LOCK a8, a9
    movi    a3, silica_rt_acb_free
    l32i    a2, a3, 0
    beqz    a2, 1f
    l32i    a4, a2, ACB_FREE_NEXT
    s32i    a4, a3, 0
    FREE_UNLOCK a8, a9
    mov     a10, a2
    movi    a11, ACB_SIZE
    call8   silica_rt_zero_block
    retw
1:  FREE_UNLOCK a8, a9
    movi    a10, ACB_SIZE
    call8   silica_rt_alloc
    mov     a2, a10
    retw
    .size   silica_rt_alloc_acb, . - silica_rt_alloc_acb

// silica_rt_free_acb(acb)
    .align  4
silica_rt_free_acb:
    entry   a1, 32
    FREE_LOCK a8, a9
    movi    a3, silica_rt_acb_free
    l32i    a4, a3, 0
    s32i    a4, a2, ACB_FREE_NEXT
    movi    a4, ST_DEAD
    s32i    a4, a2, ACB_STATUS
    s32i    a2, a3, 0
    FREE_UNLOCK a8, a9
    retw
    .size   silica_rt_free_acb, . - silica_rt_free_acb

// silica_rt_alloc_node() -> zeroed message node
    .align  4
silica_rt_alloc_node:
    entry   a1, 32
    FREE_LOCK a8, a9
    movi    a3, silica_rt_node_free
    l32i    a2, a3, 0
    beqz    a2, 1f
    l32i    a4, a2, MSG_NEXT
    s32i    a4, a3, 0
    FREE_UNLOCK a8, a9
    mov     a10, a2
    movi    a11, MSG_SIZE
    call8   silica_rt_zero_block
    retw
1:  FREE_UNLOCK a8, a9
    movi    a10, MSG_SIZE
    call8   silica_rt_alloc
    mov     a2, a10
    retw
    .size   silica_rt_alloc_node, . - silica_rt_alloc_node

// silica_rt_free_node(node)
    .align  4
    .global silica_rt_free_node
silica_rt_free_node:
    entry   a1, 32
    beqz    a2, 1f
    FREE_LOCK a8, a9
    movi    a3, silica_rt_node_free
    l32i    a4, a3, 0
    s32i    a4, a2, MSG_NEXT
    s32i    a2, a3, 0
    FREE_UNLOCK a8, a9
1:  retw
    .size   silica_rt_free_node, . - silica_rt_free_node

// silica_rt_zero_block(base, bytes): 4-byte granularity.
    .align  4
silica_rt_zero_block:
    entry   a1, 32
    movi    a6, 0
    add     a3, a2, a3
1:  bgeu    a2, a3, 2f
    s32i    a6, a2, 0
    addi    a2, a2, 4
    j       1b
2:  retw
    .size   silica_rt_zero_block, . - silica_rt_zero_block

// silica_rt_alloc_stack(bytes) -> 64-byte aligned zeroed block, or 0 when the heap cannot give it.
// Free blocks keep a 16-byte header just below the block: [-16] size, [-12] next.
    .align  4
silica_rt_alloc_stack:
    entry   a1, 32
    addi    a2, a2, 63
    movi    a3, -64
    and     a2, a2, a3                  // size rounded to 64
    FREE_LOCK a8, a9
    movi    a3, silica_rt_stack_free
    l32i    a4, a3, 0                   // a4 = candidate, a3 = address of the link that points at it
1:  beqz    a4, 5f
    addi    a6, a4, -16                 // a6 = the candidate's header
    l32i    a5, a6, 0
    beq     a5, a2, 2f
    addi    a3, a6, 4
    l32i    a4, a6, 4
    j       1b
2:  l32i    a5, a6, 4                   // unlink
    s32i    a5, a3, 0
    FREE_UNLOCK a8, a9
    mov     a10, a4
    mov     a11, a2
    call8   silica_rt_zero_block
    mov     a2, a4
    retw
5:  FREE_UNLOCK a8, a9
    addi    a10, a2, 80                 // header + alignment slack
    call8   silica_rt_alloc_or_null
    beqz    a10, 9f
    addi    a10, a10, 16 + 63
    movi    a3, -64
    and     a10, a10, a3
    addi    a6, a10, -16
    s32i    a2, a6, 0
    mov     a2, a10
    retw
9:  movi    a2, 0
    retw
    .size   silica_rt_alloc_stack, . - silica_rt_alloc_stack

// silica_rt_free_stack(block)
    .align  4
silica_rt_free_stack:
    entry   a1, 32
    beqz    a2, 1f
    FREE_LOCK a8, a9
    movi    a3, silica_rt_stack_free
    l32i    a4, a3, 0
    addi    a5, a2, -16
    s32i    a4, a5, 4
    s32i    a2, a3, 0
    FREE_UNLOCK a8, a9
1:  retw
    .size   silica_rt_free_stack, . - silica_rt_free_stack

// ---------------------------------------------------------------------------------------------
// Ready queues (caller holds the lock)

// silica_rt_rq_push(acb): onto the tail of the queue of acb's core and priority (under
// ROUND_ROBIN, of the one flat queue).
    .align  4
silica_rt_rq_push:
    entry   a1, 32
    l32i    a3, a2, ACB_CORE
    slli    a3, a3, 6
    movi    a4, silica_rt_cores
    add     a3, a3, a4                  // core block
    movi    a4, silica_rt_sched_policy
    l32i    a4, a4, 0
    movi    a5, POLICY_ROUND_ROBIN
    beq     a4, a5, 2f
    l32i    a4, a2, ACB_PRIO            // 0 low, 1 normal, 2 high -> queue index 2 - prio
    movi    a5, 2
    sub     a4, a5, a4
    j       3f
2:  movi    a4, 1                       // round robin: arrival order in the normal queue
3:  slli    a4, a4, 2
    add     a3, a3, a4                  // &head; &tail = +12, &len = +40
    l32i    a5, a3, CORE_RQ_LEN
    addi    a5, a5, 1
    s32i    a5, a3, CORE_RQ_LEN
    movi    a5, 0
    s32i    a5, a2, ACB_NEXT_READY
    l32i    a5, a3, CORE_RQ_TAIL
    beqz    a5, 1f
    s32i    a2, a5, ACB_NEXT_READY
    s32i    a2, a3, CORE_RQ_TAIL
    retw
1:  s32i    a2, a3, CORE_RQ_HEAD
    s32i    a2, a3, CORE_RQ_TAIL
    retw
    .size   silica_rt_rq_push, . - silica_rt_rq_push

// silica_rt_rq_take(core block, queue index) -> acb: the head of that queue, which must not be empty.
    .align  4
silica_rt_rq_take:
    entry   a1, 32
    slli    a3, a3, 2
    add     a2, a2, a3
    l32i    a3, a2, CORE_RQ_HEAD
    l32i    a5, a3, ACB_NEXT_READY
    s32i    a5, a2, CORE_RQ_HEAD
    bnez    a5, 1f
    s32i    a5, a2, CORE_RQ_TAIL
1:  l32i    a5, a2, CORE_RQ_LEN
    addi    a5, a5, -1
    s32i    a5, a2, CORE_RQ_LEN
    mov     a2, a3
    retw
    .size   silica_rt_rq_take, . - silica_rt_rq_take

// silica_rt_rq_pop(core block) -> acb or 0: the next actor to run under the policy in force.
    .align  4
silica_rt_rq_pop:
    entry   a1, 48
    movi    a4, silica_rt_sched_policy
    l32i    a4, a4, 0
    movi    a5, POLICY_WEIGHTED_FAIR
    beq     a4, a5, 20f
    movi    a5, POLICY_LOTTERY
    beq     a4, a5, 30f
    // PRIORITY_FIFO (and ROUND_ROBIN, whose pushes all land in the normal queue): the head of the
    // highest-priority non-empty queue
    movi    a3, 0
1:  slli    a5, a3, 2
    add     a5, a5, a2
    l32i    a6, a5, CORE_RQ_HEAD
    bnez    a6, 9f
    addi    a3, a3, 1
    movi    a5, 3
    blt     a3, a5, 1b
    movi    a2, 0
    retw
9:  mov     a10, a2
    mov     a11, a3
    call8   silica_rt_rq_take
    mov     a2, a10
    retw
20: // WEIGHTED_FAIR: serve the current tier while it has credit and actors; otherwise the next tier
    // starts with its quantum (an empty tier forfeits its turn). Some queue is non-empty, so at
    // most three tier changes reach a tier that runs.
    l32i    a6, a2, CORE_RQ_HEAD
    l32i    a7, a2, CORE_RQ_HEAD + 4
    or      a6, a6, a7
    l32i    a7, a2, CORE_RQ_HEAD + 8
    or      a6, a6, a7
    bnez    a6, 21f
    movi    a2, 0
    retw
21: l32i    a3, a2, CORE_WF_TIER
    l32i    a4, a2, CORE_WF_CREDIT
22: slli    a5, a3, 2
    add     a5, a5, a2
    l32i    a6, a5, CORE_RQ_HEAD
    beqz    a6, 23f
    beqz    a4, 23f
    addi    a4, a4, -1
    s32i    a3, a2, CORE_WF_TIER
    s32i    a4, a2, CORE_WF_CREDIT
    j       9b
23: addi    a3, a3, 1
    movi    a5, 3
    bltu    a3, a5, 24f
    movi    a3, 0
24: movi    a4, WEIGHT_HIGH
    beqz    a3, 22b
    movi    a4, WEIGHT_NORMAL
    movi    a5, 1
    beq     a3, a5, 22b
    movi    a4, WEIGHT_LOW
    j       22b
30: // LOTTERY: tickets = weight x actors queued, per tier; draw in [0, total) and find the tier
    l32i    a6, a2, CORE_RQ_LEN
    slli    a6, a6, 2                   // WEIGHT_HIGH x high
    l32i    a7, a2, CORE_RQ_LEN + 4
    slli    a7, a7, 1                   // WEIGHT_NORMAL x normal
    l32i    a8, a2, CORE_RQ_LEN + 8     // WEIGHT_LOW x low
    add     a5, a6, a7
    add     a5, a5, a8                  // total tickets
    bnez    a5, 31f
    movi    a2, 0
    retw
31: // xorshift32 (Marsaglia 2003: 13, 17, 5), not cryptographic; seeded from the cycle counter the
    // first time, kept non-zero by the OR. One state word, written under the scheduler lock the
    // caller holds. Shifts of 16 or more are extui: SRLI takes at most 15.
    movi    a9, silica_rt_lottery_state
    l32i    a3, a9, 0
    bnez    a3, 32f
    rsr     a3, ccount
    movi    a4, 1
    or      a3, a3, a4
32: slli    a4, a3, 13
    xor     a3, a3, a4
    extui   a4, a3, 17, 15
    xor     a3, a3, a4
    slli    a4, a3, 5
    xor     a3, a3, a4
    s32i    a3, a9, 0
    remu    a3, a3, a5                  // the draw
    movi    a4, 0
    bltu    a3, a6, 33f                 // below the high tier's tickets: high
    sub     a3, a3, a6
    movi    a4, 1
    bltu    a3, a7, 33f                 // then the normal tier's
    movi    a4, 2                       // else low (its count is non-zero, or total would be smaller)
33: mov     a3, a4
    j       9b
    .size   silica_rt_rq_pop, . - silica_rt_rq_pop

// silica_rt_make_ready(acb): a parked (idle) actor becomes ready. Lock held.
    .align  4
silica_rt_make_ready:
    entry   a1, 32
    l32i    a3, a2, ACB_STATUS
    bnez    a3, 1f
    movi    a3, ST_READY
    s32i    a3, a2, ACB_STATUS
    mov     a10, a2
    call8   silica_rt_rq_push
1:  retw
    .size   silica_rt_make_ready, . - silica_rt_make_ready

// silica_rt_wake(acb): a suspended context (call, ingress wait, wait_for_exit) becomes ready. Lock held.
    .align  4
silica_rt_wake:
    entry   a1, 32
    l32i    a3, a2, ACB_STATUS
    movi    a4, ST_SUSPENDED
    beq     a3, a4, 1f
    movi    a4, ST_WAITQ
    beq     a3, a4, 1f
    movi    a4, ST_WAIT_INGRESS
    beq     a3, a4, 1f
    retw
1:  movi    a3, ST_READY
    s32i    a3, a2, ACB_STATUS
    mov     a10, a2
    call8   silica_rt_rq_push
    retw
    .size   silica_rt_wake, . - silica_rt_wake

// ---------------------------------------------------------------------------------------------
// Context switching

// silica_rt_ctx_switch(save acb, load acb). Returns, in the loaded context, from that context's
// own call to this function (or lands in its trampoline the first time). Interrupts are never
// enabled by this runtime, so nothing else touches the window state meanwhile.
    .align  4
    .global silica_rt_ctx_switch
silica_rt_ctx_switch:
    entry   a1, 32
    call8   silica_rt_console_release_if_mine   // a context leaving mid-line gives the console up
    SPILL_ALL_WINDOWS
    s32i    a0, a2, ACB_CTX_A0
    s32i    a1, a2, ACB_CTX_A1
    rur     a4, threadptr               // this core's register block: the emitter's SP lives there
    l32i    a5, a4, VRG_VSP
    s32i    a5, a2, ACB_CTX_VSP
    l32i    a5, a3, ACB_CTX_VSP
    s32i    a5, a4, VRG_VSP
    // stack guards for the context about to run (the scheduler keeps the previous ones)
    movi    a6, 0
    wsr     a6, dbreakc0
    wsr     a6, dbreakc1
    l32i    a5, a3, ACB_KIND
    movi    a6, KIND_SCHED
    beq     a5, a6, 1f
    l32i    a5, a3, ACB_STK_BASE
    wsr     a5, dbreaka0
    movi    a6, DBREAKC_STORE_64
    wsr     a6, dbreakc0
    l32i    a5, a3, ACB_AUX_BASE
    beqz    a5, 1f
    wsr     a5, dbreaka1
    wsr     a6, dbreakc1
1:  isync
    l32i    a0, a3, ACB_CTX_A0
    l32i    a1, a3, ACB_CTX_A1
    retw
    .size   silica_rt_ctx_switch, . - silica_rt_ctx_switch

// silica_rt_ctx_prepare(acb, landing): fabricate the frames that make the first switch into acb
// land at `landing` with a2 = acb and a1 = top - 64. top = machine stack base + size.
//   [top-144, top-128)  a0-a3 of the landing frame (0, top-64, acb, 0), read by _WindowUnderflow8
//                       from 16 bytes under the fake callee frame at top-128
//   [top-76]            the landing frame's caller SP (top), from which a4-a7 are read at top-32
    .align  4
silica_rt_ctx_prepare:
    entry   a1, 32
    l32i    a4, a2, ACB_STK_BASE
    l32i    a5, a2, ACB_STK_SIZE
    add     a4, a4, a5                  // a4 = top
    addi    a6, a4, -144                // a6 = top - 144; offsets below are relative to it
    movi    a5, 0
    s32i    a5, a6, 0                   // [top-144] a0
    addi    a5, a4, -64
    s32i    a5, a6, 4                   // [top-140] a1 = top - 64
    s32i    a2, a6, 8                   // [top-136] a2 = acb
    movi    a5, 0
    s32i    a5, a6, 12                  // [top-132] a3
    s32i    a4, a6, 68                  // [top-76]  the landing frame's caller SP = top
    s32i    a5, a6, 112                 // [top-32 .. top-16) its a4-a7
    s32i    a5, a6, 116
    s32i    a5, a6, 120
    s32i    a5, a6, 124
    movi    a5, 0x3fffffff
    and     a3, a3, a5
    movi    a5, 0x80000000              // call8 window increment in the return address
    or      a3, a3, a5
    s32i    a3, a2, ACB_CTX_A0
    addi    a5, a4, -128
    s32i    a5, a2, ACB_CTX_A1
    l32i    a5, a2, ACB_AUX_BASE
    l32i    a6, a2, ACB_AUX_SIZE
    add     a5, a5, a6
    s32i    a5, a2, ACB_CTX_VSP
    retw
    .size   silica_rt_ctx_prepare, . - silica_rt_ctx_prepare

// Landing points. Entered by RETW with a1 = top - 64 of the context's stack and a2 = its ACB.
    .align  4
silica_rt_actor_landing:
    mov     a10, a2
    call8   silica_rt_actor_main
1:  j       1b

    .align  4
silica_rt_sched_landing:
    mov     a10, a2
    call8   silica_rt_sched_loop
1:  j       1b

// silica_rt_yield_to_sched(): switch from the running context to this core's scheduler. The
// caller has set its own status under the lock and released it. Returns when the scheduler
// dispatches the context again.
    .align  4
    .global silica_rt_yield_to_sched
silica_rt_yield_to_sched:
    entry   a1, 32
    THIS_CORE_BLOCK a3, a4
    l32i    a10, a3, CORE_CURRENT
    l32i    a11, a3, CORE_SCHED
    call8   silica_rt_ctx_switch
    retw
    .size   silica_rt_yield_to_sched, . - silica_rt_yield_to_sched

// silica_rt_current() -> the running context's ACB (main's pseudo-ACB in main).
    .align  4
    .global silica_rt_current
silica_rt_current:
    entry   a1, 32
    THIS_CORE_BLOCK a3, a4
    l32i    a2, a3, CORE_CURRENT
    movi    a3, 0
    retw
    .size   silica_rt_current, . - silica_rt_current

// silica_rt_current_actor() -> the running actor's ACB, or 0 in main / the scheduler.
    .align  4
    .global silica_rt_current_actor
silica_rt_current_actor:
    entry   a1, 32
    THIS_CORE_BLOCK a3, a4
    l32i    a2, a3, CORE_CURRENT
    movi    a3, 0
    beqz    a2, 1f
    l32i    a4, a2, ACB_KIND
    beqz    a4, 1f
    movi    a2, 0
1:  retw
    .size   silica_rt_current_actor, . - silica_rt_current_actor

// ---------------------------------------------------------------------------------------------
// The scheduler loop, one per core, on its own stack.
//
// Pops the next ready context of this core (highest priority first), records it as current, and
// switches to it; the context switches back when it parks, suspends or ends. A context that was
// ended while parked (kill_abnormal, remove_actor, a fault) is not resumed: its exit path runs here,
// on the scheduler's stack, and its stacks are recycled. When nothing is ready: if main is in
// wait_for_exit and every core is idle with empty queues, main is resumed; otherwise the core spins
// (cross-core wake-ups arrive through the queues under the lock). An idle core spins on a lock-free
// peek of its queues, so its idling does not slow the other core's spawns, casts and calls with lock
// traffic; whatever the peek sees is confirmed under the lock.

    .align  4
    .global silica_rt_sched_loop
    .type   silica_rt_sched_loop, @function
silica_rt_sched_loop:
    entry   a1, 48
    l32i    a3, a2, ACB_CORE
    slli    a3, a3, 6
    movi    a4, silica_rt_cores
    add     a3, a3, a4                  // a3 = my core block
1:  // lock-free peek: anything queued, or main waiting for quiescence (core 0 only)?
    l32i    a5, a3, CORE_RQ_HEAD
    bnez    a5, 10f
    l32i    a5, a3, CORE_RQ_HEAD + 4
    bnez    a5, 10f
    l32i    a5, a3, CORE_RQ_HEAD + 8
    bnez    a5, 10f
    l32i    a5, a3, CORE_ID
    bnez    a5, 11f
    movi    a5, silica_rt_main_acb
    l32i    a5, a5, ACB_STATUS
    movi    a6, ST_WAITQ
    beq     a5, a6, 10f
11: movi    a5, 0
    s32i    a5, a3, CORE_BUSY           // this core's own word; all_quiescent reads it under the lock
    memw
    j       1b
10: SCHED_LOCK a8, a9
    mov     a10, a3
    call8   silica_rt_rq_pop
    mov     a4, a10                     // a4 = next context
    bnez    a4, 3f
    // idle. main waiting for quiescence?
    movi    a5, 0
    s32i    a5, a3, CORE_BUSY
    movi    a5, silica_rt_main_acb
    l32i    a6, a5, ACB_STATUS
    movi    a7, ST_WAITQ
    bne     a6, a7, 2f
    l32i    a6, a3, CORE_ID
    bnez    a6, 2f                      // only core 0 runs main
    call8   silica_rt_all_quiescent
    beqz    a10, 2f
    movi    a6, ST_READY
    s32i    a6, a5, ACB_STATUS
    mov     a4, a5
    j       3f
2:  SCHED_UNLOCK a8, a9
    j       1b
3:  l32i    a5, a4, ACB_STATUS
    movi    a6, ST_DEAD
    beq     a5, a6, 7f                  // ended while queued: only its ACB is left to recycle
    movi    a6, ST_FAULTED
    beq     a5, a6, 6f
    l32i    a5, a4, ACB_ALIVE
    beqz    a5, 6f                      // ended while parked: exit path here
    movi    a5, ST_RUNNING
    s32i    a5, a4, ACB_STATUS
    movi    a5, 1
    s32i    a5, a3, CORE_BUSY
    s32i    a4, a3, CORE_CURRENT
    SCHED_UNLOCK a8, a9
    mov     a10, a2
    mov     a11, a4
    call8   silica_rt_ctx_switch
    // the context parked, suspended or ended
    s32i    a2, a3, CORE_CURRENT
    l32i    a5, a4, ACB_STATUS
    movi    a6, ST_DEAD
    bne     a5, a6, 1b
    mov     a10, a4
    call8   silica_rt_actor_recycle
    j       1b
6:  // exit path on behalf of a context that will never run again
    movi    a5, ST_RUNNING
    s32i    a5, a4, ACB_STATUS
    s32i    a2, a3, CORE_CURRENT
    SCHED_UNLOCK a8, a9
    mov     a10, a4
    call8   silica_rt_actor_exit
    mov     a10, a4
    call8   silica_rt_actor_recycle
    j       1b
7:  SCHED_UNLOCK a8, a9
    mov     a10, a4
    call8   silica_rt_free_acb
    j       1b
    .size   silica_rt_sched_loop, . - silica_rt_sched_loop

// silica_rt_all_quiescent() -> 1 when no core is busy and every ready queue is empty. Lock held.
    .align  4
silica_rt_all_quiescent:
    entry   a1, 32
    movi    a3, silica_rt_cores
    movi    a4, silica_rt_ncores
    l32i    a4, a4, 0
    movi    a5, 0
1:  bge     a5, a4, 8f
    l32i    a6, a3, CORE_BUSY
    bnez    a6, 9f
    l32i    a6, a3, CORE_RQ_HEAD
    bnez    a6, 9f
    l32i    a6, a3, CORE_RQ_HEAD + 4
    bnez    a6, 9f
    l32i    a6, a3, CORE_RQ_HEAD + 8
    bnez    a6, 9f
    addi    a3, a3, CORE_SIZE
    addi    a5, a5, 1
    j       1b
8:  movi    a2, 1
    retw
9:  movi    a2, 0
    retw
    .size   silica_rt_all_quiescent, . - silica_rt_all_quiescent

// silica_rt_actor_recycle_stacks(acb): give an ended actor's stacks back.
    .align  4
silica_rt_actor_recycle_stacks:
    entry   a1, 32
    l32i    a3, a2, ACB_KIND
    bnez    a3, 9f
    l32i    a10, a2, ACB_STK_BASE
    call8   silica_rt_free_stack
    l32i    a10, a2, ACB_AUX_BASE
    call8   silica_rt_free_stack
    movi    a3, 0
    s32i    a3, a2, ACB_STK_BASE
    s32i    a3, a2, ACB_AUX_BASE
9:  retw
    .size   silica_rt_actor_recycle_stacks, . - silica_rt_actor_recycle_stacks

// silica_rt_actor_recycle(acb): the stacks, then the ACB itself.
    .align  4
silica_rt_actor_recycle:
    entry   a1, 32
    l32i    a3, a2, ACB_KIND
    bnez    a3, 9f
    mov     a10, a2
    call8   silica_rt_actor_recycle_stacks
    mov     a10, a2
    call8   silica_rt_free_acb
9:  retw
    .size   silica_rt_actor_recycle, . - silica_rt_actor_recycle

// ---------------------------------------------------------------------------------------------
// The actor's own loop (spec §15.1.2), on the actor's stack. Frame: [a1, a1+24) the reply tuple
// buffer the behaviour fills through X2 (three 8-byte slots); [a1+24] the node being served.

    .align  4
    .global silica_rt_actor_main
    .type   silica_rt_actor_main, @function
silica_rt_actor_main:
    entry   a1, 80
    // a2 = acb. First schedule: the supervisor start trampoline runs once, then whoever waited in
    // spawn for it is woken (the reference spawn returns only after the trampoline ran).
    l32i    a3, a2, ACB_TRAMP
    beqz    a3, 1f
    l32i    a4, a2, ACB_TRAMP_RAN
    bnez    a4, 1f
    mov     a10, a2
    movi    a11, 0
    callx8  a3
    movi    a4, 1
    s32i    a4, a2, ACB_TRAMP_RAN
    l32i    a4, a2, ACB_SPAWN_WAITER
    beqz    a4, 1f
    movi    a5, 0
    s32i    a5, a2, ACB_SPAWN_WAITER
    SCHED_LOCK a8, a9
    mov     a10, a4
    call8   silica_rt_wake
    SCHED_UNLOCK a8, a9
1:  // next message: supervision ingress before the standard mailbox
    SCHED_LOCK a8, a9
    l32i    a3, a2, ACB_IN_HEAD
    beqz    a3, 2f
    l32i    a4, a3, MSG_NEXT
    s32i    a4, a2, ACB_IN_HEAD
    bnez    a4, 11f
    s32i    a4, a2, ACB_IN_TAIL
11: l32i    a4, a2, ACB_IN_DEPTH
    addi    a4, a4, -1
    s32i    a4, a2, ACB_IN_DEPTH
    j       3f
2:  l32i    a3, a2, ACB_MB_HEAD
    beqz    a3, 12f
    l32i    a4, a3, MSG_NEXT
    s32i    a4, a2, ACB_MB_HEAD
    bnez    a4, 3f
    s32i    a4, a2, ACB_MB_TAIL
    j       3f
12: // empty: end if asked to, else park
    l32i    a4, a2, ACB_ALIVE
    beqz    a4, 8f
    movi    a4, ST_IDLE
    s32i    a4, a2, ACB_STATUS
    SCHED_UNLOCK a8, a9
    call8   silica_rt_yield_to_sched
    j       1b
3:  SCHED_UNLOCK a8, a9
    s32i    a3, a1, 24
    l32i    a4, a3, MSG_KIND
    movi    a5, 2
    bne     a4, a5, 4f
    // exit notice: a structured payload runs the supervisor's restart logic, never the behaviour
    l32i    a5, a3, MSG_INGRESS_KIND
    bnez    a5, 4f
    l32i    a5, a3, MSG_PAYLOAD
    beqz    a5, 31f
    l32i    a10, a5, 0
    mov     a11, a2
    l32i    a12, a5, 8
    call8   silica_rt_supervisor_maybe_restart
    l32i    a10, a1, 24
    l32i    a10, a10, MSG_PAYLOAD
    call8   silica_rt_free
31: l32i    a10, a1, 24
    call8   silica_rt_free_node
    j       6f
4:  // the behaviour: X0 = message, X1 = state, X2 = reply buffer; X0 back = tuple pointer
    movi    a6, 1
    bne     a4, a6, 41f
    s32i    a3, a2, ACB_ACTIVE_CALL
41: l32i    a8, a2, ACB_BEH
    beqz    a8, 5f                      // a supervisor with no behaviour of its own drops the message
    mov     a10, a2
    mov     a11, a3
    mov     a12, a1
    call8   silica_rt_run_behaviour
    l32i    a3, a1, 24
    l32i    a4, a3, MSG_KIND
    movi    a5, 0
    s32i    a5, a2, ACB_ACTIVE_CALL
    movi    a6, 1
    bne     a4, a6, 5f
    // a call: the reply is in the node; wake the caller
    SCHED_LOCK a8, a9
    movi    a5, 1
    s32i    a5, a3, MSG_REPLY_SET
    l32i    a10, a3, MSG_CALLER
    beqz    a10, 43f
    call8   silica_rt_wake
43: SCHED_UNLOCK a8, a9
    j       6f
5:  l32i    a10, a1, 24
    call8   silica_rt_free_node
6:  // dispatch boundary: a pending migration takes effect, a requested stop ends the actor, and
    // the turn is given back to the scheduler (fair share of the core, §23.1.1)
    SCHED_LOCK a8, a9
    l32i    a4, a2, ACB_TARGET
    bltz    a4, 61f
    s32i    a4, a2, ACB_CORE
    movi    a4, -1
    s32i    a4, a2, ACB_TARGET
61: l32i    a4, a2, ACB_ALIVE
    beqz    a4, 8f
    l32i    a4, a2, ACB_IN_HEAD
    bnez    a4, 62f
    l32i    a4, a2, ACB_MB_HEAD
    bnez    a4, 62f
    movi    a4, ST_IDLE
    s32i    a4, a2, ACB_STATUS
    j       63f
62: movi    a4, ST_READY
    s32i    a4, a2, ACB_STATUS
    mov     a10, a2
    call8   silica_rt_rq_push
63: SCHED_UNLOCK a8, a9
    call8   silica_rt_yield_to_sched
    j       1b
8:  // the actor ends here, on its own stack; the scheduler recycles the stacks afterwards
    SCHED_UNLOCK a8, a9
    mov     a10, a2
    call8   silica_rt_actor_exit
    call8   silica_rt_yield_to_sched
9:  j       9b
    .size   silica_rt_actor_main, . - silica_rt_actor_main

// silica_rt_run_behaviour(acb, node, reply buffer): calls the behaviour with X0 = message, X1 =
// state, X2 = the buffer, and copies the result out (a call: reply into the node, state into the
// ACB; a cast: state into the ACB). Its frame is a 1.5 KB pad: a reply may point into the
// behaviour's frame region (a tuple or record built there), which lies just below this frame, and
// the runtime frames the actor's loop builds afterwards (waking the caller, yielding) fit inside the
// pad, so that region is intact when the caller reads the reply. The reference keeps user frames on
// the actor's own stack and the loop on the thread's for the same reason. No call is made here after
// the behaviour returns.
    .align  4
silica_rt_run_behaviour:
    entry   a1, 1568
    l32i    a10, a3, MSG_PAYLOAD
    l32i    a11, a3, MSG_PAYLOAD + 4
    l32i    a12, a2, ACB_STATE
    l32i    a13, a2, ACB_STATE + 4
    mov     a14, a4
    movi    a15, 0
    l32i    a8, a2, ACB_BEH
    callx8  a8
    l32i    a4, a3, MSG_KIND
    movi    a6, 1
    bne     a4, a6, 1f
    // (:reply, value, state): reply at +8, state at +16
    l32i    a12, a10, 16
    l32i    a13, a10, 20
    s32i    a12, a2, ACB_STATE
    s32i    a13, a2, ACB_STATE + 4
    l32i    a12, a10, 8
    l32i    a13, a10, 12
    s32i    a12, a3, MSG_REPLY
    s32i    a13, a3, MSG_REPLY + 4
    retw
1:  // (:no_reply, state): state at +8
    l32i    a12, a10, 8
    l32i    a13, a10, 12
    s32i    a12, a2, ACB_STATE
    s32i    a13, a2, ACB_STATE + 4
    retw
    .size   silica_rt_run_behaviour, . - silica_rt_run_behaviour

// ---------------------------------------------------------------------------------------------
// Ending an actor (spec §15.1.2.3): notify the supervisor, print the failure report the reference
// prints, complete every outstanding call with the death result (reply 0), drop the rest of the
// mailbox, remove the registered name, and mark the ACB dead. Runs on the dying actor's stack (its
// own loop found alive == 0, or a trap inside a behaviour) or on the scheduler's / the killer's.

    .align  4
    .global silica_rt_actor_exit
    .type   silica_rt_actor_exit, @function
silica_rt_actor_exit:
    entry   a1, 32
    movi    a3, 0
    s32i    a3, a2, ACB_ALIVE
    l32i    a4, a2, ACB_WAIT_NODE       // a call this actor was suspended in must not wake it later
    beqz    a4, 0f
    s32i    a3, a4, MSG_CALLER
    s32i    a3, a2, ACB_WAIT_NODE
0:  movi    a7, 0                       // a7 = "no supervisor" from deliver_exit
    l32i    a3, a2, ACB_SYNC_REPORTED
    bnez    a3, 1f
    mov     a10, a2
    call8   silica_rt_actor_deliver_exit
    mov     a7, a10
    mov     a10, a2
    call8   silica_rt_actor_deliver_unwind_report
1:  // outstanding calls: the one being served, then every queued call; other messages are dropped
    SCHED_LOCK a8, a9
    l32i    a3, a2, ACB_ACTIVE_CALL
    beqz    a3, 2f
    movi    a4, 0
    s32i    a4, a2, ACB_ACTIVE_CALL
    mov     a10, a3
    call8   silica_rt_complete_death_locked
2:  l32i    a3, a2, ACB_MB_HEAD
    beqz    a3, 4f
    l32i    a4, a3, MSG_NEXT
    s32i    a4, a2, ACB_MB_HEAD
    l32i    a4, a3, MSG_KIND
    movi    a5, 1
    bne     a4, a5, 3f
    mov     a10, a3
    call8   silica_rt_complete_death_locked
    j       2b
3:  mov     a10, a3
    call8   silica_rt_free_node
    j       2b
4:  movi    a4, 0
    s32i    a4, a2, ACB_MB_TAIL
    l32i    a3, a2, ACB_IN_HEAD
    beqz    a3, 5f
    l32i    a4, a3, MSG_NEXT
    s32i    a4, a2, ACB_IN_HEAD
    mov     a10, a3
    call8   silica_rt_free_node
    j       4b
5:  movi    a4, 0
    s32i    a4, a2, ACB_IN_TAIL
    s32i    a4, a2, ACB_IN_DEPTH
    SCHED_UNLOCK a8, a9
    beqz    a7, 6f
    movi    a10, silica_rt_root_exit_msg
    call8   silica_rt_puts
6:  // registered name (§20.3.1)
    l32i    a3, a2, ACB_REG_NAME
    beqz    a3, 7f
    addi    a3, a3, -1
    mov     a10, a2
    mov     a11, a3
    call8   silica_rt_registry_forget
7:  mov     a10, a2
    call8   silica_rt_child_table_free
    l32i    a3, a2, ACB_ROW
    beqz    a3, 8f
    movi    a4, 1
    s32i    a4, a3, 56                  // row.exit_done for the supervisor's cascade
8:  movi    a3, ST_DEAD
    s32i    a3, a2, ACB_STATUS
    retw
    .size   silica_rt_actor_exit, . - silica_rt_actor_exit

// silica_rt_complete_death_locked(node): the call's reply becomes 0 and its caller is woken.
    .align  4
silica_rt_complete_death_locked:
    entry   a1, 32
    l32i    a3, a2, MSG_REPLY_SET
    bnez    a3, 1f
    movi    a3, 0
    s32i    a3, a2, MSG_REPLY
    s32i    a3, a2, MSG_REPLY + 4
    movi    a3, 1
    s32i    a3, a2, MSG_REPLY_SET
    l32i    a10, a2, MSG_CALLER
    beqz    a10, 1f
    call8   silica_rt_wake
1:  retw
    .size   silica_rt_complete_death_locked, . - silica_rt_complete_death_locked

// silica_rt_actor_trap(reason tag): a language-level failure (badarith, case_clause) inside the
// running actor. Ends the actor where it stands (spec §15.1.2: ending is the exception to the
// no-preemption rule): the exit path runs on this stack, then the core's scheduler continues.
// Never returns. Called by rt_console.S when the trap happens in an actor.
    .align  4
    .global silica_rt_actor_trap
    .type   silica_rt_actor_trap, @function
silica_rt_actor_trap:
    entry   a1, 32
    mov     a3, a2
    call8   silica_rt_current_actor
    mov     a2, a10
    beqz    a2, 9f
    s32i    a3, a2, ACB_REASON_TAG
    movi    a4, 0
    s32i    a4, a2, ACB_ALIVE
    mov     a10, a2
    call8   silica_rt_actor_exit
    call8   silica_rt_yield_to_sched
9:  j       9b
    .size   silica_rt_actor_trap, . - silica_rt_actor_trap

// silica_rt_actor_fault_hook(cause): called by the fatal trampoline (rt_vectors.S), on the fault
// stack with a single live window, when a CPU exception was raised while an actor was running on
// this core. Returns 0 when the fault is the process's (main, the scheduler, no actor); otherwise
// the actor is ended -- reason (:explicit, :stack_exhausted) tag 4 for a stack-guard hit (cause
// 1006), :memory_fault tag 3 for anything else -- and this function does not return: the exit path
// is left to the core's scheduler (status FAULTED), whose context is resumed directly.
    .align  4
    .global silica_rt_actor_fault_hook
    .type   silica_rt_actor_fault_hook, @function
silica_rt_actor_fault_hook:
    entry   a1, 32
    mov     a5, a2
    movi    a2, 0
    movi    a3, 0
    movi    a4, silica_rt_actors_ready
    l32i    a4, a4, 0
    beqz    a4, 9f
    THIS_CORE_BLOCK a4, a6
    l32i    a6, a4, CORE_CURRENT
    beqz    a6, 9f
    l32i    a7, a6, ACB_KIND
    bnez    a7, 9f
    movi    a7, 1006
    movi    a8, 3
    bne     a5, a7, 1f
    movi    a8, 4
1:  s32i    a8, a6, ACB_REASON_TAG
    movi    a7, 0
    s32i    a7, a6, ACB_ALIVE
    movi    a7, ST_FAULTED
    s32i    a7, a6, ACB_STATUS
    // the report counter was bumped by the trampoline; this fault is handled, not fatal
    movi    a7, silica_rt_fatal_depth
    l32i    a8, a7, 0
    addi    a8, a8, -1
    s32i    a8, a7, 0
    // queue the actor so the scheduler runs its exit path, then resume the scheduler context
    SCHED_LOCK a8, a9
    mov     a10, a6
    call8   silica_rt_rq_push
    SCHED_UNLOCK a8, a9
    l32i    a3, a4, CORE_SCHED
    s32i    a3, a4, CORE_CURRENT
    rur     a4, threadptr
    l32i    a5, a3, ACB_CTX_VSP
    s32i    a5, a4, VRG_VSP
    movi    a5, 0
    wsr     a5, dbreakc0
    wsr     a5, dbreakc1
    // the abandoned context may have held the console lock (a fault inside a print)
    call8   silica_rt_console_release_if_mine
    // only this window is live (the trampoline reset WINDOWSTART, then the shim called us), so
    // the RETW below reloads the scheduler's frame from its stack, as a switch does
    rsr     a5, windowbase
    movi    a6, 1
    ssl     a5
    sll     a6, a6
    wsr     a6, windowstart
    rsync
    l32i    a0, a3, ACB_CTX_A0
    l32i    a1, a3, ACB_CTX_A1
    retw
9:  retw
    .size   silica_rt_actor_fault_hook, . - silica_rt_actor_fault_hook

// ---------------------------------------------------------------------------------------------
// spawn

// silica_rt_actor_spawn(X0 init, X1 behaviour, X2 core id, X3 supervisor trampoline, X4 policy)
// -> X0 actor_ref. X3 and X4 arrive in this core's register block (THREADPTR; offsets 0 and 8).
    .align  4
    .global silica_rt_actor_spawn
    .type   silica_rt_actor_spawn, @function
silica_rt_actor_spawn:
    entry   a1, 48
    rur     a8, threadptr
    l32i    a9, a8, 0                   // X3 trampoline
    s32i    a9, a1, 0
    l32i    a9, a8, 8                   // X4 policy
    s32i    a9, a1, 4
    movi    a9, 0
    s32i    a9, a1, 8                   // agent type
    s32i    a9, a1, 12                  // row
    s32i    a9, a1, 16                  // no supervisor to inherit a reserve from
    mov     a10, a2
    mov     a11, a3
    mov     a12, a4
    mov     a13, a5
    mov     a14, a6
    mov     a15, a1
    call8   silica_rt_spawn_common
    mov     a2, a10
    movi    a3, 0
    // the first top-level spawn is the root actor wait_for_exit watched in the reference
    beqz    a2, 9f
    movi    a4, silica_rt_wait_root_acb
    l32i    a5, a4, 0
    bnez    a5, 9f
    s32i    a2, a4, 0
9:  retw
    .size   silica_rt_actor_spawn, . - silica_rt_actor_spawn

// silica_rt_actor_spawn_linked(X0 init, X1 behaviour, X2 agent atom, X3 row pointer,
// X4 trampoline) -> X0 actor_ref. Children of a supervisor inherit the spawner's stack policy.
    .align  4
    .global silica_rt_actor_spawn_linked
    .global silica_rt_actor_spawn_dangerous_linked
    .type   silica_rt_actor_spawn_linked, @function
silica_rt_actor_spawn_linked:
silica_rt_actor_spawn_dangerous_linked:
    entry   a1, 48
    rur     a8, threadptr
    l32i    a9, a8, 8                   // X4 trampoline
    s32i    a9, a1, 0
    movi    a9, 0
    s32i    a9, a1, 4                   // policy: inherit (default)
    s32i    a6, a1, 8                   // agent type (X2)
    l32i    a9, a8, 0                   // X3 row
    s32i    a9, a1, 12
    l32i    a9, a8, 16                  // X5 the supervisor (rt_supervisors.S), whose reserve the child inherits
    s32i    a9, a1, 16
    movi    a9, 0
    s32i    a9, a8, 16                  // consumed: a later plain spawn must not inherit from it
    mov     a10, a2
    mov     a11, a3
    mov     a12, a4
    mov     a13, a5
    movi    a14, 0                      // core: runtime-assigned
    mov     a15, a1
    call8   silica_rt_spawn_common
    mov     a2, a10
    movi    a3, 0
    retw
    .size   silica_rt_actor_spawn_linked, . - silica_rt_actor_spawn_linked

// silica_rt_spawn_common(init lo, init hi, beh lo, beh hi, core, extra*) -> acb
//   extra: [0] trampoline, [4] policy word, [8] agent atom, [12] row, [16] supervisor (reserve source)
    .align  4
silica_rt_spawn_common:
    entry   a1, 64
    s32i    a2, a1, 0
    s32i    a3, a1, 4
    s32i    a4, a1, 8
    s32i    a5, a1, 12
    s32i    a6, a1, 16
    s32i    a7, a1, 20
    call8   silica_rt_actors_init
    call8   silica_rt_alloc_acb
    mov     a2, a10                     // a2 = acb
    l32i    a3, a1, 0
    s32i    a3, a2, ACB_STATE
    l32i    a3, a1, 4
    s32i    a3, a2, ACB_STATE + 4
    l32i    a3, a1, 8
    s32i    a3, a2, ACB_BEH
    l32i    a3, a1, 12
    s32i    a3, a2, ACB_BEH + 4
    l32i    a7, a1, 20                  // extra
    l32i    a3, a7, 0
    s32i    a3, a2, ACB_TRAMP
    l32i    a3, a7, 4
    s32i    a3, a2, ACB_POLICY
    l32i    a3, a7, 8
    s32i    a3, a2, ACB_AGENT
    l32i    a3, a7, 12
    s32i    a3, a2, ACB_ROW
    movi    a3, 1
    s32i    a3, a2, ACB_ALIVE
    s32i    a3, a2, ACB_PRIO
    movi    a3, -1
    s32i    a3, a2, ACB_TARGET
    // core: the argument when it names a running core (0 is also "default"), else round robin
    l32i    a3, a1, 16
    movi    a4, silica_rt_ncores
    l32i    a4, a4, 0
    beqz    a3, 1f
    bgeu    a3, a4, 1f
    j       2f
1:  movi    a5, silica_rt_next_core
    l32i    a3, a5, 0
    addi    a6, a3, 1
    bltu    a6, a4, 11f
    movi    a6, 0
11: s32i    a6, a5, 0
2:  s32i    a3, a2, ACB_CORE
    // stacks from the policy word: (reserve rounded to 16) | algorithm
    l32i    a3, a2, ACB_POLICY
    movi    a4, -16
    and     a3, a3, a4
    bnez    a3, 3f
    // inherit the supervisor's reserve when there is none (child specs), else the default
    l32i    a10, a7, 16
    beqz    a10, 21f
    l32i    a3, a10, ACB_RESERVE
    bnez    a3, 3f
21: movi    a4, DEFAULT_MACHINE_STACK
    movi    a5, DEFAULT_AUX_STACK
    movi    a3, DEFAULT_MACHINE_STACK + DEFAULT_AUX_STACK
    j       4f
3:  srli    a5, a3, 2                   // a quarter for the auxiliary stack
    sub     a4, a3, a5
    movi    a6, MIN_MACHINE_STACK
    bgeu    a4, a6, 31f
    mov     a4, a6
31: movi    a6, MIN_AUX_STACK
    bgeu    a5, a6, 4f
    mov     a5, a6
4:  s32i    a3, a2, ACB_RESERVE
    s32i    a4, a2, ACB_STK_SIZE
    s32i    a5, a2, ACB_AUX_SIZE
    mov     a10, a4
    call8   silica_rt_alloc_stack
    beqz    a10, 8f
    s32i    a10, a2, ACB_STK_BASE
    l32i    a10, a2, ACB_AUX_SIZE
    call8   silica_rt_alloc_stack
    beqz    a10, 8f
    s32i    a10, a2, ACB_AUX_BASE
    mov     a10, a2
    movi    a11, silica_rt_actor_landing
    call8   silica_rt_ctx_prepare
    // a supervisor's trampoline must have run before spawn returns: schedule the actor now and
    // suspend the spawner until the actor's first schedule wakes it
    l32i    a3, a2, ACB_TRAMP
    beqz    a3, 7f
    call8   silica_rt_current
    mov     a4, a10
    s32i    a4, a2, ACB_SPAWN_WAITER
    SCHED_LOCK a8, a9
    movi    a5, ST_READY
    s32i    a5, a2, ACB_STATUS
    mov     a10, a2
    call8   silica_rt_rq_push
    movi    a5, ST_SUSPENDED
    s32i    a5, a4, ACB_STATUS
    SCHED_UNLOCK a8, a9
    call8   silica_rt_yield_to_sched
7:  retw
8:  // the reservation could not be made (spec §15.1.2.2)
    l32i    a10, a2, ACB_STK_BASE
    call8   silica_rt_free_stack
    mov     a10, a2
    call8   silica_rt_free_acb
    call8   silica_rt_spawn_reserve_failed
    movi    a2, 0
    retw
    .size   silica_rt_spawn_common, . - silica_rt_spawn_common

// silica_rt_spawn_reserve_failed(): the spawning actor fails with (:explicit, :stack_reserve_failed);
// in main the process ends with a runtime abort, status 71 (spec §15.1.2.2, §15.4.5.5).
    .align  4
    .global silica_rt_spawn_reserve_failed
    .type   silica_rt_spawn_reserve_failed, @function
silica_rt_spawn_reserve_failed:
    entry   a1, 32
    call8   silica_rt_current_actor
    beqz    a10, 1f
    movi    a10, 4
    call8   silica_rt_actor_trap
1:  call8   silica_rt_abort_stack_reserve
    retw
    .size   silica_rt_spawn_reserve_failed, . - silica_rt_spawn_reserve_failed

// ---------------------------------------------------------------------------------------------
// Messages

// silica_rt_enqueue_message(acb, node): onto the standard mailbox; a parked actor becomes ready.
    .align  4
silica_rt_enqueue_message:
    entry   a1, 32
    SCHED_LOCK a8, a9
    movi    a4, 0
    s32i    a4, a3, MSG_NEXT
    l32i    a4, a2, ACB_MB_TAIL
    beqz    a4, 1f
    s32i    a3, a4, MSG_NEXT
    s32i    a3, a2, ACB_MB_TAIL
    j       2f
1:  s32i    a3, a2, ACB_MB_HEAD
    s32i    a3, a2, ACB_MB_TAIL
2:  mov     a10, a2
    call8   silica_rt_make_ready
    SCHED_UNLOCK a8, a9
    retw
    .size   silica_rt_enqueue_message, . - silica_rt_enqueue_message

// silica_rt_enqueue_ingress(acb, node) -> 0 ok, 1 dead target. Bounded; overflow is a runtime abort.
    .align  4
silica_rt_enqueue_ingress:
    entry   a1, 32
    SCHED_LOCK a8, a9
    l32i    a4, a2, ACB_ALIVE
    beqz    a4, 8f
    l32i    a4, a2, ACB_IN_DEPTH
    movi    a5, INGRESS_LIMIT
    bgeu    a4, a5, 9f
    addi    a4, a4, 1
    s32i    a4, a2, ACB_IN_DEPTH
    movi    a4, 0
    s32i    a4, a3, MSG_NEXT
    l32i    a4, a2, ACB_IN_TAIL
    beqz    a4, 1f
    s32i    a3, a4, MSG_NEXT
    s32i    a3, a2, ACB_IN_TAIL
    j       2f
1:  s32i    a3, a2, ACB_IN_HEAD
    s32i    a3, a2, ACB_IN_TAIL
2:  mov     a10, a2
    call8   silica_rt_make_ready
    mov     a10, a2
    call8   silica_rt_wake                  // a supervision_wait_and_drain_one waiter
    SCHED_UNLOCK a8, a9
    movi    a2, 0
    retw
8:  SCHED_UNLOCK a8, a9
    movi    a2, 1
    retw
9:  SCHED_UNLOCK a8, a9
    call8   silica_rt_abort_ingress_full
    retw
    .size   silica_rt_enqueue_ingress, . - silica_rt_enqueue_ingress

// silica_rt_actor_send(X0 actor, X1 message) -> 0
    .align  4
    .global silica_rt_actor_send
    .type   silica_rt_actor_send, @function
silica_rt_actor_send:
    entry   a1, 32
    beqz    a2, 9f
    l32i    a6, a2, ACB_ALIVE
    beqz    a6, 9f
    call8   silica_rt_alloc_node
    s32i    a4, a10, MSG_PAYLOAD
    s32i    a5, a10, MSG_PAYLOAD + 4
    mov     a11, a10
    mov     a10, a2
    call8   silica_rt_enqueue_message
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_actor_send, . - silica_rt_actor_send

// silica_rt_actor_cast(X0 actor, X1 message) -> 1 enqueued, 0 no target
    .align  4
    .global silica_rt_actor_cast
    .type   silica_rt_actor_cast, @function
silica_rt_actor_cast:
    entry   a1, 32
    beqz    a2, 9f
    l32i    a6, a2, ACB_ALIVE
    beqz    a6, 8f
    call8   silica_rt_alloc_node
    s32i    a4, a10, MSG_PAYLOAD
    s32i    a5, a10, MSG_PAYLOAD + 4
    mov     a11, a10
    mov     a10, a2
    call8   silica_rt_enqueue_message
8:  movi    a2, 1
    movi    a3, 0
    retw
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_actor_cast, . - silica_rt_actor_cast

// silica_rt_actor_call(X0 actor, X1 message) -> X0 reply, 0 when the target is dead or dies first.
// The caller suspends (a scheduler yield point) until the target's behaviour returns its reply.
    .align  4
    .global silica_rt_actor_call
    .type   silica_rt_actor_call, @function
silica_rt_actor_call:
    entry   a1, 32
    beqz    a2, 9f
    call8   silica_rt_alloc_node
    mov     a6, a10                     // a6 = node
    movi    a7, 1
    s32i    a7, a6, MSG_KIND
    s32i    a4, a6, MSG_PAYLOAD
    s32i    a5, a6, MSG_PAYLOAD + 4
    call8   silica_rt_current
    mov     a7, a10                     // a7 = me
    s32i    a7, a6, MSG_CALLER
    SCHED_LOCK a8, a9
    l32i    a4, a2, ACB_ALIVE
    beqz    a4, 8f
    movi    a4, 0
    s32i    a4, a6, MSG_NEXT
    l32i    a4, a2, ACB_MB_TAIL
    beqz    a4, 1f
    s32i    a6, a4, MSG_NEXT
    s32i    a6, a2, ACB_MB_TAIL
    j       2f
1:  s32i    a6, a2, ACB_MB_HEAD
    s32i    a6, a2, ACB_MB_TAIL
2:  mov     a10, a2
    call8   silica_rt_make_ready
    movi    a4, ST_SUSPENDED
    s32i    a4, a7, ACB_STATUS
    s32i    a6, a7, ACB_WAIT_NODE
    SCHED_UNLOCK a8, a9
3:  call8   silica_rt_yield_to_sched
    l32i    a4, a6, MSG_REPLY_SET
    beqz    a4, 4f
    movi    a4, 0
    s32i    a4, a7, ACB_WAIT_NODE
    l32i    a2, a6, MSG_REPLY
    l32i    a3, a6, MSG_REPLY + 4
    mov     a10, a6
    call8   silica_rt_free_node
    retw
4:  // woken without a reply (spurious): suspend again
    SCHED_LOCK a8, a9
    movi    a4, ST_SUSPENDED
    s32i    a4, a7, ACB_STATUS
    SCHED_UNLOCK a8, a9
    j       3b
8:  SCHED_UNLOCK a8, a9
    mov     a10, a6
    call8   silica_rt_free_node
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_actor_call, . - silica_rt_actor_call

// Registry-keyed forms: X0 = atom (registry slot), X1 = message.
    .align  4
    .global silica_rt_actor_call_registered
    .type   silica_rt_actor_call_registered, @function
silica_rt_actor_call_registered:
    entry   a1, 32
    movi    a10, silica_pid_registry_ref
    mov     a11, a2
    call8   silica_rt_registry_lookup
    beqz    a10, 9f
    mov     a12, a4
    mov     a13, a5
    call8   silica_rt_actor_call
    mov     a2, a10
    mov     a3, a11
    retw
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_actor_call_registered, . - silica_rt_actor_call_registered

    .align  4
    .global silica_rt_actor_cast_registered
    .type   silica_rt_actor_cast_registered, @function
silica_rt_actor_cast_registered:
    entry   a1, 32
    movi    a10, silica_pid_registry_ref
    mov     a11, a2
    call8   silica_rt_registry_lookup
    beqz    a10, 9f
    mov     a12, a4
    mov     a13, a5
    call8   silica_rt_actor_cast
    mov     a2, a10
    mov     a3, a11
    retw
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_actor_cast_registered, . - silica_rt_actor_cast_registered

    .align  4
    .global silica_rt_actor_cast_dangerous_registered
    .type   silica_rt_actor_cast_dangerous_registered, @function
silica_rt_actor_cast_dangerous_registered:
    entry   a1, 32
    movi    a10, silica_dangerous_pid_registry_ref
    mov     a11, a2
    call8   silica_rt_registry_lookup
    beqz    a10, 9f
    mov     a12, a4
    mov     a13, a5
    call8   silica_rt_actor_cast
    mov     a2, a10
    mov     a3, a11
    retw
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_actor_cast_dangerous_registered, . - silica_rt_actor_cast_dangerous_registered

// silica_rt_actor_self() -> X0 the running actor (0 in main)
    .align  4
    .global silica_rt_actor_self
    .type   silica_rt_actor_self, @function
silica_rt_actor_self:
    entry   a1, 32
    call8   silica_rt_current_actor
    mov     a2, a10
    movi    a3, 0
    retw
    .size   silica_rt_actor_self, . - silica_rt_actor_self

// silica_rt_actor_recv() -> X0 the next message node of the running actor (ingress first), or 0
    .align  4
    .global silica_rt_actor_recv
    .type   silica_rt_actor_recv, @function
silica_rt_actor_recv:
    entry   a1, 32
    call8   silica_rt_current_actor
    mov     a2, a10
    movi    a3, 0
    beqz    a2, 9f
    SCHED_LOCK a8, a9
    l32i    a4, a2, ACB_IN_HEAD
    beqz    a4, 1f
    l32i    a5, a4, MSG_NEXT
    s32i    a5, a2, ACB_IN_HEAD
    bnez    a5, 2f
    s32i    a5, a2, ACB_IN_TAIL
    j       2f
1:  l32i    a4, a2, ACB_MB_HEAD
    beqz    a4, 2f
    l32i    a5, a4, MSG_NEXT
    s32i    a5, a2, ACB_MB_HEAD
    bnez    a5, 2f
    s32i    a5, a2, ACB_MB_TAIL
2:  SCHED_UNLOCK a8, a9
    mov     a2, a4
9:  retw
    .size   silica_rt_actor_recv, . - silica_rt_actor_recv

// ---------------------------------------------------------------------------------------------
// Ending other actors

// silica_rt_remove_actor(X0 actor) -> 0. Ends the actor normally at its next dispatch boundary;
// a parked actor ends now.
    .align  4
    .global silica_rt_remove_actor
    .type   silica_rt_remove_actor, @function
silica_rt_remove_actor:
    entry   a1, 32
    beqz    a2, 9f
    mov     a10, a2
    movi    a11, 0
    call8   silica_rt_end_actor
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_remove_actor, . - silica_rt_remove_actor

// silica_rt_actor_kill_abnormal(X0 actor or registry atom) -> 0. Reason tag 1.
    .align  4
    .global silica_rt_actor_kill_abnormal
    .type   silica_rt_actor_kill_abnormal, @function
silica_rt_actor_kill_abnormal:
    entry   a1, 32
    beqz    a2, 9f
    movi    a4, REGISTRY_SLOTS
    bgeu    a2, a4, 1f
    movi    a10, silica_pid_registry_ref
    mov     a11, a2
    call8   silica_rt_registry_lookup
    mov     a2, a10
    beqz    a2, 9f
1:  l32i    a4, a2, ACB_ALIVE
    beqz    a4, 9f
    l32i    a6, a2, ACB_SUP
    mov     a10, a2
    movi    a11, 1
    call8   silica_rt_end_actor
    // a supervised victim: its supervisor's pending exit notices are handled before returning
    mov     a4, a6
    beqz    a4, 9f
    mov     a10, a4
    call8   silica_rt_supervisor_drain_ingress
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_actor_kill_abnormal, . - silica_rt_actor_kill_abnormal

// silica_rt_end_actor(acb, reason tag): request the end. The running actor (a self-kill) ends after
// its behaviour returns, as the reference does. An actor that is not running -- parked, suspended
// in a call, or queued but not yet dispatched -- is not in the middle of anything, so its exit
// path runs here, in the caller's context, and its stacks are recycled; a queued one keeps its
// ACB until the scheduler pops it (status DEAD). An actor running on another core ends at its
// next dispatch boundary.
    .align  4
    .global silica_rt_end_actor
silica_rt_end_actor:
    entry   a1, 32
    l32i    a4, a2, ACB_KIND
    bnez    a4, 9f
    SCHED_LOCK a8, a9
    l32i    a4, a2, ACB_ALIVE
    beqz    a4, 8f
    l32i    a4, a2, ACB_REASON_TAG
    bnez    a4, 1f
    s32i    a3, a2, ACB_REASON_TAG
1:  movi    a4, 0
    s32i    a4, a2, ACB_ALIVE
    l32i    a4, a2, ACB_STATUS
    movi    a5, ST_IDLE
    beq     a4, a5, 2f
    movi    a5, ST_SUSPENDED
    beq     a4, a5, 2f
    movi    a5, ST_WAIT_INGRESS
    beq     a4, a5, 2f
    movi    a5, ST_READY
    beq     a4, a5, 3f
    SCHED_UNLOCK a8, a9
    retw
2:  movi    a5, ST_RUNNING
    s32i    a5, a2, ACB_STATUS
    SCHED_UNLOCK a8, a9
    mov     a10, a2
    call8   silica_rt_actor_exit
    mov     a10, a2
    call8   silica_rt_actor_recycle
    retw
3:  movi    a5, ST_RUNNING
    s32i    a5, a2, ACB_STATUS
    SCHED_UNLOCK a8, a9
    mov     a10, a2
    call8   silica_rt_actor_exit
    mov     a10, a2
    call8   silica_rt_actor_recycle_stacks
    retw
8:  SCHED_UNLOCK a8, a9
9:  retw
    .size   silica_rt_end_actor, . - silica_rt_end_actor

// ---------------------------------------------------------------------------------------------
// Registries. Each registry is an actor whose state is a table of REGISTRY_SLOTS actor_refs indexed
// by atom (0 = unregistered); register_actor_named writes the table directly and the keyed call /
// cast forms read it, as in the reference; the actor itself answers {op, name, pid} messages with
// a (status, pid) pair.

// silica_rt_registry_lookup(&registry ref word, atom) -> acb or 0
    .align  4
silica_rt_registry_lookup:
    entry   a1, 32
    l32i    a2, a2, 0
    beqz    a2, 9f
    l32i    a2, a2, ACB_STATE           // the table
    beqz    a2, 9f
    movi    a4, REGISTRY_SLOTS
    bgeu    a3, a4, 9f
    slli    a3, a3, 2
    add     a2, a2, a3
    l32i    a2, a2, 0
    beqz    a2, 9f
    l32i    a4, a2, ACB_ALIVE
    bnez    a4, 8f
9:  movi    a2, 0
8:  movi    a3, 0
    retw
    .size   silica_rt_registry_lookup, . - silica_rt_registry_lookup

// silica_rt_registry_store(&registry ref word, atom, acb)
    .align  4
silica_rt_registry_store:
    entry   a1, 32
    l32i    a2, a2, 0
    beqz    a2, 9f
    l32i    a2, a2, ACB_STATE
    beqz    a2, 9f
    movi    a5, REGISTRY_SLOTS
    bgeu    a3, a5, 9f
    slli    a5, a3, 2
    add     a5, a2, a5
    s32i    a4, a5, 0
    beqz    a4, 9f
    addi    a3, a3, 1
    s32i    a3, a4, ACB_REG_NAME
9:  retw
    .size   silica_rt_registry_store, . - silica_rt_registry_store

// silica_rt_registry_slot(&registry ref word, atom) -> address of the slot, or 0
    .align  4
silica_rt_registry_slot:
    entry   a1, 32
    l32i    a2, a2, 0
    beqz    a2, 9f
    l32i    a2, a2, ACB_STATE
    beqz    a2, 9f
    movi    a4, REGISTRY_SLOTS
    bgeu    a3, a4, 9f
    slli    a3, a3, 2
    add     a2, a2, a3
    movi    a3, 0
    retw
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_registry_slot, . - silica_rt_registry_slot

// silica_rt_registry_forget(acb, atom): clear the slot in whichever registry holds this actor.
    .align  4
silica_rt_registry_forget:
    entry   a1, 32
    movi    a4, 0
    s32i    a4, a2, ACB_REG_NAME
    movi    a10, silica_pid_registry_ref
    mov     a11, a3
    call8   silica_rt_registry_slot
    beqz    a10, 1f
    l32i    a5, a10, 0
    bne     a5, a2, 1f
    s32i    a4, a10, 0
1:  movi    a10, silica_dangerous_pid_registry_ref
    mov     a11, a3
    call8   silica_rt_registry_slot
    beqz    a10, 2f
    l32i    a5, a10, 0
    bne     a5, a2, 2f
    s32i    a4, a10, 0
2:  retw
    .size   silica_rt_registry_forget, . - silica_rt_registry_forget

// silica_rt_register_actor_named(X0 actor, X1 atom) -> 0
    .align  4
    .global silica_rt_register_actor_named
    .type   silica_rt_register_actor_named, @function
silica_rt_register_actor_named:
    entry   a1, 32
    movi    a10, silica_pid_registry_ref
    mov     a11, a4
    mov     a12, a2
    call8   silica_rt_registry_store
    movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_register_actor_named, . - silica_rt_register_actor_named

    .align  4
    .global silica_rt_register_dangerous_actor_named
    .type   silica_rt_register_dangerous_actor_named, @function
silica_rt_register_dangerous_actor_named:
    entry   a1, 32
    movi    a10, silica_dangerous_pid_registry_ref
    mov     a11, a4
    mov     a12, a2
    call8   silica_rt_registry_store
    movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_register_dangerous_actor_named, . - silica_rt_register_dangerous_actor_named

// Legacy unary register_actor(actor_ref) -> 0: the global slot belongs to the registry actor.
    .align  4
    .global silica_rt_register_actor
    .type   silica_rt_register_actor, @function
silica_rt_register_actor:
    entry   a1, 32
    movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_register_actor, . - silica_rt_register_actor

// silica_rt_get_registry_actor() -> the registry actor_ref
    .align  4
    .global silica_rt_get_registry_actor
    .type   silica_rt_get_registry_actor, @function
silica_rt_get_registry_actor:
    entry   a1, 32
    movi    a2, silica_pid_registry_ref
    l32i    a2, a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_get_registry_actor, . - silica_rt_get_registry_actor

// The registry behaviour, in the emitted-function ABI: X0 = message record {op, name, pid} (three
// 8-byte slots), X1 = state (the table), X2 = the reply tuple buffer; returns X0 = that buffer with
// [+8] = a heap pair (status atom, actor_ref) and [+16] = the state.
    .align  4
    .global silica_rt_pid_registry_beh
    .type   silica_rt_pid_registry_beh, @function
silica_rt_pid_registry_beh:
    entry   a1, 32
    mov     a7, a6                      // a7 = reply buffer
    movi    a10, 16
    call8   silica_rt_alloc
    mov     a6, a10                     // a6 = pair
    l32i    a3, a2, 0                   // op
    l32i    a5, a2, 8                   // name
    l32i    a2, a2, 16                  // pid
    movi    a8, ATOM_INSERT
    beq     a3, a8, 1f
    movi    a8, ATOM_LOOKUP
    beq     a3, a8, 2f
    movi    a8, ATOM_DELETE
    beq     a3, a8, 3f
    movi    a8, ATOM_UNKNOWN_OP
    mov     a9, a2
    j       8f
1:  movi    a9, REGISTRY_SLOTS
    bgeu    a5, a9, 11f
    slli    a9, a5, 2
    add     a9, a4, a9
    s32i    a2, a9, 0
    beqz    a2, 11f
    addi    a9, a5, 1
    s32i    a9, a2, ACB_REG_NAME
11: movi    a8, ATOM_OK
    mov     a9, a2
    j       8f
2:  movi    a9, REGISTRY_SLOTS
    bgeu    a5, a9, 21f
    slli    a9, a5, 2
    add     a9, a4, a9
    l32i    a9, a9, 0
    beqz    a9, 21f
    movi    a8, ATOM_FOUND
    j       8f
21: movi    a8, ATOM_NOT_FOUND
    movi    a9, 0
    j       8f
3:  movi    a9, REGISTRY_SLOTS
    bgeu    a5, a9, 31f
    slli    a9, a5, 2
    add     a9, a4, a9
    movi    a8, 0
    s32i    a8, a9, 0
31: movi    a8, ATOM_OK
    mov     a9, a2
8:  s32i    a8, a6, 0
    movi    a8, 0
    s32i    a8, a6, 4
    s32i    a9, a6, 8
    s32i    a8, a6, 12
    s32i    a8, a7, 0
    s32i    a8, a7, 4
    s32i    a6, a7, 8
    s32i    a8, a7, 12
    s32i    a4, a7, 16
    s32i    a8, a7, 20
    mov     a2, a7
    movi    a3, 0
    retw
    .size   silica_rt_pid_registry_beh, . - silica_rt_pid_registry_beh

// silica_rt_registry_spawn(&ref word): the registry actor, with small stacks.
    .align  4
silica_rt_registry_spawn:
    entry   a1, 48
    mov     a6, a2
    l32i    a3, a2, 0
    bnez    a3, 9f
    movi    a10, REGISTRY_SLOTS * 4
    call8   silica_rt_alloc
    mov     a7, a10
    rur     a8, threadptr
    movi    a9, 0
    s32i    a9, a8, 0                   // X3: no trampoline
    movi    a9, REGISTRY_RESERVE | 2
    s32i    a9, a8, 8                   // X4: a small reserve, :keep_last_message
    mov     a10, a7
    movi    a11, 0
    movi    a12, silica_rt_pid_registry_beh
    movi    a13, 0
    movi    a14, 0
    call8   silica_rt_actor_spawn
    s32i    a10, a6, 0
    // the registries are runtime actors, not the program's root
    movi    a8, silica_rt_wait_root_acb
    l32i    a9, a8, 0
    bne     a9, a10, 9f
    movi    a9, 0
    s32i    a9, a8, 0
9:  retw
    .size   silica_rt_registry_spawn, . - silica_rt_registry_spawn

// Injected at the top of every program's main by the emitter.
    .align  4
    .global silica_pid_registry_init
    .type   silica_pid_registry_init, @function
silica_pid_registry_init:
    entry   a1, 32
    call8   silica_rt_actors_init
    movi    a10, silica_pid_registry_ref
    call8   silica_rt_registry_spawn
    movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_pid_registry_init, . - silica_pid_registry_init

    .align  4
    .global silica_dangerous_pid_registry_init
    .type   silica_dangerous_pid_registry_init, @function
silica_dangerous_pid_registry_init:
    entry   a1, 32
    call8   silica_rt_actors_init
    movi    a10, silica_dangerous_pid_registry_ref
    call8   silica_rt_registry_spawn
    movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_dangerous_pid_registry_init, . - silica_dangerous_pid_registry_init

// ---------------------------------------------------------------------------------------------
// Placement (spec §15.1.2 Actor Pinning Policy, §22.10)

// silica_rt_migrate_actor(X0 actor, X1 target core) -> 0 :ok, 1 :invalid_target,
// 2 :actor_not_found, 3 :migration_blocked. The move is applied at the actor's next dispatch
// boundary; a parked actor has none pending, so it moves at once. The mailbox moves with the
// actor (it is the actor's own), so message order is preserved (§15.1.2.4.1).
    .align  4
    .global silica_rt_migrate_actor
    .type   silica_rt_migrate_actor, @function
silica_rt_migrate_actor:
    entry   a1, 32
    beqz    a2, 8f
    l32i    a6, a2, ACB_KIND
    bnez    a6, 8f
    l32i    a6, a2, ACB_ALIVE
    beqz    a6, 8f
    bnez    a5, 7f
    movi    a6, silica_rt_ncores
    l32i    a6, a6, 0
    bgeu    a4, a6, 7f
    SCHED_LOCK a8, a9
    l32i    a6, a2, ACB_STATUS
    beqz    a6, 1f                      // idle: pin now
    movi    a7, ST_SUSPENDED
    beq     a6, a7, 1f                  // suspended in a call: resumes on the new core
    s32i    a4, a2, ACB_TARGET          // running or queued: at the next dispatch boundary
    j       2f
1:  s32i    a4, a2, ACB_CORE
2:  SCHED_UNLOCK a8, a9
    movi    a2, 0
    movi    a3, 0
    retw
7:  movi    a2, 1
    movi    a3, 0
    retw
8:  movi    a2, 2
    movi    a3, 0
    retw
    .size   silica_rt_migrate_actor, . - silica_rt_migrate_actor

// silica_rt_move_actor(X0 actor, X1 from, X2 to) -> 0 (the `move` prim; a migrate to X2)
    .align  4
    .global silica_rt_move_actor
    .type   silica_rt_move_actor, @function
silica_rt_move_actor:
    entry   a1, 32
    mov     a10, a2
    mov     a11, a3
    mov     a12, a6
    mov     a13, a7
    call8   silica_rt_migrate_actor
    movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_move_actor, . - silica_rt_move_actor

// pin_actor_to_core(X0, X1 core) -> 0 : a move (§22.10)
    .align  4
    .global silica_rt_pin_actor_to_core
    .type   silica_rt_pin_actor_to_core, @function
silica_rt_pin_actor_to_core:
    entry   a1, 32
    mov     a10, a2
    mov     a11, a3
    mov     a12, a4
    mov     a13, a5
    call8   silica_rt_migrate_actor
    movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_pin_actor_to_core, . - silica_rt_pin_actor_to_core

// The ESP32-S3's two LX7 cores are alike; both are reported as :performance cores, so a move to a
// performance core goes to core 0 and there is no efficiency core to move to.
    .align  4
    .global silica_rt_pin_actor_to_performance_core
    .type   silica_rt_pin_actor_to_performance_core, @function
silica_rt_pin_actor_to_performance_core:
    entry   a1, 32
    mov     a10, a2
    mov     a11, a3
    movi    a12, 0
    movi    a13, 0
    call8   silica_rt_migrate_actor
    movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_pin_actor_to_performance_core, . - silica_rt_pin_actor_to_performance_core

    .align  4
    .global silica_rt_pin_actor_to_efficiency_core
    .global silica_rt_pin_actor_realtime
    .global silica_rt_pin_actor_to_numa_node
    .global silica_rt_unpin_actor
    .type   silica_rt_pin_actor_to_efficiency_core, @function
silica_rt_pin_actor_to_efficiency_core:
silica_rt_pin_actor_realtime:
silica_rt_pin_actor_to_numa_node:
silica_rt_unpin_actor:
    entry   a1, 32
    movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_pin_actor_to_efficiency_core, . - silica_rt_pin_actor_to_efficiency_core

// silica_rt_set_actor_priority(X0 actor, X1 level) -> 0 :ok, 1 :actor_not_found (no actor, or one
// that has ended). Levels 0 low, 1 normal, 2 high choose the ready queue the actor waits in (§23.1.1
// priority hints; the emitter maps the program's :low / :normal / :high to them); anything else is
// normal. The level is also the actor's weight class under the weighted_fair and lottery policies.
    .align  4
    .global silica_rt_set_actor_priority
    .type   silica_rt_set_actor_priority, @function
silica_rt_set_actor_priority:
    entry   a1, 32
    beqz    a2, 8f
    l32i    a6, a2, ACB_ALIVE
    beqz    a6, 8f
    movi    a6, 3
    bltu    a4, a6, 1f
    movi    a4, 1
1:  s32i    a4, a2, ACB_PRIO
    movi    a2, 0
    movi    a3, 0
    retw
8:  movi    a2, 1
    movi    a3, 0
    retw
    .size   silica_rt_set_actor_priority, . - silica_rt_set_actor_priority

// silica_rt_set_scheduler_policy(X0 code) -> 0 :ok, 1 :invalid_policy. Codes are POLICY_* (the
// emitter maps the program's policy atoms to them and the result back, prims_actors.silica
// emit_set_scheduler_policy). Takes effect at the next push or pop on every core; see
// "Dispatch-order policies" above for what happens to actors already queued.
    .align  4
    .global silica_rt_set_scheduler_policy
    .type   silica_rt_set_scheduler_policy, @function
silica_rt_set_scheduler_policy:
    entry   a1, 32
    bnez    a3, 7f
    movi    a4, POLICY_COUNT
    bgeu    a2, a4, 7f
    SCHED_LOCK a8, a9
    movi    a4, silica_rt_sched_policy
    s32i    a2, a4, 0
    SCHED_UNLOCK a8, a9
    movi    a2, 0
    movi    a3, 0
    retw
7:  movi    a2, 1
    movi    a3, 0
    retw
    .size   silica_rt_set_scheduler_policy, . - silica_rt_set_scheduler_policy

// ---------------------------------------------------------------------------------------------
// Topology (spec §22.10). Records and lists are built in the emitter's layouts: a record is one
// 8-byte slot per field; a List[T, normal] is a 24-byte handle {length, first chunk, cursor} over
// 24-byte chunks {slot 0, slot 1, next} (two 8-byte slots per chunk, rt_list.S).

// silica_rt_list_from_words(count, words*) -> list handle (0 = nil); words are 8-byte slots.
    .align  4
silica_rt_list_from_words:
    entry   a1, 48
    beqz    a2, 9f
    mov     a6, a2                      // a6 = n
    mov     a7, a3                      // a7 = words
    movi    a10, 24
    call8   silica_rt_alloc
    mov     a5, a10                     // a5 = handle
    s32i    a6, a5, 0
    // first chunk holds n mod 2 == 1 ? one element at slot 1 : two elements
    movi    a4, 0                       // a4 = previous chunk (0: first)
    mov     a2, a6
    extui   a3, a6, 0, 1                // odd -> cursor 8
    slli    a3, a3, 3
    s32i    a3, a5, 16
1:  beqz    a2, 8f
    movi    a10, 24
    call8   silica_rt_alloc
    bnez    a4, 2f
    s32i    a10, a5, 8
    j       3f
2:  s32i    a10, a4, 16
3:  mov     a4, a10
    // fill from `cursor` (first chunk) or 0 upward
    add     a8, a10, a3
    movi    a3, 0
    addi    a9, a10, 16
4:  bgeu    a8, a9, 1b
    l32i    a11, a7, 0
    s32i    a11, a8, 0
    l32i    a11, a7, 4
    s32i    a11, a8, 4
    addi    a7, a7, 8
    addi    a8, a8, 8
    addi    a2, a2, -1
    bnez    a2, 4b
8:  mov     a2, a5
    movi    a3, 0
    retw
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_list_from_words, . - silica_rt_list_from_words

// silica_rt_core_info_record(id) -> { id, core_type, capabilities, frequency_mhz } (32 B)
    .align  4
silica_rt_core_info_record:
    entry   a1, 32
    mov     a6, a2
    movi    a10, 32
    call8   silica_rt_alloc
    s32i    a6, a10, 0
    srai    a6, a6, 31
    s32i    a6, a10, 4
    movi    a8, ATOM_PERFORMANCE
    s32i    a8, a10, 8
    mov     a2, a10
    movi    a3, 0
    retw
    .size   silica_rt_core_info_record, . - silica_rt_core_info_record

// silica_rt_get_core_capabilities(X0 id) -> X0 core_info; an unknown id gives the sentinel id -1
    .align  4
    .global silica_rt_get_core_capabilities
    .type   silica_rt_get_core_capabilities, @function
silica_rt_get_core_capabilities:
    entry   a1, 32
    movi    a4, silica_rt_ncores
    l32i    a4, a4, 0
    bnez    a3, 1f
    bgeu    a2, a4, 1f
    j       2f
1:  movi    a2, -1
2:  mov     a10, a2
    call8   silica_rt_core_info_record
    mov     a2, a10
    movi    a3, 0
    retw
    .size   silica_rt_get_core_capabilities, . - silica_rt_get_core_capabilities

// silica_rt_get_cpu_topology() -> X0 { cores, numa_nodes, cache_hierarchy }
    .align  4
    .global silica_rt_get_cpu_topology
    .type   silica_rt_get_cpu_topology, @function
silica_rt_get_cpu_topology:
    entry   a1, 64
    movi    a4, silica_rt_ncores
    l32i    a4, a4, 0
    movi    a5, 0
1:  bge     a5, a4, 2f
    mov     a10, a5
    call8   silica_rt_core_info_record
    slli    a6, a5, 3
    add     a6, a1, a6
    s32i    a10, a6, 0
    movi    a7, 0
    s32i    a7, a6, 4
    addi    a5, a5, 1
    j       1b
2:  mov     a10, a4
    mov     a11, a1
    call8   silica_rt_list_from_words
    mov     a6, a10                     // cores list
    movi    a10, 8
    call8   silica_rt_alloc
    mov     a7, a10                     // cache_hierarchy { levels: nil }
    movi    a10, 24
    call8   silica_rt_alloc
    s32i    a6, a10, 0
    s32i    a7, a10, 16
    mov     a2, a10
    movi    a3, 0
    retw
    .size   silica_rt_get_cpu_topology, . - silica_rt_get_cpu_topology

// silica_rt_get_performance_cores() -> List[int64] of every running core; efficiency cores: none
    .align  4
    .global silica_rt_get_performance_cores
    .type   silica_rt_get_performance_cores, @function
silica_rt_get_performance_cores:
    entry   a1, 48
    movi    a4, silica_rt_ncores
    l32i    a4, a4, 0
    movi    a5, 0
    movi    a7, 0
1:  bge     a5, a4, 2f
    slli    a6, a5, 3
    add     a6, a1, a6
    s32i    a5, a6, 0
    s32i    a7, a6, 4
    addi    a5, a5, 1
    j       1b
2:  mov     a10, a4
    mov     a11, a1
    call8   silica_rt_list_from_words
    mov     a2, a10
    movi    a3, 0
    retw
    .size   silica_rt_get_performance_cores, . - silica_rt_get_performance_cores

    .align  4
    .global silica_rt_get_efficiency_cores
    .type   silica_rt_get_efficiency_cores, @function
silica_rt_get_efficiency_cores:
    entry   a1, 32
    movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_get_efficiency_cores, . - silica_rt_get_efficiency_cores

// ---------------------------------------------------------------------------------------------
// wait_for_exit(): main suspends until every actor is parked and no core has work, then the program
// ends with status 0. That is what the reference does when the trial harness's "exit" line arrives
// on stdin (its `_exit(0)`): the board has no stdin, and quiescence is the point where a program's
// outstanding casts have all been handled. From an actor the call returns 0 at once.
    .align  4
    .global silica_rt_wait_for_exit
    .type   silica_rt_wait_for_exit, @function
silica_rt_wait_for_exit:
    entry   a1, 32
    call8   silica_rt_actors_init
    call8   silica_rt_current
    mov     a2, a10
    l32i    a4, a2, ACB_KIND
    movi    a5, KIND_MAIN
    bne     a4, a5, 9f
    SCHED_LOCK a8, a9
    movi    a4, ST_WAITQ
    s32i    a4, a2, ACB_STATUS
    SCHED_UNLOCK a8, a9
    call8   silica_rt_yield_to_sched
    movi    a10, 0
    call8   silica_rt_exit
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_wait_for_exit, . - silica_rt_wait_for_exit

// ---------------------------------------------------------------------------------------------
// Links and monitors (spec §15.4.8.5-8.6): the reference runtime's Phase G surface returns benign
// values; the same here.
    .align  4
    .global silica_rt_actor_link
    .global silica_rt_actor_demonitor
    .type   silica_rt_actor_link, @function
silica_rt_actor_link:
silica_rt_actor_demonitor:
    entry   a1, 32
    movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_actor_link, . - silica_rt_actor_link

    .align  4
    .global silica_rt_actor_monitor
    .type   silica_rt_actor_monitor, @function
silica_rt_actor_monitor:
    entry   a1, 32
    retw
    .size   silica_rt_actor_monitor, . - silica_rt_actor_monitor

// silica_rt_actor_add_child(X0 supervisor, X1 child) -> 0: the explicit parent/child relation.
    .align  4
    .global silica_rt_actor_add_child
    .type   silica_rt_actor_add_child, @function
silica_rt_actor_add_child:
    entry   a1, 32
    beqz    a2, 9f
    beqz    a4, 9f
    s32i    a2, a4, ACB_SUP
    SCHED_LOCK a8, a9
    l32i    a6, a2, ACB_FIRST_CHILD
    s32i    a6, a4, ACB_NEXT_SIB
    s32i    a4, a2, ACB_FIRST_CHILD
    SCHED_UNLOCK a8, a9
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_actor_add_child, . - silica_rt_actor_add_child

// ---------------------------------------------------------------------------------------------
// Exit notification (spec §15.4.10) and its behaviour-only accessors

// silica_rt_actor_deliver_exit(X0 dying actor) -> 0 delivered to its supervisor's ingress,
// 1 dropped (no supervisor, or a dead one: the caller prints the root-exit note).
    .align  4
    .global silica_rt_actor_deliver_exit
    .type   silica_rt_actor_deliver_exit, @function
silica_rt_actor_deliver_exit:
    entry   a1, 32
    l32i    a6, a2, ACB_SUP
    beqz    a6, 9f
    movi    a10, 32
    call8   silica_rt_alloc
    mov     a7, a10
    s32i    a2, a7, 0
    l32i    a8, a2, ACB_REASON_TAG
    s32i    a8, a7, 8
    l32i    a8, a2, ACB_REASON_ATOM
    s32i    a8, a7, 16
    l32i    a8, a2, ACB_AGENT
    s32i    a8, a7, 24
    call8   silica_rt_alloc_node
    movi    a8, 2
    s32i    a8, a10, MSG_KIND
    s32i    a7, a10, MSG_PAYLOAD
    mov     a11, a10
    mov     a10, a6
    call8   silica_rt_enqueue_ingress
    mov     a2, a10
    movi    a3, 0
    retw
9:  movi    a2, 1
    movi    a3, 0
    retw
    .size   silica_rt_actor_deliver_exit, . - silica_rt_actor_deliver_exit

// silica_rt_actor_supervision_ingress_enqueue(X0 actor, X1 inline int64) -> :ok (trial hook: the
// payload is not a structured notice, so the target's behaviour runs on it)
    .align  4
    .global silica_rt_actor_supervision_ingress_enqueue
    .type   silica_rt_actor_supervision_ingress_enqueue, @function
silica_rt_actor_supervision_ingress_enqueue:
    entry   a1, 32
    beqz    a2, 9f
    call8   silica_rt_alloc_node
    movi    a8, 2
    s32i    a8, a10, MSG_KIND
    movi    a8, 1
    s32i    a8, a10, MSG_INGRESS_KIND
    s32i    a4, a10, MSG_PAYLOAD
    s32i    a5, a10, MSG_PAYLOAD + 4
    mov     a11, a10
    mov     a10, a2
    call8   silica_rt_enqueue_ingress
9:  movi    a2, ATOM_OK
    movi    a3, 0
    retw
    .size   silica_rt_actor_supervision_ingress_enqueue, . - silica_rt_actor_supervision_ingress_enqueue

// silica_rt_actor_supervision_wait_and_drain_one() -> X0 failure payload: blocks the running actor
// until an exit notice is in its ingress, runs the restart logic, returns the payload.
    .align  4
    .global silica_rt_actor_supervision_wait_and_drain_one
    .type   silica_rt_actor_supervision_wait_and_drain_one, @function
silica_rt_actor_supervision_wait_and_drain_one:
    entry   a1, 32
    call8   silica_rt_current_actor
    mov     a2, a10
    beqz    a2, 9f
1:  SCHED_LOCK a8, a9
    l32i    a4, a2, ACB_IN_HEAD
    bnez    a4, 2f
    movi    a4, ST_WAIT_INGRESS
    s32i    a4, a2, ACB_STATUS
    SCHED_UNLOCK a8, a9
    call8   silica_rt_yield_to_sched
    j       1b
2:  l32i    a5, a4, MSG_NEXT
    s32i    a5, a2, ACB_IN_HEAD
    bnez    a5, 3f
    s32i    a5, a2, ACB_IN_TAIL
3:  l32i    a5, a2, ACB_IN_DEPTH
    addi    a5, a5, -1
    s32i    a5, a2, ACB_IN_DEPTH
    SCHED_UNLOCK a8, a9
    l32i    a6, a4, MSG_PAYLOAD          // a6 = payload
    l32i    a5, a4, MSG_INGRESS_KIND
    bnez    a5, 4f
    beqz    a6, 4f
    l32i    a10, a6, 0
    beqz    a10, 4f
    mov     a11, a2
    l32i    a12, a6, 8
    call8   silica_rt_supervisor_maybe_restart
4:  mov     a10, a4
    call8   silica_rt_free_node
    mov     a2, a6
    movi    a3, 0
    retw
9:  movi    a3, 0
    retw
    .size   silica_rt_actor_supervision_wait_and_drain_one, . - silica_rt_actor_supervision_wait_and_drain_one

    .align  4
    .global silica_rt_actor_failure_reason_tag
    .type   silica_rt_actor_failure_reason_tag, @function
silica_rt_actor_failure_reason_tag:
    entry   a1, 32
    beqz    a2, 1f
    l32i    a2, a2, 8
1:  movi    a3, 0
    retw
    .size   silica_rt_actor_failure_reason_tag, . - silica_rt_actor_failure_reason_tag

// reason tag -> failure_reason atom: 0 :normal, 2 :language_error, 3 :memory_fault, 4 :explicit,
// else :unknown (the reference maps 5 to :foreign_fault, which the fixed atom prefix lacks).
    .align  4
    .global silica_rt_actor_failure_reason_atom
    .type   silica_rt_actor_failure_reason_atom, @function
silica_rt_actor_failure_reason_atom:
    entry   a1, 32
    movi    a3, 0
    beqz    a2, 7f
    l32i    a4, a2, 8
    movi    a2, ATOM_NORMAL
    beqz    a4, 9f
    movi    a2, ATOM_LANGUAGE_ERROR
    movi    a5, 2
    beq     a4, a5, 9f
    movi    a2, ATOM_MEMORY_FAULT
    movi    a5, 3
    beq     a4, a5, 9f
    movi    a2, ATOM_EXPLICIT
    movi    a5, 4
    beq     a4, a5, 9f
7:  movi    a2, ATOM_UNKNOWN
9:  retw
    .size   silica_rt_actor_failure_reason_atom, . - silica_rt_actor_failure_reason_atom

    .align  4
    .global silica_rt_actor_failure_agent_type
    .type   silica_rt_actor_failure_agent_type, @function
silica_rt_actor_failure_agent_type:
    entry   a1, 32
    beqz    a2, 1f
    l32i    a2, a2, 24
1:  movi    a3, 0
    retw
    .size   silica_rt_actor_failure_agent_type, . - silica_rt_actor_failure_agent_type

    .align  4
    .global silica_rt_actor_failure_release
    .type   silica_rt_actor_failure_release, @function
silica_rt_actor_failure_release:
    entry   a1, 32
    mov     a10, a2
    call8   silica_rt_free
    movi    a2, ATOM_OK
    movi    a3, 0
    retw
    .size   silica_rt_actor_failure_release, . - silica_rt_actor_failure_release

// ---------------------------------------------------------------------------------------------
// The failure report (the reference's deliver_unwind_report). Delivered to the registered root
// FailureReporter as a Silica string when one is alive, else printed on the console:
//   \n=== Silica Actor Failure ===\nactor_id:        0x..\nagent_type_atom: N\nreason_tag:      N
//   \nsupervisor_acb:  0x..\ncall_stack:\n  #0  <behaviour symbol>\n  #1  _actor_thread_main
//   \n=== End Silica Actor Failure ===\n
// The behaviour's symbol comes from the function-name records the emitter writes
// (.rodata.silica_fnames: {address, name} pairs between _silica_fnames_start and _end).

    .align  4
    .global silica_rt_failure_reporter_register
    .type   silica_rt_failure_reporter_register, @function
silica_rt_failure_reporter_register:
    entry   a1, 32
    movi    a4, silica_rt_root_failure_reporter
    l32i    a5, a4, 0
    bnez    a5, 1f
    s32i    a2, a4, 0
1:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_failure_reporter_register, . - silica_rt_failure_reporter_register

    .align  4
    .global silica_rt_failure_reporter_set_dump_limit
    .type   silica_rt_failure_reporter_set_dump_limit, @function
silica_rt_failure_reporter_set_dump_limit:
    entry   a1, 32
    movi    a4, silica_rt_region_dump_limit
    s32i    a2, a4, 0
    s32i    a3, a4, 4
    movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_failure_reporter_set_dump_limit, . - silica_rt_failure_reporter_set_dump_limit

// silica_rt_behaviour_name(fn address) -> C string, or the "<unknown behavior>" text.
// A record is {.word address, .word name} followed by the name's bytes, the next record 4-aligned
// after the NUL (emitter_core writes them in that order).
    .align  4
silica_rt_behaviour_name:
    entry   a1, 32
    movi    a3, _silica_fnames_start
    movi    a4, _silica_fnames_end
1:  bgeu    a3, a4, 9f
    l32i    a5, a3, 0
    beq     a5, a2, 2f
    l32i    a5, a3, 4                   // the name: skip to its NUL, then align
11: l8ui    a6, a5, 0
    addi    a5, a5, 1
    bnez    a6, 11b
    addi    a5, a5, 3
    movi    a6, -4
    and     a3, a5, a6
    j       1b
2:  l32i    a2, a3, 4
    movi    a3, 0
    retw
9:  movi    a2, silica_rt_banner_unknown
    movi    a3, 0
    retw
    .size   silica_rt_behaviour_name, . - silica_rt_behaviour_name

// The report is composed into a buffer (silica_rt_report_buf, 512 B) by the emit helpers below,
// then printed or cast.
    .section .bss.silica_rt_report, "aw", @nobits
    .balign 8
silica_rt_report_buf:
    .space  512
silica_rt_report_len:
    .space  4
    .section .text.silica_rt_actors, "ax"
    .literal_position

// append a C string
    .align  4
silica_rt_report_puts:
    entry   a1, 32
    movi    a3, silica_rt_report_len
    l32i    a4, a3, 0
    movi    a5, silica_rt_report_buf
    add     a5, a5, a4
    movi    a7, 500
1:  l8ui    a6, a2, 0
    beqz    a6, 2f
    bgeu    a4, a7, 2f
    s8i     a6, a5, 0
    addi    a5, a5, 1
    addi    a2, a2, 1
    addi    a4, a4, 1
    j       1b
2:  s32i    a4, a3, 0
    retw
    .size   silica_rt_report_puts, . - silica_rt_report_puts

// append "0x" + 8 hex digits
    .align  4
silica_rt_report_hex:
    entry   a1, 32
    mov     a6, a2
    movi    a3, silica_rt_report_len
    l32i    a4, a3, 0
    movi    a5, silica_rt_report_buf
    add     a5, a5, a4
    movi    a7, 48
    s8i     a7, a5, 0
    movi    a7, 120
    s8i     a7, a5, 1
    addi    a5, a5, 2
    movi    a7, 28
1:  ssr     a7
    srl     a8, a6
    extui   a8, a8, 0, 4
    movi    a9, 10
    bltu    a8, a9, 2f
    addi    a8, a8, 39
2:  addi    a8, a8, 48
    s8i     a8, a5, 0
    addi    a5, a5, 1
    addi    a7, a7, -4
    bgez    a7, 1b
    addi    a4, a4, 10
    s32i    a4, a3, 0
    retw
    .size   silica_rt_report_hex, . - silica_rt_report_hex

// append a signed decimal (32-bit)
    .align  4
silica_rt_report_dec:
    entry   a1, 48
    movi    a3, silica_rt_report_len
    l32i    a4, a3, 0
    movi    a5, silica_rt_report_buf
    add     a5, a5, a4
    bgez    a2, 1f
    movi    a6, 45
    s8i     a6, a5, 0
    addi    a5, a5, 1
    addi    a4, a4, 1
    neg     a2, a2
1:  addi    a6, a1, 12                  // digits backwards into [a1, a1+12)
    mov     a7, a6
    movi    a8, 10
2:  remu    a9, a2, a8
    quou    a2, a2, a8
    addi    a9, a9, 48
    addi    a7, a7, -1
    s8i     a9, a7, 0
    bnez    a2, 2b
3:  bgeu    a7, a6, 4f
    l8ui    a9, a7, 0
    s8i     a9, a5, 0
    addi    a5, a5, 1
    addi    a7, a7, 1
    addi    a4, a4, 1
    j       3b
4:  s32i    a4, a3, 0
    retw
    .size   silica_rt_report_dec, . - silica_rt_report_dec

// silica_rt_actor_deliver_unwind_report(X0 dying actor) -> 0
    .align  4
    .global silica_rt_actor_deliver_unwind_report
    .type   silica_rt_actor_deliver_unwind_report, @function
silica_rt_actor_deliver_unwind_report:
    entry   a1, 32
    // the buffer is shared by both cores: composed and written (or cast) under the console lock,
    // which is re-entrant on this core for the write and never held across a switch (a cast is an
    // enqueue, not a yield)
    call8   silica_rt_console_acquire
    movi    a3, silica_rt_report_len
    movi    a4, 0
    s32i    a4, a3, 0
    movi    a10, silica_rt_banner_head
    call8   silica_rt_report_puts
    mov     a10, a2
    call8   silica_rt_report_hex
    movi    a10, silica_rt_banner_agent
    call8   silica_rt_report_puts
    l32i    a10, a2, ACB_AGENT
    call8   silica_rt_report_dec
    movi    a10, silica_rt_banner_reason
    call8   silica_rt_report_puts
    l32i    a10, a2, ACB_REASON_TAG
    call8   silica_rt_report_dec
    movi    a10, silica_rt_banner_sup
    call8   silica_rt_report_puts
    l32i    a10, a2, ACB_SUP
    call8   silica_rt_report_hex
    movi    a10, silica_rt_banner_stack
    call8   silica_rt_report_puts
    l32i    a10, a2, ACB_BEH
    call8   silica_rt_behaviour_name
    call8   silica_rt_report_puts
    movi    a10, silica_rt_banner_tail
    call8   silica_rt_report_puts
    movi    a3, silica_rt_report_len
    l32i    a4, a3, 0
    movi    a5, silica_rt_report_buf
    add     a6, a5, a4
    movi    a7, 0
    s8i     a7, a6, 0                   // NUL after the text (the buffer keeps spare bytes)
    // a registered, alive root FailureReporter (other than the dying actor) receives the text
    movi    a6, silica_rt_root_failure_reporter
    l32i    a6, a6, 0
    beqz    a6, 5f
    beq     a6, a2, 4f
    l32i    a7, a6, ACB_ALIVE
    beqz    a7, 5f
    mov     a10, a5
    mov     a11, a4
    call8   silica_rt_string_from_bytes
    mov     a11, a10
    movi    a12, 0
    mov     a10, a6
    call8   silica_rt_actor_cast
    bnez    a10, 9f
    j       5f
4:  movi    a6, silica_rt_root_failure_reporter
    movi    a7, 0
    s32i    a7, a6, 0
5:  mov     a10, a5
    mov     a12, a4
    call8   silica_rt_write
9:  call8   silica_rt_console_release
    movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_actor_deliver_unwind_report, . - silica_rt_actor_deliver_unwind_report

// ---------------------------------------------------------------------------------------------
// get_actor_memory_usage(X0 actor) -> { reserved, committed, retained, high_water } (32 B).
// Fixed blocks: reserved = committed = retained = the two blocks together; high_water = the deepest
// the machine stack has been, found as the lowest non-zero word of the block (handed out zeroed).
    .align  4
    .global silica_rt_get_actor_memory_usage
    .type   silica_rt_get_actor_memory_usage, @function
silica_rt_get_actor_memory_usage:
    entry   a1, 32
    mov     a6, a2
    movi    a10, 32
    call8   silica_rt_alloc
    mov     a2, a10
    movi    a3, 0
    beqz    a6, 9f
    l32i    a4, a6, ACB_STK_BASE
    beqz    a4, 9f
    l32i    a5, a6, ACB_STK_SIZE
    l32i    a7, a6, ACB_AUX_SIZE
    add     a7, a7, a5
    s32i    a7, a2, 0
    s32i    a7, a2, 8
    s32i    a7, a2, 16
    add     a5, a4, a5                  // top
1:  bgeu    a4, a5, 2f
    l32i    a7, a4, 0
    bnez    a7, 2f
    addi    a4, a4, 4
    j       1b
2:  sub     a5, a5, a4
    s32i    a5, a2, 24
9:  retw
    .size   silica_rt_get_actor_memory_usage, . - silica_rt_get_actor_memory_usage

// ---------------------------------------------------------------------------------------------
// Supervisors: the child table runtime lives in rt_supervisors.S. Until it does, the entry points
// the emitter and this file reach are defined there or here as the reference's benign values.
