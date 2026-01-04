	.section	__TEXT,__text,regular,pure_instructions
	.build_version macos, 16, 0
	.globl	_func_literal_30                ; -- Begin function func_literal_30
	.p2align	2
_func_literal_30:                       ; @func_literal_30
	.cfi_startproc
; %bb.0:
	stp	x20, x19, [sp, #-32]!           ; 16-byte Folded Spill
	.cfi_def_cfa_offset 32
	stp	x29, x30, [sp, #16]             ; 16-byte Folded Spill
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	.cfi_offset w19, -24
	.cfi_offset w20, -32
	ldr	x8, [x1]
	ldr	x9, [x0]
	mov	w0, #8
	add	x19, x9, x8, lsl #1
	bl	_malloc
	ldp	x29, x30, [sp, #16]             ; 16-byte Folded Reload
	str	x19, [x0]
	ldp	x20, x19, [sp], #32             ; 16-byte Folded Reload
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	_func_literal_31                ; -- Begin function func_literal_31
	.p2align	2
_func_literal_31:                       ; @func_literal_31
	.cfi_startproc
; %bb.0:                                ; %case_check_1
	mov	w8, #1
	cmp	x0, #0
	cinc	w8, w8, gt
	lsr	w8, w8, #1
	cmp	w8, #0
	csel	x0, x0, xzr, ne
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	_func_literal_32                ; -- Begin function func_literal_32
	.p2align	2
_func_literal_32:                       ; @func_literal_32
	.cfi_startproc
; %bb.0:
	stp	x20, x19, [sp, #-32]!           ; 16-byte Folded Spill
	.cfi_def_cfa_offset 32
	stp	x29, x30, [sp, #16]             ; 16-byte Folded Spill
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	.cfi_offset w19, -24
	.cfi_offset w20, -32
	ldr	x8, [x0]
	mov	w0, #8
	ldr	x9, [x1]
	add	x10, x8, x9
	madd	x19, x8, x9, x10
	bl	_malloc
	ldp	x29, x30, [sp, #16]             ; 16-byte Folded Reload
	str	x19, [x0]
	ldp	x20, x19, [sp], #32             ; 16-byte Folded Reload
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	_func_literal_33                ; -- Begin function func_literal_33
	.p2align	2
_func_literal_33:                       ; @func_literal_33
	.cfi_startproc
; %bb.0:
	stp	x20, x19, [sp, #-32]!           ; 16-byte Folded Spill
	.cfi_def_cfa_offset 32
	stp	x29, x30, [sp, #16]             ; 16-byte Folded Spill
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	.cfi_offset w19, -24
	.cfi_offset w20, -32
	ldr	x8, [x0]
	mov	w0, #8
	ldr	x9, [x1]
	mul	x9, x8, x9
	add	x10, x9, #10
	add	x9, x9, #11
	cmp	x10, #0
	csel	x9, x9, x10, lt
	add	x19, x8, x9, asr #1
	bl	_malloc
	ldp	x29, x30, [sp, #16]             ; 16-byte Folded Reload
	str	x19, [x0]
	ldp	x20, x19, [sp], #32             ; 16-byte Folded Reload
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	_func_literal_34                ; -- Begin function func_literal_34
	.p2align	2
_func_literal_34:                       ; @func_literal_34
	.cfi_startproc
; %bb.0:                                ; %case_check_0
	cmp	x0, #0
	csel	x0, xzr, x0, eq
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	_main                           ; -- Begin function main
	.p2align	2
_main:                                  ; @main
	.cfi_startproc
; %bb.0:
	mov	x0, xzr
	ret
	.cfi_endproc
                                        ; -- End function
.subsections_via_symbols
