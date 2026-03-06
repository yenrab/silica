.text
.globl _main

    ; pure
_main:     STP X29, X30, [SP, #-16]!

    SUB SP, SP, #16
    SUB SP, SP, #16
    MOV W1, #1
    STR W1, [SP, #0]
    ADRP X1, L_str_0@PAGE
    ADD X1, X1, L_str_0@PAGEOFF
    STR X1, [SP, #8]
    MOV X1, SP
    STR X1, [SP, #16]
    MOV X1, #42
    STR X1, [SP, #24]
    ADD X0, SP, #16
    MOV X1, X0
    LDR X2, [SP, #16]
    LDR X0, [SP, #24]
    LDR X0, [SP, #16]
    LDR W1, [SP, #16]
    LDR X0, [SP, #24]
    LDR X0, [SP, #24]
    ADD SP, SP, #32

    LDP X29, X30, [SP], #16
    RET
.section __TEXT,__rodata
.align	8
L_str_0:
    .ascii "hello"
    .byte 0

.text
