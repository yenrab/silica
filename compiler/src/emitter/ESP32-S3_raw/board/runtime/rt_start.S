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

// Reset entry for the bare-metal ESP32-S3 runtime (no ESP-IDF, no FreeRTOS).
//
// The image is flashed at offset 0x0 in place of the second-stage bootloader, so the ROM loads
// its segments into internal SRAM and calls _start as an ordinary windowed function. _start never
// returns to the ROM:
//   1. mask interrupts, move VECBASE to rt_vectors, drop every older register window;
//   2. switch to our own stack (the ROM's is a few KB, Silica recursion needs far more);
//   3. disable the RTC and timer-group watchdogs the ROM leaves armed, and set the super
//      watchdog to auto-feed (as the IDF bootloader does);
//   4. zero .bss and the heap;
//   5. arm the stack guards: data watchpoints on the lowest 64 bytes of the machine stack and of the
//      auxiliary stack, so an overflow raises a debug exception (reported as cause 1006) instead of
//      silently corrupting whatever lies below;
//   6. call the program's `main` and print the exit marker with its result (low byte, like a host
//      process exit status).
//
// Stack budget: the machine stack (128 KB, silica_esp32s3.ld) holds the windowed frames; the auxiliary
// stack (64 KB, below) holds the emitter's AArch64-style pushes and slabs. A recursion of depth 2000
// with one 16-byte push per level and 48-byte frames uses 96 KB + 32 KB.
//
// Console protocol (read by board/tools/run_on_board.py):
//   \x02SILICA:START\x03            printed before main runs
//   \x02SILICA:EXIT:<status>\x03    printed when the program ends; any diagnostic follows it
// Program output is exactly the bytes between the two markers, so the host runner can rebuild the
// same `<output><status>\n` text the trial harness writes to a .sout file.

#define RTC_CNTL_WDTCONFIG0_REG   0x60008098
#define RTC_CNTL_WDTWPROTECT_REG  0x600080B0
#define RTC_CNTL_SWD_CONF_REG     0x600080B4
#define RTC_CNTL_SWD_WPROTECT_REG 0x600080B8
#define TIMG0_WDTCONFIG0_REG      0x6001F048
#define TIMG0_WDTWPROTECT_REG     0x6001F064
#define TIMG1_WDTCONFIG0_REG      0x60020048
#define TIMG1_WDTWPROTECT_REG     0x60020064
#define WDT_WKEY_VALUE            0x50D83AA1
#define SWD_WKEY_VALUE            0x8F1D312A
#define SWD_AUTO_FEED_EN          0x80000000

    .section .text.silica_rt_start, "ax"
    .literal_position
    .align  4
    .global _start
    .type   _start, @function
_start:
    entry   a1, 32

    // 1. No interrupts; our vectors; this window is the only live one.
    movi    a2, 0
    wsr     a2, intenable
    movi    a2, _silica_vecbase
    wsr     a2, vecbase
    rsr     a2, windowbase
    movi    a3, 1
    ssl     a2
    sll     a3, a3
    wsr     a3, windowstart
    rsync
    movi    a2, 0x40020            // PS: WOE=1, UM=1, INTLEVEL=0, EXCM=0
    wsr     a2, ps
    rsync

    // 2. Our stack. a0 = 0 terminates backtraces; a1 must be 16-byte aligned.
    movi    a0, 0
    movi    a1, _silica_stack_top

    // 3. Watchdogs.
    movi    a4, WDT_WKEY_VALUE
    movi    a5, 0
    movi    a2, RTC_CNTL_WDTWPROTECT_REG
    s32i    a4, a2, 0
    movi    a3, RTC_CNTL_WDTCONFIG0_REG
    s32i    a5, a3, 0
    s32i    a5, a2, 0
    movi    a2, TIMG0_WDTWPROTECT_REG
    s32i    a4, a2, 0
    movi    a3, TIMG0_WDTCONFIG0_REG
    s32i    a5, a3, 0
    s32i    a5, a2, 0
    movi    a2, TIMG1_WDTWPROTECT_REG
    s32i    a4, a2, 0
    movi    a3, TIMG1_WDTCONFIG0_REG
    s32i    a5, a3, 0
    s32i    a5, a2, 0
    movi    a2, RTC_CNTL_SWD_WPROTECT_REG
    movi    a4, SWD_WKEY_VALUE
    s32i    a4, a2, 0
    movi    a3, RTC_CNTL_SWD_CONF_REG
    l32i    a4, a3, 0
    movi    a6, SWD_AUTO_FEED_EN
    or      a4, a4, a6
    s32i    a4, a3, 0
    s32i    a5, a2, 0
    memw

    // 4. Zero .bss, then the heap (allocations are returned zeroed).
    movi    a2, _silica_bss_start
    movi    a3, _silica_bss_end
    call0   silica_rt_zero_words_c0
    movi    a2, _silica_heap_start
    movi    a3, _silica_heap_end
    call0   silica_rt_zero_words_c0

    // 5. Stack guards (Xtensa data breakpoints: DBREAKC bit 31 = break on store, mask 0 = 64-byte
    //    range; both addresses are 64-byte aligned). Armed after the zeroing above, which stores there.
    movi    a2, _silica_heap_end          // machine stack bottom
    wsr     a2, dbreaka0
    movi    a3, 0x80000000
    wsr     a3, dbreakc0
    movi    a2, silica_rt_aux_stack_bottom
    wsr     a2, dbreaka1
    wsr     a3, dbreakc1
    isync

    // 6. The program.
    call8   silica_rt_start_windowed
1:  j       1b

// call0 leaf: zero [a2, a3), both 4-byte aligned. Clobbers a2, a4.
    .align  4
silica_rt_zero_words_c0:
    movi    a4, 0
    bgeu    a2, a3, 2f
1:  s32i    a4, a2, 0
    addi    a2, a2, 4
    bltu    a2, a3, 1b
2:  ret

// Runs on our stack in a fresh windowed frame: aux stack, heap init, start marker, main, exit marker.
    .align  4
    .global silica_rt_start_windowed
    .type   silica_rt_start_windowed, @function
silica_rt_start_windowed:
    entry   a1, 32
    movi    a2, silica_rt_vsp
    movi    a3, silica_rt_aux_stack_top
    s32i    a3, a2, 0
    call8   silica_rt_heap_init
    movi    a10, silica_rt_start_marker
    call8   silica_rt_puts
    call8   main                   // int64 result in a10:a11
    call8   silica_rt_exit         // a10 = low word = status
1:  j       1b
    .size   silica_rt_start_windowed, . - silica_rt_start_windowed

// Machine state the emitted code keeps in memory (design_documents/ports/esp32s3_xtensa_port.md §2):
//   silica_rt_vrg  the global transfer block holding the emitter's virtual registers X3-X8 (0..47) and
//                  D0-D7 (48..111). They are caller-saved, so one block shared by every frame is exact.
//   silica_rt_vsp  the auxiliary data stack pointer: the emitter's SP, which AArch64-style pushes and
//                  stack slabs move. The machine stack pointer a1 cannot move inside a windowed frame.
#define AUX_STACK_SIZE 0x10000

    .section .bss.silica_rt_aux_stack, "aw", @nobits
    .balign 64
silica_rt_aux_stack_bottom:                // the lowest 64 bytes are the guard (watchpoint 1)
    .space  AUX_STACK_SIZE
    .global silica_rt_aux_stack_top
silica_rt_aux_stack_top:

    .section .bss.silica_rt_regs, "aw", @nobits
    .balign 16
    .global silica_rt_vrg
silica_rt_vrg:
    .space  112
    .global silica_rt_vsp
silica_rt_vsp:
    .space  4

    .section .rodata.silica_rt_start, "a"
silica_rt_start_marker:
    .byte   0x02
    .ascii  "SILICA:START"
    .byte   0x03, 0
