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

// Console and process-end routines for the bare-metal ESP32-S3 runtime.
//
// All output goes to UART0 (GPIO43 TX on the port test board), which the ROM has already set up at
// 115200 8N1. Bytes are written raw to the TX FIFO: no CR/LF translation, no dropped bytes, so the
// captured stream is byte-for-byte what the program printed. (The ROM's ets_write_char_uart turns
// '\n' into "\r\n" and swallows '\r', which would make output differ from the host.)
//
// Every routine is a normal windowed (CALL8) function following the Silica convention on this target:
// argument n is a 64-bit pair (arg0 in a2:a3, arg1 in a4:a5, arg2 in a6:a7; a 32-bit value uses the low
// register), result in a2. That is what emitted code passes, with X0-X2 in the caller's a10-a15.
//
//   silica_rt_putc(c)                  one byte
//   silica_rt_write(ptr, len)          len bytes (len is a 32-bit count)
//   silica_rt_puts(cstr)               NUL-terminated
//   silica_rt_print_u32(v)             unsigned decimal
//   silica_rt_print_hex32(v)           0x-prefixed, 8 digits
//   silica_rt_flush()                  wait until the TX FIFO has drained
//   silica_rt_exit(status)             exit marker with (status & 0xFF), then halt
//   silica_rt_abort()                  exit status 134 (what abort() gives a host process)
//   silica_rt_fatal_report(cause, epc, excvaddr)   exit status 139, then the fault details
//   silica_rt_badarith()               "badarith\n", exit status 1 (division by zero; the host prints
//                                      this on stderr, which the trial harness merges into .sout)
//   silica_rt_case_clause()            "case_clause\n", exit status 1 (no case arm matched)
//   silica_rt_print_i64(v) / _u64(v)   signed / unsigned 64-bit decimal (the emitter's print_int64 etc.)
//   silica_rt_print_bool(v)            "true" / "false"
//   silica_rt_print_string(s)          the bytes of a Silica string (tag -1, data +8, length +16);
//                                      _stderr is the same routine (one console)
// The host runtime's helpers for these live in the emitter (terms/print_*_inline.silica); this target
// calls the routines here instead. 64-bit division is libgcc's __udivdi3 / __umoddi3, which follow the
// windowed C ABI (pairs in a10:a11 and a12:a13, result in a10:a11).

#define UART0_FIFO_REG    0x60000000
#define UART0_STATUS_REG  0x6000001C
#define UART_TXFIFO_MAX   120          // FIFO holds 128; leave slack

    .section .text.silica_rt_console, "ax"
    .literal_position

    .align  4
    .global silica_rt_putc
    .type   silica_rt_putc, @function
silica_rt_putc:
    entry   a1, 32
    movi    a3, UART0_STATUS_REG
    movi    a5, UART_TXFIFO_MAX
1:  l32i    a4, a3, 0
    extui   a4, a4, 16, 10
    bgeu    a4, a5, 1b
    movi    a3, UART0_FIFO_REG
    extui   a2, a2, 0, 8
    s32i    a2, a3, 0
    memw
    retw
    .size   silica_rt_putc, . - silica_rt_putc

    .align  4
    .global silica_rt_write
    .type   silica_rt_write, @function
silica_rt_write:
    entry   a1, 32
    beqz    a4, 2f
    add     a3, a2, a4            // a3 = end
    mov     a6, a2
1:  l8ui    a10, a6, 0
    call8   silica_rt_putc
    addi    a6, a6, 1
    bltu    a6, a3, 1b
2:  retw
    .size   silica_rt_write, . - silica_rt_write

    .align  4
    .global silica_rt_puts
    .type   silica_rt_puts, @function
silica_rt_puts:
    entry   a1, 32
    mov     a6, a2
1:  l8ui    a10, a6, 0
    beqz    a10, 2f
    call8   silica_rt_putc
    addi    a6, a6, 1
    j       1b
2:  retw
    .size   silica_rt_puts, . - silica_rt_puts

// Unsigned decimal. Digits are produced least significant first into a 12-byte buffer at the
// bottom of the frame, then written forwards. Frame 48: [sp+0, sp+12) buffer, top 32 reserved.
    .align  4
    .global silica_rt_print_u32
    .type   silica_rt_print_u32, @function
silica_rt_print_u32:
    entry   a1, 48
    addi    a6, a1, 12            // one past the buffer end
    mov     a7, a6
    movi    a5, 10
1:  remu    a4, a2, a5
    quou    a2, a2, a5
    addi    a4, a4, 48
    addi    a7, a7, -1
    s8i     a4, a7, 0
    bnez    a2, 1b
    mov     a10, a7
    sub     a12, a6, a7
    call8   silica_rt_write
    retw
    .size   silica_rt_print_u32, . - silica_rt_print_u32

    .align  4
    .global silica_rt_print_hex32
    .type   silica_rt_print_hex32, @function
silica_rt_print_hex32:
    entry   a1, 32
    mov     a6, a2
    movi    a10, 48               // '0'
    call8   silica_rt_putc
    movi    a10, 120              // 'x'
    call8   silica_rt_putc
    movi    a7, 28
1:  ssr     a7
    srl     a10, a6
    extui   a10, a10, 0, 4
    movi    a4, 10
    bltu    a10, a4, 2f
    addi    a10, a10, 39          // 'a' - '0' - 10
2:  addi    a10, a10, 48
    call8   silica_rt_putc
    addi    a7, a7, -4
    bgez    a7, 1b
    retw
    .size   silica_rt_print_hex32, . - silica_rt_print_hex32

    .align  4
    .global silica_rt_flush
    .type   silica_rt_flush, @function
silica_rt_flush:
    entry   a1, 32
    movi    a3, UART0_STATUS_REG
1:  l32i    a4, a3, 0
    extui   a4, a4, 16, 10
    bnez    a4, 1b
    retw
    .size   silica_rt_flush, . - silica_rt_flush

// Exit marker: \x02SILICA:EXIT:<status>\x03, status = low byte like a host exit code.
    .align  4
    .global silica_rt_exit_marker
    .type   silica_rt_exit_marker, @function
silica_rt_exit_marker:
    entry   a1, 32
    extui   a6, a2, 0, 8
    movi    a10, silica_rt_exit_prefix
    call8   silica_rt_puts
    mov     a10, a6
    call8   silica_rt_print_u32
    movi    a10, 3
    call8   silica_rt_putc
    retw
    .size   silica_rt_exit_marker, . - silica_rt_exit_marker

    .align  4
    .global silica_rt_halt
    .type   silica_rt_halt, @function
silica_rt_halt:
    entry   a1, 32
    call8   silica_rt_flush
1:  waiti   15
    j       1b
    .size   silica_rt_halt, . - silica_rt_halt

    .align  4
    .global silica_rt_exit
    .type   silica_rt_exit, @function
silica_rt_exit:
    entry   a1, 32
    mov     a10, a2
    call8   silica_rt_exit_marker
    movi    a10, 10
    call8   silica_rt_putc
    call8   silica_rt_halt
    retw
    .size   silica_rt_exit, . - silica_rt_exit

    .align  4
    .global silica_rt_abort
    .type   silica_rt_abort, @function
silica_rt_abort:
    entry   a1, 32
    movi    a10, 134
    call8   silica_rt_exit
    retw
    .size   silica_rt_abort, . - silica_rt_abort

    .align  4
    .global silica_rt_badarith
    .type   silica_rt_badarith, @function
silica_rt_badarith:
    entry   a1, 32
    movi    a10, silica_rt_badarith_msg
    call8   silica_rt_puts
    movi    a10, 1
    call8   silica_rt_exit
    retw
    .size   silica_rt_badarith, . - silica_rt_badarith

    .align  4
    .global silica_rt_case_clause
    .type   silica_rt_case_clause, @function
silica_rt_case_clause:
    entry   a1, 32
    movi    a10, silica_rt_case_clause_msg
    call8   silica_rt_puts
    movi    a10, 1
    call8   silica_rt_exit
    retw
    .size   silica_rt_case_clause, . - silica_rt_case_clause

// Unsigned 64-bit decimal: digits least significant first into a 24-byte buffer at the bottom of
// the frame, written forwards. Frame 64: [sp+0, sp+24) buffer, top 32 reserved.
    .align  4
    .global silica_rt_print_u64
    .type   silica_rt_print_u64, @function
silica_rt_print_u64:
    entry   a1, 64
    addi    a6, a1, 24            // one past the buffer end
    mov     a7, a6
    mov     a4, a2
    mov     a5, a3
1:  mov     a10, a4
    mov     a11, a5
    movi    a12, 10
    movi    a13, 0
    call8   __umoddi3
    addi    a10, a10, 48
    addi    a7, a7, -1
    s8i     a10, a7, 0
    mov     a10, a4
    mov     a11, a5
    movi    a12, 10
    movi    a13, 0
    call8   __udivdi3
    mov     a4, a10
    mov     a5, a11
    or      a8, a4, a5
    bnez    a8, 1b
    mov     a10, a7
    sub     a12, a6, a7
    call8   silica_rt_write
    retw
    .size   silica_rt_print_u64, . - silica_rt_print_u64

// Signed: a minus sign, then the magnitude (two's-complement negation of the pair; INT64_MIN negates
// to itself, which as an unsigned value is its own magnitude).
    .align  4
    .global silica_rt_print_i64
    .type   silica_rt_print_i64, @function
silica_rt_print_i64:
    entry   a1, 32
    bgez    a3, 1f
    movi    a8, 0
    saltu   a9, a8, a2            // borrow: low word non-zero
    sub     a2, a8, a2
    sub     a3, a8, a3
    sub     a3, a3, a9
    movi    a10, 45               // '-'
    call8   silica_rt_putc
1:  mov     a10, a2
    mov     a11, a3
    call8   silica_rt_print_u64
    retw
    .size   silica_rt_print_i64, . - silica_rt_print_i64

    .align  4
    .global silica_rt_print_bool
    .type   silica_rt_print_bool, @function
silica_rt_print_bool:
    entry   a1, 32
    movi    a10, silica_rt_true_str
    bnez    a2, 1f
    movi    a10, silica_rt_false_str
1:  call8   silica_rt_puts
    retw
    .size   silica_rt_print_bool, . - silica_rt_print_bool

    .align  4
    .global silica_rt_print_string
    .global silica_rt_print_string_stderr
    .type   silica_rt_print_string, @function
silica_rt_print_string:
silica_rt_print_string_stderr:
    entry   a1, 32
    l32i    a8, a2, 0
    addi    a8, a8, 1
    bnez    a8, 1f
    l32i    a10, a2, 8
    l32i    a12, a2, 16
    call8   silica_rt_write
    retw
1:  call8   silica_rt_abort       // not a Silica string
    retw
    .size   silica_rt_print_string, . - silica_rt_print_string

// a2 cause (EXCCAUSE, or 1001.. for a non-user vector), a4 EPC1, a6 EXCVADDR.
    .align  4
    .global silica_rt_fatal_report
    .type   silica_rt_fatal_report, @function
silica_rt_fatal_report:
    entry   a1, 32
    mov     a5, a6
    mov     a7, a4
    mov     a6, a2
    movi    a10, 139
    call8   silica_rt_exit_marker
    movi    a10, silica_rt_fault_cause
    call8   silica_rt_puts
    mov     a10, a6
    call8   silica_rt_print_u32
    movi    a10, silica_rt_fault_epc
    call8   silica_rt_puts
    mov     a10, a7
    call8   silica_rt_print_hex32
    movi    a10, silica_rt_fault_vaddr
    call8   silica_rt_puts
    mov     a10, a5
    call8   silica_rt_print_hex32
    movi    a8, 1006
    bne     a6, a8, 1f
    movi    a10, silica_rt_fault_guard
    call8   silica_rt_puts
1:  movi    a10, 10
    call8   silica_rt_putc
    call8   silica_rt_halt
    retw
    .size   silica_rt_fatal_report, . - silica_rt_fatal_report

    .section .rodata.silica_rt_console, "a"
silica_rt_exit_prefix:
    .byte   0x02
    .asciz  "SILICA:EXIT:"
silica_rt_fault_cause:
    .asciz  " fault: exccause="
silica_rt_fault_epc:
    .asciz  " epc="
silica_rt_fault_vaddr:
    .asciz  " excvaddr="
silica_rt_fault_guard:
    .asciz  " (stack guard: machine or auxiliary stack overflow)"
silica_rt_badarith_msg:
    .asciz  "badarith\n"
silica_rt_case_clause_msg:
    .asciz  "case_clause\n"
silica_rt_true_str:
    .asciz  "true"
silica_rt_false_str:
    .asciz  "false"
