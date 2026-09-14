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

// Heap for the bare-metal ESP32-S3 runtime: a bump allocator over [_silica_heap_start,
// _silica_heap_end), which rt_start zeroes before main runs.
//
// This is bring-up scaffolding with a known limit, not the region allocator the language needs:
// free is a no-op, so a long-running program eventually exhausts the heap. Exhaustion is a hard
// stop (message after the exit marker, status 134) rather than a NULL the caller might not check.
//
//   silica_rt_heap_init()          reset the bump pointer (called once by rt_start)
//   silica_rt_alloc(size) -> ptr   8-byte aligned, zeroed; size is a 32-bit byte count
//   silica_rt_free(ptr)            accepted and ignored (also silica_rt_region_free / _destroy)
//   silica_rt_heap_used() -> bytes
//   silica_rt_region_grow / silica_rt_region_contains   region blocks, see below

    .section .bss.silica_rt_heap, "aw", @nobits
    .align  4
silica_rt_heap_next:
    .space  4

    .section .text.silica_rt_heap, "ax"
    .literal_position

    .align  4
    .global silica_rt_heap_init
    .type   silica_rt_heap_init, @function
silica_rt_heap_init:
    entry   a1, 32
    movi    a3, silica_rt_heap_next
    movi    a4, _silica_heap_start
    s32i    a4, a3, 0
    retw
    .size   silica_rt_heap_init, . - silica_rt_heap_init

    .align  4
    .global silica_rt_alloc
    .global silica_rt_region_alloc
    .global silica_rt_raw_alloc
    .set    silica_rt_region_alloc, silica_rt_alloc    // the emitter's aggregate/pair allocations
    .set    silica_rt_raw_alloc, silica_rt_alloc       // alloc_region's first block
    .type   silica_rt_alloc, @function
silica_rt_alloc:
    entry   a1, 32
    addi    a2, a2, 7
    movi    a5, -8
    and     a2, a2, a5            // round size up to 8
    movi    a3, silica_rt_heap_next
    l32i    a4, a3, 0             // a4 = result
    add     a6, a4, a2            // a6 = new next
    bltu    a6, a4, 1f            // wrapped
    movi    a7, _silica_heap_end
    bltu    a7, a6, 1f            // past the end
    s32i    a6, a3, 0
    mov     a2, a4
    movi    a3, 0                 // the high word of the pair the emitter reads as X0
    retw
1:  movi    a10, 134
    call8   silica_rt_exit_marker
    movi    a10, silica_rt_heap_oom_msg
    call8   silica_rt_puts
    call8   silica_rt_halt
    retw
    .size   silica_rt_alloc, . - silica_rt_alloc

    .align  4
    .global silica_rt_free
    .global silica_rt_region_free
    .global silica_rt_region_destroy
    .set    silica_rt_region_free, silica_rt_free       // region release: nothing to give back yet
    .set    silica_rt_region_destroy, silica_rt_free
    .type   silica_rt_free, @function
silica_rt_free:
    entry   a1, 32
    retw
    .size   silica_rt_free, . - silica_rt_free

// Regions (terms/prims/prims_memory.silica): a region is a chain of blocks, each with a 32-byte
// header: [0] bump, [8] end of the block being filled (these two only matter in the first block,
// the handle), [16] next extension block, [24] this block's own data end; data from +32. Fields
// are 8-byte slots with the high word 0, as the emitted code reads them.
//
// silica_rt_region_grow: called from the middle of an emitted allocation sequence when the current
// block is full. Convention inherited from the AArch64 helper: the region is X1 (here a4), the
// request is X5 - X3 (both live in the global block silica_rt_vrg, at 16 and 0); every register the
// caller holds is preserved (a2-a7 untouched, a8-a15 are the caller's own scratch). A block holds at
// least 4096 data bytes, or the 16-byte-rounded request when that is larger.
    .align  4
    .global silica_rt_region_grow
    .type   silica_rt_region_grow, @function
silica_rt_region_grow:
    entry   a1, 48
    movi    a8, silica_rt_vrg
    l32i    a9, a8, 16            // X5 = bump + request
    l32i    a8, a8, 0             // X3 = bump
    sub     a9, a9, a8
    addi    a9, a9, 15
    movi    a8, -16
    and     a9, a9, a8            // data bytes, rounded up to 16
    movi    a8, 4096
    bgeu    a9, a8, 1f
    mov     a9, a8
1:  s32i    a9, a1, 0             // keep the size across the call
    addi    a10, a9, 32
    call8   silica_rt_alloc       // never returns 0 (exhaustion halts)
    l32i    a9, a1, 0
    addi    a8, a10, 32           // data start
    add     a9, a8, a9            // data end
    s32i    a9, a10, 24           // block's own span end
    l32i    a11, a4, 16           // chain the block in front of the region's extensions
    s32i    a11, a10, 16
    s32i    a10, a4, 16
    s32i    a8, a4, 0             // the region now bumps in the new block
    s32i    a9, a4, 8
    retw
    .size   silica_rt_region_grow, . - silica_rt_region_grow

// silica_rt_region_contains: ref X1 (a4), region X2 (a6) -> X5 (silica_rt_vrg + 16) = 1 when the ref
// lies in the region's first block or any extension block; everything else preserved.
    .align  4
    .global silica_rt_region_contains
    .type   silica_rt_region_contains, @function
silica_rt_region_contains:
    entry   a1, 32
    addi    a8, a6, 32            // first block: [region + 32, [region + 24])
    l32i    a9, a6, 24
    bltu    a4, a8, 2f
    bltu    a4, a9, 3f
2:  l32i    a8, a6, 16            // extension blocks
1:  beqz    a8, 4f
    addi    a9, a8, 32
    bltu    a4, a9, 5f
    l32i    a9, a8, 24
    bltu    a4, a9, 3f
5:  l32i    a8, a8, 16
    j       1b
3:  movi    a8, 1
    j       6f
4:  movi    a8, 0
6:  movi    a9, silica_rt_vrg
    s32i    a8, a9, 16
    movi    a8, 0
    s32i    a8, a9, 20
    retw
    .size   silica_rt_region_contains, . - silica_rt_region_contains

    .align  4
    .global silica_rt_heap_used
    .type   silica_rt_heap_used, @function
silica_rt_heap_used:
    entry   a1, 32
    movi    a3, silica_rt_heap_next
    l32i    a2, a3, 0
    movi    a4, _silica_heap_start
    sub     a2, a2, a4
    retw
    .size   silica_rt_heap_used, . - silica_rt_heap_used

    .section .rodata.silica_rt_heap, "a"
silica_rt_heap_oom_msg:
    .asciz  " heap exhausted\n"
