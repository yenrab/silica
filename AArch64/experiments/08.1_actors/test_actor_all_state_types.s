	.section	__TEXT,__text,regular,pure_instructions
	.build_version macos, 16, 0
	.globl	_func_literal_44                ; -- Begin function func_literal_44
	.p2align	2
_func_literal_44:                       ; @func_literal_44
	.cfi_startproc
; %bb.0:
	stp	x20, x19, [sp, #-32]!           ; 16-byte Folded Spill
	.cfi_def_cfa_offset 32
	stp	x29, x30, [sp, #16]             ; 16-byte Folded Spill
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	.cfi_offset w19, -24
	.cfi_offset w20, -32
	mov	w0, #8
	mov	x19, x1
	bl	_malloc
	ldp	x29, x30, [sp, #16]             ; 16-byte Folded Reload
	str	x19, [x0]
	ldp	x20, x19, [sp], #32             ; 16-byte Folded Reload
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	_main                           ; -- Begin function main
	.p2align	2
_main:                                  ; @main
	.cfi_startproc
; %bb.0:
	stp	x29, x30, [sp, #-16]!           ; 16-byte Folded Spill
	.cfi_def_cfa_offset 16
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	mov	w0, #32
	bl	_malloc
	mov	w8, #2
	mov	w9, #514
	mov	w10, #1
Lloh0:
	adrp	x1, _func_literal_44@PAGE
Lloh1:
	add	x1, x1, _func_literal_44@PAGEOFF
	mov	w2, wzr
	str	x8, [x0]
	strh	w9, [x0, #8]
	stp	x10, x8, [x0, #16]
	bl	_silica_actor_spawn
	ldp	x29, x30, [sp], #16             ; 16-byte Folded Reload
	ret
	.loh AdrpAdd	Lloh0, Lloh1
	.cfi_endproc
                                        ; -- End function
.subsections_via_symbols
