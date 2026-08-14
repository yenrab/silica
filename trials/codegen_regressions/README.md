# codegen_regressions

Minimal programs that each pin down one code-generation defect found while getting the
selfhost compiler to compile `trials/base`. Every file here failed before a specific emitter
fix and passes after it, so a future change that reintroduces the defect fails `make integrate`
in this directory instead of surfacing as a hang somewhere inside the compiler.

These are deliberately small (13–76 lines) because each one was reduced from a compiler hang
until it was the smallest program that still misbehaved. Keep them small: their value is that a
failure here names the defect, and a large program does not.

## Running

```
make -C trials/codegen_regressions integrate
```

`integrate` checks two goldens per program: `.ascomp` (emitted assembly) and `.scout` (program
output followed by its exit status). The exit status is the whole answer for most of these — the
program computes a value whose correctness depends on the frame or the aggregate layout being
right, and returns it.

The goldens were recorded with `binaries/seed-compiler`. Since `integrate` defaults to
`binaries/silica-compiler`, a clean run here is also a differential test: it asserts the selfhost
emitter produces the same assembly as the seed for these shapes. Assembly goldens move whenever
the emitter's output legitimately changes; refresh them with
`trials/tmp_refresh_ascomp_from_sams.sh` after confirming the `.scout` results still hold.

## What each program guards

| Program | Defect it pins down | Value |
| --- | --- | --- |
| `agg_param_tailcall` | A function with an aggregate first parameter shadows it in X20, so it must save X19/X20 even when its body is a tail call | 30 |
| `and_shortcircuit` | `and` must not evaluate its right operand when the left one is false | 2 |
| `arg_clobber`, `arg_clobber2` | Argument marshalling must not write X0–X7 before every argument has been evaluated | 3, 3 |
| `kind5_arg_clobber` | A kind-5 (whole-body tail call) wrapper must shadow later GPR params before nested argument calls clobber X1–X7 | 17 |
| `hof_callee_spill` | A later let must not steal the callee-saved register holding a function parameter; `f(...)` is a live use of `f` | 28 |
| `hof_record_indirect` | An indirect callee in X17 must survive `region_alloc` when an argument is a heap-promoted `record_make` (not a user call); save/reload around args, including before a kind-5 tail BR | 7 |
| `bound_wrap` | Parallel GPR staging must heap-promote an inline `record_make` before STR slots (the `term_to_asm_debug_with_outer` shape); interleaved payload + omitted demoted-tail RET hung emit | 42 |
| `decl_pair_escape` | `extract_function`'s `({tag,name,params,loc,body}, rest)` shape: nested `record_make` in a returned pair must be heap-promoted, and a spilled tuple element `n` must survive a later `BL` (`body_bound` is the spill reg, not X1). Selfhost emitting `MOV X0, #0` for `fn main(){42}` when decls were corrupted | 42 |
| `make_expr_append_stage` | `append(combined, prepend(node, empty))` with 8 live params so `combined` spills: staging must accept memory_ref bindings and treat `list_cons` as a clobbering arg (else root OOB → placeholder `#0`) | 42 |
| `find_kw_substr` | `substring(s, i, i+len(kw))` must keep `s` in X0 while evaluating the end index; only saving X1 let `len(kw)` clobber the haystack and made typecheck hang | 42 |
| `direct_tuple_calls` | A direct `(…)` body containing calls must build on its own slab; X8 is caller-saved | 34 |
| `fixpoint`, `fixpoint_one` | Fixed-point iteration over lists (the `propagate_until_fixed` shape) | 3, 11 |
| `list_eq` | List equality over int64 elements | 21 |
| `list_rec_nested` | A cons cell holding a record with a non-scalar field must size the element by the record's inlined layout, not one word per field | 7 |
| `logic_ops` | `and`/`or`/`not` truth table, short-circuiting | 241 |
| `prim_tuple_let` | A checked-int64 primitive's `(ok, value)` pair pops its own slab, so neither the let binding nor the epilogue may pop it again | 40 |
| `rec_call_field` | Record field stores must start after the area reserved for a nested call's returned aggregate | 14 |
| `rec_call_field_direct` | A direct `{…}` body's epilogue must pop the shell plus every nested allocation | 14 |
| `rec_nested_nonscalar` | A nested record literal is inlined into the outer record even when its fields are not all scalars; storing a pointer left it dangling | 22 |
| `region_case_arm` | A region allocation reachable only inside a case arm still counts as region use, so the region helpers get emitted | 9 |
| `or_continued_line` | A line-ending `or` plus a blank (whitespace-only) line is one expression; newline/tab/CR/space are lexer whitespace, not E1040 statement breaks (`prims_record` `value_flat_leaf_field_type_p`). `//` stops at newline; `/` is division | 7 |
| `seq_literal_rhs` | Sequence `<- 7 produces` with no `;` (newlines are not tokens): `produces` must stay a legal follower of a binding RHS so propagate does not empty the literal's roles | 7 |
| `line_comments` | Whole-line `//` and end-of-line `//` must stop at the newline (not eat `main` / the next statement); a lone `/` is still division | 7 |
| `case_string_newline` | Case patterns must unescape like string expressions so `"\n"` matches a real newline (`skip_line_comment`); otherwise `//` eats the rest of the file | 7 |
| `case_string_quote` | Case patterns must skip `\"` when finding the closer so `"\""` is a quote, not a backslash (selfhost `case peek of { "\"" -> read_string }` otherwise never lexes strings → false E1040 on `bound_wrap`) | 1 |
| `region_grow` | A region outgrows its first block: allocation attaches another block instead of aborting, and refs from earlier blocks stay readable and still test in-region | 42 |
| `shadow_x21` | Parameters shadowed into X19/X20 push the body's first let onto X21, so the frame must count them when deciding which callee-saved pairs to save | 17 |
| `tuple_let_case` | A let-spine body whose tail is an id bound to a block ending in a case leaves nothing on the frame, so the epilogue must not pop | 14 |
| `concat_call_operand` | `concat`'s left operand is staged in X0, so a call in the right operand must not be allowed to overwrite it, or both operands become the right one | 8 |
| `concat_fold_call` | The same clobber when the right operand is a call *with arguments* (SIR kind 5): the call detection counted only zero-argument calls, so a `concat(head, recurse(tail))` fold collapsed to its base case | 6 |
| `concat_third_param` | `string_concat` must trigger full param shadow so a third string argument survives `BL L_concat_helper` (`emit_compare_op`'s `reg_prefix`) | 3 |
| `list_head_call` | A scalar list head must live in a callee-saved register: recursing on the tail before using the head destroyed it when it sat in X1 | 6 |
| `tup_call_elem` | A tuple-returning call in tuple-element position must give its caller sret slab back, or the later elements are stored through a shifted SP and the tuple pops less than it pushed | 77 |
| `tup_let_rec` | Nested `record_make` in a returned pair must be heap-promoted and its self-SUB reclaimed (`extract_function`); let-bound early promote hung typecheck | 42 |
| `tup_let_rec_nested` | Early promote of a let-bound record must not reclaim the RHS slab before the body: nested field pointers in `(r, r.inner.a)` must stay live (selfhost typecheck hang) | 42 |
| `tup_rec_destr`, `tup_rec_depth` | Destructuring a tuple of records across calls that clobber the sret pointer | 3, 3 |
| `tuple3_nested_destr`, `tuple3_nested_destr2` | Projecting elements 1 and 2 of a 3-tuple must use the freshly returned base, not a stale register | 7, 7 |
| `repro_destr`, `repro_letcall`, `repro_seqcase`, `repro_walk3` | Reductions from the original `walk3` investigation: aggregate returns through destructuring, let-bound calls, sequence blocks, and the full walk | 77, 60, 81, 42 |
| `w3_min`, `w3_min2`, `w3_norec_read`, `w3_nostring`, `w3_probe`, `w3_step_depth`, `w3_step_let`, `w3_step_param` | Successive reductions of `repro_walk3`, each removing one ingredient (records, strings, the let, recursion depth) to isolate the escaping interior pointer | 42, 7, 2, 42, 32, 7, 7, 7 |

## Not included

Two reductions from the same investigation are still failing and are therefore not here, because
promoting them would record a wrong answer as the golden. They remain under `.scratch/repros`:

- `repro_armseq` — returns 107 where 103 is correct.
- `w3_nolet` — hangs; the arms of its case disagree about how much they pushed.

Add each one here once it passes, with its correct value as the golden.
