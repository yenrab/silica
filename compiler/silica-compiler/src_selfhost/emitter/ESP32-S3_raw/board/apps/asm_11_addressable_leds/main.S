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

// Bring-up app 11: the addressable LED chain -- 24 WS2813B-2121 on GPIO7 (through J4 9-10),
// powered from the 5 V converter, GRB byte order, 24 bits per LED, most significant bit first.
//
// The WS2813B reads each bit from the length of its high pulse (0: 220-380 ns, 1: 580-1000 ns)
// and latches the colours after a low of more than 280 us. The stream is bit-banged through the
// GPIO set/clear registers. Measured on the test unit at the ROM's 20 MHz CPU clock (50 ns/cycle):
// a GPIO store is accepted every ~4.4 cycles, and "set; N nops; clear; memw" gives a high pulse of
// ~N+1 cycles, so a 0 is set + 4 nops (~250 ns) and a 1 is set + 14 nops (~750 ns). The memw after
// every bit drains the write buffer, so each bit starts from the same state; the low phase, where
// the next bit is chosen, is not timing-critical. Interrupts are off in this runtime, so nothing can
// stretch a pulse. If the CPU clock is ever raised, these nop counts must be recomputed.
//
// Brightness is kept at 0x18/0xFF per channel so 24 LEDs draw little from the 5 V rail.
// Expected: all red, all green, all blue (1 s each), one white dot running along the chain, off.
// Console: "red", "green", "blue", "chase", "off", status 0.

#define LED_DATA_PIN  7
#define N_LEDS        24
#define LEVEL         0x18

    .section .bss.main, "aw", @nobits
    .align  4
frame:  .space  N_LEDS * 3

    .section .text.main, "ax"
    .literal_position

// send_frame(): stream frame[] to the chain, then hold the line low for the latch.
    .align  4
    .type   send_frame, @function
send_frame:
    entry   a1, 32
    movi    a2, frame
    movi    a3, N_LEDS * 3
    movi    a4, 1 << LED_DATA_PIN
    movi    a5, 0x60004000        // GPIO base: +8 = OUT_W1TS, +12 = OUT_W1TC
1:  l8ui    a6, a2, 0
    movi    a7, 8
2:  bbsi    a6, 7, 3f
    s32i    a4, a5, 8             // 0 bit: ~250 ns high
    nop
    nop
    nop
    nop
    s32i    a4, a5, 12
    memw
    j       4f
3:  s32i    a4, a5, 8             // 1 bit: ~750 ns high
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    s32i    a4, a5, 12
    memw
4:  slli    a6, a6, 1
    addi    a7, a7, -1
    bnez    a7, 2b
    addi    a2, a2, 1
    addi    a3, a3, -1
    bnez    a3, 1b
    movi    a10, 500              // latch: > 280 us low
    call8   silica_rt_delay_us
    retw
    .size   send_frame, . - send_frame

// fill(g, r, b): every LED the same colour.
    .align  4
    .type   fill, @function
fill:
    entry   a1, 32
    movi    a5, frame
    movi    a6, N_LEDS
1:  s8i     a2, a5, 0
    s8i     a3, a5, 1
    s8i     a4, a5, 2
    addi    a5, a5, 3
    addi    a6, a6, -1
    bnez    a6, 1b
    retw
    .size   fill, . - fill

// show(g, r, b, name): fill, send, name it, hold one second.
    .align  4
    .type   show, @function
show:
    entry   a1, 32
    mov     a6, a5
    mov     a10, a2
    mov     a11, a3
    mov     a12, a4
    call8   fill
    call8   send_frame
    mov     a10, a6
    call8   silica_rt_puts
    movi    a10, 1000000
    call8   silica_rt_delay_us
    retw
    .size   show, . - show

    .align  4
    .global main
    .type   main, @function
main:
    entry   a1, 32
    movi    a10, LED_DATA_PIN
    call8   silica_rt_gpio_output
    movi    a10, 1000             // line low well past a reset before the first frame
    call8   silica_rt_delay_us
    movi    a10, 0
    movi    a11, LEVEL
    movi    a12, 0
    movi    a13, n_red
    call8   show
    movi    a10, LEVEL
    movi    a11, 0
    movi    a12, 0
    movi    a13, n_green
    call8   show
    movi    a10, 0
    movi    a11, 0
    movi    a12, LEVEL
    movi    a13, n_blue
    call8   show
    movi    a10, n_chase
    call8   silica_rt_puts
    movi    a2, 0                 // a2 = lit position
1:  movi    a10, 0
    movi    a11, 0
    movi    a12, 0
    call8   fill
    movi    a3, frame
    addx2   a4, a2, a2            // a4 = 3 * position
    add     a3, a3, a4
    movi    a4, LEVEL
    s8i     a4, a3, 0
    s8i     a4, a3, 1
    s8i     a4, a3, 2
    call8   send_frame
    movi    a10, 100000
    call8   silica_rt_delay_us
    addi    a2, a2, 1
    movi    a3, N_LEDS
    blt     a2, a3, 1b
    movi    a10, 0
    movi    a11, 0
    movi    a12, 0
    movi    a13, n_off
    call8   show
    movi    a2, 0
    movi    a3, 0
    retw
    .size   main, . - main

    .section .rodata.main, "a"
n_red:   .asciz "red\n"
n_green: .asciz "green\n"
n_blue:  .asciz "blue\n"
n_chase: .asciz "chase\n"
n_off:   .asciz "off\n"
