	.section	__TEXT,__text,regular,pure_instructions
	.build_version macos, 16, 0
	.globl	_func_literal_42                ; -- Begin function func_literal_42
	.p2align	2
_func_literal_42:                       ; @func_literal_42
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
	add	x19, x8, x9
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
	stp	x20, x19, [sp, #-32]!           ; 16-byte Folded Spill
	.cfi_def_cfa_offset 32
	stp	x29, x30, [sp, #16]             ; 16-byte Folded Spill
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	.cfi_offset w19, -24
	.cfi_offset w20, -32
	mov	w0, #24
	bl	_malloc
	mov	w8, #42
	mov	w9, #1
Lloh0:
	adrp	x10, l_str_const_0@PAGE
Lloh1:
	add	x10, x10, l_str_const_0@PAGEOFF
Lloh2:
	adrp	x19, _func_literal_42@PAGE
Lloh3:
	add	x19, x19, _func_literal_42@PAGEOFF
	str	x8, [x0]
	strb	w9, [x0, #8]
	str	x10, [x0, #16]
	mov	w0, #8
	bl	_malloc
	mov	x1, x19
	mov	w2, wzr
	str	xzr, [x0]
	bl	_silica_actor_spawn
	ldp	x29, x30, [sp, #16]             ; 16-byte Folded Reload
	mov	x0, xzr
	ldp	x20, x19, [sp], #32             ; 16-byte Folded Reload
	ret
	.loh AdrpAdd	Lloh2, Lloh3
	.loh AdrpAdd	Lloh0, Lloh1
	.cfi_endproc
                                        ; -- End function
	.section	__TEXT,__cstring,cstring_literals
l_str_const_0:                          ; @str_const_0
	.asciz	"test"

.subsections_via_symbols
