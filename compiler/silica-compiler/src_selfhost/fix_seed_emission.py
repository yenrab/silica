#!/usr/bin/env python3
"""Strip known seed-as-host emission defects from *.sams under src_selfhost.

1) X8 sret slot leak: SUB SP,#N / MOV X8,SP before X0 heap returns (any N).
2) case_done / post-BLR re-box of an already-heap result (extra region_alloc +
   unbalanced ADD). Covers common payload sizes emitted by the seed.
3) Orphan ADD SP,#16 after a balanced heapify (leftover from (1)/(2)).
4) Leftover pair stores through X8 after (1) stripped the sret setup — rewrite
   to a 16-byte heap pair return.
5) List element strides the seed under-sizes:
   - TokenSlot lists: #16/#16 -> #64/#64 (parser)
   - Token lists in constraint_runner: #32/#32 -> #48/#48
   - Declaration lists: seed uses (field_count+1)/2*16 = #64, but Decl value-flat
     layout is 128 (see make_fn_decl_list / emit_function offsets #96/#112).
     cons_declaration prepend #64→#128; Decl tails in sir_generator #64→#128.
   - SIR function lists: value-flat is 56 bytes; emit_function heapifies 64.
     Seed dummy_sir_function leaves a 16-byte gap and heapifies 80 from SP+#16;
     cons_sir_functions uses #80. Walks (emit_functions etc.) often use #48.
     Fix dummy pack to 64, force cons/tails to #64, and patch SIR-fn list
     tails in emitter_core / sir_generator_core from #48/#80 → #64.
6) checked_int64_* pair materialization: SUB SP,#16 / MOV dest,SP left live until
   epilogue → misaligned LDP/frame teardown (wbt_map@smart_node hang). Heapify
   (idempotent; also collapses accidental double-heapify from a prior bug).
7) Large record returns: seed does MOV X0,SP / MOV Xn,X0 … MOV X0,Xn /
   ADD SP,#N and returns a dangling stack pointer (Module, work-table, etc.).
   Caller reads look fine until the next stack frame reuses that memory
   (e.g. empty_work_table after emit_prog → corrupted sir_module.functions).

Also rewrites lexer tuple_proj LDR #48 -> #8 (pointer-pair second field).
"""
from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent

# Heap payload sizes commonly re-boxed after case joins / BLR.
REBOX_SIZES = (16, 32, 48, 64, 80, 96, 112, 128, 144, 160, 208, 224, 304, 320, 336)

SRET_LEAK = re.compile(r"    SUB SP, SP, #\d+\n    MOV X8, SP\n")
LOAD48 = re.compile(r"LDR (X\d+), \[(X\d+), #48\]")

# After stripping a pair/triple heapify, a lone ADD #16 is often left before LDP.
ORPHAN_BEFORE_LDP = re.compile(
    r"(    ADD SP, SP, #\d+\n)"
    r"\n"
    r"    ADD SP, SP, #16\n"
    r"\n"
    r"(    LDP X\d+, X\d+, \[SP\], #16\n)"
)

# Orphan ADD #16 sitting alone between a case_done label and the next branch.
ORPHAN_AFTER_CASE_DONE = re.compile(
    r"(L_case_\w+_done:\n)\n"
    r"    ADD SP, SP, #16\n"
    r"\n"
    r"(    B L_case_\w+_done\n)"
)

# Orphan ADD #16 immediately before epilogue LDP (leftover sret undo).
ORPHAN_BEFORE_EPILOGUE_LDP = re.compile(
    r"(L_case_\w+_done:\n)\n"
    r"    ADD SP, SP, #16\n"
    r"\n"
    r"(    LDP X\d+, X\d+, \[SP\], #16\n)"
)

# After a balanced heapify (ADD #16 + ADD #N), seed often leaves an extra
# ADD #32/#48/... before branching to case_done — stack smash / hang-via-restart.
ORPHAN_AFTER_HEAPIFY_BRANCH = re.compile(
    r"(    ADD SP, SP, #16\n"
    r"    ADD SP, SP, #\d+\n)"
    r"\n"
    r"    ADD SP, SP, #(?:16|32|48|64|80|96)\n"
    r"\n"
    r"(    B L_case_\w+_done\n)"
)
ORPHAN_AFTER_HEAPIFY_BRANCH_REPL = r"\1\n\2"

# After collect_params_* returns a 5-pointer heap tuple in X0, seed often keeps the
# prior tuple pointer (e.g. X24 from bracket-suffix) and loads fields 2–5 from it.
# Rewrite those LDRs to use the real result register.
COLLECT_PARAMS_FIELD_CLOBBER = re.compile(
    r"(    ADRP X9, (constraint_extract_params_collect_params_and_find_type_and_(?:body|semicolon))@PAGE\n"
    r"    ADD X9, X9, \2@PAGEOFF\n"
    r"    BLR X9\n"
    r"    MOV (X\d+), X0\n"
    r"    LDR X0, \[\3, #0\]\n"
    r"    MOV (X\d+), X0\n)"
    r"    LDR X1, \[X\d+, #8\]\n"
    r"    MOV (X\d+), X1\n"
    r"    LDR X0, \[X\d+, #16\]\n"
    r"    MOV (X\d+), X0\n"
    r"    LDR X0, \[X\d+, #24\]\n"
    r"    MOV (X\d+), X0\n"
    r"    LDR X0, \[X\d+, #32\]\n"
    r"    MOV (X\d+), X0\n"
)
COLLECT_PARAMS_FIELD_CLOBBER_REPL = (
    "\\1"
    "    LDR X1, [\\3, #8]\n"
    "    MOV \\5, X1\n"
    "    LDR X0, [\\3, #16]\n"
    "    MOV \\6, X0\n"
    "    LDR X0, [\\3, #24]\n"
    "    MOV \\7, X0\n"
    "    LDR X0, [\\3, #32]\n"
    "    MOV \\8, X0\n"
)

# Pair return still written via X8 after sret setup was stripped.
# Spill the first element: the bool case false-arm often still holds the second
# component in X19 (e.g. changed_rest), so overwriting X19 with the list breaks
# `case changed of` later (non-0/1 → case_clause).
X8_PAIR_RETURN = re.compile(
    r"    MOV X9, X0\n"
    r"    STR X9, \[X8, #0\]\n"
    r"    MOV X1, (X\d+)\n"
    r"(L_case_(\w+)_br_0:\n"
    r"(?:.*\n)*?"
    r"L_case_\3_done:\n)\n"
    r"    STR X9, \[X8, #8\]\n"
    r"    MOV X0, X8\n"
    r"    MOV X19, X0\n"
    r"    MOV X0, X19\n"
    r"    ADD SP, SP, #16\n"
)

X8_PAIR_REPL = (
    "    STR X0, [SP, #-16]!\n"
    "    MOV X1, \\1\n"
    "\\2\n"
    "    MOV X20, X9\n"
    "    LDR X19, [SP], #16\n"
    "    MOV X0, #16\n"
    "    BL _silica_rt_region_alloc\n"
    "    STR X19, [X0, #0]\n"
    "    STR X20, [X0, #8]\n"
    "    MOV X19, X0\n"
    "    MOV X0, X19\n"
)

# Already-applied bad rewrite: list saved in X19 before case that reads X19.
X8_PAIR_BAD = re.compile(
    r"    MOV X19, X0\n"
    r"    MOV X1, (X\d+)\n"
    r"(L_case_(\w+)_br_0:\n"
    r"(?:.*\n)*?"
    r"L_case_\3_done:\n)\n"
    r"    MOV X20, X9\n"
    r"    MOV X0, #16\n"
    r"    BL _silica_rt_region_alloc\n"
    r"    STR X19, \[X0, #0\]\n"
    r"    STR X20, \[X0, #8\]\n"
    r"    MOV X19, X0\n"
    r"    MOV X0, X19\n"
)

X8_PAIR_BAD_REPL = (
    "    STR X0, [SP, #-16]!\n"
    "    MOV X1, \\1\n"
    "\\2\n"
    "    MOV X20, X9\n"
    "    LDR X19, [SP], #16\n"
    "    MOV X0, #16\n"
    "    BL _silica_rt_region_alloc\n"
    "    STR X19, [X0, #0]\n"
    "    STR X20, [X0, #8]\n"
    "    MOV X19, X0\n"
    "    MOV X0, X19\n"
)

# Triple sret via X8: bool@0, ptr@8, ptr@16. Replace with heap 24-byte block.
X8_TRIPLE_RETURN = re.compile(
    r"    MOV X9, #0\n"
    r"    STR X9, \[X8, #0\]\n"
    r"(    ; unsupported term kind \(dummy k\)\?\n"
    r"    ; effects \[\]\|[^\n]+\n"
    r"    ADRP X9, parser_ast_zero_location@PAGE\n"
    r"    ADD X9, X9, parser_ast_zero_location@PAGEOFF\n"
    r"    BLR X9\n"
    r"\n"
    r"    MOV X9, X0\n)"
    r"    STR X9, \[X8, #8\]\n"
    r"    ADRP X9, (L_str_\d+)@PAGE\n"
    r"    ADD X9, X9, (?:L_str_\d+)@PAGEOFF\n"
    r"    STR X9, \[X8, #16\]\n"
    r"    MOV X0, X8\n"
)

X8_TRIPLE_RETURN_REPL = (
    "    MOV X0, #24\n"
    "    BL _silica_rt_region_alloc\n"
    "    STR X0, [SP, #-16]!\n"
    "    MOV X9, #0\n"
    "    STR X9, [X0, #0]\n"
    "\\1"
    "    LDR X0, [SP], #16\n"
    "    STR X9, [X0, #8]\n"
    "    ADRP X9, \\2@PAGE\n"
    "    ADD X9, X9, \\2@PAGEOFF\n"
    "    STR X9, [X0, #16]\n"
)

# After (bool, loc_ptr, str) destructure, third field #40 -> #16.
TRIPLE_THIRD_FIELD_40 = re.compile(
    r"(BLR X9\n"
    r"    MOV (X\d+), X0\n"
    r"    LDR X0, \[\2, #0\]\n"
    r"    MOV X\d+, X0\n"
    r"    LDR X1, \[\2, #8\]\n"
    r"    MOV X\d+, X1\n"
    r"    LDR X0, \[\2, )#40(\]\n)"
)
TRIPLE_THIRD_FIELD_40_REPL = r"\1#16\3"

# Repair botched prior rewrite: `#16Xnn` missing `]`.
TRIPLE_THIRD_FIELD_BROKEN = re.compile(
    r"LDR X0, \[(X\d+), #16\1(\s+MOV)"
)
TRIPLE_THIRD_FIELD_BROKEN_REPL = r"LDR X0, [\1, #16]\n\2"

# Seed emit_checked_pair_from_regs left a stack pair live across the whole frame.
# Negative lookahead: already-heapified emission starts with MOV X0,SP; SUB SP,#16.
# Must stay idempotent — Makefile runs this script after assembly and again in objects.
CHECKED_PAIR_STACK_LEAK = re.compile(
    r"    MOV W9, W0\n"
    r"    UXTW X9, W9\n"
    r"    SUB SP, SP, #16\n"
    r"    STR X9, \[SP\]\n"
    r"    STR X1, \[SP, #8\]\n"
    r"    MOV (X\d+), SP\n"
    r"(?!    SUB SP, SP, #16\n)"
)
CHECKED_PAIR_STACK_LEAK_REPL = (
    "    MOV W9, W0\n"
    "    UXTW X9, W9\n"
    "    SUB SP, SP, #16\n"
    "    STR X9, [SP]\n"
    "    STR X1, [SP, #8]\n"
    "    MOV X0, SP\n"
    "    SUB SP, SP, #16\n"
    "    STR X0, [SP, #8]\n"
    "    MOV X0, #16\n"
    "    BL _silica_rt_region_alloc\n"
    "    MOV X4, X0\n"
    "    LDR X1, [SP, #8]\n"
    "    LDR X3, [X1, #0]\n"
    "    STR X3, [X4, #0]\n"
    "    LDR X3, [X1, #8]\n"
    "    STR X3, [X4, #8]\n"
    "    MOV \\1, X4\n"
    "    ADD SP, SP, #16\n"
    "    ADD SP, SP, #16\n"
)

# Collapse accidental double application of CHECKED_PAIR_STACK_LEAK_REPL.
CHECKED_PAIR_DOUBLE_HEAPIFY = re.compile(
    r"    MOV W9, W0\n"
    r"    UXTW X9, W9\n"
    r"    SUB SP, SP, #16\n"
    r"    STR X9, \[SP\]\n"
    r"    STR X1, \[SP, #8\]\n"
    r"    MOV X0, SP\n"
    r"    SUB SP, SP, #16\n"
    r"    STR X0, \[SP, #8\]\n"
    r"    MOV X0, #16\n"
    r"    BL _silica_rt_region_alloc\n"
    r"    MOV X4, X0\n"
    r"    LDR X1, \[SP, #8\]\n"
    r"    LDR X3, \[X1, #0\]\n"
    r"    STR X3, \[X4, #0\]\n"
    r"    LDR X3, \[X1, #8\]\n"
    r"    STR X3, \[X4, #8\]\n"
    r"    MOV X0, X4\n"
    r"    ADD SP, SP, #16\n"
    r"    ADD SP, SP, #16\n"
    r"    SUB SP, SP, #16\n"
    r"    STR X0, \[SP, #8\]\n"
    r"    MOV X0, #16\n"
    r"    BL _silica_rt_region_alloc\n"
    r"    MOV X4, X0\n"
    r"    LDR X1, \[SP, #8\]\n"
    r"    LDR X3, \[X1, #0\]\n"
    r"    STR X3, \[X4, #0\]\n"
    r"    LDR X3, \[X1, #8\]\n"
    r"    STR X3, \[X4, #8\]\n"
    r"    MOV (X\d+), X4\n"
    r"    ADD SP, SP, #16\n"
    r"    ADD SP, SP, #16\n"
)
CHECKED_PAIR_DOUBLE_HEAPIFY_REPL = CHECKED_PAIR_STACK_LEAK_REPL

SLOT_TAIL_16 = re.compile(
    r"MOV X1, #16\n    MOV X2, #16\n    BL L_list_tail_helper"
)
TOKEN_TAIL_32 = re.compile(
    r"MOV X1, #32\n    MOV X2, #32\n    BL L_list_tail_helper"
)
SLOT_PREPEND_16 = re.compile(
    r"MOV X3, #16\n    MOV X4, #16\n    MOV X5, #1\n    BL L_list_prepend_helper"
)
TOKEN_PREPEND_32 = re.compile(
    r"MOV X3, #32\n    MOV X4, #32\n    MOV X5, #1\n    BL L_list_prepend_helper"
)
# Decl value-flat is 128; seed emits 64 for nested-field records.
DECL_PREPEND_64 = re.compile(
    r"MOV X3, #64\n    MOV X4, #64\n    MOV X5, #1\n    BL L_list_prepend_helper"
)
DECL_TAIL_64 = re.compile(
    r"MOV X1, #64\n    MOV X2, #64\n    BL L_list_tail_helper"
)
DECL_PREPEND_128 = (
    "MOV X3, #128\n    MOV X4, #128\n    MOV X5, #1\n    BL L_list_prepend_helper"
)
DECL_TAIL_128 = "MOV X1, #128\n    MOV X2, #128\n    BL L_list_tail_helper"
SIR_FN_PREPEND_48 = re.compile(
    r"MOV X3, #48\n    MOV X4, #48\n    MOV X5, #1\n    BL L_list_prepend_helper"
)
SIR_FN_TAIL_48 = re.compile(
    r"MOV X1, #48\n    MOV X2, #48\n    BL L_list_tail_helper"
)
def fix_dummy_sir_function(text: str) -> tuple[str, int]:
    """Rewrite seed-broken dummy_sir_function nested-record pack.

    Seed leaves a 16-byte gap before body (unsupported nested Term) and heapifies
    from SP+#16, so the returned record starts at effects instead of name.
    Value-flat SIR function is 56 bytes; emit_function heapifies 64. Use 64.
    """
    start = text.find("sir_ast_dummy_sir_function:")
    if start < 0:
        return text, 0
    # Already fixed?
    window = text[start : start + 2000]
    if "fix_seed: nested Term inlined" in window:
        return text, 0
    if "ADD X0, SP, #16" not in window or "STR X10, [SP, #40]" not in window:
        return text, 0
    end = text.find("\nsir_ast_", start + len("sir_ast_dummy_sir_function:"))
    if end < 0:
        return text, 0
    # Keep prologue through STP X19; replace from SUB SP through RET.
    body = text[start:end]
    ret_i = body.rfind("\n    RET\n")
    if ret_i < 0:
        return text, 0
    sub_i = body.find("\n    SUB SP, SP, #80\n")
    if sub_i < 0:
        return text, 0
    new_mid = """
    SUB SP, SP, #64
    ADRP X9, L_str_0@PAGE
    ADD X9, X9, L_str_0@PAGEOFF
    STR X9, [SP, #0]
    ADRP X9, L_str_0@PAGE
    ADD X9, X9, L_str_0@PAGEOFF
    STR X9, [SP, #8]
    ADRP X9, L_str_1@PAGE
    ADD X9, X9, L_str_1@PAGEOFF
    STR X9, [SP, #16]
    ; fix_seed: nested Term inlined at #24/#32
    ADRP X9, sir_ast_dummy_sir_term@PAGE
    ADD X9, X9, sir_ast_dummy_sir_term@PAGEOFF
    BLR X9

    MOV X9, X0
    LDR X10, [X9, #0]
    STR X10, [SP, #24]
    LDR X10, [X9, #8]
    STR X10, [SP, #32]

    ADRP X9, L_str_0@PAGE
    ADD X9, X9, L_str_0@PAGEOFF
    STR X9, [SP, #40]
    ADRP X9, L_str_0@PAGE
    ADD X9, X9, L_str_0@PAGEOFF
    STR X9, [SP, #48]
    STR XZR, [SP, #56]
    MOV X0, SP
    SUB SP, SP, #16
    STR X0, [SP, #8]
    MOV X0, #64
    BL _silica_rt_region_alloc
    MOV X4, X0
    LDR X1, [SP, #8]
    LDR X3, [X1, #0]
    STR X3, [X4, #0]
    LDR X3, [X1, #8]
    STR X3, [X4, #8]
    LDR X3, [X1, #16]
    STR X3, [X4, #16]
    LDR X3, [X1, #24]
    STR X3, [X4, #24]
    LDR X3, [X1, #32]
    STR X3, [X4, #32]
    LDR X3, [X1, #40]
    STR X3, [X4, #40]
    LDR X3, [X1, #48]
    STR X3, [X4, #48]
    LDR X3, [X1, #56]
    STR X3, [X4, #56]
    MOV X0, X4
    ADD SP, SP, #16
    ADD SP, SP, #64
"""
    new_body = body[:sub_i] + new_mid + body[ret_i:]
    return text[:start] + new_body + text[end:], 1


# SIR function value-flat is 56 bytes; emit_function heapifies 64. Prefer 64 over 80.
SIR_FN_PREPEND_64 = (
    "MOV X3, #64\n    MOV X4, #64\n    MOV X5, #1\n    BL L_list_prepend_helper"
)
SIR_FN_TAIL_64 = "MOV X1, #64\n    MOV X2, #64\n    BL L_list_tail_helper"
SIR_FN_PREPEND_80_TO_64 = re.compile(
    r"MOV X3, #80\n    MOV X4, #80\n    MOV X5, #1\n    BL L_list_prepend_helper"
)
SIR_FN_TAIL_80_TO_64 = re.compile(
    r"MOV X1, #80\n    MOV X2, #80\n    BL L_list_tail_helper"
)


# Staged `lex <- slots.head.token.lexeme` often becomes LDR from SP instead of
# list head + token.lexeme (#8). Rewrite when immediately followed by slot tail.
SLOT_HEAD_LEXEME_SP = re.compile(
    r"    LDR X0, \[SP, #8\]\n"
    r"    MOV (X\d+), X0\n"
    r"    MOV X9, (X\d+)\n"
    r"    MOV X0, X9\n"
    r"    MOV X1, #64\n"
    r"    MOV X2, #64\n"
    r"    BL L_list_tail_helper\n"
)
SLOT_HEAD_LEXEME_SP_REPL = (
    "    MOV X9, \\2\n"
    "    LDR X10, [X9, #8]\n"
    "    LDR X11, [X9, #16]\n"
    "    ADD X10, X10, X11\n"
    "    LDR X0, [X10, #8]\n"
    "    MOV \\1, X0\n"
    "    MOV X9, \\2\n"
    "    MOV X0, X9\n"
    "    MOV X1, #64\n"
    "    MOV X2, #64\n"
    "    BL L_list_tail_helper\n"
)

# Staged `k <- slots.head.token.kind` often becomes LDR from SP (#0/#8) instead of
# list head + token.kind (#0). Detect when followed by a token_kind_* call
# (is_*_decl_start / is_*_section_start predicates). List arg is in X19.
SLOT_HEAD_KIND_SP = re.compile(
    r"    LDR X0, \[SP, #(?:0|8)\]\n"
    r"    MOV (X\d+), X0\n"
    r"((?:    ;[^\n]*\n)*)"
    r"    ADRP X9, (lexer_token_kind_token_kind_\w+)@PAGE\n"
    r"    ADD X9, X9, \3@PAGEOFF\n"
    r"    BLR X9\n"
)
SLOT_HEAD_KIND_SP_REPL = (
    "    MOV X9, X19\n"
    "    LDR X10, [X9, #8]\n"
    "    LDR X11, [X9, #16]\n"
    "    ADD X10, X10, X11\n"
    "    LDR X0, [X10, #0]\n"
    "    MOV \\1, X0\n"
    "\\2"
    "    ADRP X9, \\3@PAGE\n"
    "    ADD X9, X9, \\3@PAGEOFF\n"
    "    BLR X9\n"
)

# Alternate form: kind compared via push/pop around token_kind_* (collect_params).
# List is typically in X20 after skip_leading_whitespace (trimmed).
SLOT_HEAD_KIND_PUSH = re.compile(
    r"    LDR X9, \[SP, #(?:0|8)\]\n"
    r"    STR X9, \[SP, #-16\]!\n"
    r"((?:    ;[^\n]*\n)*)"
    r"    ADRP X9, (lexer_token_kind_token_kind_\w+)@PAGE\n"
    r"    ADD X9, X9, \2@PAGEOFF\n"
    r"    BLR X9\n"
    r"\n"
    r"    MOV X10, X0\n"
    r"    LDR X9, \[SP\], #16\n"
)
SLOT_HEAD_KIND_PUSH_REPL = (
    "    MOV X9, X20\n"
    "    LDR X10, [X9, #8]\n"
    "    LDR X11, [X9, #16]\n"
    "    ADD X10, X10, X11\n"
    "    LDR X9, [X10, #0]\n"
    "    STR X9, [SP, #-16]!\n"
    "\\1"
    "    ADRP X9, \\2@PAGE\n"
    "    ADD X9, X9, \\2@PAGEOFF\n"
    "    BLR X9\n"
    "\n"
    "    MOV X10, X0\n"
    "    LDR X9, [SP], #16\n"
)

# Lexeme compare via LDR [SP,#8] before string_cmp; load from TokenSlot head in X20.
SLOT_HEAD_LEXEME_CMP = re.compile(
    r"    LDR X9, \[SP, #8\]\n"
    r"    ADRP X10, (L_str_\d+)@PAGE\n"
    r"    ADD X10, X10, \1@PAGEOFF\n"
    r"    STP X0, X1, \[SP, #-16\]!\n"
    r"    MOV X0, X9\n"
    r"    MOV X1, X10\n"
    r"    BL L_string_cmp_helper\n"
)
SLOT_HEAD_LEXEME_CMP_REPL = (
    "    MOV X9, X20\n"
    "    LDR X10, [X9, #8]\n"
    "    LDR X11, [X9, #16]\n"
    "    ADD X10, X10, X11\n"
    "    LDR X9, [X10, #8]\n"
    "    ADRP X10, \\1@PAGE\n"
    "    ADD X10, X10, \\1@PAGEOFF\n"
    "    STP X0, X1, [SP, #-16]!\n"
    "    MOV X0, X9\n"
    "    MOV X1, X10\n"
    "    BL L_string_cmp_helper\n"
)

# Staged `tok <- slots.head.token` before token_introduces_fn_type often becomes
# LDR X0,[SP,#0] (list header; length read as kind). Only rewrite when that
# bogus load is forwarded into token_introduces_fn_type.
SLOT_HEAD_TOKEN_SP = re.compile(
    r"    LDR X0, \[SP, #(?:0|8)\]\n"
    r"    MOV (X\d+), X0\n"
    r"    MOV X16, \1\n"
    r"    STR X16, \[SP, #-16\]!\n"
    r"    LDR X0, \[SP, #0\]\n"
    r"    ADD SP, SP, #16\n"
    r"((?:    ;[^\n]*\n)*)"
    r"    ADRP X9, (parser_tuples_token_introduces_fn_type)@PAGE\n"
    r"    ADD X9, X9, \3@PAGEOFF\n"
    r"    BLR X9\n"
)
SLOT_HEAD_TOKEN_SP_REPL = (
    "    MOV X9, X19\n"
    "    LDR X10, [X9, #8]\n"
    "    LDR X11, [X9, #16]\n"
    "    ADD X10, X10, X11\n"
    "    MOV X0, X10\n"
    "    MOV \\1, X0\n"
    "    MOV X16, \\1\n"
    "    STR X16, [SP, #-16]!\n"
    "    LDR X0, [SP, #0]\n"
    "    ADD SP, SP, #16\n"
    "\\2"
    "    ADRP X9, \\3@PAGE\n"
    "    ADD X9, X9, \\3@PAGEOFF\n"
    "    BLR X9\n"
)


def rebox_body(nbytes: int) -> str:
    copies = "".join(
        f"    LDR X3, [X1, #{i * 8}]\n"
        f"    STR X3, [X4, #{i * 8}]\n"
        for i in range(nbytes // 8)
    )
    return (
        "    SUB SP, SP, #16\n"
        "    STR X0, [SP, #8]\n"
        f"    MOV X0, #{nbytes}\n"
        "    BL _silica_rt_region_alloc\n"
        "    MOV X4, X0\n"
        "    LDR X1, [SP, #8]\n"
        f"{copies}"
        "    MOV X0, X4\n"
        "    ADD SP, SP, #16\n"
        f"    ADD SP, SP, #{nbytes}\n"
    )


# MOV X0, SP / MOV Xn, X0 … MOV X0, Xn / ADD SP, #N — return after freeing frame.
STACK_RECORD_RETURN_START = re.compile(
    r"    MOV X0, SP\n"
    r"    MOV (X\d+), X0\n"
)


def heapify_dangling_stack_returns(text: str) -> tuple[str, int]:
    """Copy stack records to the heap before ADD SP frees them (idempotent)."""
    count = 0
    pos = 0
    out: list[str] = []
    while True:
        m = STACK_RECORD_RETURN_START.search(text, pos)
        if not m:
            out.append(text[pos:])
            break
        out.append(text[pos : m.start()])
        reg = m.group(1)
        after = m.end()
        # Already heapified: next ops allocate from the stack pointer.
        if re.match(
            r"    SUB SP, SP, #16\n    STR X0, \[SP, #8\]\n    MOV X0, #\d+\n"
            r"    BL _silica_rt_region_alloc\n",
            text[after:],
        ):
            out.append(m.group(0))
            pos = after
            continue
        end_m = re.search(
            rf"    MOV X0, {reg}\n    ADD SP, SP, #(\d+)\n",
            text[after : after + 2500],
        )
        if not end_m:
            out.append(m.group(0))
            pos = after
            continue
        nbytes = int(end_m.group(1))
        if nbytes not in REBOX_SIZES:
            out.append(m.group(0))
            pos = after
            continue
        window = text[after : after + end_m.start()]
        if "_silica_rt_region_alloc" in window:
            out.append(m.group(0))
            pos = after
            continue
        copies = "".join(
            f"    LDR X3, [X1, #{i * 8}]\n"
            f"    STR X3, [X4, #{i * 8}]\n"
            for i in range(nbytes // 8)
        )
        out.append(
            "    MOV X0, SP\n"
            "    SUB SP, SP, #16\n"
            "    STR X0, [SP, #8]\n"
            f"    MOV X0, #{nbytes}\n"
            "    BL _silica_rt_region_alloc\n"
            "    MOV X4, X0\n"
            "    LDR X1, [SP, #8]\n"
            f"{copies}"
            f"    MOV {reg}, X4\n"
            "    ADD SP, SP, #16\n"
        )
        count += 1
        pos = after
    return "".join(out), count


def fix_text(
    text: str,
    *,
    rewrite48: bool,
    slot_stride: bool,
    token_stride: bool,
    decl_prepend: bool,
    decl_tail: bool,
    sir_fn_stride: bool,
) -> tuple[str, dict[str, int]]:
    counts = {
        "rebox": 0,
        "sret": 0,
        "orphan": 0,
        "ldr48": 0,
        "x8pair": 0,
        "slot16": 0,
        "token32": 0,
        "decl64": 0,
        "sirfn48": 0,
        "slot_lex": 0,
        "slot_kind": 0,
        "slot_tok": 0,
        "slot_kind_push": 0,
        "slot_lex_cmp": 0,
        "collect_clobber": 0,
        "x8triple": 0,
        "triple40": 0,
        "checked_pair": 0,
        "stack_ret": 0,
    }
    new = text
    # Collapse prior double-heapify first, then rewrite remaining stack leaks.
    new, n = CHECKED_PAIR_DOUBLE_HEAPIFY.subn(CHECKED_PAIR_DOUBLE_HEAPIFY_REPL, new)
    counts["checked_pair"] += n
    new, n = CHECKED_PAIR_STACK_LEAK.subn(CHECKED_PAIR_STACK_LEAK_REPL, new)
    counts["checked_pair"] += n
    new, n = heapify_dangling_stack_returns(new)
    counts["stack_ret"] += n
    for nbytes in REBOX_SIZES:
        body = rebox_body(nbytes)
        # Escape for regex; body has no specials except we need exact match.
        body_re = re.escape(body)
        for prefix_re, repl in (
            (r"(L_case_\w+_done:\n)\n" + body_re, r"\1\n"),
            # Seed often leaves a stray ADD #16 between BLR and the re-box.
            (r"(    BLR X9\n)\n    ADD SP, SP, #16\n\n" + body_re, r"\1\n"),
            (r"(    BLR X9\n)\n" + body_re, r"\1\n"),
        ):
            new, n = re.compile(prefix_re).subn(repl, new)
            counts["rebox"] += n
    new, n = SRET_LEAK.subn("", new)
    counts["sret"] += n
    new, n = ORPHAN_BEFORE_LDP.subn(r"\1\n\2", new)
    counts["orphan"] += n
    new, n = ORPHAN_AFTER_CASE_DONE.subn(r"\1\n\2", new)
    counts["orphan"] += n
    new, n = ORPHAN_BEFORE_EPILOGUE_LDP.subn(r"\1\n\2", new)
    counts["orphan"] += n
    new, n = ORPHAN_AFTER_HEAPIFY_BRANCH.subn(ORPHAN_AFTER_HEAPIFY_BRANCH_REPL, new)
    counts["orphan"] += n
    # Post-BLR orphan ADD #16 immediately before epilogue LDP (no re-box left).
    new, n = re.compile(
        r"(    BLR X9\n)\n"
        r"    ADD SP, SP, #16\n"
        r"\n"
        r"(    LDP X\d+, X\d+, \[SP\], #16\n)"
    ).subn(r"\1\n\2", new)
    counts["orphan"] += n
    new, n = X8_PAIR_RETURN.subn(X8_PAIR_REPL, new)
    counts["x8pair"] += n
    new, n = X8_PAIR_BAD.subn(X8_PAIR_BAD_REPL, new)
    counts["x8pair"] += n
    new, n = X8_TRIPLE_RETURN.subn(X8_TRIPLE_RETURN_REPL, new)
    counts["x8triple"] += n
    new, n = TRIPLE_THIRD_FIELD_BROKEN.subn(TRIPLE_THIRD_FIELD_BROKEN_REPL, new)
    counts["triple40"] += n
    new, n = TRIPLE_THIRD_FIELD_40.subn(TRIPLE_THIRD_FIELD_40_REPL, new)
    counts["triple40"] += n
    if slot_stride:
        new, n = SLOT_TAIL_16.subn(
            "MOV X1, #64\n    MOV X2, #64\n    BL L_list_tail_helper", new
        )
        counts["slot16"] += n
        new, n = SLOT_PREPEND_16.subn(
            "MOV X3, #64\n    MOV X4, #64\n    MOV X5, #1\n    BL L_list_prepend_helper",
            new,
        )
        counts["slot16"] += n
    if token_stride:
        new, n = TOKEN_TAIL_32.subn(
            "MOV X1, #48\n    MOV X2, #48\n    BL L_list_tail_helper", new
        )
        counts["token32"] += n
        new, n = TOKEN_PREPEND_32.subn(
            "MOV X3, #48\n    MOV X4, #48\n    MOV X5, #1\n    BL L_list_prepend_helper",
            new,
        )
        counts["token32"] += n
    if decl_prepend:
        new, n = DECL_PREPEND_64.subn(DECL_PREPEND_128, new)
        counts["decl64"] += n
    if decl_tail:
        new, n = DECL_TAIL_64.subn(DECL_TAIL_128, new)
        counts["decl64"] += n
    if sir_fn_stride:
        new, n = SIR_FN_PREPEND_48.subn(SIR_FN_PREPEND_64, new)
        counts["sirfn48"] += n
        new, n = SIR_FN_PREPEND_80_TO_64.subn(SIR_FN_PREPEND_64, new)
        counts["sirfn48"] += n
        new, n = SIR_FN_TAIL_48.subn(SIR_FN_TAIL_64, new)
        counts["sirfn48"] += n
        new, n = SIR_FN_TAIL_80_TO_64.subn(SIR_FN_TAIL_64, new)
        counts["sirfn48"] += n
    new, n = SLOT_HEAD_LEXEME_SP.subn(SLOT_HEAD_LEXEME_SP_REPL, new)
    counts["slot_lex"] += n
    new, n = SLOT_HEAD_KIND_SP.subn(SLOT_HEAD_KIND_SP_REPL, new)
    counts["slot_kind"] += n
    new, n = SLOT_HEAD_KIND_PUSH.subn(SLOT_HEAD_KIND_PUSH_REPL, new)
    counts["slot_kind_push"] += n
    new, n = SLOT_HEAD_LEXEME_CMP.subn(SLOT_HEAD_LEXEME_CMP_REPL, new)
    counts["slot_lex_cmp"] += n
    new, n = SLOT_HEAD_TOKEN_SP.subn(SLOT_HEAD_TOKEN_SP_REPL, new)
    counts["slot_tok"] += n
    new, n = COLLECT_PARAMS_FIELD_CLOBBER.subn(COLLECT_PARAMS_FIELD_CLOBBER_REPL, new)
    counts["collect_clobber"] += n
    if rewrite48:
        new, n = LOAD48.subn(r"LDR \1, [\2, #8]", new)
        counts["ldr48"] += n
    return new, counts


def main() -> int:
    paths = sorted(ROOT.rglob("*.sams"))
    if not paths:
        print("skip: no .sams", file=sys.stderr)
        return 0
    totals = {
        "rebox": 0,
        "sret": 0,
        "orphan": 0,
        "ldr48": 0,
        "x8pair": 0,
        "slot16": 0,
        "token32": 0,
        "decl64": 0,
        "sirfn48": 0,
        "slot_lex": 0,
        "slot_kind": 0,
        "slot_tok": 0,
        "slot_kind_push": 0,
        "slot_lex_cmp": 0,
        "collect_clobber": 0,
        "x8triple": 0,
        "triple40": 0,
        "checked_pair": 0,
        "stack_ret": 0,
    }
    for path in paths:
        # Some .sams embed raw bytes in .ascii/.byte; keep round-trip lossless.
        text = path.read_text(encoding="utf-8", errors="surrogateescape")
        rewrite48 = "lexer" in path.parts
        # TokenSlot under-sized as #16 across parser extract/runner units.
        slot_stride = "parser" in path.parts
        # Token under-sized as #32: constraint_runner reverse/tokens_to_slots and
        # lexer_runner cons_token (Token is 48 bytes; #32 leaves next-chunk at
        # +32 while consumers tail with #48 → length lies, only one element linked).
        token_stride = path.name in ("constraint_runner.sams", "lexer_runner.sams")
        # Decl value-flat is 128 bytes; seed emits #64 for nested-field records.
        # Prepend: only parser_ast.sams (cons_declaration). Do not touch ExprNode
        # #64 prepends in the same file — patch cons_declaration via a scoped replace.
        # Tails: sir_generator Decl walks (SIR fn lists use #48, not #64).
        decl_prepend = False
        decl_tail = (
            "sir_generator" in path.parts
            and path.name
            in (
                "sir_generator_core.sams",
                "traits.sams",
                "ffi.sams",
                "overload_mangle.sams",
                "qualified_call_mangler.sams",
            )
        )
        new, counts = fix_text(
            text,
            rewrite48=rewrite48,
            slot_stride=slot_stride,
            token_stride=token_stride,
            decl_prepend=decl_prepend,
            decl_tail=decl_tail,
            sir_fn_stride=False,
        )
        # Scoped: only parser_ast_cons_declaration prepend 64→128.
        if path.name == "parser_ast.sams":
            pat = re.compile(
                r"(parser_ast_cons_declaration:.*?)(\nparser_ast_[a-z_]+:)",
                re.DOTALL,
            )

            def _repl(m: re.Match[str]) -> str:
                body, n = DECL_PREPEND_64.subn(DECL_PREPEND_128, m.group(1))
                counts["decl64"] += n
                return body + m.group(2)

            new2, nsub = pat.subn(_repl, new, count=1)
            if nsub:
                new = new2
            # ExprNode value-flat is 96 (kind/value/name/Location/inner/right/binds/effects);
            # seed emits #64. Patch make_expr prepend and expr_nodes_* tails only.
            expr_fns = (
                "parser_ast_make_expr",
                "parser_ast_expr_nodes_len",
                "parser_ast_expr_nodes_reverse",
                "parser_ast_expr_nodes_append",
                "parser_ast_expr_remap_nodes",
                "parser_ast_expr_node_at_from_head",
                "parser_ast_expr_node_at",
                "parser_ast_dummy_expr",
            )
            for fn in expr_fns:
                epat = re.compile(
                    rf"({fn}:.*?)(\nparser_ast_[a-z_]+:)",
                    re.DOTALL,
                )

                def _expr_repl(m: re.Match[str], _fn: str = fn) -> str:
                    body = m.group(1)
                    body2, n1 = re.subn(
                        r"MOV X3, #64\n    MOV X4, #64\n    MOV X5, #1\n    BL L_list_prepend_helper",
                        "MOV X3, #96\n    MOV X4, #96\n    MOV X5, #1\n    BL L_list_prepend_helper",
                        body,
                    )
                    body2, n2 = re.subn(
                        r"MOV X1, #64\n    MOV X2, #64\n    BL L_list_tail_helper",
                        "MOV X1, #96\n    MOV X2, #96\n    BL L_list_tail_helper",
                        body2,
                    )
                    counts["decl64"] += n1 + n2  # reuse counter as list-stride fixes
                    return body2 + m.group(2)

                new3, nsub = epat.subn(_expr_repl, new, count=1)
                if nsub:
                    new = new3

        # Scoped: SIR function records — emit_function heapifies 64; align cons + walks.
        # Do NOT rewrite dummy_sir_function: a prior pack rewrite hung emit_declarations.
        if path.name == "sir_ast.sams":
            spat = re.compile(
                r"(sir_ast_cons_sir_functions:.*?)(\nsir_ast_[a-z_]+:)",
                re.DOTALL,
            )

            def _sir_repl(m: re.Match[str]) -> str:
                body = m.group(1)
                body, n1 = SIR_FN_PREPEND_48.subn(SIR_FN_PREPEND_64, body)
                body, n2 = SIR_FN_PREPEND_80_TO_64.subn(SIR_FN_PREPEND_64, body)
                body, n3 = re.subn(
                    r"MOV X3, #80\n    MOV X4, #80\n    MOV X5, #1\n    BL L_list_prepend_helper",
                    SIR_FN_PREPEND_64,
                    body,
                )
                body, n4 = SIR_FN_TAIL_48.subn(SIR_FN_TAIL_64, body)
                body, n5 = SIR_FN_TAIL_80_TO_64.subn(SIR_FN_TAIL_64, body)
                counts["sirfn48"] += n1 + n2 + n3 + n4 + n5
                return body + m.group(2)

            new4, nsub = spat.subn(_sir_repl, new, count=1)
            if nsub:
                new = new4

        # SIR-fn list walks: seed emits #48 (or leftover #80); must match cons #64.
        if path.name in ("emitter_core.sams", "sir_generator_core.sams"):
            if path.name == "emitter_core.sams":
                sir_fn_walk_labels = (
                    "emitter_core_emit_functions",
                    "emitter_core_aggregate_return_heap_stub_scan",
                    "emitter_core_module_has_aggregate_stack_let_scan",
                )
            else:
                sir_fn_walk_labels = (
                    "sir_generator_core_sir_functions_use_actor_runtime",
                )
            for lab in sir_fn_walk_labels:
                start = new.find(lab + ":")
                if start < 0:
                    continue
                # Next top-level label only (skip L_* locals inside the function).
                nxt = re.search(
                    r"\n(?!L_)[a-zA-Z_][a-zA-Z0-9_]*:",
                    new[start + len(lab) + 1 :],
                )
                end = start + len(lab) + 1 + nxt.start() if nxt else len(new)
                body = new[start:end]
                body2, n1 = re.subn(
                    r"MOV X1, #48\n    MOV X2, #48\n    BL L_list_tail_helper",
                    SIR_FN_TAIL_64,
                    body,
                )
                body2, n2 = re.subn(
                    r"MOV X1, #80\n    MOV X2, #80\n    BL L_list_tail_helper",
                    SIR_FN_TAIL_64,
                    body2,
                )
                if n1 or n2:
                    new = new[:start] + body2 + new[end:]
                    counts["sirfn48"] += n1 + n2
        if new != text:
            path.write_text(new, encoding="utf-8", errors="surrogateescape")
            parts = [f"{k}={v}" for k, v in counts.items() if v]
            print(f"fixed: {path.relative_to(ROOT)} ({', '.join(parts)})")
            for k, v in counts.items():
                totals[k] += v
    print(
        "total: "
        f"rebox={totals['rebox']} sret={totals['sret']} "
        f"orphan={totals['orphan']} ldr48={totals['ldr48']} "
        f"x8pair={totals['x8pair']} slot16={totals['slot16']} "
        f"token32={totals['token32']} decl64={totals['decl64']} sirfn48={totals['sirfn48']} "
        f"slot_lex={totals['slot_lex']} "
        f"slot_kind={totals['slot_kind']} slot_tok={totals['slot_tok']} "
        f"slot_kind_push={totals['slot_kind_push']} slot_lex_cmp={totals['slot_lex_cmp']} "
        f"collect_clobber={totals['collect_clobber']} "
        f"x8triple={totals['x8triple']} triple40={totals['triple40']} "
        f"checked_pair={totals['checked_pair']} stack_ret={totals['stack_ret']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
