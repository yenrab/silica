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

// Supervisor runtime for the bare-metal ESP32-S3 (spec §15.4.8-§15.4.12): the child table and its
// operations, which the emitter's prims_actors.silica reaches as silica_rt_supervisor_* and
// silica_rt_child_table_*, and which the per-supervisor start trampoline (module_linkage.silica)
// reaches as silica_rt_supervisor_materialize_init_children. The reference for every behaviour is
// apple_silicon_mac/terms/prims/prims_actors_child_table_asm.silica; the ACB fields it uses are the
// ones rt_actors.S defines (supervisor, agent type, row, child table head/len, strategy, restart
// caps, sync_failure_reported).
//
// Child table: a singly linked list of 88-byte rows anchored at ACB+116 (head) with the high-water
// row count at ACB+120; a row whose child is 0 is a tombstone (:temporary / :transient children
// that were not restarted, terminated children). Row layout, 8-byte slots as the reference's:
//   +0 child_ref  +8 id atom  +16 agent_type atom  +24 initial state  +32 behaviour  +40 restart atom
//   +48 shutdown  +56 exit_done  +64 flavor atom  +72 supervisor start trampoline  +80 next row
// A child spec record (value-flat, as the emitter lays it out in a child-spec list cell or a
// start_child argument): id@0, agent_type@8, initial_state@16, behavior@24, restart@32,
// shutdown@40, flavor@48, hidden start trampoline@56 (64 bytes; the child-spec list keeps one such
// cell per 64-byte chunk, prims_list.silica chunk_elem_slot_bytes).
//
// Cooperative model: the reference kills a sibling by marking it and spinning on the row's exit_done
// flag while the child's own thread unwinds; here a parked child is ended synchronously in the
// caller's context (rt_actors.S silica_rt_end_actor runs the exit path and stamps exit_done), so
// the flag is set when end_actor returns. A child running on another core ends at its next
// dispatch boundary; the wait loops below cover that case.
//
// Restart frequency windows are measured in seconds from the 16 MHz system timer (SYSTIMER unit
// 0), started here on first use.

#define ACB_STATE        0
#define ACB_BEH          8
#define ACB_ALIVE        16
#define ACB_STATUS       28
#define ACB_KIND         84
#define ACB_SUP          96
#define ACB_AGENT        100
#define ACB_ROW          104
#define ACB_CT_HEAD      116
#define ACB_CT_LEN       120
#define ACB_SYNC_REPORTED 124
#define ACB_TRAMP        128
#define ACB_STRATEGY     136
#define ACB_ALLOWED      140
#define ACB_TIMEFRAME    144
#define ACB_RESTARTS     148
#define ACB_WINDOW       152
#define ACB_REASON_TAG   156
#define ACB_IN_HEAD      40
#define ACB_IN_TAIL      44
#define ACB_IN_DEPTH     48

#define ST_DEAD          4

#define MSG_NEXT         0
#define MSG_KIND         4
#define MSG_PAYLOAD      8
#define MSG_INGRESS_KIND 32

#define ROW_CHILD        0
#define ROW_ID           8
#define ROW_AGENT        16
#define ROW_INIT         24
#define ROW_BEH          32
#define ROW_RESTART      40
#define ROW_SHUTDOWN     48
#define ROW_EXIT_DONE    56
#define ROW_FLAVOR       64
#define ROW_TRAMP        72
#define ROW_NEXT         80
#define ROW_SIZE         88

#define SPEC_ID          0
#define SPEC_AGENT       8
#define SPEC_INIT        16
#define SPEC_BEH         24
#define SPEC_RESTART     32
#define SPEC_SHUTDOWN    40
#define SPEC_FLAVOR      48
#define SPEC_TRAMP       56
#define SPEC_LIST_CHUNK  64

#define ATOM_UNKNOWN     7
#define ATOM_PERMANENT   9
#define ATOM_TRANSIENT   10
#define ATOM_ONE_FOR_ALL 13
#define ATOM_REST_FOR_ONE 14
#define ATOM_DANGEROUS   22

#define SYSTIMER_CONF_REG          0x60023000
#define SYSTIMER_UNIT0_OP_REG      0x60023004
#define SYSTIMER_UNIT0_VALUE_HI_REG 0x60023040
#define SYSTIMER_UNIT0_VALUE_LO_REG 0x60023044
#define SYSTIMER_CLK_EN            0x80000000
#define SYSTIMER_UNIT0_WORK_EN     0x40000000
#define SYSTIMER_UNIT0_UPDATE      0x40000000
#define SYSTIMER_UNIT0_VALUE_VALID 0x20000000
#define SYSTIMER_TICKS_PER_SECOND  16000000

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

    .section .rodata.silica_rt_supervisors, "a"
silica_rt_sup_root_exit_msg:
    .asciz  "[silica] root actor exited (no supervisor)\n"

    .section .text.silica_rt_supervisors, "ax"
    .literal_position

// ---------------------------------------------------------------------------------------------
// Time: seconds since the system timer was started (first call).

    .align  4
    .global silica_rt_now_seconds
    .type   silica_rt_now_seconds, @function
silica_rt_now_seconds:
    entry   a1, 32
    movi    a3, SYSTIMER_CONF_REG
    l32i    a4, a3, 0
    movi    a5, SYSTIMER_CLK_EN | SYSTIMER_UNIT0_WORK_EN
    or      a4, a4, a5
    s32i    a4, a3, 0
    movi    a3, SYSTIMER_UNIT0_OP_REG
    movi    a4, SYSTIMER_UNIT0_UPDATE
    s32i    a4, a3, 0
    movi    a5, SYSTIMER_UNIT0_VALUE_VALID
1:  l32i    a4, a3, 0
    and     a4, a4, a5
    beqz    a4, 1b
    movi    a3, SYSTIMER_UNIT0_VALUE_LO_REG
    l32i    a10, a3, 0
    movi    a3, SYSTIMER_UNIT0_VALUE_HI_REG
    l32i    a11, a3, 0
    movi    a12, SYSTIMER_TICKS_PER_SECOND
    movi    a13, 0
    call8   __udivdi3
    mov     a2, a10
    movi    a3, 0
    retw
    .size   silica_rt_now_seconds, . - silica_rt_now_seconds

// ---------------------------------------------------------------------------------------------
// Rows

// silica_rt_child_table_free(acb): the rows go back to nothing (the heap never frees); the anchors
// are cleared. sync_failure_reported outlives the table.
    .align  4
    .global silica_rt_child_table_free
    .type   silica_rt_child_table_free, @function
silica_rt_child_table_free:
    entry   a1, 32
    movi    a3, 0
    s32i    a3, a2, ACB_CT_HEAD
    s32i    a3, a2, ACB_CT_LEN
    movi    a2, 0
    retw
    .size   silica_rt_child_table_free, . - silica_rt_child_table_free

// silica_rt_child_table_append_row(acb) -> the new (zeroed) row at the tail; len is not changed.
    .align  4
silica_rt_child_table_append_row:
    entry   a1, 32
    movi    a10, ROW_SIZE
    call8   silica_rt_alloc
    l32i    a3, a2, ACB_CT_HEAD
    bnez    a3, 1f
    s32i    a10, a2, ACB_CT_HEAD
    mov     a2, a10
    movi    a3, 0
    retw
1:  l32i    a4, a3, ROW_NEXT
    beqz    a4, 2f
    mov     a3, a4
    j       1b
2:  s32i    a10, a3, ROW_NEXT
    mov     a2, a10
    movi    a3, 0
    retw
    .size   silica_rt_child_table_append_row, . - silica_rt_child_table_append_row

// silica_rt_child_table_row_at(acb, index) -> row or 0
    .align  4
silica_rt_child_table_row_at:
    entry   a1, 32
    l32i    a4, a2, ACB_CT_HEAD
1:  beqz    a4, 9f
    beqz    a3, 8f
    l32i    a4, a4, ROW_NEXT
    addi    a3, a3, -1
    j       1b
8:  mov     a2, a4
    movi    a3, 0
    retw
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_child_table_row_at, . - silica_rt_child_table_row_at

// silica_rt_supervisor_spawn_child_row(row, sup) -> child. spawn_linked(initial state, behaviour,
// agent type, row, trampoline); a :dangerous child is registered in the dangerous registry as well.
    .align  4
    .global silica_rt_supervisor_spawn_child_row_by_flavor
    .type   silica_rt_supervisor_spawn_child_row_by_flavor, @function
silica_rt_supervisor_spawn_child_row_by_flavor:
    entry   a1, 48
    mov     a6, a2                      // a6 = row
    mov     a7, a3                      // a7 = supervisor
    rur     a8, threadptr               // this core's register block (X3.. at 0)
    s32i    a6, a8, 0                   // X3 = row
    movi    a9, 0
    s32i    a9, a8, 4
    l32i    a9, a6, ROW_TRAMP
    s32i    a9, a8, 8                   // X4 = trampoline
    movi    a9, 0
    s32i    a9, a8, 12
    s32i    a7, a8, 16                  // X5 = the supervisor whose stack reserve the child inherits
    s32i    a9, a8, 20
    l32i    a10, a6, ROW_INIT
    l32i    a11, a6, ROW_INIT + 4
    l32i    a12, a6, ROW_BEH
    l32i    a13, a6, ROW_BEH + 4
    l32i    a14, a6, ROW_AGENT
    movi    a15, 0
    call8   silica_rt_actor_spawn_linked
    mov     a5, a10                     // a5 = child
    beqz    a5, 9f
    s32i    a7, a5, ACB_SUP
    s32i    a5, a6, ROW_CHILD
    movi    a9, 0
    s32i    a9, a6, ROW_CHILD + 4
    s32i    a9, a6, ROW_EXIT_DONE
    l32i    a4, a6, ROW_FLAVOR
    movi    a9, ATOM_DANGEROUS
    bne     a4, a9, 1f
    mov     a10, a5
    movi    a11, 0
    l32i    a12, a6, ROW_ID
    movi    a13, 0
    call8   silica_rt_register_dangerous_actor_named
1:  mov     a10, a5
    movi    a11, 0
    l32i    a12, a6, ROW_ID
    movi    a13, 0
    call8   silica_rt_register_actor_named
9:  mov     a2, a5
    movi    a3, 0
    retw
    .size   silica_rt_supervisor_spawn_child_row_by_flavor, . - silica_rt_supervisor_spawn_child_row_by_flavor

// silica_rt_fill_row_from_spec(row, spec): copy the visible fields and the hidden trampoline.
    .align  4
silica_rt_fill_row_from_spec:
    entry   a1, 32
    movi    a4, 0
    s32i    a4, a2, ROW_CHILD
    s32i    a4, a2, ROW_CHILD + 4
    l32i    a4, a3, SPEC_ID
    s32i    a4, a2, ROW_ID
    l32i    a4, a3, SPEC_AGENT
    s32i    a4, a2, ROW_AGENT
    l32i    a4, a3, SPEC_INIT
    s32i    a4, a2, ROW_INIT
    l32i    a4, a3, SPEC_INIT + 4
    s32i    a4, a2, ROW_INIT + 4
    l32i    a4, a3, SPEC_BEH
    s32i    a4, a2, ROW_BEH
    l32i    a4, a3, SPEC_BEH + 4
    s32i    a4, a2, ROW_BEH + 4
    l32i    a4, a3, SPEC_RESTART
    s32i    a4, a2, ROW_RESTART
    l32i    a4, a3, SPEC_SHUTDOWN
    s32i    a4, a2, ROW_SHUTDOWN
    l32i    a4, a3, SPEC_SHUTDOWN + 4
    s32i    a4, a2, ROW_SHUTDOWN + 4
    movi    a4, 0
    s32i    a4, a2, ROW_EXIT_DONE
    l32i    a4, a3, SPEC_FLAVOR
    s32i    a4, a2, ROW_FLAVOR
    l32i    a4, a3, SPEC_TRAMP
    s32i    a4, a2, ROW_TRAMP
    retw
    .size   silica_rt_fill_row_from_spec, . - silica_rt_fill_row_from_spec

// ---------------------------------------------------------------------------------------------
// silica_rt_supervisor_materialize_init_children(X0 supervisor ACB, X1 init result tuple): called by
// the per-supervisor start trampoline on the supervisor's first schedule. The tuple holds the
// supervisor_flags record (a pointer in slot 0, or the three fields flat in slots 0-2 for older
// shapes) and the child-spec list handle (slot 1, or slot 3 in the flat shape).
    .align  4
    .global silica_rt_supervisor_materialize_init_children
    .type   silica_rt_supervisor_materialize_init_children, @function
silica_rt_supervisor_materialize_init_children:
    entry   a1, 64
    mov     a6, a2                      // a6 = supervisor
    beqz    a4, 9f
    l32i    a5, a4, 0
    movi    a8, 4096
    bltu    a5, a8, 1f
    // slot 0 is the flags record
    l32i    a8, a5, 0
    s32i    a8, a6, ACB_STRATEGY
    l32i    a8, a5, 8
    s32i    a8, a6, ACB_ALLOWED
    l32i    a8, a5, 16
    s32i    a8, a6, ACB_TIMEFRAME
    l32i    a7, a4, 8                   // a7 = list handle
    j       2f
1:  s32i    a5, a6, ACB_STRATEGY
    l32i    a8, a4, 8
    s32i    a8, a6, ACB_ALLOWED
    l32i    a8, a4, 16
    s32i    a8, a6, ACB_TIMEFRAME
    l32i    a7, a4, 24
2:  beqz    a7, 9f
    l32i    a2, a7, 0                   // a2 = remaining count
    l32i    a3, a7, 8                   // a3 = chunk
    l32i    a4, a7, 16                  // a4 = cursor (first chunk only)
3:  beqz    a2, 9f
    beqz    a3, 9f
    add     a5, a3, a4                  // a5 = the spec in this cell
    s32i    a5, a1, 0
    mov     a10, a6
    call8   silica_rt_child_table_append_row
    mov     a7, a10                     // a7 = row
    mov     a10, a7
    l32i    a11, a1, 0
    call8   silica_rt_fill_row_from_spec
    mov     a10, a7
    mov     a11, a6
    call8   silica_rt_supervisor_spawn_child_row_by_flavor
    l32i    a8, a6, ACB_CT_LEN
    addi    a8, a8, 1
    s32i    a8, a6, ACB_CT_LEN
    l32i    a3, a3, SPEC_LIST_CHUNK      // next chunk (one cell per chunk)
    movi    a4, 0
    addi    a2, a2, -1
    j       3b
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_supervisor_materialize_init_children, . - silica_rt_supervisor_materialize_init_children

// silica_rt_supervisor_start_child_in(sup, spec) -> child: reuse the first tombstone row or append.
    .align  4
silica_rt_supervisor_start_child_in:
    entry   a1, 48
    mov     a6, a2                      // a6 = sup
    mov     a7, a3                      // a7 = spec
    l32i    a4, a6, ACB_CT_HEAD
    movi    a5, 0                       // a5 = appended flag
1:  beqz    a4, 2f
    l32i    a8, a4, ROW_CHILD
    beqz    a8, 3f
    l32i    a4, a4, ROW_NEXT
    j       1b
2:  mov     a10, a6
    call8   silica_rt_child_table_append_row
    mov     a4, a10
    movi    a5, 1
3:  mov     a10, a4
    mov     a11, a7
    call8   silica_rt_fill_row_from_spec
    mov     a10, a4
    mov     a11, a6
    call8   silica_rt_supervisor_spawn_child_row_by_flavor
    mov     a2, a10
    beqz    a5, 9f
    l32i    a8, a6, ACB_CT_LEN
    addi    a8, a8, 1
    s32i    a8, a6, ACB_CT_LEN
9:  movi    a3, 0
    retw
    .size   silica_rt_supervisor_start_child_in, . - silica_rt_supervisor_start_child_in

// silica_rt_supervisor_start_child(X0 spec) -> child, in the running supervisor
    .align  4
    .global silica_rt_supervisor_start_child
    .type   silica_rt_supervisor_start_child, @function
silica_rt_supervisor_start_child:
    entry   a1, 32
    mov     a6, a2
    call8   silica_rt_current_actor
    beqz    a10, 9f
    mov     a11, a6
    call8   silica_rt_supervisor_start_child_in
    mov     a2, a10
    movi    a3, 0
    retw
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_supervisor_start_child, . - silica_rt_supervisor_start_child

// silica_rt_supervisor_add_child(X0 sup, X1 spec) -> child (call_supervisor :add_child)
    .align  4
    .global silica_rt_supervisor_add_child
    .type   silica_rt_supervisor_add_child, @function
silica_rt_supervisor_add_child:
    entry   a1, 32
    mov     a6, a2
    mov     a7, a4
    mov     a10, a6
    call8   silica_rt_supervisor_drain_ingress
    mov     a10, a6
    mov     a11, a7
    call8   silica_rt_supervisor_start_child_in
    mov     a2, a10
    movi    a3, 0
    retw
    .size   silica_rt_supervisor_add_child, . - silica_rt_supervisor_add_child

// ---------------------------------------------------------------------------------------------
// Exit notices

// silica_rt_supervisor_drain_ingress(sup): every structured exit notice waiting in the supervisor's
// ingress runs the restart logic now, in the caller's context.
    .align  4
    .global silica_rt_supervisor_drain_ingress
    .type   silica_rt_supervisor_drain_ingress, @function
silica_rt_supervisor_drain_ingress:
    entry   a1, 32
    mov     a6, a2
    beqz    a6, 9f
1:  SCHED_LOCK a8, a9
    l32i    a7, a6, ACB_IN_HEAD
    beqz    a7, 8f
    l32i    a4, a7, MSG_NEXT
    s32i    a4, a6, ACB_IN_HEAD
    bnez    a4, 2f
    s32i    a4, a6, ACB_IN_TAIL
2:  l32i    a4, a6, ACB_IN_DEPTH
    beqz    a4, 3f
    addi    a4, a4, -1
    s32i    a4, a6, ACB_IN_DEPTH
3:  SCHED_UNLOCK a8, a9
    l32i    a4, a7, MSG_INGRESS_KIND
    bnez    a4, 4f
    l32i    a5, a7, MSG_PAYLOAD
    beqz    a5, 4f
    l32i    a10, a5, 0
    mov     a11, a6
    l32i    a12, a5, 8
    call8   silica_rt_supervisor_maybe_restart
    mov     a10, a5
    call8   silica_rt_free
4:  mov     a10, a7
    call8   silica_rt_free_node
    j       1b
8:  SCHED_UNLOCK a8, a9
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_supervisor_drain_ingress, . - silica_rt_supervisor_drain_ingress

// silica_rt_supervisor_drop_pending_exit(sup, child): remove the exit notice a deliberately stopped
// child left in the supervisor's ingress, so no restart follows.
    .align  4
silica_rt_supervisor_drop_pending_exit:
    entry   a1, 32
    SCHED_LOCK a8, a9
    addi    a4, a2, ACB_IN_HEAD         // a4 = &link to the current node
    l32i    a5, a4, 0
1:  beqz    a5, 8f
    l32i    a6, a5, MSG_INGRESS_KIND
    bnez    a6, 2f
    l32i    a6, a5, MSG_PAYLOAD
    beqz    a6, 2f
    l32i    a7, a6, 0
    beq     a7, a3, 3f
2:  addi    a4, a5, MSG_NEXT
    l32i    a5, a5, MSG_NEXT
    j       1b
3:  l32i    a7, a5, MSG_NEXT
    s32i    a7, a4, 0
    bnez    a7, 4f
    // the removed node was the tail: the tail is the node whose link we rewrote (or none)
    addi    a7, a2, ACB_IN_HEAD
    beq     a4, a7, 31f
    addi    a7, a4, -MSG_NEXT
    s32i    a7, a2, ACB_IN_TAIL
    j       4f
31: movi    a7, 0
    s32i    a7, a2, ACB_IN_TAIL
4:  l32i    a7, a2, ACB_IN_DEPTH
    beqz    a7, 5f
    addi    a7, a7, -1
    s32i    a7, a2, ACB_IN_DEPTH
5:  SCHED_UNLOCK a8, a9
    mov     a10, a6
    call8   silica_rt_free
    mov     a10, a5
    call8   silica_rt_free_node
    retw
8:  SCHED_UNLOCK a8, a9
    retw
    .size   silica_rt_supervisor_drop_pending_exit, . - silica_rt_supervisor_drop_pending_exit

// silica_rt_supervisor_stop_child(sup, row): end the child in a row and wait until it is gone
// (spec §15.4.12.2 sequential stop); its exit notice is dropped and the flag cleared.
    .align  4
silica_rt_supervisor_stop_child:
    entry   a1, 32
    l32i    a4, a3, ROW_CHILD
    beqz    a4, 9f
    mov     a10, a4
    movi    a11, 0
    call8   silica_rt_end_actor
1:  l32i    a5, a3, ROW_EXIT_DONE
    bnez    a5, 2f
    memw
    j       1b
2:  mov     a10, a2
    mov     a11, a4
    call8   silica_rt_supervisor_drop_pending_exit
    movi    a5, 0
    s32i    a5, a3, ROW_EXIT_DONE
9:  retw
    .size   silica_rt_supervisor_stop_child, . - silica_rt_supervisor_stop_child

// ---------------------------------------------------------------------------------------------
// silica_rt_supervisor_maybe_restart(dead child, supervisor, reason tag): the restart
// decision (§15.4.12). Restart policy: :permanent always, :transient only after an abnormal exit,
// anything else tombstones the row. Frequency cap: allowed_restart_count within
// restarts_time_frame seconds (both 0 = unlimited; allowed 0 with a frame = no restarts); a breach
// ends the supervisor (a root supervisor reports first). Strategy: :one_for_one respawns the row,
// :one_for_all stops the other live children then respawns every row, :rest_for_one stops the
// children after the dead one then respawns from it on.
    .align  4
    .global silica_rt_supervisor_maybe_restart
    .type   silica_rt_supervisor_maybe_restart, @function
silica_rt_supervisor_maybe_restart:
    entry   a1, 64
    // internal convention: a2 child, a3 supervisor, a4 reason tag (callers pass a10-a12)
    s32i    a4, a1, 0                   // reason tag
    mov     a6, a3                      // a6 = supervisor
    l32i    a7, a6, ACB_CT_HEAD         // a7 = row
    movi    a5, 0                       // a5 = row index
1:  beqz    a7, 9f
    l32i    a4, a7, ROW_CHILD
    beq     a4, a2, 2f
    l32i    a7, a7, ROW_NEXT
    addi    a5, a5, 1
    j       1b
2:  // policy
    l32i    a4, a7, ROW_RESTART
    movi    a8, ATOM_PERMANENT
    beq     a4, a8, 3f
    movi    a8, ATOM_TRANSIENT
    bne     a4, a8, 8f
    l32i    a4, a1, 0
    beqz    a4, 8f
3:  // frequency cap
    l32i    a3, a6, ACB_ALLOWED
    l32i    a4, a6, ACB_TIMEFRAME
    or      a8, a3, a4
    beqz    a8, 4f
    beqz    a3, 7f
    call8   silica_rt_now_seconds
    addi    a2, a10, 1                  // a2 = now + 1 (0 marks "no window yet")
    l32i    a8, a6, ACB_WINDOW
    beqz    a8, 31f
    sub     a9, a2, a8
    bltu    a4, a9, 31f                 // outside the window: start a new one
    l32i    a8, a6, ACB_RESTARTS
    addi    a8, a8, 1
    bltu    a3, a8, 7f                  // count > allowed
    s32i    a8, a6, ACB_RESTARTS
    j       4f
31: s32i    a2, a6, ACB_WINDOW
    movi    a8, 1
    s32i    a8, a6, ACB_RESTARTS
    bltu    a3, a8, 7f
4:  // strategy
    l32i    a4, a6, ACB_STRATEGY
    movi    a8, ATOM_ONE_FOR_ALL
    beq     a4, a8, 5f
    movi    a8, ATOM_REST_FOR_ONE
    beq     a4, a8, 6f
    mov     a10, a7
    mov     a11, a6
    call8   silica_rt_supervisor_spawn_child_row_by_flavor
    j       9f
5:  // one_for_all: stop every other live child, then respawn every row that has a child
    l32i    a3, a6, ACB_CT_HEAD
51: beqz    a3, 52f
    beq     a3, a7, 511f
    l32i    a4, a3, ROW_CHILD
    beqz    a4, 511f
    mov     a10, a6
    mov     a11, a3
    call8   silica_rt_supervisor_stop_child
511: l32i   a3, a3, ROW_NEXT
    j       51b
52: l32i    a3, a6, ACB_CT_HEAD
53: beqz    a3, 9f
    l32i    a4, a3, ROW_CHILD
    beqz    a4, 531f
    mov     a10, a3
    mov     a11, a6
    call8   silica_rt_supervisor_spawn_child_row_by_flavor
531: l32i   a3, a3, ROW_NEXT
    j       53b
6:  // rest_for_one: stop and tombstone the rows after the dead one, then respawn from it on
    l32i    a3, a7, ROW_NEXT
61: beqz    a3, 62f
    l32i    a4, a3, ROW_CHILD
    beqz    a4, 611f
    mov     a10, a6
    mov     a11, a3
    call8   silica_rt_supervisor_stop_child
    movi    a4, 0
    s32i    a4, a3, ROW_CHILD
611: l32i   a3, a3, ROW_NEXT
    j       61b
62: mov     a3, a7
63: beqz    a3, 9f
    mov     a10, a3
    mov     a11, a6
    call8   silica_rt_supervisor_spawn_child_row_by_flavor
    l32i    a3, a3, ROW_NEXT
    j       63b
7:  // cap breached: the supervisor ends. A root supervisor reports its own failure first.
    l32i    a8, a6, ACB_SUP
    bnez    a8, 71f
    l32i    a8, a6, ACB_SYNC_REPORTED
    bnez    a8, 71f
    movi    a8, 1
    s32i    a8, a6, ACB_SYNC_REPORTED
    movi    a8, 0
    s32i    a8, a6, ACB_ALIVE
    mov     a10, a6
    call8   silica_rt_actor_deliver_unwind_report
    movi    a10, silica_rt_sup_root_exit_msg
    call8   silica_rt_puts
71: mov     a10, a6
    movi    a11, 0
    call8   silica_rt_end_actor
    j       9f
8:  // no restart: tombstone
    movi    a4, 0
    s32i    a4, a7, ROW_CHILD
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_supervisor_maybe_restart, . - silica_rt_supervisor_maybe_restart

// ---------------------------------------------------------------------------------------------
// Behaviour-only accessors (trials)

// silica_rt_child_table_first_ref() -> the running supervisor's first live child, or 0
    .align  4
    .global silica_rt_child_table_first_ref
    .type   silica_rt_child_table_first_ref, @function
silica_rt_child_table_first_ref:
    entry   a1, 32
    call8   silica_rt_current_actor
    movi    a2, 0
    movi    a3, 0
    beqz    a10, 9f
    l32i    a4, a10, ACB_CT_HEAD
1:  beqz    a4, 9f
    l32i    a2, a4, ROW_CHILD
    bnez    a2, 9f
    l32i    a4, a4, ROW_NEXT
    j       1b
9:  retw
    .size   silica_rt_child_table_first_ref, . - silica_rt_child_table_first_ref

// silica_rt_child_table_row_ref(X0 index) -> the child of that row (0 for a tombstone / no row)
    .align  4
    .global silica_rt_child_table_row_ref
    .type   silica_rt_child_table_row_ref, @function
silica_rt_child_table_row_ref:
    entry   a1, 32
    mov     a6, a2
    call8   silica_rt_current_actor
    movi    a2, 0
    movi    a3, 0
    beqz    a10, 9f
    bltz    a6, 9f
    l32i    a4, a10, ACB_CT_LEN
    bgeu    a6, a4, 9f
    mov     a11, a6
    call8   silica_rt_child_table_row_at
    beqz    a10, 9f
    l32i    a2, a10, ROW_CHILD
9:  retw
    .size   silica_rt_child_table_row_ref, . - silica_rt_child_table_row_ref

// ---------------------------------------------------------------------------------------------
// call_supervisor helpers (the SIR generator lowers call_supervisor to these prims)

// silica_rt_supervisor_count_children(X0 sup) -> live rows
    .align  4
    .global silica_rt_supervisor_count_children
    .type   silica_rt_supervisor_count_children, @function
silica_rt_supervisor_count_children:
    entry   a1, 32
    mov     a6, a2
    mov     a10, a6
    call8   silica_rt_supervisor_drain_ingress
    movi    a2, 0
    movi    a3, 0
    l32i    a4, a6, ACB_CT_HEAD
1:  beqz    a4, 9f
    l32i    a5, a4, ROW_CHILD
    beqz    a5, 2f
    addi    a2, a2, 1
2:  l32i    a4, a4, ROW_NEXT
    j       1b
9:  retw
    .size   silica_rt_supervisor_count_children, . - silica_rt_supervisor_count_children

// silica_rt_first_live_row(sup) -> row or 0
    .align  4
silica_rt_first_live_row:
    entry   a1, 32
    l32i    a4, a2, ACB_CT_HEAD
1:  beqz    a4, 9f
    l32i    a5, a4, ROW_CHILD
    bnez    a5, 8f
    l32i    a4, a4, ROW_NEXT
    j       1b
8:  mov     a2, a4
    movi    a3, 0
    retw
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_first_live_row, . - silica_rt_first_live_row

// silica_rt_live_row_by_id(sup, id) -> row or 0
    .align  4
silica_rt_live_row_by_id:
    entry   a1, 32
    l32i    a4, a2, ACB_CT_HEAD
1:  beqz    a4, 9f
    l32i    a5, a4, ROW_CHILD
    beqz    a5, 2f
    l32i    a5, a4, ROW_ID
    beq     a5, a3, 8f
2:  l32i    a4, a4, ROW_NEXT
    j       1b
8:  mov     a2, a4
    movi    a3, 0
    retw
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_live_row_by_id, . - silica_rt_live_row_by_id

// silica_rt_supervisor_get_child(X0 sup, X1 id) -> the first live child (the reference ignores id)
    .align  4
    .global silica_rt_supervisor_get_child
    .type   silica_rt_supervisor_get_child, @function
silica_rt_supervisor_get_child:
    entry   a1, 32
    mov     a6, a2
    mov     a10, a6
    call8   silica_rt_supervisor_drain_ingress
    mov     a10, a6
    call8   silica_rt_first_live_row
    movi    a2, 0
    movi    a3, 0
    beqz    a10, 9f
    l32i    a2, a10, ROW_CHILD
9:  retw
    .size   silica_rt_supervisor_get_child, . - silica_rt_supervisor_get_child

// silica_rt_supervisor_get_child_info(X0 sup, X1 id) -> { id, agent_type, child, restart } (32 B)
    .align  4
    .global silica_rt_supervisor_get_child_info
    .type   silica_rt_supervisor_get_child_info, @function
silica_rt_supervisor_get_child_info:
    entry   a1, 32
    mov     a6, a2
    mov     a7, a4                      // requested id
    mov     a10, a6
    call8   silica_rt_supervisor_drain_ingress
    mov     a10, a6
    call8   silica_rt_first_live_row
    mov     a5, a10
    movi    a10, 32
    call8   silica_rt_alloc
    mov     a2, a10
    movi    a3, 0
    beqz    a5, 9f
    s32i    a7, a2, 0
    l32i    a4, a5, ROW_AGENT
    movi    a8, ATOM_UNKNOWN
    bne     a4, a8, 1f
    l32i    a4, a5, ROW_ID
1:  s32i    a4, a2, 8
    l32i    a4, a5, ROW_CHILD
    s32i    a4, a2, 16
    l32i    a4, a5, ROW_RESTART
    bnez    a4, 2f
    movi    a4, ATOM_PERMANENT
2:  s32i    a4, a2, 24
9:  retw
    .size   silica_rt_supervisor_get_child_info, . - silica_rt_supervisor_get_child_info

// silica_rt_supervisor_get_child_id / _agent_type / _restart (X0 sup, X1 id) -> the row's field, or 0
    .align  4
    .global silica_rt_supervisor_get_child_id
    .type   silica_rt_supervisor_get_child_id, @function
silica_rt_supervisor_get_child_id:
    entry   a1, 32
    movi    a6, ROW_ID
    j       silica_rt_supervisor_row_field
    .size   silica_rt_supervisor_get_child_id, . - silica_rt_supervisor_get_child_id

    .align  4
    .global silica_rt_supervisor_get_child_agent_type
    .type   silica_rt_supervisor_get_child_agent_type, @function
silica_rt_supervisor_get_child_agent_type:
    entry   a1, 32
    movi    a6, ROW_AGENT
    j       silica_rt_supervisor_row_field
    .size   silica_rt_supervisor_get_child_agent_type, . - silica_rt_supervisor_get_child_agent_type

    .align  4
    .global silica_rt_supervisor_get_child_restart
    .type   silica_rt_supervisor_get_child_restart, @function
silica_rt_supervisor_get_child_restart:
    entry   a1, 32
    movi    a6, ROW_RESTART
    j       silica_rt_supervisor_row_field
    .size   silica_rt_supervisor_get_child_restart, . - silica_rt_supervisor_get_child_restart

// shared tail: a2 = sup, a4 = id, a6 = field offset (entered in the caller's window)
    .align  4
silica_rt_supervisor_row_field:
    mov     a7, a2
    mov     a10, a7
    call8   silica_rt_supervisor_drain_ingress
    mov     a10, a7
    mov     a11, a4
    call8   silica_rt_live_row_by_id
    movi    a2, 0
    movi    a3, 0
    beqz    a10, 9f
    add     a10, a10, a6
    l32i    a2, a10, 0
9:  retw
    .size   silica_rt_supervisor_row_field, . - silica_rt_supervisor_row_field

// silica_rt_supervisor_which_children(X0 sup) -> List[{ id, agent_type, child, restart }]: built the
// way the reference builds it, one 32-byte record per 40-byte chunk, newest first.
    .align  4
    .global silica_rt_supervisor_which_children
    .type   silica_rt_supervisor_which_children, @function
silica_rt_supervisor_which_children:
    entry   a1, 32
    mov     a6, a2
    mov     a10, a6
    call8   silica_rt_supervisor_drain_ingress
    movi    a7, 0                       // a7 = list handle so far
    l32i    a5, a6, ACB_CT_HEAD
1:  beqz    a5, 9f
    l32i    a4, a5, ROW_CHILD
    beqz    a4, 8f
    movi    a10, 40
    call8   silica_rt_alloc
    mov     a2, a10                     // a2 = chunk
    l32i    a4, a5, ROW_ID
    s32i    a4, a2, 0
    l32i    a4, a5, ROW_AGENT
    s32i    a4, a2, 8
    l32i    a4, a5, ROW_CHILD
    s32i    a4, a2, 16
    l32i    a4, a5, ROW_RESTART
    s32i    a4, a2, 24
    beqz    a7, 2f
    l32i    a4, a7, 8
    s32i    a4, a2, 32
2:  movi    a10, 24
    call8   silica_rt_alloc
    movi    a4, 1
    beqz    a7, 3f
    l32i    a4, a7, 0
    addi    a4, a4, 1
3:  s32i    a4, a10, 0
    s32i    a2, a10, 8
    mov     a7, a10
8:  l32i    a5, a5, ROW_NEXT
    j       1b
9:  mov     a2, a7
    movi    a3, 0
    retw
    .size   silica_rt_supervisor_which_children, . - silica_rt_supervisor_which_children

// silica_rt_supervisor_terminate_child / remove_child (X0 sup, X1 id) -> 0
    .align  4
    .global silica_rt_supervisor_terminate_child
    .global silica_rt_supervisor_remove_child
    .type   silica_rt_supervisor_terminate_child, @function
silica_rt_supervisor_terminate_child:
silica_rt_supervisor_remove_child:
    entry   a1, 32
    mov     a6, a2
    mov     a7, a4
    mov     a10, a6
    call8   silica_rt_supervisor_drain_ingress
    mov     a10, a6
    mov     a11, a7
    call8   silica_rt_live_row_by_id
    beqz    a10, 9f
    mov     a5, a10
    mov     a10, a6
    mov     a11, a5
    call8   silica_rt_supervisor_stop_child
    movi    a4, 0
    s32i    a4, a5, ROW_CHILD
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_supervisor_terminate_child, . - silica_rt_supervisor_terminate_child

// silica_rt_supervisor_restart_child(X0 sup, X1 id) -> the restarted child (0 when the id is unknown)
    .align  4
    .global silica_rt_supervisor_restart_child
    .type   silica_rt_supervisor_restart_child, @function
silica_rt_supervisor_restart_child:
    entry   a1, 48
    mov     a6, a2
    mov     a7, a4
    mov     a10, a6
    call8   silica_rt_supervisor_drain_ingress
    // the row with this id, live or tombstoned
    l32i    a5, a6, ACB_CT_HEAD
1:  beqz    a5, 9f
    l32i    a4, a5, ROW_ID
    beq     a4, a7, 2f
    l32i    a5, a5, ROW_NEXT
    j       1b
2:  l32i    a4, a6, ACB_STRATEGY
    movi    a8, ATOM_ONE_FOR_ALL
    beq     a4, a8, 5f
    l32i    a4, a5, ROW_CHILD
    beqz    a4, 3f
    mov     a10, a6
    mov     a11, a5
    call8   silica_rt_supervisor_stop_child
3:  mov     a10, a5
    mov     a11, a6
    call8   silica_rt_supervisor_spawn_child_row_by_flavor
    l32i    a2, a5, ROW_CHILD
    movi    a3, 0
    retw
5:  // one_for_all: stop every live child, then respawn every row
    movi    a2, 0
    l32i    a3, a6, ACB_CT_HEAD
51: beqz    a3, 52f
    l32i    a4, a3, ROW_CHILD
    beqz    a4, 511f
    mov     a10, a6
    mov     a11, a3
    call8   silica_rt_supervisor_stop_child
511: l32i   a3, a3, ROW_NEXT
    j       51b
52: l32i    a3, a6, ACB_CT_HEAD
53: beqz    a3, 8f
    mov     a10, a3
    mov     a11, a6
    call8   silica_rt_supervisor_spawn_child_row_by_flavor
    l32i    a4, a3, ROW_ID
    bne     a4, a7, 531f
    l32i    a2, a3, ROW_CHILD
531: l32i   a3, a3, ROW_NEXT
    j       53b
8:  movi    a3, 0
    retw
9:  movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_rt_supervisor_restart_child, . - silica_rt_supervisor_restart_child
