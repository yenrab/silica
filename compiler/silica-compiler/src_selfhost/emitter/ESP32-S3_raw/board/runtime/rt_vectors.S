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

// Exception vector table for the bare-metal ESP32-S3 runtime. rt_start points VECBASE here.
//
// The window overflow/underflow handlers are the standard Xtensa windowed-ABI handlers, copied
// instruction for instruction from ESP-IDF components/xtensa/xtensa_vectors.S. They define the
// frame contract the emitter must honour:
//   - the 16 bytes below a function's SP receive its caller's a0-a3 on overflow, so nothing may
//     ever be stored below SP;
//   - a function that makes CALL8 calls has its own a4-a7 spilled to [caller_sp-32, caller_sp-16),
//     i.e. the top of its frame (CALL12: a4-a11 to [caller_sp-48, caller_sp-16)).
// Locals therefore live at or below [sp + frame - 32] (frame - 48 when the function uses CALL12).
//
// Every other vector reports a fatal exception: EXCCAUSE, EPC1 and EXCVADDR go to the console after
// the exit marker (so they never pollute captured program output) and the exit status is 139, the
// status a SIGSEGV'd process reports on the host.

    .section .WindowVectors.text, "ax"

    .org    0x0
    .global _WindowOverflow4
_WindowOverflow4:
    s32e    a0, a5, -16
    s32e    a1, a5, -12
    s32e    a2, a5,  -8
    s32e    a3, a5,  -4
    rfwo

    .org    0x40
    .global _WindowUnderflow4
_WindowUnderflow4:
    l32e    a0, a5, -16
    l32e    a1, a5, -12
    l32e    a2, a5,  -8
    l32e    a3, a5,  -4
    rfwu

// Alloca exception (EXCCAUSE 5), raised by MOVSP when the caller's base-save registers are live.
// Same code as ESP-IDF's _xt_alloca_exc with the LX7 PS.OWB field (shift 8, 4 bits) spelled out.
    .align  4
    .global silica_rt_alloca_exc
silica_rt_alloca_exc:
    rsr     a0, windowbase
    rotw    -1
    rsr     a2, ps
    extui   a3, a2, 8, 4
    xor     a3, a3, a4
    rsr     a4, excsave1
    slli    a3, a3, 8
    xor     a2, a2, a3
    wsr     a2, ps
    rsync
    bbci.l  a4, 31, _WindowUnderflow4
    rotw    -1
    bbci.l  a8, 30, _WindowUnderflow8
    rotw    -1
    j       _WindowUnderflow12

    .org    0x80
    .global _WindowOverflow8
_WindowOverflow8:
    s32e    a0, a9, -16
    l32e    a0, a1, -12
    s32e    a1, a9, -12
    s32e    a2, a9,  -8
    s32e    a3, a9,  -4
    s32e    a4, a0, -32
    s32e    a5, a0, -28
    s32e    a6, a0, -24
    s32e    a7, a0, -20
    rfwo

    .org    0xC0
    .global _WindowUnderflow8
_WindowUnderflow8:
    l32e    a0, a9, -16
    l32e    a1, a9, -12
    l32e    a2, a9,  -8
    l32e    a7, a1, -12
    l32e    a3, a9,  -4
    l32e    a4, a7, -32
    l32e    a5, a7, -28
    l32e    a6, a7, -24
    l32e    a7, a7, -20
    rfwu

    .org    0x100
    .global _WindowOverflow12
_WindowOverflow12:
    s32e    a0,  a13, -16
    l32e    a0,  a1,  -12
    s32e    a1,  a13, -12
    s32e    a2,  a13,  -8
    s32e    a3,  a13,  -4
    s32e    a4,  a0,  -48
    s32e    a5,  a0,  -44
    s32e    a6,  a0,  -40
    s32e    a7,  a0,  -36
    s32e    a8,  a0,  -32
    s32e    a9,  a0,  -28
    s32e    a10, a0,  -24
    s32e    a11, a0,  -20
    rfwo

    .org    0x140
    .global _WindowUnderflow12
_WindowUnderflow12:
    l32e    a0,  a13, -16
    l32e    a1,  a13, -12
    l32e    a2,  a13,  -8
    l32e    a11, a1,  -12
    l32e    a3,  a13,  -4
    l32e    a4,  a11, -48
    l32e    a5,  a11, -44
    l32e    a6,  a11, -40
    l32e    a7,  a11, -36
    l32e    a8,  a11, -32
    l32e    a9,  a11, -28
    l32e    a10, a11, -24
    l32e    a11, a11, -20
    rfwu

// Interrupts are never enabled by this runtime (INTENABLE = 0), so a level-2..5 vector firing means
// something went wrong; report it like any other fatal exception.
    .section .Level2InterruptVector.text, "ax"
    .global silica_rt_level2_vector
silica_rt_level2_vector:
    wsr     a0, excsave2
    movi    a0, 1002
    j       silica_rt_fatal_trampoline

    .section .Level3InterruptVector.text, "ax"
    .global silica_rt_level3_vector
silica_rt_level3_vector:
    wsr     a0, excsave3
    movi    a0, 1003
    j       silica_rt_fatal_trampoline

    .section .Level4InterruptVector.text, "ax"
    .global silica_rt_level4_vector
silica_rt_level4_vector:
    wsr     a0, excsave4
    movi    a0, 1004
    j       silica_rt_fatal_trampoline

    .section .Level5InterruptVector.text, "ax"
    .global silica_rt_level5_vector
silica_rt_level5_vector:
    wsr     a0, excsave5
    movi    a0, 1005
    j       silica_rt_fatal_trampoline

// Debug exceptions (cause 1006) come from the stack-guard watchpoints rt_start arms; the faulting PC
// is in EPC6, copied to EPC1 so the report shows it.
    .section .DebugExceptionVector.text, "ax"
    .global silica_rt_debug_vector
silica_rt_debug_vector:
    wsr     a0, excsave6
    rsr     a0, epc6
    wsr     a0, epc1
    movi    a0, 1006
    j       silica_rt_fatal_trampoline

    .section .NMIExceptionVector.text, "ax"
    .global silica_rt_nmi_vector
silica_rt_nmi_vector:
    wsr     a0, excsave7
    movi    a0, 1007
    j       silica_rt_fatal_trampoline

    .section .KernelExceptionVector.text, "ax"
    .global silica_rt_kernel_vector
silica_rt_kernel_vector:
    wsr     a0, excsave1
    movi    a0, 1001
    j       silica_rt_fatal_trampoline

// User exceptions: alloca (cause 5) is recoverable; everything else is fatal.
    .section .UserExceptionVector.text, "ax"
    .global silica_rt_user_vector
silica_rt_user_vector:
    wsr     a0, excsave1
    rsr     a0, exccause
    addi    a0, a0, -5
    beqz    a0, 1f
    rsr     a0, exccause
    j       silica_rt_fatal_trampoline
1:  j       silica_rt_alloca_exc      // EXCSAVE1 still holds the interrupted a0, as it expects

    .section .DoubleExceptionVector.text, "ax"
    .global silica_rt_double_vector
silica_rt_double_vector:
    movi    a0, 1008
    j       silica_rt_fatal_trampoline

// a0 = cause code (EXCCAUSE for user exceptions, 1001+ for the other vectors). The faulting
// context is abandoned: a fresh stack and a single live window are set up, the diagnostic is
// printed, and the CPU halts. The windowed register file is unusable in a double exception, so
// the reset of WINDOWSTART here is what makes the report safe to call from any vector.
    .section .text.silica_rt_fatal, "ax"
    .align  4
    .global silica_rt_fatal_trampoline
silica_rt_fatal_trampoline:
    wsr     a0, excsave1          // cause code survives the window reset below
    movi    a0, 0
    wsr     a0, intenable
    rsr     a0, windowbase
    movi    a1, 1
    ssl     a0
    sll     a1, a1
    wsr     a1, windowstart
    rsync
    movi    a1, 0x40020           // PS: WOE=1, EXCM=0, UM=1, INTLEVEL=0, CALLINC=0
    wsr     a1, ps
    rsync
    movi    a1, _silica_fault_stack_top
    rsr     a2, excsave1
    rsr     a3, epc1
    rsr     a4, excvaddr
    movi    a0, 0
    call0   silica_rt_fatal_report_c0
1:  j       1b

// call0 shim into the windowed world: a2 cause, a3 epc, a4 excvaddr, passed on as arguments 0-2.
    .align  4
silica_rt_fatal_report_c0:
    mov     a10, a2
    mov     a12, a3
    mov     a14, a4
    call8   silica_rt_fatal_report
1:  j       1b
